//! Configuration types for custom analyzer definitions within a schema.
//!
//! These types allow users to declaratively define custom text analyzers
//! composed of a tokenizer, optional char filters, and optional token
//! filters. Definitions are stored in the schema's `analyzers` map and
//! referenced by name from [`TextOption::analyzer`].
//!
//! # JSON Format
//!
//! ```json
//! {
//!   "char_filters": [{"type": "unicode_normalization", "form": "nfkc"}],
//!   "tokenizer": {"type": "regex", "pattern": "\\w+"},
//!   "token_filters": [{"type": "lowercase"}, {"type": "stop"}]
//! }
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A custom analyzer definition composed of a tokenizer and optional
/// char/token filter chains.
///
/// # Fields
///
/// * `char_filters` - Applied to raw text before tokenization.
/// * `tokenizer` - Splits text into tokens.
/// * `token_filters` - Applied sequentially to the token stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerDefinition {
    /// Char filters applied to raw text before tokenization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub char_filters: Vec<CharFilterConfig>,

    /// The tokenizer that splits text into tokens.
    pub tokenizer: TokenizerConfig,

    /// Token filters applied to the token stream after tokenization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub token_filters: Vec<TokenFilterConfig>,
}

/// Configuration for a tokenizer component.
///
/// Uses `{"type": "..."}` JSON format via serde's internally tagged
/// representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TokenizerConfig {
    /// Splits on whitespace boundaries.
    Whitespace,

    /// Splits on Unicode word boundaries.
    UnicodeWord,

    /// Splits using a regular expression pattern.
    Regex {
        /// The regex pattern (default: `\w+`).
        #[serde(default = "default_regex_pattern")]
        pattern: String,

        /// If `true`, the pattern matches gaps between tokens
        /// rather than the tokens themselves.
        #[serde(default)]
        gaps: bool,
    },

    /// Produces n-grams of the specified size range.
    Ngram {
        /// Minimum n-gram size.
        min_gram: usize,
        /// Maximum n-gram size.
        max_gram: usize,
    },

    /// Morphological tokenizer using Lindera.
    Lindera {
        /// Tokenization mode: `"normal"`, `"search"`, or `"decompose"`.
        mode: String,
        /// Dictionary URI (e.g. `"embedded://ipadic"`).
        dict: String,
        /// Optional user dictionary URI.
        #[serde(default)]
        user_dict: Option<String>,
    },

    /// Treats the entire input as a single token.
    Whole,
}

/// Configuration for a char filter component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CharFilterConfig {
    /// Applies Unicode normalization (NFC, NFD, NFKC, or NFKD).
    UnicodeNormalization {
        /// Normalization form: `"nfc"`, `"nfd"`, `"nfkc"`, or `"nfkd"`.
        form: String,
    },

    /// Replaces text matching a regex pattern.
    PatternReplace {
        /// The regex pattern to match.
        pattern: String,
        /// The replacement string.
        replacement: String,
    },

    /// Replaces strings using a mapping dictionary.
    Mapping {
        /// Key-value pairs for replacement.
        mapping: HashMap<String, String>,
    },

    /// Expands Japanese iteration marks (踊り字).
    JapaneseIterationMark {
        /// Whether to normalize kanji iteration marks.
        #[serde(default = "default_true")]
        kanji: bool,
        /// Whether to normalize kana iteration marks.
        #[serde(default = "default_true")]
        kana: bool,
    },
}

/// Configuration for a token filter component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TokenFilterConfig {
    /// Converts tokens to lowercase.
    Lowercase,

    /// Removes stop words from the token stream.
    Stop {
        /// Custom stop word list. If `None`, uses default English
        /// stop words.
        #[serde(default)]
        words: Option<Vec<String>>,
    },

    /// Applies stemming to tokens.
    Stem {
        /// Stemmer type: `"porter"` (default), `"simple"`, or
        /// `"identity"`.
        #[serde(default)]
        stem_type: Option<String>,
    },

    /// Multiplies token scores by a boost factor.
    Boost {
        /// The boost multiplier.
        boost: f32,
    },

    /// Limits the number of tokens in the stream.
    Limit {
        /// Maximum number of tokens to emit.
        limit: usize,
    },

    /// Strips leading and trailing whitespace from tokens.
    Strip,

    /// Removes empty tokens from the stream.
    RemoveEmpty,

    /// Flattens a synonym graph into a linear token stream.
    FlattenGraph,
}

fn default_regex_pattern() -> String {
    r"\w+".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_definition_serde_roundtrip() {
        let def = AnalyzerDefinition {
            char_filters: vec![CharFilterConfig::UnicodeNormalization {
                form: "nfkc".into(),
            }],
            tokenizer: TokenizerConfig::Regex {
                pattern: r"\w+".into(),
                gaps: false,
            },
            token_filters: vec![
                TokenFilterConfig::Lowercase,
                TokenFilterConfig::Stop {
                    words: Some(vec!["the".into(), "a".into()]),
                },
                TokenFilterConfig::Stem { stem_type: None },
            ],
        };

        let json = serde_json::to_string(&def).unwrap();
        let deserialized: AnalyzerDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token_filters.len(), 3);
        assert_eq!(deserialized.char_filters.len(), 1);
    }

    #[test]
    fn test_tokenizer_config_variants() {
        let configs = vec![
            r#"{"type": "whitespace"}"#,
            r#"{"type": "unicode_word"}"#,
            r#"{"type": "regex", "pattern": "\\w+", "gaps": false}"#,
            r#"{"type": "ngram", "min_gram": 2, "max_gram": 3}"#,
            r#"{"type": "whole"}"#,
        ];
        for json in configs {
            let config: TokenizerConfig = serde_json::from_str(json).unwrap();
            let serialized = serde_json::to_string(&config).unwrap();
            let _roundtrip: TokenizerConfig = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_char_filter_config_variants() {
        let configs = vec![
            r#"{"type": "unicode_normalization", "form": "nfkc"}"#,
            r#"{"type": "pattern_replace", "pattern": "foo", "replacement": "bar"}"#,
            r#"{"type": "mapping", "mapping": {"a": "b"}}"#,
            r#"{"type": "japanese_iteration_mark"}"#,
        ];
        for json in configs {
            let config: CharFilterConfig = serde_json::from_str(json).unwrap();
            let serialized = serde_json::to_string(&config).unwrap();
            let _roundtrip: CharFilterConfig = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_token_filter_config_variants() {
        let configs = vec![
            r#"{"type": "lowercase"}"#,
            r#"{"type": "stop"}"#,
            r#"{"type": "stop", "words": ["the", "a"]}"#,
            r#"{"type": "stem"}"#,
            r#"{"type": "stem", "stem_type": "porter"}"#,
            r#"{"type": "boost", "boost": 2.0}"#,
            r#"{"type": "limit", "limit": 100}"#,
            r#"{"type": "strip"}"#,
            r#"{"type": "remove_empty"}"#,
            r#"{"type": "flatten_graph"}"#,
        ];
        for json in configs {
            let config: TokenFilterConfig = serde_json::from_str(json).unwrap();
            let serialized = serde_json::to_string(&config).unwrap();
            let _roundtrip: TokenFilterConfig = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_full_schema_with_analyzers_json() {
        let json = r#"{
            "char_filters": [{"type": "unicode_normalization", "form": "nfkc"}],
            "tokenizer": {"type": "lindera", "mode": "normal", "dict": "embedded://ipadic"},
            "token_filters": [{"type": "lowercase"}]
        }"#;
        let def: AnalyzerDefinition = serde_json::from_str(json).unwrap();
        assert!(matches!(def.tokenizer, TokenizerConfig::Lindera { .. }));
    }

    #[test]
    fn test_minimal_definition() {
        let json = r#"{"tokenizer": {"type": "whitespace"}}"#;
        let def: AnalyzerDefinition = serde_json::from_str(json).unwrap();
        assert!(def.char_filters.is_empty());
        assert!(def.token_filters.is_empty());
    }
}
