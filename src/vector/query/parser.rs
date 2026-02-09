//! DSL parser for vector search queries.
//!
//! Parses `field:~"text"` syntax into VectorSearchRequest.

use pest::Parser;
use pest_derive::Parser;

use crate::data::DataValue;
use crate::error::{IrisError, Result};
use crate::vector::store::request::{QueryPayload, VectorSearchRequest};

/// Pest grammar parser for vector query DSL.
#[derive(Parser)]
#[grammar = "vector/query/parser.pest"]
struct VectorQueryStringParser;

/// Parser for vector search DSL.
///
/// Converts `field:~"text"` syntax into `VectorSearchRequest`.
///
/// # Supported Syntax
///
/// - `content:~"cute kitten"` — field-specific text query
/// - `content:~"cute kitten"^0.8` — with weight (boost)
/// - `~"cute kitten"` — uses default field
/// - `content:~"cats" image:~"dogs"^0.5` — multiple queries
///
/// # Example
///
/// ```ignore
/// use iris::vector::query::VectorQueryParser;
///
/// let parser = VectorQueryParser::new()
///     .with_default_field("content");
///
/// let request = parser.parse(r#"content:~"cute kitten""#).unwrap();
/// assert_eq!(request.query_payloads.len(), 1);
/// ```
pub struct VectorQueryParser {
    default_fields: Vec<String>,
}

impl VectorQueryParser {
    /// Create a new VectorQueryParser with no default fields.
    pub fn new() -> Self {
        Self {
            default_fields: Vec::new(),
        }
    }

    /// Set a single default field for queries without explicit field prefix.
    pub fn with_default_field(mut self, field: impl Into<String>) -> Self {
        self.default_fields = vec![field.into()];
        self
    }

    /// Set multiple default fields for queries without explicit field prefix.
    pub fn with_default_fields(mut self, fields: Vec<String>) -> Self {
        self.default_fields = fields;
        self
    }

    /// Parse a vector query DSL string into a VectorSearchRequest.
    pub fn parse(&self, query_str: &str) -> Result<VectorSearchRequest> {
        let pairs = VectorQueryStringParser::parse(Rule::query, query_str).map_err(|e| {
            IrisError::invalid_argument(format!("Failed to parse vector query: {}", e))
        })?;

        let mut request = VectorSearchRequest::default();

        for pair in pairs {
            if pair.as_rule() == Rule::query {
                for inner in pair.into_inner() {
                    if inner.as_rule() == Rule::vector_clause {
                        let payload = self.parse_vector_clause(inner)?;
                        request.query_payloads.push(payload);
                    }
                }
            }
        }

        if request.query_payloads.is_empty() {
            return Err(IrisError::invalid_argument(
                "Vector query must contain at least one clause",
            ));
        }

        Ok(request)
    }

    /// Parse a single vector clause (e.g., `content:~"cute kitten"^0.8`).
    fn parse_vector_clause(&self, pair: pest::iterators::Pair<Rule>) -> Result<QueryPayload> {
        let mut field_name: Option<String> = None;
        let mut text: Option<String> = None;
        let mut weight: f32 = 1.0;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::field_prefix => {
                    // Extract field_name from field_prefix
                    for fp_inner in inner.into_inner() {
                        if fp_inner.as_rule() == Rule::field_name {
                            field_name = Some(fp_inner.as_str().to_string());
                        }
                    }
                }
                Rule::quoted_text => {
                    // Extract text from quoted_text → inner_text
                    for qt_inner in inner.into_inner() {
                        if qt_inner.as_rule() == Rule::inner_text {
                            text = Some(qt_inner.as_str().to_string());
                        }
                    }
                }
                Rule::boost => {
                    // Extract weight from boost → float_value
                    for b_inner in inner.into_inner() {
                        if b_inner.as_rule() == Rule::float_value {
                            weight = b_inner.as_str().parse::<f32>().map_err(|e| {
                                IrisError::invalid_argument(format!(
                                    "Invalid boost value: {}",
                                    e
                                ))
                            })?;
                        }
                    }
                }
                _ => {}
            }
        }

        // Resolve field name
        let field = match field_name {
            Some(f) => f,
            None => {
                if self.default_fields.is_empty() {
                    return Err(IrisError::invalid_argument(
                        "No field specified and no default field configured",
                    ));
                }
                self.default_fields[0].clone()
            }
        };

        let text = text.ok_or_else(|| {
            IrisError::invalid_argument("Missing quoted text in vector clause")
        })?;

        Ok(QueryPayload::with_weight(
            field,
            DataValue::Text(text),
            weight,
        ))
    }
}

impl Default for VectorQueryParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query() {
        let parser = VectorQueryParser::new();
        let request = parser.parse(r#"content:~"cute kitten""#).unwrap();

        assert_eq!(request.query_payloads.len(), 1);
        let payload = &request.query_payloads[0];
        assert_eq!(payload.field, "content");
        assert_eq!(payload.weight, 1.0);
        if let DataValue::Text(ref text) = payload.payload {
            assert_eq!(text, "cute kitten");
        } else {
            panic!("Expected DataValue::Text");
        }
    }

    #[test]
    fn test_boost() {
        let parser = VectorQueryParser::new();
        let request = parser.parse(r#"content:~"text"^0.8"#).unwrap();

        assert_eq!(request.query_payloads.len(), 1);
        let payload = &request.query_payloads[0];
        assert_eq!(payload.field, "content");
        assert!((payload.weight - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default_field() {
        let parser = VectorQueryParser::new().with_default_field("embedding");
        let request = parser.parse(r#"~"cute kitten""#).unwrap();

        assert_eq!(request.query_payloads.len(), 1);
        assert_eq!(request.query_payloads[0].field, "embedding");
    }

    #[test]
    fn test_multiple_clauses() {
        let parser = VectorQueryParser::new();
        let request = parser
            .parse(r#"content:~"cats" image:~"dogs"^0.5"#)
            .unwrap();

        assert_eq!(request.query_payloads.len(), 2);

        assert_eq!(request.query_payloads[0].field, "content");
        assert_eq!(request.query_payloads[0].weight, 1.0);

        assert_eq!(request.query_payloads[1].field, "image");
        assert!((request.query_payloads[1].weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_empty_query_error() {
        let parser = VectorQueryParser::new();
        assert!(parser.parse("").is_err());
    }

    #[test]
    fn test_missing_tilde_error() {
        let parser = VectorQueryParser::new();
        // content:"text" without ~ should fail
        assert!(parser.parse(r#"content:"text""#).is_err());
    }

    #[test]
    fn test_no_field_no_default_error() {
        let parser = VectorQueryParser::new(); // no default field
        assert!(parser.parse(r#"~"text""#).is_err());
    }

    #[test]
    fn test_unicode_text() {
        let parser = VectorQueryParser::new();
        let request = parser.parse(r#"content:~"日本語テスト""#).unwrap();

        assert_eq!(request.query_payloads.len(), 1);
        if let DataValue::Text(ref text) = request.query_payloads[0].payload {
            assert_eq!(text, "日本語テスト");
        } else {
            panic!("Expected DataValue::Text");
        }
    }

    #[test]
    fn test_integer_boost() {
        let parser = VectorQueryParser::new();
        let request = parser.parse(r#"content:~"text"^2"#).unwrap();

        assert!((request.query_payloads[0].weight - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_field_with_underscore() {
        let parser = VectorQueryParser::new();
        let request = parser.parse(r#"my_field:~"text""#).unwrap();

        assert_eq!(request.query_payloads[0].field, "my_field");
    }

    #[test]
    fn test_field_with_dot() {
        let parser = VectorQueryParser::new();
        let request = parser.parse(r#"nested.field:~"text""#).unwrap();

        assert_eq!(request.query_payloads[0].field, "nested.field");
    }
}
