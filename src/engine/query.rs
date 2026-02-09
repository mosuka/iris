//! Unified query parser for hybrid search.
//!
//! Parses a single DSL string containing both lexical and vector query clauses
//! into a [`SearchRequest`] for the Engine.
//!
//! # Syntax
//!
//! Lexical and vector clauses can be freely mixed in a single query string:
//!
//! - **Lexical**: Standard query syntax (`title:hello`, `"phrase"`, `AND`/`OR`, etc.)
//! - **Vector**: `field:~"text"` syntax with optional boost (`^0.8`)
//!
//! # Examples
//!
//! ```ignore
//! use iris::engine::query::UnifiedQueryParser;
//!
//! let parser = UnifiedQueryParser::new(lexical_parser, vector_parser);
//!
//! // Hybrid search
//! let request = parser.parse(r#"title:hello content:~"cute kitten"^0.8"#)?;
//! assert!(request.lexical.is_some());
//! assert!(request.vector.is_some());
//!
//! // Lexical only
//! let request = parser.parse("title:hello AND body:world")?;
//! assert!(request.lexical.is_some());
//! assert!(request.vector.is_none());
//!
//! // Vector only
//! let request = parser.parse(r#"content:~"cats" image:~"dogs"^0.5"#)?;
//! assert!(request.lexical.is_none());
//! assert!(request.vector.is_some());
//! ```

use regex::Regex;

use crate::engine::search::{FusionAlgorithm, SearchRequest};
use crate::error::{IrisError, Result};
use crate::lexical::query::parser::QueryParser;
use crate::vector::query::parser::VectorQueryParser;

/// Unified query parser that composes lexical and vector parsers.
///
/// Parses a single DSL string into a [`SearchRequest`] by splitting the input
/// into lexical and vector portions and delegating to the appropriate sub-parser.
///
/// Vector clauses are identified by the `~"` pattern (tilde immediately before
/// a double quote), which is unambiguous with lexical syntax (where `~` only
/// appears after terms or phrases, e.g. `roam~2`, `"hello world"~10`).
pub struct UnifiedQueryParser {
    lexical_parser: QueryParser,
    vector_parser: VectorQueryParser,
    default_fusion: FusionAlgorithm,
}

impl UnifiedQueryParser {
    /// Create a new UnifiedQueryParser with the given sub-parsers.
    ///
    /// Default fusion algorithm is RRF with k=60.
    pub fn new(lexical_parser: QueryParser, vector_parser: VectorQueryParser) -> Self {
        Self {
            lexical_parser,
            vector_parser,
            default_fusion: FusionAlgorithm::RRF { k: 60.0 },
        }
    }

    /// Set the default fusion algorithm for hybrid queries.
    pub fn with_fusion(mut self, fusion: FusionAlgorithm) -> Self {
        self.default_fusion = fusion;
        self
    }

    /// Parse a unified query string into a SearchRequest.
    ///
    /// The query string may contain both lexical and vector clauses:
    /// - Vector clauses: `field:~"text"`, `~"text"`, `field:~"text"^0.8`
    /// - Lexical clauses: everything else (`title:hello`, `AND`, `"phrase"`, etc.)
    pub fn parse(&self, query_str: &str) -> Result<SearchRequest> {
        let query_str = query_str.trim();
        if query_str.is_empty() {
            return Err(IrisError::invalid_argument(
                "Query string must not be empty",
            ));
        }

        let (lexical_str, vector_str) = self.split_query(query_str);

        let lexical = if let Some(ref s) = lexical_str {
            Some(self.lexical_parser.parse(s)?)
        } else {
            None
        };

        let vector = if let Some(ref s) = vector_str {
            Some(self.vector_parser.parse(s)?)
        } else {
            None
        };

        if lexical.is_none() && vector.is_none() {
            return Err(IrisError::invalid_argument(
                "Query must contain at least one lexical or vector clause",
            ));
        }

        let fusion = if lexical.is_some() && vector.is_some() {
            Some(self.default_fusion)
        } else {
            None
        };

        Ok(SearchRequest {
            lexical,
            vector,
            fusion,
            ..Default::default()
        })
    }

    /// Split a query string into lexical and vector portions.
    ///
    /// Uses regex to identify vector clauses (`field:~"text"^boost` or `~"text"`)
    /// and extracts them. The remainder is treated as lexical query text.
    fn split_query(&self, input: &str) -> (Option<String>, Option<String>) {
        // Pattern: optional field prefix + ~"text" + optional boost
        // - (?:[\w][\w.]*:)? — optional field name with colon
        // - ~"[^"]*"         — tilde + quoted text
        // - (?:\^[\d]+(?:\.[\d]+)?)? — optional boost value
        let vector_re =
            Regex::new(r#"(?:[\w][\w.]*:)?~"[^"]*"(?:\^[\d]+(?:\.[\d]+)?)?"#).unwrap();

        // Collect vector clauses
        let vector_clauses: Vec<&str> = vector_re.find_iter(input).map(|m| m.as_str()).collect();

        // Remove vector clauses from input to get lexical part
        let lexical_raw = vector_re.replace_all(input, " ");
        let lexical_cleaned = clean_lexical_string(&lexical_raw);

        let lexical = if lexical_cleaned.is_empty() {
            None
        } else {
            Some(lexical_cleaned)
        };

        let vector = if vector_clauses.is_empty() {
            None
        } else {
            Some(vector_clauses.join(" "))
        };

        (lexical, vector)
    }
}

/// Clean up a lexical query string after vector clause removal.
///
/// Handles:
/// 1. Collapse multiple whitespace into single space
/// 2. Remove leading/trailing boolean operators (AND, OR)
/// 3. Collapse consecutive boolean operators (AND AND → AND)
fn clean_lexical_string(s: &str) -> String {
    // Collapse multiple whitespace
    let ws_re = Regex::new(r"\s+").unwrap();
    let s = ws_re.replace_all(s, " ");
    let s = s.trim();

    if s.is_empty() {
        return String::new();
    }

    // Split into tokens and filter out dangling boolean operators
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut result: Vec<&str> = Vec::new();

    for token in &tokens {
        let upper = token.to_uppercase();
        if upper == "AND" || upper == "OR" {
            // Only add boolean operator if there's a preceding non-boolean token
            // and the previous token is not already a boolean operator.
            if !result.is_empty() {
                let last_upper = result.last().unwrap().to_uppercase();
                if last_upper != "AND" && last_upper != "OR" {
                    result.push(token);
                }
                // else: skip consecutive boolean operator
            }
            // else: skip leading boolean operator
        } else {
            result.push(token);
        }
    }

    // Remove trailing boolean operator
    if let Some(last) = result.last() {
        let upper = last.to_uppercase();
        if upper == "AND" || upper == "OR" {
            result.pop();
        }
    }

    result.join(" ")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::analysis::analyzer::standard::StandardAnalyzer;
    use crate::data::DataValue;

    fn make_parser() -> UnifiedQueryParser {
        let analyzer = Arc::new(StandardAnalyzer::new().unwrap());
        let lexical = QueryParser::new(analyzer).with_default_field("title");
        let vector = VectorQueryParser::new().with_default_field("content");
        UnifiedQueryParser::new(lexical, vector)
    }

    #[test]
    fn test_lexical_only() {
        let parser = make_parser();
        let request = parser.parse("title:hello").unwrap();

        assert!(request.lexical.is_some());
        assert!(request.vector.is_none());
        assert!(request.fusion.is_none());
    }

    #[test]
    fn test_vector_only() {
        let parser = make_parser();
        let request = parser.parse(r#"content:~"cats""#).unwrap();

        assert!(request.lexical.is_none());
        assert!(request.vector.is_some());
        assert!(request.fusion.is_none());

        let vector = request.vector.unwrap();
        assert_eq!(vector.query_payloads.len(), 1);
        assert_eq!(vector.query_payloads[0].field, "content");
    }

    #[test]
    fn test_hybrid() {
        let parser = make_parser();
        let request = parser.parse(r#"title:hello content:~"cats""#).unwrap();

        assert!(request.lexical.is_some());
        assert!(request.vector.is_some());
        assert!(request.fusion.is_some());

        // Fusion defaults to RRF
        if let Some(FusionAlgorithm::RRF { k }) = request.fusion {
            assert!((k - 60.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected RRF fusion");
        }
    }

    #[test]
    fn test_vector_with_boost() {
        let parser = make_parser();
        let request = parser.parse(r#"content:~"text"^0.8"#).unwrap();

        let vector = request.vector.unwrap();
        assert!((vector.query_payloads[0].weight - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_multiple_vector_clauses() {
        let parser = make_parser();
        let request = parser.parse(r#"a:~"x" b:~"y"^0.5"#).unwrap();

        assert!(request.lexical.is_none());
        let vector = request.vector.unwrap();
        assert_eq!(vector.query_payloads.len(), 2);
        assert_eq!(vector.query_payloads[0].field, "a");
        assert_eq!(vector.query_payloads[1].field, "b");
        assert!((vector.query_payloads[1].weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_lexical_and_with_vector() {
        let parser = make_parser();
        let request = parser
            .parse(r#"title:hello AND title:world content:~"cats""#)
            .unwrap();

        assert!(request.lexical.is_some());
        assert!(request.vector.is_some());
        assert!(request.fusion.is_some());
    }

    #[test]
    fn test_vector_between_and() {
        let parser = make_parser();
        // After removing content:~"cats", we get "title:hello AND AND title:world"
        // which should be cleaned to "title:hello AND title:world"
        let request = parser
            .parse(r#"title:hello AND content:~"cats" AND title:world"#)
            .unwrap();

        assert!(request.lexical.is_some());
        assert!(request.vector.is_some());
    }

    #[test]
    fn test_default_fields() {
        let parser = make_parser();
        // Lexical uses default field "title", vector uses default field "content"
        let request = parser.parse(r#"hello ~"cats""#).unwrap();

        assert!(request.lexical.is_some());
        assert!(request.vector.is_some());

        let vector = request.vector.unwrap();
        assert_eq!(vector.query_payloads[0].field, "content");
    }

    #[test]
    fn test_empty_query_error() {
        let parser = make_parser();
        assert!(parser.parse("").is_err());
        assert!(parser.parse("   ").is_err());
    }

    #[test]
    fn test_custom_fusion() {
        let analyzer = Arc::new(StandardAnalyzer::new().unwrap());
        let lexical = QueryParser::new(analyzer).with_default_field("title");
        let vector = VectorQueryParser::new().with_default_field("content");
        let parser = UnifiedQueryParser::new(lexical, vector).with_fusion(
            FusionAlgorithm::WeightedSum {
                lexical_weight: 0.7,
                vector_weight: 0.3,
            },
        );

        let request = parser.parse(r#"title:hello content:~"cats""#).unwrap();

        if let Some(FusionAlgorithm::WeightedSum {
            lexical_weight,
            vector_weight,
        }) = request.fusion
        {
            assert!((lexical_weight - 0.7).abs() < f32::EPSILON);
            assert!((vector_weight - 0.3).abs() < f32::EPSILON);
        } else {
            panic!("Expected WeightedSum fusion");
        }
    }

    #[test]
    fn test_unicode_vector_text() {
        let parser = make_parser();
        let request = parser.parse(r#"content:~"日本語テスト""#).unwrap();

        let vector = request.vector.unwrap();
        if let DataValue::Text(ref text) = vector.query_payloads[0].payload {
            assert_eq!(text, "日本語テスト");
        } else {
            panic!("Expected DataValue::Text");
        }
    }

    // -- Tests for clean_lexical_string helper --

    #[test]
    fn test_clean_leading_boolean() {
        assert_eq!(clean_lexical_string("AND hello"), "hello");
        assert_eq!(clean_lexical_string("OR hello"), "hello");
    }

    #[test]
    fn test_clean_trailing_boolean() {
        assert_eq!(clean_lexical_string("hello AND"), "hello");
        assert_eq!(clean_lexical_string("hello OR"), "hello");
    }

    #[test]
    fn test_clean_consecutive_boolean() {
        assert_eq!(clean_lexical_string("hello AND AND world"), "hello AND world");
        assert_eq!(clean_lexical_string("hello OR OR world"), "hello OR world");
        assert_eq!(clean_lexical_string("hello AND OR world"), "hello AND world");
    }

    #[test]
    fn test_clean_multiple_spaces() {
        assert_eq!(clean_lexical_string("hello   world"), "hello world");
    }

    #[test]
    fn test_clean_empty() {
        assert_eq!(clean_lexical_string(""), "");
        assert_eq!(clean_lexical_string("   "), "");
        assert_eq!(clean_lexical_string("AND"), "");
        assert_eq!(clean_lexical_string("AND OR"), "");
    }
}
