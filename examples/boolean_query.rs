//! BooleanQuery example - demonstrates complex boolean logic with AND, OR, NOT operations.

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
use iris::lexical::index::inverted::query::boolean::BooleanQuery;
use iris::lexical::index::inverted::query::phrase::PhraseQuery;
use iris::lexical::index::inverted::query::range::NumericRangeQuery;
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::lexical::search::searcher::LexicalSearchRequest;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};

fn main() -> Result<()> {
    println!("=== BooleanQuery Example - Complex Boolean Logic ===\n");

    // Create a storage backend
    let temp_dir = TempDir::new().unwrap();
    let storage =
        StorageFactory::create(StorageConfig::File(FileStorageConfig::new(temp_dir.path())))?;

    // Create an analyzer
    let standard_analyzer: Arc<dyn Analyzer> = Arc::new(StandardAnalyzer::new()?);
    let keyword_analyzer: Arc<dyn Analyzer> = Arc::new(KeywordAnalyzer::new());
    let mut per_field_analyzer = PerFieldAnalyzer::new(Arc::clone(&standard_analyzer));
    per_field_analyzer.add_analyzer("category", Arc::clone(&keyword_analyzer));

    // Create a lexical engine
    let lexical_index_config = LexicalIndexConfig::Inverted(InvertedIndexConfig {
        analyzer: Arc::new(per_field_analyzer.clone()),
        ..InvertedIndexConfig::default()
    });
    let lexical_engine = LexicalStore::new(storage, lexical_index_config)?;

    let documents = vec![
        Document::new()
            .with_field("title", DataValue::Text("Advanced Python Programming".into()))
            .with_field("body", DataValue::Text("Learn advanced Python techniques including decorators, metaclasses, and async programming".into()))
            .with_field("author", DataValue::Text("Alice Johnson".into()))
            .with_field("category", DataValue::Text("programming".into()))
            .with_field("price", DataValue::Float64(59.99))
            .with_field("rating", DataValue::Float64(4.7))
            .with_field("tags", DataValue::Text("python advanced programming".into()))
            .with_field("id", DataValue::Text("book001".into())),
        Document::new()
            .with_field("title", DataValue::Text("JavaScript for Web Development".into()))
            .with_field("body", DataValue::Text("Modern JavaScript techniques for frontend and backend web development".into()))
            .with_field("author", DataValue::Text("Bob Smith".into()))
            .with_field("category", DataValue::Text("web-development".into()))
            .with_field("price", DataValue::Float64(45.50))
            .with_field("rating", DataValue::Float64(4.3))
            .with_field("tags", DataValue::Text("javascript web frontend backend".into()))
            .with_field("id", DataValue::Text("book002".into())),
        Document::new()
            .with_field("title", DataValue::Text("Machine Learning with Python".into()))
            .with_field("body", DataValue::Text("Practical machine learning algorithms implemented in Python".into()))
            .with_field("author", DataValue::Text("Carol Davis".into()))
            .with_field("category", DataValue::Text("data-science".into()))
            .with_field("price", DataValue::Float64(72.99))
            .with_field("rating", DataValue::Float64(4.8))
            .with_field("tags", DataValue::Text("python machine-learning data-science".into()))
            .with_field("id", DataValue::Text("book003".into())),
        Document::new()
            .with_field("title", DataValue::Text("Web Design Fundamentals".into()))
            .with_field("body", DataValue::Text("Learn the basics of web design including HTML, CSS, and responsive design".into()))
            .with_field("author", DataValue::Text("David Brown".into()))
            .with_field("category", DataValue::Text("web-development".into()))
            .with_field("price", DataValue::Float64(39.99))
            .with_field("rating", DataValue::Float64(4.1))
            .with_field("tags", DataValue::Text("web design html css".into()))
            .with_field("id", DataValue::Text("book004".into())),
        Document::new()
            .with_field("title", DataValue::Text("Data Science with R".into()))
            .with_field("body", DataValue::Text("Statistical computing and data analysis using the R programming language".into()))
            .with_field("author", DataValue::Text("Eva Wilson".into()))
            .with_field("category", DataValue::Text("data-science".into()))
            .with_field("price", DataValue::Float64(65.00))
            .with_field("rating", DataValue::Float64(4.5))
            .with_field("tags", DataValue::Text("r data-science statistics".into()))
            .with_field("id", DataValue::Text("book005".into())),
        Document::new()
            .with_field("title", DataValue::Text("Advanced JavaScript Patterns".into()))
            .with_field("body", DataValue::Text("Design patterns and advanced programming techniques in JavaScript".into()))
            .with_field("author", DataValue::Text("Frank Miller".into()))
            .with_field("category", DataValue::Text("programming".into()))
            .with_field("price", DataValue::Float64(54.99))
            .with_field("rating", DataValue::Float64(4.6))
            .with_field("tags", DataValue::Text("javascript advanced patterns".into()))
            .with_field("id", DataValue::Text("book006".into())),
    ];

    println!("Adding {} documents to the index...", documents.len());

    // Add documents to the lexical engine
    for doc in documents {
        lexical_engine.add_document(doc)?;
    }

    // Commit changes to engine
    lexical_engine.commit()?;

    println!("\n=== BooleanQuery Examples ===\n");

    // Example 1: Simple AND query
    // Note: Using lowercase terms because StandardAnalyzer normalizes text
    println!("1. Books about Python AND programming:");
    let mut query = BooleanQuery::new();
    query.add_must(Box::new(TermQuery::new("body", "python")));
    query.add_must(Box::new(TermQuery::new("body", "programming")));
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
        if let Some(doc) = &hit.document
            && let Some(field) = doc.get_field("title")
            && let DataValue::Text(title) = field
        {
            println!("      Title: {title}");
        }
    }

    // Example 2: Simple OR query
    println!("\n2. Books about Python OR JavaScript:");
    let mut query = BooleanQuery::new();
    query.add_should(Box::new(TermQuery::new("body", "python")));
    query.add_should(Box::new(TermQuery::new("body", "javascript")));
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
        if let Some(doc) = &hit.document
            && let Some(field) = doc.get_field("title")
            && let DataValue::Text(title) = field
        {
            println!("      Title: {title}");
        }
    }

    // Example 3: NOT query (must not contain)
    println!("\n3. Programming books that are NOT about JavaScript:");
    let mut query = BooleanQuery::new();
    query.add_must(Box::new(TermQuery::new("category", "programming")));
    query.add_must_not(Box::new(TermQuery::new("body", "javascript")));
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
        if let Some(doc) = &hit.document
            && let Some(field) = doc.get_field("title")
            && let DataValue::Text(title) = field
        {
            println!("      Title: {title}");
        }
    }

    // Example 4: Complex boolean query with multiple conditions
    println!("\n4. Web development books with high rating (>= 4.2) and reasonable price (<= $50):");
    let mut query = BooleanQuery::new();
    query.add_must(Box::new(TermQuery::new("category", "web-development")));
    query.add_must(Box::new(NumericRangeQuery::f64_range(
        "rating",
        Some(4.2),
        None,
    )));
    query.add_must(Box::new(NumericRangeQuery::f64_range(
        "price",
        None,
        Some(50.0),
    )));
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
            if let Some(field) = doc.get_field("rating")
                && let DataValue::Float64(rating) = field
            {
                println!("      Rating: {rating:.1}");
            }
        }
    }

    // Example 5: Phrase query in boolean context
    println!("\n5. Data science books that contain 'machine learning' phrase:");
    let mut query = BooleanQuery::new();
    query.add_must(Box::new(TermQuery::new("category", "data-science")));
    query.add_must(Box::new(PhraseQuery::new(
        "body",
        vec!["machine".to_string(), "learning".to_string()],
    )));
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
        if let Some(doc) = &hit.document
            && let Some(field) = doc.get_field("title")
            && let DataValue::Text(title) = field
        {
            println!("      Title: {title}");
        }
    }

    // Example 6: Multiple OR conditions with AND
    println!("\n6. Advanced books about either Python OR JavaScript:");
    let mut language_query = BooleanQuery::new();
    language_query.add_should(Box::new(TermQuery::new("body", "python")));
    language_query.add_should(Box::new(TermQuery::new("body", "javascript")));

    let mut query = BooleanQuery::new();
    query.add_must(Box::new(TermQuery::new("tags", "advanced")));
    query.add_must(Box::new(language_query));

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
        if let Some(doc) = &hit.document
            && let Some(field) = doc.get_field("title")
            && let DataValue::Text(title) = field
        {
            println!("      Title: {title}");
        }
    }

    // Example 7: Exclude expensive books
    println!("\n7. Books under $60 that are NOT about web design:");
    let mut query = BooleanQuery::new();
    query.add_must(Box::new(NumericRangeQuery::f64_range(
        "price",
        None,
        Some(60.0),
    )));
    query.add_must_not(Box::new(TermQuery::new("body", "design")));
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

    // Example 8: Optional conditions (SHOULD clauses)
    println!("\n8. Programming books, preferably about Python or with high rating:");
    let mut query = BooleanQuery::new();
    query.add_must(Box::new(TermQuery::new("category", "programming")));
    query.add_should(Box::new(TermQuery::new("body", "python")));
    query.add_should(Box::new(NumericRangeQuery::f64_range(
        "rating",
        Some(4.5),
        None,
    )));
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

    // Example 9: Nested boolean queries - Complex logic
    println!(
        "\n9. Nested boolean queries - (Python OR JavaScript) AND (advanced OR high-rating) AND NOT expensive:"
    );

    // First nested query: (Python OR JavaScript)
    let mut language_query = BooleanQuery::new();
    language_query.add_should(Box::new(TermQuery::new("body", "python")));
    language_query.add_should(Box::new(TermQuery::new("body", "javascript")));

    // Second nested query: (advanced OR high-rating)
    let mut quality_query = BooleanQuery::new();
    quality_query.add_should(Box::new(TermQuery::new("tags", "advanced")));
    quality_query.add_should(Box::new(NumericRangeQuery::f64_range(
        "rating",
        Some(4.5),
        None,
    )));

    // Main query combining all conditions
    let mut main_query = BooleanQuery::new();
    main_query.add_must(Box::new(language_query)); // Must match (Python OR JavaScript)
    main_query.add_must(Box::new(quality_query)); // Must match (advanced OR high-rating)
    main_query.add_must_not(Box::new(NumericRangeQuery::f64_range(
        // Must NOT be expensive
        "price",
        Some(70.0),
        None,
    )));

    let request =
        LexicalSearchRequest::new(Box::new(main_query) as Box<dyn Query>).load_documents(true);
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
            if let Some(field) = doc.get_field("rating")
                && let DataValue::Float64(rating) = field
            {
                println!("      Rating: {rating:.1}");
            }
        }
    }

    // Example 10: Triple-nested boolean query - More complex logic
    println!(
        "\n10. Triple-nested query - ((Python AND advanced) OR (JavaScript AND web)) AND price < $60:"
    );

    // First nested sub-query: (Python AND advanced)
    let mut python_advanced = BooleanQuery::new();
    python_advanced.add_must(Box::new(TermQuery::new("body", "python")));
    python_advanced.add_must(Box::new(TermQuery::new("tags", "advanced")));

    // Second nested sub-query: (JavaScript AND web)
    let mut javascript_web = BooleanQuery::new();
    javascript_web.add_must(Box::new(TermQuery::new("body", "javascript")));
    javascript_web.add_must(Box::new(TermQuery::new("tags", "web")));

    // Combine the two sub-queries with OR
    let mut combined_query = BooleanQuery::new();
    combined_query.add_should(Box::new(python_advanced));
    combined_query.add_should(Box::new(javascript_web));

    // Final query with price constraint
    let mut final_query = BooleanQuery::new();
    final_query.add_must(Box::new(combined_query));
    final_query.add_must(Box::new(NumericRangeQuery::f64_range(
        "price",
        None,
        Some(60.0),
    )));

    let request =
        LexicalSearchRequest::new(Box::new(final_query) as Box<dyn Query>).load_documents(true);
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

    // Example 11: Count matching documents
    println!("\n11. Counting books about either data science OR web development:");
    let mut query = BooleanQuery::new();
    query.add_should(Box::new(TermQuery::new("category", "data-science")));
    query.add_should(Box::new(TermQuery::new("category", "web-development")));
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("   Count: {count} books");

    lexical_engine.close()?;
    println!("\nBooleanQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_query_example() {
        let result = main();
        assert!(result.is_ok());
    }
}
