//! VectorEngine: High-level vector search engine.
//!
//! This module provides a unified interface for vector indexing and search,
//! analogous to `LexicalEngine` for lexical search.
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

use crate::lexical::engine::LexicalEngine;
use crate::lexical::index::inverted::query::boolean::BooleanQuery;
use crate::lexical::index::inverted::query::{Query, term::TermQuery};
use crate::lexical::search::searcher::LexicalSearchRequest;

use parking_lot::{Mutex, RwLock};

use crate::embedding::embedder::{EmbedInput, Embedder};
use crate::embedding::per_field::PerFieldEmbedder;
use crate::error::{Result, SarissaError};
use crate::maintenance::deletion::DeletionManager;
use crate::storage::Storage;
use crate::storage::prefixed::PrefixedStorage;
use crate::vector::core::document::{
    DocumentPayload, DocumentVector, Payload, PayloadSource, StoredVector,
};
use crate::vector::core::vector::Vector;
use crate::vector::field::{
    FieldHit, FieldSearchInput, VectorField, VectorFieldReader, VectorFieldStats, VectorFieldWriter,
};
use crate::vector::index::config::{FlatIndexConfig, HnswIndexConfig, IvfIndexConfig};
use crate::vector::index::field::{AdapterBackedVectorField, LegacyVectorFieldWriter};
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
use crate::vector::engine::config::{
    FlatOption, HnswOption, IvfOption, VectorFieldConfig, VectorOption,
    VectorIndexConfig, VectorIndexKind,
};
use crate::vector::wal::{WalEntry, WalManager};

/// A high-level vector search engine that provides both indexing and searching.
///
/// The `VectorEngine` provides a simplified, unified interface for all vector operations,
/// managing multiple vector fields, persistence, and search.
pub struct VectorEngine {
    config: Arc<VectorIndexConfig>,
    field_configs: Arc<RwLock<HashMap<String, VectorFieldConfig>>>,
    fields: Arc<RwLock<HashMap<String, FieldHandle>>>,
    metadata_index: Arc<LexicalEngine>,
    embedder_registry: Arc<VectorEmbedderRegistry>,
    embedder_executor: Mutex<Option<Arc<EmbedderExecutor>>>,
    wal: Arc<WalManager>,
    /// Manager for logical deletions.
    deletion_manager: Arc<DeletionManager>,
    storage: Arc<dyn Storage>,
    documents: Arc<RwLock<HashMap<u64, DocumentVector>>>,
    snapshot_wal_seq: AtomicU64,
    // next_doc_id: AtomicU64, // REMOVED: Managed by LexicalEngine
    closed: AtomicU64, // 0 = open, 1 = closed
}

impl fmt::Debug for VectorEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VectorEngine")
            .field("config", &self.config)
            .field("field_count", &self.fields.read().len())
            .finish()
    }
}

impl VectorEngine {
    /// Create a new vector engine with the given storage and configuration.
    pub fn new(storage: Arc<dyn Storage>, config: VectorIndexConfig) -> Result<Self> {
        let embedder_registry = Arc::new(VectorEmbedderRegistry::new());
        let field_configs = Arc::new(RwLock::new(config.fields.clone()));

        // Initialize Metadata Index (LexicalEngine)
        // We use a sub-storage for metadata to keep it isolated
        // let metadata_storage = storage.sub_storage("metadata")?; // sub_storage trait method not available
        let metadata_storage = Arc::new(PrefixedStorage::new("metadata", storage.clone()));
        let metadata_index = Arc::new(LexicalEngine::new(
            metadata_storage,
            config.metadata_config.clone(),
        )?);

        // Store the embedder from config before moving config into Arc
        let config_embedder = config.embedder.clone();
        let deletion_config = config.deletion_config.clone();

        let deletion_manager = Arc::new(DeletionManager::new(deletion_config, storage.clone())?);

        let mut engine = Self {
            config: Arc::new(config),
            field_configs: field_configs.clone(),
            fields: Arc::new(RwLock::new(HashMap::new())),
            metadata_index,
            embedder_registry,
            embedder_executor: Mutex::new(None),
            wal: Arc::new(WalManager::new(storage.clone(), "vector_engine.wal")?),
            deletion_manager,
            storage,
            documents: Arc::new(RwLock::new(HashMap::new())),
            snapshot_wal_seq: AtomicU64::new(0),
            // next_doc_id: AtomicU64::new(0),
            closed: AtomicU64::new(0),
        };
        // Ensure global deletion bitmap is initialized
        let shard_id = engine.config.shard_id;
        engine.deletion_manager.initialize_segment(
            "global",
            crate::util::id::create_doc_id(shard_id, 0),
            crate::util::id::create_doc_id(shard_id, crate::util::id::MAX_LOCAL_ID),
        )?;

        engine.load_persisted_state()?;

        // Register embedder instances from the config (after fields are instantiated)
        engine.register_embedder_from_config(config_embedder)?;

        Ok(engine)
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
            return Err(SarissaError::invalid_config(format!(
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
            return Err(SarissaError::index("VectorEngine is closed"));
        }
        Ok(())
    }

    fn embed_document_payload_internal(
        &self,
        _doc_id: u64,
        payload: DocumentPayload,
    ) -> Result<DocumentVector> {
        // Ensure fields are registered (implicit schema generation if enabled)
        for (field_name, field_payload) in payload.fields.iter() {
            self.ensure_field_for_payload(field_name, field_payload)?;
        }

        let mut document = DocumentVector::new();
        document.metadata = payload.metadata;

        for (field_name, field_payload) in payload.fields.into_iter() {
            let vector = self.embed_payload(&field_name, field_payload)?;
            document.fields.insert(field_name, vector);
        }

        Ok(document)
    }

    fn ensure_field_for_payload(&self, field_name: &str, payload: &Payload) -> Result<()> {
        // Fast path: already registered
        if self.fields.read().contains_key(field_name) {
            return Ok(());
        }

        if !self.config.implicit_schema {
            return Err(SarissaError::invalid_argument(format!(
                "vector field '{field_name}' is not registered"
            )));
        }

        let field_config = self.build_field_config_for_payload(field_name, payload)?;

        // Persist in config cache
        self.field_configs
            .write()
            .insert(field_name.to_string(), field_config.clone());

        // Build field runtime
        let field = self.create_vector_field(field_name.to_string(), field_config)?;
        self.register_field_impl(field)?;

        // Register embedder for this field
        self.register_embedder_for_field(field_name, self.config.embedder.clone())?;

        // Persist manifest to record new field configuration
        self.persist_manifest()?;
        Ok(())
    }

    fn build_field_config_for_payload(
        &self,
        field_name: &str,
        payload: &Payload,
    ) -> Result<VectorFieldConfig> {
        let dimension = match &payload.source {
            PayloadSource::Text { .. } | PayloadSource::Bytes { .. } => {
                self.config.default_dimension.ok_or_else(|| {
                    SarissaError::invalid_config(
                        "implicit schema requires default_dimension to be set",
                    )
                })?
            }
            PayloadSource::Vector { data } => data.len(),
        };

        if dimension == 0 {
            return Err(SarissaError::invalid_config(format!(
                "cannot register field '{field_name}' with zero dimension"
            )));
        }

        let vector_option = match self.config.default_index_kind {
            VectorIndexKind::Flat => VectorOption::Flat(FlatOption {
                dimension,
                distance: self.config.default_distance,
                base_weight: self.config.default_base_weight,
                quantizer: None,
            }),
            VectorIndexKind::Hnsw => VectorOption::Hnsw(HnswOption {
                dimension,
                distance: self.config.default_distance,
                base_weight: self.config.default_base_weight,
                quantizer: None,
                ..Default::default()
            }),
            VectorIndexKind::Ivf => VectorOption::Ivf(IvfOption {
                dimension,
                distance: self.config.default_distance,
                base_weight: self.config.default_base_weight,
                quantizer: None,
                ..Default::default()
            }),
        };

        Ok(VectorFieldConfig {
            vector: Some(vector_option),
            lexical: None,
        })
    }

    fn register_embedder_for_field(
        &self,
        field_name: &str,
        embedder: Arc<dyn Embedder>,
    ) -> Result<()> {
        if let Some(per_field) = embedder.as_any().downcast_ref::<PerFieldEmbedder>() {
            let field_embedder = per_field.get_embedder(field_name).clone();
            self.embedder_registry
                .register(field_name.to_string(), field_embedder);
        } else {
            self.embedder_registry
                .register(field_name.to_string(), embedder);
        }
        Ok(())
    }

    /// Embeds a single `Payload` into a `StoredVector`.
    fn embed_payload(&self, field_name: &str, payload: Payload) -> Result<StoredVector> {
        let fields = self.fields.read();
        let handle = fields.get(field_name).ok_or_else(|| {
            SarissaError::invalid_argument(format!("vector field '{field_name}' is not registered"))
        })?;
        let field_config = handle.field.config().clone();
        drop(fields);

        // Check if vector indexing is enabled for this field
        let dimension = match &field_config.vector {
            Some(opt) => opt.dimension(),
            None => {
                // If not configured for vector indexing, return empty vector
                // This allows the field to be used for lexical indexing without storing vectors
                return Ok(StoredVector::new(Arc::from([])));
            }
        };

        let Payload { source } = payload;

        match source {
            PayloadSource::Text { value } => {
                let executor = self.ensure_embedder_executor()?;
                let embedder = self.embedder_registry.resolve(field_name)?;

                if !embedder.supports_text() {
                    return Err(SarissaError::invalid_config(format!(
                        "embedder '{}' does not support text embedding",
                        field_name
                    )));
                }

                let embedder_name_owned = field_name.to_string();
                let text_value = value;
                let text_for_embed = text_value.clone();
                let vector = executor
                    .run(async move { embedder.embed(&EmbedInput::Text(&text_for_embed)).await })?;
                vector.validate_dimension(dimension)?;
                if !vector.is_valid() {
                    return Err(SarissaError::InvalidOperation(format!(
                        "embedder '{}' produced invalid values for field '{}'",
                        embedder_name_owned, field_name
                    )));
                }
                let mut stored: StoredVector = vector.into();

                // Store original text if lexical indexing is enabled
                if field_config.lexical.is_some() {
                    stored
                        .attributes
                        .insert("__sarissa_lexical_source".to_string(), text_value);
                }
                Ok(stored)
            }
            PayloadSource::Bytes { bytes, mime } => {
                let executor = self.ensure_embedder_executor()?;
                let embedder = self.embedder_registry.resolve(field_name)?;

                if !embedder.supports_image() {
                    return Err(SarissaError::invalid_config(format!(
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
                vector.validate_dimension(dimension)?;
                if !vector.is_valid() {
                    return Err(SarissaError::InvalidOperation(format!(
                        "embedder '{}' produced invalid values for field '{}': {:?}",
                        embedder_name_owned, field_name, vector
                    )));
                }
                let stored: StoredVector = vector.into();
                Ok(stored)
            }
            PayloadSource::Vector { data } => {
                let vector = Vector::new(data.to_vec());
                vector.validate_dimension(dimension)?;
                if !vector.is_valid() {
                    return Err(SarissaError::InvalidOperation(format!(
                        "provided vector for field '{}' contains invalid values",
                        field_name
                    )));
                }
                let stored: StoredVector = vector.into();
                Ok(stored)
            }
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
                return Err(SarissaError::invalid_config(format!(
                    "vector field '{field_name}' validation failed: no vector configuration found"
                )));
            }
        };

        if vector_option.dimension() == 0 {
            return Err(SarissaError::invalid_config(format!(
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
                return Err(SarissaError::invalid_config(format!(
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

    fn validate_document_fields(&self, document: &DocumentVector) -> Result<()> {
        let fields = self.fields.read();
        for field_name in document.fields.keys() {
            if !fields.contains_key(field_name) {
                return Err(SarissaError::invalid_argument(format!(
                    "vector field '{field_name}' is not registered"
                )));
            }
        }
        Ok(())
    }

    /*
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
    */

    fn delete_fields_for_doc(&self, doc_id: u64, doc: &DocumentVector) -> Result<()> {
        let fields = self.fields.read();
        for field_name in doc.fields.keys() {
            let field = fields.get(field_name).ok_or_else(|| {
                SarissaError::not_found(format!(
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
        fields_data: &HashMap<String, StoredVector>,
    ) -> Result<()> {
        let fields = self.fields.read();
        for (field_name, stored_vector) in fields_data {
            let field = fields.get(field_name).ok_or_else(|| {
                SarissaError::not_found(format!("vector field '{field_name}' is not registered"))
            })?;
            field
                .runtime
                .writer()
                .add_stored_vector(doc_id, stored_vector, version)?;
        }
        Ok(())
    }

    fn load_persisted_state(&mut self) -> Result<()> {
        let storage = self.registry_storage();
        // Registry snapshot loading is replaced by LexicalEngine persistence.
        // LexicalEngine handles its own persistence automatically.

        self.load_document_snapshot(storage.clone())?;
        self.load_collection_manifest(storage.clone())?;
        // Instantiate fields after manifest load so that persisted implicit fields are registered
        self.instantiate_configured_fields()?;
        self.replay_wal_into_fields()?;
        // self.recompute_next_doc_id(); // Removed
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
                            .collect();
                        SnapshotDocument {
                            doc_id: legacy.doc_id,
                            document: DocumentVector {
                                fields,
                                metadata: legacy.metadata,
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
            return Err(SarissaError::invalid_config(format!(
                "collection manifest version mismatch: expected {}, found {}",
                COLLECTION_MANIFEST_VERSION, manifest.version
            )));
        }

        let snapshot_seq = self.snapshot_wal_seq.load(Ordering::SeqCst);
        if manifest.snapshot_wal_seq != snapshot_seq {
            return Err(SarissaError::invalid_config(format!(
                "collection manifest snapshot sequence {} does not match persisted snapshot {}",
                manifest.snapshot_wal_seq, snapshot_seq
            )));
        }

        if manifest.wal_last_seq < manifest.snapshot_wal_seq {
            return Err(SarissaError::invalid_config(
                "collection manifest WAL sequence regressed",
            ));
        }

        if !manifest.field_configs.is_empty() {
            *self.field_configs.write() = manifest.field_configs.clone();
        }

        Ok(())
    }

    fn replay_wal_into_fields(&self) -> Result<()> {
        let mut documents = self.documents.read().clone();
        self.apply_documents_to_fields(&documents)?;

        // Read records (this also updates internal next_seq based on finding)
        let mut records = self.wal.read_all()?;

        let mut applied_seq = self.snapshot_wal_seq.load(Ordering::SeqCst);
        let start_seq = applied_seq.saturating_add(1);

        // Ensure WAL manager knows about the sequence number from snapshot
        let current_wal_seq = self.wal.last_seq();
        if applied_seq > current_wal_seq {
            self.wal.set_next_seq(applied_seq + 1);
        }

        if records.is_empty() {
            // If WAL is empty but we have documents, ensure they are in sync?
            // Assuming snapshot is source of truth if WAL is empty.
            *self.documents.write() = documents;
            return Ok(());
        }

        records.sort_by(|a, b| a.seq.cmp(&b.seq));
        for record in records.into_iter() {
            if record.seq < start_seq {
                continue;
            }
            applied_seq = record.seq;
            match record.entry {
                WalEntry::Upsert { doc_id, document } => {
                    // logic..
                    if document.fields.is_empty() {
                        documents.remove(&doc_id);
                        continue;
                    }
                    // Version management using WAL seq
                    let version = record.seq;
                    self.apply_field_updates(doc_id, version, &document.fields)?;

                    documents.insert(doc_id, document);
                }
                WalEntry::Delete { doc_id } => {
                    // We assume Lexical is already in sync or we don't need to sync it here for fields (as fields don't store metadata)
                    // But we DO need to delete from fields.
                    // To delete from fields, we need version? delete_document takes version.
                    let version = record.seq;

                    // Re-implement delete_fields_for_entry logic without registry
                    // We can just iterate configured fields or iterate based on what we know about the doc?
                    // NOTE: delete_fields_for_entry iterated `entry.fields`.
                    // But `entry` came from `registry`. We don't have it.
                    // However, we have `documents` map which has the previous state of the document!
                    // documents.get(&doc_id).
                    if let Some(doc) = documents.get(&doc_id) {
                        // We can delete fields based on the document we are about to remove.
                        let fields_guard = self.fields.read();
                        for field_name in doc.fields.keys() {
                            if let Some(field) = fields_guard.get(field_name) {
                                field.runtime.writer().delete_document(doc_id, version)?;
                            }
                        }
                    } else {
                        // If not in documents map, maybe it was never there or already deleted.
                        // We can iterate ALL fields to sure-kill? Expensive.
                        // Or just ignore if we assume documents map is accurate.
                        // Warning: if partial state, we might miss deletion.
                        // Let's rely on documents map.
                    }

                    documents.remove(&doc_id);
                }
            }
        }

        if applied_seq > self.snapshot_wal_seq.load(Ordering::SeqCst) {
            self.snapshot_wal_seq.store(applied_seq, Ordering::SeqCst);
        }

        *self.documents.write() = documents;
        Ok(())
    }

    fn apply_documents_to_fields(&self, documents: &HashMap<u64, DocumentVector>) -> Result<()> {
        for (doc_id, document) in documents.iter() {
            if document.fields.is_empty() {
                continue;
            }
            // For initial load, version 0 is fine/accepted behavior as we are rebuilding state
            self.apply_field_updates(*doc_id, 0, &document.fields)?;
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
        // WAL is self-persisting
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
            last_wal_seq: self.wal.last_seq(),
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
            wal_last_seq: self.wal.last_seq(),
            field_configs: self.field_configs.read().clone(),
        };
        let serialized = serde_json::to_vec(&manifest)?;
        self.write_atomic(storage, COLLECTION_MANIFEST_FILE, &serialized)
    }

    /// Upsert a document    // Internal helper for upserting/indexing
    fn upsert_document_internal(&self, document: DocumentVector) -> Result<u64> {
        self.check_closed()?;
        self.validate_document_fields(&document)?;

        // 1. Extract or generate ID
        let external_id = document
            .metadata
            .get("_id")
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // 2. Index metadata (Lexical Engine handles ID assignment)
        // We construct a Lexical Document for the metadata
        let mut lex_doc_builder = crate::lexical::core::document::Document::builder();
        use crate::lexical::core::field::TextOption;

        let field_configs = self.field_configs.read();

        // Iterate over all known fields (from config) to check for lexical options
        for (field_name, config) in field_configs.iter() {
            if let Some(lexical_opt) = &config.lexical {
                // Check if value exists in metadata (explicit metadata)
                if let Some(metadata_val) = document.metadata.get(field_name) {
                    // Parse metadata_val based on lexical option type?
                    // For now, assume TextOption logic (strings) or implement type parsing
                    // The simple path: all metadata is string -> TextOption
                    // But config might specify IntegerOption. We should try to parse.
                    // IMPORTANT: Current LexicalBuilder.add_text expects String.
                    // We only support Text for now from metadata.
                    if let crate::lexical::core::field::FieldOption::Text(text_opt) = lexical_opt {
                        lex_doc_builder =
                            lex_doc_builder.add_text(field_name, metadata_val, text_opt.clone());
                    }
                }
                // Check if value exists in vector fields (embedded text source)
                else if let Some(stored_vec) = document.fields.get(field_name) {
                    if let Some(original_text) =
                        stored_vec.attributes.get("__sarissa_lexical_source")
                    {
                        println!(
                            "DEBUG: indexing field {} lexical source: {}",
                            field_name, original_text
                        );
                        if let crate::lexical::core::field::FieldOption::Text(text_opt) =
                            lexical_opt
                        {
                            lex_doc_builder = lex_doc_builder.add_text(
                                field_name,
                                original_text,
                                text_opt.clone(),
                            );
                        }
                    }
                }
            }
        }

        // Also add any metadata fields that are NOT in the config but present in document.metadata?
        // Current behavior (implicit dynamic fields) added ANY non-id metadata.
        // If we want to maintain dynamic metadata behavior:
        for (k, v) in &document.metadata {
            if k != "_id" && !field_configs.contains_key(k) {
                // Heuristic: only add if we have some policy? Or always as default text?
                // Let's keep existing behavior: treat as Default Text.
                lex_doc_builder = lex_doc_builder.add_text(k, v, TextOption::default());
            }
        }

        // Ensure _id is handled (usually implicit in LexicalEngine logic, but let's leave it to index_document)

        let lex_doc = lex_doc_builder.build();

        // Index into LexicalEngine (handles ID assignment/deduplication)
        // index_document attempts to FIND existing ID first (Overwrite behavior)
        let doc_id = self.metadata_index.index_document(&external_id, lex_doc)?;

        // 3. Write to WAL
        // Serialize the document for WAL
        // Note: We persist the FULL document including vectors and metadata
        self.wal.append(&WalEntry::Upsert {
            doc_id,
            document: document.clone(),
        })?;

        // 4. Update In-Memory Structures
        // Update document store
        self.documents.write().insert(doc_id, document.clone());

        // Update Field Writers
        // Using version 0 (sequence from WAL/snapshot handling happens elsewhere or 0 during upsert if not tracked tightly here)
        self.apply_field_updates(doc_id, 0, &document.fields)?;

        Ok(doc_id)
    }

    // Internal helper for indexing as chunk (always append)
    fn upsert_document_internal_chunk(&self, document: DocumentVector) -> Result<u64> {
        self.check_closed()?;
        self.validate_document_fields(&document)?;

        // 1. Extract or generate ID
        let external_id = document
            .metadata
            .get("_id")
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // 2. Index metadata
        let mut lex_doc_builder = crate::lexical::core::document::Document::builder();
        use crate::lexical::core::field::TextOption;

        // Add _id field explicitly (LexicalEngine::add_document doesn't do it automatically like index_document)
        lex_doc_builder = lex_doc_builder.add_text("_id", &external_id, TextOption::default());

        let field_configs = self.field_configs.read();

        // Iterate over all known fields (from config) to check for lexical options
        for (field_name, config) in field_configs.iter() {
            if let Some(lexical_opt) = &config.lexical {
                // Check if value exists in metadata (explicit metadata)
                if let Some(metadata_val) = document.metadata.get(field_name) {
                    if let crate::lexical::core::field::FieldOption::Text(text_opt) = lexical_opt {
                        lex_doc_builder =
                            lex_doc_builder.add_text(field_name, metadata_val, text_opt.clone());
                    }
                }
                // Check if value exists in vector fields (embedded text source)
                else if let Some(stored_vec) = document.fields.get(field_name) {
                    if let Some(original_text) =
                        stored_vec.attributes.get("__sarissa_lexical_source")
                    {
                        if let crate::lexical::core::field::FieldOption::Text(text_opt) =
                            lexical_opt
                        {
                            lex_doc_builder = lex_doc_builder.add_text(
                                field_name,
                                original_text,
                                text_opt.clone(),
                            );
                        }
                    }
                }
            }
        }

        for (k, v) in &document.metadata {
            if k != "_id" && !field_configs.contains_key(k) {
                lex_doc_builder = lex_doc_builder.add_text(k, v, TextOption::default());
            }
        }
        let lex_doc = lex_doc_builder.build();

        // Index into LexicalEngine using add_document (Always Append)
        let doc_id = self.metadata_index.add_document(lex_doc)?;

        // 3. Write to WAL
        self.wal.append(&WalEntry::Upsert {
            doc_id,
            document: document.clone(),
        })?;

        // 4. Update In-Memory Structures
        self.documents.write().insert(doc_id, document.clone());

        // Update Field Writers
        self.apply_field_updates(doc_id, 0, &document.fields)?;

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
    pub fn add_vectors(&self, doc: DocumentVector) -> Result<u64> {
        self.upsert_document_internal(doc)
    }

    /// Add a document from payload (will be embedded if configured).
    ///
    /// Returns the assigned document ID.
    pub fn add_payloads(&self, payload: DocumentPayload) -> Result<u64> {
        self.upsert_document_payload(payload)
    }

    /// Add or update vectors for an external ID.
    pub fn index_vectors(&self, external_id: &str, mut doc: DocumentVector) -> Result<u64> {
        doc.metadata
            .insert("_id".to_string(), external_id.to_string());
        self.upsert_document_internal(doc)
    }

    /// Add or update payloads for an external ID.
    pub fn index_payloads(&self, external_id: &str, mut payload: DocumentPayload) -> Result<u64> {
        payload
            .metadata
            .insert("_id".to_string(), external_id.to_string());
        self.upsert_document_payload(payload)
    }

    /// Add multiple vectors with automatically assigned doc_ids.
    pub fn add_vectors_batch(
        &self,
        docs: impl IntoIterator<Item = DocumentVector>,
    ) -> Result<Vec<u64>> {
        docs.into_iter().map(|doc| self.add_vectors(doc)).collect()
    }

    /// Index multiple payloads with automatically assigned doc_ids.
    pub fn index_payloads_batch(&self, payloads: Vec<DocumentPayload>) -> Result<Vec<u64>> {
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
    pub fn index_payload_chunk(&self, payload: DocumentPayload) -> Result<u64> {
        self.check_closed()?;

        // Embed payload into vector document
        // We use a dummy ID (0) here because the real ID is assigned by LexicalEngine later
        let document = self.embed_document_payload_internal(0, payload)?;
        let doc_id = self.upsert_document_internal_chunk(document)?;
        Ok(doc_id)
    }

    /// Index multiple document payloads as chunks.
    pub fn index_payloads_chunk(&self, payloads: Vec<DocumentPayload>) -> Result<Vec<u64>> {
        let mut doc_ids = Vec::with_capacity(payloads.len());
        for payload in payloads {
            let doc_id = self.index_payload_chunk(payload)?;
            doc_ids.push(doc_id);
        }
        Ok(doc_ids)
    }

    /// Upsert a document with a specific document ID.
    /// Note: This method is now legacy/unsafe as IDs are managed by LexicalEngine.
    /// It should only be used if you know the ID is valid and aligned with Lexical.
    pub fn upsert_vectors(&self, _doc_id: u64, doc: DocumentVector) -> Result<()> {
        // Warning: ignoring doc_id request and letting internal logic assign/check ID based on metadata
        self.upsert_document_internal(doc)?;
        Ok(())
    }

    /// Upsert a document from payload (will be embedded if configured).
    pub fn upsert_payloads(&self, _doc_id: u64, payload: DocumentPayload) -> Result<()> {
        self.upsert_document_payload(payload)?;
        Ok(())
    }

    /// Upsert a document from payload (internal helper).
    fn upsert_document_payload(&self, payload: DocumentPayload) -> Result<u64> {
        // Embed without ID (using 0 as placeholder if needed by embedder API, but we use internal call)
        let document = self.embed_document_payload_internal(0, payload)?;
        self.upsert_document_internal(document)
    }

    /// Get a document by its internal ID.
    pub fn get_document(&self, doc_id: u64) -> Result<Option<DocumentVector>> {
        Ok(self.documents.read().get(&doc_id).cloned())
    }

    /// Get a document by its external ID.
    pub fn get_document_by_id(&self, external_id: &str) -> Result<Option<DocumentVector>> {
        if let Some(doc_id) = self
            .metadata_index
            .find_doc_id_by_term("_id", external_id)?
        {
            self.get_document(doc_id)
        } else {
            Ok(None)
        }
    }

    /// Delete a document by its external ID.
    pub fn delete_document_by_id(&self, external_id: &str) -> Result<bool> {
        let doc_ids = self
            .metadata_index
            .find_doc_ids_by_term("_id", external_id)?;

        if !doc_ids.is_empty() {
            let mut found = false;
            for doc_id in doc_ids {
                // Ignore not found errors for individual chunks (idempotency)
                match self.delete_vectors(doc_id) {
                    Ok(_) => found = true,
                    Err(SarissaError::Other(ref msg)) if msg.starts_with("Not found") => continue,
                    Err(e) => return Err(e),
                }
            }
            Ok(found)
        } else {
            Ok(false)
        }
    }

    /// Delete a document by ID.
    /// Delete a document by ID.
    pub fn delete_vectors(&self, doc_id: u64) -> Result<()> {
        let documents = self.documents.read();
        let doc = documents
            .get(&doc_id)
            .ok_or_else(|| SarissaError::not_found(format!("doc_id {doc_id}")))?;

        self.delete_fields_for_doc(doc_id, doc)?;

        if let Some(ext_id) = doc.metadata.get("_id") {
            self.metadata_index.delete_document_by_id(ext_id)?;
        }
        drop(documents);

        self.wal.append(&WalEntry::Delete { doc_id })?;
        self.documents.write().remove(&doc_id);

        // WAL is durable on append, so we don't need full persist_state here
        // But we might want to update snapshots periodically? For now, keep it simple.
        self.persist_state()?; // Still need to update registry/doc snapshots if we want them in sync
        Ok(())
    }

    /// Embed a document payload into vectors.
    pub fn embed_document_payload(&self, payload: DocumentPayload) -> Result<DocumentVector> {
        // Just embed, don't upsert. 0 is dummy id.
        self.embed_document_payload_internal(0, payload)
    }

    /// Embed a payload for query.
    pub fn embed_query_payload(&self, field_name: &str, payload: Payload) -> Result<QueryVector> {
        let vector = self.embed_payload(field_name, payload)?;
        Ok(QueryVector {
            vector,
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
            SarissaError::not_found(format!("vector field '{field_name}' is not registered"))
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
            SarissaError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        let reader_arc: Arc<dyn VectorFieldReader> = Arc::from(reader);
        field.runtime.replace_reader(reader_arc);
        Ok(())
    }

    /// Reset the reader for a specific field to default.
    pub fn reset_field_reader(&self, field_name: &str) -> Result<()> {
        let fields = self.fields.read();
        let field = fields.get(field_name).ok_or_else(|| {
            SarissaError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        field.runtime.reset_reader();
        Ok(())
    }

    /// Materialize the delegate reader for a field (build persistent index).
    pub fn materialize_delegate_reader(&self, field_name: &str) -> Result<()> {
        let fields = self.fields.read();
        let handle = fields.get(field_name).ok_or_else(|| {
            SarissaError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;

        let in_memory = handle
            .field
            .as_any()
            .downcast_ref::<InMemoryVectorField>()
            .ok_or_else(|| {
                SarissaError::InvalidOperation(format!(
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
            SarissaError::not_found(format!("vector field '{field_name}' is not registered"))
        })?;
        handle.runtime.replace_reader(reader);
        Ok(())
    }

    /// Create a searcher for this engine.
    pub fn searcher(&self) -> Result<Box<dyn crate::vector::search::searcher::VectorSearcher>> {
        Ok(Box::new(VectorEngineSearcher::from_engine_ref(self)))
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
        self.metadata_index.commit()?;
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

        let stats = self.metadata_index.stats()?;
        let doc_count = stats.doc_count.saturating_sub(stats.deleted_count);

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
        self.metadata_index.close()?;
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

/// Searcher implementation for [`VectorEngine`].
#[derive(Debug)]
pub struct VectorEngineSearcher {
    config: Arc<VectorIndexConfig>,
    fields: Arc<RwLock<HashMap<String, FieldHandle>>>,
    metadata_index: Arc<LexicalEngine>,
    documents: Arc<RwLock<HashMap<u64, DocumentVector>>>,
}

impl VectorEngineSearcher {
    /// Create a new searcher from an engine reference.
    pub fn from_engine_ref(engine: &VectorEngine) -> Self {
        Self {
            config: Arc::clone(&engine.config),
            fields: Arc::clone(&engine.fields),
            metadata_index: Arc::clone(&engine.metadata_index),
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
                        return Err(SarissaError::not_found(format!(
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
        let filter: &crate::vector::engine::filter::VectorFilter = if let Some(f) = &request.filter
        {
            if crate::vector::engine::filter::VectorFilter::is_empty(f) {
                return Ok(None);
            }
            f
        } else {
            return Ok(None);
        };

        // Convert VectorFilter to Lexical BooleanQuery
        let mut boolean_query = BooleanQuery::new();
        let mut has_clauses = false;

        // 1. Document Level Metadata (Main use case)
        for (key, value) in &filter.document.equals {
            let term_query = TermQuery::new(key, value);
            boolean_query.add_must(Box::new(term_query));
            has_clauses = true;
        }

        // 2. Field Level Metadata (Currently treated same as doc metadata or ignored if field scoping not supported in Lexical yet)
        // For now, let's treat them as global metadata constraints for simplicity, or we can prefix keys?
        // Implementation plan focused on document metadata. Ignoring field-specific metadata filter for now if not critical.
        // If we want to support it, we'd need to index field metadata as `field_name.key`.
        // Let's stick to document metadata for the integration step.

        if !has_clauses {
            return Ok(None);
        }

        let lexical_request = LexicalSearchRequest::new(Box::new(boolean_query) as Box<dyn Query>)
            .load_documents(false); // Only need IDs

        // Execute search
        let results = self.metadata_index.search(lexical_request)?;

        // Collect Doc IDs
        let allowed_ids: HashSet<u64> = results.hits.into_iter().map(|hit| hit.doc_id).collect();
        Ok(Some(allowed_ids))
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
                    return Err(SarissaError::invalid_argument(
                        "VectorScoreMode::LateInteraction is not supported yet",
                    ));
                }
            }
            entry.field_hits.push(hit);
        }

        Ok(())
    }
    fn search_lexical(
        &self,
        query: &crate::vector::engine::request::LexicalQuery,
        limit: usize,
    ) -> Result<HashMap<u64, VectorHit>> {
        use crate::lexical::index::inverted::query::{
            Query,
            boolean::{BooleanClause, BooleanQuery, Occur},
            term::TermQuery,
        };
        use crate::vector::engine::request::{
            BooleanQueryOptions, LexicalQuery, MatchQueryOptions, TermQueryOptions,
        };

        let parser = self.metadata_index.query_parser()?;

        fn convert_query(
            lq: &LexicalQuery,
            parser: &crate::lexical::index::inverted::query::parser::QueryParser,
        ) -> Result<Box<dyn Query>> {
            match lq {
                LexicalQuery::MatchAll => Err(SarissaError::NotImplemented(
                    "MatchAll query not yet supported".to_string(),
                )),
                LexicalQuery::Term(TermQueryOptions { field, term, boost }) => {
                    let q = TermQuery::new(field.clone(), term.clone()).with_boost(*boost);
                    Ok(Box::new(q))
                }
                LexicalQuery::Match(MatchQueryOptions {
                    field,
                    query,
                    operator: _,
                    boost,
                }) => {
                    let mut q = parser.parse_field(field, query)?;
                    if *boost != 1.0 {
                        q.set_boost(*boost);
                    }
                    Ok(q)
                }
                LexicalQuery::Boolean(BooleanQueryOptions {
                    must,
                    must_not,
                    should,
                    boost,
                }) => {
                    let mut bq = BooleanQuery::new();
                    bq.set_boost(*boost);
                    for q in must {
                        bq.add_clause(BooleanClause::new(convert_query(q, parser)?, Occur::Must));
                    }
                    for q in must_not {
                        bq.add_clause(BooleanClause::new(
                            convert_query(q, parser)?,
                            Occur::MustNot,
                        ));
                    }
                    for q in should {
                        bq.add_clause(BooleanClause::new(convert_query(q, parser)?, Occur::Should));
                    }
                    Ok(Box::new(bq))
                }
            }
        }

        let internal_query = convert_query(query, &parser)?;

        use crate::lexical::search::searcher::{LexicalSearchQuery, LexicalSearchRequest};
        let lex_request = LexicalSearchRequest {
            query: LexicalSearchQuery::Obj(internal_query),
            params: crate::lexical::search::searcher::LexicalSearchParams {
                max_docs: limit,
                load_documents: false,
                ..Default::default()
            },
        };

        let results = self.metadata_index.search(lex_request)?;

        let mut hits = HashMap::new();
        for hit in results.hits {
            hits.insert(
                hit.doc_id,
                VectorHit {
                    doc_id: hit.doc_id,
                    score: hit.score,
                    field_hits: Vec::new(),
                },
            );
        }

        Ok(hits)
    }

    fn merge_results(
        &self,
        vector_hits: HashMap<u64, VectorHit>,
        lexical_hits: HashMap<u64, VectorHit>,
        config: &Option<crate::vector::engine::request::FusionConfig>,
    ) -> Result<Vec<VectorHit>> {
        use crate::vector::engine::request::FusionConfig;

        let mut fused_scores: HashMap<u64, f32> = HashMap::new();
        for id in vector_hits.keys() {
            fused_scores.insert(*id, 0.0);
        }
        for id in lexical_hits.keys() {
            fused_scores.insert(*id, 0.0);
        }

        match config {
            Some(FusionConfig::Rrf { k }) => {
                let mut sorted_vector: Vec<_> = vector_hits.values().collect();
                sorted_vector
                    .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal));

                let mut sorted_lexical: Vec<_> = lexical_hits.values().collect();
                sorted_lexical
                    .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal));

                for (rank, hit) in sorted_vector.iter().enumerate() {
                    let rrf_score = 1.0 / (*k as f32 + (rank + 1) as f32);
                    *fused_scores.entry(hit.doc_id).or_default() += rrf_score;
                }

                for (rank, hit) in sorted_lexical.iter().enumerate() {
                    let rrf_score = 1.0 / (*k as f32 + (rank + 1) as f32);
                    *fused_scores.entry(hit.doc_id).or_default() += rrf_score;
                }
            }
            Some(FusionConfig::WeightedSum {
                vector_weight,
                lexical_weight,
            }) => {
                for (id, hit) in &vector_hits {
                    *fused_scores.entry(*id).or_default() += hit.score * vector_weight;
                }
                for (id, hit) in &lexical_hits {
                    *fused_scores.entry(*id).or_default() += hit.score * lexical_weight;
                }
            }
            None => {
                if lexical_hits.is_empty() {
                    for (id, hit) in &vector_hits {
                        fused_scores.insert(*id, hit.score);
                    }
                } else if vector_hits.is_empty() {
                    for (id, hit) in &lexical_hits {
                        fused_scores.insert(*id, hit.score);
                    }
                } else {
                    let k = 60;
                    let mut sorted_vector: Vec<_> = vector_hits.values().collect();
                    sorted_vector.sort_by(|a, b| {
                        b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal)
                    });
                    let mut sorted_lexical: Vec<_> = lexical_hits.values().collect();
                    sorted_lexical.sort_by(|a, b| {
                        b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal)
                    });

                    for (rank, hit) in sorted_vector.iter().enumerate() {
                        *fused_scores.entry(hit.doc_id).or_default() +=
                            1.0 / (k as f32 + (rank + 1) as f32);
                    }
                    for (rank, hit) in sorted_lexical.iter().enumerate() {
                        *fused_scores.entry(hit.doc_id).or_default() +=
                            1.0 / (k as f32 + (rank + 1) as f32);
                    }
                }
            }
        }

        let mut final_hits = Vec::with_capacity(fused_scores.len());

        for (doc_id, score) in fused_scores {
            let field_hits = if let Some(vh) = vector_hits.get(&doc_id) {
                vh.field_hits.clone()
            } else {
                Vec::new()
            };

            final_hits.push(VectorHit {
                doc_id,
                score,
                field_hits,
            });
        }

        final_hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(CmpOrdering::Equal));

        Ok(final_hits)
    }
}

impl crate::vector::search::searcher::VectorSearcher for VectorEngineSearcher {
    fn search(&self, request: &VectorSearchRequest) -> Result<VectorSearchResults> {
        if request.query_vectors.is_empty() && request.lexical_query.is_none() {
            return Err(SarissaError::invalid_argument(
                "VectorSearchRequest requires at least one query vector or lexical query",
            ));
        }

        if request.limit == 0 {
            return Ok(VectorSearchResults::default());
        }

        if request.overfetch < 1.0 {
            return Err(SarissaError::invalid_argument(
                "VectorSearchRequest overfetch must be >= 1.0",
            ));
        }

        if matches!(request.score_mode, VectorScoreMode::LateInteraction) {
            return Err(SarissaError::invalid_argument(
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
            // or should we pass allowed_ids to lexical search? LexicalEngine might not accept generic ID list easily yet).
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
                let field = fields.get(&field_name).ok_or_else(|| {
                    SarissaError::not_found(format!("vector field '{field_name}'"))
                })?;
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
        let vector_search_performed = fields_with_queries > 0;

        // 2. Lexical Search
        let lexical_hits_map = if let Some(lex_query) = &request.lexical_query {
            self.search_lexical(lex_query, request.limit)?
        } else {
            HashMap::new()
        };

        let lexical_search_performed = !lexical_hits_map.is_empty()
            || (request.lexical_query.is_some() && vector_search_performed);

        if !vector_search_performed && !lexical_search_performed {
            if request.lexical_query.is_some() {
                // Lexical query executed but returned no results
                return Ok(VectorSearchResults::default());
            }
            return Err(SarissaError::invalid_argument(
                "no query vectors matched the requested fields and no lexical query provided",
            ));
        }

        // 3. Fusion
        let hits = self.merge_results(vector_hits_map, lexical_hits_map, &request.fusion_config)?;

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
        };

        let results = self.search(&count_request)?;
        Ok(results.hits.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexical::engine::config::LexicalIndexConfig;
    use crate::maintenance::deletion::DeletionConfig;
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    use crate::vector::DistanceMetric;
    use crate::vector::core::document::StoredVector;
    use crate::vector::engine::config::{
        FlatOption, VectorFieldConfig, VectorOption, VectorIndexKind,
    };
    use crate::vector::engine::filter::VectorFilter;
    use crate::vector::engine::request::QueryVector;
    use std::collections::HashMap;

    fn sample_config() -> VectorIndexConfig {
        let field_config = VectorFieldConfig {
            vector: Some(VectorOption::Flat(FlatOption {
                dimension: 3,
                distance: DistanceMetric::Cosine,
                base_weight: 1.0,
                quantizer: None,
            })),
            lexical: None,
        };
        use crate::embedding::precomputed::PrecomputedEmbedder;

        VectorIndexConfig {
            fields: HashMap::from([("body".into(), field_config)]),
            default_fields: vec!["body".into()],
            metadata: HashMap::new(),
            default_distance: DistanceMetric::Cosine,
            default_dimension: None,
            default_index_kind: VectorIndexKind::Flat,
            default_base_weight: 1.0,
            implicit_schema: false,
            embedder: Arc::new(PrecomputedEmbedder::new()),
            deletion_config: DeletionConfig::default(),
            shard_id: 0,
            metadata_config: LexicalIndexConfig::builder()
                .analyzer(Arc::new(
                    crate::analysis::analyzer::keyword::KeywordAnalyzer::default(),
                ))
                .build(),
        }
    }

    fn sample_query(limit: usize) -> VectorSearchRequest {
        let mut query = VectorSearchRequest::default();
        query.limit = limit;
        query.query_vectors.push(QueryVector {
            vector: StoredVector::new(Arc::<[f32]>::from([1.0, 0.0, 0.0])),
            weight: 1.0,
            fields: None,
        });
        query
    }

    fn create_engine(config: VectorIndexConfig, storage: Arc<dyn Storage>) -> VectorEngine {
        VectorEngine::new(storage, config).expect("engine")
    }

    #[test]
    fn engine_creation_works() {
        let config = sample_config();
        let storage: Arc<dyn Storage> =
            Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let engine = create_engine(config, storage);

        let stats = engine.stats().expect("stats");
        assert_eq!(stats.document_count, 0);
        assert!(stats.fields.contains_key("body"));
    }

    #[test]
    fn engine_add_and_search() {
        let config = sample_config();
        let storage: Arc<dyn Storage> =
            Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let engine = create_engine(config, storage);

        let mut doc = DocumentVector::new();
        doc.set_field(
            "body",
            StoredVector::new(Arc::<[f32]>::from([1.0, 0.0, 0.0])),
        );

        let doc_id = engine.add_vectors(doc).expect("add vectors");
        engine.commit().expect("commit");

        let stats = engine.stats().expect("stats");
        assert_eq!(stats.document_count, 1);

        let results = engine.search(sample_query(5)).expect("search");
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].doc_id, doc_id);
    }

    #[test]
    fn engine_upsert_and_delete() {
        let config = sample_config();
        let storage: Arc<dyn Storage> =
            Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let engine = create_engine(config, storage);

        let mut doc = DocumentVector::new();
        doc.set_field(
            "body",
            StoredVector::new(Arc::<[f32]>::from([0.5, 0.5, 0.0])),
        );
        doc.metadata.insert("_id".to_string(), "42".to_string());

        // Use index_vectors to get the assigned ID (Lexical integration)
        let doc_id = engine.index_vectors("42", doc).expect("upsert");
        engine.commit().expect("commit");
        let stats = engine.stats().expect("stats");
        assert_eq!(stats.document_count, 1);

        engine.delete_vectors(doc_id).expect("delete");
        engine.commit().expect("commit");
        let stats = engine.stats().expect("stats");
        assert_eq!(stats.document_count, 0);
    }

    #[test]
    fn engine_persistence_across_instances() {
        let config = sample_config();
        let storage: Arc<dyn Storage> =
            Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));

        {
            let engine = create_engine(config.clone(), storage.clone());
            let mut doc = DocumentVector::new();
            doc.set_field(
                "body",
                StoredVector::new(Arc::<[f32]>::from([1.0, 0.0, 0.0])),
            );
            engine.upsert_vectors(10, doc).expect("upsert");
            engine.commit().expect("commit");
            let stats = engine.stats().expect("stats");
            assert_eq!(stats.document_count, 1, "First instance stats failed");
        }

        let engine = create_engine(config, storage.clone());
        // Debug: List files
        let files = storage.list_files().unwrap();
        println!("Files in storage: {:?}", files);

        let stats = engine.stats().expect("stats");
        assert_eq!(stats.document_count, 1);

        let results = engine.search(sample_query(5)).expect("search");
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.hits[0].doc_id, 0);
    }

    #[test]
    fn engine_id_based_operations() {
        let config = sample_config();
        let storage: Arc<dyn Storage> =
            Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let engine = create_engine(config, storage);

        let mut doc = DocumentVector::new();
        doc.set_field(
            "body",
            StoredVector::new(Arc::<[f32]>::from([1.0, 0.0, 0.0])),
        );

        // 1. Index by external ID
        let internal_id = engine.index_vectors("v_ext_1", doc).expect("index vectors");

        // 2. Get by internal ID
        let found = engine.get_document(internal_id).expect("get doc");
        assert!(found.is_some());

        // 3. Get by external ID
        let found_ext = engine.get_document_by_id("v_ext_1").expect("get doc ext");
        assert!(found_ext.is_some());

        // 4. Delete by external ID
        let deleted = engine.delete_document_by_id("v_ext_1").expect("delete ext");
        assert!(deleted);

        // 5. Verify deletion
        let found_after = engine.get_document_by_id("v_ext_1").expect("get after");
        assert!(found_after.is_none());

        // 6. Non-existent
        assert!(!engine.delete_document_by_id("non_existent").unwrap());
    }

    #[test]
    fn engine_metadata_filtering() {
        let config = sample_config();
        let storage: Arc<dyn Storage> =
            Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let engine = create_engine(config, storage);

        let mut doc1 = DocumentVector::new();
        doc1.set_field(
            "body",
            StoredVector::new(Arc::<[f32]>::from([1.0, 0.0, 0.0])),
        );
        doc1.metadata
            .insert("category".to_string(), "sports".to_string());
        doc1.metadata
            .insert("tag".to_string(), "super cool".to_string());

        let mut doc2 = DocumentVector::new();
        doc2.set_field(
            "body",
            StoredVector::new(Arc::<[f32]>::from([0.0, 1.0, 0.0])),
        );
        doc2.metadata
            .insert("category".to_string(), "news".to_string());
        doc2.metadata
            .insert("tag".to_string(), "bad cool".to_string());

        engine.index_vectors("doc1", doc1).expect("upsert doc1");
        engine.index_vectors("doc2", doc2).expect("upsert doc2");
        engine.commit().expect("commit");

        // 1. Exact match on single term
        let mut request = sample_query(5);
        let mut filter = VectorFilter::default();
        filter
            .document
            .equals
            .insert("category".to_string(), "sports".to_string());
        request.filter = Some(filter);

        let results = engine.search(request).expect("search sports");
        assert_eq!(results.hits.len(), 1);
        // doc IDs are auto-assigned, doc1 should be 0 because it was added first
        assert_eq!(results.hits[0].doc_id, 0);

        // 2. Exact match on multi-term (testing behavior)
        let mut request = sample_query(5);
        let mut filter = VectorFilter::default();
        filter
            .document
            .equals
            .insert("tag".to_string(), "super cool".to_string());
        request.filter = Some(filter);

        let results = engine.search(request).expect("search super cool");
        // We print the count to know for sure.
        println!("Multi-term exact hits: {}", results.hits.len());

        // 3. Partial match (testing if tokenized)
        let mut request = sample_query(5);
        let mut filter = VectorFilter::default();
        filter
            .document
            .equals
            .insert("tag".to_string(), "super".to_string());
        request.filter = Some(filter);

        let results = engine.search(request).expect("search super");
        println!("Multi-term token 'super' hits: {}", results.hits.len());
    }
}
