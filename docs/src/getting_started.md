# Getting Started

Welcome to the Iris getting started guide. This section is designed to try out Iris quickly.

## Workflow Overview

Building a search application with Iris typically involves the following steps:

1. **Installation**: Adding `iris` to your project dependencies.
2. **Configuration**: Setting up the `Engine` with `Schema` and choosing a storage backend (Memory, File, or Mmap).
3. **Indexing**: Inserting documents that contain both text (for lexical search) and vectors (for semantic search).
4. **Searching**: Executing queries to retrieve relevant results.

## In this Section

* **[Installation](./getting_started/installation.md)**
Learn how to add Iris to your Rust project and configure necessary feature flags (e.g., for different tokenizer support).

## Quick Example

For a complete, runnable example of how to set up a Hybrid Search (combining vector and text search), please refer to the **[Unified Search Example](https://github.com/mosuka/iris/blob/main/examples/search.rs)** in the repository.

```rust
use iris::{Document, Engine, FieldOption, FusionAlgorithm, Schema, SearchRequestBuilder};
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::{FieldOption as LexicalFieldOption, TextOption, TermQuery};
use iris::vector::{FlatOption, VectorOption, VectorSearchRequestBuilder};
use iris::storage::{StorageConfig, StorageFactory};
use iris::storage::memory::MemoryStorageConfig;
use std::sync::Arc;

fn main() -> iris::Result<()> {
    // 1. Create storage
    let storage = StorageFactory::create(StorageConfig::Memory(MemoryStorageConfig::default()))?;

    // 2. Define schema with separate lexical and vector fields
    let schema = Schema::builder()
        .add_field("content", FieldOption::Lexical(LexicalFieldOption::Text(TextOption::default())))
        .add_field("content_vec", FieldOption::Vector(VectorOption::Flat(FlatOption { dimension: 384, ..Default::default() })))
        .build();

    // 3. Create engine with analyzer and embedder
    let engine = Engine::builder(storage, schema)
        .analyzer(Arc::new(StandardAnalyzer::default()))
        .embedder(Arc::new(MyEmbedder))  // Your embedder implementation
        .build()?;

    engine.index(
        Document::new_with_id("doc1")
            .add_text("content", "Rust is a systems programming language")
            .add_text("content_vec", "Rust is a systems programming language")
    )?;
    engine.index(
        Document::new_with_id("doc2")
            .add_text("content", "Python is great for machine learning")
            .add_text("content_vec", "Python is great for machine learning")
    )?;
    engine.commit()?;

    // 4. Hybrid search (combines lexical keyword match + semantic similarity)
    let results = engine.search(
        SearchRequestBuilder::new()
            .with_lexical(Box::new(TermQuery::new("content", "programming")))
            .with_vector(VectorSearchRequestBuilder::new().add_text("content_vec", "systems language").build())
            .fusion(FusionAlgorithm::RRF { k: 60.0 })
            .build()
    )?;

    // 5. Display results
    for hit in results {
        if let Ok(Some(doc)) = engine.get_document(hit.doc_id) {
            let id = doc.id().unwrap_or("unknown");
            println!("[{}] score={:.4}", id, hit.score);
        }
    }

    Ok(())
}
```
