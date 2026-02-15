use tempfile::TempDir;

use iris::Document;
use iris::Engine;
use iris::lexical::FieldOption as LexicalOption;
use iris::lexical::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::FieldOption as VectorOption;
use iris::vector::VectorSearchRequestBuilder;
use iris::{FieldOption, Schema};
use iris::{FusionAlgorithm, LexicalSearchRequest, SearchRequestBuilder};

#[tokio::test(flavor = "multi_thread")]
async fn test_advanced_fusion_normalization() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    let vector_opt = VectorOption::default();
    let lexical_opt = LexicalOption::default();

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(lexical_opt))
        .add_field("embedding", FieldOption::Vector(vector_opt))
        .build();

    let engine = Engine::new(storage, config).await?;

    // 3. Index Documents
    // Doc 1: Good lexical, Bad vector
    let mut vec1 = vec![0.0; 128];
    vec1[0] = 1.0;
    engine
        .put_document(
            "doc1",
            Document::builder()
                .add_field("title", "apple")
                .add_field("embedding", vec1)
                .build(),
        )
        .await?;

    // Doc 2: Bad lexical, Good vector
    let mut vec2 = vec![0.0; 128];
    vec2[1] = 1.0;
    engine
        .put_document(
            "doc2",
            Document::builder()
                .add_field("title", "banana")
                .add_field("embedding", vec2)
                .build(),
        )
        .await?;

    engine.commit().await?;

    let mut query_vec = vec![0.0; 128];
    query_vec[1] = 1.0;
    let request = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
            "title", "apple",
        ))))
        .vector_search_request(
            VectorSearchRequestBuilder::new()
                .add_vector("embedding", query_vec)
                .build(),
        )
        .fusion_algorithm(FusionAlgorithm::WeightedSum {
            lexical_weight: 0.5,
            vector_weight: 0.5,
        })
        .build();

    let results = engine.search(request).await?;
    assert_eq!(results.len(), 2);

    // Without normalization, vector scores (cosine similarity with small values)
    // might be much smaller than lexical scores (BM25 for rare term).
    // With Min-Max normalization, both get [0, 1] range.
    // Doc 1: Lexical=1.0, Vector=0.0 -> WeightedSum = 0.5
    // Doc 2: Lexical=0.0, Vector=1.0 -> WeightedSum = 0.5
    // Actually, with only 2 docs, one is min and one is max.
    // They should have equal scores if weights are equal.
    assert_eq!(results[0].score, results[1].score);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_field_boosts() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(LexicalOption::default()))
        .add_field("body", FieldOption::Lexical(LexicalOption::default()))
        .build();

    let engine = Engine::new(storage, config).await?;

    // 3. Index Documents
    engine
        .put_document(
            "doc1",
            Document::builder()
                .add_field("title", "rust")
                .add_field("body", "programming")
                .build(),
        )
        .await?;
    engine
        .put_document(
            "doc2",
            Document::builder()
                .add_field("title", "java")
                .add_field("body", "rust")
                .build(),
        )
        .await?;
    engine.commit().await?;

    // 4. Search for "rust" in both fields with different boosts
    // Case A: Boost title
    let req_a = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(Box::new(
            iris::lexical::BooleanQueryBuilder::new()
                .should(Box::new(TermQuery::new("title", "rust")))
                .should(Box::new(TermQuery::new("body", "rust")))
                .build(),
        )))
        .add_field_boost("title", 10.0)
        .add_field_boost("body", 1.0)
        .build();

    let res_a = engine.search(req_a).await?;
    // res_a[0].id is the external ID (String)
    let docs_a = engine.get_documents(&res_a[0].id).await?;
    let doc_a = &docs_a[0];
    assert_eq!(
        doc_a.fields.get("_id").and_then(|v| v.as_text()),
        Some("doc1"),
        "Doc 1 should win when title is boosted"
    );

    // Case B: Boost body
    let req_b = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(Box::new(
            iris::lexical::BooleanQueryBuilder::new()
                .should(Box::new(TermQuery::new("title", "rust")))
                .should(Box::new(TermQuery::new("body", "rust")))
                .build(),
        )))
        .add_field_boost("title", 1.0)
        .add_field_boost("body", 10.0)
        .build();

    let res_b = engine.search(req_b).await?;
    let docs_b = engine.get_documents(&res_b[0].id).await?;
    let doc_b = &docs_b[0];
    assert_eq!(
        doc_b.fields.get("_id").and_then(|v| v.as_text()),
        Some("doc2"),
        "Doc 2 should win when body is boosted"
    );

    Ok(())
}
