# Lexical Search

Lexical search finds documents by matching keywords against an inverted index. Laurus provides a rich set of query types that cover exact matching, phrase matching, fuzzy matching, and more.

## Basic Usage

```rust
use laurus::{SearchRequestBuilder, LexicalSearchRequest};
use laurus::lexical::TermQuery;

let request = SearchRequestBuilder::new()
    .lexical_search_request(
        LexicalSearchRequest::new(
            Box::new(TermQuery::new("body", "rust"))
        )
    )
    .limit(10)
    .build();

let results = engine.search(request).await?;
```

## Query Types

### TermQuery

Matches documents containing an exact term in a specific field.

```rust
use laurus::lexical::TermQuery;

// Find documents where "body" contains the term "rust"
let query = TermQuery::new("body", "rust");
```

> **Note:** Terms are matched after analysis. If the field uses `StandardAnalyzer`, both the indexed text and the query term are lowercased, so `TermQuery::new("body", "rust")` will match "Rust" in the original text.

### PhraseQuery

Matches documents containing an exact sequence of terms.

```rust
use laurus::lexical::query::phrase::PhraseQuery;

// Find documents containing the exact phrase "machine learning"
let query = PhraseQuery::new("body", vec!["machine".to_string(), "learning".to_string()]);

// Or use the convenience method from a phrase string:
let query = PhraseQuery::from_phrase("body", "machine learning");
```

Phrase queries require term positions to be stored (the default for `TextOption`).

### BooleanQuery

Combines multiple queries with boolean logic.

```rust
use laurus::lexical::query::boolean::{BooleanQuery, BooleanQueryBuilder, Occur};

let query = BooleanQueryBuilder::new()
    .must(Box::new(TermQuery::new("body", "rust")))       // AND
    .must(Box::new(TermQuery::new("body", "programming"))) // AND
    .must_not(Box::new(TermQuery::new("body", "python")))  // NOT
    .build();
```

| Occur | Meaning | DSL Equivalent |
| :--- | :--- | :--- |
| `Must` | Document MUST match | `+term` or `AND` |
| `Should` | Document SHOULD match (boosts score) | `term` or `OR` |
| `MustNot` | Document MUST NOT match | `-term` or `NOT` |
| `Filter` | MUST match, but does not affect score | (no DSL equivalent) |

### FuzzyQuery

Matches terms within a specified edit distance (Levenshtein distance).

```rust
use laurus::lexical::query::fuzzy::FuzzyQuery;

// Find documents matching "programing" within edit distance 2
// This will match "programming", "programing", etc.
let query = FuzzyQuery::new("body", "programing");  // default max_edits = 2
```

### WildcardQuery

Matches terms using wildcard patterns.

```rust
use laurus::lexical::query::wildcard::WildcardQuery;

// '?' matches exactly one character, '*' matches zero or more
let query = WildcardQuery::new("filename", "*.pdf")?;
let query = WildcardQuery::new("body", "pro*")?;
let query = WildcardQuery::new("body", "col?r")?;  // matches "color" and "colour"
```

### PrefixQuery

Matches documents containing terms that start with a specific prefix.

```rust
use laurus::lexical::query::prefix::PrefixQuery;

// Find documents where "body" contains terms starting with "pro"
// This matches "programming", "program", "production", etc.
let query = PrefixQuery::new("body", "pro");
```

### RegexpQuery

Matches documents containing terms that match a regular expression pattern.

```rust
use laurus::lexical::query::regexp::RegexpQuery;

// Find documents where "body" contains terms matching the regex
let query = RegexpQuery::new("body", "^pro.*ing$")?;

// Match version-like patterns
let query = RegexpQuery::new("version", r"^v\d+\.\d+")?;
```

> **Note:** `RegexpQuery::new()` returns `Result` because the regex pattern is validated at construction time. Invalid patterns will produce an error.

### NumericRangeQuery

Matches documents with numeric field values within a range.

```rust
use laurus::lexical::NumericRangeQuery;
use laurus::lexical::core::field::NumericType;

// Find documents where "price" is between 10.0 and 100.0 (inclusive)
let query = NumericRangeQuery::new(
    "price",
    NumericType::Float,
    Some(10.0),   // min
    Some(100.0),  // max
    true,         // include min
    true,         // include max
);

// Open-ended range: price >= 50
let query = NumericRangeQuery::new(
    "price",
    NumericType::Float,
    Some(50.0),
    None,     // no upper bound
    true,
    false,
);
```

### GeoQuery

Matches documents by geographic location.

```rust
use laurus::lexical::query::geo::GeoQuery;

// Find documents within 10km of Tokyo Station (35.6812, 139.7671)
let query = GeoQuery::within_radius("location", 35.6812, 139.7671, 10.0)?; // radius in kilometers

// Find documents within a bounding box (min_lat, min_lon, max_lat, max_lon)
let query = GeoQuery::within_bounding_box(
    "location",
    35.0, 139.0,  // min (lat, lon)
    36.0, 140.0,  // max (lat, lon)
)?;
```

### SpanQuery

Matches terms based on their proximity within a document. Use `SpanTermQuery` and `SpanNearQuery` to build proximity queries:

```rust
use laurus::lexical::query::span::{SpanQuery, SpanTermQuery, SpanNearQuery};

// Find documents where "quick" appears near "fox" (within 3 positions)
let query = SpanNearQuery::new(
    "body",
    vec![
        Box::new(SpanTermQuery::new("body", "quick")) as Box<dyn SpanQuery>,
        Box::new(SpanTermQuery::new("body", "fox")) as Box<dyn SpanQuery>,
    ],
    3,    // slop (max distance between terms)
    true, // in_order (terms must appear in order)
);
```

## Scoring

Lexical search results are scored using **BM25**. The score reflects how relevant a document is to the query:

- Higher term frequency in the document increases the score
- Rarer terms across the index increase the score
- Shorter documents are boosted relative to longer ones

### Field Boosts

You can boost specific fields to influence relevance:

```rust
use laurus::LexicalSearchRequest;

let mut request = LexicalSearchRequest::new(Box::new(query));
request.field_boosts.insert("title".to_string(), 2.0);  // title matches count double
request.field_boosts.insert("body".to_string(), 1.0);
```

## LexicalSearchRequest Options

| Option | Default | Description |
| :--- | :--- | :--- |
| `query` | (required) | The query to execute |
| `limit` | 10 | Maximum number of results |
| `load_documents` | true | Whether to load full document content |
| `min_score` | 0.0 | Minimum score threshold |
| `timeout_ms` | None | Search timeout in milliseconds |
| `parallel` | false | Enable parallel search across segments |
| `sort_by` | `Score` | Sort by relevance score, or by a field (`asc` / `desc`) |
| `field_boosts` | empty | Per-field score multipliers |

### Builder Methods

`LexicalSearchRequest` supports a builder-style API for setting options:

```rust
use laurus::LexicalSearchRequest;
use laurus::lexical::TermQuery;

let request = LexicalSearchRequest::new(Box::new(TermQuery::new("body", "rust")))
    .limit(20)
    .min_score(0.5)
    .timeout_ms(5000)
    .parallel(true)
    .sort_by_field_desc("date")
    .with_field_boost("title", 2.0)
    .with_field_boost("body", 1.0);
```

## Using the Query DSL

Instead of building queries programmatically, you can use the text-based Query DSL:

```rust
use laurus::lexical::QueryParser;
use laurus::analysis::analyzer::standard::StandardAnalyzer;
use std::sync::Arc;

let analyzer = Arc::new(StandardAnalyzer::default());
let parser = QueryParser::new(analyzer).with_default_field("body");

// Simple term
let query = parser.parse("rust")?;

// Boolean
let query = parser.parse("rust AND programming")?;

// Phrase
let query = parser.parse("\"machine learning\"")?;

// Field-specific
let query = parser.parse("title:rust AND body:programming")?;

// Fuzzy
let query = parser.parse("programing~2")?;

// Range
let query = parser.parse("year:[2020 TO 2024]")?;
```

See [Query DSL](../advanced/query_dsl.md) for the complete syntax reference.

## Next Steps

- Semantic similarity search: [Vector Search](vector_search.md)
- Combine lexical + vector: [Hybrid Search](hybrid_search.md)
- Full DSL syntax reference: [Query DSL](../advanced/query_dsl.md)
