use tempfile::TempDir;

use iris::Engine;
use iris::lexical::FieldOption as LexicalOption;
use iris::lexical::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::FieldOption as VectorOption;
use iris::{DataValue, Document};
use iris::{FieldOption, LexicalSearchRequest, Schema};

#[tokio::test(flavor = "multi_thread")]
async fn test_wal_recovery_uncommitted() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Config
    let vector_opt = VectorOption::default();
    let lexical_opt = LexicalOption::default();

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(lexical_opt))
        .add_field("embedding", FieldOption::Vector(vector_opt))
        .build();

    // 3. Round 1: Index but DO NOT commit
    {
        let engine = Engine::new(storage.clone(), config.clone()).await?;

        // Initial state
        let query = Box::new(TermQuery::new("title", "rust"));
        let search_request = iris::SearchRequestBuilder::new()
            .lexical_search_request(LexicalSearchRequest::new(query))
            .build();
        let search_results = engine.search(search_request).await?;
        assert_eq!(search_results.len(), 0);

        let doc1 = Document::builder()
            .add_field("title", DataValue::Text("Rust Programming".into()))
            .add_field("embedding", DataValue::Vector(vec![0.1; 128]))
            .build();

        engine.put_document("doc1", doc1).await?;

        // Drop engine WITHOUT commit
    }

    // 4. Round 2: Recover from WAL
    {
        // Re-open engine on SAME storage
        let engine = Engine::new(storage.clone(), config.clone()).await?;

        // Commit to ensure flushed to searchable index
        engine.commit().await?;

        // Should have recovered doc1 from WAL and now committed
        let query = Box::new(TermQuery::new("title", "rust"));
        let search_request = iris::SearchRequestBuilder::new()
            .lexical_search_request(LexicalSearchRequest::new(query))
            .build();
        let search_results = engine.search(search_request).await?;
        assert_eq!(
            search_results.len(),
            1,
            "Document should be recovered from WAL"
        );
    }

    Ok(())
}
