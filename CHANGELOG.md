# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-18

### Added

- **Lexical search** powered by an inverted index with BM25 scoring
  - Query types: TermQuery, PhraseQuery, BooleanQuery, FuzzyQuery, WildcardQuery, PrefixQuery, RegexpQuery, NumericRangeQuery, GeoQuery, SpanQuery
  - Text-based query DSL with boolean operators, phrase matching, fuzzy matching, and field-specific queries
  - Per-field score boosting
- **Vector search** using HNSW (Hierarchical Navigable Small World) index
  - Distance metrics: Cosine, Euclidean, Dot Product
  - Payload filtering and multi-vector queries with configurable score modes
- **Hybrid search** combining lexical and vector results
  - Fusion algorithms: RRF (Reciprocal Rank Fusion), linear combination, and weighted scoring
- **Text analysis pipeline** with composable components
  - Analyzers: StandardAnalyzer, SimpleAnalyzer, KeywordAnalyzer, EnglishAnalyzer, JapaneseAnalyzer, PipelineAnalyzer, PerFieldAnalyzer
  - Tokenizers: RegexTokenizer, UnicodeTokenizer, NGramTokenizer
  - Token filters: LowercaseFilter, StopFilter, StemmerFilter, SynonymGraphFilter, UnicodeNormalizationFilter, LengthFilter
  - Character filters: MappingCharFilter, RegexCharFilter, UnicodeNormalizationCharFilter
- **Embedding support** via optional feature flags
  - `embeddings-candle`: Local CLIP embeddings using Candle
  - `embeddings-openai`: OpenAI API-based embeddings
  - `embeddings-multimodal`: Multimodal (text + image) embeddings
- **Flexible schema system** with typed fields (Text, Keyword, Numeric, Geo, Vector, Bytes)
- **Storage backends**: FileStorage (persistent) and MemoryStorage (in-memory)
- **Document management** with auto-generated or user-specified document IDs
- **Segment-based architecture** with merge policies for index maintenance

[0.1.0]: https://github.com/mosuka/iris/releases/tag/v0.1.0
