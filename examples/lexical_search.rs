//! Lexical Search Example - Basic usage guide via the unified Engine API.
//!
//! This example demonstrates the fundamental steps to use Iris for lexical search:
//! 1. Setup storage and configuration
//! 2. Initialize the Engine
//! 3. Add documents
//! 4. Perform a search using the Engine's search API

use iris::data::Document;
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::engine::search::SearchRequestBuilder;
use iris::error::Result;
use iris::lexical::core::field::{FieldOption, TextOption};
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use tempfile::TempDir;

fn main() -> Result<()> {
    println!("=== Lexical Search Basic Example (Unified Engine) ===\n");

    // 1. Setup Storage
    // We use a temporary directory for this example, but in a real app you'd use a persistent path.
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Index via Engine
    // We define our fields and their lexical indexing options.
    let config = IndexConfig::builder()
        .add_field(
            "title",
            FieldConfig {
                lexical: Some(FieldOption::Text(TextOption::default())),
                vector: None,
            },
        )
        .add_field(
            "body",
            FieldConfig {
                lexical: Some(FieldOption::Text(TextOption::default())),
                vector: None,
            },
        )
        .add_field(
            "category",
            FieldConfig {
                lexical: Some(FieldOption::Text(TextOption::default())),
                vector: None,
            },
        )
        .build();

    // 3. Create Engine
    let engine = Engine::new(storage, config)?;

    // 4. Add Documents
    // Let's index a few simple documents.
    let documents = vec![
        Document::new_with_id("doc1")
            .add_field("title", "The Rust Programming Language")
            .add_field("body", "Rust is fast and memory efficient.")
            .add_field("category", "TECHNOLOGY"),
        Document::new_with_id("doc2")
            .add_field("title", "Learning Search Engines")
            .add_field("body", "Search engines are complex but fascinating.")
            .add_field("category", "EDUCATION"),
        Document::new_with_id("doc3")
            .add_field("title", "Cooking with Rust (Iron Skillets)")
            .add_field("body", "How to season your cast iron skillet.")
            .add_field("category", "LIFESTYLE"),
    ];

    println!("Indexing {} documents...", documents.len());
    for doc in documents {
        engine.index(doc)?;
    }
    // Commit changes to make them searchable.
    engine.commit()?;

    // 5. Search

    println!("\n--- Search 1: 'Rust' in 'title' ---");
    // We construct a Lexical search request using a TermQuery.
    let request = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("title", "rust")))
        .build();

    let results = engine.search(request)?;

    println!("Found {} hits:", results.len());
    for (i, hit) in results.iter().enumerate() {
        // We can retrieve the actual document content using engine.get_document()
        if let Ok(Some(doc)) = engine.get_document(hit.doc_id) {
            let title = doc
                .get_field("title")
                .and_then(|v| v.as_text())
                .unwrap_or("");
            let category = doc
                .get_field("category")
                .and_then(|v| v.as_text())
                .unwrap_or("");
            println!(
                "{}. ID: {}, Title: '{}', Category: {}, Score: {:.4}",
                i + 1,
                doc.id.as_deref().unwrap_or("unknown"),
                title,
                category,
                hit.score
            );
        }
    }

    Ok(())
}
