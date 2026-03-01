# Spelling Correction

Laurus includes a built-in spelling correction system that can suggest corrections for misspelled query terms and provide "Did you mean?" functionality.

## Overview

The spelling corrector uses edit distance (Levenshtein distance) combined with word frequency data to suggest corrections. It supports:

- **Word-level suggestions** — correct individual misspelled words
- **Auto-correction** — automatically apply high-confidence corrections
- **"Did you mean?"** — suggest alternative queries to the user
- **Query learning** — improve suggestions by learning from user queries
- **Custom dictionaries** — use your own word lists

## Basic Usage

### SpellingCorrector

```rust
use laurus::spelling::corrector::SpellingCorrector;

// Create a corrector with the built-in English dictionary
let mut corrector = SpellingCorrector::new();

// Correct a query
let result = corrector.correct("programing langauge");

// Check if suggestions are available
if result.has_suggestions() {
    for (word, suggestions) in &result.word_suggestions {
        println!("'{}' -> {:?}", word, suggestions);
    }
}

// Get the best corrected query
if let Some(corrected) = result.query() {
    println!("Corrected: {}", corrected);
}
```

### "Did You Mean?"

The `DidYouMean` wrapper provides a higher-level interface for search UIs:

```rust
use laurus::spelling::corrector::{SpellingCorrector, DidYouMean};

let corrector = SpellingCorrector::new();
let mut did_you_mean = DidYouMean::new(corrector);

if let Some(suggestion) = did_you_mean.suggest("programing") {
    println!("Did you mean: {}?", suggestion);
}
```

## Configuration

Use `CorrectorConfig` to customize behavior:

```rust
use laurus::spelling::corrector::{CorrectorConfig, SpellingCorrector};

let config = CorrectorConfig {
    max_distance: 2,              // Maximum edit distance (default: 2)
    max_suggestions: 5,           // Max suggestions per word (default: 5)
    min_frequency: 1,             // Minimum word frequency threshold (default: 1)
    auto_correct: false,          // Enable auto-correction (default: false)
    auto_correct_threshold: 0.8,  // Confidence threshold for auto-correction (default: 0.8)
    use_index_terms: true,        // Use indexed terms as dictionary (default: true)
    learn_from_queries: true,     // Learn from user queries (default: true)
};
```

### Configuration Options

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `max_distance` | `usize` | `2` | Maximum Levenshtein edit distance for candidate suggestions |
| `max_suggestions` | `usize` | `5` | Maximum number of suggestions returned per word |
| `min_frequency` | `u32` | `1` | Minimum frequency a word must have in the dictionary to be suggested |
| `auto_correct` | `bool` | `false` | When true, automatically apply corrections above the threshold |
| `auto_correct_threshold` | `f64` | `0.8` | Confidence score (0.0–1.0) required for auto-correction |
| `use_index_terms` | `bool` | `true` | Use terms from the search index as dictionary words |
| `learn_from_queries` | `bool` | `true` | Learn new words from user search queries |

## CorrectionResult

The `correct()` method returns a `CorrectionResult` with detailed information:

| Field | Type | Description |
| :--- | :--- | :--- |
| `original` | `String` | The original query string |
| `corrected` | `Option<String>` | The corrected query (if auto-correction was applied) |
| `word_suggestions` | `HashMap<String, Vec<Suggestion>>` | Suggestions grouped by misspelled word |
| `confidence` | `f64` | Overall confidence score (0.0–1.0) |
| `auto_corrected` | `bool` | Whether auto-correction was applied |

### Helper Methods

| Method | Returns | Description |
| :--- | :--- | :--- |
| `has_suggestions()` | `bool` | True if any word has suggestions |
| `best_suggestion()` | `Option<&Suggestion>` | The single highest-scoring suggestion |
| `query()` | `Option<String>` | The corrected query string, if corrections were made |
| `should_show_did_you_mean()` | `bool` | Whether to display a "Did you mean?" prompt |

## Custom Dictionaries

You can provide your own dictionary instead of using the built-in English one:

```rust
use laurus::spelling::corrector::SpellingCorrector;
use laurus::spelling::dictionary::SpellingDictionary;

// Build a custom dictionary
let mut dictionary = SpellingDictionary::new();
dictionary.add_word("elasticsearch", 100);
dictionary.add_word("lucene", 80);
dictionary.add_word("laurus", 90);

let corrector = SpellingCorrector::with_dictionary(dictionary);
```

## Learning from Index Terms

When `use_index_terms` is enabled, the corrector can learn from terms in your search index:

```rust
let mut corrector = SpellingCorrector::new();

// Feed index terms to the corrector
let index_terms = vec!["rust", "programming", "search", "engine"];
corrector.learn_from_terms(&index_terms);
```

This improves suggestion quality by incorporating domain-specific vocabulary.

## Statistics

Monitor the corrector's state with `stats()`:

```rust
let stats = corrector.stats();
println!("Dictionary words: {}", stats.dictionary_words);
println!("Total frequency: {}", stats.dictionary_total_frequency);
println!("Learned queries: {}", stats.queries_learned);
```

## Next Steps

- [Lexical Search](lexical_search.md) — full-text search with query types
- [Query DSL](../advanced/query_dsl.md) — human-readable query syntax
