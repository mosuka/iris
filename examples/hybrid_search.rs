use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use iris::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use iris::error::{Result, IrisError};
use iris::lexical::core::field::TextOption;
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::document::{DocumentPayload, Payload, StoredVector};
use iris::vector::core::field::{FlatOption, VectorOption};
use iris::vector::core::vector::Vector;
use iris::vector::engine::VectorEngine;
use iris::vector::engine::config::{VectorFieldConfig, VectorIndexConfig};
use iris::vector::engine::request::{
    FusionConfig, LexicalQuery, QueryVector, TermQueryOptions, VectorSearchRequest,
};
use std::any::Any;

// Simple Mock Embedder for the example
#[derive(Debug, Clone)]
struct SimpleEmbedder;

#[async_trait]
impl Embedder for SimpleEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(t) => {
                // Determine vector based on text content for demo purposes
                if t.contains("apple") {
                    Ok(Vector::new(vec![1.0, 0.0, 0.0, 0.0]))
                } else if t.contains("banana") {
                    Ok(Vector::new(vec![0.0, 1.0, 0.0, 0.0]))
                } else if t.contains("orange") {
                    Ok(Vector::new(vec![0.0, 0.0, 1.0, 0.0]))
                } else {
                    Ok(Vector::new(vec![0.0, 0.0, 0.0, 0.0]))
                }
            }
            _ => Err(IrisError::invalid_argument("unsupported input")),
        }
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
        "SimpleEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn main() -> std::result::Result<(), Box<dyn Error>> {
    println!("Hybrid Search Example (Unified VectorEngine)");
    println!("============================================");

    let _rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // 1. Setup Storage
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Vector Engine
    // We define a field "content" that has BOTH vector and lexical indexing enabled.
    let content_config = VectorFieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 4,
            base_weight: 1.0,
            ..Default::default()
        })),
        // Enable lexical indexing for this field using default settings
        lexical: Some(iris::lexical::core::field::FieldOption::Text(
            TextOption::default(),
        )),
    };

    let config = VectorIndexConfig::builder()
        .embedder(SimpleEmbedder)
        .field("content", content_config)
        .default_field("content")
        .build()?;

    let engine = VectorEngine::new(storage.clone(), config)?;

    // 3. Index Documents with External IDs
    // The engine automatically handles splitting into vector and lexical indices.

    // Doc 1: "apple banana" -> "product-1"
    let mut payload1 = DocumentPayload::new();
    // We provide text payload. The engine will:
    // 1. Embed it using SimpleEmbedder -> Vector
    // 2. Index the text lexically -> Inverted Index
    payload1.set_field("content", Payload::text("apple banana"));

    println!("Indexing 'product-1' (apple banana)...");
    engine.index_payloads("product-1", payload1)?;

    // Doc 2: "banana orange" -> "product-2"
    let mut payload2 = DocumentPayload::new();
    payload2.set_field("content", Payload::text("banana orange"));

    println!("Indexing 'product-2' (banana orange)...");
    engine.index_payloads("product-2", payload2)?;

    // Commit changes
    engine.commit()?;

    // 4. Perform Searches

    println!("\n--- Vector Search (Query: 'apple') ---");
    // Vector for "apple" is [1.0, 0.0, 0.0, 0.0] via SimpleEmbedder
    // Should match product-1 (apple banana) closely.
    let vec_req = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: StoredVector::new(Arc::new([1.0, 0.0, 0.0, 0.0])),
            weight: 1.0,
            fields: None,
        }],
        limit: 5,
        ..Default::default()
    };
    let vec_res = engine.search(vec_req)?;
    for hit in vec_res.hits {
        println!("Doc: {}, Score: {}", hit.doc_id, hit.score);
    }

    println!("\n--- Lexical Search (Query: 'orange') ---");
    // Should match product-2
    let lex_req = VectorSearchRequest {
        lexical_query: Some(LexicalQuery::Term(TermQueryOptions {
            field: "content".to_string(),
            term: "orange".to_string(),
            boost: 1.0,
        })),
        limit: 5,
        ..Default::default()
    };
    let lex_res = engine.search(lex_req)?;
    for hit in lex_res.hits {
        println!("Doc: {}, Score: {}", hit.doc_id, hit.score);
    }

    println!("\n--- Hybrid Search (Vector: 'apple', Lexical: 'orange') ---");
    // Vector matches product-1. Lexical matches product-2.
    // RRF Fusion should rank them.
    let hybrid_req = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: StoredVector::new(Arc::new([1.0, 0.0, 0.0, 0.0])),
            weight: 1.0,
            fields: None,
        }],
        lexical_query: Some(LexicalQuery::Term(TermQueryOptions {
            field: "content".to_string(),
            term: "orange".to_string(),
            boost: 1.0,
        })),
        fusion_config: Some(FusionConfig::Rrf { k: 60 }),
        limit: 5,
        ..Default::default()
    };
    let hybrid_res = engine.search(hybrid_req)?;
    for hit in hybrid_res.hits {
        println!("Doc: {}, Score: {}", hit.doc_id, hit.score);
    }

    Ok(())
}
