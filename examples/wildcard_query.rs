//! WildcardQuery example - demonstrates pattern matching with * and ? wildcards.

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
use iris::lexical::Query;
use iris::lexical::WildcardQuery;
use iris::storage::StorageConfig;
use iris::storage::StorageFactory;
use iris::storage::file::FileStorageConfig;
use iris::{DataValue, Document};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== WildcardQuery Example - Pattern Matching with Wildcards ===\n");

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

    // Add documents with various patterns for wildcard matching
    let documents = vec![
        Document::builder()
            .add_field(
                "title",
                DataValue::Text("JavaScript Tutorial for Beginners".into()),
            )
            .add_field(
                "filename",
                DataValue::Text("javascript_tutorial.pdf".into()),
            )
            .add_field(
                "description",
                DataValue::Text("Complete JavaScript programming guide".into()),
            )
            .add_field("category", DataValue::Text("programming".into()))
            .add_field("extension", DataValue::Text("pdf".into()))
            .add_field("id", DataValue::Text("file001".into()))
            .build(),
        Document::builder()
            .add_field(
                "title",
                DataValue::Text("Python Programming Reference".into()),
            )
            .add_field("filename", DataValue::Text("python_reference.html".into()))
            .add_field(
                "description",
                DataValue::Text("Comprehensive Python programming reference".into()),
            )
            .add_field("category", DataValue::Text("programming".into()))
            .add_field("extension", DataValue::Text("html".into()))
            .add_field("id", DataValue::Text("file002".into()))
            .build(),
        Document::builder()
            .add_field(
                "title",
                DataValue::Text("Machine Learning Algorithms".into()),
            )
            .add_field("filename", DataValue::Text("ml_algorithms.docx".into()))
            .add_field(
                "description",
                DataValue::Text("Understanding machine learning techniques".into()),
            )
            .add_field("category", DataValue::Text("data-science".into()))
            .add_field("extension", DataValue::Text("docx".into()))
            .add_field("id", DataValue::Text("file003".into()))
            .build(),
        Document::builder()
            .add_field(
                "title",
                DataValue::Text("Database Design Principles".into()),
            )
            .add_field("filename", DataValue::Text("database_design.pptx".into()))
            .add_field(
                "description",
                DataValue::Text("Principles of good database design".into()),
            )
            .add_field("category", DataValue::Text("database".into()))
            .add_field("extension", DataValue::Text("pptx".into()))
            .add_field("id", DataValue::Text("file004".into()))
            .build(),
        Document::builder()
            .add_field(
                "title",
                DataValue::Text("Web Development Best Practices".into()),
            )
            .add_field("filename", DataValue::Text("web_dev_practices.txt".into()))
            .add_field(
                "description",
                DataValue::Text("Best practices for web development".into()),
            )
            .add_field("category", DataValue::Text("web-development".into()))
            .add_field("extension", DataValue::Text("txt".into()))
            .add_field("id", DataValue::Text("file005".into()))
            .build(),
        Document::builder()
            .add_field("title", DataValue::Text("React Component Patterns".into()))
            .add_field("filename", DataValue::Text("react_patterns.jsx".into()))
            .add_field(
                "description",
                DataValue::Text("Common patterns in React component development".into()),
            )
            .add_field("category", DataValue::Text("frontend".into()))
            .add_field("extension", DataValue::Text("jsx".into()))
            .add_field("id", DataValue::Text("file006".into()))
            .build(),
        Document::builder()
            .add_field(
                "title",
                DataValue::Text("API Documentation Template".into()),
            )
            .add_field("filename", DataValue::Text("api_docs_template.md".into()))
            .add_field(
                "description",
                DataValue::Text("Template for creating API documentation".into()),
            )
            .add_field("category", DataValue::Text("documentation".into()))
            .add_field("extension", DataValue::Text("md".into()))
            .add_field("id", DataValue::Text("file007".into()))
            .build(),
        Document::builder()
            .add_field("title", DataValue::Text("Configuration Settings".into()))
            .add_field("filename", DataValue::Text("app_config.json".into()))
            .add_field(
                "description",
                DataValue::Text("Application configuration file".into()),
            )
            .add_field("category", DataValue::Text("configuration".into()))
            .add_field("extension", DataValue::Text("json".into()))
            .add_field("id", DataValue::Text("file008".into()))
            .build(),
    ];

    println!("Adding {} documents to the index...", documents.len());
    for (i, doc) in documents.into_iter().enumerate() {
        lexical_engine.upsert_document((i + 1) as u64, doc)?;
    }

    lexical_engine.commit()?;

    println!("\n=== WildcardQuery Examples ===\n");

    // Example 1: Wildcard at the end (prefix matching)
    println!("1. Files starting with 'java' using 'java*' pattern:");
    let query = WildcardQuery::new("filename", "java*")?;
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
            && let Some(field) = doc.get_field("filename")
            && let DataValue::Text(filename) = field
        {
            println!("      Filename: {filename}");
        }
    }

    // Example 2: Wildcard at the beginning (suffix matching)
    println!("\n2. Files ending with '.pdf' using '*.pdf' pattern:");
    let query = WildcardQuery::new("filename", "*.pdf")?;
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
            && let Some(field) = doc.get_field("filename")
            && let DataValue::Text(filename) = field
        {
            println!("      Filename: {filename}");
        }
    }

    // Example 3: Wildcard in the middle
    println!("\n3. Files with 'web' followed by anything ending in '.txt' using 'web*.txt':");
    let query = WildcardQuery::new("filename", "web*.txt")?;
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
            && let Some(field) = doc.get_field("filename")
            && let DataValue::Text(filename) = field
        {
            println!("      Filename: {filename}");
        }
    }

    // Example 4: Single character wildcard (?)
    println!("\n4. Extensions with pattern '?sx' (jsx, tsx, etc.):");
    let query = WildcardQuery::new("extension", "?sx")?;
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
            && let Some(field) = doc.get_field("extension")
            && let DataValue::Text(ext) = field
        {
            println!("      Extension: {ext}");
        }
    }

    // Example 5: Multiple wildcards
    println!("\n5. Categories starting with 'prog' and ending with 'ing' using 'prog*ing':");
    let query = WildcardQuery::new("category", "prog*ing")?;
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
            && let Some(field) = doc.get_field("category")
            && let DataValue::Text(category) = field
        {
            println!("      Category: {category}");
        }
    }

    // Example 6: Complex pattern with both wildcards
    println!("\n6. Filenames with pattern '*_*.????' (underscore and 4-char extension):");
    let query = WildcardQuery::new("filename", "*_*.????")?;
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
            && let Some(field) = doc.get_field("filename")
            && let DataValue::Text(filename) = field
        {
            println!("      Filename: {filename}");
        }
    }

    // Example 7: Title pattern matching
    println!("\n7. Titles containing 'Development' using '*Development*':");
    let query = WildcardQuery::new("title", "*Development*")?;
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

    // Example 8: Single character matching
    println!("\n8. Extensions with exactly 3 characters using '???':");
    let query = WildcardQuery::new("extension", "???")?;
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
            && let Some(field) = doc.get_field("extension")
            && let DataValue::Text(ext) = field
        {
            println!("      Extension: {ext}");
        }
    }

    // Example 9: Match all files with any extension
    println!("\n9. All files with any extension using '*.*':");
    let query = WildcardQuery::new("filename", "*.*")?;
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
            && let Some(field) = doc.get_field("filename")
            && let DataValue::Text(filename) = field
        {
            println!("      Filename: {filename}");
        }
    }

    // Example 10: No matches
    println!("\n10. Pattern with no matches using 'xyz*abc':");
    let query = WildcardQuery::new("filename", "xyz*abc")?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);

    // Example 11: Count matching documents
    println!("\n11. Counting files with 'data' in filename using '*data*':");
    let query = WildcardQuery::new("filename", "*data*")?;
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("    Count: {count} files");

    lexical_engine.close()?;
    println!("\nWildcardQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wildcard_query_example() {
        let result = main().await;
        assert!(result.is_ok());
    }
}
