//! Hybrid Search Example — combining lexical and vector search.
//!
//! Demonstrates hybrid search that fuses lexical (keyword) and vector
//! (semantic) results using Reciprocal Rank Fusion (RRF):
//!
//! - Lexical-only search (for comparison)
//! - Vector-only search (for comparison)
//! - Hybrid search via Builder API
//! - Hybrid search via DSL
//! - Hybrid search with filter
//!
//! Run with: `cargo run --example hybrid_search`

mod common;

use std::sync::Arc;

use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::core::field::IntegerOption;
use iris::lexical::{QueryParser, TermQuery, TextOption};
use iris::vector::{FlatOption, VectorQueryParser, VectorSearchRequestBuilder};
use iris::{
    Document, Engine, FusionAlgorithm, LexicalSearchRequest, PerFieldEmbedder, Result, Schema,
    SearchRequestBuilder, UnifiedQueryParser,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Iris Hybrid Search Example ===\n");

    // ─── Setup ───────────────────────────────────────────────────────────
    let storage = common::memory_storage()?;

    let schema = Schema::builder()
        .add_text_field("title", TextOption::default())
        .add_text_field("text", TextOption::default())
        .add_text_field("category", TextOption::default())
        .add_integer_field("page", IntegerOption::default())
        .add_flat_field("text_vec", FlatOption::default().dimension(4))
        .add_default_field("text")
        .build();

    let analyzer = common::per_field_analyzer(&["title", "category"]);

    let embedder: Arc<dyn iris::Embedder> = Arc::new(common::MockEmbedder);
    let mut per_field_embedder = PerFieldEmbedder::new(embedder.clone());
    per_field_embedder.add_embedder("text_vec", embedder.clone());
    let per_field_embedder = Arc::new(per_field_embedder);

    let std_analyzer = Arc::new(StandardAnalyzer::default());

    let engine = Engine::builder(storage, schema)
        .analyzer(analyzer)
        .embedder(per_field_embedder.clone())
        .build()
        .await?;

    // ─── Index chunked documents ─────────────────────────────────────────
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
                .add_text("text_vec", text)
                .build();
            engine.add_document(id, doc).await?;
            total_chunks += 1;
        }
    }
    engine.commit().await?;
    println!(
        "Indexed {} books as {} chunks total.\n",
        books.as_array().unwrap().len(),
        total_chunks
    );

    // =====================================================================
    // [A] Lexical-only Search (for comparison)
    // =====================================================================
    println!("{}", "=".repeat(60));
    println!("[A] Lexical-only: term 'ownership' in text");
    println!("{}", "=".repeat(60));
    let results = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "text",
                    "ownership",
                ))))
                .limit(3)
                .build(),
        )
        .await?;
    common::print_search_results(&results);

    // =====================================================================
    // [B] Vector-only Search (for comparison)
    // =====================================================================
    println!("\n{}", "=".repeat(60));
    println!("[B] Vector-only: 'memory safety'");
    println!("{}", "=".repeat(60));
    let results = engine
        .search(
            SearchRequestBuilder::new()
                .vector_search_request(
                    VectorSearchRequestBuilder::new()
                        .add_text("text_vec", "memory safety")
                        .build(),
                )
                .limit(3)
                .build(),
        )
        .await?;
    common::print_search_results(&results);

    // =====================================================================
    // [C] Hybrid Search — Builder API with RRF Fusion
    // =====================================================================
    println!("\n{}", "=".repeat(60));
    println!("[C] Hybrid (RRF): vector='concurrent' + lexical='async'");
    println!("{}", "=".repeat(60));
    let results = engine
        .search(
            SearchRequestBuilder::new()
                .vector_search_request(
                    VectorSearchRequestBuilder::new()
                        .add_text("text_vec", "concurrent")
                        .build(),
                )
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "text", "async",
                ))))
                .fusion_algorithm(FusionAlgorithm::RRF { k: 60.0 })
                .limit(3)
                .build(),
        )
        .await?;
    common::print_search_results(&results);

    // =====================================================================
    // [D] Hybrid Search via DSL
    // =====================================================================
    println!("\n{}", "=".repeat(60));
    println!("[D] Hybrid DSL: text:async text_vec:~\"concurrent\"");
    println!("{}", "=".repeat(60));

    let unified_parser = UnifiedQueryParser::new(
        QueryParser::new(std_analyzer).with_default_field("text"),
        VectorQueryParser::new(per_field_embedder.clone()),
    );

    let mut request = unified_parser
        .parse("text:async text_vec:~\"concurrent\"")
        .await?;
    request.limit = 3;
    let results = engine.search(request).await?;
    common::print_search_results(&results);

    // =====================================================================
    // [E] Hybrid Search with Filter
    // =====================================================================
    println!("\n{}", "=".repeat(60));
    println!(
        "[E] Hybrid + filter: vector='type system' + lexical='trait' + category='type-system'"
    );
    println!("{}", "=".repeat(60));
    let results = engine
        .search(
            SearchRequestBuilder::new()
                .vector_search_request(
                    VectorSearchRequestBuilder::new()
                        .add_text("text_vec", "type system")
                        .build(),
                )
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "text", "trait",
                ))))
                .filter_query(Box::new(TermQuery::new("category", "type-system")))
                .fusion_algorithm(FusionAlgorithm::RRF { k: 60.0 })
                .limit(3)
                .build(),
        )
        .await?;
    common::print_search_results(&results);

    println!("\nHybrid search example completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_search_example() {
        let result = main();
        assert!(result.is_ok(), "hybrid_search failed: {:?}", result.err());
    }
}
