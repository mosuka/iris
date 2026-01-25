//! External ID Support Demo via the unified Engine API.
//!
//! This example demonstrates how Iris handles document updates and deletions
//! using a system-reserved `_id` field.

use std::sync::Arc;

use async_trait::async_trait;
use iris::data::Document;
use iris::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::error::Result;
use iris::lexical::core::field::{FieldOption, TextOption};
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::field::{FlatOption, VectorOption};
use iris::vector::core::vector::Vector;
use std::any::Any;

// Simple Mock Embedder
#[derive(Debug, Clone)]
struct SimpleEmbedder;

#[async_trait]
impl Embedder for SimpleEmbedder {
    async fn embed(&self, _input: &EmbedInput<'_>) -> Result<Vector> {
        Ok(Vector::new(vec![1.0, 0.0, 0.0]))
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
    println!("=== External ID Support Demo (Unified Engine) ===\n");

    // 1. Initialize Engine
    println!("-> Initializing Engine...");
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    let config = IndexConfig::builder()
        .embedder(Arc::new(SimpleEmbedder))
        .add_field(
            "description",
            FieldConfig {
                vector: Some(VectorOption::Flat(FlatOption {
                    dimension: 3,
                    ..Default::default()
                })),
                lexical: Some(FieldOption::Text(TextOption::default())),
            },
        )
        .build();

    let engine = Engine::new(storage, config)?;

    // 2. Index Documents
    // "product-A": "Green Apple"
    println!("-> Indexing 'product-A'...");
    let doc_a = Document::new()
        .with_id("product-A")
        .with_field("description", "Green Apple");
    engine.index(doc_a)?;

    // "product-B": "Yellow Banana"
    println!("-> Indexing 'product-B'...");
    let doc_b = Document::new()
        .with_id("product-B")
        .with_field("description", "Yellow Banana");
    engine.index(doc_b)?;

    engine.commit()?;

    // 3. Update Document
    // Change product-A to "Red Apple" (same ID)
    // The Engine will automatically detect the same ID and replace the old document.
    println!("\n-> Updating 'product-A' to 'Red Apple'...");
    let doc_a_new = Document::new()
        .with_id("product-A")
        .with_field("description", "Red Apple");
    engine.index(doc_a_new)?;
    engine.commit()?;

    // 4. Delete Document
    println!("-> Deleting 'product-B'...");
    engine.delete("product-B")?;
    engine.commit()?;

    println!("\nDemo completed. Document management is handled via the unified Engine API.");
    Ok(())
}
