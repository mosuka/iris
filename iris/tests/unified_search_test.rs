use tempfile::TempDir;

use iris::Engine;
use iris::SearchRequestBuilder;
use iris::lexical::FieldOption as LexicalOption;
use iris::lexical::Query;
use iris::lexical::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::FieldOption as VectorOption;
use iris::vector::VectorSearchRequestBuilder;
use iris::{DataValue, Document};
use iris::{FieldOption, LexicalSearchRequest, Schema};

#[tokio::test(flavor = "multi_thread")]
async fn test_unified_search_hybrid() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines
    use iris::vector::FlatOption;
    let vector_opt: VectorOption = FlatOption::default().dimension(3).into();
    let lexical_opt = LexicalOption::default();

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(lexical_opt))
        .add_field("embedding", FieldOption::Vector(vector_opt))
        .build();

    let engine = Engine::new(storage.clone(), config).await?;

    // 3. Index Documents
    let doc1 = Document::builder()
        .add_field("title", DataValue::Text("Rust Programming".into()))
        .add_field("embedding", DataValue::Vector(vec![1.0, 0.0, 0.0]))
        .build();

    let doc2 = Document::builder()
        .add_field("title", DataValue::Text("Vector Search".into()))
        .add_field("embedding", DataValue::Vector(vec![0.0, 1.0, 0.0]))
        .build();

    engine.put_document("doc1", doc1).await?;
    engine.put_document("doc2", doc2).await?;
    engine.commit().await?;

    // 4. Test Lexical Search (should find "doc1")
    let lexical_query = Box::new(TermQuery::new("title", "rust")) as Box<dyn Query>;
    let req = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(lexical_query))
        .build();

    let results = engine.search(req).await?;
    println!("Lexical Results: {:?}", results);
    assert!(
        results.iter().any(|r| r.score > 0.0),
        "Should match doc1 lexically"
    );
    // Verify the correct document was found
    assert!(
        results.iter().any(|r| r.id == "doc1"),
        "Should find doc1 ('Rust Programming') for term query 'rust'"
    );

    // 5. Test Vector Search (should find "doc2" which is closer to [0, 1, 0])
    let vector_req = VectorSearchRequestBuilder::new()
        .add_vector("embedding", vec![0.0, 1.0, 0.0])
        .build();

    let req = SearchRequestBuilder::new()
        .vector_search_request(vector_req)
        .build();

    let results = engine.search(req).await?;
    println!("Vector Results: {:?}", results);
    assert!(!results.is_empty(), "Should return vector results");
    // The closest vector to [0,1,0] should be doc2's embedding [0,1,0]
    assert_eq!(
        results[0].id, "doc2",
        "Top result should be doc2 (exact match for [0,1,0])"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unified_search_hybrid_fusion() -> iris::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(Default::default()))
        .add_field(
            "embedding",
            FieldOption::Vector(iris::vector::FlatOption::default().dimension(3).into()),
        )
        .build();

    let engine = Engine::new(storage, config).await?;

    // Index documents where Lexical and Vector favorites differ
    // Doc 1: "Rust" in title, Vector [1, 0, 0]
    // Doc 2: "C++" in title, Vector [0, 1, 0]
    engine
        .put_document(
            "1",
            Document::builder()
                .add_field("title", "Rust")
                .add_field("embedding", vec![1.0, 0.0, 0.0])
                .build(),
        )
        .await?;
    engine
        .put_document(
            "2",
            Document::builder()
                .add_field("title", "C++")
                .add_field("embedding", vec![0.0, 1.0, 0.0])
                .build(),
        )
        .await?;
    engine.commit().await?;

    // Search for "Rust" (Lexical) AND [0, 1, 0] (Vector - matches Doc 2)
    use iris::FusionAlgorithm;
    let req = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
            "title", "rust",
        ))))
        .vector_search_request(
            VectorSearchRequestBuilder::new()
                .add_vector("embedding", vec![0.0, 1.0, 0.0])
                .build(),
        )
        .fusion_algorithm(FusionAlgorithm::RRF { k: 60.0 })
        .build();

    let results = engine.search(req).await?;

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
