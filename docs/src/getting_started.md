# Getting Started

Welcome to the Iris getting started guide. This section is designed to try out Iris quickly.

## Workflow Overview

Building a search application with Iris typically involves the following steps:

1. **Installation**: Adding `iris` to your project dependencies.
2. **Configuration**: Setting up the `VectorEngine` (or `LexicalEngine`) and choosing a storage backend (Memory, File, or Mmap).
3. **Indexing**: Inserting documents that contain both text (for lexical search) and vectors (for semantic search).
4. **Searching**: Executing queries to retrieve relevant results.

## In this Section

* **[Installation](./getting_started/installation.md)**
Learn how to add Iris to your Rust project and configure necessary feature flags (e.g., for different tokenizer support).

## Quick Example

For a complete, runnable example of how to set up a Hybrid Search (combining vector and text search), please refer to the **[Hybrid Search Example](https://github.com/mosuka/iris/blob/main/examples/hybrid_search.rs)** in the repository.

```rust
// Pseudo-code of a typical setup
let storage = Arc::new(MemoryStorage::default());
let config = VectorIndexConfig::builder()
    .add_field("embedding", 768)?
    .build()?;
let engine = VectorEngine::new(storage, config)?;

// Indexing...
engine.upsert_vectors(doc_id, document)?;

// Searching...
let results = engine.search(request).await?;
```
