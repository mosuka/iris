# Architecture

This document describes the internal architecture of Laurus for contributors and developers who want to understand or modify the codebase.

For a user-facing overview, see the [mdBook Architecture page](docs/src/architecture.md).

## Source Code Structure

```
laurus/src/
├── lib.rs                    # Crate root, public API re-exports
├── error.rs                  # LaurusError enum (thiserror)
├── data.rs                   # Document, DataValue, DocumentBuilder
│
├── engine.rs                 # Unified Engine (facade pattern)
│   ├── schema.rs             # Schema, FieldOption, SchemaBuilder
│   ├── search.rs             # SearchRequest, FusionAlgorithm, SearchResult
│   └── query.rs              # UnifiedQueryParser (PEG-based)
│
├── analysis/                 # Text analysis pipeline
│   ├── analyzer/             # Analyzer trait + implementations
│   │   ├── analyzer.rs       # Core Analyzer trait
│   │   ├── standard.rs       # StandardAnalyzer
│   │   ├── japanese.rs       # JapaneseAnalyzer (Lindera)
│   │   ├── english.rs        # EnglishAnalyzer
│   │   ├── keyword.rs        # KeywordAnalyzer
│   │   ├── simple.rs         # SimpleAnalyzer
│   │   ├── pipeline.rs       # PipelineAnalyzer (custom chains)
│   │   └── per_field.rs      # PerFieldAnalyzer (field dispatch)
│   ├── tokenizer/            # Tokenizer implementations
│   ├── token_filter/         # Token filter implementations
│   ├── char_filter/          # Character filter implementations
│   ├── synonym/              # Synonym support
│   └── token.rs              # Token types
│
├── lexical/                  # Lexical (full-text) search
│   ├── core/                 # Core data structures (posting lists, etc.)
│   ├── index/                # Indexing logic (segments, merging)
│   ├── query/                # Query types (Term, Phrase, Boolean, etc.)
│   ├── search/               # Search execution and scoring (BM25)
│   ├── store/                # LexicalStore (facade for lexical operations)
│   ├── writer.rs             # Index writer
│   ├── reader.rs             # Index reader
│   └── document.rs           # Lexical document representation
│
├── vector/                   # Vector similarity search
│   ├── core/                 # Core types (Vector, distance functions)
│   ├── index/                # Vector index implementations
│   │   ├── flat/             # Flat (brute-force) index
│   │   ├── hnsw/             # HNSW index
│   │   └── ivf/              # IVF index
│   ├── query/                # Vector query types
│   ├── search/               # Vector search execution
│   ├── store/                # VectorStore (facade for vector operations)
│   ├── writer.rs             # Vector writer
│   └── reader.rs             # Vector reader
│
├── storage/                  # Storage abstraction layer
│   ├── file.rs               # FileStorage (disk-based, with mmap support)
│   ├── memory.rs             # MemoryStorage (in-memory)
│   ├── prefixed.rs           # PrefixedStorage (namespace isolation)
│   ├── column.rs             # Column-oriented storage
│   └── structured.rs         # Structured storage
│
├── embedding/                # Embedding providers
│   ├── embedder.rs           # Embedder trait
│   ├── per_field.rs          # PerFieldEmbedder
│   ├── precomputed.rs        # PrecomputedEmbedder
│   ├── candle_bert_embedder/ # Local BERT embeddings (feature-gated)
│   ├── candle_clip_embedder/ # Local CLIP embeddings (feature-gated)
│   └── openai_embedder/      # OpenAI API embeddings (feature-gated)
│
├── spelling/                 # Spelling correction
│   ├── corrector.rs          # SpellingCorrector, DidYouMean
│   ├── dictionary.rs         # SpellingDictionary, BuiltinDictionary
│   └── suggestion.rs         # SuggestionEngine
│
├── store/                    # Document storage (WAL, DocumentLog)
├── maintenance/              # Deletion, compaction, cleanup
└── util/                     # Utility functions
```

## Design Patterns

### Facade Pattern: Engine

`Engine` is the main entry point that coordinates all subsystems:

```
Engine
├── Schema          # Field definitions and routing
├── LexicalStore    # Full-text indexing and search
├── VectorStore     # Vector indexing and search
└── DocumentLog     # WAL and document storage
```

When a document is indexed, the Engine:

1. Injects the `_id` field
2. Writes to the WAL (Write-Ahead Log)
3. Stores the document for retrieval
4. Splits fields by type (lexical vs vector) based on the schema
5. Routes to `LexicalStore` and `VectorStore` respectively

### Builder Pattern

All major configuration objects use builders:

- `Schema::builder()` → `SchemaBuilder`
- `Document::builder()` → `DocumentBuilder`
- `Engine::builder(storage, schema)` → `EngineBuilder`
- `SearchRequest::builder()` → `SearchRequestBuilder`

### Trait-Based Abstraction

Core extension points are defined as traits:

| Trait | Purpose | Implementations |
| :--- | :--- | :--- |
| `Analyzer` | Text analysis pipeline | Standard, Japanese, English, Keyword, Simple, Pipeline, PerField |
| `Embedder` | Vector embedding | CandleBert, CandleClip, OpenAI, Precomputed, PerField |
| `Storage` | Data persistence | FileStorage, MemoryStorage |
| `Query` | Search query execution | Term, Phrase, Boolean, Fuzzy, Wildcard, Range, Geo, Span, etc. |

### Factory Pattern

`StorageFactory::create(config)` and `StorageFactory::open(config)` create storage instances from configuration enums.

## Data Flow

### Indexing

```
put_document(id, doc)
    │
    ├─ inject _id field
    ├─ delete existing (if upsert)
    ├─ WAL: append(id, doc) → (doc_id, seq)
    ├─ store document for retrieval
    ├─ split fields by schema type
    │   ├─ lexical fields → LexicalStore.upsert_document(doc_id, doc)
    │   └─ vector fields  → VectorStore.upsert_document(doc_id, vec_doc)
    └─ update sequence trackers
```

On `commit()`:

```
commit()
    ├─ LexicalStore.commit()   → flush segments to storage
    ├─ VectorStore.commit()    → flush vector index to storage
    ├─ DocumentLog.commit()    → persist stored documents
    └─ WAL.truncate()          → remove committed entries
```

### Search (Hybrid)

```
search(request)
    │
    ├─ 1. Filter query (if any)
    │       └─ LexicalStore: execute filter → allowed_ids
    │
    ├─ 2. Parallel execution
    │       ├─ Lexical: execute query → (id, score) pairs
    │       └─ Vector: execute query → (id, score) pairs
    │              ├─ embed query text/bytes
    │              └─ ANN search with allowed_ids filter
    │
    ├─ 3. Fusion (if hybrid)
    │       ├─ RRF: rank-based combination (default)
    │       └─ WeightedSum: score-based linear combination
    │
    └─ 4. Load documents (if requested)
            └─ DocumentLog: retrieve stored fields
```

## WAL and Crash Recovery

The Write-Ahead Log ensures durability:

1. **Every mutation** (upsert, delete) is written to the WAL before any index update
2. **Each entry** has a monotonically increasing sequence number
3. **On commit**, after all stores are flushed, the WAL is truncated
4. **On recovery** (Engine startup), the WAL is replayed:
   - Compare WAL seq with each store's last-applied seq
   - Re-apply any entries the store hasn't seen
   - This makes recovery idempotent

### Consistency Model

- Both LexicalStore and VectorStore must succeed for an operation to be considered applied
- If VectorStore fails during indexing, the LexicalStore change is rolled back
- Sequence trackers are updated only after both stores succeed

## ID Management

### Dual-Tiered ID System

- **External ID** (`String`): User-provided document identifier (e.g., `"article-42"`)
- **Internal ID** (`u64`): Physical handle used for fast lookups

### Internal ID Structure (Shard-Prefixed)

```
Bit Layout (u64):
┌─────────────────┬──────────────────────────────────────────────────┐
│ Bits 48-63      │ Bits 0-47                                       │
│ Shard ID (16b)  │ Local ID (48b)                                  │
└─────────────────┴──────────────────────────────────────────────────┘
```

This allows:

- **Zero-cost aggregation**: shard ID is embedded in the document ID
- **Fast routing**: extract shard from any internal ID with a bit shift
- **O(1) fetch**: direct array index within a shard

## Segment Management

### Lexical Segments

Each segment is an immutable inverted index with these files:

| Extension | Content |
| :--- | :--- |
| `.dict` | Term dictionary |
| `.post` | Posting lists |
| `.bkd` | BKD tree (numeric/date fields) |
| `.docs` | Stored field data |
| `.dv` | Doc values |
| `.meta` | Segment metadata |
| `.lens` | Field lengths |

### Segment Lifecycle

1. **Create**: New documents are buffered in memory
2. **Flush**: On commit, in-memory data becomes an immutable segment
3. **Search**: All segments are searched and results merged
4. **Merge**: Background process combines small segments into larger ones
5. **Delete**: Logical deletion via bitmaps; physical removal during compaction

## Storage Isolation

The Engine uses `PrefixedStorage` to isolate subsystem data:

```
storage root/
├── lexical/        # LexicalStore data (segments, metadata)
├── vector/         # VectorStore data (vector indices)
└── documents/      # DocumentLog data (WAL, stored documents)
```

Each subsystem sees only its own namespace through the prefix wrapper.

## Performance Considerations

- **SIMD**: Distance calculations (cosine, euclidean, etc.) use SIMD intrinsics when available
- **Rayon**: Batch operations use rayon for data parallelism
- **Memory-mapped I/O**: FileStorage supports mmap for efficient large-file access
- **Zero-copy serialization**: rkyv is used for high-performance binary serialization paths
- **Quantization**: Vector indices support scalar 8-bit and product quantization for memory reduction

---

## On-Disk File Format Specification

This section documents every binary and structured file format used by Laurus.
All multi-byte integers are **little-endian** unless otherwise noted.

### Directory Layout

```
<storage root>/
├── documents/                          # DocumentLog namespace
│   ├── engine.wal                      # Write-Ahead Log
│   ├── segments.json                   # Document store manifest (via StructWriter)
│   ├── doc_segment_000000.docs         # Document segment 0
│   ├── doc_segment_000001.docs         # Document segment 1
│   └── ...
│
├── lexical/                            # LexicalStore namespace
│   ├── index.meta                      # Global index metadata (binary)
│   ├── metadata.json                   # Index metadata (JSON)
│   │
│   ├── segment_000001.dict             # Term dictionary
│   ├── segment_000001.post             # Posting lists
│   ├── segment_000001.docs             # Stored fields (binary, type-tagged)
│   ├── segment_000001.lens             # Field lengths
│   ├── segment_000001.fstats           # Field statistics
│   ├── segment_000001.dv               # DocValues (rkyv)
│   ├── segment_000001.meta             # Segment metadata (JSON)
│   ├── segment_000001.json             # Stored documents (JSON, compatibility)
│   ├── segment_000001.delmap           # Deletion bitmap (if deletions exist)
│   ├── segment_000001.<field>.bkd      # BKD tree per numeric/geo field
│   └── ...                             # Additional segments
│
└── vector/                             # VectorStore namespace
    ├── <path>.flat                      # Flat vector index
    ├── <path>.hnsw                      # HNSW vector index
    └── <path>.ivf                       # IVF vector index
```

Each top-level directory corresponds to a `PrefixedStorage` namespace.
See `laurus/src/storage/prefixed.rs`.

---

### Write-Ahead Log (WAL)

**File**: `documents/engine.wal`
**Source**: `laurus/src/store/log.rs`

The WAL is an append-only binary file that ensures durability. Every mutation
(upsert or delete) is written to the WAL **before** any index update.

#### Binary Layout

```
┌──────────────────────────────────────────────────────────┐
│                    WAL File                               │
├──────────────────────────────────────────────────────────┤
│ Record 0                                                  │
│ ┌──────────────┬────────────────────────────────────────┐│
│ │ u32 LE       │ JSON bytes                              ││
│ │ (byte count) │ (LogRecord)                             ││
│ └──────────────┴────────────────────────────────────────┘│
│ Record 1                                                  │
│ ┌──────────────┬────────────────────────────────────────┐│
│ │ u32 LE       │ JSON bytes                              ││
│ │ (byte count) │ (LogRecord)                             ││
│ └──────────────┴────────────────────────────────────────┘│
│ ...                                                       │
└──────────────────────────────────────────────────────────┘
```

Each record is:

| Offset | Size | Type | Description |
| :----- | :--- | :--- | :---------- |
| 0 | 4 | `u32 LE` | Length of the JSON payload in bytes |
| 4 | N | `u8[N]` | JSON-encoded `LogRecord` |

After each record, `flush_and_sync()` is called for fsync durability.

#### LogRecord JSON Schema

```json
{
  "seq": 42,
  "entry": {
    "Upsert": {
      "doc_id": 1,
      "external_id": "article-42",
      "document": { "fields": { ... } }
    }
  }
}
```

Or for deletions:

```json
{
  "seq": 43,
  "entry": {
    "Delete": {
      "doc_id": 1,
      "external_id": "article-42"
    }
  }
}
```

#### Recovery Flow

```
Engine::open()
    │
    ├─ DocumentLog::read_all()
    │   ├─ Read [u32 len][json] pairs until EOF
    │   ├─ Update next_seq = max(seq) + 1
    │   ├─ Update next_doc_id = max(doc_id) + 1
    │   └─ Sync next_doc_id with committed doc_store segments
    │
    ├─ Compare WAL seq with each store's last_wal_seq
    │   └─ Re-apply entries the store hasn't seen
    │
    └─ After successful commit → truncate WAL to 0 bytes
```

---

### Document Store

**Source**: `laurus/src/store/document.rs`

Documents are stored in segmented binary files for retrieval. This is separate
from the lexical index's stored fields — it provides the WAL-backed document
retrieval layer.

#### Segment File: `doc_segment_{:06}.docs`

Written with `StructWriter`. Format:

```
┌─────────────────────────────────────────────────────┐
│ u32 LE: doc_count                                    │
├─────────────────────────────────────────────────────┤
│ Entry 0                                              │
│ ┌──────────────┬───────────────────────────────────┐│
│ │ u64 LE       │ bytes (varint-prefixed)            ││
│ │ doc_id       │ JSON-encoded Document              ││
│ └──────────────┴───────────────────────────────────┘│
│ Entry 1                                              │
│ ┌──────────────┬───────────────────────────────────┐│
│ │ u64 LE       │ bytes (varint-prefixed)            ││
│ │ doc_id       │ JSON-encoded Document              ││
│ └──────────────┴───────────────────────────────────┘│
│ ...                                                  │
├─────────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum (StructWriter trailer)        │
└─────────────────────────────────────────────────────┘
```

Documents are sorted by `doc_id` within each segment. The `bytes` field uses
StructWriter's `write_bytes()` which prefixes the data with a varint length.

#### Manifest: `segments.json`

Written with `StructWriter` (varint-prefixed JSON bytes + CRC32 checksum).
Contains a `StoreManifest`:

```json
{
  "version": 1,
  "segments": [
    {
      "id": 0,
      "start_doc_id": 1,
      "end_doc_id": 100,
      "doc_count": 100
    }
  ],
  "next_segment_id": 1
}
```

Written atomically via a `.tmp` rename.

---

### Lexical Index Segments

**Source**: `laurus/src/lexical/index/inverted/writer.rs`

Each segment is named `segment_{:06}` (e.g., `segment_000001`) and produces
up to 10 files per segment.

#### Global Index Metadata: `index.meta`

**Source**: `laurus/src/lexical/index/inverted/writer.rs:946-963`

```
┌───────────────────────────────────────┐
│ u32 LE: Magic = 0x494D4554 ("IMET")   │
│ u32 LE: Version = 1                   │
│ u64 LE: Timestamp (Unix epoch secs)   │
│ u64 LE: Total docs added              │
│ u32 LE: Segments created              │
├───────────────────────────────────────┤
│ u32 LE: CRC32 checksum                │
└───────────────────────────────────────┘
```

#### Term Dictionary (`.dict`)

**Source**: `laurus/src/lexical/index/structures/dictionary.rs:468-492`

Stores a sorted mapping of terms to their posting list locations.

```
┌────────────────────────────────────────────────────┐
│ Header                                              │
│ ┌────────────────────────────────────────────────┐  │
│ │ u32 LE: Magic = 0x53544443 ("STDC")            │  │
│ │ u32 LE: Version = 1                            │  │
│ │ varint:  Term count                            │  │
│ └────────────────────────────────────────────────┘  │
│                                                     │
│ Term Entries (repeated term_count times)             │
│ ┌────────────────────────────────────────────────┐  │
│ │ string:  Term text (varint-prefixed UTF-8)     │  │
│ │ u64 LE:  Posting offset (byte offset in .post) │  │
│ │ u64 LE:  Posting length (bytes)                │  │
│ │ u64 LE:  Document frequency                    │  │
│ │ u64 LE:  Total frequency                       │  │
│ └────────────────────────────────────────────────┘  │
│                                                     │
├────────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum                              │
└────────────────────────────────────────────────────┘
```

Terms are stored in **sorted** (lexicographic) order, enabling binary search
and prefix-based lookups.

#### Posting Lists (`.post`)

**Source**: `laurus/src/lexical/index/inverted/core/posting.rs:164-205`

Contains the inverted index data — for each term, the list of documents
containing that term along with frequency, weight, and optional positions.

```
┌────────────────────────────────────────────────────────┐
│ Posting List for Term 0 (at offset from .dict)          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ string:  Term text (varint-prefixed UTF-8)         │  │
│ │ varint:  Total frequency                           │  │
│ │ varint:  Document frequency                        │  │
│ │ varint:  Posting count                             │  │
│ │                                                    │  │
│ │ Per-posting (repeated posting_count times):         │  │
│ │ ┌────────────────────────────────────────────────┐ │  │
│ │ │ varint: doc_id delta (from previous doc_id)    │ │  │
│ │ │ varint: frequency                              │ │  │
│ │ │ f32 LE: weight                                 │ │  │
│ │ │ u8:     has_positions (0 or 1)                 │ │  │
│ │ │ [if has_positions == 1]:                       │ │  │
│ │ │   varint: position count                       │ │  │
│ │ │   varint: position delta * position_count      │ │  │
│ │ └────────────────────────────────────────────────┘ │  │
│ └────────────────────────────────────────────────────┘  │
│                                                         │
│ Posting List for Term 1                                  │
│ └─ ... (same structure)                                  │
│                                                         │
├────────────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum                                  │
└────────────────────────────────────────────────────────┘
```

**Delta compression**: Document IDs are stored as deltas from the previous
doc_id (first doc_id is delta from 0). Positions within a posting are also
delta-compressed. This significantly reduces file size for sorted ID sequences.

#### Stored Fields (`.docs`)

**Source**: `laurus/src/lexical/index/inverted/writer.rs:675-741`

Binary type-tagged field storage for each document.

```
┌────────────────────────────────────────────────────────┐
│ varint: document count                                  │
│                                                         │
│ Per-document (repeated doc_count times):                 │
│ ┌────────────────────────────────────────────────────┐  │
│ │ u64 LE:  doc_id                                    │  │
│ │ varint:  field count                               │  │
│ │                                                    │  │
│ │ Per-field (repeated field_count times):              │  │
│ │ ┌────────────────────────────────────────────────┐ │  │
│ │ │ string: field name (varint-prefixed UTF-8)     │ │  │
│ │ │ u8:     type tag                               │ │  │
│ │ │ <type-specific data>                           │ │  │
│ │ └────────────────────────────────────────────────┘ │  │
│ └────────────────────────────────────────────────────┘  │
│                                                         │
├────────────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum                                  │
└────────────────────────────────────────────────────────┘
```

**Type Tags and Encoding**:

| Tag | Type | Encoding |
| :-- | :--- | :------- |
| `0` | Text | `string` (varint-prefixed UTF-8) |
| `1` | Int64 | `u64 LE` (bit pattern preserved) |
| `2` | Float64 | `f64 LE` |
| `3` | Bool | `u8` (0 = false, 1 = true) |
| `4` | Bytes | `string` (MIME type) + `varint` (raw len) + `bytes` (varint-prefixed) |
| `5` | DateTime | `string` (RFC 3339 text) |
| `6` | Geo | `f64 LE` (latitude) + `f64 LE` (longitude) |
| `7` | Null | (no data) |
| `9` | Vector | `varint` (dimension) + `f32 LE * dimension` |

#### Field Lengths (`.lens`)

**Source**: `laurus/src/lexical/index/inverted/writer.rs:776-798`

Stores the token count (field length) for each field in each document.
Used by BM25 scoring.

```
┌────────────────────────────────────────────────┐
│ varint: document count                          │
│                                                 │
│ Per-document:                                    │
│ ┌─────────────────────────────────────────────┐ │
│ │ u64 LE:  doc_id                             │ │
│ │ varint:  field count                        │ │
│ │ Per-field:                                   │ │
│ │   string: field name (varint-prefixed UTF-8)│ │
│ │   u32 LE: field length (token count)        │ │
│ └─────────────────────────────────────────────┘ │
│                                                 │
├────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum                          │
└────────────────────────────────────────────────┘
```

#### Field Statistics (`.fstats`)

**Source**: `laurus/src/lexical/index/inverted/writer.rs:800-821`

Aggregate statistics per field across the segment. Used by BM25 for
average field length calculation.

```
┌────────────────────────────────────────────────┐
│ varint: field count                              │
│                                                  │
│ Per-field:                                        │
│ ┌──────────────────────────────────────────────┐ │
│ │ string: field name (varint-prefixed UTF-8)   │ │
│ │ u64 LE: document count                       │ │
│ │ f64 LE: average field length                 │ │
│ │ u64 LE: minimum field length                 │ │
│ │ u64 LE: maximum field length                 │ │
│ └──────────────────────────────────────────────┘ │
│                                                  │
├────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum                          │
└────────────────────────────────────────────────┘
```

#### DocValues (`.dv`)

**Source**: `laurus/src/lexical/index/structures/doc_values.rs`

Column-oriented storage for field values, optimized for sorting, faceting,
and aggregations. Uses `rkyv` for zero-copy deserialization.

```
┌────────────────────────────────────────────────────────┐
│ Header                                                  │
│ ┌────────────────────────────────────────────────────┐  │
│ │ u8[4]: Magic = "DVFF" (0x44, 0x56, 0x46, 0x46)    │  │
│ │ u8[2]: Version = [1, 0]                            │  │
│ │ u32 LE: field count                                │  │
│ └────────────────────────────────────────────────────┘  │
│                                                         │
│ Per-field (repeated field_count times):                   │
│ ┌────────────────────────────────────────────────────┐  │
│ │ u32 LE:  field name length                         │  │
│ │ u8[N]:   field name (UTF-8, NOT varint-prefixed)   │  │
│ │ u64 LE:  value count                               │  │
│ │ u64 LE:  serialized data length                    │  │
│ │ u8[M]:   rkyv-serialized Vec<(u64, FieldValue)>    │  │
│ └────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

> **Note**: Unlike other segment files, the `.dv` file does **not** use
> `StructWriter` — it writes raw bytes directly via `std::io::Write`.
> Therefore it has **no** trailing CRC32 checksum.

#### BKD Tree (`.{field}.bkd`)

**Source**: `laurus/src/lexical/index/structures/bkd_tree.rs`

A simplified BKD Tree for numeric range queries and geospatial filtering.
One file per numeric/geo field per segment.

```
┌────────────────────────────────────────────────────────────────┐
│ Header (fixed size = 48 + num_dims * 16 bytes)                  │
│ ┌──────────────────────────────────────────────────────────────┐│
│ │ u32 LE: Magic = 0x54444B42 ("BKDT")                         ││
│ │ u32 LE: Version = 1                                         ││
│ │ u32 LE: num_dims (number of dimensions)                     ││
│ │ u32 LE: bytes_per_dim = 8 (f64)                             ││
│ │ u64 LE: total_point_count                                   ││
│ │ u64 LE: num_blocks                                          ││
│ │ f64 LE: min_values[0..num_dims]                             ││
│ │ f64 LE: max_values[0..num_dims]                             ││
│ │ u64 LE: index_start_offset                                  ││
│ │ u64 LE: root_node_offset                                    ││
│ └──────────────────────────────────────────────────────────────┘│
│                                                                 │
│ Leaf Blocks (written during recursive tree build)                │
│ ┌──────────────────────────────────────────────────────────────┐│
│ │ varint: point count in this block                            ││
│ │ Per-point:                                                   ││
│ │   f64 LE: values[0..num_dims]                                ││
│ │   u64 LE: doc_id                                             ││
│ └──────────────────────────────────────────────────────────────┘│
│                                                                 │
│ Index Section (at index_start_offset)                            │
│ ┌──────────────────────────────────────────────────────────────┐│
│ │ Per internal node (28 bytes each):                           ││
│ │   u32 LE: split_dim                                          ││
│ │   f64 LE: split_value                                        ││
│ │   u64 LE: left_offset                                        ││
│ │   u64 LE: right_offset                                       ││
│ └──────────────────────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────────────┘
```

The tree is built recursively:

- Leaf blocks contain up to `block_size` (default 512) points
- Internal nodes split on dimensions in round-robin order
- Leaves are written first, then the index section is appended

#### Segment Metadata (`.meta`)

**Source**: `laurus/src/lexical/index/inverted/writer.rs:890-927`

JSON file containing segment-level metadata:

```json
{
  "segment_id": "segment_000001",
  "doc_count": 1000,
  "min_doc_id": 1,
  "max_doc_id": 1000,
  "generation": 0,
  "has_deletions": false,
  "shard_id": 0
}
```

#### Deletion Bitmap (`.delmap`)

**Source**: `laurus/src/maintenance/deletion.rs:177-199`

Tracks logically deleted documents within a segment. Only created when
the segment has deletions (`has_deletions = true` in `.meta`).

```
┌──────────────────────────────────────────────────────┐
│ Header                                                │
│ ┌──────────────────────────────────────────────────┐  │
│ │ u32 LE:  Magic = 0x44454C42 ("DELB")             │  │
│ │ u32 LE:  Version = 3                             │  │
│ └──────────────────────────────────────────────────┘  │
│                                                       │
│ Metadata                                               │
│ ┌──────────────────────────────────────────────────┐  │
│ │ string:  segment_id (varint-prefixed UTF-8)      │  │
│ │ u64 LE:  total_docs                              │  │
│ │ u64 LE:  deleted_count                           │  │
│ │ u64 LE:  last_modified (Unix epoch secs)         │  │
│ │ u64 LE:  version                                 │  │
│ │ u64 LE:  min_doc_id                              │  │
│ │ u64 LE:  max_doc_id                              │  │
│ └──────────────────────────────────────────────────┘  │
│                                                       │
│ Deleted IDs                                            │
│ ┌──────────────────────────────────────────────────┐  │
│ │ varint:  count of deleted doc IDs                │  │
│ │ u64 LE:  doc_id * count                          │  │
│ └──────────────────────────────────────────────────┘  │
│                                                       │
├──────────────────────────────────────────────────────┤
│ u32 LE: CRC32 checksum                                │
└──────────────────────────────────────────────────────┘
```

#### Compatibility JSON Documents (`.json`)

**Source**: `laurus/src/lexical/index/inverted/writer.rs:838-859`

A JSON array of documents for backward compatibility with `BasicIndexReader`.
This is a convenience format — the binary `.docs` file is the primary store.

---

### Vector Index Segments

**Source**: `laurus/src/vector/index/`

Vector indices store embedding vectors and optional graph structures for
approximate nearest neighbor (ANN) search. Three index types are supported.

#### Flat Index (`.flat`)

**Source**: `laurus/src/vector/index/flat/writer.rs:355-400`

Brute-force exact search index. Stores all vectors sequentially.

```
┌────────────────────────────────────────────────────────┐
│ Header                                                  │
│ ┌────────────────────────────────────────────────────┐  │
│ │ u32 LE: vector_count                               │  │
│ │ u32 LE: dimension                                  │  │
│ └────────────────────────────────────────────────────┘  │
│                                                         │
│ Vectors (repeated vector_count times)                    │
│ ┌────────────────────────────────────────────────────┐  │
│ │ u64 LE:  doc_id                                    │  │
│ │ u32 LE:  field_name length                         │  │
│ │ u8[N]:   field_name (UTF-8)                        │  │
│ │ f32 LE:  values[0..dimension]                      │  │
│ └────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

> **Note**: Flat index files use raw `std::io::Write` — no StructWriter,
> no trailing CRC32 checksum.

#### HNSW Index (`.hnsw`)

**Source**: `laurus/src/vector/index/hnsw/writer.rs:1070-1160`

Hierarchical Navigable Small World graph for fast approximate search.

```
┌──────────────────────────────────────────────────────────────┐
│ Header                                                        │
│ ┌──────────────────────────────────────────────────────────┐  │
│ │ u64 LE: vector_count                                     │  │
│ │ u32 LE: dimension                                        │  │
│ │ u32 LE: M (max connections per layer)                    │  │
│ │ u32 LE: ef_construction                                  │  │
│ └──────────────────────────────────────────────────────────┘  │
│                                                               │
│ Vectors (sorted by doc_id, repeated vector_count times)       │
│ ┌──────────────────────────────────────────────────────────┐  │
│ │ u64 LE:  doc_id                                          │  │
│ │ u32 LE:  field_name length                               │  │
│ │ u8[N]:   field_name (UTF-8)                              │  │
│ │ f32 LE:  values[0..dimension]                            │  │
│ └──────────────────────────────────────────────────────────┘  │
│                                                               │
│ Graph Section                                                 │
│ ┌──────────────────────────────────────────────────────────┐  │
│ │ u8:     has_graph (0 = no graph, 1 = has graph)          │  │
│ │                                                          │  │
│ │ [if has_graph == 1]:                                     │  │
│ │ u64 LE: entry_point (doc_id, u64::MAX if none)           │  │
│ │ u32 LE: max_level                                        │  │
│ │ u64 LE: node_count                                       │  │
│ │                                                          │  │
│ │ Per-node (sorted by doc_id, repeated node_count times):  │  │
│ │ ┌────────────────────────────────────────────────────┐   │  │
│ │ │ u64 LE: doc_id                                     │   │  │
│ │ │ u32 LE: layer_count                                │   │  │
│ │ │ Per-layer (repeated layer_count times):             │   │  │
│ │ │   u32 LE: neighbor_count                           │   │  │
│ │ │   u64 LE: neighbor_doc_id * neighbor_count         │   │  │
│ │ └────────────────────────────────────────────────────┘   │  │
│ └──────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

The graph is a multi-layer structure where:

- **Layer 0** (base): every node is present, max `2*M` connections
- **Layer 1+** (upper): exponentially fewer nodes, max `M` connections
- **Entry point**: the node present at the highest layer

#### IVF Index (`.ivf`)

**Source**: `laurus/src/vector/index/ivf/writer.rs:920-977`

Inverted File index using k-means clustering for memory-efficient search.

```
┌────────────────────────────────────────────────────────────────┐
│ Header                                                          │
│ ┌────────────────────────────────────────────────────────────┐  │
│ │ u32 LE: vector_count                                       │  │
│ │ u32 LE: dimension                                          │  │
│ │ u32 LE: n_clusters                                         │  │
│ │ u32 LE: n_probe                                            │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ Centroids (repeated n_clusters times)                            │
│ ┌────────────────────────────────────────────────────────────┐  │
│ │ f32 LE: values[0..dimension]                               │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ Inverted Lists (repeated n_clusters times)                       │
│ ┌────────────────────────────────────────────────────────────┐  │
│ │ u32 LE: list_size (vectors in this cluster)                │  │
│ │                                                            │  │
│ │ Per-vector (repeated list_size times):                      │  │
│ │ ┌──────────────────────────────────────────────────────┐   │  │
│ │ │ u64 LE:  doc_id                                      │   │  │
│ │ │ u32 LE:  field_name length                           │   │  │
│ │ │ u8[N]:   field_name (UTF-8)                          │   │  │
│ │ │ f32 LE:  values[0..dimension]                        │   │  │
│ │ └──────────────────────────────────────────────────────┘   │  │
│ └────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

At search time, only `n_probe` clusters (nearest to the query vector) are
scanned, trading accuracy for speed.

---

### Encoding Primitives

**Source**: `laurus/src/storage/structured.rs`, `laurus/src/util/varint.rs`

#### StructWriter / StructReader

All lexical index files use `StructWriter` / `StructReader` for binary I/O.
This provides consistent encoding and a trailing CRC32 checksum.

| Method | Wire Format | Size |
| :----- | :---------- | :--- |
| `write_u8(v)` | `u8` | 1 byte |
| `write_u16(v)` | `u16 LE` | 2 bytes |
| `write_u32(v)` | `u32 LE` | 4 bytes |
| `write_u64(v)` | `u64 LE` | 8 bytes |
| `write_f32(v)` | `f32 LE` | 4 bytes |
| `write_f64(v)` | `f64 LE` | 8 bytes |
| `write_varint(v)` | VarInt (see below) | 1–10 bytes |
| `write_string(s)` | `varint(len)` + `utf8 bytes` | variable |
| `write_bytes(b)` | `varint(len)` + `raw bytes` | variable |
| `write_raw(b)` | `raw bytes` (no length prefix) | exact |
| `close()` | `u32 LE` CRC32 checksum | 4 bytes |

`StructReader::is_eof()` accounts for the 4-byte trailing checksum by
returning `true` when `position >= file_size - 4`.

#### Variable-Length Integer (VarInt)

**Source**: `laurus/src/util/varint.rs`

LEB128-style encoding where each byte stores 7 data bits. The high bit
(0x80) is the continuation flag:

```
Value: 300 (0x012C)

Binary: 00000001 00101100
                ↓
Split into 7-bit groups (LSB first):
  Group 0: 0101100 = 0x2C  →  0xAC (continuation bit set)
  Group 1: 0000010 = 0x02  →  0x02 (no continuation)

Wire bytes: [0xAC, 0x02]
```

| Value Range | Bytes Used |
| :---------- | :--------- |
| 0 – 127 | 1 |
| 128 – 16,383 | 2 |
| 16,384 – 2,097,151 | 3 |
| ... | ... |
| u64::MAX | 10 |

#### Delta Compression

Used in posting lists for doc_id sequences and position arrays.
Instead of storing absolute values, stores the difference from the
previous value:

```
Original:  [100, 105, 110, 120, 150]
Deltas:    [100,   5,   5,  10,  30]  ← each stored as varint
```

This is effective because sorted document IDs have small gaps,
and positions within a document are naturally ascending.

#### rkyv Serialization

Used in DocValues (`.dv` files) for zero-copy deserialization of
`Vec<(u64, FieldValue)>`. The `rkyv` crate produces archived data
that can be accessed directly from a memory-mapped file without
deserializing into heap-allocated Rust objects.
