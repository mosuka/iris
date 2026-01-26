//! TermQuery example - demonstrates single term exact matching search.

use std::sync::Arc;

use tempfile::TempDir;

use iris::analysis::analyzer::analyzer::Analyzer;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::data::{DataValue, Document};
use iris::error::Result;
use iris::lexical::index::config::InvertedIndexConfig;
use iris::lexical::index::inverted::query::Query;
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::lexical::search::searcher::LexicalSearchRequest;
use iris::lexical::store::LexicalStore;
use iris::lexical::store::config::LexicalIndexConfig;
use iris::storage::StorageConfig;
use iris::storage::StorageFactory;
use iris::storage::file::FileStorageConfig;

fn main() -> Result<()> {
    println!("=== TermQuery Example - Single Term Exact Matching ===\n");

    // Create a storage backend
    let temp_dir = TempDir::new().unwrap();
    let storage =
        StorageFactory::create(StorageConfig::File(FileStorageConfig::new(temp_dir.path())))?;

    // Create an analyzer
    let standard_analyzer: Arc<dyn Analyzer> = Arc::new(StandardAnalyzer::new()?);
    let keyword_analyzer: Arc<dyn Analyzer> = Arc::new(KeywordAnalyzer::new());
    let mut per_field_analyzer = PerFieldAnalyzer::new(Arc::clone(&standard_analyzer));
    per_field_analyzer.add_analyzer("id", Arc::clone(&keyword_analyzer));

    // Create a lexical engine
    let lexical_index_config = LexicalIndexConfig::Inverted(InvertedIndexConfig {
        analyzer: Arc::new(per_field_analyzer.clone()),
        ..InvertedIndexConfig::default()
    });
    let lexical_engine = LexicalStore::new(storage, lexical_index_config)?;

    // Add documents with various terms
    let documents = vec![
        Document::new()
            .add_field("title", DataValue::Text("Rust Programming Language".into()))
            .add_field(
                "body",
                DataValue::Text("Rust is a systems programming language focused on safety, speed, and concurrency".into()),
            )
            .add_field("author", DataValue::Text("Steve Klabnik".into()))
            .add_field("category", DataValue::Text("programming".into()))
            .add_field("id", DataValue::Text("doc1".into())),
        Document::new()
            .add_field("title", DataValue::Text("Python for Beginners".into()))
            .add_field(
                "body",
                DataValue::Text("Python is a versatile and easy-to-learn programming language".into()),
            )
            .add_field("author", DataValue::Text("John Smith".into()))
            .add_field("category", DataValue::Text("programming".into()))
            .add_field("id", DataValue::Text("doc2".into())),
        Document::new()
            .add_field("title", DataValue::Text("JavaScript Essentials".into()))
            .add_field(
                "body",
                DataValue::Text("JavaScript is the language of the web, used for frontend and backend development".into()),
            )
            .add_field("author", DataValue::Text("Jane Doe".into()))
            .add_field("category", DataValue::Text("web-development".into()))
            .add_field("id", DataValue::Text("doc3".into())),
        Document::new()
            .add_field("title", DataValue::Text("Machine Learning Fundamentals".into()))
            .add_field(
                "body",
                DataValue::Text("Machine learning is a subset of artificial intelligence focused on algorithms".into()),
            )
            .add_field("author", DataValue::Text("Alice Johnson".into()))
            .add_field("category", DataValue::Text("data-science".into()))
            .add_field("id", DataValue::Text("doc4".into())),
        Document::new()
            .add_field("title", DataValue::Text("Data Structures in C++".into()))
            .add_field(
                "body",
                DataValue::Text("Understanding data structures is crucial for efficient programming".into()),
            )
            .add_field("author", DataValue::Text("Bob Wilson".into()))
            .add_field("category", DataValue::Text("programming".into()))
            .add_field("id", DataValue::Text("doc5".into())),
    ];

    println!("Adding {} documents to the index...", documents.len());
    for doc in documents {
        lexical_engine.add_document(doc)?;
    }

    lexical_engine.commit()?;

    println!("\n=== TermQuery Examples ===\n");

    // Example 1: Search for exact term in title field
    println!("1. Searching for 'Rust' in title field:");
    let query = TermQuery::new("title", "Rust");
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("title")
                && let DataValue::Text(title) = field
            {
                println!("      Title: {title}");
            }
            if let Some(field) = doc.get_field("author")
                && let DataValue::Text(author) = field
            {
                println!("      Author: {author}");
            }
        }
    }

    // Example 2: Search for exact term in body field
    println!("\n2. Searching for 'language' in body field:");
    let query = TermQuery::new("body", "language");
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("title")
                && let DataValue::Text(title) = field
            {
                println!("      Title: {title}");
            }
            if let Some(field) = doc.get_field("author")
                && let DataValue::Text(author) = field
            {
                println!("      Author: {author}");
            }
        }
    }

    // Example 3: Search for exact term in category field
    println!("\n3. Searching for 'programming' in category field:");
    let query = TermQuery::new("category", "programming");
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("title")
                && let DataValue::Text(title) = field
            {
                println!("      Title: {title}");
            }
            if let Some(field) = doc.get_field("author")
                && let DataValue::Text(author) = field
            {
                println!("      Author: {author}");
            }
        }
    }

    // Example 4: Search for non-existent term
    println!("\n4. Searching for non-existent term 'golang':");
    let query = TermQuery::new("title", "golang");
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);

    // Example 5: Case sensitivity demonstration
    println!("\n5. Case sensitivity - searching for 'rust' (lowercase):");
    let query = TermQuery::new("title", "rust");
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    println!("   Note: TermQuery is case-sensitive by default");

    // Example 6: Author exact match
    println!("\n6. Searching for exact author 'John Smith':");
    let query = TermQuery::new("author", "John Smith");
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("title")
                && let DataValue::Text(title) = field
            {
                println!("      Title: {title}");
            }
            if let Some(field) = doc.get_field("author")
                && let DataValue::Text(author) = field
            {
                println!("      Author: {author}");
            }
        }
    }

    // Example 7: Count matching documents
    println!("\n7. Counting documents containing 'programming':");
    let query = TermQuery::new("body", "programming");
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("   Count: {count} documents");

    lexical_engine.close()?;
    println!("\nTermQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_query_example() {
        let result = main();
        assert!(result.is_ok());
    }
}
