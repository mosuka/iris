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
//! use laurus::engine::query::UnifiedQueryParser;
//!
//! let parser = UnifiedQueryParser::new(lexical_parser, vector_parser);
//!
//! // Hybrid search
//! let request = parser.parse(r#"title:hello content:~"cute kitten"^0.8"#).await?;
//! assert!(matches!(request.query, SearchQuery::Hybrid { .. }));
//!
//! // Lexical only
//! let request = parser.parse("title:hello AND body:world").await?;
//! assert!(matches!(request.query, SearchQuery::Lexical(_)));
//!
//! // Vector only
//! let request = parser.parse(r#"content:~"cats" image:~"dogs"^0.5"#).await?;
//! assert!(matches!(request.query, SearchQuery::Vector(_)));
//! ```

use std::sync::LazyLock;

use regex::Regex;

use crate::engine::search::{FusionAlgorithm, SearchQuery, SearchRequest};
use crate::error::{LaurusError, Result};
use crate::lexical::query::parser::LexicalQueryParser;
use crate::lexical::search::searcher::LexicalSearchQuery;
use crate::vector::query::parser::VectorQueryParser;

/// Unified query parser that composes lexical and vector parsers.
///
/// Parses a single DSL string into a [`SearchRequest`] by splitting the input
/// into lexical and vector portions and delegating to the appropriate sub-parser.
///
/// Vector clauses are identified by the `~"` pattern (tilde immediately before
/// a double quote), which is unambiguous with lexical syntax (where `~` only
/// appears after terms or phrases, e.g. `roam~2`, `"hello world"~10`).
///
/// After vector clauses are extracted, any leftover dangling boolean operators
/// (`AND`, `OR`) at the edges or in consecutive positions are cleaned up
/// before the lexical portion is parsed.
pub struct UnifiedQueryParser {
    lexical_parser: LexicalQueryParser,
    vector_parser: VectorQueryParser,
    default_fusion: FusionAlgorithm,
}

impl UnifiedQueryParser {
    /// Create a new `UnifiedQueryParser` with the given sub-parsers.
    ///
    /// The default fusion algorithm for hybrid queries is
    /// [`FusionAlgorithm::RRF { k: 60.0 }`](FusionAlgorithm::RRF).
    ///
    /// # Parameters
    ///
    /// - `lexical_parser` - Parser for lexical (text) query clauses.
    /// - `vector_parser` - Parser for vector (`~"text"`) query clauses.
    pub fn new(lexical_parser: LexicalQueryParser, vector_parser: VectorQueryParser) -> Self {
        Self {
            lexical_parser,
            vector_parser,
            default_fusion: FusionAlgorithm::RRF { k: 60.0 },
        }
    }

    /// Set the default fusion algorithm for hybrid queries.
    ///
    /// The fusion algorithm is only applied when the parsed query contains
    /// **both** lexical and vector clauses. For queries with only one type
    /// of clause, no fusion is performed.
    pub fn with_fusion(mut self, fusion: FusionAlgorithm) -> Self {
        self.default_fusion = fusion;
        self
    }

    /// Parse a unified query string into a [`SearchRequest`].
    ///
    /// The query string may contain both lexical and vector clauses:
    /// - Vector clauses: `field:~"text"`, `~"text"`, `field:~"text"^0.8`
    /// - Lexical clauses: everything else (`title:hello`, `AND`, `"phrase"`, etc.)
    ///
    /// Vector text is embedded into vectors at parse time via the
    /// `VectorQueryParser`'s embedder, so this method is `async`.
    ///
    /// When the parsed query contains both lexical and vector clauses, the
    /// returned `SearchRequest` will have its `fusion_algorithm` set to the
    /// parser's default (configurable via [`with_fusion`](Self::with_fusion)).
    ///
    /// # Parameters
    ///
    /// - `query_str` - The unified query DSL string to parse.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError::Other`] (invalid argument) if the query string
    /// is empty or consists only of whitespace.
    ///
    /// Returns [`LaurusError::Other`] (invalid argument) if, after splitting,
    /// no valid lexical or vector clause could be parsed from the input.
    pub async fn parse(&self, query_str: &str) -> Result<SearchRequest> {
        let query_str = query_str.trim();
        if query_str.is_empty() {
            return Err(LaurusError::invalid_argument(
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
            Some(self.vector_parser.parse(s).await?)
        } else {
            None
        };

        if lexical.is_none() && vector.is_none() {
            return Err(LaurusError::invalid_argument(
                "Query must contain at least one lexical or vector clause",
            ));
        }

        let fusion = if lexical.is_some() && vector.is_some() {
            Some(self.default_fusion)
        } else {
            None
        };

        let query = match (lexical, vector) {
            (Some(lex_query), Some(vec_req)) => SearchQuery::Hybrid {
                lexical: LexicalSearchQuery::Obj(lex_query),
                vector: vec_req.query,
            },
            (Some(lex_query), None) => SearchQuery::Lexical(LexicalSearchQuery::Obj(lex_query)),
            (None, Some(vec_req)) => SearchQuery::Vector(vec_req.query),
            (None, None) => unreachable!(),
        };

        Ok(SearchRequest {
            query,
            fusion_algorithm: fusion,
            ..Default::default()
        })
    }

    /// Split a query string into lexical and vector portions.
    ///
    /// Uses a regex to identify vector clauses (patterns matching
    /// `[field:]~"text"[^boost]`) and extracts them. The remainder, after
    /// cleanup via [`clean_lexical_string`], is treated as the lexical
    /// query text. Returns `(lexical, vector)` where either may be `None`.
    fn split_query(&self, input: &str) -> (Option<String>, Option<String>) {
        static VECTOR_RE: LazyLock<Regex> = LazyLock::new(|| {
            // Pattern: optional field prefix + ~"text" + optional boost
            // - (?:[\w][\w.]*:)? — optional field name with colon
            // - ~"[^"]*"         — tilde + quoted text
            // - (?:\^[\d]+(?:\.[\d]+)?)? — optional boost value
            Regex::new(r#"(?:[\w][\w.]*:)?~"[^"]*"(?:\^[\d]+(?:\.[\d]+)?)?"#).unwrap()
        });

        // Collect vector clauses
        let vector_clauses: Vec<&str> = VECTOR_RE.find_iter(input).map(|m| m.as_str()).collect();

        // Remove vector clauses from input to get lexical part
        let lexical_raw = VECTOR_RE.replace_all(input, " ");
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
    static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    // Collapse multiple whitespace
    let s = WHITESPACE_RE.replace_all(s, " ");
    let s = s.trim();

    if s.is_empty() {
        return String::new();
    }

    // Split into tokens and filter out dangling boolean operators
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut result: Vec<&str> = Vec::new();

    for token in &tokens {
        if token.eq_ignore_ascii_case("AND") || token.eq_ignore_ascii_case("OR") {
            // Only add boolean operator if there's a preceding non-boolean token
            // and the previous token is not already a boolean operator.
            if !result.is_empty() {
                let last = result.last().unwrap();
                if !last.eq_ignore_ascii_case("AND") && !last.eq_ignore_ascii_case("OR") {
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
    if let Some(last) = result.last()
        && (last.eq_ignore_ascii_case("AND") || last.eq_ignore_ascii_case("OR"))
    {
        result.pop();
    }

    result.join(" ")
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::*;
    use crate::analysis::analyzer::standard::StandardAnalyzer;
    use crate::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
    use crate::engine::search::VectorSearchQuery;
    use crate::error::Result as IrisResult;
    use crate::vector::core::vector::Vector;
    use crate::vector::store::request::QueryVector;

    /// Mock embedder that returns a zero vector of dimension 4.
    #[derive(Debug)]
    struct MockEmbedder;

    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, _input: &EmbedInput<'_>) -> IrisResult<Vector> {
            Ok(Vector::new(vec![0.0; 4]))
        }
        fn supported_input_types(&self) -> Vec<EmbedInputType> {
            vec![EmbedInputType::Text]
        }
        fn name(&self) -> &str {
            "mock"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Assert that the request contains a lexical-only query.
    fn assert_lexical_only(request: &SearchRequest) {
        assert!(matches!(request.query, SearchQuery::Lexical(_)));
    }

    /// Assert that the request contains a vector-only query and return a
    /// reference to the vector query.
    fn assert_vector_only(request: &SearchRequest) -> &VectorSearchQuery {
        match &request.query {
            SearchQuery::Vector(v) => v,
            _ => panic!("Expected SearchQuery::Vector"),
        }
    }

    /// Assert that the request contains a hybrid query and return references
    /// to the lexical and vector components.
    fn assert_hybrid(request: &SearchRequest) -> (&LexicalSearchQuery, &VectorSearchQuery) {
        match &request.query {
            SearchQuery::Hybrid { lexical, vector } => (lexical, vector),
            _ => panic!("Expected SearchQuery::Hybrid"),
        }
    }

    /// Extract the query vectors from a vector search query.
    fn get_vectors(vq: &VectorSearchQuery) -> &Vec<QueryVector> {
        match vq {
            VectorSearchQuery::Vectors(v) => v,
            _ => panic!("Expected VectorSearchQuery::Vectors"),
        }
    }

    fn make_parser() -> UnifiedQueryParser {
        let analyzer = Arc::new(StandardAnalyzer::new().unwrap());
        let lexical = LexicalQueryParser::new(analyzer).with_default_field("title");
        let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder);
        let vector = VectorQueryParser::new(embedder).with_default_field("content");
        UnifiedQueryParser::new(lexical, vector)
    }

    #[tokio::test]
    async fn test_lexical_only() {
        let parser = make_parser();
        let request = parser.parse("title:hello").await.unwrap();

        assert_lexical_only(&request);
        assert!(request.fusion_algorithm.is_none());
    }

    #[tokio::test]
    async fn test_vector_only() {
        let parser = make_parser();
        let request = parser.parse(r#"content:~"cats""#).await.unwrap();

        let vq = assert_vector_only(&request);
        assert!(request.fusion_algorithm.is_none());

        let vecs = get_vectors(vq);
        assert_eq!(vecs.len(), 1);
        assert_eq!(vecs[0].fields.as_ref().unwrap()[0], "content");
    }

    #[tokio::test]
    async fn test_hybrid() {
        let parser = make_parser();
        let request = parser
            .parse(r#"title:hello content:~"cats""#)
            .await
            .unwrap();

        assert_hybrid(&request);
        assert!(request.fusion_algorithm.is_some());

        // Fusion defaults to RRF
        if let Some(FusionAlgorithm::RRF { k }) = request.fusion_algorithm {
            assert!((k - 60.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected RRF fusion");
        }
    }

    #[tokio::test]
    async fn test_vector_with_boost() {
        let parser = make_parser();
        let request = parser.parse(r#"content:~"text"^0.8"#).await.unwrap();

        let vq = assert_vector_only(&request);
        let vecs = get_vectors(vq);
        assert!((vecs[0].weight - 0.8).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_multiple_vector_clauses() {
        let parser = make_parser();
        let request = parser.parse(r#"a:~"x" b:~"y"^0.5"#).await.unwrap();

        let vq = assert_vector_only(&request);
        let vecs = get_vectors(vq);
        assert_eq!(vecs.len(), 2);
        assert_eq!(vecs[0].fields.as_ref().unwrap()[0], "a");
        assert_eq!(vecs[1].fields.as_ref().unwrap()[0], "b");
        assert!((vecs[1].weight - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_lexical_and_with_vector() {
        let parser = make_parser();
        let request = parser
            .parse(r#"title:hello AND title:world content:~"cats""#)
            .await
            .unwrap();

        assert_hybrid(&request);
        assert!(request.fusion_algorithm.is_some());
    }

    #[tokio::test]
    async fn test_vector_between_and() {
        let parser = make_parser();
        // After removing content:~"cats", we get "title:hello AND AND title:world"
        // which should be cleaned to "title:hello AND title:world"
        let request = parser
            .parse(r#"title:hello AND content:~"cats" AND title:world"#)
            .await
            .unwrap();

        assert_hybrid(&request);
    }

    #[tokio::test]
    async fn test_default_fields() {
        let parser = make_parser();
        // Lexical uses default field "title", vector uses default field "content"
        let request = parser.parse(r#"hello ~"cats""#).await.unwrap();

        let (_, vq) = assert_hybrid(&request);
        let vecs = get_vectors(vq);
        assert_eq!(vecs[0].fields.as_ref().unwrap()[0], "content");
    }

    #[tokio::test]
    async fn test_empty_query_error() {
        let parser = make_parser();
        assert!(parser.parse("").await.is_err());
        assert!(parser.parse("   ").await.is_err());
    }

    #[tokio::test]
    async fn test_custom_fusion() {
        let analyzer = Arc::new(StandardAnalyzer::new().unwrap());
        let lexical = LexicalQueryParser::new(analyzer).with_default_field("title");
        let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder);
        let vector = VectorQueryParser::new(embedder).with_default_field("content");
        let parser =
            UnifiedQueryParser::new(lexical, vector).with_fusion(FusionAlgorithm::WeightedSum {
                lexical_weight: 0.7,
                vector_weight: 0.3,
            });

        let request = parser
            .parse(r#"title:hello content:~"cats""#)
            .await
            .unwrap();

        if let Some(FusionAlgorithm::WeightedSum {
            lexical_weight,
            vector_weight,
        }) = request.fusion_algorithm
        {
            assert!((lexical_weight - 0.7).abs() < f32::EPSILON);
            assert!((vector_weight - 0.3).abs() < f32::EPSILON);
        } else {
            panic!("Expected WeightedSum fusion");
        }
    }

    #[tokio::test]
    async fn test_unicode_vector_text() {
        let parser = make_parser();
        let request = parser.parse(r#"content:~"日本語テスト""#).await.unwrap();

        let vq = assert_vector_only(&request);
        let vecs = get_vectors(vq);
        assert_eq!(vecs.len(), 1);
        assert_eq!(vecs[0].fields.as_ref().unwrap()[0], "content");
        assert_eq!(vecs[0].vector.dimension(), 4);
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
        assert_eq!(
            clean_lexical_string("hello AND AND world"),
            "hello AND world"
        );
        assert_eq!(clean_lexical_string("hello OR OR world"), "hello OR world");
        assert_eq!(
            clean_lexical_string("hello AND OR world"),
            "hello AND world"
        );
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
