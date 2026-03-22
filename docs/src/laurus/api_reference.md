# API Reference

This page provides a quick reference of the most important types and methods in Laurus. For full details, generate the Rustdoc:

```bash
cargo doc --open
```

## Engine

The central coordinator for all indexing and search operations.

| Method | Description |
| :--- | :--- |
| `Engine::builder(storage, schema)` | Create an `EngineBuilder` |
| `engine.put_document(id, doc).await?` | Upsert a document (replace if ID exists) |
| `engine.add_document(id, doc).await?` | Add a document as a chunk (multiple chunks can share an ID) |
| `engine.delete_documents(id).await?` | Delete all documents/chunks by external ID |
| `engine.get_documents(id).await?` | Get all documents/chunks by external ID |
| `engine.search(request).await?` | Execute a search request |
| `engine.commit().await?` | Flush all pending changes to storage |
| `engine.add_field(name, field_option).await?` | Dynamically add a new field to the schema at runtime |
| `engine.delete_field(name).await?` | Remove a field from the schema at runtime |
| `engine.schema()` | Return the current `Schema` |
| `engine.stats()?` | Get index statistics |

> **`put_document` vs `add_document`:** `put_document` performs an upsert — if a document with the same external ID already exists, it is deleted and replaced. `add_document` always appends, allowing multiple document chunks to share the same external ID. See [Schema & Fields — Indexing Documents](../concepts/schema_and_fields.md#indexing-documents) for details.

### EngineBuilder

| Method | Description |
| :--- | :--- |
| `EngineBuilder::new(storage, schema)` | Create a builder with storage and schema |
| `.analyzer(Arc<dyn Analyzer>)` | Set the text analyzer (default: `StandardAnalyzer`) |
| `.embedder(Arc<dyn Embedder>)` | Set the vector embedder (optional) |
| `.build().await?` | Build the `Engine` |

## Schema

Defines document structure.

| Method | Description |
| :--- | :--- |
| `Schema::builder()` | Create a `SchemaBuilder` |

### SchemaBuilder

| Method | Description |
| :--- | :--- |
| `.add_text_field(name, TextOption)` | Add a full-text field |
| `.add_integer_field(name, IntegerOption)` | Add an integer field |
| `.add_float_field(name, FloatOption)` | Add a float field |
| `.add_boolean_field(name, BooleanOption)` | Add a boolean field |
| `.add_datetime_field(name, DateTimeOption)` | Add a datetime field |
| `.add_geo_field(name, GeoOption)` | Add a geographic field |
| `.add_bytes_field(name, BytesOption)` | Add a binary field |
| `.add_hnsw_field(name, HnswOption)` | Add an HNSW vector field |
| `.add_flat_field(name, FlatOption)` | Add a Flat vector field |
| `.add_ivf_field(name, IvfOption)` | Add an IVF vector field |
| `.add_default_field(name)` | Set a default search field |
| `.build()` | Build the `Schema` |

## Document

A collection of named field values.

| Method | Description |
| :--- | :--- |
| `Document::builder()` | Create a `DocumentBuilder` |
| `doc.get(name)` | Get a field value by name |
| `doc.has_field(name)` | Check if a field exists |
| `doc.field_names()` | Get all field names |

### DocumentBuilder

| Method | Description |
| :--- | :--- |
| `.add_text(name, value)` | Add a text field |
| `.add_integer(name, value)` | Add an integer field |
| `.add_float(name, value)` | Add a float field |
| `.add_boolean(name, value)` | Add a boolean field |
| `.add_datetime(name, value)` | Add a datetime field |
| `.add_vector(name, vec)` | Add a pre-computed vector |
| `.add_geo(name, lat, lon)` | Add a geographic point |
| `.add_bytes(name, data)` | Add binary data |
| `.build()` | Build the `Document` |

## Search

### SearchRequestBuilder

| Method | Description |
| :--- | :--- |
| `SearchRequestBuilder::new()` | Create a new builder |
| `.query_dsl(dsl)` | Set a unified DSL string (parsed at search time) |
| `.lexical_query(query)` | Set the lexical search query (`LexicalSearchQuery`) |
| `.vector_query(query)` | Set the vector search query (`VectorSearchQuery`) |
| `.filter_query(query)` | Set a pre-filter query |
| `.fusion_algorithm(algo)` | Set the fusion algorithm (default: RRF) |
| `.limit(n)` | Maximum results (default: 10) |
| `.offset(n)` | Skip N results (default: 0) |
| `.add_field_boost(field, boost)` | Add a field-level boost for lexical search |
| `.lexical_min_score(f32)` | Set minimum score threshold for lexical search |
| `.lexical_timeout_ms(u64)` | Set lexical search timeout in milliseconds |
| `.lexical_parallel(bool)` | Enable parallel lexical search |
| `.sort_by(SortField)` | Set sort order for lexical search results |
| `.vector_score_mode(VectorScoreMode)` | Set score combination mode for vector search |
| `.vector_min_score(f32)` | Set minimum score threshold for vector search |
| `.build()` | Build the `SearchRequest` |

### LexicalSearchQuery

| Variant | Description |
| :--- | :--- |
| `LexicalSearchQuery::Dsl(String)` | Query specified as a DSL string (parsed at search time) |
| `LexicalSearchQuery::Obj(Box<dyn Query>)` | Query specified as a pre-built Query object |

### VectorSearchQuery

| Variant | Description |
| :--- | :--- |
| `VectorSearchQuery::Payloads(Vec<QueryPayload>)` | Raw payloads (text, bytes, etc.) to be embedded at search time |
| `VectorSearchQuery::Vectors(Vec<QueryVector>)` | Pre-embedded query vectors ready for nearest-neighbor search |

### SearchResult

| Field | Type | Description |
| :--- | :--- | :--- |
| `id` | `String` | External document ID |
| `score` | `f32` | Relevance score |
| `document` | `Option<Document>` | Document content (if loaded) |

### FusionAlgorithm

| Variant | Description |
| :--- | :--- |
| `RRF { k: f64 }` | Reciprocal Rank Fusion (default k=60.0) |
| `WeightedSum { lexical_weight, vector_weight }` | Linear combination of scores |

## Query Types (Lexical)

| Query | Description | Example |
| :--- | :--- | :--- |
| `TermQuery::new(field, term)` | Exact term match | `TermQuery::new("body", "rust")` |
| `PhraseQuery::new(field, terms)` | Exact phrase | `PhraseQuery::new("body", vec!["machine".into(), "learning".into()])` |
| `BooleanQueryBuilder::new()` | Boolean combination | `.must(q1).should(q2).must_not(q3).build()` |
| `FuzzyQuery::new(field, term)` | Fuzzy match (default max_edits=2) | `FuzzyQuery::new("body", "programing").max_edits(1)` |
| `WildcardQuery::new(field, pattern)` | Wildcard | `WildcardQuery::new("file", "*.pdf")` |
| `NumericRangeQuery::new(...)` | Numeric range | See [Lexical Search](../concepts/search.md) |
| `GeoQuery::within_radius(...)` | Geo radius | See [Lexical Search](../concepts/search.md) |
| `SpanNearQuery::new(...)` | Proximity | See [Lexical Search](../concepts/search.md) |
| `PrefixQuery::new(field, prefix)` | Prefix match | `PrefixQuery::new("body", "pro")` |
| `RegexpQuery::new(field, pattern)?` | Regex match | `RegexpQuery::new("body", "^pro.*ing$")?` |

## Query Parsers

| Parser | Description |
| :--- | :--- |
| `QueryParser::new(analyzer)` | Parse lexical DSL queries |
| `VectorQueryParser::new(embedder)` | Parse vector DSL queries |
| `UnifiedQueryParser::new(lexical, vector)` | Parse hybrid DSL queries |

## Analyzers

| Type | Description |
| :--- | :--- |
| `StandardAnalyzer` | RegexTokenizer + lowercase + stop words |
| `SimpleAnalyzer` | Tokenization only (no filtering) |
| `EnglishAnalyzer` | RegexTokenizer + lowercase + English stop words |
| `JapaneseAnalyzer` | Japanese morphological analysis |
| `KeywordAnalyzer` | No tokenization (exact match) |
| `PipelineAnalyzer` | Custom tokenizer + filter chain |
| `PerFieldAnalyzer` | Per-field analyzer dispatch |

## Embedders

| Type | Feature Flag | Description |
| :--- | :--- | :--- |
| `CandleBertEmbedder` | `embeddings-candle` | Local BERT model |
| `OpenAIEmbedder` | `embeddings-openai` | OpenAI API |
| `CandleClipEmbedder` | `embeddings-multimodal` | Local CLIP model |
| `PrecomputedEmbedder` | *(default)* | Pre-computed vectors |
| `PerFieldEmbedder` | *(default)* | Per-field embedder dispatch |

## Storage

| Type | Description |
| :--- | :--- |
| `MemoryStorage` | In-memory (non-durable) |
| `FileStorage` | File-system based (supports `use_mmap` for memory-mapped I/O) |
| `StorageFactory::create(config)` | Create from config |

## DataValue

| Variant | Rust Type |
| :--- | :--- |
| `DataValue::Null` | — |
| `DataValue::Bool(bool)` | `bool` |
| `DataValue::Int64(i64)` | `i64` |
| `DataValue::Float64(f64)` | `f64` |
| `DataValue::Text(String)` | `String` |
| `DataValue::Bytes(Vec<u8>, Option<String>)` | `(data, mime_type)` |
| `DataValue::Vector(Vector)` | `Vector` |
| `DataValue::DateTime(DateTime<Utc>)` | `chrono::DateTime<Utc>` |
| `DataValue::Geo(f64, f64)` | `(latitude, longitude)` |
