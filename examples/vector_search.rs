//! Vector Search Example - Basic usage guide via the unified Engine API.
//!
//! This example demonstrates the fundamental steps to use Iris for vector search:
//! 1. Setup storage and configuration with an Embedder
//! 2. Initialize the Engine
//! 3. Add documents with text content (vectors are generated automatically)
//! 4. Perform a nearest neighbor search (KNN) using the unified search API
//!
//! To run this example:
//! ```bash
//! cargo run --example vector_search --features embeddings-candle
//! ```

#[cfg(feature = "embeddings-candle")]
use iris::data::Document;
#[cfg(feature = "embeddings-candle")]
use iris::embedding::candle_bert_embedder::CandleBertEmbedder;
#[cfg(feature = "embeddings-candle")]
use iris::engine::Engine;
#[cfg(feature = "embeddings-candle")]
use iris::engine::config::{FieldConfig, IndexConfig};
#[cfg(feature = "embeddings-candle")]
use iris::engine::search::SearchRequestBuilder;
#[cfg(feature = "embeddings-candle")]
use iris::error::Result;
#[cfg(feature = "embeddings-candle")]
use iris::lexical::core::field::{FieldOption, TextOption};
#[cfg(feature = "embeddings-candle")]
use iris::storage::file::FileStorageConfig;
#[cfg(feature = "embeddings-candle")]
use iris::storage::{StorageConfig, StorageFactory};
#[cfg(feature = "embeddings-candle")]
use iris::vector::core::field::{FlatOption, VectorOption};
#[cfg(feature = "embeddings-candle")]
use iris::vector::store::query::VectorSearchRequestBuilder;
#[cfg(feature = "embeddings-candle")]
use std::sync::Arc;
#[cfg(feature = "embeddings-candle")]
use tempfile::TempDir;

#[cfg(feature = "embeddings-candle")]
fn main() -> Result<()> {
    println!("=== Vector Search Example (Unified Engine + Candle BERT) ===\n");

    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Embedder
    // We use "sentence-transformers/all-MiniLM-L6-v2" (384-dimensional).
    println!("Loading BERT model (this may take a while on first run)...");
    let embedder = Arc::new(CandleBertEmbedder::new(
        "sentence-transformers/all-MiniLM-L6-v2",
    )?);

    // 3. Configure Index via Engine
    let config = IndexConfig::builder()
        .embedder(embedder)
        // Define "content" as a field with both vector and lexical indexing
        .add_field(
            "content",
            FieldConfig {
                vector: Some(VectorOption::Flat(FlatOption {
                    dimension: 384,
                    ..Default::default()
                })),
                lexical: Some(FieldOption::Text(TextOption::default())),
            },
        )
        // metadata fields
        .add_lexical_field("category", FieldOption::Text(TextOption::default()))
        .build();

    // 4. Create Engine
    let engine = Engine::new(storage, config)?;

    // 5. Add Documents
    let docs = vec![
        ("doc1", "The Rust Programming Language", "TECHNOLOGY"),
        ("doc2", "Learning Search Engines", "EDUCATION"),
        ("doc3", "Cooking with Rust (Iron Skillets)", "LIFESTYLE"),
    ];

    println!("Indexing {} documents...", docs.len());
    for (id, content, cat) in docs {
        let doc = Document::new()
            .with_id(id)
            .with_field("content", content)
            .with_field("category", cat);
        engine.index(doc)?;
    }
    engine.commit()?;

    // 6. Search
    println!("\n--- Vector Search: 'Rust' in 'content' ---");
    // We use VectorSearchRequestBuilder to build the vector part of the query.
    // The Engine will automatically embed the query text.
    let request = SearchRequestBuilder::new()
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("content", "Rust")
                .build(),
        )
        .build();

    let results = engine.search(request)?;

    println!("Found {} hits:", results.len());
    for (i, hit) in results.iter().enumerate() {
        if let Ok(Some(doc)) = engine.get_document(hit.doc_id) {
            let content = doc
                .get_field("content")
                .and_then(|v| v.as_text())
                .unwrap_or("");
            println!(
                "{}. ID: {}, Content: '{}', Score: {:.4}",
                i + 1,
                doc.id.as_deref().unwrap_or("unknown"),
                content,
                hit.score
            );
        }
    }

    Ok(())
}

#[cfg(not(feature = "embeddings-candle"))]
fn main() {
    println!("This example requires the 'embeddings-candle' feature.");
    println!("Please run with: cargo run --example vector_search --features embeddings-candle");
}
