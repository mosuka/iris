//! PhraseQuery example - demonstrates phrase search for exact word sequences.

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
use iris::lexical::PhraseQuery;
use iris::lexical::Query;
use iris::parking_lot::RwLock;
use iris::storage::StorageConfig;
use iris::storage::StorageFactory;
use iris::storage::file::FileStorageConfig;
use iris::storage::prefixed::PrefixedStorage;
use iris::store::document::UnifiedDocumentStore;
use iris::{DataValue, Document};

fn main() -> Result<()> {
    println!("=== PhraseQuery Example - Exact Phrase Matching ===\n");

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

    // Add documents with various phrases
    let documents = vec![
        Document::new()
            .add_field("title", DataValue::Text("Machine Learning Basics".into()))
            .add_field("body", DataValue::Text("Machine learning is a powerful tool for data analysis and artificial intelligence applications".into()))
            .add_field("author", DataValue::Text("Dr. Smith".into()))
            .add_field("description", DataValue::Text("An introduction to machine learning concepts and algorithms".into()))
            .add_field("id", DataValue::Text("001".into())),
        Document::new()
            .add_field("title", DataValue::Text("Deep Learning Networks".into()))
            .add_field("body", DataValue::Text("Deep learning networks use artificial neural networks with multiple layers for complex pattern recognition".into()))
            .add_field("author", DataValue::Text("Prof. Johnson".into()))
            .add_field("description", DataValue::Text("Advanced techniques in deep learning and neural network architectures".into()))
            .add_field("id", DataValue::Text("002".into())),
        Document::new()
            .add_field("title", DataValue::Text("Natural Language Processing".into()))
            .add_field("body", DataValue::Text("Natural language processing combines computational linguistics with machine learning and artificial intelligence".into()))
            .add_field("author", DataValue::Text("Dr. Wilson".into()))
            .add_field("description", DataValue::Text("Processing and understanding human language using computational methods".into()))
            .add_field("id", DataValue::Text("003".into())),
        Document::new()
            .add_field("title", DataValue::Text("Computer Vision Applications".into()))
            .add_field("body", DataValue::Text("Computer vision applications include image recognition, object detection, and visual pattern analysis".into()))
            .add_field("author", DataValue::Text("Prof. Davis".into()))
            .add_field("description", DataValue::Text("Practical applications of computer vision in various industries".into()))
            .add_field("id", DataValue::Text("004".into())),
        Document::new()
            .add_field("title", DataValue::Text("Data Science Fundamentals".into()))
            .add_field("body", DataValue::Text("Data science combines statistics, programming, and domain expertise to extract insights from data".into()))
            .add_field("author", DataValue::Text("Dr. Brown".into()))
            .add_field("description", DataValue::Text("Essential concepts and tools for data science practitioners".into()))
            .add_field("id", DataValue::Text("005".into())),
    ];

    println!("Adding {} documents to the index...", documents.len());
    for doc in documents {
        lexical_engine.add_document(doc)?;
    }

    lexical_engine.commit()?;

    println!("\n=== PhraseQuery Examples ===\n");

    // Example 1: Simple two-word phrase
    println!("1. Searching for phrase 'machine learning' in body:");
    let query = PhraseQuery::new("body", vec!["machine".to_string(), "learning".to_string()]);
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

    // Example 2: Three-word phrase
    println!("\n2. Searching for phrase 'artificial neural networks' in body:");
    let query = PhraseQuery::new(
        "body",
        vec![
            "artificial".to_string(),
            "neural".to_string(),
            "networks".to_string(),
        ],
    );
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

    // Example 3: Phrase in title field
    println!("\n3. Searching for phrase 'deep learning' in title:");
    let query = PhraseQuery::new("title", vec!["deep".to_string(), "learning".to_string()]);
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

    // Example 4: Phrase with common words
    println!("\n4. Searching for phrase 'data science' in description:");
    let query = PhraseQuery::new(
        "description",
        vec!["data".to_string(), "science".to_string()],
    );
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

    // Example 5: Non-existent phrase
    println!("\n5. Searching for non-existent phrase 'quantum computing':");
    let query = PhraseQuery::new("body", vec!["quantum".to_string(), "computing".to_string()]);
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);

    // Example 6: Single word phrase (equivalent to TermQuery)
    println!("\n6. Searching for single word phrase 'intelligence' in body:");
    let query = PhraseQuery::new("body", vec!["intelligence".to_string()]);
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

    // Example 7: Longer phrase search
    println!("\n7. Searching for long phrase 'extract insights from data' in body:");
    let query = PhraseQuery::new(
        "body",
        vec![
            "extract".to_string(),
            "insights".to_string(),
            "from".to_string(),
            "data".to_string(),
        ],
    );
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

    // Example 8: Count phrase matches
    println!("\n8. Counting documents with phrase 'computer vision':");
    let query = PhraseQuery::new("body", vec!["computer".to_string(), "vision".to_string()]);
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("   Count: {count} documents");

    lexical_engine.close()?;
    println!("\nPhraseQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phrase_query_example() {
        let result = main();
        assert!(result.is_ok());
    }
}
