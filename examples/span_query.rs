//! Span Query Example
//!
//! This example demonstrates how to use Span Queries in Iris for positional and proximity search.
//! Span queries allow you to find terms that are near each other, or within specific distances.

use std::sync::Arc;

use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::data::{DataValue, Document};
use iris::error::Result;
use iris::lexical::store::LexicalStore;
use iris::lexical::store::config::LexicalIndexConfig;
use iris::lexical::index::config::InvertedIndexConfig;
use iris::lexical::index::inverted::query::span::{SpanQueryBuilder, SpanQueryWrapper};
use iris::lexical::index::inverted::query::{LexicalSearchResults, Query};
use iris::lexical::search::searcher::LexicalSearchRequest;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use tempfile::TempDir;

fn main() -> Result<()> {
    println!("=== Span Query Example ===\n");

    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Setup Analyzer and Index
    let analyzer = Arc::new(StandardAnalyzer::new()?);
    let index_config = LexicalIndexConfig::Inverted(InvertedIndexConfig {
        analyzer,
        ..InvertedIndexConfig::default()
    });

    // 3. Create Engine
    let engine = LexicalStore::new(storage, index_config)?;

    // 4. Add Documents
    let documents = vec![
        Document::new()
            .with_field("title", DataValue::Text("The quick brown fox".into()))
            .with_field("body", DataValue::Text("Jumped over the lazy dog".into())),
        Document::new()
            .with_field("title", DataValue::Text(" The quick red fox".into()))
            .with_field("body", DataValue::Text("Jumped over the lazy cat".into())),
        Document::new()
            .with_field("title", DataValue::Text("The lazy brown dog".into()))
            .with_field("body", DataValue::Text("Jumped over the quick fox".into())),
    ];

    println!("Indexing {} documents...", documents.len());
    for doc in documents {
        engine.add_document(doc)?;
    }
    engine.commit()?;

    // 5. Search Examples
    let builder = SpanQueryBuilder::new("title");

    // Example 1: SpanTermQuery (Basic positional match)
    // Find documents containing "quick" in the "title" field.
    println!("\n--- Search 1: SpanTermQuery 'quick' ---");
    let span_term = builder.term("quick");
    let query_wrapper = SpanQueryWrapper::new(Box::new(span_term));
    let request =
        LexicalSearchRequest::new(Box::new(query_wrapper) as Box<dyn Query>).load_documents(true);
    let results = engine.search(request)?;
    print_results(&results);

    // Example 2: SpanNearQuery (Proximity search)
    // Find "quick" and "fox" within 1 word of each other (slop=1).
    // "quick brown fox" -> "quick"(pos 1), "fox"(pos 3). distance=1. Matches slop=1.
    println!("\n--- Search 2: SpanNearQuery 'quick' near 'fox' (slop=1) ---");
    let span_near = builder.near(
        vec![
            Box::new(builder.term("quick")),
            Box::new(builder.term("fox")),
        ],
        1,    // slop
        true, // in_order
    );
    let query_wrapper = SpanQueryWrapper::new(Box::new(span_near));
    let request =
        LexicalSearchRequest::new(Box::new(query_wrapper) as Box<dyn Query>).load_documents(true);
    let results = engine.search(request)?;
    print_results(&results);

    // Example 3: SpanNearQuery (Ordered vs Unordered)
    // Find "fox" near "quick" (reverse order).
    // With in_order=true, this should NOT match "quick brown fox".
    println!("\n--- Search 3: SpanNearQuery 'fox' near 'quick' (in_order=true) ---");
    let span_near_ordered = builder.near(
        vec![
            Box::new(builder.term("fox")),
            Box::new(builder.term("quick")),
        ],
        5,    // loose slop
        true, // in_order
    );
    let query_wrapper = SpanQueryWrapper::new(Box::new(span_near_ordered));
    let request =
        LexicalSearchRequest::new(Box::new(query_wrapper) as Box<dyn Query>).load_documents(true);
    let results = engine.search(request)?;
    print_results(&results); // Should be empty for the first doc

    // With in_order=false, this SHOULD match "quick brown fox" (as long as they are near).
    println!("\n--- Search 4: SpanNearQuery 'fox' near 'quick' (in_order=false) ---");
    let span_near_unordered = builder.near(
        vec![
            Box::new(builder.term("fox")),
            Box::new(builder.term("quick")),
        ],
        5,     // loose slop
        false, // in_order
    );
    let query_wrapper = SpanQueryWrapper::new(Box::new(span_near_unordered));
    let request =
        LexicalSearchRequest::new(Box::new(query_wrapper) as Box<dyn Query>).load_documents(true);
    let results = engine.search(request)?;
    print_results(&results);

    // Example 5: SpanContainingQuery (Big span containing little span)
    // Note: In our current small example, "brown fox" (pos 2-4) contains "fox" (pos 3-4)?
    // No, "containing" usually means one span strictly encloses or covers the other.
    // "quick brown fox" (0-3) contains "brown" (1-2).
    println!("\n--- Search 5: SpanContainingQuery 'quick .. fox' containing 'brown' ---");
    let big_span = builder.near(
        vec![
            Box::new(builder.term("quick")),
            Box::new(builder.term("fox")),
        ],
        1,
        true,
    );
    let little_span = builder.term("brown");

    let containing_query = builder.containing(Box::new(big_span), Box::new(little_span));
    let wrapper = SpanQueryWrapper::new(Box::new(containing_query));
    let req = LexicalSearchRequest::new(Box::new(wrapper) as Box<dyn Query>);
    let res = engine.search(req)?;
    print_results(&res);

    // Example 6: SpanWithinQuery (Little span within big span)
    println!("\n--- Search 6: SpanWithinQuery 'brown' within 'quick .. fox' ---");
    let big_span = builder.near(
        vec![
            Box::new(builder.term("quick")),
            Box::new(builder.term("fox")),
        ],
        1,
        true,
    );
    let little_span = builder.term("brown");

    // within is just reverse of containing usually, but 'distance' parameter in builder.within is for SpanWithin checks if we had a distance constraint?
    // Looking at SpanWithinQuery implementation, it seems to just check containment.
    // Actually `builder.within` has a `distance` param?
    // SpanWithinQuery::new(field, include, exclude, distance) -> wait, is it include within exclude?
    // Let's check SpanWithinQuery definition.
    // include: SpanQuery, exclude: SpanQuery, distance: u32.
    // Usually "within" means "find A within B".

    let within_query = builder.within(
        Box::new(little_span),
        Box::new(big_span),
        0, // distance param might be unused or for different type of 'within'
    );
    let wrapper = SpanQueryWrapper::new(Box::new(within_query));
    let req = LexicalSearchRequest::new(Box::new(wrapper) as Box<dyn Query>);
    let res = engine.search(req)?;
    print_results(&res);

    Ok(())
}

fn print_results(results: &LexicalSearchResults) {
    println!("Found {} hits:", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!("{}. Doc ID: {}, Score: {:.4}", i + 1, hit.doc_id, hit.score);
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("title")
                && let DataValue::Text(title) = field
            {
                println!("   Title: {}", title);
            }
        }
    }
}
