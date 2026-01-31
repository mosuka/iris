use tempfile::TempDir;

use iris::Document;
use iris::Engine;
use iris::lexical::FieldOption as LexicalOption;
use iris::lexical::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::VectorOption;
use iris::vector::VectorSearchRequestBuilder;
use iris::{FieldConfig, IndexConfig};
use iris::{FusionAlgorithm, SearchRequestBuilder};

#[test]
fn test_advanced_fusion_normalization() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    let vector_opt = VectorOption::default();
    let lexical_opt = LexicalOption::default();

    let config = IndexConfig::builder()
        .add_field(
            "title",
            FieldConfig {
                lexical: Some(lexical_opt.clone()),
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

    let engine = Engine::new(storage, config)?;

    // 3. Index Documents
    // Doc 1: Good lexical, Bad vector
    let mut vec1 = vec![0.0; 128];
    vec1[0] = 1.0;
    engine.index(
        Document::new_with_id("doc1")
            .add_field("title", "apple")
            .add_field("embedding", vec1),
    )?;

    // Doc 2: Bad lexical, Good vector
    let mut vec2 = vec![0.0; 128];
    vec2[1] = 1.0;
    engine.index(
        Document::new_with_id("doc2")
            .add_field("title", "banana")
            .add_field("embedding", vec2),
    )?;

    engine.commit()?;

    let mut query_vec = vec![0.0; 128];
    query_vec[1] = 1.0;
    let request = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("title", "apple")))
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_vector("embedding", query_vec)
                .build(),
        )
        .fusion(FusionAlgorithm::WeightedSum {
            lexical_weight: 0.5,
            vector_weight: 0.5,
        })
        .build();

    let results = engine.search(request)?;
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

#[test]
fn test_field_boosts() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    let config = IndexConfig::builder()
        .add_field(
            "title",
            FieldConfig {
                lexical: Some(LexicalOption::default()),
                vector: None,
            },
        )
        .add_field(
            "body",
            FieldConfig {
                lexical: Some(LexicalOption::default()),
                vector: None,
            },
        )
        .build();

    let engine = Engine::new(storage, config)?;

    // 3. Index Documents
    engine.index(
        Document::new_with_id("doc1")
            .add_field("title", "rust")
            .add_field("body", "programming"),
    )?;
    engine.index(
        Document::new_with_id("doc2")
            .add_field("title", "java")
            .add_field("body", "rust"),
    )?;
    engine.commit()?;

    // 4. Search for "rust" in both fields with different boosts
    // Case A: Boost title
    let req_a = SearchRequestBuilder::new()
        .with_lexical(Box::new(
            iris::lexical::BooleanQueryBuilder::new()
                .should(Box::new(TermQuery::new("title", "rust")))
                .should(Box::new(TermQuery::new("body", "rust")))
                .build(),
        ))
        .add_field_boost("title", 10.0)
        .add_field_boost("body", 1.0)
        .build();

    let res_a = engine.search(req_a)?;
    assert_eq!(res_a[0].doc_id, 1); // doc1 has "rust" in title (doc_id 1 if indexed first)
    // Wait, doc_id might be internal. Let's check which doc it is.
    let doc_a = engine.get_document(res_a[0].doc_id)?.unwrap();
    assert_eq!(
        doc_a.id.as_deref(),
        Some("doc1"),
        "Doc 1 should win when title is boosted"
    );

    // Case B: Boost body
    let req_b = SearchRequestBuilder::new()
        .with_lexical(Box::new(
            iris::lexical::BooleanQueryBuilder::new()
                .should(Box::new(TermQuery::new("title", "rust")))
                .should(Box::new(TermQuery::new("body", "rust")))
                .build(),
        ))
        .add_field_boost("title", 1.0)
        .add_field_boost("body", 10.0)
        .build();

    let res_b = engine.search(req_b)?;
    let doc_b = engine.get_document(res_b[0].doc_id)?.unwrap();
    assert_eq!(
        doc_b.id.as_deref(),
        Some("doc2"),
        "Doc 2 should win when body is boosted"
    );

    Ok(())
}
