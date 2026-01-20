# Basic Usage

This guide demonstrates how to perform a **Hybrid Search** using the unified `VectorEngine`.

```rust
use std::sync::Arc;
use iris::vector::engine::VectorEngine;
use iris::vector::engine::config::VectorEngineConfig;
use iris::vector::index::config::HnswIndexConfig;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::vector::Vector;
use iris::lexical::core::document::Document as LexicalDocument;
use iris::lexical::core::field::TextOption;
use iris::vector::engine::query::{VectorSearchRequest, HybridSearchQuery};

#[tokio::main]
async fn main() -> iris::error::Result<()> {
    // 1. Setup Storage
    let storage_path = "tmp/iris_index";
    std::fs::create_dir_all(storage_path)?;
    let storage_config = StorageConfig::File(FileStorageConfig::new(storage_path));
    let storage = StorageFactory::create(storage_config.clone())?;

    // 2. Setup Vector Engine
    // The VectorEngine now handles both vector and lexical indexing internally.
    let vector_config = VectorEngineConfig {
        index_config: HnswIndexConfig {
             dimension: 3, // Example dimension
             ..Default::default() 
        },
        ..Default::default()
    };
    let mut engine = VectorEngine::new(storage, vector_config).await?;

    // 3. Index a Document
    let mut lex_doc = LexicalDocument::new();
    lex_doc.add_text("title", "Rust Programming", TextOption::default());
    
    let vector = Vector::new(vec![0.1, 0.2, 0.3]);
    let doc_id = "doc_1";

    // Insert document with both vector and lexical data
    engine.insert(doc_id, vector, Some(lex_doc)).await?;

    // 4. Search
    let query_vector = vec![0.1, 0.2, 0.3].into();
    
    let request = VectorSearchRequest::builder()
        .vector(query_vector)
        .hybrid_query(HybridSearchQuery {
            keyword_query: "Rust".to_string(),
            keyword_weight: 0.5,
            vector_weight: 0.5,
            top_k: 10,
        })
        .build();

    let results = engine.search(request).await?;
    
    println!("Found {} hits", results.hits.len());
    for hit in results.hits {
        println!("Hit: {}, Score: {}", hit.id, hit.score);
    }

    Ok(())
}
```
