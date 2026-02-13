//! External ID Support Demo via the unified Engine API.
//!
//! This example demonstrates how Iris handles document updates and deletions
//! using a system-reserved `_id` field.

use std::sync::Arc;

use async_trait::async_trait;
use iris::Document;
use iris::Engine;
use iris::Result;
use iris::lexical::{FieldOption as LexicalFieldOption, TextOption};
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::Vector;
use iris::vector::{FlatOption, FieldOption as VectorOption};
use iris::{EmbedInput, EmbedInputType, Embedder};
use iris::{FieldOption, Schema};
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

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== External ID Support Demo (Unified Engine) ===\n");

    // 1. Initialize Engine
    println!("-> Initializing Engine...");
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    let schema = Schema::builder()
        .add_field(
            "description",
            FieldOption::Lexical(LexicalFieldOption::Text(TextOption::default())),
        )
        .add_field(
            "description_vec",
            FieldOption::Vector(VectorOption::Flat(FlatOption {
                dimension: 3,
                ..Default::default()
            })),
        )
        .build();

    let engine = Engine::builder(storage, schema)
        .embedder(Arc::new(SimpleEmbedder))
        .build()
        .await?;

    // 2. Index Documents
    // "product-A": "Green Apple"
    println!("-> Indexing 'product-A'...");
    let doc_a = Document::builder()
        .add_field("description", "Green Apple")
        .add_field("description_vec", "Green Apple")
        .build();
    engine.put_document("product-A", doc_a).await?;

    // "product-B": "Yellow Banana"
    println!("-> Indexing 'product-B'...");
    let doc_b = Document::builder()
        .add_field("description", "Yellow Banana")
        .add_field("description_vec", "Yellow Banana")
        .build();
    engine.put_document("product-B", doc_b).await?;

    engine.commit().await?;

    // 3. Update Document
    // Change product-A to "Red Apple" (same ID)
    // The Engine will automatically detect the same ID and replace the old document.
    println!("\n-> Updating 'product-A' to 'Red Apple'...");
    let doc_a_new = Document::builder()
        .add_field("description", "Red Apple")
        .add_field("description_vec", "Red Apple")
        .build();
    engine.put_document("product-A", doc_a_new).await?;
    engine.commit().await?;

    // 4. Delete Document
    println!("-> Deleting 'product-B'...");
    engine.delete_documents("product-B").await?;
    engine.commit().await?;

    println!("\nDemo completed. Document management is handled via the unified Engine API.");
    Ok(())
}
