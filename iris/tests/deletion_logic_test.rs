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
use iris::{FieldOption, LexicalSearchRequest, Schema};

// Ignored on Windows due to FileStorage file handle synchronization issues.
#[cfg_attr(target_os = "windows", ignore)]
#[tokio::test(flavor = "multi_thread")]
async fn test_engine_unified_deletion() -> iris::Result<()> {
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

    let engine = Engine::new(storage.clone(), config).await?;

    // 3. Index Document with ID "doc1"
    let doc1 = Document::builder()
        .add_field("title", "Hello Iris")
        .add_field("embedding", vec![0.1; 128])
        .build();

    engine.put_document("doc1", doc1).await?;
    engine.commit().await?;

    // 4. Verify it exists in both
    // Lexical check
    let req_lexical = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
            "title", "hello",
        ))))
        .build();
    let res_lexical = engine.search(req_lexical).await?;
    assert_eq!(res_lexical.len(), 1, "Should be found lexically");

    // Vector check
    let req_vector = SearchRequestBuilder::new()
        .vector_search_request(
            VectorSearchRequestBuilder::new()
                .add_vector("embedding", vec![0.1; 128])
                .build(),
        )
        .build();
    let res_vector = engine.search(req_vector).await?;
    assert_eq!(res_vector.len(), 1, "Should be found via vector");

    // 5. Delete by ID
    engine.delete_documents("doc1").await?;
    engine.commit().await?;

    // 6. Verify it is GONE from both
    let res_lexical_after = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "title", "hello",
                ))))
                .build(),
        )
        .await?;
    assert_eq!(res_lexical_after.len(), 0, "Should be deleted lexically");

    let res_vector_after = engine
        .search(
            SearchRequestBuilder::new()
                .vector_search_request(
                    VectorSearchRequestBuilder::new()
                        .add_vector("embedding", vec![0.1; 128])
                        .build(),
                )
                .build(),
        )
        .await?;
    assert_eq!(res_vector_after.len(), 0, "Should be deleted via vector");

    Ok(())
}

// Ignored on Windows due to FileStorage file handle synchronization issues.
#[cfg_attr(target_os = "windows", ignore)]
#[tokio::test(flavor = "multi_thread")]
async fn test_engine_upsert() -> iris::Result<()> {
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

    let engine = Engine::new(storage.clone(), config).await?;

    // 3. Index Document with ID "doc1"
    let doc1 = Document::builder()
        .add_field("title", "Initial Version")
        .add_field("embedding", vec![1.0, 0.0])
        .build();

    engine.put_document("doc1", doc1).await?;
    engine.commit().await?;

    // 4. Verify initial version exists
    let res = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "title", "initial",
                ))))
                .build(),
        )
        .await?;
    assert_eq!(res.len(), 1);

    // 5. Index updated document with SAME ID "doc1"
    let doc1_v2 = Document::builder()
        .add_field("title", "Updated Version")
        .add_field("embedding", vec![0.0, 1.0])
        .build();

    engine.put_document("doc1", doc1_v2).await?;
    engine.commit().await?;

    // 6. Verify update
    // Old version lookup should fail
    let res_old = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "title", "initial",
                ))))
                .build(),
        )
        .await?;
    assert_eq!(res_old.len(), 0, "Old version should be replaced");

    // New version lookup should succeed
    let res_new = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "title", "updated",
                ))))
                .build(),
        )
        .await?;
    assert_eq!(res_new.len(), 1, "New version should be found");

    // Vector check for new vector
    let res_vec = engine
        .search(
            SearchRequestBuilder::new()
                .vector_search_request(
                    VectorSearchRequestBuilder::new()
                        .add_vector("embedding", vec![0.0, 1.0])
                        .build(),
                )
                .build(),
        )
        .await?;
    assert_eq!(res_vec.len(), 1, "New vector should be found");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_engine_delete_nonexistent() -> iris::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    use iris::vector::FlatOption;
    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(LexicalOption::default()))
        .add_field(
            "embedding",
            FieldOption::Vector(VectorOption::Flat(FlatOption {
                dimension: 2,
                ..Default::default()
            })),
        )
        .build();

    let engine = Engine::new(storage, config).await?;

    // Deleting a non-existent document should not error
    let result = engine.delete_documents("nonexistent_doc").await;
    assert!(
        result.is_ok(),
        "Deleting non-existent document should succeed silently"
    );

    Ok(())
}

// Ignored on Windows due to FileStorage file handle synchronization issues.
#[cfg_attr(target_os = "windows", ignore)]
#[tokio::test(flavor = "multi_thread")]
async fn test_engine_double_delete() -> iris::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    use iris::vector::FlatOption;
    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(LexicalOption::default()))
        .add_field(
            "embedding",
            FieldOption::Vector(VectorOption::Flat(FlatOption {
                dimension: 2,
                ..Default::default()
            })),
        )
        .build();

    let engine = Engine::new(storage, config).await?;

    // Index a document
    engine
        .put_document(
            "doc1",
            Document::builder()
                .add_field("title", "Hello")
                .add_field("embedding", vec![1.0, 0.0])
                .build(),
        )
        .await?;
    engine.commit().await?;

    // Delete it twice — second delete should succeed silently
    engine.delete_documents("doc1").await?;
    engine.commit().await?;

    let result = engine.delete_documents("doc1").await;
    assert!(result.is_ok(), "Double delete should succeed silently");
    engine.commit().await?;

    // Verify document is gone
    let res = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "title", "hello",
                ))))
                .build(),
        )
        .await?;
    assert_eq!(res.len(), 0, "Document should remain deleted");

    Ok(())
}
