# Advanced Features

This section covers advanced topics for users who want to go deeper into Laurus's capabilities.

## Topics

### [Query DSL](advanced/query_dsl.md)

A human-readable query language for lexical, vector, and hybrid search. Supports boolean operators, phrase matching, fuzzy search, range queries, and more — all in a single query string.

### [ID Management](advanced/id_management.md)

How Laurus manages document identity with a dual-tiered ID system:

- External IDs (user-provided strings)
- Internal IDs (shard-prefixed `u64` for performance)

### [Persistence & WAL](advanced/persistence.md)

How Laurus ensures data durability through Write-Ahead Logging (WAL) and the commit lifecycle.

### [Deletions & Compaction](advanced/deletions.md)

How documents are deleted (logical deletion via bitmaps) and how space is reclaimed (compaction).

### [Error Handling](advanced/error_handling.md)

Understanding `LaurusError` and `Result<T>` for robust application development. Covers all error variants, matching patterns, and common error scenarios.

### [Extensibility](advanced/extensibility.md)

Implementing custom components by extending Laurus's trait-based abstractions:

- Custom `Analyzer` for text analysis
- Custom `Embedder` for vector embeddings
- Custom `Storage` for new backends
