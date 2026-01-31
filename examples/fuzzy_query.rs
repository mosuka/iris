//! FuzzyQuery example - demonstrates approximate string matching with edit distance.

use std::sync::Arc;

use tempfile::TempDir;

use iris::Result;
use iris::analysis::analyzer::analyzer::Analyzer;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::FuzzyQuery;
use iris::lexical::InvertedIndexConfig;
use iris::lexical::LexicalIndexConfig;
use iris::lexical::LexicalSearchRequest;
use iris::lexical::LexicalStore;
use iris::lexical::Query;
use iris::parking_lot::RwLock;
use iris::storage::file::FileStorageConfig;
use iris::storage::prefixed::PrefixedStorage;
use iris::storage::{StorageConfig, StorageFactory};
use iris::store::document::UnifiedDocumentStore;
use iris::{DataValue, Document};

fn main() -> Result<()> {
    println!("=== FuzzyQuery Example - Approximate String Matching ===\n");

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

    // Add documents with various spellings and terms for fuzzy matching
    let documents = vec![
        Document::new()
            .add_field("title", DataValue::Text("JavaScript Programming Guide".into()))
            .add_field("body", DataValue::Text("Comprehensive guide to JavaScript development and programming techniques".into()))
            .add_field("author", DataValue::Text("John Smith".into()))
            .add_field("tags", DataValue::Text("javascript programming tutorial".into()))
            .add_field("id", DataValue::Text("doc001".into())),
        Document::new()
            .add_field("title", DataValue::Text("Python Programming Fundamentals".into()))
            .add_field("body", DataValue::Text("Learn Python programming language from scratch with practical examples".into()))
            .add_field("author", DataValue::Text("Alice Johnson".into()))
            .add_field("tags", DataValue::Text("python programming beginner".into()))
            .add_field("id", DataValue::Text("doc002".into())),
        Document::new()
            .add_field("title", DataValue::Text("Machine Learning Algorithms".into()))
            .add_field("body", DataValue::Text("Understanding algorithms used in machine learning and artificial intelligence".into()))
            .add_field("author", DataValue::Text("Bob Wilson".into()))
            .add_field("tags", DataValue::Text("machine-learning algorithms ai".into()))
            .add_field("id", DataValue::Text("doc003".into())),
        Document::new()
            .add_field("title", DataValue::Text("Database Management Systems".into()))
            .add_field("body", DataValue::Text("Introduction to database systems, SQL, and data management principles".into()))
            .add_field("author", DataValue::Text("Carol Davis".into()))
            .add_field("tags", DataValue::Text("database sql management".into()))
            .add_field("id", DataValue::Text("doc004".into())),
        Document::new()
            .add_field("title", DataValue::Text("Web Development with React".into()))
            .add_field("body", DataValue::Text("Building modern web applications using React framework and components".into()))
            .add_field("author", DataValue::Text("David Brown".into()))
            .add_field("tags", DataValue::Text("react web-development frontend".into()))
            .add_field("id", DataValue::Text("doc005".into())),
        Document::new()
            .add_field("title", DataValue::Text("Artificial Intelligence Overview".into()))
            .add_field("body", DataValue::Text("Introduction to artificial intelligence concepts, applications, and algorithms".into()))
            .add_field("author", DataValue::Text("Eva Martinez".into()))
            .add_field("tags", DataValue::Text("artificial-intelligence overview concepts".into()))
            .add_field("id", DataValue::Text("doc006".into())),
        Document::new()
            .add_field("title", DataValue::Text("Software Engineering Principles".into()))
            .add_field("body", DataValue::Text("Best practices in software engineering, design patterns, and development".into()))
            .add_field("author", DataValue::Text("Frank Miller".into()))
            .add_field("tags", DataValue::Text("software engineering principles".into()))
            .add_field("id", DataValue::Text("doc007".into())),
    ];

    println!("Adding {} documents to the index...", documents.len());
    for doc in documents {
        lexical_engine.add_document(doc)?;
    }

    // Commit changes to engine
    lexical_engine.commit()?;

    println!("\n=== FuzzyQuery Examples ===\n");

    // Example 1: Simple fuzzy search with small edit distance
    println!("1. Fuzzy search for 'javascritp' (typo for 'javascript') with edit distance 1:");
    let query = FuzzyQuery::new("body", "javascritp").max_edits(1);
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

    // Example 2: Fuzzy search with higher edit distance
    println!("\n2. Fuzzy search for 'programing' (missing 'm') with edit distance 2:");
    let query = FuzzyQuery::new("body", "programing").max_edits(2);
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

    // Example 3: Fuzzy search in title field
    println!("\n3. Fuzzy search for 'machne' (missing 'i') in title with edit distance 1:");
    let query = FuzzyQuery::new("title", "machne").max_edits(1);
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

    // Example 4: Fuzzy search for author names
    println!("\n4. Fuzzy search for 'Jon' (should match 'John') in author with edit distance 1:");
    let query = FuzzyQuery::new("author", "Jon").max_edits(1);
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

    // Example 5: Fuzzy search with various misspellings
    println!("\n5. Fuzzy search for 'algoritm' (missing 'h') with edit distance 2:");
    let query = FuzzyQuery::new("body", "algoritm").max_edits(2);
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

    // Example 6: Fuzzy search in tags
    println!("\n6. Fuzzy search for 'artifical' (missing 'i') in tags with edit distance 1:");
    let query = FuzzyQuery::new("tags", "artifical").max_edits(1);
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

    // Example 7: Fuzzy search with exact match (edit distance 0)
    println!("\n7. Fuzzy search for exact 'python' with edit distance 0:");
    let query = FuzzyQuery::new("body", "python").max_edits(0);
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

    // Example 8: Fuzzy search with high edit distance (more permissive)
    println!("\n8. Fuzzy search for 'databse' (missing 'a') with edit distance 3:");
    let query = FuzzyQuery::new("body", "databse").max_edits(3);
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

    // Example 9: No fuzzy matches found
    println!("\n9. Fuzzy search for 'xyz123' (no similar terms) with edit distance 2:");
    let query = FuzzyQuery::new("body", "xyz123").max_edits(2);
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);

    // Example 10: Count fuzzy matches
    println!("\n10. Counting documents with fuzzy match for 'developement' (extra 'e'):");
    let query = FuzzyQuery::new("body", "developement").max_edits(2);
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("    Count: {count} documents");

    lexical_engine.close()?;
    println!("\nFuzzyQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_query_example() {
        let result = main();
        assert!(result.is_ok());
    }
}
