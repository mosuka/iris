use iris::data::Document;
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::engine::search::SearchRequestBuilder;
use iris::error::Result;
use iris::lexical::core::field::{FieldOption, TextOption};
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};

#[test]
fn test_schema_lexical_guardrails() -> Result<()> {
    // 1. Setup Storage
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine with specific schema
    let config = IndexConfig::builder()
        .add_field(
            "indexed_and_stored",
            FieldConfig {
                lexical: Some(FieldOption::Text(TextOption {
                    indexed: true,
                    stored: true,
                    ..Default::default()
                })),
                vector: None,
            },
        )
        .add_field(
            "indexed_only",
            FieldConfig {
                lexical: Some(FieldOption::Text(TextOption {
                    indexed: true,
                    stored: false,
                    ..Default::default()
                })),
                vector: None,
            },
        )
        .add_field(
            "stored_only",
            FieldConfig {
                lexical: Some(FieldOption::Text(TextOption {
                    indexed: false,
                    stored: true,
                    ..Default::default()
                })),
                vector: None,
            },
        )
        .build();

    let engine = Engine::new(storage, config)?;

    // 3. Index a document with various fields (including one NOT in schema)
    let doc = Document::new_with_id("test1")
        .add_field("indexed_and_stored", "value1")
        .add_field("indexed_only", "value2")
        .add_field("stored_only", "value3")
        .add_field("unknown_field", "should be ignored");

    engine.index(doc)?;
    engine.commit()?;

    // 4. Verify Searching

    // Case A: Search "indexed_and_stored" -> Should find it
    let req_a = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("indexed_and_stored", "value1")))
        .build();
    let res_a = engine.search(req_a)?;
    assert_eq!(res_a.len(), 1, "Should find 'indexed_and_stored' field");
    let doc_a = engine.get_document(res_a[0].doc_id)?.unwrap();
    assert_eq!(
        doc_a
            .get_field("indexed_and_stored")
            .and_then(|v| v.as_text()),
        Some("value1")
    );

    // Case B: Search "indexed_only" -> Should find it but value should NOT be in get_document results
    let req_b = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("indexed_only", "value2")))
        .build();
    let res_b = engine.search(req_b)?;
    assert_eq!(res_b.len(), 1, "Should find 'indexed_only' field");
    let doc_b = engine.get_document(res_b[0].doc_id)?.unwrap();
    assert!(
        doc_b.get_field("indexed_only").is_none(),
        "Field 'indexed_only' should NOT be stored"
    );

    // Case C: Search "stored_only" -> Should NOT find it
    let req_c = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("stored_only", "value3")))
        .build();
    let res_c = engine.search(req_c)?;
    assert_eq!(
        res_c.len(),
        0,
        "Should NOT find 'stored_only' field via search"
    );
    // But it should be present in retrieval if we get by ID
    let doc_c = engine.get_document(res_a[0].doc_id)?.unwrap(); // use ID from earlier
    assert_eq!(
        doc_c.get_field("stored_only").and_then(|v| v.as_text()),
        Some("value3"),
        "Field 'stored_only' should be stored"
    );

    // Case D: Search "unknown_field" -> Should NOT find it
    let req_d = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("unknown_field", "should")))
        .build();
    let res_d = engine.search(req_d)?;
    assert_eq!(
        res_d.len(),
        0,
        "Should NOT find 'unknown_field' because it's not in schema"
    );
    let doc_d = engine.get_document(res_a[0].doc_id)?.unwrap();
    assert!(
        doc_d.get_field("unknown_field").is_none(),
        "Field 'unknown_field' should NOT be stored because it's not in schema"
    );

    Ok(())
}
