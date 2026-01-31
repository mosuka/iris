//! VectorStore: Low-level vector storage and retrieval.
//!
//! This module provides a component for managing vector fields and segments.
//! It does NOT handle document ID mapping or metadata storage.
//!
//! # Module Structure
//!
//! - [`config`] - Configuration types (VectorIndexConfig, VectorFieldConfig, VectorIndexKind)
//! - [`embedder`] - Embedding utilities
//! - [`filter`] - Metadata filtering
//! - [`memory`] - In-memory field implementation
//! - [`registry`] - Document vector registry
//! - [`request`] - Search request types
//! - [`response`] - Search response types
//! - [`snapshot`] - Snapshot persistence
//! - [`wal`] - Write-Ahead Logging
//!
//! # Refactoring: Segmented Document Storage
//!
//! VectorStore has been refactored to move from a "single massive documents.json snapshot"
//! to a "segmented document storage" model. Document data is now stored in binary segments
//! on disk, and only currently active (uncommitted) documents are kept in memory.

pub mod config;
pub mod embedder;
pub mod embedding_writer;
pub mod filter;
pub mod memory;
pub mod query;
pub mod request;
pub mod response;
pub mod snapshot;
#[cfg(test)]
mod tests;

use std::cmp::Ordering as CmpOrdering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{Mutex, RwLock};

use crate::data::{DataValue, Document};
use crate::embedding::embedder::Embedder;
use crate::embedding::per_field::PerFieldEmbedder;
use crate::error::{IrisError, Result};
use crate::maintenance::deletion::DeletionManager;
use crate::storage::Storage;
use crate::storage::prefixed::PrefixedStorage;
use crate::vector::core::vector::Vector;
use crate::vector::index::field::{
    AdapterBackedVectorField, FieldHit, FieldSearchInput, VectorField, VectorFieldReader,
    VectorFieldStats, VectorFieldWriter,
};
use crate::vector::index::field_factory::VectorFieldFactory;

use self::embedder::{EmbedderExecutor, VectorEmbedderRegistry};
use self::memory::{FieldHandle, FieldRuntime, InMemoryVectorField};
// use self::segmented_docs::SegmentedDocumentStore;
use crate::store::document::UnifiedDocumentStore;

use self::request::{FieldSelector, QueryVector, VectorScoreMode, VectorSearchRequest};
use self::response::{VectorHit, VectorSearchResults, VectorStats};
use self::snapshot::{
    COLLECTION_MANIFEST_FILE, COLLECTION_MANIFEST_VERSION, CollectionManifest, REGISTRY_NAMESPACE,
};

use crate::vector::store::config::{VectorFieldConfig, VectorIndexConfig};

/// A vector storage component.
pub struct VectorStore {
    config: Arc<VectorIndexConfig>,
    field_configs: Arc<RwLock<HashMap<String, VectorFieldConfig>>>,
    fields: Arc<RwLock<HashMap<String, FieldHandle>>>,
    embedder_registry: Arc<VectorEmbedderRegistry>,
    embedder_executor: Mutex<Option<Arc<EmbedderExecutor>>>,
    /// Manager for logical deletions.
    deletion_manager: Arc<DeletionManager>,

    storage: Arc<dyn Storage>,
    doc_store: Arc<RwLock<UnifiedDocumentStore>>,
    snapshot_wal_seq: AtomicU64,
    closed: AtomicU64, // 0 = open, 1 = closed
}

impl fmt::Debug for VectorStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VectorStore")
            .field("config", &self.config)
            .field("field_count", &self.fields.read().len())
            .finish()
    }
}

impl VectorStore {
    /// Create a new vector store with the given storage and configuration.
    pub fn new(
        storage: Arc<dyn Storage>,
        config: VectorIndexConfig,
        doc_store: Arc<RwLock<UnifiedDocumentStore>>,
    ) -> Result<Self> {
        let embedder_registry = Arc::new(VectorEmbedderRegistry::new());
        let field_configs = Arc::new(RwLock::new(config.fields.clone()));

        // Store the embedder from config before moving config into Arc
        let config_embedder = config.embedder.clone();
        let deletion_config = config.deletion_config.clone();

        let deletion_manager = Arc::new(DeletionManager::new(deletion_config, storage.clone())?);

        let executor = Arc::new(EmbedderExecutor::new()?);

        let mut store = Self {
            config: Arc::new(config),
            field_configs: field_configs.clone(),
            fields: Arc::new(RwLock::new(HashMap::new())),
            embedder_registry,
            embedder_executor: Mutex::new(Some(executor)),
            deletion_manager,
            storage: storage.clone(),
            doc_store,
            snapshot_wal_seq: AtomicU64::new(0),
            closed: AtomicU64::new(0),
        };

        let shard_id = store.config.shard_id;
        store.deletion_manager.initialize_segment(
            "global",
            crate::util::id::create_doc_id(shard_id, 0),
            crate::util::id::create_doc_id(shard_id, crate::util::id::MAX_LOCAL_ID),
        )?;

        // Do not load document snapshot anymore as documents are managed by UnifiedDocumentStore.
        // store.load_persisted_state()?;
        // We still need load_collection_manifest though!
        store.load_collection_manifest(storage.clone())?;
        store.instantiate_configured_fields()?;

        store.register_embedder_from_config(config_embedder)?;

        Ok(store)
    }

    /// Register embedder instances from the Embedder trait object.
    fn register_embedder_from_config(&self, embedder: Arc<dyn Embedder>) -> Result<()> {
        let configs = self.field_configs.read().clone();
        if let Some(per_field) = embedder.as_any().downcast_ref::<PerFieldEmbedder>() {
            for field_name in configs.keys() {
                let field_embedder = per_field.get_embedder(field_name).clone();
                self.embedder_registry
                    .register(field_name.clone(), field_embedder.clone());
            }
        } else {
            for field_name in configs.keys() {
                self.embedder_registry
                    .register(field_name.clone(), embedder.clone());
            }
        }

        Ok(())
    }

    /// Register a concrete field implementation. Each field name must be unique.
    pub fn register_field_impl(&self, field: Arc<dyn VectorField>) -> Result<()> {
        let name = field.name().to_string();
        let mut fields = self.fields.write();
        if fields.contains_key(&name) {
            return Err(IrisError::invalid_config(format!(
                "vector field '{name}' is already registered"
            )));
        }
        let runtime = FieldRuntime::from_field(&field);
        fields.insert(name, FieldHandle { field, runtime });
        Ok(())
    }

    /// Convenience helper to register a field backed by legacy adapters.
    pub fn register_adapter_field(
        &self,
        name: impl Into<String>,
        config: VectorFieldConfig,
        writer: Arc<dyn VectorFieldWriter>,
        reader: Arc<dyn VectorFieldReader>,
    ) -> Result<()> {
        let field: Arc<dyn VectorField> =
            Arc::new(AdapterBackedVectorField::new(name, config, writer, reader));
        self.register_field_impl(field)
    }

    // =========================================================================
    // Internal methods
    // =========================================================================

    fn check_closed(&self) -> Result<()> {
        if self.closed.load(Ordering::Relaxed) == 1 {
            return Err(IrisError::index("VectorStore is closed"));
        }
        Ok(())
    }

    fn ensure_embedder_executor(&self) -> Result<Arc<EmbedderExecutor>> {
        let mut guard = self.embedder_executor.lock();
        if let Some(executor) = guard.as_ref() {
            return Ok(executor.clone());
        }
        let executor = Arc::new(EmbedderExecutor::new()?);
        *guard = Some(executor.clone());
        Ok(executor)
    }

    fn instantiate_configured_fields(&mut self) -> Result<()> {
        let configs: Vec<(String, VectorFieldConfig)> = self
            .field_configs
            .read()
            .iter()
            .map(|(name, config)| (name.clone(), config.clone()))
            .collect();

        for (name, config) in configs {
            if self.fields.read().contains_key(&name) {
                continue;
            }
            let field = self.create_vector_field(name, config)?;
            self.register_field_impl(field)?;
        }
        Ok(())
    }

    fn create_vector_field(
        &self,
        name: String,
        config: VectorFieldConfig,
    ) -> Result<Arc<dyn VectorField>> {
        let storage = self.field_storage(&name);
        let executor = self.ensure_embedder_executor()?;
        VectorFieldFactory::create_field(
            name,
            config,
            storage,
            self.deletion_manager.get_bitmap("global"),
            self.config.embedder.clone(),
            executor,
        )
    }

    fn write_field_delegate_index(
        &self,
        field_name: &str,
        config: &VectorFieldConfig,
        vectors: Vec<(u64, String, Vector)>,
    ) -> Result<()> {
        let vector_option = config.vector.as_ref().ok_or_else(|| {
            IrisError::invalid_config(format!(
                "vector field '{field_name}' validation failed: no vector configuration found"
            ))
        })?;

        let executor = self.ensure_embedder_executor()?;
        let storage = self.field_storage(field_name);
        let writer = VectorFieldFactory::create_writer(
            field_name,
            vector_option,
            storage,
            self.config.embedder.clone(),
            executor,
        )?;

        writer.rebuild(vectors)?;
        writer.flush()?;

        Ok(())
    }

    fn load_delegate_reader(
        &self,
        field_name: &str,
        config: &VectorFieldConfig,
    ) -> Result<Arc<dyn VectorFieldReader>> {
        let storage = self.field_storage(field_name);
        let global_bitmap = self.deletion_manager.get_bitmap("global");
        let vector_option = config.vector.as_ref().ok_or_else(|| {
            IrisError::invalid_config(format!(
                "vector field '{field_name}' has no vector configuration"
            ))
        })?;

        VectorFieldFactory::create_reader(field_name, vector_option, storage, global_bitmap)
    }

    fn field_storage(&self, field_name: &str) -> Arc<dyn Storage> {
        let prefix = Self::field_storage_prefix(field_name);
        Arc::new(PrefixedStorage::new(prefix, self.storage.clone())) as Arc<dyn Storage>
    }

    fn registry_storage(&self) -> Arc<dyn Storage> {
        Arc::new(PrefixedStorage::new(
            REGISTRY_NAMESPACE,
            self.storage.clone(),
        )) as Arc<dyn Storage>
    }

    fn field_storage_prefix(field_name: &str) -> String {
        let mut sanitized: String = field_name
            .chars()
            .map(|ch| match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
                _ => '_',
            })
            .collect();
        if sanitized.is_empty() {
            sanitized.push_str("field");
        }
        format!("vector_fields/{sanitized}")
    }

    fn validate_document_fields(&self, _document: &Document) -> Result<()> {
        Ok(())
    }

    fn delete_fields_for_doc(&self, doc_id: u64, doc: &Document) -> Result<()> {
        let fields = self.fields.read();
        for field_name in doc.fields.keys() {
            if let Some(field) = fields.get(field_name) {
                field.runtime.writer().delete_document(doc_id, 0)?;
            }
        }

        self.deletion_manager
            .delete_document("global", doc_id, "delete_request")?;

        Ok(())
    }

    fn apply_field_updates(&self, doc_id: u64, version: u64, document: &Document) -> Result<()> {
        let fields = self.fields.read();
        for (field_name, val) in &document.fields {
            if let Some(field) = fields.get(field_name) {
                field.runtime.writer().add_value(doc_id, val, version)?;
            }
        }
        Ok(())
    }

    fn load_collection_manifest(&self, storage: Arc<dyn Storage>) -> Result<()> {
        if !storage.file_exists(COLLECTION_MANIFEST_FILE) {
            return Ok(());
        }

        let mut input = storage.open_input(COLLECTION_MANIFEST_FILE)?;
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer)?;
        input.close()?;
        if buffer.is_empty() {
            return Ok(());
        }

        let manifest: CollectionManifest = serde_json::from_slice(&buffer)?;
        if manifest.version != COLLECTION_MANIFEST_VERSION {
            return Err(IrisError::invalid_config(format!(
                "collection manifest version mismatch: expected {}, found {}",
                COLLECTION_MANIFEST_VERSION, manifest.version
            )));
        }

        if !manifest.field_configs.is_empty() {
            *self.field_configs.write() = manifest.field_configs.clone();
        }

        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.check_closed()?;

        // 1. Flush fields
        let fields = self.fields.read();
        for field_entry in fields.values() {
            field_entry.runtime.writer().flush()?;
        }

        Ok(())
    }

    fn find_all_internal_ids_by_external_id(&self, external_id: &str) -> Result<Vec<u64>> {
        let mut results = self.doc_store.read().find_all_by_external_id(external_id)?;

        // Remove duplicates if any
        results.sort_unstable();
        results.dedup();

        Ok(results)
    }

    fn persist_state(&self) -> Result<()> {
        // documents are persisted in flush() -> add_segment()
        self.persist_manifest()
    }

    fn persist_manifest(&self) -> Result<()> {
        let storage = self.registry_storage();
        let manifest = CollectionManifest {
            version: COLLECTION_MANIFEST_VERSION,
            snapshot_wal_seq: self.snapshot_wal_seq.load(Ordering::SeqCst),
            wal_last_seq: self.snapshot_wal_seq.load(Ordering::SeqCst),
            field_configs: self.field_configs.read().clone(),
        };
        let serialized = serde_json::to_vec(&manifest)?;
        self.write_atomic(storage, COLLECTION_MANIFEST_FILE, &serialized)
    }

    fn write_atomic(&self, storage: Arc<dyn Storage>, name: &str, bytes: &[u8]) -> Result<()> {
        let tmp_name = format!("{name}.tmp");
        let mut output = storage.create_output(&tmp_name)?;
        output.write_all(bytes)?;
        output.flush_and_sync()?;
        output.close()?;
        if storage.file_exists(name) {
            storage.delete_file(name)?;
        }
        storage.rename_file(&tmp_name, name)
    }

    // =========================================================================
    // Public API
    // =========================================================================

    pub fn config(&self) -> &VectorIndexConfig {
        self.config.as_ref()
    }

    pub fn embedder(&self) -> Arc<dyn Embedder> {
        Arc::clone(self.config.get_embedder())
    }

    pub fn add_document(&self, doc: Document) -> Result<u64> {
        self.check_closed()?;

        if doc.id().is_some() {
            return self.put_document(doc);
        }

        let doc_id = self.doc_store.write().add_document(doc.clone())?;
        self.upsert_document_by_internal_id(doc_id, doc)?;
        Ok(doc_id)
    }

    pub fn add_documents(&self, docs: impl IntoIterator<Item = Document>) -> Result<Vec<u64>> {
        docs.into_iter().map(|doc| self.add_document(doc)).collect()
    }

    pub fn put_document(&self, mut doc: Document) -> Result<u64> {
        self.check_closed()?;
        let external_id = doc.id().map(|s| s.to_string()).ok_or_else(|| {
            IrisError::invalid_argument("Document ID is required for put_document")
        })?;

        if !doc.fields.contains_key("_id") {
            doc.fields
                .insert("_id".to_string(), DataValue::Text(external_id.clone()));
        }

        let existing_ids = self.find_all_internal_ids_by_external_id(&external_id)?;
        for &id in &existing_ids {
            self.delete_document_by_internal_id(id)?;
        }

        let doc_id = self.doc_store.write().add_document(doc.clone())?;
        self.upsert_document_by_internal_id(doc_id, doc)?;
        Ok(doc_id)
    }

    pub fn get_documents(&self, external_id: &str) -> Result<Vec<Document>> {
        let ids = self.find_all_internal_ids_by_external_id(external_id)?;
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(doc) = self.get_document_by_internal_id(id)? {
                results.push(doc);
            }
        }
        Ok(results)
    }

    pub fn delete_documents(&self, external_id: &str) -> Result<bool> {
        let ids = self.find_all_internal_ids_by_external_id(external_id)?;
        println!(
            "DEBUG: delete_documents({}) found ids: {:?}",
            external_id, ids
        );
        if ids.is_empty() {
            return Ok(false);
        }
        for id in ids {
            self.delete_document_by_internal_id(id)?;
        }
        Ok(true)
    }

    pub fn index_payload_chunk(&self, payload: Document) -> Result<u64> {
        self.check_closed()?;
        let doc_id = self.doc_store.write().add_document(payload.clone())?;
        self.upsert_document_by_internal_id(doc_id, payload)?;
        Ok(doc_id)
    }

    // Internal helper for indexing
    fn upsert_document_with_id(&self, doc_id: u64, document: Document) -> Result<()> {
        self.check_closed()?;
        self.validate_document_fields(&document)?;

        // 2. Update In-Memory Structures
        // NOTE: Document is managed by UnifiedDocumentStore.
        // We only update field writers here.

        // Update Field Writers
        self.apply_field_updates(doc_id, 0, &document)?;

        Ok(())
    }
    /// Register an external field implementation.
    pub fn register_field(&self, _name: String, field: Box<dyn VectorField>) -> Result<()> {
        let field_arc: Arc<dyn VectorField> = Arc::from(field);
        self.register_field_impl(field_arc)
    }

    /// Get statistics for a specific field.
    pub fn field_stats(&self, field_name: &str) -> Result<VectorFieldStats> {
        let fields = self.fields.read();
        let field = fields.get(field_name).ok_or_else(|| {
            IrisError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        field.runtime.reader().stats()
    }

    /// Replace the reader for a specific field.
    pub fn replace_field_reader(
        &self,
        field_name: &str,
        reader: Box<dyn VectorFieldReader>,
    ) -> Result<()> {
        let fields = self.fields.read();
        let field = fields.get(field_name).ok_or_else(|| {
            IrisError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        let reader_arc: Arc<dyn VectorFieldReader> = Arc::from(reader);
        field.runtime.replace_reader(reader_arc);
        Ok(())
    }

    /// Reset the reader for a specific field to default.
    pub fn reset_field_reader(&self, field_name: &str) -> Result<()> {
        let fields = self.fields.read();
        let field = fields.get(field_name).ok_or_else(|| {
            IrisError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        field.runtime.reset_reader();
        Ok(())
    }

    /// Materialize the delegate reader for a field (build persistent index).
    pub fn materialize_delegate_reader(&self, field_name: &str) -> Result<()> {
        let fields = self.fields.read();
        let handle = fields.get(field_name).ok_or_else(|| {
            IrisError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;

        let in_memory = handle
            .field
            .as_any()
            .downcast_ref::<InMemoryVectorField>()
            .ok_or_else(|| {
                IrisError::InvalidOperation(format!(
                    "field '{field_name}' does not support delegate materialization"
                ))
            })?;

        let vectors = in_memory.vector_tuples();
        let config = in_memory.config().clone();
        drop(fields);

        self.write_field_delegate_index(field_name, &config, vectors)?;
        let reader = self.load_delegate_reader(field_name, &config)?;

        let fields = self.fields.read();
        let handle = fields.get(field_name).ok_or_else(|| {
            IrisError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        handle.runtime.replace_reader(reader);
        Ok(())
    }

    pub(crate) fn upsert_document_by_internal_id(&self, doc_id: u64, doc: Document) -> Result<()> {
        self.upsert_document_with_id(doc_id, doc)
    }

    pub(crate) fn get_document_by_internal_id(&self, doc_id: u64) -> Result<Option<Document>> {
        self.doc_store.read().get_document(doc_id)
    }

    pub(crate) fn delete_document_by_internal_id(&self, doc_id: u64) -> Result<()> {
        let doc = self
            .get_document_by_internal_id(doc_id)?
            .ok_or_else(|| IrisError::not_found(format!("doc_id {doc_id}")))?;

        self.delete_fields_for_doc(doc_id, &doc)?;
        self.doc_store.write().delete_document(doc_id)?;
        self.deletion_manager
            .delete_document("global", doc_id, "user_request")?;

        self.persist_state()?;
        Ok(())
    }

    pub fn searcher(&self) -> Result<Box<dyn crate::vector::search::searcher::VectorSearcher>> {
        Ok(Box::new(VectorStoreSearcher::from_engine_ref(self)))
    }

    pub fn search(&self, mut request: VectorSearchRequest) -> Result<VectorSearchResults> {
        for query_payload in std::mem::take(&mut request.query_payloads) {
            let mut qv = self.embed_query_payload(&query_payload.field, query_payload.payload)?;
            qv.weight = query_payload.weight;
            request.query_vectors.push(qv);
        }

        let searcher = self.searcher()?;
        searcher.search(&request)
    }

    pub fn commit(&self) -> Result<()> {
        self.check_closed()?;
        self.flush()?;
        self.persist_state()
    }

    pub fn stats(&self) -> Result<VectorStats> {
        let fields = self.fields.read();
        let mut field_stats = HashMap::with_capacity(fields.len());
        for (name, field) in fields.iter() {
            let stats = field.runtime.reader().stats()?;
            field_stats.insert(name.clone(), stats);
        }

        let mut doc_count = 0;
        for segment in self.doc_store.read().segments() {
            doc_count += segment.doc_count as usize;
        }

        if let Some(bitmap) = self.deletion_manager.get_bitmap("global") {
            doc_count =
                doc_count.saturating_sub(bitmap.deleted_count.load(Ordering::SeqCst) as usize);
        }

        Ok(VectorStats {
            document_count: doc_count,
            fields: field_stats,
        })
    }

    pub fn storage(&self) -> &Arc<dyn Storage> {
        &self.storage
    }

    pub fn close(&self) -> Result<()> {
        self.closed.store(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) == 1
    }

    pub fn set_last_wal_seq(&self, seq: u64) {
        self.snapshot_wal_seq.store(seq, Ordering::SeqCst);
    }

    pub fn last_wal_seq(&self) -> u64 {
        self.snapshot_wal_seq.load(Ordering::SeqCst)
    }

    pub fn optimize(&self) -> Result<()> {
        let fields = self.fields.read();
        for field_entry in fields.values() {
            field_entry.field.optimize()?;
        }
        Ok(())
    }

    fn embed_query_payload(&self, field_name: &str, value: DataValue) -> Result<QueryVector> {
        let embedder = self.embedder_registry.resolve(field_name)?;
        let executor = self.ensure_embedder_executor()?;

        let result_vec = match value {
            DataValue::Text(t) => {
                let embedder_owned = embedder.clone();
                executor.run(async move {
                    let input = crate::embedding::embedder::EmbedInput::Text(&t);
                    embedder_owned.embed(&input).await
                })?
            }
            DataValue::Bytes(b, m) => {
                let embedder_owned = embedder.clone();
                executor.run(async move {
                    let input = crate::embedding::embedder::EmbedInput::Bytes(&b, m.as_deref());
                    embedder_owned.embed(&input).await
                })?
            }
            DataValue::Vector(v) => Vector::new(v),
            _ => {
                return Err(IrisError::invalid_argument(format!(
                    "unsupported query data type for field '{field_name}'"
                )));
            }
        };

        Ok(QueryVector {
            vector: result_vec.data,
            weight: 1.0,
            fields: None,
        })
    }
}

/// Searcher implementation for [`VectorStore`].
#[derive(Debug)]
pub struct VectorStoreSearcher {
    config: Arc<VectorIndexConfig>,
    fields: Arc<RwLock<HashMap<String, FieldHandle>>>,
    _doc_store: Arc<RwLock<UnifiedDocumentStore>>,
}

impl VectorStoreSearcher {
    pub fn from_engine_ref(engine: &VectorStore) -> Self {
        Self {
            config: Arc::clone(&engine.config),
            fields: Arc::clone(&engine.fields),
            _doc_store: Arc::clone(&engine.doc_store),
        }
    }

    fn resolve_fields(&self, request: &VectorSearchRequest) -> Result<Vec<String>> {
        match &request.fields {
            Some(selectors) => self.apply_field_selectors(selectors),
            None => Ok(self.config.default_fields.clone()),
        }
    }

    fn apply_field_selectors(&self, selectors: &[FieldSelector]) -> Result<Vec<String>> {
        let fields = self.fields.read();
        let mut result = Vec::new();

        for selector in selectors {
            match selector {
                FieldSelector::Exact(name) => {
                    if fields.contains_key(name) {
                        if !result.contains(name) {
                            result.push(name.clone());
                        }
                    } else {
                        return Err(IrisError::not_found(format!(
                            "vector field '{name}' is not registered",
                        )));
                    }
                }
                FieldSelector::Prefix(prefix) => {
                    for field_name in fields.keys() {
                        if field_name.starts_with(prefix) && !result.contains(field_name) {
                            result.push(field_name.clone());
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    fn build_filter_matches(
        &self,
        request: &VectorSearchRequest,
        _target_fields: &[String],
    ) -> Result<Option<HashSet<u64>>> {
        if let Some(ids) = &request.allowed_ids {
            return Ok(Some(ids.iter().copied().collect()));
        }
        Ok(None)
    }

    fn scaled_field_limit(&self, limit: usize, overfetch: f32) -> usize {
        ((limit as f32) * overfetch).ceil() as usize
    }

    fn query_vectors_for_field(
        &self,
        field_name: &str,
        _config: &VectorFieldConfig,
        request: &VectorSearchRequest,
    ) -> Vec<QueryVector> {
        let mut result = Vec::new();
        for candidate in &request.query_vectors {
            let include = if let Some(fields) = &candidate.fields {
                fields.iter().any(|f| f.as_str() == field_name)
            } else {
                true
            };

            if include {
                result.push(candidate.clone());
            }
        }
        result
    }

    fn merge_field_hits(
        &self,
        doc_hits: &mut HashMap<u64, VectorHit>,
        hits: Vec<FieldHit>,
        field_weight: f32,
        score_mode: VectorScoreMode,
        allowed_ids: Option<&HashSet<u64>>,
    ) -> Result<()> {
        // existence check is skipped for performance, assuming vector index is in sync.
        for hit in hits {
            if let Some(allowed) = allowed_ids {
                if !allowed.contains(&hit.doc_id) {
                    continue;
                }
            }

            let weighted_score = hit.score * field_weight;
            let entry = doc_hits.entry(hit.doc_id).or_insert_with(|| VectorHit {
                doc_id: hit.doc_id,
                score: 0.0,
                field_hits: Vec::new(),
            });

            match score_mode {
                VectorScoreMode::WeightedSum => {
                    entry.score += weighted_score;
                }
                VectorScoreMode::MaxSim => {
                    entry.score = entry.score.max(weighted_score);
                }
                VectorScoreMode::LateInteraction => {
                    return Err(IrisError::invalid_argument(
                        "VectorScoreMode::LateInteraction is not supported yet",
                    ));
                }
            }
            entry.field_hits.push(hit);
        }

        Ok(())
    }
}

impl crate::vector::search::searcher::VectorSearcher for VectorStoreSearcher {
    fn search(&self, request: &VectorSearchRequest) -> Result<VectorSearchResults> {
        if request.query_vectors.is_empty() && request.lexical_query.is_none() {
            return Err(IrisError::invalid_argument(
                "VectorSearchRequest requires at least one query vector or lexical query",
            ));
        }

        if request.limit == 0 {
            return Ok(VectorSearchResults::default());
        }

        if request.overfetch < 1.0 {
            return Err(IrisError::invalid_argument(
                "VectorSearchRequest overfetch must be >= 1.0",
            ));
        }

        let mut vector_hits_map: HashMap<u64, VectorHit> = HashMap::new();

        if !request.query_vectors.is_empty() {
            let target_fields = self.resolve_fields(request)?;
            let allowed_ids = self.build_filter_matches(request, &target_fields)?;

            if allowed_ids.as_ref().is_some_and(|ids| ids.is_empty())
                && request.lexical_query.is_none()
            {
                return Ok(VectorSearchResults::default());
            }

            let field_limit = self.scaled_field_limit(request.limit, request.overfetch);
            let fields = self.fields.read();

            for field_name in target_fields {
                let field = fields
                    .get(&field_name)
                    .ok_or_else(|| IrisError::not_found(format!("vector field '{field_name}'")))?;
                let matching_vectors =
                    self.query_vectors_for_field(&field_name, field.field.config(), request);
                if matching_vectors.is_empty() {
                    continue;
                }

                let field_query = FieldSearchInput {
                    field: field_name.clone(),
                    query_vectors: matching_vectors,
                    limit: field_limit,
                    allowed_ids: allowed_ids.clone(),
                };

                let field_results = field.runtime.reader().search(field_query)?;
                let field_weight = field
                    .field
                    .config()
                    .vector
                    .as_ref()
                    .map(|v| v.base_weight())
                    .unwrap_or(1.0);

                self.merge_field_hits(
                    &mut vector_hits_map,
                    field_results.hits,
                    field_weight,
                    request.score_mode,
                    allowed_ids.as_ref(),
                )?;
            }
        }

        let mut hits = Vec::with_capacity(vector_hits_map.len());
        for hit in vector_hits_map.values() {
            hits.push(hit.clone());
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal));

        let mut final_hits = hits;

        if request.min_score > 0.0 {
            final_hits.retain(|hit| hit.score >= request.min_score);
        }

        if final_hits.len() > request.limit {
            final_hits.truncate(request.limit);
        }

        Ok(VectorSearchResults { hits: final_hits })
    }

    fn count(&self, request: &VectorSearchRequest) -> Result<u64> {
        let results = self.search(request)?;
        Ok(results.hits.len() as u64)
    }
}
