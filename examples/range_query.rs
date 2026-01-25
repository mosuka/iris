//! RangeQuery example - demonstrates range search for numeric and date values.

use std::sync::Arc;

use tempfile::TempDir;

use iris::analysis::analyzer::analyzer::Analyzer;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::data::{DataValue, Document};
use iris::error::Result;
use iris::lexical::store::LexicalStore;
use iris::lexical::store::config::LexicalIndexConfig;
use iris::lexical::index::config::InvertedIndexConfig;
use iris::lexical::index::inverted::query::Query;
use iris::lexical::index::inverted::query::range::NumericRangeQuery;
use iris::lexical::search::searcher::LexicalSearchRequest;
use iris::storage::StorageConfig;
use iris::storage::StorageFactory;
use iris::storage::file::FileStorageConfig;

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
    let lexical_engine = LexicalStore::new(storage, lexical_index_config)?;

    let documents = vec![
        Document::new()
            .with_field(
                "title",
                DataValue::Text("Introduction to Algorithms".into()),
            )
            .with_field(
                "description",
                DataValue::Text("Comprehensive guide to algorithms and data structures".into()),
            )
            .with_field("price", DataValue::Float64(89.99))
            .with_field("rating", DataValue::Float64(4.8))
            .with_field("year", DataValue::Int64(2009))
            .with_field("pages", DataValue::Int64(1312))
            .with_field("id", DataValue::Text("book001".into())),
        Document::new()
            .with_field("title", DataValue::Text("Clean Code".into()))
            .with_field(
                "description",
                DataValue::Text("A handbook of agile software craftsmanship".into()),
            )
            .with_field("price", DataValue::Float64(45.50))
            .with_field("rating", DataValue::Float64(4.6))
            .with_field("year", DataValue::Int64(2008))
            .with_field("pages", DataValue::Int64(464))
            .with_field("id", DataValue::Text("book002".into())),
        Document::new()
            .with_field("title", DataValue::Text("Design Patterns".into()))
            .with_field(
                "description",
                DataValue::Text("Elements of reusable object-oriented software".into()),
            )
            .with_field("price", DataValue::Float64(62.95))
            .with_field("rating", DataValue::Float64(4.5))
            .with_field("year", DataValue::Int64(1994))
            .with_field("pages", DataValue::Int64(395))
            .with_field("id", DataValue::Text("book003".into())),
        Document::new()
            .with_field("title", DataValue::Text("The Pragmatic Programmer".into()))
            .with_field(
                "description",
                DataValue::Text("Your journey to mastery".into()),
            )
            .with_field("price", DataValue::Float64(52.99))
            .with_field("rating", DataValue::Float64(4.7))
            .with_field("year", DataValue::Int64(2019))
            .with_field("pages", DataValue::Int64(352))
            .with_field("id", DataValue::Text("book004".into())),
        Document::new()
            .with_field("title", DataValue::Text("Refactoring".into()))
            .with_field(
                "description",
                DataValue::Text("Improving the design of existing code".into()),
            )
            .with_field("price", DataValue::Float64(58.75))
            .with_field("rating", DataValue::Float64(4.4))
            .with_field("year", DataValue::Int64(2018))
            .with_field("pages", DataValue::Int64(448))
            .with_field("id", DataValue::Text("book005".into())),
        Document::new()
            .with_field("title", DataValue::Text("Code Complete".into()))
            .with_field(
                "description",
                DataValue::Text("A practical handbook of software construction".into()),
            )
            .with_field("price", DataValue::Float64(73.99))
            .with_field("rating", DataValue::Float64(4.9))
            .with_field("year", DataValue::Int64(2004))
            .with_field("pages", DataValue::Int64(914))
            .with_field("category", DataValue::Text("software".into()))
            .with_field("id", DataValue::Text("book006".into())),
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
