//! Unified Search Example — Chunk-based Document Registration
//!
//! This example demonstrates how to:
//! 1. Register multiple chunks per document using `add_document()`.
//! 2. Attach per-chunk metadata (page number, category) as Document fields.
//! 3. Perform Vector Search across chunks.
//! 4. Use Lexical filters to narrow Vector Search results (filtered vector search).
//! 5. Perform Hybrid Search combining lexical and vector queries.
//!
//! Each chunk is stored as an independent internal document sharing the same
//! external ID (`_id`), following the same non-normalized pattern used by
//! Qdrant, Pinecone, and other vector databases.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use iris::Document;
use iris::Engine;
use iris::Result;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::core::field::{IntegerOption, NumericType};
use iris::lexical::{NumericRangeQuery, TermQuery, TextOption};
use iris::{EmbedInput, EmbedInputType, Embedder, PerFieldEmbedder};
use iris::{FusionAlgorithm, Schema, SearchRequestBuilder};
use serde_json::json;

use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::FlatOption;
use iris::vector::Vector;
use iris::vector::VectorSearchRequestBuilder;

// --- Mock Embedder ---
// Converts text into a 4-dimensional vector based on keyword matching.
// In production, this would be a real embedding model (e.g., BERT, OpenAI).

#[derive(Debug, Clone)]
struct MockEmbedder;

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(t) => {
                let t = t.to_lowercase();
                // Semantic dimensions: [systems, language, memory, concurrency]
                let mut vec = [0.0f32; 4];
                if t.contains("ownership") || t.contains("borrow") || t.contains("lifetime") {
                    vec = [0.2, 0.3, 0.9, 0.1]; // Memory safety
                } else if t.contains("async") || t.contains("thread") || t.contains("concurrent") {
                    vec = [0.3, 0.1, 0.2, 0.9]; // Concurrency
                } else if t.contains("type") || t.contains("generic") || t.contains("trait") {
                    vec = [0.4, 0.8, 0.2, 0.1]; // Type system
                } else if t.contains("cargo") || t.contains("crate") || t.contains("build") {
                    vec = [0.8, 0.3, 0.1, 0.2]; // Build system
                } else if t.contains("memory") || t.contains("safe") {
                    vec = [0.3, 0.2, 0.8, 0.2]; // Memory
                }
                Ok(Vector::new(vec.to_vec()))
            }
            _ => Ok(Vector::new(vec![0.0; 4])),
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
        "MockEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn main() -> Result<()> {
    println!("=== Iris Chunk-based Document Search Example ===\n");

    // 1. Setup Storage (In-memory for this example)
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // 2. Define Schema
    //
    // Lexical fields:
    //   - "title"      : Document title (keyword, exact match)
    //   - "text"  : Chunk content (tokenized, full-text search)
    //   - "category"    : Category label (keyword, for filtering)
    //   - "page"        : Page number (integer, for range filtering)
    //
    // Vector field:
    //   - "text_vec"   : Auto-embedded from text (4-dim, flat index)
    let schema = Schema::builder()
        .add_text_field("title", TextOption::default())
        .add_text_field("text", TextOption::default())
        .add_text_field("category", TextOption::default())
        .add_integer_field("page", IntegerOption::default())
        .add_flat_field("text_vec", FlatOption::default().dimension(4))
        .add_default_field("text")
        .build();

    // Analyzers: "title" and "category" use KeywordAnalyzer (exact match),
    // "text" uses StandardAnalyzer (tokenization + lowercasing).
    let std_analyzer = Arc::new(StandardAnalyzer::default());
    let kw_analyzer = Arc::new(KeywordAnalyzer::new());

    let mut per_field_analyzer = PerFieldAnalyzer::new(std_analyzer.clone());
    per_field_analyzer.add_analyzer("title", kw_analyzer.clone());
    per_field_analyzer.add_analyzer("category", kw_analyzer.clone());
    per_field_analyzer.add_analyzer("text", std_analyzer.clone());

    // Embedder for the "text_vec" vector field
    let embedder = Arc::new(MockEmbedder);
    let mut per_field_embedder = PerFieldEmbedder::new(embedder.clone());
    per_field_embedder.add_embedder("text_vec", embedder.clone());

    let engine = Engine::builder(storage, schema)
        .analyzer(Arc::new(per_field_analyzer))
        .embedder(Arc::new(per_field_embedder))
        .build()?;

    // 3. Register Chunked Documents
    //
    // Source document: "The Rust Programming Language" (3 chapters, 6 chunks)
    // Each chunk shares the same external ID but has its own page, text, and vector.

    println!("--- Indexing chunked documents ---\n");

    let books = json!([
        {
            "id": "book_a",
            "title": "The Rust Programming Language",
            "chunks": [
                { "text": "Chapter 1: Getting Started", "page": 1, "category": "basics" },
                { "text": "Cargo is the Rust build system and package manager. Use cargo new to create a crate.", "page": 2, "category": "basics" },
                { "text": "Every value in Rust has an owner. Ownership rules prevent data races at compile time.", "page": 3, "category": "memory" },
                { "text": "References and borrowing let you use values without taking ownership of them.", "page": 4, "category": "memory" },
                { "text": "Generic types and trait bounds enable polymorphism without runtime overhead.", "page": 5, "category": "type-system" },
                { "text": "Async functions and tokio provide concurrent programming with lightweight tasks and threads.", "page": 6, "category": "concurrency" }
            ]
        },
        {
            "id": "book_b",
            "title": "Programming in Rust",
            "chunks": [
                { "text": "Rust's type system catches many bugs at compile time. Trait objects enable dynamic dispatch.", "page": 1, "category": "type-system" },
                { "text": "The borrow checker ensures memory safety without garbage collection. Lifetime annotations help.", "page": 2, "category": "memory" },
                { "text": "Rust async/await provides zero-cost concurrency for building scalable concurrent network services.", "page": 3, "category": "concurrency" }
            ]
        }
    ]);

    let mut total_chunks = 0;
    for book in books.as_array().unwrap() {
        let id = book["id"].as_str().unwrap();
        let title = book["title"].as_str().unwrap();
        for chunk in book["chunks"].as_array().unwrap() {
            let text = chunk["text"].as_str().unwrap();
            let page = chunk["page"].as_i64().unwrap();
            let category = chunk["category"].as_str().unwrap();
            let doc = Document::builder()
                .add_text("title", title)
                .add_text("text", text)
                .add_text("category", category)
                .add_integer("page", page)
                .add_text("text_vec", text) // Auto-embedded by MockEmbedder
                .build();
            engine.add_document(id, doc)?;
            total_chunks += 1;
        }
    }

    engine.commit()?;
    println!(
        "Indexed {} books as {} chunks total.\n",
        books.as_array().unwrap().len(),
        total_chunks
    );

    // 4. Case A: Vector Search (find chunks semantically similar to "memory safety")
    println!("[Case A] Vector Search: 'memory safety'");
    println!("  → Finds chunks about ownership, borrowing, lifetimes\n");

    let results = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_text("text_vec", "memory safety")
                    .build(),
            )
            .limit(3)
            .build(),
    )?;
    print_results(&results);

    // 5. Case B: Filtered Vector Search (memory safety, but only "concurrency" category)
    println!("\n[Case B] Filtered Vector Search: 'memory safety' + category='concurrency'");
    println!("  → Narrows results to concurrency-related chunks only\n");

    let results = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_text("text_vec", "memory safety")
                    .build(),
            )
            .filter(Box::new(TermQuery::new("category", "concurrency")))
            .limit(3)
            .build(),
    )?;
    print_results(&results);

    // 6. Case C: Filtered Vector Search with numeric range (pages 1-3 only)
    println!("\n[Case C] Filtered Vector Search: 'type system' + page <= 3");
    println!("  → Searches early pages only\n");

    let results = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_text("text_vec", "type system")
                    .build(),
            )
            .filter(Box::new(NumericRangeQuery::new(
                "page",
                NumericType::Integer,
                Some(1.0),
                Some(3.0),
                true,
                true,
            )))
            .limit(3)
            .build(),
    )?;
    print_results(&results);

    // 7. Case D: Lexical Search (keyword: "ownership")
    println!("\n[Case D] Lexical Search: 'ownership'");
    println!("  → Exact keyword match across all chunks\n");

    let results = engine.search(
        SearchRequestBuilder::new()
            .with_lexical(Box::new(TermQuery::new("text", "ownership")))
            .limit(3)
            .build(),
    )?;
    print_results(&results);

    // 8. Case E: Hybrid Search (RRF Fusion)
    println!("\n[Case E] Hybrid Search (RRF): vector='concurrent' + lexical='async'");
    println!("  → Combines semantic similarity with keyword match\n");

    let results = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_text("text_vec", "concurrent")
                    .build(),
            )
            .with_lexical(Box::new(TermQuery::new("text", "async")))
            .fusion(FusionAlgorithm::RRF { k: 60.0 })
            .limit(3)
            .build(),
    )?;
    print_results(&results);

    Ok(())
}

/// Print search results with chunk metadata.
fn print_results(results: &[iris::SearchResult]) {
    if results.is_empty() {
        println!("  (No results found)");
        return;
    }
    for (i, hit) in results.iter().enumerate() {
        let doc = hit.document.as_ref();
        let title = doc
            .and_then(|d| d.get("title"))
            .and_then(|v| v.as_text())
            .unwrap_or("?");
        let page = doc
            .and_then(|d| d.get("page"))
            .and_then(|v| v.as_integer())
            .map(|p| p.to_string())
            .unwrap_or("?".into());
        let category = doc
            .and_then(|d| d.get("category"))
            .and_then(|v| v.as_text())
            .unwrap_or("?");
        let text_content = doc
            .and_then(|d| d.get("text"))
            .and_then(|v| v.as_text())
            .unwrap_or("?");

        // Truncate long text for display
        let display_text = if text_content.len() > 60 {
            format!("{}...", &text_content[..60])
        } else {
            text_content.to_string()
        };

        println!(
            "  {}. [{}] p.{} [{}] (score: {:.4})",
            i + 1,
            title,
            page,
            category,
            hit.score
        );
        println!("     {}", display_text);
    }
}
