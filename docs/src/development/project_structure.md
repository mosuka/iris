# Project Structure

Laurus is organized as a Cargo workspace with three crates.

## Workspace Layout

```text
laurus/                          # Repository root
├── Cargo.toml                   # Workspace definition
├── laurus/                      # Core search engine library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs               # Public API and module declarations
│   │   ├── engine.rs            # Engine, EngineBuilder, SearchRequest
│   │   ├── analysis/            # Text analysis pipeline
│   │   ├── lexical/             # Inverted index and lexical search
│   │   ├── vector/              # Vector indexes (Flat, HNSW, IVF)
│   │   ├── embedding/           # Embedder implementations
│   │   ├── storage/             # Storage backends (memory, file, mmap)
│   │   ├── store/               # Document log (WAL)
│   │   ├── spelling/            # Spelling correction
│   │   ├── data/                # DataValue, Document types
│   │   └── error.rs             # LaurusError type
│   └── examples/                # Runnable examples
├── laurus-cli/                  # Command-line interface
│   ├── Cargo.toml
│   └── src/
│       └── main.rs              # CLI entry point (clap)
├── laurus-server/               # gRPC server + HTTP gateway
│   ├── Cargo.toml
│   ├── proto/                   # Protobuf service definitions
│   └── src/
│       ├── lib.rs               # Server library
│       ├── config.rs            # TOML configuration
│       ├── grpc/                # gRPC service implementations
│       └── gateway/             # HTTP/JSON gateway (axum)
└── docs/                        # mdBook documentation
    ├── book.toml
    └── src/
        └── SUMMARY.md           # Table of contents
```

## Crate Responsibilities

| Crate | Type | Description |
| :--- | :--- | :--- |
| `laurus` | Library | Core search engine with lexical, vector, and hybrid search |
| `laurus-cli` | Binary | CLI tool for index management, document CRUD, search, and REPL |
| `laurus-server` | Library + Binary | gRPC server with optional HTTP/JSON gateway |

Both `laurus-cli` and `laurus-server` depend on the `laurus` library crate.

## Design Conventions

- **Module style**: File-based modules (Rust 2018 edition style), not `mod.rs`
  - `src/tokenizer.rs` + `src/tokenizer/dictionary.rs`
  - Not: `src/tokenizer/mod.rs`
- **Error handling**: `thiserror` for library error types, `anyhow` only in binary crates
- **No `unwrap()` / `expect()`** in production code (allowed in tests)
- **Async**: All public APIs use async/await with Tokio runtime
- **Unsafe**: Every `unsafe` block must have a `// SAFETY: ...` comment
- **Documentation**: All public types, functions, and enums must have doc comments (`///`)
- **Licensing**: Dependencies must be MIT or Apache-2.0 compatible
