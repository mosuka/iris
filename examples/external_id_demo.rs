use std::sync::Arc;

use async_trait::async_trait;
use sarissa::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use sarissa::error::Result;
use sarissa::lexical::core::field::TextOption;
use sarissa::storage::memory::MemoryStorageConfig;
use sarissa::storage::{StorageConfig, StorageFactory};
use sarissa::vector::core::document::{DocumentPayload, Payload, StoredVector};
use sarissa::vector::core::vector::Vector;
use sarissa::vector::engine::VectorEngine;
use sarissa::vector::engine::config::{
    FlatOption, VectorFieldConfig, VectorOption, VectorIndexConfig,
};
use sarissa::vector::engine::request::{QueryVector, VectorSearchRequest};
use std::any::Any;

// Simple Mock Embedder
#[derive(Debug, Clone)]
struct SimpleEmbedder;

#[async_trait]
impl Embedder for SimpleEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(t) => {
                let t_lower = t.to_lowercase();
                // Mock embedding logic
                if t_lower.contains("apple") {
                    Ok(Vector::new(vec![1.0, 0.0, 0.0]))
                } else if t_lower.contains("banana") {
                    Ok(Vector::new(vec![0.0, 1.0, 0.0]))
                } else {
                    Ok(Vector::new(vec![0.0, 0.0, 0.0]))
                }
            }
            _ => Ok(Vector::new(vec![0.0, 0.0, 0.0])),
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

fn main() -> Result<()> {
    println!("=== External ID Support Demo (Unified VectorEngine) ===\n");

    let _rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // 1. Initialize Engine
    println!("-> Initializing Vector Engine...");
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // Configure "description" field to be both Vector and Lexical
    let description_config = VectorFieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 3,
            base_weight: 1.0,
            ..Default::default()
        })),
        lexical: Some(sarissa::lexical::core::field::FieldOption::Text(
            TextOption::default(),
        )),
    };

    let config = VectorIndexConfig::builder()
        .embedder(SimpleEmbedder)
        .field("description", description_config)
        .default_field("description")
        .build()?;

    let engine = VectorEngine::new(storage, config)?;

    // 2. Index Documents with External IDs
    // "product-A": "Green Apple"
    println!("-> Indexing 'product-A'...");
    let mut payload_a = DocumentPayload::new();
    payload_a.set_field("description", Payload::text("Green Apple"));
    engine.index_payloads("product-A", payload_a)?;

    // "product-B": "Yellow Banana"
    println!("-> Indexing 'product-B'...");
    let mut payload_b = DocumentPayload::new();
    payload_b.set_field("description", Payload::text("Yellow Banana"));
    engine.index_payloads("product-B", payload_b)?;

    engine.commit()?;

    // 3. Search by Vector (Query "Apple" -> [1,0,0])
    println!("\n-> Searching for 'Apple' (Vector Search)...");
    let search_req = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            // Mock query vector for "Apple"
            vector: StoredVector::new(Arc::new([1.0, 0.0, 0.0])),
            weight: 1.0,
            fields: None,
        }],
        limit: 5,
        ..Default::default()
    };
    let results = engine.search(search_req)?;
    for hit in results.hits {
        println!("   Hit: {} (Score: {})", hit.doc_id, hit.score);
    }

    // 4. Update Document
    // Change product-A to "Red Apple" (same ID)
    println!("\n-> Updating 'product-A' to 'Red Apple'...");
    let mut payload_a_new = DocumentPayload::new();
    payload_a_new.set_field("description", Payload::text("Red Apple"));
    engine.index_payloads("product-A", payload_a_new)?;
    engine.commit()?;

    // 5. Verify Update
    println!("-> Verifying Update...");
    // ... verification logic depends on searching again or inspecting.

    // 6. Delete Document
    println!("\n-> Deleting 'product-B' feature is pending API update.");

    Ok(())
}
