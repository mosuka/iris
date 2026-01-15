use chrono::{TimeZone, Utc};
use sarissa::lexical::core::document::Document;
use sarissa::lexical::core::field::{DateTimeOption, FloatOption, IntegerOption, NumericType};
use sarissa::lexical::index::inverted::query::Query;
use sarissa::lexical::index::inverted::query::range::NumericRangeQuery;
use sarissa::lexical::index::inverted::writer::{InvertedIndexWriter, InvertedIndexWriterConfig};
use sarissa::lexical::writer::LexicalIndexWriter;
use sarissa::storage::Storage;
use sarissa::storage::memory::{MemoryStorage, MemoryStorageConfig};
use std::sync::Arc;

#[test]
fn test_bkd_file_creation_and_query() {
    let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
    let config = InvertedIndexWriterConfig {
        max_buffered_docs: 10,
        ..Default::default()
    };
    let mut writer = InvertedIndexWriter::new(storage.clone(), config).unwrap();

    // Add documents
    // Doc 1: age=30, score=95.5
    let doc1 = Document::builder()
        .add_integer(
            "age",
            30,
            IntegerOption {
                indexed: true,
                stored: true,
            },
        )
        .add_float(
            "score",
            95.5,
            FloatOption {
                indexed: true,
                stored: true,
            },
        )
        .add_datetime(
            "created_at",
            Utc.timestamp_opt(1600000000, 0).unwrap(),
            DateTimeOption {
                indexed: true,
                stored: true,
            },
        )
        .add_text("description", "User profile 1", Default::default())
        .build();
    writer.add_document(doc1).unwrap();

    // Doc 2: age=20, score=80.0
    let doc2 = Document::builder()
        .add_integer(
            "age",
            20,
            IntegerOption {
                indexed: true,
                stored: true,
            },
        )
        .add_float(
            "score",
            80.0,
            FloatOption {
                indexed: true,
                stored: true,
            },
        )
        .add_datetime(
            "created_at",
            Utc.timestamp_opt(1500000000, 0).unwrap(),
            DateTimeOption {
                indexed: true,
                stored: true,
            },
        )
        .add_text("description", "User profile 2", Default::default())
        .build();
    writer.add_document(doc2).unwrap();

    // Doc 3: age=40, score=100.0
    let doc3 = Document::builder()
        .add_integer(
            "age",
            40,
            IntegerOption {
                indexed: true,
                stored: true,
            },
        )
        .add_float(
            "score",
            100.0,
            FloatOption {
                indexed: true,
                stored: true,
            },
        )
        .add_datetime(
            "created_at",
            Utc.timestamp_opt(1700000000, 0).unwrap(),
            DateTimeOption {
                indexed: true,
                stored: true,
            },
        )
        .add_text("description", "User profile 3", Default::default())
        .build();
    writer.add_document(doc3).unwrap();

    // Commit to flush segment
    writer.commit().unwrap();

    // Verify files existed
    let age_bkd = "segment_000000.age.bkd";
    assert!(
        storage.file_exists(age_bkd),
        "BKD file for age should exist"
    );

    // Open Reader
    let reader = writer.build_reader().unwrap();

    // Query 1: Age [25, 35] -> Should match Doc 1 (age 30) -> ID 0
    let query_age = NumericRangeQuery::new(
        "age",
        NumericType::Integer,
        Some(25.0),
        Some(35.0),
        true,
        true,
    );

    let helper_collect =
        |mut m: Box<dyn sarissa::lexical::index::inverted::query::matcher::Matcher>| -> Vec<u64> {
            let mut docs = Vec::new();
            // Assuming Matcher::next returns bool
            // PreComputedMatcher might behave properly.
            while m.next().unwrap() {
                docs.push(m.doc_id());
            }
            docs
        };

    let matcher = query_age.matcher(&*reader).unwrap();
    let matched_age = helper_collect(matcher);

    assert_eq!(matched_age, vec![0]);

    // Query 2: Score >= 90.0 -> Doc 1 (95.5), Doc 3 (100.0) -> IDs 0, 2
    let query_score =
        NumericRangeQuery::new("score", NumericType::Float, Some(90.0), None, true, true);
    let matched_score = helper_collect(query_score.matcher(&*reader).unwrap());

    assert_eq!(matched_score, vec![0, 2]);

    // Query 3: Created At < 1600000000 -> Doc 2 (1500000000) -> ID 1
    let query_date = NumericRangeQuery::new(
        "created_at",
        NumericType::Integer,
        None,
        Some(1600000000.0),
        false,
        false,
    );
    let matched_date = helper_collect(query_date.matcher(&*reader).unwrap());

    assert_eq!(matched_date, vec![1]);
}
