# IRiS

[![Crates.io](https://img.shields.io/crates/v/iris.svg)](https://crates.io/crates/iris)
[![Documentation](https://docs.rs/iris/badge.svg)](https://docs.rs/iris)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

IRiS is a **high-performance** search core library written in Rust, designed for **Information Retrieval with Semantics**.

IRiS provides the foundational mechanisms **essential for** advanced search capabilities:

- **Lexical search primitives** for precise, exact-match retrieval
- **Vector-based similarity search** for deep semantic understanding
- **Hybrid scoring and ranking** to synthesize multiple signals into coherent results

Rather than functioning as a monolithic search engine, IRiS is architected as a **composable search core** — a suite of modular building blocks designed to be embedded into applications, extended with custom logic, or orchestrated within distributed systems.

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

## Examples

You can find numerous usage examples in the [`examples/`](examples/) directory, covering:

- [Basic Lexical Search](examples/term_query.rs)
- [Vector Search & Embeddings](examples/vector_search.rs)
- [Hybrid Search](examples/hybrid_search.rs)
- [Multimodal Search](examples/multimodal_search.rs)

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
