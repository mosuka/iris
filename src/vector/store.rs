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

pub mod config;
pub mod embedder;
pub mod filter;
pub mod memory;
pub mod query;
pub mod request;
pub mod response;
pub mod snapshot;

use std::cmp::Ordering as CmpOrdering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// use crate::lexical::store::LexicalStore; // Removed dependency
// use crate::lexical::index::inverted::query::{Query, term::TermQuery}; // Unused
// use crate::lexical::search::searcher::LexicalSearchRequest; // Unused

use parking_lot::{Mutex, RwLock};

use crate::data::DataValue;
use crate::embedding::embedder::{EmbedInput, Embedder};
use crate::embedding::per_field::PerFieldEmbedder;
use crate::error::{IrisError, Result};
use crate::maintenance::deletion::DeletionManager;
use crate::storage::Storage;
use crate::storage::prefixed::PrefixedStorage;
use crate::vector::core::vector::{StoredVector, Vector};
use crate::vector::index::config::{FlatIndexConfig, HnswIndexConfig, IvfIndexConfig};
use crate::vector::index::field::{
    AdapterBackedVectorField, FieldHit, FieldSearchInput, LegacyVectorFieldWriter, VectorField,
    VectorFieldReader, VectorFieldStats, VectorFieldWriter,
};
use crate::vector::index::flat::{
    field_reader::FlatFieldReader, reader::FlatVectorIndexReader, writer::FlatIndexWriter,
};
use crate::vector::index::hnsw::segment::manager::{SegmentManager, SegmentManagerConfig};
use crate::vector::index::hnsw::{
    field_reader::HnswFieldReader, reader::HnswIndexReader, writer::HnswIndexWriter,
};
use crate::vector::index::ivf::{
    field_reader::IvfFieldReader, reader::IvfIndexReader, writer::IvfIndexWriter,
};
use crate::vector::index::segmented_field::SegmentedVectorField;
use crate::vector::writer::{VectorIndexWriter, VectorIndexWriterConfig};

use self::embedder::{EmbedderExecutor, VectorEmbedderRegistry};
// use self::filter::RegistryFilterMatches; // REMOVED
use self::memory::{FieldHandle, FieldRuntime, InMemoryVectorField};
// use self::registry::{DocumentEntry, DocumentVectorRegistry}; // REMOVED

use self::request::{FieldSelector, QueryVector, VectorScoreMode, VectorSearchRequest};
use self::response::{VectorHit, VectorSearchResults, VectorStats};
use self::snapshot::{
    COLLECTION_MANIFEST_FILE, COLLECTION_MANIFEST_VERSION, CollectionManifest,
    DOCUMENT_SNAPSHOT_FILE, DOCUMENT_SNAPSHOT_TEMP_FILE, DocumentSnapshot, FIELD_INDEX_BASENAME,
    REGISTRY_NAMESPACE, SnapshotDocument,
};
use crate::vector::core::field::VectorOption;
use crate::vector::store::config::{VectorFieldConfig, VectorIndexConfig}; // Updated import path

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
    // documents: Arc<RwLock<HashMap<u64, crate::data::Document>>>, // Store might not need to cache full documents if LexicalStore does?
    // Actually VectorStore usually needs to return vector data or payloads?
    // Let's keep it for now but remove IDs management logic.
    documents: Arc<RwLock<HashMap<u64, crate::data::Document>>>,
    next_doc_id: AtomicU64,
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
    pub fn new(storage: Arc<dyn Storage>, config: VectorIndexConfig) -> Result<Self> {
        let embedder_registry = Arc::new(VectorEmbedderRegistry::new());
        let field_configs = Arc::new(RwLock::new(config.fields.clone()));

        // Store the embedder from config before moving config into Arc
        let config_embedder = config.embedder.clone();
        let deletion_config = config.deletion_config.clone();

        let deletion_manager = Arc::new(DeletionManager::new(deletion_config, storage.clone())?);

        let mut store = Self {
            config: Arc::new(config),
            field_configs: field_configs.clone(),
            fields: Arc::new(RwLock::new(HashMap::new())),
            embedder_registry,
            embedder_executor: Mutex::new(None),
            deletion_manager,
            storage,
            documents: Arc::new(RwLock::new(HashMap::new())),
            next_doc_id: AtomicU64::new(0),
            snapshot_wal_seq: AtomicU64::new(0),
            closed: AtomicU64::new(0),
        };

        let shard_id = store.config.shard_id;
        store.deletion_manager.initialize_segment(
            "global",
            crate::util::id::create_doc_id(shard_id, 0),
            crate::util::id::create_doc_id(shard_id, crate::util::id::MAX_LOCAL_ID),
        )?;

        store.load_persisted_state()?; // Needs adjustment to not use metadata index

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

    fn embed_document_payload_internal(
        &self,
        _doc_id: u64,
        payload: crate::data::Document,
    ) -> Result<crate::data::Document> {
        // (Schema validation or implicit registration logic removed or relaxed to allow metadata)

        let mut document = crate::data::Document::new();
        // Copy original fields
        document.fields = payload.fields.clone();

        for (field_name, field_val) in payload.fields.into_iter() {
            // If it's a Text/String or Bytes field and needs embedding, we do it.
            if matches!(
                field_val,
                crate::data::DataValue::Text(_)
                    | crate::data::DataValue::String(_)
                    | crate::data::DataValue::Bytes(_, _)
            ) {
                if let Ok(stored_vec) = self.embed_payload(&field_name, field_val) {
                    document.fields.insert(
                        field_name,
                        crate::data::DataValue::Vector(stored_vec.data.clone()),
                    );
                }
            }
        }

        Ok(document)
    }

    /// Embeds a single `DataValue` into a `StoredVector`.
    fn embed_payload(
        &self,
        field_name: &str,
        value: crate::data::DataValue,
    ) -> Result<StoredVector> {
        let fields = self.fields.read();
        let handle = fields.get(field_name).ok_or_else(|| {
            IrisError::invalid_argument(format!("vector field '{field_name}' is not registered"))
        })?;
        let field_config = handle.field.config().clone();
        drop(fields);

        // Check if vector indexing is enabled for this field
        let dimension = match &field_config.vector {
            Some(opt) => opt.dimension(),
            None => {
                // If not configured for vector indexing, return empty vector
                // This allows the field to be used for lexical indexing without storing vectors
                return Ok(StoredVector::new(Vec::new()));
            }
        };

        match value {
            crate::data::DataValue::Text(text_value) => {
                let executor = self.ensure_embedder_executor()?;
                let embedder = self.embedder_registry.resolve(field_name)?;

                if !embedder.supports_text() {
                    return Err(IrisError::invalid_config(format!(
                        "embedder '{}' does not support text embedding",
                        field_name
                    )));
                }

                let embedder_name_owned = field_name.to_string();
                let text_for_embed = text_value.clone();
                let vector = executor
                    .run(async move { embedder.embed(&EmbedInput::Text(&text_for_embed)).await })?;
                vector.validate_dimension(dimension)?;
                if !vector.is_valid() {
                    return Err(IrisError::InvalidOperation(format!(
                        "embedder '{}' produced invalid values for field '{}'",
                        embedder_name_owned, field_name
                    )));
                }
                let mut stored: StoredVector = vector.into();

                // Store original text if lexical indexing is enabled
                if field_config.lexical.is_some() {
                    use crate::data::DataValue;
                    use crate::lexical::core::field::{Field, FieldOption, TextOption};
                    stored.attributes.insert(
                        "__iris_lexical_source".to_string(),
                        Field::new(
                            DataValue::Text(text_value),
                            FieldOption::Text(TextOption::default()),
                        ),
                    );
                }
                Ok(stored)
            }
            crate::data::DataValue::Bytes(bytes, mime) => {
                let executor = self.ensure_embedder_executor()?;
                let embedder = self.embedder_registry.resolve(field_name)?;

                if !embedder.supports_image() {
                    return Err(IrisError::invalid_config(format!(
                        "embedder '{}' does not support image embedding",
                        field_name
                    )));
                }

                let embedder_name_owned = field_name.to_string();
                let payload_bytes = bytes.clone();
                let mime_hint = mime.clone();
                let vector = executor.run(async move {
                    embedder
                        .embed(&EmbedInput::Bytes(&payload_bytes, mime_hint.as_deref()))
                        .await
                })?;
                let vector_obj = Vector::new(vector.data); // data is Vec<f32>
                if !vector_obj.is_valid() {
                    return Err(IrisError::InvalidOperation(format!(
                        "embedder '{}' produced invalid values for field '{}': {:?}",
                        embedder_name_owned, field_name, vector_obj
                    )));
                }
                let stored: StoredVector = vector_obj.into();
                Ok(stored)
            }
            crate::data::DataValue::Vector(data) => {
                let vector = Vector::new(data.to_vec());
                vector.validate_dimension(dimension)?;
                if !vector.is_valid() {
                    return Err(IrisError::InvalidOperation(format!(
                        "provided vector for field '{}' contains invalid values",
                        field_name
                    )));
                }
                let stored: StoredVector = vector.into();
                Ok(stored)
            }
            _ => Err(IrisError::invalid_argument(
                "unsupported field value for embedding",
            )),
        }
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
            // Skip if already registered
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
        // If no vector config is present, we might want to default to something or handle it.
        // For now, assuming if create_vector_field is called, we expect a vector field
        // or effectively an "empty" vector field.
        // However, existing logic matched on config.index.

        match config.vector {
            Some(VectorOption::Hnsw(_)) => {
                let storage = self.field_storage(&name);
                let manager_config = SegmentManagerConfig::default();
                let segment_manager =
                    Arc::new(SegmentManager::new(manager_config, storage.clone())?);

                Ok(Arc::new(SegmentedVectorField::create(
                    name,
                    config,
                    segment_manager,
                    storage,
                    self.deletion_manager.get_bitmap("global"),
                )?))
            }
            _ => {
                let delegate = self.build_delegate_writer(&name, &config)?;
                Ok(Arc::new(InMemoryVectorField::new(name, config, delegate)?))
            }
        }
    }

    fn build_delegate_writer(
        &self,
        field_name: &str,
        config: &VectorFieldConfig,
    ) -> Result<Option<Arc<dyn VectorFieldWriter>>> {
        let vector_option = match &config.vector {
            Some(opt) => opt,
            None => return Ok(None),
        };

        if vector_option.dimension() == 0 {
            return Ok(None);
        }

        let writer_config = VectorIndexWriterConfig::default();
        let storage = self.field_storage(field_name);

        let delegate: Arc<dyn VectorFieldWriter> = match vector_option {
            VectorOption::Flat(opt) => {
                let flat = FlatIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    embedder: self.config.embedder.clone(), // Pass global embedder or need field specific?
                    ..FlatIndexConfig::default()
                };
                let writer = FlatIndexWriter::with_storage(
                    flat,
                    writer_config.clone(),
                    "vectors.index",
                    storage.clone(),
                )?;
                Arc::new(LegacyVectorFieldWriter::new(field_name.to_string(), writer))
            }
            VectorOption::Hnsw(opt) => {
                let hnsw = HnswIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    m: opt.m,
                    ef_construction: opt.ef_construction,
                    embedder: self.config.embedder.clone(),
                    ..HnswIndexConfig::default()
                };
                let writer = HnswIndexWriter::with_storage(
                    hnsw,
                    writer_config.clone(),
                    "vectors.index",
                    storage.clone(),
                )?;
                Arc::new(LegacyVectorFieldWriter::new(field_name.to_string(), writer))
            }
            VectorOption::Ivf(opt) => {
                let ivf = IvfIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    n_clusters: opt.n_clusters,
                    n_probe: opt.n_probe,
                    embedder: self.config.embedder.clone(),
                    ..IvfIndexConfig::default()
                };
                let writer =
                    IvfIndexWriter::with_storage(ivf, writer_config, "vectors.index", storage)?;
                Arc::new(LegacyVectorFieldWriter::new(field_name.to_string(), writer))
            }
        };
        Ok(Some(delegate))
    }

    fn write_field_delegate_index(
        &self,
        field_name: &str,
        config: &VectorFieldConfig,
        vectors: Vec<(u64, String, Vector)>,
    ) -> Result<()> {
        let vector_option = match &config.vector {
            Some(opt) => opt,
            None => {
                return Err(IrisError::invalid_config(format!(
                    "vector field '{field_name}' validation failed: no vector configuration found"
                )));
            }
        };

        if vector_option.dimension() == 0 {
            return Err(IrisError::invalid_config(format!(
                "vector field '{field_name}' cannot materialize a zero-dimension index"
            )));
        }

        let storage = self.field_storage(field_name);
        let mut pending_vectors = Some(vectors);

        match vector_option {
            VectorOption::Flat(opt) => {
                let flat = FlatIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    embedder: self.config.embedder.clone(),
                    ..FlatIndexConfig::default()
                };
                let mut writer = FlatIndexWriter::with_storage(
                    flat,
                    VectorIndexWriterConfig::default(),
                    FIELD_INDEX_BASENAME,
                    storage.clone(),
                )?;
                let vectors = pending_vectors.take().unwrap_or_default();
                writer.build(vectors)?;
                writer.finalize()?;
                writer.write()?;
            }
            VectorOption::Hnsw(opt) => {
                let hnsw = HnswIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    m: opt.m,
                    ef_construction: opt.ef_construction,
                    embedder: self.config.embedder.clone(),
                    ..HnswIndexConfig::default()
                };
                let mut writer = HnswIndexWriter::with_storage(
                    hnsw,
                    VectorIndexWriterConfig::default(),
                    FIELD_INDEX_BASENAME,
                    storage.clone(),
                )?;
                let vectors = pending_vectors.take().unwrap_or_default();
                writer.build(vectors)?;
                writer.finalize()?;
                writer.write()?;
            }
            VectorOption::Ivf(opt) => {
                let ivf = IvfIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    n_clusters: opt.n_clusters,
                    n_probe: opt.n_probe,
                    embedder: self.config.embedder.clone(),
                    ..IvfIndexConfig::default()
                };
                let mut writer = IvfIndexWriter::with_storage(
                    ivf,
                    VectorIndexWriterConfig::default(),
                    FIELD_INDEX_BASENAME,
                    storage.clone(),
                )?;
                let vectors = pending_vectors.take().unwrap_or_default();
                writer.build(vectors)?;
                writer.finalize()?;
                writer.write()?;
            }
        }

        Ok(())
    }

    fn load_delegate_reader(
        &self,
        field_name: &str,
        config: &VectorFieldConfig,
    ) -> Result<Arc<dyn VectorFieldReader>> {
        let storage = self.field_storage(field_name);

        let global_bitmap = self.deletion_manager.get_bitmap("global");

        let vector_option = match &config.vector {
            Some(opt) => opt,
            None => {
                return Err(IrisError::invalid_config(format!(
                    "vector field '{field_name}' has no vector configuration"
                )));
            }
        };

        Ok(match vector_option {
            VectorOption::Flat(opt) => {
                let flat_config = crate::vector::index::config::FlatIndexConfig {
                    dimension: opt.dimension,
                    distance_metric: opt.distance,
                    loading_mode: crate::vector::index::config::IndexLoadingMode::default(),
                    ..Default::default()
                };
                let mut reader = FlatVectorIndexReader::load(
                    &*storage,
                    FIELD_INDEX_BASENAME,
                    flat_config.distance_metric,
                )?;
                if let Some(bitmap) = &global_bitmap {
                    reader.set_deletion_bitmap(bitmap.clone());
                }
                let reader = Arc::new(reader);
                Arc::new(FlatFieldReader::new(field_name.to_string(), reader))
            }
            VectorOption::Hnsw(opt) => {
                let mut reader =
                    HnswIndexReader::load(&*storage, FIELD_INDEX_BASENAME, opt.distance)?;
                if let Some(bitmap) = &global_bitmap {
                    reader.set_deletion_bitmap(bitmap.clone());
                }
                let reader = Arc::new(reader);
                Arc::new(HnswFieldReader::new(field_name.to_string(), reader))
            }
            VectorOption::Ivf(opt) => {
                let mut reader =
                    IvfIndexReader::load(&*storage, FIELD_INDEX_BASENAME, opt.distance)?;
                if let Some(bitmap) = &global_bitmap {
                    reader.set_deletion_bitmap(bitmap.clone());
                }
                let reader = Arc::new(reader);
                Arc::new(IvfFieldReader::with_n_probe(
                    field_name.to_string(),
                    reader,
                    opt.n_probe,
                ))
            }
        })
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

    fn validate_document_fields(&self, _document: &crate::data::Document) -> Result<()> {
        // Relaxed: allow metadata fields that are not registered as vector fields.
        Ok(())
    }

    fn bump_next_doc_id(&self, doc_id: u64) {
        let _ = self
            .next_doc_id
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                if doc_id >= current {
                    Some(doc_id.saturating_add(1))
                } else {
                    None
                }
            });
    }

    fn recompute_next_doc_id(&self) {
        let max_id = self.documents.read().keys().copied().max().unwrap_or(0);
        self.bump_next_doc_id(max_id);
    }

    fn delete_fields_for_doc(&self, doc_id: u64, doc: &crate::data::Document) -> Result<()> {
        let fields = self.fields.read();
        for field_name in doc.fields.keys() {
            let field = fields.get(field_name).ok_or_else(|| {
                IrisError::not_found(format!(
                    "vector field '{field_name}' not registered during delete"
                ))
            })?;
            field.runtime.writer().delete_document(doc_id, 0)?; // Use version 0 as we don't track per-field version
        }

        // Logical deletion in bitmap
        self.deletion_manager
            .delete_document("global", doc_id, "delete_request")?;

        Ok(())
    }

    fn apply_field_updates(
        &self,
        doc_id: u64,
        version: u64,
        fields_data: &HashMap<String, DataValue>,
    ) -> Result<()> {
        let fields = self.fields.read();
        for (field_name, val) in fields_data {
            if let Some(stored_vector) = val.as_vector_ref() {
                let field = fields.get(field_name).ok_or_else(|| {
                    IrisError::not_found(format!("vector field '{field_name}' is not registered"))
                })?;
                // Convert Vec<f32> to StoredVector
                let sv = StoredVector::new(stored_vector.clone());
                field
                    .runtime
                    .writer()
                    .add_stored_vector(doc_id, &sv, version)?;
            }
        }
        Ok(())
    }

    fn load_persisted_state(&mut self) -> Result<()> {
        let storage = self.registry_storage();
        // Registry snapshot loading is replaced by LexicalStore persistence.
        // LexicalStore handles its own persistence automatically.

        self.load_document_snapshot(storage.clone())?;
        self.load_collection_manifest(storage.clone())?;
        // Instantiate fields after manifest load so that persisted implicit fields are registered
        self.instantiate_configured_fields()?;

        // Populate fields from loaded documents (restore in-memory state)
        // Only for fields that don't handle their own persistence (e.g. Flat/InMemory)
        {
            let configs = self.field_configs.read();
            let hydration_needed: std::collections::HashSet<String> = configs
                .iter()
                .filter(|(_, cfg)| {
                    matches!(
                        cfg.vector,
                        Some(crate::vector::core::field::VectorOption::Flat(_))
                    )
                })
                .map(|(k, _)| k.clone())
                .collect();
            drop(configs); // Drop lock

            if !hydration_needed.is_empty() {
                let documents = self.documents.read();
                for (doc_id, doc) in documents.iter() {
                    let mut vector_fields = HashMap::new();
                    for (name, val) in &doc.fields {
                        if hydration_needed.contains(name) {
                            if let crate::data::DataValue::Vector(_) = val {
                                vector_fields.insert(name.clone(), val.clone());
                            }
                        }
                    }
                    if !vector_fields.is_empty() {
                        // We treat snapshot loading as version 0 updates
                        self.apply_field_updates(*doc_id, 0, &vector_fields)?;
                    }
                }
            }
        }

        self.recompute_next_doc_id();
        self.persist_manifest()
    }

    fn load_document_snapshot(&self, storage: Arc<dyn Storage>) -> Result<()> {
        if !storage.file_exists(DOCUMENT_SNAPSHOT_FILE) {
            self.documents.write().clear();
            self.snapshot_wal_seq.store(0, Ordering::SeqCst);
            return Ok(());
        }

        let mut input = storage.open_input(DOCUMENT_SNAPSHOT_FILE)?;
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer)?;
        input.close()?;

        if buffer.is_empty() {
            self.documents.write().clear();
            self.snapshot_wal_seq.store(0, Ordering::SeqCst);
            return Ok(());
        }

        // Legacy format with FieldVectors (multiple vectors per field).
        #[derive(serde::Deserialize)]
        struct LegacyFieldVectors {
            #[serde(default)]
            vectors: Vec<StoredVector>,
        }

        #[derive(serde::Deserialize)]
        struct LegacySnapshotDocument {
            doc_id: u64,
            #[serde(default)]
            fields: HashMap<String, LegacyFieldVectors>,
            #[serde(default)]
            metadata: HashMap<String, String>,
        }

        let snapshot = match serde_json::from_slice::<DocumentSnapshot>(&buffer) {
            Ok(snapshot) => snapshot,
            Err(primary_err) => {
                let docs: Vec<LegacySnapshotDocument> =
                    serde_json::from_slice(&buffer).map_err(|_| primary_err)?;
                let converted = docs
                    .into_iter()
                    .map(|legacy| {
                        // Convert FieldVectors to StoredVector (take first vector only).
                        let fields = legacy
                            .fields
                            .into_iter()
                            .filter_map(|(name, fv)| {
                                fv.vectors.into_iter().next().map(|v| (name, v))
                            })
                            .collect::<Vec<(String, StoredVector)>>();
                        SnapshotDocument {
                            doc_id: legacy.doc_id,
                            document: {
                                let mut doc = crate::data::Document::new();
                                for (k, v) in fields {
                                    let vec_data: Vec<f32> = v.data.iter().copied().collect();
                                    doc =
                                        doc.with_field(k, crate::data::DataValue::Vector(vec_data));
                                }
                                for (k, v) in legacy.metadata {
                                    doc = doc.with_field(k, crate::data::DataValue::Text(v));
                                }
                                doc
                            },
                        }
                    })
                    .collect();
                DocumentSnapshot {
                    last_wal_seq: 0,
                    documents: converted,
                }
            }
        };
        let map = snapshot
            .documents
            .into_iter()
            .map(|doc| (doc.doc_id, doc.document))
            .collect();
        *self.documents.write() = map;
        self.snapshot_wal_seq
            .store(snapshot.last_wal_seq, Ordering::SeqCst);
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

        let snapshot_seq = self.snapshot_wal_seq.load(Ordering::SeqCst);
        if manifest.snapshot_wal_seq != snapshot_seq {
            return Err(IrisError::invalid_config(format!(
                "collection manifest snapshot sequence {} does not match persisted snapshot {}",
                manifest.snapshot_wal_seq, snapshot_seq
            )));
        }

        if manifest.wal_last_seq < manifest.snapshot_wal_seq {
            return Err(IrisError::invalid_config(
                "collection manifest WAL sequence regressed",
            ));
        }

        if !manifest.field_configs.is_empty() {
            *self.field_configs.write() = manifest.field_configs.clone();
        }

        Ok(())
    }

    pub fn flush_vectors(&self) -> Result<()> {
        let fields = self.fields.read();
        for field_entry in fields.values() {
            field_entry.runtime.writer().flush()?;
        }
        Ok(())
    }

    fn persist_state(&self) -> Result<()> {
        // Registry is removed, Lexical persists itself on commit (called in commit method)
        // self.persist_registry_snapshot()?;
        self.persist_document_snapshot()?;
        // WAL is self-persisting (but no longer used internally)
        self.persist_manifest()
    }

    // fn persist_registry_snapshot(&self) -> Result<()> { ... } REMOVED

    fn persist_document_snapshot(&self) -> Result<()> {
        let storage = self.registry_storage();
        let guard = self.documents.read();
        let documents: Vec<SnapshotDocument> = guard
            .iter()
            .map(|(doc_id, document)| SnapshotDocument {
                doc_id: *doc_id,
                document: document.clone(),
            })
            .collect();
        drop(guard);
        let snapshot = DocumentSnapshot {
            last_wal_seq: self.snapshot_wal_seq.load(Ordering::SeqCst),
            documents,
        };
        let serialized = serde_json::to_vec(&snapshot)?;

        if serialized.len() > 256 * 1024 {
            self.write_atomic(storage.clone(), DOCUMENT_SNAPSHOT_TEMP_FILE, &serialized)?;
            storage.delete_file(DOCUMENT_SNAPSHOT_FILE).ok();
            storage.rename_file(DOCUMENT_SNAPSHOT_TEMP_FILE, DOCUMENT_SNAPSHOT_FILE)?;
        } else {
            self.write_atomic(storage.clone(), DOCUMENT_SNAPSHOT_FILE, &serialized)?;
        }

        self.snapshot_wal_seq
            .store(snapshot.last_wal_seq, Ordering::SeqCst);
        Ok(())
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

    /// Upsert a document    // Internal helper for upserting/indexing
    fn upsert_document_payload(&self, payload: crate::data::Document) -> Result<u64> {
        let doc = self.embed_document_payload_internal(0, payload)?;
        let doc_id = self.next_doc_id.fetch_add(1, Ordering::SeqCst);
        self.upsert_vectors(doc_id, doc)?;
        Ok(doc_id)
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

    /// Get the index configuration.
    pub fn config(&self) -> &VectorIndexConfig {
        self.config.as_ref()
    }

    /// Get the embedder for this engine.
    pub fn embedder(&self) -> Arc<dyn Embedder> {
        Arc::clone(self.config.get_embedder())
    }

    /// Add a document to the collection.
    ///
    /// Returns the assigned document ID.
    pub fn add_document(&self, doc: crate::data::Document) -> Result<u64> {
        self.check_closed()?;
        let doc_id = self.next_doc_id.fetch_add(1, Ordering::SeqCst);
        self.upsert_vectors(doc_id, doc)?;
        Ok(doc_id)
    }

    /// Add multiple documents with automatically assigned doc_ids.
    pub fn add_documents(
        &self,
        docs: impl IntoIterator<Item = crate::data::Document>,
    ) -> Result<Vec<u64>> {
        docs.into_iter().map(|doc| self.add_document(doc)).collect()
    }

    /// Add a document from payload (will be embedded if configured).
    ///
    /// Returns the assigned document ID.
    pub fn add_payloads(&self, payload: crate::data::Document) -> Result<u64> {
        self.upsert_document_payload(payload)
    }

    /// Add or update a document for an external ID.
    pub fn index_document(&self, external_id: &str, mut doc: crate::data::Document) -> Result<u64> {
        self.check_closed()?;
        doc.fields.insert(
            "_id".to_string(),
            crate::data::DataValue::Text(external_id.to_string()),
        );
        let doc_id = self.next_doc_id.fetch_add(1, Ordering::SeqCst);
        self.upsert_vectors(doc_id, doc)?;
        Ok(doc_id)
    }

    /// Add multiple vectors with automatically assigned doc_ids.
    pub fn add_vectors_batch(
        &self,
        docs: impl IntoIterator<Item = crate::data::Document>,
    ) -> Result<Vec<u64>> {
        docs.into_iter().map(|doc| self.add_document(doc)).collect()
    }

    /// Index multiple payloads with automatically assigned doc_ids.
    pub fn index_payloads_batch(&self, payloads: Vec<crate::data::Document>) -> Result<Vec<u64>> {
        let mut doc_ids = Vec::with_capacity(payloads.len());
        for payload in payloads {
            // Note: calling add_payloads (which calls upsert_document_payload).
            // This is NOT chunk mode, it's standard add.
            let doc_id = self.add_payloads(payload)?;
            doc_ids.push(doc_id);
        }
        Ok(doc_ids)
    }

    /// Index a single document payload as a new chunk, sharing the same external ID.
    /// This bypasses the overwrite check and always appends.
    pub fn index_payload_chunk(&self, payload: crate::data::Document) -> Result<u64> {
        self.check_closed()?;
        // Generate internal ID
        let doc_id = self.next_doc_id.fetch_add(1, Ordering::SeqCst);
        self.upsert_vectors(doc_id, payload)?;
        Ok(doc_id)
    }

    /// Index multiple document payloads as chunks.
    pub fn index_payloads_chunk(&self, payloads: Vec<crate::data::Document>) -> Result<Vec<u64>> {
        let mut doc_ids = Vec::with_capacity(payloads.len());
        for payload in payloads {
            let doc_id = self.index_payload_chunk(payload)?;
            doc_ids.push(doc_id);
        }
        Ok(doc_ids)
    }

    // Internal helper for indexing
    fn upsert_document_with_id(&self, doc_id: u64, document: crate::data::Document) -> Result<()> {
        self.check_closed()?;
        self.validate_document_fields(&document)?;

        // 1. Extract Vectors
        let mut vector_fields = HashMap::new();
        for (name, val) in &document.fields {
            if let crate::data::DataValue::Vector(_) = val {
                vector_fields.insert(name.clone(), val.clone());
            }
        }

        // 2. Update In-Memory Structures
        // Update document store
        self.documents.write().insert(doc_id, document.clone());

        // Update Field Writers
        self.apply_field_updates(doc_id, 0, &vector_fields)?;

        Ok(())
    }

    /// Upsert a document with a specific document ID.
    pub fn upsert_vectors(&self, doc_id: u64, doc: crate::data::Document) -> Result<()> {
        // Embed first
        let embedded_doc = self.embed_document_payload_internal(doc_id, doc)?;
        self.upsert_document_with_id(doc_id, embedded_doc)
    }

    /// Upsert a document from payload (will be embedded if configured).
    pub fn upsert_payloads(&self, _doc_id: u64, payload: crate::data::Document) -> Result<()> {
        self.upsert_document_payload(payload)?;
        Ok(())
    }

    /// Get a document by its internal ID.
    pub fn get_document(&self, doc_id: u64) -> Result<Option<crate::data::Document>> {
        Ok(self.documents.read().get(&doc_id).cloned())
    }

    /// Delete a document by ID.
    /// Delete a document by ID.
    pub fn delete_vectors(&self, doc_id: u64) -> Result<()> {
        let documents = self.documents.read();
        let doc = documents
            .get(&doc_id)
            .ok_or_else(|| IrisError::not_found(format!("doc_id {doc_id}")))?;

        self.delete_fields_for_doc(doc_id, doc)?;

        if let Some(_ext_id) = doc.get("_id").and_then(|v| v.as_text()) {
            // self.metadata_index.delete_document_by_id(ext_id)?;
        }
        drop(documents);

        // self.wal.append(&WalEntry::Delete { doc_id })?; // Removed
        self.documents.write().remove(&doc_id);

        // WAL is durable on append, so we don't need full persist_state here
        // But we might want to update snapshots periodically? For now, keep it simple.
        self.persist_state()?; // Still need to update registry/doc snapshots if we want them in sync
        Ok(())
    }

    /// Embed a document payload into vectors.
    pub fn embed_document_payload(
        &self,
        payload: crate::data::Document,
    ) -> Result<crate::data::Document> {
        // Just embed, don't upsert. 0 is dummy id.
        self.embed_document_payload_internal(0, payload)
    }

    /// Embed a payload for query.
    pub fn embed_query_payload(
        &self,
        field_name: &str,
        value: crate::data::DataValue,
    ) -> Result<QueryVector> {
        let stored = self.embed_payload(field_name, value)?;
        Ok(QueryVector {
            vector: stored.data,
            weight: 1.0,
            fields: None, // Will be set by caller if needed
        })
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

    pub fn set_last_wal_seq(&self, seq: u64) {
        self.snapshot_wal_seq.store(seq, Ordering::SeqCst);
    }

    pub fn last_wal_seq(&self) -> u64 {
        self.snapshot_wal_seq.load(Ordering::SeqCst)
    }

    /// Create a searcher for this engine.
    pub fn searcher(&self) -> Result<Box<dyn crate::vector::search::searcher::VectorSearcher>> {
        Ok(Box::new(VectorStoreSearcher::from_engine_ref(self)))
    }

    /// Execute a search query.
    pub fn search(&self, mut request: VectorSearchRequest) -> Result<VectorSearchResults> {
        // Embed query_payloads and add to query_vectors.
        for query_payload in std::mem::take(&mut request.query_payloads) {
            let mut qv = self.embed_query_payload(&query_payload.field, query_payload.payload)?;
            qv.weight = query_payload.weight;
            request.query_vectors.push(qv);
        }

        let searcher = self.searcher()?;
        searcher.search(&request)
    }

    /// Count documents matching the search criteria.
    pub fn count(&self, mut request: VectorSearchRequest) -> Result<u64> {
        // Embed query_payloads and add to query_vectors.
        for query_payload in std::mem::take(&mut request.query_payloads) {
            let mut qv = self.embed_query_payload(&query_payload.field, query_payload.payload)?;
            qv.weight = query_payload.weight;
            request.query_vectors.push(qv);
        }

        let searcher = self.searcher()?;
        searcher.count(&request)
    }

    /// Commit pending changes (persist state).
    pub fn commit(&self) -> Result<()> {
        // Commit all fields
        let fields = self.fields.read();
        for field in fields.values() {
            // This is crucial: writer buffers must be flushed to reader/storage
            field.runtime.writer().flush()?;
            // And refresh reader view??
            // Writers usually refresh readers on commit implicitly or we need explicit refresh.
            // Let's assume commit is enough for durability/visibility if writer handles it.
        }
        drop(fields);

        // self.metadata_index.commit()?;
        self.persist_state()
    }

    /// Get collection statistics.
    pub fn stats(&self) -> Result<VectorStats> {
        let fields = self.fields.read();
        let mut field_stats = HashMap::with_capacity(fields.len());
        for (name, field) in fields.iter() {
            let stats = field.runtime.reader().stats()?;
            field_stats.insert(name.clone(), stats);
        }

        // let stats = self.metadata_index.stats()?;
        // let doc_count = stats.doc_count.saturating_sub(stats.deleted_count);
        let doc_count = self.documents.read().len(); // Approximate count from memory for now

        Ok(VectorStats {
            document_count: doc_count as usize,
            fields: field_stats,
        })
    }

    /// Get the storage backend.
    pub fn storage(&self) -> &Arc<dyn Storage> {
        &self.storage
    }

    /// Close the collection and release resources.
    pub fn close(&self) -> Result<()> {
        self.closed.store(1, Ordering::SeqCst);
        // self.metadata_index.close()?;
        Ok(())
    }

    /// Check if the collection is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) == 1
    }

    /// Optimize the vector index.
    ///
    /// This triggers optimization (e.g., segment merging, index rebuild) for all registered fields.
    pub fn optimize(&self) -> Result<()> {
        let fields = self.fields.read();

        for (_field_name, field_entry) in fields.iter() {
            field_entry.field.optimize()?;
        }

        Ok(())
    }
}

/// Searcher implementation for [`VectorStore`].
#[derive(Debug)]
pub struct VectorStoreSearcher {
    config: Arc<VectorIndexConfig>,
    fields: Arc<RwLock<HashMap<String, FieldHandle>>>,
    // metadata_index: Arc<LexicalStore>, // Removed
    documents: Arc<RwLock<HashMap<u64, crate::data::Document>>>,
}

impl VectorStoreSearcher {
    /// Create a new searcher from an engine reference.
    pub fn from_engine_ref(engine: &VectorStore) -> Self {
        Self {
            config: Arc::clone(&engine.config),
            fields: Arc::clone(&engine.fields),
            // metadata_index: Arc::clone(&engine.metadata_index),
            documents: Arc::clone(&engine.documents),
        }
    }

    /// Resolve which fields to search based on the request.
    fn resolve_fields(&self, request: &VectorSearchRequest) -> Result<Vec<String>> {
        match &request.fields {
            Some(selectors) => self.apply_field_selectors(selectors),
            None => Ok(self.config.default_fields.clone()),
        }
    }

    /// Apply field selectors to determine which fields to search.
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
        // 1. Unified Filtering (via allowed_ids from Engine)
        if let Some(ids) = &request.allowed_ids {
            return Ok(Some(ids.iter().copied().collect()));
        }

        // 2. Legacy Filtering (VectorStore internal) - Disabled
        if let Some(f) = &request.filter {
            if !crate::vector::store::filter::VectorFilter::is_empty(f) {
                return Err(IrisError::NotImplemented(
                    "VectorStore internal filtering is disabled. Use Engine to filter.".to_string(),
                ));
            }
        }

        Ok(None)
    }

    /// Get the scaled field limit based on overfetch factor.
    fn scaled_field_limit(&self, limit: usize, overfetch: f32) -> usize {
        ((limit as f32) * overfetch).ceil() as usize
    }

    /// Get query vectors that match a specific field.
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

    /// Merge field hits into document hits.
    fn merge_field_hits(
        &self,
        doc_hits: &mut HashMap<u64, VectorHit>,
        hits: Vec<FieldHit>,
        field_weight: f32,
        score_mode: VectorScoreMode,
        allowed_ids: Option<&HashSet<u64>>,
    ) -> Result<()> {
        let _doc_ids: Vec<u64> = hits.iter().map(|h| h.doc_id).collect();
        // Since we don't have a cheap in-memory "exists" check other than documents map,
        // and we assume the searcher's index is consistent with metadata_index,
        // we might skip explicit existence check OR use documents map.
        // Using documents map is safe because it reflects the current state including WAL.
        let documents = self.documents.read();

        for hit in hits {
            if !documents.contains_key(&hit.doc_id) {
                continue;
            }

            if let Some(allowed) = allowed_ids
                && !allowed.contains(&hit.doc_id)
            {
                continue;
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
    // search_lexical removed. VectorStore only supports vector search.
    // Hybrid search is handled by the Unified Engine.
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

        if matches!(request.score_mode, VectorScoreMode::LateInteraction) {
            return Err(IrisError::invalid_argument(
                "VectorScoreMode::LateInteraction is not supported yet",
            ));
        }

        // 1. Vector Search
        let mut vector_hits_map: HashMap<u64, VectorHit> = HashMap::new();
        let mut fields_with_queries = 0_usize;

        if !request.query_vectors.is_empty() {
            let target_fields = self.resolve_fields(request)?;
            let allowed_ids = self.build_filter_matches(request, &target_fields)?;

            // If filter allows no IDs, and we only have vector search, we can return empty.
            // BUT if we have lexical search, we should proceed (lexical search handles its own filtering?
            // or should we pass allowed_ids to lexical search? LexicalStore might not accept generic ID list easily yet).
            // For now, assume lexical search is independent of vector-specific filtering logic unless filters are global.
            // VectorFilter applies to metadata, so it SHOULD apply to lexical search too.
            // TODO: Apply filter to lexical search.

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

                fields_with_queries += 1;

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

        // Check if we did any vector search
        let _vector_search_performed = fields_with_queries > 0;

        // 2. Lexical Search - DISABLED in VectorStore
        // VectorStore no longer performs lexical search.
        if request.lexical_query.is_some() {
            return Err(IrisError::NotImplemented(
                "VectorStore lexical search is disabled. Use Unified Engine.".to_string(),
            ));
        }

        // 3. Fusion - DISABLED/SIMPLIFIED
        // Since we have no lexical hits, we just return vector hits sorted.

        let mut hits = Vec::with_capacity(vector_hits_map.len());
        for hit in vector_hits_map.values() {
            hits.push(hit.clone());
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal));

        // 4. Post-processing (sort & limit)
        // Already handled in merge_results usually, but let's double check sort
        // merge_results should return sorted vector.

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
        if request.query_vectors.is_empty() {
            let documents = self.documents.read();
            return Ok(documents.len() as u64);
        }

        let count_request = VectorSearchRequest {
            query_vectors: request.query_vectors.clone(),
            query_payloads: request.query_payloads.clone(),
            fields: request.fields.clone(),
            limit: usize::MAX,
            score_mode: request.score_mode,
            overfetch: 1.0,
            filter: request.filter.clone(),
            min_score: request.min_score,
            lexical_query: request.lexical_query.clone(),
            fusion_config: request.fusion_config.clone(),
            allowed_ids: request.allowed_ids.clone(),
        };

        let results = self.search(&count_request)?;
        Ok(results.hits.len() as u64)
    }
}
