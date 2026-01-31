//! Unified Embedding Vector Writer Prototype.
//!
//! This module implements a wrapper around `VectorIndexWriter` that handles automated
//! embedding of content, bridging the gap between high-level document operations
//! and low-level vector indexing.

use std::sync::Arc;

use crate::data::DataValue;
use crate::embedding::embedder::{EmbedInput, Embedder};
use crate::error::{IrisError, Result};
use crate::vector::core::vector::Vector;
use crate::vector::store::embedder::EmbedderExecutor;
use crate::vector::writer::VectorIndexWriter;

/// A wrapper around a VectorIndexWriter that automatically handles content embedding.
pub struct EmbeddingVectorIndexWriter {
    inner: Box<dyn VectorIndexWriter>,
    embedder: Arc<dyn Embedder>,
    executor: Arc<EmbedderExecutor>,
}

impl EmbeddingVectorIndexWriter {
    /// Create a new embedding vector index writer.
    pub fn new(
        inner: Box<dyn VectorIndexWriter>,
        embedder: Arc<dyn Embedder>,
        executor: Arc<EmbedderExecutor>,
    ) -> Self {
        Self {
            inner,
            embedder,
            executor,
        }
    }

    /// Add a value (Text, ImageBytes, etc.) to the index, embedding it automatically.
    pub fn add_value(&mut self, doc_id: u64, field_name: String, value: DataValue) -> Result<()> {
        // If it's already a vector, bypass embedding
        if let DataValue::Vector(v) = value {
            return self
                .inner
                .add_vectors(vec![(doc_id, field_name, Vector::new(v))]);
        }

        // Validate input type compat before async block to save time
        match &value {
            DataValue::Text(_) if !self.embedder.supports_text() => {
                return Err(IrisError::invalid_argument(format!(
                    "Embedder '{}' does not support text input",
                    self.embedder.name()
                )));
            }
            DataValue::Bytes(_, mime) if !self.embedder.supports_image() => {
                if mime.as_ref().map_or(false, |m| m.starts_with("image/")) {
                    return Err(IrisError::invalid_argument(format!(
                        "Embedder '{}' does not support image input",
                        self.embedder.name()
                    )));
                }
            }
            _ => {
                // Other types not supported for now unless embedder supports custom
            }
        }

        // Prepare owned data for the async block
        let (text_owned, bytes_owned, mime_owned) = match value {
            DataValue::Text(t) => (Some(t), None, None),
            DataValue::Bytes(b, m) => (None, Some(b), m),
            _ => {
                return Err(IrisError::invalid_argument(
                    "Unsupported data type for embedding",
                ));
            }
        };

        let embedder = self.embedder.clone();

        // Run embedding in the executor
        // We construct EmbedInput inside the closure to manage lifetimes
        let vector = self.executor.run(async move {
            let input = if let Some(ref text) = text_owned {
                EmbedInput::Text(text)
            } else if let Some(ref bytes) = bytes_owned {
                EmbedInput::Bytes(bytes, mime_owned.as_deref())
            } else {
                return Err(IrisError::internal("Unreachable state in embedding writer"));
            };

            embedder.embed(&input).await
        })?;

        // Add the resulting vector to the underlying writer
        self.inner.add_vectors(vec![(doc_id, field_name, vector)])
    }
}

// Implement VectorIndexWriter trait methods by delegating to inner
impl std::fmt::Debug for EmbeddingVectorIndexWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingVectorIndexWriter")
            .field("inner", &self.inner)
            .field("embedder", &self.embedder.name())
            .finish()
    }
}

// We can optionally implement VectorIndexWriter for this wrapper too,
// to allow seamless usage where a Writer is expected.
impl VectorIndexWriter for EmbeddingVectorIndexWriter {
    fn next_vector_id(&self) -> u64 {
        self.inner.next_vector_id()
    }

    fn add_value(
        &mut self,
        doc_id: u64,
        field_name: String,
        value: crate::data::DataValue,
    ) -> Result<()> {
        self.add_value(doc_id, field_name, value)
    }

    fn build(&mut self, vectors: Vec<(u64, String, Vector)>) -> Result<()> {
        self.inner.build(vectors)
    }

    fn add_vectors(&mut self, vectors: Vec<(u64, String, Vector)>) -> Result<()> {
        self.inner.add_vectors(vectors)
    }

    fn finalize(&mut self) -> Result<()> {
        self.inner.finalize()
    }

    fn progress(&self) -> f32 {
        self.inner.progress()
    }

    fn estimated_memory_usage(&self) -> usize {
        self.inner.estimated_memory_usage()
    }

    fn vectors(&self) -> &[(u64, String, Vector)] {
        self.inner.vectors()
    }

    fn write(&self) -> Result<()> {
        self.inner.write()
    }

    fn has_storage(&self) -> bool {
        self.inner.has_storage()
    }

    fn delete_document(&mut self, doc_id: u64) -> Result<()> {
        self.inner.delete_document(doc_id)
    }

    fn commit(&mut self) -> Result<()> {
        self.inner.commit()
    }

    fn rollback(&mut self) -> Result<()> {
        self.inner.rollback()
    }

    fn pending_docs(&self) -> u64 {
        self.inner.pending_docs()
    }

    fn close(&mut self) -> Result<()> {
        self.inner.close()
    }

    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    fn build_reader(&self) -> Result<Arc<dyn crate::vector::reader::VectorIndexReader>> {
        self.inner.build_reader()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    use crate::vector::index::config::FlatIndexConfig;
    use crate::vector::index::flat::writer::FlatIndexWriter;
    use crate::vector::writer::VectorIndexWriterConfig;
    use std::any::Any;

    #[derive(Debug)]
    struct MockEmbedder;

    #[async_trait::async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
            match input {
                EmbedInput::Text(_) => Ok(Vector::new(vec![1.0, 2.0, 3.0])),
                _ => Ok(Vector::new(vec![0.0, 0.0, 0.0])),
            }
        }

        fn supported_input_types(&self) -> Vec<crate::embedding::embedder::EmbedInputType> {
            vec![crate::embedding::embedder::EmbedInputType::Text]
        }

        fn name(&self) -> &str {
            "mock"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_embedding_writer() {
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let mut index_config = FlatIndexConfig::default();
        index_config.dimension = 3;
        index_config.normalize_vectors = false;
        let inner = Box::new(
            FlatIndexWriter::with_storage(
                index_config,
                VectorIndexWriterConfig::default(),
                "test".to_string(),
                storage,
            )
            .unwrap(),
        );

        let embedder = Arc::new(MockEmbedder);
        let executor = Arc::new(EmbedderExecutor::new().unwrap());

        let mut writer = EmbeddingVectorIndexWriter::new(inner, embedder, executor);

        writer
            .add_value(1, "field".to_string(), DataValue::Text("hello".to_string()))
            .unwrap();

        // Finalize to make vectors available? FlatIndexWriter might buffer.
        // But FlatIndexWriter doesn't store in memory for vectors() call unless raw storage is used?
        // Actually vectors() returns &[(...)]

        assert_eq!(writer.vectors().len(), 1);
        assert_eq!(writer.vectors()[0].2.data, vec![1.0, 2.0, 3.0]);
    }
}
