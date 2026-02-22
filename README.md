# Laurus : Lexical Augmented Unified Retrieval Using Semantics

[![Crates.io](https://img.shields.io/crates/v/laurus.svg)](https://crates.io/crates/laurus)
[![Documentation](https://docs.rs/laurus/badge.svg)](https://docs.rs/laurus)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Laurus is a composable search core library written in Rust — built for Lexical Augmented Unified Retrieval Using Semantics.
Rather than a monolithic engine, Laurus provides modular building blocks for embedding powerful search into any application:

- **Lexical search primitives** for precise, exact-match retrieval
- **Vector-based similarity search** for deep semantic understanding
- **Hybrid scoring and ranking** to synthesize multiple signals into coherent results

Rather than functioning as a monolithic search engine, Laurus is architected as a **composable search core** — a suite of modular building blocks designed to be embedded into applications, extended with custom logic, or orchestrated within distributed systems.

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory and online at [https://mosuka.github.io/laurus/](https://mosuka.github.io/laurus/):

- [**Getting Started**](https://mosuka.github.io/laurus/getting_started.html): Installation and basic usage.
- [**Architecture**](https://mosuka.github.io/laurus/architecture.html): System architecture overview.
- [**Core Concepts**](https://mosuka.github.io/laurus/concepts.html): Schema, Analysis, Embeddings, and Storage.
- [**Indexing**](https://mosuka.github.io/laurus/indexing.html): Lexical and Vector indexing.
- [**Search**](https://mosuka.github.io/laurus/search.html): Lexical, Vector, and Hybrid search.
- [**Advanced Features**](https://mosuka.github.io/laurus/advanced.html): Query DSL, ID Management, Persistence, and Deletions.
- [**API Reference**](https://docs.rs/laurus)

## Features

- **Pure Rust Implementation**: Memory-safe and fast performance with zero-cost abstractions.
- **Hybrid Search**: Seamlessly combine BM25 lexical search with HNSW vector search using configurable fusion strategies.
- **Multimodal capabilities**: Native support for text-to-image and image-to-image search via CLIP embeddings.
- **Rich Query DSL**: Term, phrase, boolean, fuzzy, wildcard, range, and geographic queries.
- **Flexible Analysis**: Configurable pipelines for tokenization, normalization, and stemming (including CJK support).
- **Pluggable Storage**: Interfaces for in-memory, file-system, and memory-mapped storage backends.

## Quick Start

```rust
use laurus::lexical::{TermQuery, TextOption};
use laurus::{Document, Engine, LexicalSearchRequest, Schema, SearchRequestBuilder};
use laurus::storage::{StorageConfig, StorageFactory};
use laurus::storage::memory::MemoryStorageConfig;

#[tokio::main]
async fn main() -> laurus::Result<()> {
    // 1. Create storage
    let storage = StorageFactory::create(StorageConfig::Memory(MemoryStorageConfig::default()))?;

    // 2. Define schema
    let schema = Schema::builder()
        .add_text_field("title", TextOption::default())
        .add_text_field("body", TextOption::default())
        .build();

    // 3. Create engine
    let engine = Engine::new(storage, schema).await?;

    // 4. Index documents
    engine
        .add_document(
            "doc1",
            Document::builder()
                .add_text("title", "Introduction to Rust")
                .add_text("body", "Rust is a systems programming language focused on safety.")
                .build(),
        )
        .await?;
    engine
        .add_document(
            "doc2",
            Document::builder()
                .add_text("title", "Python for Data Science")
                .add_text("body", "Python is widely used in data science and machine learning.")
                .build(),
        )
        .await?;
    engine.commit().await?;

    // 5. Search
    let results = engine
        .search(
            SearchRequestBuilder::new()
                .lexical_search_request(LexicalSearchRequest::new(Box::new(TermQuery::new(
                    "body", "rust",
                ))))
                .build(),
        )
        .await?;

    for hit in &results {
        println!("[{}] score={:.4}", hit.id, hit.score);
    }

    Ok(())
}
```

## Examples

You can find usage examples in the [`laurus/examples/`](laurus/examples/) directory:

- [Quickstart](laurus/examples/quickstart.rs) - Basic full-text search
- [Lexical Search](laurus/examples/lexical_search.rs) - All query types (Term, Phrase, Boolean, Fuzzy, Wildcard, Range, Geo, Span)
- [Vector Search](laurus/examples/vector_search.rs) - Semantic similarity search with embeddings
- [Hybrid Search](laurus/examples/hybrid_search.rs) - Combining lexical and vector search with fusion
- [Multimodal Search](laurus/examples/multimodal_search.rs) - Text-to-image and image-to-image search
- [Synonym Graph Filter](laurus/examples/synonym_graph_filter.rs) - Synonym expansion in analysis pipeline
- [Candle Embedder](laurus/examples/search_with_candle.rs) - Local BERT embeddings
- [OpenAI Embedder](laurus/examples/search_with_openai.rs) - Cloud-based embeddings

## Contributing

We welcome contributions!

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
