use std::sync::Arc;

use sarissa::analysis::analyzer::standard::StandardAnalyzer;
use sarissa::lexical::core::document::Document;
use sarissa::lexical::core::field::TextOption;
use sarissa::lexical::index::inverted::query::Query;
use sarissa::lexical::index::inverted::query::span::{
    SpanNearQuery, SpanQuery, SpanQueryBuilder, SpanQueryWrapper, SpanTermQuery,
};
use sarissa::lexical::index::inverted::writer::{InvertedIndexWriter, InvertedIndexWriterConfig};
use sarissa::lexical::writer::LexicalIndexWriter;
use sarissa::storage::memory::MemoryStorage;

#[test]
fn test_span_term_query_integration() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Storage and Writer
    let storage = Arc::new(MemoryStorage::new(
        sarissa::storage::memory::MemoryStorageConfig::default(),
    ));
    let config = InvertedIndexWriterConfig {
        analyzer: Arc::new(StandardAnalyzer::new()?),
        ..Default::default()
    };
    let mut writer = InvertedIndexWriter::new(storage.clone(), config)?;

    // 2. Add Documents
    // Doc 0: "hello world"
    writer.add_document(
        Document::builder()
            .add_text("content", "hello world", TextOption::default())
            .build(),
    )?;
    // Doc 1: "world hello"
    writer.add_document(
        Document::builder()
            .add_text("content", "world hello", TextOption::default())
            .build(),
    )?;

    writer.commit()?;
    let reader = writer.build_reader()?;

    // 3. Test SpanTermQuery
    let query = SpanTermQuery::new("content", "hello");

    // Doc 0: "hello" is at 0
    let spans0 = query.get_spans(0, reader.as_ref())?;
    assert_eq!(spans0.len(), 1);
    assert_eq!(spans0[0].start, 0);
    assert_eq!(spans0[0].end, 1);

    // Doc 1: "hello" is at 1
    let spans1 = query.get_spans(1, reader.as_ref())?;
    assert_eq!(spans1.len(), 1);
    assert_eq!(spans1[0].start, 1);
    assert_eq!(spans1[0].end, 2);

    Ok(())
}

#[test]
fn test_span_query_wrapper_integration() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Storage and Writer
    let storage = Arc::new(MemoryStorage::new(Default::default()));
    let config = InvertedIndexWriterConfig {
        analyzer: Arc::new(StandardAnalyzer::new()?),
        ..Default::default()
    };
    let mut writer = InvertedIndexWriter::new(storage.clone(), config)?;

    // 2. Add Documents
    // Doc 0
    writer.add_document(
        Document::builder()
            .add_text("content", "apple banana cherry", TextOption::default())
            .build(),
    )?;
    // Doc 1
    writer.add_document(
        Document::builder()
            .add_text("content", "banana cherry apple", TextOption::default())
            .build(),
    )?;

    writer.commit()?;
    let reader = writer.build_reader()?;

    // 3. Test SpanQueryWrapper with SpanTermQuery
    let span_query = Box::new(SpanTermQuery::new("content", "apple"));
    let wrapper = SpanQueryWrapper::new(span_query);

    let mut matcher = wrapper.matcher(reader.as_ref())?;

    // Should match Doc 0 ("apple" at 0)
    assert!(matcher.next()?);
    assert_eq!(matcher.doc_id(), 0);

    // Should match Doc 1 ("apple" at 2)
    assert!(matcher.next()?);
    assert_eq!(matcher.doc_id(), 1);

    assert!(!matcher.next()?);

    Ok(())
}
