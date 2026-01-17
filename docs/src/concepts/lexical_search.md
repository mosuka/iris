# Lexical Search

Lexical search matches documents based on exact or approximate keyword matches. It is the traditional "search engine" functionality found in Lucene or Elasticsearch.

## Document Structure
In Sarissa, a **Document** is the fundamental unit of indexing. It follows a **schema-less** design, allowing fields to be added dynamically without defining a schema upfront.

Each `Document` consists of multiple `Fields` stored in a Map where the key is the field name. Each `Field` has a **Value** and **Options** defining how it should be indexed.

```mermaid
classDiagram
    class Document {
        fields: Map<String, Field>
    }

    class Field {
        value: FieldValue
        option: FieldOption
    }

    class FieldValue {
        Text
        Integer
        Float
        Boolean
        DateTime
        Geo
        Blob
        Null
    }

    class FieldOption {
        <<enumeration>>
        Text(TextOption)
        Integer(IntegerOption)
        Float(FloatOption)
        Boolean(BooleanOption)
        DateTime(DateTimeOption)
        Geo(GeoOption)
        Blob(BlobOption)
    }

    class TextOption {
        indexed: bool
        stored: bool
        term_vectors: bool
    }

    class IntegerOption {
        indexed: bool
        stored: bool
    }

    class FloatOption {
        indexed: bool
        stored: bool
    }

    class BooleanOption {
        indexed: bool
        stored: bool
    }

    class DateTimeOption {
        indexed: bool
        stored: bool
    }

    class GeoOption {
        indexed: bool
        stored: bool
    }

    class BlobOption {
        stored: bool
    }

    Document *-- Field
    Field --> FieldValue
    Field --> FieldOption
```

### Document
The fundamental unit of indexing in Sarissa.
- **Schema-less**: Fields can be added dynamically without a predefined schema.
- **Map Structure**: Fields are stored in a `HashMap` where the key is the field name (String).
- **Flexible**: A single document can contain a mix of different field types (Text, Integer, Blob, etc.).

### Field
A container representing a single data point within a document.
- **Value**: The actual data content (e.g., "Hello World", 123, true). Defined by `FieldValue`.
- **Option**: Configuration for how this data should be handled (e.g., indexed, stored). Defined by `FieldOption`.

### Field Values
- **Text**: UTF-8 string. Typically analyzed and indexed for full-text search.
- **Integer / Float**: Numeric values. Used for range queries (BKD Tree) and sorting.
- **Boolean**: True/False values.
- **DateTime**: UTC timestamps.
- **Geo**: Latitude/Longitude coordinates. Indexed in a 2D BKD tree for efficient spatial queries (distance and bounding box) and stored for precise calculations.
- **Blob**: Raw byte data with MIME type. Used for storing binary content (images, etc.) or vector source data. **Stored only**, never indexed by the lexical engine.

### Field Options
Configuration for the field defining how it should be indexed and stored.

- **TextOption**:
    - `indexed`: If true, the text is analyzed and added to the inverted index (searchable).
    - `stored`: If true, the original text is stored in the doc store (retrievable).
    - `term_vectors`: If true, stores term positions and offsets (needed for highlighting and "More Like This").
- **IntegerOption / FloatOption**:
    - `indexed`: If true, the value is added to the BKD tree (range searchable).
    - `stored`: If true, the original value is stored.
- **BooleanOption**:
    - `indexed`: If true, the value is indexed.
    - `stored`: If true, the original value is stored.
- **DateTimeOption**:
    - `indexed`: If true, the timestamp is added to the BKD tree (range searchable).
    - `stored`: If true, the original timestamp is stored.
- **GeoOption**:
    - `indexed`: If true, the coordinates are added to the 2D BKD tree (efficient spatial search).
    - `stored`: If true, the original coordinates are stored.
- **BlobOption**:
    - `stored`: If true, the binary data is stored. **Note**: Blobs cannot be indexed by the lexical engine.

## Indexing Process
The indexing process converts raw documents into a searchable index.

```mermaid
graph TD
    subgraph "Indexing Flow"
        Input["Raw Data"] --> DocBuilder["Document Construction"]
        
        subgraph "Processing (InvertedIndexWriter)"
            DocBuilder -->|Text| CharFilter["Char Filter"]
            DocBuilder -->|Numeric/Date/Geo| Normalizer["String Normalizer"]
            DocBuilder -->|Numeric/Date/Geo| PtExt["Point Extractor"]
            DocBuilder -->|Stored Field| StoreProc["Field Values Collector"]
            DocBuilder -->|All Fields| LenTracker["Field Length Tracker"]
            DocBuilder -->|Doc Values| DVTracker["Doc Values Collector"]
            
            subgraph "Analysis Chain"
                CharFilter --> Tokenizer["Tokenizer"]
                Tokenizer --> TokenFilter["Token Filter"]
            end
        end
        
        subgraph "In-Memory Buffering"
            TokenFilter -->|Terms| InvBuffer["Term Posting Index"]
            Normalizer -->|Terms| InvBuffer
            PtExt -->|Points| BkdBuffer["Point Values Buffer"]
            StoreProc -->|Data| DocsBuffer["Stored Docs Buffer"]
        end
        
        subgraph "Segment Flushing (Disk)"
            InvBuffer -->|Write| Postings[".dict / .post"]
            BkdBuffer -->|Sort & Write| BKD[".bkd"]
            DocsBuffer -->|Write| DOCS[".docs"]
            DVTracker -->|Write| DV[".dv"]
            LenTracker -->|Write| LENS[".lens"]
            InvBuffer -.->|Stats| Meta[".meta / .fstats"]
        end
    end
```

1. **Document Processing**:
   - **Analysis & Normalization**: Text is processed through the Analysis Chain (`Char Filter`, `Tokenizer`, `Token Filter`). Non-text fields are handled by the `String Normalizer`.
   - **Point Extraction**: Multidimensional values (Numeric, Date, and Geo) are extracted by the `Point Extractor` for spatial indexing (BKD Tree).
   - **Tracking & Collection**: `Field Length Tracker` and `Doc Values Collector` gather metadata and columnar data.
2. **In-Memory Buffering**:
   - Terms are added to the `Term Posting Index`.
   - Extracted points and stored fields are staged in the `Point Values Buffer` and `Stored Docs Buffer`.
3. **Segment Flushing**:
   - Buffered data is periodically sorted and serialized into immutable **Segment** files on disk.
4. **Merging**:
   - A background process automatically merges smaller segments into larger ones to optimize read performance and reclaim space from deleted documents.

### Analyzers
Text analysis is the process of converting raw text into tokens. An Analyzer is typically composed of a pipeline:

1. **Char Filters**: Transform the raw character stream (e.g., removing HTML tags).
2. **Tokenizer**: Splits the character stream into a token stream (e.g., splitting by whitespace).
3. **Token Filters**: Modify the token stream (e.g., lowercasing, stemming, removing stop words).

Sarissa provides several built-in analyzers with pre-configured pipelines:

- **StandardAnalyzer**: Good default for most European languages (alias for a generic English setup).
    - Tokenizer: `RegexTokenizer` (Splits on Unicode word boundaries)
    - Token Filters: `LowercaseFilter`, `StopFilter` (English stop words)
- **KeywordAnalyzer**: Treats the entire input as a single token.
    - Tokenizer: `WholeTokenizer`
    - No filters.
- **SimpleAnalyzer**: Basic tokenization without filtering.
    - Tokenizer: Configurable (defaults to `RegexTokenizer` in some contexts)
    - No filters.
- **No-op Analyzer**: Performs no analysis, yielding an empty token stream.
    - Useful for stored-only fields or when a field should not be searchable.
- **PipelineAnalyzer**: A flexible builder for creating custom analysis pipelines.
    - Allows combining any **Char Filter**, **Tokenizer**, and **Token Filter** chain.
    - Example: `PipelineAnalyzer::new(tokenizer).add_char_filter(...).add_filter(...)`
- **LanguageAnalyzer**: Analyzers specialized for specific languages.
    - **EnglishAnalyzer**: Specialized for English (similar to StandardAnalyzer but explicit).
        - Tokenizer: `RegexTokenizer` (Splits on Unicode word boundaries)
        - Token Filters: `LowercaseFilter`, `StopFilter` (English stop words)
    - **JapaneseAnalyzer**: Optimized for Japanese text.
        - Char Filters: `UnicodeNormalizationCharFilter` (NFKC), `JapaneseIterationMarkCharFilter` (Normalizes iteration marks like 々)
        - Tokenizer: `LinderaTokenizer` (Morphological analysis using UniDic)
        - Token Filters: `LowercaseFilter`, `StopFilter` (Japanese stop words)
- **PerFieldAnalyzer**: Wraps multiple analyzers to apply them based on field names (Lucene-compatible).
    - Useful when different fields require different analysis strategies (e.g., standard for "body", keyword for "tags").
    - Example:
      ```rust
      let mut analyzer = PerFieldAnalyzer::new(default_analyzer);
      analyzer.add_analyzer("tags", keyword_analyzer);
      ```

## Index Components
The indexing architecture is responsible for transforming raw documents into efficient, searchable structures. It consists of several logical components, each managing a specific part of the lifecycle.

### Index Writer (`InvertedIndexWriter`)
The primary interface for adding documents to the index. It orchestrates the flow from processing to buffering and triggers flushes.

### Document Processing
- **Analysis Chain**: Performs sequentially through a `Char Filter`, a `Tokenizer`, and a `Token Filter` on text fields to produce searchable terms.
- **String Normalizer**: Converts Numeric, Date, and Geo values into text form for basic keyword matching in the inverted index.
- **Point Extractor**: Isolates numeric, temporal, and geographical data for multi-dimensional range and spatial search (BKD Tree).
- **Data Collectors & Trackers**:
    - **Field Values Collector**: Buffers fields marked as `stored` for the Doc Store.
    - **Field Length Tracker**: Records token counts per field for length-normalized scoring (BM25).
    - **Doc Values Collector**: Gathers values for columnar storage (`.dv`), enabling fast sorting and aggregations (SIMD-optimized).

### In-Memory Buffering
- **Term Posting Index (`TermPostingIndex`)**: A high-performance buffer mapping terms to their occurrences (postings).
- **Point Values Buffer**: Stages multidimensional data before it is structured into a BKD tree.
- **Stored Docs Buffer**: Buffers serialized field data until a segment flush occurs.

### Segment Manager
Manages the lifecycle and visibility of segments, maintaining the `segments.manifest` file and tracking document deletions.

### Index Segment Files
A single segment is composed of several specialized files:
- **Term Dictionary (.dict)**: Maps terms to their locations in the postings list.
- **Postings Lists (.post)**: Stores document IDs, frequencies, and positions for each term.
- **BKD Tree (.bkd)**: Provides multidimensional indexing for numeric and date fields.
- **Doc Store (.docs)**: Stores the original (stored) field values.
- **Doc Values (.dv)**: Stores field values in a columnar format for sorting and scoring.
- **Segment Metadata (.meta)**: JSON file containing segment statistics and ID ranges.
- **Field Statistics (.fstats)**: Stores per-field information like document frequency and average length.
- **Field Lengths (.lens)**: Stores the length of each field for every document (used in BM25 scoring).
**Core component of the Text Inverted Index.**
A sorted list of all unique terms (tokens) extracted from the documents. It acts as the primary entry point for text search.
- **Function**: Maps a term (e.g., "rust") to its location in the Postings List.
- **Features**: Supports fast distinct term lookup, prefix search, and range scans.
- **Format**:
    - **Magic**: `STDC` (Sorted) or `HTDC` (Hash)
    - **Entries**:
        - `Term`: String
        - `Pointer`: `PostingOffset` (u64), `PostingLength` (u64)
        - `Stats`: `DocFrequency` (u64), `TotalFrequency` (u64)

### Postings Lists (`.post`)
**Core component of the Text Inverted Index.**
Stores the relationships between terms and documents.
- **Function**: For a given term, provides the list of Document IDs containing it.
- **Features**: Highly compressed using delta encoding and varints. Includes frequency and position data for scoring and phrase queries.
- **Format**:
    - **Header**: `Term` (String), `TotalFreq` (Varint), `DocFreq` (Varint), `PostingCount` (Varint).
    - **Postings List**: Sequence of:
        - `DocIDDelta`: VarInt (difference from previous DocID)
        - `Frequency`: VarInt (term freq in doc)
        - `Weight`: Float32 (contribution to score)
        - `HasPositions`: Byte (0 or 1)
        - `Positions`: (Optional) `Count` (VarInt) + `PositionDeltas` (VarInts)

### BKD Tree (`.bkd`)
**Component for Numeric and Geospatial Search.**
A persistent tree structure for multi-dimensional data.
- **Function**: Efficiently handles range queries (e.g., `price > 100`, `date in 2023`).

- **Features**: Block-based storage optimized for disk I/O.
- **Format**:
    - **Magic**: `BKDT`
    - **Index Section**: Internal nodes for tree traversal.
    - **Leaf Blocks**: Contiguous blocks of `(Value Vector, DocID)` pairs.

### Document Store (`.docs`)
**Component for Data Retrieval.**
Stores the original content of fields marked as `stored`.
- **Function**: Retrieves the full document content (JSON) after the search has identified the matching DocIDs.
- **Format**:
    - **Magic**: `DOCS` + Version + DocCount
    - **Data**: Sequential list of documents. Each document contains `DocID`, `FieldCount`, and then for each field: `Name`, `TypeTag`, and `Value`.

### Auxiliary Components

#### Doc Values (`.dv`)
Columnar storage for sorting and aggregations.
- **Function**: Fast access to specific field values across many documents.

#### Field Statistics (`.fstats`)
Global statistics for each field (min/max length, doc count).
- **Usage**: Query planning and optimization.

#### Field Lengths (`.lens`)
Stores the number of tokens per field per document.
- **Usage**: Essential for BM25 scoring (length normalization).

#### Segment Metadata (`.meta`) & Manifest (`segments.manifest`)
Registry and metadata files.
- **Format**: JSON (`SegmentInfo`).
- **Fields**: `segment_id`, `doc_count`, `doc_offset`, `generation`, `has_deletions`.
- **Usage**: Managing segment lifecycle, versioning, and status.

## Search Process
The search process in Sarissa involves several stages to efficiently retrieve and rank documents.

```mermaid
graph TD
    subgraph "Search Flow"
        UserQuery["User Query"] --> Parser
        
        subgraph "Searcher"
            Parser["Query Parser"] --> QueryObj["Query"]
            QueryObj --> WeightObj["Weight"]
            WeightObj --> MatcherObj["Matcher"]
            WeightObj --> ScorerObj["Scorer"]
            
            subgraph "Index Access"
                MatcherObj -.->|Look up| II["Inverted Index"]
                MatcherObj -.->|Range Scan| BKD["BKD Tree"]
            end
            
            MatcherObj -->|Doc IDs| CollectorObj["Collector"]
            ScorerObj -->|Scores| CollectorObj
            CollectorObj -.->|Sort by Field| DV["Doc Values"]
            CollectorObj -->|Top Doc IDs| Fetcher["Fetcher"]
            Fetcher -.->|Retrieve Fields| Docs
        end
        
        Fetcher --> Result["Search Results"]
    end
```

1. **Query Parsing**: The `Query Parser` converts the input query string into a structured `Query` tree.
2. **Weight Creation**: The `Query` creates a `Weight` component, which calculates global statistics and normalization factors across all segments.
3. **Matching & Scoring**:
   - For each segment, the `Weight` creates a `Matcher` and a `Scorer`.
   - The `Matcher` iterates over the `Inverted Index` or `BKD Tree` to identify matching `Doc IDs`.
   - The `Scorer` calculates relevance (e.g., BM25) for each matching document.
4. **Collection & Fetching**:
   - The `Collector` aggregates the top results, potentially using `Doc Values` for custom sorting.
   - The `Fetcher` retrieves the original field content from the `Doc Store` for the final result set.

### Query Rewriting
Sarissa implements a **Multi-Term Query** framework (similar to Lucene) for queries like `FuzzyQuery`, `PrefixQuery`, `WildcardQuery`, and `RegexpQuery`.

Instead of matching these complex patterns directly against every document, they undergo a **rewrite** process:
1. **Term Enumeration**: The query identifies all unique terms in the `Term Dictionary` that match the pattern (e.g., `hel*` matches `hello`, `help`, `held`).
2. **Expansion**: These terms are expanded into a `BooleanQuery` based on the selected strategy.
3. **Scoring Strategy (`RewriteMethod`)**:
   - **TopTermsBlended**: (Default) Collects top N terms by frequency and assigns a constant score.
   - **TopTermsScoring**: Collects top N terms and maintains their individual BM25 scores.
   - **ConstantScore**: All matching terms receive a score equal to the query boost.
   - **BooleanQuery**: Expands to all matching terms without limits (may hit clause limits).

## Search Components
The search architecture is composed of several modular components that work together to execute queries and rank results.

### Searcher (`InvertedIndexSearcher`)
The primary entry point for search operations.
- **Coordination**: It coordinates the search across multiple immutable segments.
- **Parallelism**: Supports parallel execution of sub-queries (e.g., within a `BooleanQuery`) to improve performance.
- **API**: Provides high-level methods for searching with string queries or structured `Query` objects.

### Query Parser
Translates human-readable query strings into structured objects.
- **Grammar**: Uses a recursive descent parser to handle operators like `+`, `-`, `OR`, `*`, and field-specific searches (e.g., `title:rust`).
- **Flexible**: Can be configured with default fields and analyzers.

### Query
A logical representation of the search criteria (Term, Prefix, Fuzzy, etc.). It acts as a factory for creating `Weight` objects.

### Weight
An intermediate component that bridges the query and the segment-level execution. It handles global scoring statistics (like IDF) and creates segment-specific `Matcher` and `Scorer` instances.

### Matcher
A low-level iterator that identifies matching documents within a single segment. It supports efficient skipping (`skip_to`) for boolean operations.

### Scorer
Calculates the relevance score for a document.
- **BM25**: Default implementation using the Okapi BM25 algorithm.
- **Constant**: Assigns a fixed score to all matched documents.
- **SIMD Optimization**: Supports SIMD-accelerated batch scoring for high-throughput ranking across large result sets.

### Collector
Aggregates and sorts matching documents.
- **TopDocsCollector**: Collects the top N documents by score.
- **TopFieldCollector**: Collects documents sorted by a specific field value (e.g., price, date).
- **CountCollector**: Returns only the total number of hits without retrieving document IDs.

### Index Reader (`InvertedIndexReader`)
The unified interface for accessing indexed data across all segments. It handles resource loading and provides the necessary contexts to the `Searcher`.

## Scoring (BM25)
Sarissa uses the **Okapi BM25** algorithm as its default similarity function. It is a probabilistic retrieval framework that improves upon TF-IDF by adding saturation and length normalization.

**Formula Components**:
- **TF (Term Frequency)**: How often the term appears in the document. Contribution saturates (diminishing returns) to prevent keyword spamming.
- **IDF (Inverse Document Frequency)**: How rare the term is across the entire index. Rare terms carry more weight.
- **Field Length Norm**: Shorter fields (e.g., "Title") are considered more relevant than long fields (e.g., "Body") for the same match.

## Query Types
Sarissa supports a diverse set of queries for different use cases.

### Core Queries
- **TermQuery**: Exact match for a single token.
  - *Example*: Field "status" matches "active".
- **BooleanQuery**: Combines queries with `MUST` (+), `SHOULD` (OR), `MUST_NOT` (-).
  - *Minimum Should Match*: Supports specifying a minimum number of `SHOULD` clauses that must match for the document to be considered a hit.
  - *Example*: `+rust -c++` (Must contain "rust", must not contain "c++").


### Approximate Queries
- **FuzzyQuery**: Matches terms within a specific Levenshtein edit distance (default 2).
  - *Example*: "helo" matches "hello".
- **WildcardQuery**: Supported standard wildcards `*` (any) and `?` (single char).
  - *Example*: `te*t` matches "test", "text".
- **PrefixQuery**: Matches terms starting with a specific prefix.
  - *Example*: `data*` matches "database", "datum".
- **RegexpQuery**: Full regular expression support.
  - *Example*: `[0-9]{3}-[0-9]{4}`.

### Range Queries
- **NumericRangeQuery**: Efficient BKD-tree based range search for integers and floats. Supports inclusive/exclusive bounds.
  - *Example*: `price` in `[100, 500]`, `age` > 18.
- **DateTimeRangeQuery**: Specialized range query for timestamps.
  - *Example*: `created_at` in `[2023-01-01, 2023-12-31]`.

### Positional Queries
- **PhraseQuery**: Matches an exact sequence of terms. "Slop" allows for some distance/permutation.
  - *Example*: "distributed search engine" (slop 0), "search distributed" (slop 2).
- **SpanQuery**: Advanced control over term positions.
  - `SpanTerm`: Basic unit.
  - `SpanNear`: Matches spans within a certain distance.
  - `SpanOr`: Union of spans.
  - `SpanNot`: Exclude matches if another span overlaps.

### Geospatial (Requires `geo` feature)
- **GeoDistanceQuery**: Matches points within a radius from a center point.
- **GeoBoundingBoxQuery**: Matches points within a rectangular area.

*Both queries are optimized using a 2D BKD tree for fast candidate retrieval.*

### Complex Queries
- **MultiFieldQuery**: Executes a search across several fields simultaneously.
  - *Strategies*: Matches can be combined using `BestFields` (highest score wins) or `MostFields` (scores are summed).
  - *Example*: Search "query string" across "title^2" and "content".
- **AdvancedQuery**: A high-level wrapper for complex query orchestration.
  - *Features*: Supports query-level boosts, minimum score thresholds, and tiered filtering (Must, MustNot, Post-Filtering).
  - *Optimization*: Automatically optimizes query structure and handles execution timeouts.
