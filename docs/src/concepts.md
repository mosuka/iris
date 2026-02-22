# Core Concepts

This section covers the foundational building blocks of Laurus. Understanding these concepts will help you design effective schemas and configure your search engine.

## Topics

### [Schema & Fields](concepts/schema_and_fields.md)

How to define the structure of your documents. Covers:

- `Schema` and `SchemaBuilder`
- Lexical field types (Text, Integer, Float, Boolean, DateTime, Geo, Bytes)
- Vector field types (Flat, HNSW, IVF)
- `Document` and `DocumentBuilder`
- `DataValue` — the unified value type

### [Text Analysis](concepts/analysis.md)

How text is processed before indexing. Covers:

- The `Analyzer` trait and the analysis pipeline
- Built-in analyzers (Standard, Japanese, Keyword, Pipeline)
- `PerFieldAnalyzer` — different analyzers for different fields
- Tokenizers and token filters

### [Embeddings](concepts/embedding.md)

How text and images are converted to vectors. Covers:

- The `Embedder` trait
- Built-in embedders (Candle BERT, OpenAI, CLIP, Precomputed)
- `PerFieldEmbedder` — different embedders for different fields

### [Storage](concepts/storage.md)

Where index data is stored. Covers:

- The `Storage` trait
- Storage backends (Memory, File, Mmap)
- `PrefixedStorage` for component isolation
- Choosing the right backend for your use case
