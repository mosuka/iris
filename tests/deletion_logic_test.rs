use tempfile::TempDir;

use iris::Document;
use iris::Engine;
use iris::SearchRequestBuilder;
use iris::lexical::FieldOption as LexicalOption;
use iris::lexical::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::FieldOption as VectorOption;
use iris::vector::VectorSearchRequestBuilder;
use iris::{FieldOption, Schema};

#[test]
fn test_engine_unified_deletion() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    let vector_opt = VectorOption::default(); // dim=128
    let lexical_opt = LexicalOption::default();

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(lexical_opt))
        .add_field("embedding", FieldOption::Vector(vector_opt))
        .build();

    let engine = Engine::new(storage.clone(), config)?;

    // 3. Index Document with ID "doc1"
    let doc1 = Document::builder()
        .add_field("title", "Hello Iris")
        .add_field("embedding", vec![0.1; 128])
        .build();

    engine.put_document("doc1", doc1)?;
    engine.commit()?;

    // 4. Verify it exists in both
    // Lexical check
    let req_lexical = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("title", "hello")))
        .build();
    let res_lexical = engine.search(req_lexical)?;
    assert_eq!(res_lexical.len(), 1, "Should be found lexically");

    // Vector check
    let req_vector = SearchRequestBuilder::new()
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_vector("embedding", vec![0.1; 128])
                .build(),
        )
        .build();
    let res_vector = engine.search(req_vector)?;
    assert_eq!(res_vector.len(), 1, "Should be found via vector");

    // 5. Delete by ID
    engine.delete_documents("doc1")?;
    engine.commit()?;

    // 6. Verify it is GONE from both
    let res_lexical_after = engine.search(
        SearchRequestBuilder::new()
            .with_lexical(Box::new(TermQuery::new("title", "hello")))
            .build(),
    )?;
    assert_eq!(res_lexical_after.len(), 0, "Should be deleted lexically");

    let res_vector_after = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_vector("embedding", vec![0.1; 128])
                    .build(),
            )
            .build(),
    )?;
    assert_eq!(res_vector_after.len(), 0, "Should be deleted via vector");

    Ok(())
}

#[test]
fn test_engine_upsert() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    use iris::vector::FlatOption;
    let vector_opt = VectorOption::Flat(FlatOption {
        dimension: 2,
        ..Default::default()
    });
    let lexical_opt = LexicalOption::default();

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(lexical_opt))
        .add_field("embedding", FieldOption::Vector(vector_opt))
        .build();

    let engine = Engine::new(storage.clone(), config)?;

    // 3. Index Document with ID "doc1"
    let doc1 = Document::builder()
        .add_field("title", "Initial Version")
        .add_field("embedding", vec![1.0, 0.0])
        .build();

    engine.put_document("doc1", doc1)?;
    engine.commit()?;

    // 4. Verify initial version exists
    let res = engine.search(
        SearchRequestBuilder::new()
            .with_lexical(Box::new(TermQuery::new("title", "initial")))
            .build(),
    )?;
    assert_eq!(res.len(), 1);

    // 5. Index updated document with SAME ID "doc1"
    let doc1_v2 = Document::builder()
        .add_field("title", "Updated Version")
        .add_field("embedding", vec![0.0, 1.0])
        .build();

    engine.put_document("doc1", doc1_v2)?;
    engine.commit()?;

    // 6. Verify update
    // Old version lookup should fail
    let res_old = engine.search(
        SearchRequestBuilder::new()
            .with_lexical(Box::new(TermQuery::new("title", "initial")))
            .build(),
    )?;
    assert_eq!(res_old.len(), 0, "Old version should be replaced");

    // New version lookup should succeed
    let res_new = engine.search(
        SearchRequestBuilder::new()
            .with_lexical(Box::new(TermQuery::new("title", "updated")))
            .build(),
    )?;
    assert_eq!(res_new.len(), 1, "New version should be found");

    // Vector check for new vector
    let res_vec = engine.search(
        SearchRequestBuilder::new()
            .with_vector(
                VectorSearchRequestBuilder::new()
                    .add_vector("embedding", vec![0.0, 1.0])
                    .build(),
            )
            .build(),
    )?;
    assert_eq!(res_vec.len(), 1, "New vector should be found");

    Ok(())
}
