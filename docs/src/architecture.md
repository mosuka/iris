# Architecture

This page explains how Iris is structured internally. Understanding the architecture will help you make better decisions about schema design, analyzer selection, and search strategies.

## High-Level Overview

Iris is organized around a single `Engine` that coordinates four internal components:

<div class="mermaid">
graph TB
    subgraph Engine
        SCH["Schema"]
        LS["LexicalStore<br/>(Inverted Index)"]
        VS["VectorStore<br/>(HNSW / Flat / IVF)"]
        DL["DocumentLog<br/>(WAL + Document Storage)"]
    end

    Storage["Storage (trait)<br/>Memory / File / Mmap"]

    LS --- Storage
    VS --- Storage
    DL --- Storage
</div>

| Component | Responsibility |
| :--- | :--- |
| **Schema** | Declares fields and their types; determines how each field is routed |
| **LexicalStore** | Inverted index for keyword search (BM25 scoring) |
| **VectorStore** | Vector index for similarity search (Flat, HNSW, or IVF) |
| **DocumentLog** | Write-ahead log (WAL) for durability + raw document storage |

All three stores share a single `Storage` backend, isolated by key prefixes (`lexical/`, `vector/`, `documents/`).

## Engine Lifecycle

### Building an Engine

The `EngineBuilder` assembles the engine from its parts:

```rust
let engine = Engine::builder(storage, schema)
    .analyzer(analyzer)      // optional: for text fields
    .embedder(embedder)      // optional: for vector fields
    .build()
    .await?;
```

<div class="mermaid">
sequenceDiagram
    participant User
    participant EngineBuilder
    participant Engine

    User->>EngineBuilder: new(storage, schema)
    User->>EngineBuilder: .analyzer(analyzer)
    User->>EngineBuilder: .embedder(embedder)
    User->>EngineBuilder: .build().await
    EngineBuilder->>EngineBuilder: split_schema()
    Note over EngineBuilder: Separate fields into<br/>LexicalIndexConfig<br/>+ VectorIndexConfig
    EngineBuilder->>Engine: Create LexicalStore
    EngineBuilder->>Engine: Create VectorStore
    EngineBuilder->>Engine: Create DocumentLog
    EngineBuilder->>Engine: Recover from WAL
    EngineBuilder-->>User: Engine ready
</div>

During `build()`, the engine:

1. **Splits the schema** — lexical fields go to `LexicalIndexConfig`, vector fields go to `VectorIndexConfig`
2. **Creates prefixed storage** — each component gets an isolated namespace (`lexical/`, `vector/`, `documents/`)
3. **Initializes stores** — `LexicalStore` and `VectorStore` are constructed with their configs
4. **Recovers from WAL** — replays any uncommitted operations from a previous session

### Schema Splitting

The `Schema` contains both lexical and vector fields. At build time, `split_schema()` separates them:

<div class="mermaid">
graph LR
    S["Schema<br/>title: Text<br/>body: Text<br/>category: Text<br/>page: Integer<br/>content_vec: HNSW"]

    S --> LC["LexicalIndexConfig<br/>title: TextOption<br/>body: TextOption<br/>category: TextOption<br/>page: IntegerOption<br/>_id: KeywordAnalyzer"]

    S --> VC["VectorIndexConfig<br/>content_vec: HnswOption<br/>(dim=384, m=16, ef=200)"]
</div>

Key details:

- The reserved `_id` field is always added to the lexical config with `KeywordAnalyzer` (exact match)
- A `PerFieldAnalyzer` wraps per-field analyzer settings; if you pass a simple `StandardAnalyzer`, it becomes the default for all text fields
- A `PerFieldEmbedder` works the same way for vector fields

## Indexing Data Flow

When you call `engine.add_document(id, doc)`:

<div class="mermaid">
sequenceDiagram
    participant User
    participant Engine
    participant WAL as DocumentLog (WAL)
    participant Lexical as LexicalStore
    participant Vector as VectorStore

    User->>Engine: add_document("doc-1", doc)
    Engine->>WAL: Append to WAL
    Engine->>Engine: Assign internal ID (u64)

    loop For each field in document
        alt Lexical field (text, integer, etc.)
            Engine->>Lexical: Analyze + index field
        else Vector field
            Engine->>Vector: Embed + index field
        end
    end

    Note over Engine: Document is buffered<br/>but NOT yet searchable

    User->>Engine: commit()
    Engine->>Lexical: Flush segments to storage
    Engine->>Vector: Flush segments to storage
    Engine->>WAL: Mark checkpoint
    Note over Engine: Documents are<br/>now searchable
</div>

Key points:

- **WAL-first**: every write is logged before modifying in-memory structures
- **Dual indexing**: each field is routed to either the lexical or vector store based on the schema
- **Commit required**: documents become searchable only after `commit()`

## Search Data Flow

When you call `engine.search(request)`:

<div class="mermaid">
sequenceDiagram
    participant User
    participant Engine
    participant Lexical as LexicalStore
    participant Vector as VectorStore
    participant Fusion

    User->>Engine: search(request)

    opt Filter query present
        Engine->>Lexical: Execute filter query
        Lexical-->>Engine: Allowed document IDs
    end

    par Lexical search
        Engine->>Lexical: Execute lexical query
        Lexical-->>Engine: Ranked hits (BM25)
    and Vector search
        Engine->>Vector: Execute vector query
        Vector-->>Engine: Ranked hits (similarity)
    end

    alt Both lexical and vector results
        Engine->>Fusion: Fuse results (RRF or WeightedSum)
        Fusion-->>Engine: Merged ranked list
    end

    Engine->>Engine: Apply offset + limit
    Engine-->>User: Vec of SearchResult
</div>

The search pipeline has three stages:

1. **Filter** (optional) — execute a filter query on the lexical index to get a set of allowed document IDs
2. **Search** — run lexical and/or vector queries in parallel
3. **Fusion** — if both query types are present, merge results using RRF (default, k=60) or WeightedSum

## Storage Architecture

All components share a single `Storage` trait implementation, but use key prefixes to isolate their data:

<div class="mermaid">
graph TB
    Engine --> PS1["PrefixedStorage<br/>prefix: 'lexical/'"]
    Engine --> PS2["PrefixedStorage<br/>prefix: 'vector/'"]
    Engine --> PS3["PrefixedStorage<br/>prefix: 'documents/'"]

    PS1 --> S["Storage Backend<br/>(Memory / File / Mmap)"]
    PS2 --> S
    PS3 --> S
</div>

| Backend | Description | Best For |
| :--- | :--- | :--- |
| `MemoryStorage` | All data in memory | Testing, small datasets, ephemeral use |
| `FileStorage` | Standard file I/O | General production use |
| `MmapStorage` | Memory-mapped files | Large datasets, read-heavy workloads |

## Per-Field Dispatch

When a `PerFieldAnalyzer` is provided, the engine dispatches analysis to field-specific analyzers. The same pattern applies to `PerFieldEmbedder`.

<div class="mermaid">
graph LR
    PFA["PerFieldAnalyzer"]
    PFA -->|"title"| KA["KeywordAnalyzer"]
    PFA -->|"body"| SA["StandardAnalyzer"]
    PFA -->|"description"| JA["JapaneseAnalyzer"]
    PFA -->|"_id"| KA2["KeywordAnalyzer<br/>(always)"]
    PFA -->|other fields| DEF["Default Analyzer<br/>(StandardAnalyzer)"]
</div>

This allows different fields to use different analysis strategies within the same engine.

## Summary

| Aspect | Detail |
| :--- | :--- |
| **Core struct** | `Engine` — coordinates all operations |
| **Builder** | `EngineBuilder` — assembles Engine from Storage + Schema + Analyzer + Embedder |
| **Schema split** | Lexical fields → `LexicalIndexConfig`, Vector fields → `VectorIndexConfig` |
| **Write path** | WAL → in-memory buffers → `commit()` → persistent storage |
| **Read path** | Query → parallel lexical/vector search → fusion → ranked results |
| **Storage isolation** | `PrefixedStorage` with `lexical/`, `vector/`, `documents/` prefixes |
| **Per-field dispatch** | `PerFieldAnalyzer` and `PerFieldEmbedder` route to field-specific implementations |

## Next Steps

- Understand field types and schema design: [Schema & Fields](concepts/schema_and_fields.md)
- Learn about text analysis: [Text Analysis](concepts/analysis.md)
- Learn about embeddings: [Embeddings](concepts/embedding.md)
