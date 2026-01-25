use tempfile::TempDir;

use iris::data::{DataValue, Document};
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::engine::search::SearchRequestBuilder;
use iris::lexical::core::field::FieldOption as LexicalOption;
use iris::lexical::index::inverted::query::Query;
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::field::VectorOption;
use iris::vector::store::query::VectorSearchRequestBuilder;

#[test]
fn test_unified_search_hybrid() -> iris::error::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines
    use iris::vector::core::field::FlatOption;
    let vector_opt: VectorOption = FlatOption::default().dimension(3).into();
    let lexical_opt = LexicalOption::default();

    let config = IndexConfig::builder()
        .add_field(
            "title",
            FieldConfig {
                lexical: Some(lexical_opt),
                vector: None,
            },
        )
        .add_field(
            "embedding",
            FieldConfig {
                lexical: None,
                vector: Some(vector_opt),
            },
        )
        .build();

    let engine = Engine::new(storage.clone(), config)?;

    // 3. Index Documents
    let doc1 = Document::new()
        .with_id("doc1")
        .with_field("title", DataValue::Text("Rust Programming".into()))
        .with_field("embedding", DataValue::Vector(vec![1.0, 0.0, 0.0]));

    let doc2 = Document::new()
        .with_id("doc2")
        .with_field("title", DataValue::Text("Vector Search".into()))
        .with_field("embedding", DataValue::Vector(vec![0.0, 1.0, 0.0]));

    engine.index(doc1)?;
    engine.index(doc2)?;
    engine.commit()?;

    // 4. Test Lexical Search (should find "doc1")
    let lexical_query = Box::new(TermQuery::new("title", "rust")) as Box<dyn Query>;
    let req = SearchRequestBuilder::new()
        .with_lexical(lexical_query)
        .build();

    let results = engine.search(req)?;
    println!("Lexical Results: {:?}", results);
    assert!(
        results.iter().any(|r| r.score > 0.0),
        "Should match doc1 lexically"
    );
    // Note: doc_id is internal, but since we indexed sequentially:
    // doc1 = 0, doc2 = 1 likely.
    // However, VectorStore and LexicalStore might assign different IDs!
    // Since we didn't solve ID mapping yet, results.doc_id corresponds to the engine that returned it.
    // If Vector returned it, it's VectorID. If Lexical, LexicalID.
    // They are synchronized by insertion order IF both succeed.

    // 5. Test Vector Search (should find "doc2" which is closer to [0, 1, 0])
    let vector_req = VectorSearchRequestBuilder::new()
        .add_vector("embedding", vec![0.0, 1.0, 0.0])
        .build();

    let req = SearchRequestBuilder::new().with_vector(vector_req).build();

    let results = engine.search(req)?;
    println!("Vector Results: {:?}", results);
    assert!(!results.is_empty(), "Should return vector results");
    // doc2 (vec [0,1,0]) should match perfectly with query [0,1,0].

    Ok(())
}

#[test]
fn test_unified_search_hybrid_fusion() -> iris::error::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    let config = IndexConfig::builder()
        .add_field(
            "title",
            FieldConfig {
                lexical: Some(Default::default()),
                vector: None,
            },
        )
        .add_field(
            "embedding",
            FieldConfig {
                lexical: None,
                vector: Some(
                    iris::vector::core::field::FlatOption::default()
                        .dimension(3)
                        .into(),
                ),
            },
        )
        .build();

    let engine = Engine::new(storage, config)?;

    // Index documents where Lexical and Vector favorites differ
    // Doc 1: "Rust" in title, Vector [1, 0, 0]
    // Doc 2: "C++" in title, Vector [0, 1, 0]
    engine.index(
        Document::new()
            .with_id("1")
            .with_field("title", "Rust")
            .with_field("embedding", vec![1.0, 0.0, 0.0]),
    )?;
    engine.index(
        Document::new()
            .with_id("2")
            .with_field("title", "C++")
            .with_field("embedding", vec![0.0, 1.0, 0.0]),
    )?;
    engine.commit()?;

    // Search for "Rust" (Lexical) AND [0, 1, 0] (Vector - matches Doc 2)
    use iris::engine::search::FusionAlgorithm;
    let req = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("title", "rust")))
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_vector("embedding", vec![0.0, 1.0, 0.0])
                .build(),
        )
        .fusion(FusionAlgorithm::RRF { k: 60.0 })
        .build();

    let results = engine.search(req)?;

    // In RRF, both Doc 1 and Doc 2 should be present because Doc 1 is top in Lexical, Doc 2 is top in Vector.
    assert_eq!(results.len(), 2);

    // Check that documents are loaded
    for res in results {
        assert!(
            res.document.is_some(),
            "Document should be loaded during fusion"
        );
    }

    Ok(())
}
