# Text Analysis

Text analysis is the process of converting raw text into searchable tokens. When a document is indexed, the analyzer breaks text fields into individual terms; when a query is executed, the same analyzer processes the query text to ensure consistency.

## The Analysis Pipeline

<div class="mermaid">
graph LR
    Input["Raw Text<br/>'The quick brown FOX jumps!'"]
    T["Tokenizer<br/>Split into words"]
    F1["LowercaseFilter"]
    F2["StopFilter"]
    F3["StemFilter"]
    Output["Terms<br/>'quick', 'brown', 'fox', 'jump'"]

    Input --> T --> F1 --> F2 --> F3 --> Output
</div>

The analysis pipeline consists of:

1. **Tokenizer** — splits text into raw tokens (words, characters, n-grams)
2. **Token Filters** — transform, remove, or expand tokens (lowercase, stop words, stemming, synonyms)

## The Analyzer Trait

All analyzers implement the `Analyzer` trait:

```rust
pub trait Analyzer: Send + Sync {
    fn analyze(&self, text: &str) -> Result<TokenStream>;
}
```

`TokenStream` is a `Box<dyn Iterator<Item = Token> + Send>` — a lazy iterator over tokens.

A `Token` contains:

| Field | Type | Description |
| :--- | :--- | :--- |
| `text` | `String` | The token text |
| `position` | `usize` | Position in the original text |
| `position_increment` | `usize` | Distance from previous token |
| `position_length` | `usize` | Span of the token (>1 for synonyms) |
| `boost` | `f32` | Token-level scoring weight |

## Built-in Analyzers

### StandardAnalyzer

The default analyzer. Suitable for most Western languages.

Pipeline: `RegexTokenizer` (Unicode word boundaries) → `LowercaseFilter` → `StopFilter` (33 common English stop words)

```rust
use iris::analysis::analyzer::standard::StandardAnalyzer;

let analyzer = StandardAnalyzer::default();
// "The Quick Brown Fox" → ["quick", "brown", "fox"]
// ("The" is removed by stop word filtering)
```

### JapaneseAnalyzer

Uses morphological analysis for Japanese text segmentation.

Pipeline: `LinderaTokenizer` → `LowercaseFilter`

```rust
use iris::analysis::analyzer::japanese::JapaneseAnalyzer;

let analyzer = JapaneseAnalyzer::new()?;
// "東京都に住んでいる" → ["東京", "都", "に", "住ん", "で", "いる"]
```

### KeywordAnalyzer

Treats the entire input as a single token. No tokenization or normalization.

```rust
use iris::analysis::analyzer::keyword::KeywordAnalyzer;

let analyzer = KeywordAnalyzer;
// "Hello World" → ["Hello World"]
```

Use this for fields that should match exactly (categories, tags, status codes).

### PipelineAnalyzer

Build a custom pipeline by combining any tokenizer with any sequence of filters:

```rust
use iris::analysis::analyzer::pipeline::PipelineAnalyzer;
use iris::analysis::tokenizer::regex::RegexTokenizer;
use iris::analysis::token_filter::lowercase::LowercaseFilter;
use iris::analysis::token_filter::stop::StopFilter;
use iris::analysis::token_filter::stem::StemFilter;

let analyzer = PipelineAnalyzer::new(Arc::new(RegexTokenizer::new()?))
    .add_filter(Arc::new(LowercaseFilter::new()))
    .add_filter(Arc::new(StopFilter::new()))
    .add_filter(Arc::new(StemFilter::english()));
```

## PerFieldAnalyzer

`PerFieldAnalyzer` lets you assign different analyzers to different fields within the same engine:

<div class="mermaid">
graph LR
    PFA["PerFieldAnalyzer"]
    PFA -->|"title"| KW["KeywordAnalyzer"]
    PFA -->|"body"| STD["StandardAnalyzer"]
    PFA -->|"description_ja"| JP["JapaneseAnalyzer"]
    PFA -->|other fields| DEF["Default<br/>(StandardAnalyzer)"]
</div>

```rust
use std::sync::Arc;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::per_field::PerFieldAnalyzer;

// Default analyzer for fields not explicitly configured
let mut per_field = PerFieldAnalyzer::new(
    Arc::new(StandardAnalyzer::default())
);

// Use KeywordAnalyzer for exact-match fields
per_field.add_analyzer("category", Arc::new(KeywordAnalyzer));
per_field.add_analyzer("status", Arc::new(KeywordAnalyzer));

let engine = Engine::builder(storage, schema)
    .analyzer(Arc::new(per_field))
    .build()
    .await?;
```

> **Note:** The `_id` field is always analyzed with `KeywordAnalyzer` regardless of configuration.

## Tokenizers

| Tokenizer | Description |
| :--- | :--- |
| `RegexTokenizer` | Unicode word boundaries; splits on whitespace and punctuation |
| `WhitespaceTokenizer` | Splits on whitespace only |
| `LinderaTokenizer` | Japanese morphological analysis (Lindera/MeCab) |
| `NgramTokenizer` | Generates n-gram tokens of configurable size |

## Token Filters

| Filter | Description |
| :--- | :--- |
| `LowercaseFilter` | Converts tokens to lowercase |
| `StopFilter` | Removes common words ("the", "is", "a") |
| `StemFilter` | Reduces words to their root form ("running" → "run") |
| `SynonymGraphFilter` | Expands tokens with synonyms from a dictionary |

### Synonym Expansion

The `SynonymGraphFilter` expands terms using a synonym dictionary:

```rust
use iris::analysis::synonym::dictionary::SynonymDictionary;
use iris::analysis::token_filter::synonym_graph::SynonymGraphFilter;

let mut dict = SynonymDictionary::new(None)?;
dict.add_synonym_group(vec!["ml".into(), "machine learning".into()]);
dict.add_synonym_group(vec!["ai".into(), "artificial intelligence".into()]);

// keep_original=true means original token is preserved alongside synonyms
let filter = SynonymGraphFilter::new(dict, true)
    .with_boost(0.8);  // synonyms get 80% weight
```

The `boost` parameter controls how much weight synonyms receive relative to original tokens. A value of `0.8` means synonym matches contribute 80% as much to the score as exact matches.
