# Query DSL

Iris provides a unified query DSL (Domain Specific Language) that allows lexical (keyword) and vector (semantic) search in a single query string. The `UnifiedQueryParser` splits the input into lexical and vector portions and delegates to the appropriate sub-parser.

## Overview

```text
title:hello AND content:~"cute kitten"^0.8
|--- lexical --|    |--- vector --------|
```

The `~"` pattern distinguishes vector clauses from lexical clauses. Everything else is treated as a lexical query.

## Lexical Query Syntax

Lexical queries search the inverted index using exact or approximate keyword matching.

### Term Query

Match a single term against a field (or the default field):

```text
hello
title:hello
```

### Boolean Operators

Combine clauses with `AND` and `OR` (case-insensitive):

```text
title:hello AND body:world
title:hello OR title:goodbye
```

Space-separated clauses without an explicit operator use implicit boolean (behaves like OR with scoring).

### Required / Prohibited Clauses

Use `+` (must match) and `-` (must not match):

```text
+title:hello -title:goodbye
```

### Phrase Query

Match an exact phrase using double quotes. Optional proximity (`~N`) allows N words between terms:

```text
"hello world"
"hello world"~2
```

### Fuzzy Query

Approximate matching with edit distance. Append `~` and optionally the maximum edit distance:

```text
roam~
roam~2
```

### Wildcard Query

Use `?` (single character) and `*` (zero or more characters):

```text
te?t
test*
```

### Range Query

Inclusive `[]` or exclusive `{}` ranges, useful for numeric and date fields:

```text
price:[100 TO 500]
date:{2024-01-01 TO 2024-12-31}
price:[* TO 100]
```

### Boost

Increase the weight of a clause with `^`:

```text
title:hello^2
"important phrase"^1.5
```

### Grouping

Use parentheses for sub-expressions:

```text
(title:hello OR title:hi) AND body:world
```

### PEG Grammar

The full lexical grammar ([parser.pest](https://github.com/mosuka/iris/blob/main/src/lexical/query/parser.pest)):

```pest
query          = { SOI ~ boolean_query ~ EOI }
boolean_query  = { clause ~ (boolean_op ~ clause | clause)* }
clause         = { required_clause | prohibited_clause | sub_clause }
required_clause   = { "+" ~ sub_clause }
prohibited_clause = { "-" ~ sub_clause }
sub_clause     = { grouped_query | field_query | term_query }
grouped_query  = { "(" ~ boolean_query ~ ")" ~ boost? }
boolean_op     = { ^"AND" | ^"OR" }
field_query    = { field ~ ":" ~ field_value }
field_value    = { range_query | phrase_query | fuzzy_term
                 | wildcard_term | simple_term }
phrase_query   = { "\"" ~ phrase_content ~ "\"" ~ proximity? ~ boost? }
proximity      = { "~" ~ number }
fuzzy_term     = { term ~ "~" ~ fuzziness? ~ boost? }
wildcard_term  = { wildcard_pattern ~ boost? }
simple_term    = { term ~ boost? }
boost          = { "^" ~ boost_value }
```

## Vector Query Syntax

Vector queries embed text into vectors at parse time and perform similarity search.

### Basic Syntax

```text
field:~"text"
field:~"text"^weight
```

| Element | Required | Description | Example |
| :--- | :---: | :--- | :--- |
| `field:` | No | Target vector field name | `content:` |
| `~` | **Yes** | Vector query marker | |
| `"text"` | **Yes** | Text to embed | `"cute kitten"` |
| `^weight` | No | Score weight (default: 1.0) | `^0.8` |

### Examples

```text
# Single field
content:~"cute kitten"

# With boost weight
content:~"cute kitten"^0.8

# Default field (when configured)
~"cute kitten"

# Multiple clauses
content:~"cats" image:~"dogs"^0.5

# Nested field name (dot notation)
metadata.embedding:~"text"
```

### Multiple Clauses

Multiple vector clauses are space-separated. All clauses are executed and their scores are combined using the `score_mode` (default: `WeightedSum`):

```text
content:~"cats" image:~"dogs"^0.5
```

This produces:

```text
score = similarity("cats", content) * 1.0
      + similarity("dogs", image)   * 0.5
```

There are no `AND`/`OR` operators in the vector DSL. Vector search is inherently a ranking operation, and the weight (`^`) controls the contribution of each clause.

### Score Modes

| Mode | Description |
| :--- | :--- |
| `WeightedSum` (default) | Sum of (similarity * weight) across all clauses |
| `MaxSim` | Maximum similarity score across clauses |
| `LateInteraction` | Late interaction scoring |

Score mode cannot be set from DSL syntax. Use the Rust API to override:

```rust
let mut request = parser.parse(r#"content:~"cats" image:~"dogs""#).await?;
request.score_mode = VectorScoreMode::MaxSim;
```

### PEG Grammar

The full vector grammar ([parser.pest](https://github.com/mosuka/iris/blob/main/src/vector/query/parser.pest)):

```pest
query          = { SOI ~ vector_clause+ ~ EOI }
vector_clause  = { field_prefix? ~ "~" ~ quoted_text ~ boost? }
field_prefix   = { field_name ~ ":" }
field_name     = @{ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_" | ".")* }
quoted_text    = ${ "\"" ~ inner_text ~ "\"" }
inner_text     = @{ (!("\"") ~ ANY)* }
boost          = { "^" ~ float_value }
float_value    = @{ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
```

## Unified (Hybrid) Query Syntax

The `UnifiedQueryParser` allows mixing lexical and vector clauses freely in a single query string:

```
title:hello content:~"cute kitten"^0.8
```

### How It Works

1. **Split**: Vector clauses (matching `field:~"text"^boost` pattern) are extracted via regex.
2. **Delegate**: Vector portion goes to `VectorQueryParser`, remainder goes to lexical `QueryParser`.
3. **Fuse**: If both lexical and vector results exist, they are combined using a fusion algorithm.

### Disambiguation

The `~"` pattern unambiguously identifies vector clauses because in lexical syntax, `~` only appears _after_ a term or phrase (e.g., `roam~2`, `"hello world"~10`), never before a quote.

### Fusion Algorithms

When a query contains both lexical and vector clauses, results are fused:

| Algorithm | Formula | Description |
| :--- | :--- | :--- |
| **RRF** (default) | `score = sum(1 / (k + rank))` | Reciprocal Rank Fusion. Robust to different score distributions. Default k=60. |
| **WeightedSum** | `score = lexical * a + vector * b` | Linear combination with configurable weights. |

> **Note**: The fusion algorithm cannot be specified in the DSL syntax. It is configured when constructing the `UnifiedQueryParser` via `.with_fusion()`. The default is RRF (k=60). See [Custom Fusion](#custom-fusion) for a code example.

### Examples

```
# Lexical only — no fusion
title:hello AND body:world

# Vector only — no fusion
content:~"cute kitten"

# Hybrid — fusion applied automatically
title:hello content:~"cute kitten"

# Hybrid with boolean operators
title:hello AND category:animal content:~"cute kitten"^0.8

# Multiple vector clauses + lexical
category:animal content:~"cats" image:~"dogs"^0.5

# Default fields (when configured)
hello ~"cats"
```

## Code Examples

### Lexical Search with DSL

```rust
use std::sync::Arc;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::query::QueryParser;

let analyzer = Arc::new(StandardAnalyzer::new()?);
let parser = QueryParser::new(analyzer)
    .with_default_field("title");

let query = parser.parse("title:hello AND body:world")?;
```

### Vector Search with DSL

```rust
use std::sync::Arc;
use iris::vector::query::VectorQueryParser;

let parser = VectorQueryParser::new(embedder)
    .with_default_field("content");

let request = parser.parse(r#"content:~"cute kitten"^0.8"#).await?;
```

### Hybrid Search with Unified DSL

```rust
use iris::engine::query::UnifiedQueryParser;

let unified = UnifiedQueryParser::new(lexical_parser, vector_parser);

let request = unified.parse(
    r#"title:hello content:~"cute kitten"^0.8"#
).await?;
// request.lexical  -> Some(...)  — lexical query
// request.vector   -> Some(...)  — vector query
// request.fusion   -> Some(RRF)  — fusion algorithm
```

### Custom Fusion

```rust
use iris::engine::search::FusionAlgorithm;

let unified = UnifiedQueryParser::new(lexical_parser, vector_parser)
    .with_fusion(FusionAlgorithm::WeightedSum {
        lexical_weight: 0.3,
        vector_weight: 0.7,
    });
```
