//! RangeQuery example - demonstrates range search for numeric and date values.

use std::sync::Arc;

use tempfile::TempDir;

use iris::Result;
use iris::analysis::analyzer::analyzer::Analyzer;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::InvertedIndexConfig;
use iris::lexical::LexicalIndexConfig;
use iris::lexical::LexicalSearchRequest;
use iris::lexical::LexicalStore;
use iris::lexical::NumericRangeQuery;
use iris::lexical::Query;
use iris::parking_lot::RwLock;
use iris::storage::StorageConfig;
use iris::storage::StorageFactory;
use iris::storage::file::FileStorageConfig;
use iris::storage::prefixed::PrefixedStorage;
use iris::store::document::UnifiedDocumentStore;
use iris::{DataValue, Document};

fn main() -> Result<()> {
    println!("=== RangeQuery Example - Numeric and Date Range Search ===\n");

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
    let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    let doc_store = Arc::new(RwLock::new(
        UnifiedDocumentStore::open(doc_storage).unwrap(),
    ));
    let lexical_engine = LexicalStore::new(storage, lexical_index_config, doc_store)?;

    let documents = vec![
        Document::new()
            .add_field(
                "title",
                DataValue::Text("Introduction to Algorithms".into()),
            )
            .add_field(
                "description",
                DataValue::Text("Comprehensive guide to algorithms and data structures".into()),
            )
            .add_field("price", DataValue::Float64(89.99))
            .add_field("rating", DataValue::Float64(4.8))
            .add_field("year", DataValue::Int64(2009))
            .add_field("pages", DataValue::Int64(1312))
            .add_field("id", DataValue::Text("book001".into())),
        Document::new()
            .add_field("title", DataValue::Text("Clean Code".into()))
            .add_field(
                "description",
                DataValue::Text("A handbook of agile software craftsmanship".into()),
            )
            .add_field("price", DataValue::Float64(45.50))
            .add_field("rating", DataValue::Float64(4.6))
            .add_field("year", DataValue::Int64(2008))
            .add_field("pages", DataValue::Int64(464))
            .add_field("id", DataValue::Text("book002".into())),
        Document::new()
            .add_field("title", DataValue::Text("Design Patterns".into()))
            .add_field(
                "description",
                DataValue::Text("Elements of reusable object-oriented software".into()),
            )
            .add_field("price", DataValue::Float64(62.95))
            .add_field("rating", DataValue::Float64(4.5))
            .add_field("year", DataValue::Int64(1994))
            .add_field("pages", DataValue::Int64(395))
            .add_field("id", DataValue::Text("book003".into())),
        Document::new()
            .add_field("title", DataValue::Text("The Pragmatic Programmer".into()))
            .add_field(
                "description",
                DataValue::Text("Your journey to mastery".into()),
            )
            .add_field("price", DataValue::Float64(52.99))
            .add_field("rating", DataValue::Float64(4.7))
            .add_field("year", DataValue::Int64(2019))
            .add_field("pages", DataValue::Int64(352))
            .add_field("id", DataValue::Text("book004".into())),
        Document::new()
            .add_field("title", DataValue::Text("Refactoring".into()))
            .add_field(
                "description",
                DataValue::Text("Improving the design of existing code".into()),
            )
            .add_field("price", DataValue::Float64(58.75))
            .add_field("rating", DataValue::Float64(4.4))
            .add_field("year", DataValue::Int64(2018))
            .add_field("pages", DataValue::Int64(448))
            .add_field("id", DataValue::Text("book005".into())),
        Document::new()
            .add_field("title", DataValue::Text("Code Complete".into()))
            .add_field(
                "description",
                DataValue::Text("A practical handbook of software construction".into()),
            )
            .add_field("price", DataValue::Float64(73.99))
            .add_field("rating", DataValue::Float64(4.9))
            .add_field("year", DataValue::Int64(2004))
            .add_field("pages", DataValue::Int64(914))
            .add_field("category", DataValue::Text("software".into()))
            .add_field("id", DataValue::Text("book006".into())),
    ];

    println!("Adding {} documents to the index...", documents.len());
    for doc in documents {
        lexical_engine.add_document(doc)?;
    }
    lexical_engine.commit()?;

    println!("\n=== RangeQuery Examples ===\n");

    // Example 1: Price range query
    println!("1. Books with price between $50.00 and $70.00:");
    let query = NumericRangeQuery::f64_range("price", Some(50.0), Some(70.0));
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
            if let Some(field) = doc.get_field("price")
                && let DataValue::Float64(price) = field
            {
                println!("      Price: ${price:.2}");
            }
        }
    }

    // Example 2: Rating range query (high-rated books)
    println!("\n2. Books with rating 4.5 or higher:");
    let query = NumericRangeQuery::f64_range("rating", Some(4.5), None);
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
            if let Some(field) = doc.get_field("rating")
                && let DataValue::Float64(rating) = field
            {
                println!("      Rating: {rating:.1}");
            }
        }
    }

    // Example 3: Year range query (recent books)
    println!("\n3. Books published after 2010:");
    let query = NumericRangeQuery::i64_range("year", Some(2010), None);
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
            if let Some(field) = doc.get_field("year")
                && let DataValue::Int64(year) = field
            {
                println!("      Year: {year}");
            }
        }
    }

    // Example 4: Page count range query (shorter books)
    println!("\n4. Books with 400 pages or fewer:");
    let query = NumericRangeQuery::i64_range("pages", None, Some(400));
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
            if let Some(field) = doc.get_field("pages")
                && let DataValue::Int64(pages) = field
            {
                println!("      Pages: {pages}");
            }
        }
    }

    // Example 5: Exact year range (books from 2008-2009)
    println!("\n5. Books published between 2008 and 2009:");
    let query = NumericRangeQuery::i64_range("year", Some(2008), Some(2009));
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
            if let Some(field) = doc.get_field("year")
                && let DataValue::Int64(year) = field
            {
                println!("      Year: {year}");
            }
        }
    }

    // Example 6: Budget-friendly books (price under $50)
    println!("\n6. Budget-friendly books (price under $50.00):");
    let query = NumericRangeQuery::f64_range_exclusive_upper("price", None, Some(50.0));
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
            if let Some(field) = doc.get_field("price")
                && let DataValue::Float64(price) = field
            {
                println!("      Price: ${price:.2}");
            }
        }
    }

    // Example 7: Large books (more than 500 pages)
    println!("\n7. Large books (more than 500 pages):");
    let query = NumericRangeQuery::i64_range("pages", Some(500), None);
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
            if let Some(field) = doc.get_field("pages")
                && let DataValue::Int64(pages) = field
            {
                println!("      Pages: {pages}");
            }
        }
    }

    // Example 8: Count books in price range
    println!("\n8. Counting books with price between $40.00 and $80.00:");
    let query = NumericRangeQuery::f64_range("price", Some(40.0), Some(80.0));
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("   Count: {count} books");

    // Example 9: Empty range (no results expected)
    println!("\n9. Books with impossible price range ($200-$300):");
    let query = NumericRangeQuery::f64_range("price", Some(200.0), Some(300.0));
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);

    lexical_engine.close()?;
    println!("\nRangeQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_query_example() {
        let result = main();
        assert!(result.is_ok());
    }
}
