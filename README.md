# Iris : Information Retrieval with Semantics

[![Crates.io](https://img.shields.io/crates/v/iris.svg)](https://crates.io/crates/iris)
[![Documentation](https://docs.rs/iris/badge.svg)](https://docs.rs/iris)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Iris is a search core library written in Rust, designed for **Information Retrieval with Semantics**.

Iris provides the foundational mechanisms **essential for** advanced search capabilities:

- **Lexical search primitives** for precise, exact-match retrieval
- **Vector-based similarity search** for deep semantic understanding
- **Hybrid scoring and ranking** to synthesize multiple signals into coherent results

Rather than functioning as a monolithic search engine, Iris is architected as a **composable search core** — a suite of modular building blocks designed to be embedded into applications, extended with custom logic, or orchestrated within distributed systems.

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory and online at [https://mosuka.github.io/iris/](https://mosuka.github.io/iris/):

- [**Getting Started**](https://mosuka.github.io/iris/getting_started/index.html): Installation and basic usage.
- [**Core Concepts**](https://mosuka.github.io/iris/concepts/index.html): Architecture, Lexical Search, and Vector Search.
- [**Advanced Features**](https://mosuka.github.io/iris/advanced/index.html): ID Management, Persistence, and Deletions.
- [**API Reference**](https://docs.rs/iris)

## Features

- **Pure Rust Implementation**: Memory-safe and fast performance with zero-cost abstractions.
- **Hybrid Search**: Seamlessly combine BM25 lexical search with HNSW vector search using configurable fusion strategies.
- **Multimodal capabilities**: Native support for text-to-image and image-to-image search via CLIP embeddings.
- **Rich Query DSL**: Term, phrase, boolean, fuzzy, wildcard, range, and geographic queries.
- **Flexible Analysis**: Configurable pipelines for tokenization, normalization, and stemming (including CJK support).
- **Pluggable Storage**: Interfaces for in-memory, file-system, and memory-mapped storage backends.

## Quick Start

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

    // 5. Display results with document content
    for hit in results {
        if let Ok(Some(doc)) = engine.get_document(hit.doc_id) {
            let id = doc.id().unwrap_or("unknown");
            let content = doc.fields.get("content").and_then(|v| v.as_text()).unwrap_or("");
            println!("[{}] {} (internal_id={}, score={:.4})", id, content, hit.doc_id, hit.score);
        }
    }

    Ok(())
}
```

## Examples

You can find usage examples in the [`examples/`](examples/) directory:

### Search

- [Unified Search](examples/search.rs) - Lexical, Vector, and Hybrid search in one cohesive example
- [Multimodal Search](examples/multimodal_search.rs) - Text-to-image and image-to-image search

### Query Types

- [Term Query](examples/term_query.rs) - Basic keyword search
- [Boolean Query](examples/boolean_query.rs) - Complex boolean expressions (AND, OR, NOT)
- [Phrase Query](examples/phrase_query.rs) - Exact phrase matching
- [Fuzzy Query](examples/fuzzy_query.rs) - Approximate string matching
- [Wildcard Query](examples/wildcard_query.rs) - Pattern-based search
- [Range Query](examples/range_query.rs) - Numeric and date range queries
- [Geo Query](examples/geo_query.rs) - Geographic search
- [Span Query](examples/span_query.rs) - Positional queries

### Embeddings

- [Candle Embedder](examples/embedding_with_candle.rs) - Local BERT embeddings
- [OpenAI Embedder](examples/embedding_with_openai.rs) - Cloud-based embeddings

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under either of

- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
