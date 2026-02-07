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
    //   - "chunk_text"  : Chunk content (tokenized, full-text search)
    //   - "category"    : Category label (keyword, for filtering)
    //   - "page"        : Page number (integer, for range filtering)
    //
    // Vector field:
    //   - "embedding"   : Auto-embedded from chunk_text (4-dim, flat index)
    let schema = Schema::builder()
        .add_text_field("title", TextOption::default())
        .add_text_field("chunk_text", TextOption::default())
        .add_text_field("category", TextOption::default())
        .add_integer_field("page", IntegerOption::default())
        .add_flat_field("embedding", FlatOption::default().dimension(4))
        .add_default_field("chunk_text")
        .build();

    // Analyzers: "title" and "category" use KeywordAnalyzer (exact match),
    // "chunk_text" uses StandardAnalyzer (tokenization + lowercasing).
    let std_analyzer = Arc::new(StandardAnalyzer::default());
    let kw_analyzer = Arc::new(KeywordAnalyzer::new());

    let mut per_field_analyzer = PerFieldAnalyzer::new(std_analyzer.clone());
    per_field_analyzer.add_analyzer("title", kw_analyzer.clone());
    per_field_analyzer.add_analyzer("category", kw_analyzer.clone());
    per_field_analyzer.add_analyzer("chunk_text", std_analyzer.clone());

    // Embedder for the "embedding" vector field
    let embedder = Arc::new(MockEmbedder);
    let mut per_field_embedder = PerFieldEmbedder::new(embedder.clone());
    per_field_embedder.add_embedder("embedding", embedder.clone());

    let engine = Engine::builder(storage, schema)
        .analyzer(Arc::new(per_field_analyzer))
        .embedder(Arc::new(per_field_embedder))
        .build()?;

    // 3. Register Chunked Documents
    //
    // Source document: "The Rust Programming Language" (3 chapters, 6 chunks)
    // Each chunk shares the same external ID but has its own page, text, and vector.

    println!("--- Indexing chunked documents ---\n");

    // Book A: "The Rust Programming Language"
    let book_a_chunks = [
        ("Chapter 1: Getting Started", 1, "basics"),
        (
            "Cargo is the Rust build system and package manager. Use cargo new to create a crate.",
            2,
            "basics",
        ),
        (
            "Every value in Rust has an owner. Ownership rules prevent data races at compile time.",
            3,
            "memory",
        ),
        (
            "References and borrowing let you use values without taking ownership of them.",
            4,
            "memory",
        ),
        (
            "Generic types and trait bounds enable polymorphism without runtime overhead.",
            5,
            "type-system",
        ),
        (
            "Async functions and tokio provide concurrent programming with lightweight tasks and threads.",
            6,
            "concurrency",
        ),
    ];

    for (text, page, category) in &book_a_chunks {
        let doc = Document::builder()
            .add_text("title", "The Rust Programming Language")
            .add_text("chunk_text", *text)
            .add_text("category", *category)
            .add_integer("page", *page as i64)
            .add_text("embedding", *text) // Auto-embedded by MockEmbedder
            .build();
        engine.add_document("book_a", doc)?;
    }

    // Book B: "Programming in Rust" (3 chunks)
    let book_b_chunks = [
        (
            "Rust's type system catches many bugs at compile time. Trait objects enable dynamic dispatch.",
            1,
            "type-system",
        ),
        (
            "The borrow checker ensures memory safety without garbage collection. Lifetime annotations help.",
            2,
            "memory",
        ),
        (
            "Rust async/await provides zero-cost concurrency for building scalable concurrent network services.",
            3,
            "concurrency",
        ),
    ];

    for (text, page, category) in &book_b_chunks {
        let doc = Document::builder()
            .add_text("title", "Programming in Rust")
            .add_text("chunk_text", *text)
            .add_text("category", *category)
            .add_integer("page", *page as i64)
            .add_text("embedding", *text)
            .build();
        engine.add_document("book_b", doc)?;
    }

    engine.commit()?;
    println!(
        "Indexed 2 books as {} chunks total.\n",
        book_a_chunks.len() + book_b_chunks.len()
    );

    // Note: VectorSearchRequestBuilder::add_vector() is used for pre-computed
    // query vectors. The MockEmbedder maps concepts to these vectors:
    //   Memory safety  → [0.3, 0.2, 0.8, 0.2]
    //   Concurrency    → [0.3, 0.1, 0.2, 0.9]
    //   Type system    → [0.4, 0.8, 0.2, 0.1]
    //   Build system   → [0.8, 0.3, 0.1, 0.2]

    // 4. Case A: Vector Search (find chunks semantically similar to "memory safety")
    println!("[Case A] Vector Search: 'memory safety'");
    println!("  → Finds chunks about ownership, borrowing, lifetimes\n");

    let results = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_vector("embedding", vec![0.3, 0.2, 0.8, 0.2])
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
                    .add_vector("embedding", vec![0.3, 0.2, 0.8, 0.2])
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
                    .add_vector("embedding", vec![0.4, 0.8, 0.2, 0.1])
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
            .with_lexical(Box::new(TermQuery::new("chunk_text", "ownership")))
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
                    .add_vector("embedding", vec![0.3, 0.1, 0.2, 0.9])
                    .build(),
            )
            .with_lexical(Box::new(TermQuery::new("chunk_text", "async")))
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
        let chunk_text = doc
            .and_then(|d| d.get("chunk_text"))
            .and_then(|v| v.as_text())
            .unwrap_or("?");

        // Truncate long text for display
        let display_text = if chunk_text.len() > 60 {
            format!("{}...", &chunk_text[..60])
        } else {
            chunk_text.to_string()
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
