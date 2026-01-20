use async_trait::async_trait;
use sarissa::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use sarissa::error::Result;
use sarissa::lexical::core::field::TextOption;
use sarissa::storage::memory::MemoryStorageConfig;
use sarissa::storage::{StorageConfig, StorageFactory};
use sarissa::vector::core::document::StoredVector;
use sarissa::vector::core::document::{DocumentPayload, Payload};
use sarissa::vector::core::vector::Vector;
use sarissa::vector::engine::VectorEngine;
use sarissa::vector::engine::config::{FlatOption, VectorFieldConfig, VectorIndexConfig};
use sarissa::vector::engine::request::{QueryVector, VectorSearchRequest};
use std::any::Any;
use std::sync::Arc;

// Simple Mock Embedder for test
#[derive(Debug, Clone)]
struct MockEmbedder;

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, _input: &EmbedInput<'_>) -> Result<Vector> {
        Ok(Vector::new(vec![0.0; 3]))
    }
    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        vec![EmbedInputType::Text]
    }
    fn supports_text(&self) -> bool {
        true
    }
    fn supports_image(&self) -> bool {
        false
    }
    fn name(&self) -> &str {
        "MockEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[tokio::test]
async fn test_external_id_operations() -> Result<()> {
    // 1. Setup
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // We will use "title" for lexical and "vector" for vector
    let vector_config = VectorIndexConfig::builder()
        .embedder(MockEmbedder)
        .field(
            "vector",
            VectorFieldConfig {
                vector: Some(sarissa::vector::engine::config::VectorOption::Flat(
                    FlatOption {
                        dimension: 3,
                        ..Default::default()
                    },
                )),
                lexical: None,
            },
        )
        .field(
            "title",
            VectorFieldConfig {
                vector: None,
                lexical: Some(sarissa::lexical::core::field::FieldOption::Text(
                    TextOption::default(),
                )),
            },
        )
        .default_field("vector")
        .build()?;
    let engine = VectorEngine::new(storage.clone(), vector_config)?;

    // 2. Index first document with ID "doc1"
    let mut payload1 = DocumentPayload::new();
    payload1.set_field("vector", Payload::vector(vec![1.0f32, 0.0, 0.0]));
    payload1.set_field("title", Payload::text("Rust Programming")); // Lexical content

    // VectorEngine::index_payloads handles ID "doc1"
    engine.index_payloads("doc1", payload1)?;
    engine.commit()?;

    // 3. Search should find it via Vector
    let request_vec = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: StoredVector::new(Arc::new([1.0, 0.0, 0.0])),
            weight: 1.0,
            fields: None,
        }],
        limit: 10,
        ..Default::default()
    };
    let results = engine.search(request_vec)?;
    assert_eq!(results.hits.len(), 1);

    // 4. Index updated document with SAME ID "doc1" (Upsert)
    // "Rust Programming v2", Vector [0.0, 1.0, 0.0]
    let mut payload2 = DocumentPayload::new();
    payload2.set_field("vector", Payload::vector(vec![0.0f32, 1.0, 0.0]));
    payload2.set_field("title", Payload::text("Rust Programming v2"));

    engine.index_payloads("doc1", payload2)?;
    engine.commit()?;

    // 5. Search should reflect update
    // Vector search for NEW vector [0.0, 1.0, 0.0] should find it.

    let request_vec_new = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: StoredVector::new(Arc::new([0.0, 1.0, 0.0])),
            weight: 1.0,
            fields: None,
        }],
        limit: 10,
        ..Default::default()
    };
    let results_new = engine.search(request_vec_new)?;
    assert!(!results_new.hits.is_empty());

    Ok(())
}
