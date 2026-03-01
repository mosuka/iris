# Search

This section covers how to query your indexed data. Laurus supports three search modes that can be used independently or combined.

## Topics

### [Lexical Search](search/lexical_search.md)

Keyword-based search using an inverted index. Covers:

- All query types: Term, Phrase, Boolean, Fuzzy, Wildcard, Range, Geo, Span
- BM25 scoring and field boosts
- Using the Query DSL for text-based queries

### [Vector Search](search/vector_search.md)

Semantic similarity search using vector embeddings. Covers:

- VectorSearchRequestBuilder API
- Multi-field vector search and score modes
- Filtered vector search

### [Hybrid Search](search/hybrid_search.md)

Combining lexical and vector search for best-of-both-worlds results. Covers:

- SearchRequestBuilder API
- Fusion algorithms (RRF, WeightedSum)
- Filtered hybrid search
- Pagination with offset/limit

### [Spelling Correction](search/spelling_correction.md)

Suggest corrections for misspelled query terms. Covers:

- SpellingCorrector and "Did you mean?" features
- Custom dictionaries and configuration
- Learning from index terms and user queries
