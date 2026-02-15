# Advanced Features

This section covers advanced topics for users who want to go deeper into Iris's capabilities.

## Topics

### [Query DSL](advanced/query_dsl.md)

A human-readable query language for lexical, vector, and hybrid search. Supports boolean operators, phrase matching, fuzzy search, range queries, and more — all in a single query string.

### [ID Management](advanced/id_management.md)

How Iris manages document identity with a dual-tiered ID system:

- External IDs (user-provided strings)
- Internal IDs (shard-prefixed `u64` for performance)

### [Persistence & WAL](advanced/persistence.md)

How Iris ensures data durability through Write-Ahead Logging (WAL) and the commit lifecycle.

### [Deletions & Compaction](advanced/deletions.md)

How documents are deleted (logical deletion via bitmaps) and how space is reclaimed (compaction).
