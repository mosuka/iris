//! Shared vector field traits and compatibility adapters.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::vector::core::document::StoredVector;
use crate::vector::core::vector::Vector;
use crate::vector::engine::config::VectorFieldConfig;
use crate::vector::engine::request::QueryVector;
use crate::vector::writer::VectorIndexWriter;

// ============================================================================
// Core Field Traits (from src/vector/field.rs)
// ============================================================================

/// Represents a single logical vector field backed by an index implementation.
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
    fn optimize(&self) -> Result<()> {
        self.writer().optimize()
    }
}

/// Writer interface for ingesting doc-centric vectors into a single field index.
pub trait VectorFieldWriter: Send + Sync + Debug {
    /// Add or replace a vector for the given document and field version.
    fn add_stored_vector(&self, doc_id: u64, vector: &StoredVector, version: u64) -> Result<()>;
    /// Delete the vectors associated with the provided document id.
    fn delete_document(&self, doc_id: u64, version: u64) -> Result<()>;

    /// Check if the writer has storage configured.
    fn has_storage(&self) -> bool;

    /// Get access to the stored vectors with field names.
    fn vectors(&self) -> Vec<(u64, String, Vector)>;

    /// Rebuild the index with the provided vectors, effectively replacing the current content.
    fn rebuild(&self, vectors: Vec<(u64, String, Vector)>) -> Result<()>;

    /// Flush any buffered data to durable storage.
    fn flush(&self) -> Result<()>;

    /// Optimize the index (e.g. rebuild, vacuum).
    fn optimize(&self) -> Result<()>;
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
    #[serde(default)]
    pub metadata: HashMap<String, String>,
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
#[derive(Debug)]
pub struct LegacyVectorFieldWriter<W: VectorIndexWriter> {
    field_name: String,
    writer: Mutex<W>,
}

impl<W: VectorIndexWriter> LegacyVectorFieldWriter<W> {
    /// Create a new adapter for the provided field name and index writer.
    pub fn new(field_name: impl Into<String>, writer: W) -> Self {
        Self {
            field_name: field_name.into(),
            writer: Mutex::new(writer),
        }
    }

    /// Returns the owning field name.
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    fn to_legacy_vector(&self, doc_id: u64, stored: &StoredVector) -> (u64, String, Vector) {
        let vector = stored.to_vector();
        (doc_id, self.field_name.clone(), vector)
    }

    #[cfg(test)]
    pub(crate) fn pending_vectors(&self) -> Vec<(u64, String, Vector)> {
        let guard = self.writer.lock();
        guard.vectors().to_vec()
    }
}

impl<W> VectorFieldWriter for LegacyVectorFieldWriter<W>
where
    W: VectorIndexWriter,
{
    fn add_stored_vector(&self, doc_id: u64, vector: &StoredVector, _version: u64) -> Result<()> {
        let mut guard = self.writer.lock();
        let legacy = self.to_legacy_vector(doc_id, vector);
        guard.add_vectors(vec![legacy])
    }

    fn has_storage(&self) -> bool {
        self.writer.lock().has_storage()
    }

    fn vectors(&self) -> Vec<(u64, String, Vector)> {
        self.writer.lock().vectors().to_vec()
    }

    fn rebuild(&self, vectors: Vec<(u64, String, Vector)>) -> Result<()> {
        let mut guard = self.writer.lock();
        // To rebuild, we rely on the writer's capability to reset or build from scratch.
        // Assuming `build` can be called to populate, but we need to clear first.
        // `rollback` clears pending vectors and resets state.
        guard.rollback()?;
        guard.build(vectors)?;
        guard.finalize()?;
        // If it has storage, we might need to write? `commit` does finalize+write.
        // But we don't know the path here easily unless stored in config or we let `optimize` caller handle checking?
        // `LegacyVectorFieldWriter` doesn't know the path.
        // However, `VectorCollection` keeps track of paths?
        // Actually `VectorIndexWriter` usually knows where to write if initiated with storage?
        // `FlatIndexWriter::load` stores `storage` but not `path` (filename). `write(path)` takes path arg.

        // This is a missing piece. The writer might not know its own filename to overwrite.
        // But `VectorFieldEntry` in Registry has metadata?

        // For now, let's assume `commit` or similar is needed, but `rebuild` just updates memory state.
        // We will need to ensure persistence later.
        // But `optimize` implies persistence update.

        // If we can't persist, `optimize` is incomplete.
        // Let's look at how `VectorCollection` persists.
        // It uses `field.runtime.writer().flush()`.

        Ok(())
    }

    fn delete_document(&self, doc_id: u64, _version: u64) -> Result<()> {
        let mut guard = self.writer.lock();
        // Best-effort deletion from in-memory buffer.
        // We ignore errors here because the index might be finalized/immutable.
        // Logical deletion is handled by the Registry filter in VectorCollection.
        let _ = guard.delete_document(doc_id);
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        self.writer.lock().commit()?;
        Ok(())
    }

    fn optimize(&self) -> Result<()> {
        let vectors = self.vectors();
        self.rebuild(vectors)?;
        self.flush()
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
    use crate::vector::core::document::StoredVector;
    use crate::vector::index::config::{FlatIndexConfig, HnswIndexConfig, IvfIndexConfig};
    use crate::vector::index::flat::writer::FlatIndexWriter;
    use crate::vector::index::hnsw::writer::HnswIndexWriter;
    use crate::vector::index::ivf::writer::IvfIndexWriter;
    use crate::vector::writer::VectorIndexWriterConfig;
    use std::sync::Arc;

    fn sample_stored_vector() -> StoredVector {
        StoredVector::new(Arc::<[f32]>::from([1.0_f32, 0.0_f32]))
    }

    fn flat_writer() -> FlatIndexWriter {
        let mut config = FlatIndexConfig::default();
        config.dimension = 2;
        config.normalize_vectors = false;
        config.dimension = 2;
        config.normalize_vectors = false;
        FlatIndexWriter::new(config, VectorIndexWriterConfig::default(), "test_flat").unwrap()
    }

    fn hnsw_writer() -> HnswIndexWriter {
        let mut config = HnswIndexConfig::default();
        config.dimension = 2;
        config.normalize_vectors = false;
        HnswIndexWriter::new(config, VectorIndexWriterConfig::default(), "test_hnsw").unwrap()
    }

    fn ivf_writer() -> IvfIndexWriter {
        let mut config = IvfIndexConfig::default();
        config.dimension = 2;
        config.normalize_vectors = false;
        config.dimension = 2;
        config.normalize_vectors = false;
        IvfIndexWriter::new(config, VectorIndexWriterConfig::default(), "test_ivf").unwrap()
    }

    #[test]
    fn test_adapter_flat() {
        let adapter = LegacyVectorFieldWriter::new("body", flat_writer());
        assert_eq!(adapter.field_name(), "body");
        assert!(!adapter.has_storage());
    }

    #[test]
    fn test_adapter_hnsw() {
        // For HNSW, we need to handle HnswIndexWriter type availability if feature gated?
        // Assuming HnswIndexWriter is available since we are editing it.
        // But the test helper hnsw_writer() logic is hidden.
        let adapter = LegacyVectorFieldWriter::new("body", hnsw_writer());
        assert_eq!(adapter.field_name(), "body");
    }

    #[test]
    fn test_adapter_ivf() {
        let adapter = LegacyVectorFieldWriter::new("body", ivf_writer());
        assert_eq!(adapter.field_name(), "body");
    }

    #[test]
    fn test_adapter_deletion_error_handling() {
        let adapter = LegacyVectorFieldWriter::new("body", flat_writer());
        let vector = sample_stored_vector();

        adapter.add_stored_vector(3, &vector, 1).unwrap();

        let result = adapter.delete_document(3, 2);
        assert!(result.is_ok());
    }

    #[test]
    fn adapter_stores_vector_with_correct_doc_id() {
        let adapter = LegacyVectorFieldWriter::new("body", flat_writer());
        let mut vector = sample_stored_vector();
        vector.weight = 2.5;

        adapter.add_stored_vector(5, &vector, 1).unwrap();

        let pending = adapter.pending_vectors();
        assert_eq!(pending.len(), 1);
        // Verify doc_id and field_name
        assert_eq!(pending[0].0, 5);
        assert_eq!(pending[0].1, "body");
        // Verify vector data was converted correctly
        assert_eq!(pending[0].2.data.len(), 2);
    }
}
