# Introduction

Sarissa is a fast, featureful full-text search library for Rust, inspired by Whoosh.

## Features

- **Pure Rust**: Single binary, easy deployment, high performance.
- **Hybrid Search**: Seamlessly combine lexical (BM25) and semantic (Vector) search.
- **Flexible Analysis**: Configurable tokenizers and analyzers (Standard, Keyword, Lindera for Japanese).
- **Advanced Querying**: Boolean, Phrase, Range, Fuzzy, Wildcard, Geo, and Span queries.
- **Pluggable Storage**: Abstract storage interface supporting memory mapping and file-based storage.

## Philosophy

Sarissa aims to provide a "batteries-included" search experience that is easy to get started with but powerful enough for production use cases requiring customizability. It decouples the complexity of inverted indexes and HNSW graphs behind a clean, high-level API.
