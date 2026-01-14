# Basic Usage

This guide demonstrates how to perform a **Hybrid Search**, which combines keywords and vectors.

```rust
use std::sync::Arc;
use sarissa::hybrid::engine::HybridEngine;
use sarissa::hybrid::search::searcher::{HybridSearchRequest, HybridSearchParams};
use sarissa::hybrid::core::document::HybridDocument;
use sarissa::lexical::core::document::Document as LexicalDocument;
use sarissa::lexical::core::field::{Field, FieldValue, TextOption};
use sarissa::lexical::engine::LexicalEngine;
use sarissa::lexical::engine::config::LexicalIndexConfig;
use sarissa::lexical::index::config::InvertedIndexConfig;
use sarissa::vector::engine::VectorEngine;
use sarissa::vector::engine::config::VectorEngineConfig;
use sarissa::vector::index::config::HnswIndexConfig;
use sarissa::storage::file::FileStorageConfig;
use sarissa::storage::{StorageConfig, StorageFactory};
use sarissa::analysis::analyzer::standard::StandardAnalyzer;
use sarissa::analysis::analyzer::per_field::PerFieldAnalyzer;
use sarissa::vector::core::document::{DocumentPayload, Payload, PayloadSource};

#[tokio::main]
async fn main() -> sarissa::error::Result<()> {
    // 1. Setup Storage
    let storage_path = "tmp/sarissa_index";
    std::fs::create_dir_all(storage_path)?;
    let storage_config = StorageConfig::File(FileStorageConfig::new(storage_path));
    let storage = StorageFactory::create(storage_config.clone())?;

    // 2. Setup Analyzers (Lexical)
    let analyzer = Arc::new(PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new()?)));
    let lexical_config = LexicalIndexConfig::Inverted(InvertedIndexConfig {
        analyzer,
        ..Default::default()
    });
    let lexical_engine = LexicalEngine::new(storage.clone(), lexical_config)?;

    // 3. Setup Vector Engine (Semantic)
    let vector_config = VectorEngineConfig {
        index_config: HnswIndexConfig {
             dimension: 3, // Example dimension
             ..Default::default() 
        },
        ..Default::default()
    };
    let vector_engine = VectorEngine::new(storage.clone(), vector_config)?;

    // 4. Create Hybrid Engine
    let mut hybrid_engine = HybridEngine::new(storage, lexical_engine, vector_engine)?;

    // 5. Index a Document
    let mut lex_doc = LexicalDocument::new();
    lex_doc.add_text("title", "Rust Programming", TextOption::default());
    
    let mut vec_payload = DocumentPayload::new();
    // Using pre-computed vector for simplicity
    vec_payload.set_field("vector_field", Payload::vector(Arc::new([0.1, 0.2, 0.3])));

    let doc = HybridDocument {
        lexical_doc: Some(lex_doc),
        vector_payload: Some(vec_payload),
    };

    let doc_id = hybrid_engine.index_document("unique_id_1", doc).await?;
    hybrid_engine.commit()?;

    // 6. Search
    let params = HybridSearchParams {
        keyword_weight: 0.5,
        vector_weight: 0.5,
        top_k: 10,
        ..Default::default()
    };

    let request = HybridSearchRequest::new()
        .with_text("Rust") // Lexical Query
        .with_vector("vector_field", &[0.1, 0.2, 0.3]) // Vector Query
        .with_params(params);

    let results = hybrid_engine.search(request).await?;
    
    println!("Found {} hits", results.results.len());
    Ok(())
}
```
