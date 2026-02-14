//! Shared vector field traits and compatibility adapters.

use std::any::Any;

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::data::DataValue;
use crate::embedding::embedder::{EmbedInput, Embedder};
use crate::embedding::per_field::PerFieldEmbedder;
use crate::error::Result;
use crate::vector::core::vector::StoredVector;
use crate::vector::core::vector::Vector;
use crate::vector::store::config::VectorFieldConfig;
use crate::vector::store::request::QueryVector;
use crate::vector::writer::VectorIndexWriter;

// ============================================================================
// Core Field Traits (from src/vector/field.rs)
// ============================================================================

/// Represents a single logical vector field backed by an index implementation.
#[async_trait]
pub trait VectorField: Send + Sync + Debug {
    /// Returns the field (column) name.
    fn name(&self) -> &str;
    /// Returns the immutable configuration for this field.
    fn config(&self) -> &VectorFieldConfig;
    /// Returns the field writer that ingests vectors for this field.
    fn writer(&self) -> &dyn VectorFieldWriter;
    /// Returns the field reader that serves queries for this field.
    fn reader(&self) -> &dyn VectorFieldReader;
    /// Returns a cloneable writer handle for sharing across runtimes.
    fn writer_handle(&self) -> Arc<dyn VectorFieldWriter>;
    /// Returns a cloneable reader handle for sharing across runtimes.
    fn reader_handle(&self) -> Arc<dyn VectorFieldReader>;
    /// Returns a type-erased reference for downcasting to concrete implementations.
    fn as_any(&self) -> &dyn Any;

    /// Optimize the field storage/index.
    async fn optimize(&self) -> Result<()> {
        self.writer().optimize().await
    }
}

/// Writer interface for ingesting doc-centric vectors into a single field index.
#[async_trait]
pub trait VectorFieldWriter: Send + Sync + Debug {
    /// Add or replace a vector for the given document and field version.
    async fn add_stored_vector(
        &self,
        doc_id: u64,
        vector: &StoredVector,
        version: u64,
    ) -> Result<()>;
    /// Add or replace a value (to be embedded) for the given document and field version.
    async fn add_value(&self, doc_id: u64, value: &DataValue, version: u64) -> Result<()> {
        // Default implementation just errors if not supported/implemented
        if let DataValue::Vector(v) = value {
            let sv = StoredVector::new(v.clone());
            self.add_stored_vector(doc_id, &sv, version).await
        } else {
            Err(crate::error::IrisError::invalid_argument(
                "add_value not supported for this field writer (needs embedding helper)",
            ))
        }
    }
    /// Delete the vectors associated with the provided document id.
    async fn delete_document(&self, doc_id: u64, version: u64) -> Result<()>;

    /// Check if the writer has storage configured.
    async fn has_storage(&self) -> bool;

    /// Get access to the stored vectors with field names.
    async fn vectors(&self) -> Vec<(u64, String, Vector)>;

    /// Rebuild the index with the provided vectors, effectively replacing the current content.
    async fn rebuild(&self, vectors: Vec<(u64, String, Vector)>) -> Result<()>;

    /// Flush any buffered data to durable storage.
    async fn flush(&self) -> Result<()>;

    /// Optimize the index (e.g. rebuild, vacuum).
    async fn optimize(&self) -> Result<()>;
}

/// Reader interface that exposes field-local search/statistics.
pub trait VectorFieldReader: Send + Sync + Debug {
    /// Execute a field-scoped ANN search.
    fn search(&self, request: FieldSearchInput) -> Result<FieldSearchResults>;
    /// Return the latest field statistics (vector count, dimension, ...).
    fn stats(&self) -> Result<VectorFieldStats>;
}

/// Query parameters passed to field-level searchers.
#[derive(Debug, Clone)]
pub struct FieldSearchInput {
    pub field: String,
    pub query_vectors: Vec<QueryVector>,
    pub limit: usize,
    pub allowed_ids: Option<std::collections::HashSet<u64>>,
}

/// Field-level hits returned by an index.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldSearchResults {
    #[serde(default)]
    pub hits: Vec<FieldHit>,
}

/// A single hit originating from a concrete field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldHit {
    pub doc_id: u64,
    pub field: String,
    pub score: f32,
    pub distance: f32,
}

/// Basic statistics collected per field.
#[derive(Debug, Clone, Copy, Default)]
pub struct VectorFieldStats {
    pub vector_count: usize,
    pub dimension: usize,
}

// ============================================================================
// Adapters (from src/vector/index/field.rs)
// ============================================================================

/// Bridges the new doc-centric `VectorFieldWriter` trait to existing index writers.
///
/// When an `embedder` is provided, embedding is performed **outside** the Mutex
/// so that I/O-bound embedding (e.g. HTTP calls) does not block other writes.
pub struct LegacyVectorFieldWriter<W: VectorIndexWriter> {
    field_name: String,
    writer: Mutex<W>,
    /// Optional embedder for performing embedding outside the lock.
    embedder: Option<Arc<dyn Embedder>>,
}

impl<W: VectorIndexWriter> std::fmt::Debug for LegacyVectorFieldWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LegacyVectorFieldWriter")
            .field("field_name", &self.field_name)
            .field("writer", &self.writer)
            .field(
                "embedder",
                &self.embedder.as_ref().map(|e| e.name().to_string()),
            )
            .finish()
    }
}

impl<W: VectorIndexWriter> LegacyVectorFieldWriter<W> {
    /// Create a new adapter for the provided field name and index writer.
    pub fn new(field_name: impl Into<String>, writer: W) -> Self {
        Self {
            field_name: field_name.into(),
            writer: Mutex::new(writer),
            embedder: None,
        }
    }

    /// Set an embedder for performing embedding outside the writer lock.
    ///
    /// When set, `add_value()` with Text/Bytes data will embed first (no lock),
    /// then acquire the lock only for `add_vectors()`.
    pub fn with_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Returns the owning field name.
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    fn to_legacy_vector(&self, doc_id: u64, stored: &StoredVector) -> (u64, String, Vector) {
        let vector = Vector::new(stored.data.to_vec());
        (doc_id, self.field_name.clone(), vector)
    }

    #[cfg(test)]
    pub(crate) async fn pending_vectors(&self) -> Vec<(u64, String, Vector)> {
        let guard = self.writer.lock().await;
        guard.vectors().to_vec()
    }
}

#[async_trait]
impl<W> VectorFieldWriter for LegacyVectorFieldWriter<W>
where
    W: VectorIndexWriter,
{
    async fn add_stored_vector(
        &self,
        doc_id: u64,
        vector: &StoredVector,
        _version: u64,
    ) -> Result<()> {
        let mut guard = self.writer.lock().await;
        let legacy = self.to_legacy_vector(doc_id, vector);
        guard.add_vectors(vec![legacy])
    }

    async fn add_value(&self, doc_id: u64, value: &DataValue, _version: u64) -> Result<()> {
        // If it's already a vector, lock and add directly
        if let DataValue::Vector(v) = value {
            let mut guard = self.writer.lock().await;
            let legacy = (doc_id, self.field_name.clone(), Vector::new(v.clone()));
            return guard.add_vectors(vec![legacy]);
        }

        // For non-vector data: embed OUTSIDE the lock to avoid blocking other writes
        if let Some(ref embedder) = self.embedder {
            let input = match value {
                DataValue::Text(t) => EmbedInput::Text(t),
                DataValue::Bytes(b, m) => EmbedInput::Bytes(b, m.as_deref()),
                _ => {
                    return Err(crate::error::IrisError::invalid_argument(
                        "Unsupported data type for embedding",
                    ));
                }
            };

            // Embed without holding the lock
            let vector = if let Some(pf) = embedder.as_any().downcast_ref::<PerFieldEmbedder>() {
                pf.embed_field(&self.field_name, &input).await?
            } else {
                embedder.embed(&input).await?
            };

            // Now lock briefly to add the resulting vector
            let mut guard = self.writer.lock().await;
            return guard.add_vectors(vec![(doc_id, self.field_name.clone(), vector)]);
        }

        // Fallback: no embedder set, delegate to the writer's add_value (holds lock)
        let mut guard = self.writer.lock().await;
        VectorIndexWriter::add_value(
            &mut *guard,
            doc_id,
            self.field_name.clone(),
            value.clone(),
        )
        .await
    }

    async fn has_storage(&self) -> bool {
        self.writer.lock().await.has_storage()
    }

    async fn vectors(&self) -> Vec<(u64, String, Vector)> {
        self.writer.lock().await.vectors().to_vec()
    }

    async fn rebuild(&self, vectors: Vec<(u64, String, Vector)>) -> Result<()> {
        let mut guard = self.writer.lock().await;
        guard.rollback()?;
        guard.build(vectors)?;
        guard.finalize()?;
        Ok(())
    }

    async fn delete_document(&self, doc_id: u64, _version: u64) -> Result<()> {
        let mut guard = self.writer.lock().await;
        // Best-effort deletion from in-memory buffer.
        let _ = guard.delete_document(doc_id);
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.writer.lock().await.commit()?;
        Ok(())
    }

    async fn optimize(&self) -> Result<()> {
        let vectors = self.vectors().await;
        self.rebuild(vectors).await?;
        self.flush().await
    }
}

/// Concrete [`VectorField`] implementation backed by adapters.
#[derive(Debug)]
pub struct AdapterBackedVectorField {
    name: String,
    config: VectorFieldConfig,
    writer: Arc<dyn VectorFieldWriter>,
    reader: Arc<dyn VectorFieldReader>,
}

impl AdapterBackedVectorField {
    /// Create a new adapter-backed vector field definition.
    pub fn new(
        name: impl Into<String>,
        config: VectorFieldConfig,
        writer: Arc<dyn VectorFieldWriter>,
        reader: Arc<dyn VectorFieldReader>,
    ) -> Self {
        Self {
            name: name.into(),
            config,
            writer,
            reader,
        }
    }

    /// Returns the shared writer handle.
    pub fn writer_handle(&self) -> &Arc<dyn VectorFieldWriter> {
        &self.writer
    }

    /// Returns the shared reader handle.
    pub fn reader_handle(&self) -> &Arc<dyn VectorFieldReader> {
        &self.reader
    }
}

#[async_trait]
impl VectorField for AdapterBackedVectorField {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &VectorFieldConfig {
        &self.config
    }

    fn writer(&self) -> &dyn VectorFieldWriter {
        self.writer.as_ref()
    }

    fn reader(&self) -> &dyn VectorFieldReader {
        self.reader.as_ref()
    }

    fn writer_handle(&self) -> Arc<dyn VectorFieldWriter> {
        self.writer.clone()
    }

    fn reader_handle(&self) -> Arc<dyn VectorFieldReader> {
        self.reader.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::core::vector::StoredVector;
    use crate::vector::index::config::{FlatIndexConfig, HnswIndexConfig, IvfIndexConfig};
    use crate::vector::index::flat::writer::FlatIndexWriter;
    use crate::vector::index::hnsw::writer::HnswIndexWriter;
    use crate::vector::index::ivf::writer::IvfIndexWriter;
    use crate::vector::writer::VectorIndexWriterConfig;

    fn sample_stored_vector() -> StoredVector {
        StoredVector::new(vec![1.0, 0.0])
    }

    fn flat_writer() -> FlatIndexWriter {
        let config = FlatIndexConfig {
            dimension: 2,
            normalize_vectors: false,
            ..Default::default()
        };
        FlatIndexWriter::new(config, VectorIndexWriterConfig::default(), "test_flat").unwrap()
    }

    fn hnsw_writer() -> HnswIndexWriter {
        let config = HnswIndexConfig {
            dimension: 2,
            normalize_vectors: false,
            ..Default::default()
        };
        HnswIndexWriter::new(config, VectorIndexWriterConfig::default(), "test_hnsw").unwrap()
    }

    fn ivf_writer() -> IvfIndexWriter {
        let config = IvfIndexConfig {
            dimension: 2,
            normalize_vectors: false,
            ..Default::default()
        };
        IvfIndexWriter::new(config, VectorIndexWriterConfig::default(), "test_ivf").unwrap()
    }

    #[tokio::test]
    async fn test_adapter_flat() {
        let adapter = LegacyVectorFieldWriter::new("body", flat_writer());
        assert_eq!(adapter.field_name(), "body");
        assert!(!adapter.has_storage().await);
    }

    #[tokio::test]
    async fn test_adapter_hnsw() {
        let adapter = LegacyVectorFieldWriter::new("body", hnsw_writer());
        assert_eq!(adapter.field_name(), "body");
    }

    #[tokio::test]
    async fn test_adapter_ivf() {
        let adapter = LegacyVectorFieldWriter::new("body", ivf_writer());
        assert_eq!(adapter.field_name(), "body");
    }

    #[tokio::test]
    async fn test_adapter_deletion_error_handling() {
        let adapter = LegacyVectorFieldWriter::new("body", flat_writer());
        let vector = sample_stored_vector();

        adapter.add_stored_vector(3, &vector, 1).await.unwrap();

        let result = adapter.delete_document(3, 2).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn adapter_stores_vector_with_correct_doc_id() {
        let adapter = LegacyVectorFieldWriter::new("body", flat_writer());
        let vector = sample_stored_vector();

        adapter.add_stored_vector(5, &vector, 1).await.unwrap();

        let pending = adapter.pending_vectors().await;
        assert_eq!(pending.len(), 1);
        // Verify doc_id and field_name
        assert_eq!(pending[0].0, 5);
        assert_eq!(pending[0].1, "body");
        // Verify vector data was converted correctly
        assert_eq!(pending[0].2.data.len(), 2);
    }
}
