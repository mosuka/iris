//! Wildcard query implementation for pattern matching.

use std::fmt::Debug;
use std::sync::Arc;

use regex::Regex;

use crate::error::Result;
use crate::lexical::index::inverted::core::terms::{TermDictionaryAccess, TermsEnum};
use crate::lexical::index::inverted::query::Query;
use crate::lexical::index::inverted::query::matcher::{EmptyMatcher, Matcher};
use crate::lexical::index::inverted::query::multi_term::{MultiTermQuery, RewriteMethod};
use crate::lexical::index::inverted::query::scorer::Scorer;
use crate::lexical::index::inverted::reader::InvertedIndexReader;
use crate::lexical::reader::LexicalIndexReader;

/// A query that matches documents containing terms that match a wildcard pattern.
///
/// Supports the following wildcards:
/// - `*` matches zero or more characters
/// - `?` matches exactly one character
/// - `\*` and `\?` match literal `*` and `?` characters
#[derive(Debug, Clone)]
pub struct WildcardQuery {
    /// The field to search in.
    field: String,
    /// The wildcard pattern.
    pattern: String,
    /// The compiled regex for matching.
    regex: Arc<Regex>,
    /// The boost factor for this query.
    /// The boost factor for this query.
    boost: f32,
    /// Rewrite method for multi-term expansion
    rewrite_method: RewriteMethod,
}

impl WildcardQuery {
    /// Create a new wildcard query.
    pub fn new<S: Into<String>>(field: S, pattern: S) -> Result<Self> {
        let field = field.into();
        let pattern = pattern.into();
        let regex = Self::compile_pattern(&pattern)?;

        Ok(WildcardQuery {
            field,
            pattern,
            regex: Arc::new(regex),
            boost: 1.0,
            rewrite_method: RewriteMethod::default(),
        })
    }

    /// Set the boost factor for this query.
    pub fn with_boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }

    /// Set the rewrite method.
    pub fn with_rewrite_method(mut self, rewrite_method: RewriteMethod) -> Self {
        self.rewrite_method = rewrite_method;
        self
    }

    /// Get the field name.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Get the wildcard pattern.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get the rewrite method.
    pub fn rewrite_method(&self) -> RewriteMethod {
        self.rewrite_method
    }

    /// Compile a wildcard pattern into a regex.
    fn compile_pattern(pattern: &str) -> Result<Regex> {
        let mut regex_pattern = String::new();
        regex_pattern.push('^'); // Match from the beginning

        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '\\' => {
                    // Handle escape sequences
                    if i + 1 < chars.len() {
                        match chars[i + 1] {
                            '*' => {
                                regex_pattern.push_str("\\*");
                                i += 1; // Skip the escaped character
                            }
                            '?' => {
                                regex_pattern.push_str("\\?");
                                i += 1; // Skip the escaped character
                            }
                            c => {
                                // Other escaped characters
                                regex_pattern.push('\\');
                                regex_pattern.push(c);
                                i += 1; // Skip the escaped character
                            }
                        }
                    } else {
                        regex_pattern.push('\\');
                    }
                }
                '*' => {
                    regex_pattern.push_str(".*");
                }
                '?' => {
                    regex_pattern.push('.');
                }
                // Regex special characters that need escaping
                '^' | '$' | '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '|' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(chars[i]);
                }
                c => {
                    regex_pattern.push(c);
                }
            }
            i += 1;
        }

        regex_pattern.push('$'); // Match to the end

        Regex::new(&regex_pattern).map_err(|e| {
            crate::error::SarissaError::analysis(format!("Invalid wildcard pattern: {e}"))
        })
    }

    /// Check if a term matches the wildcard pattern.
    pub fn matches(&self, term: &str) -> bool {
        self.regex.is_match(term)
    }
}

impl MultiTermQuery for WildcardQuery {
    fn field(&self) -> &str {
        &self.field
    }

    fn enumerate_terms(&self, reader: &dyn LexicalIndexReader) -> Result<Vec<(String, u64, f32)>> {
        let mut results = Vec::new();

        if let Some(inverted_reader) = reader.as_any().downcast_ref::<InvertedIndexReader>() {
            if let Some(terms) = inverted_reader.terms(&self.field)? {
                let mut iterator = terms.iterator()?;

                // Naive implementation: iterate over all terms and check match
                // Optimization TODO: Use AutomatonTermsEnum if we can convert regex to Level-based automaton
                // or if we can extract a common prefix to seek to.

                // Try to extract prefix for optimization
                let prefix: String = self
                    .pattern
                    .chars()
                    .take_while(|&c| c != '*' && c != '?' && c != '[')
                    .collect();

                if !prefix.is_empty() {
                    iterator.seek(&prefix)?;
                    // Check logic similar to PrefixQuery
                    if let Some(term_stats) = iterator.current() {
                        if term_stats.term.starts_with(&prefix) && self.matches(&term_stats.term) {
                            results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                        }
                    }

                    while let Some(term_stats) = iterator.next()? {
                        if !term_stats.term.starts_with(&prefix) {
                            break;
                        }
                        if self.matches(&term_stats.term) {
                            results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                        }
                        // Check limits
                        if let Some(limit) = self.rewrite_method.max_expansions() {
                            if results.len() >= limit {
                                break;
                            }
                        }
                    }
                } else {
                    // No prefix, must scan all terms
                    while let Some(term_stats) = iterator.next()? {
                        if self.matches(&term_stats.term) {
                            results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                        }
                        // Check limits
                        if let Some(limit) = self.rewrite_method.max_expansions() {
                            if results.len() >= limit {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

impl Query for WildcardQuery {
    fn matcher(&self, reader: &dyn LexicalIndexReader) -> Result<Box<dyn Matcher>> {
        let matching_terms = self.enumerate_terms(reader)?;

        if matching_terms.is_empty() {
            return Ok(Box::new(EmptyMatcher::new()));
        }

        // Construct BooleanQuery
        use crate::lexical::index::inverted::query::boolean::{BooleanClause, BooleanQuery, Occur};
        use crate::lexical::index::inverted::query::term::TermQuery;

        let mut boolean_query = BooleanQuery::new();
        boolean_query.set_boost(self.boost);

        for (term, _, _) in matching_terms {
            let term_query = TermQuery::new(self.field.clone(), term);
            boolean_query.add_clause(BooleanClause::new(
                Box::new(term_query),
                Occur::Should, // OR
            ));
        }

        boolean_query.matcher(reader)
    }

    fn scorer(&self, reader: &dyn LexicalIndexReader) -> Result<Box<dyn Scorer>> {
        // Delegate to BooleanQuery scorer
        let matching_terms = self.enumerate_terms(reader)?;

        if matching_terms.is_empty() {
            // Return dummy scorer
            use crate::lexical::index::inverted::query::scorer::BM25Scorer;
            return Ok(Box::new(BM25Scorer::new(
                0,
                0,
                reader.doc_count(),
                0.0,
                reader.doc_count(),
                0.0,
            )));
        }

        use crate::lexical::index::inverted::query::boolean::{BooleanClause, BooleanQuery, Occur};
        use crate::lexical::index::inverted::query::term::TermQuery;

        let mut boolean_query = BooleanQuery::new();
        boolean_query.set_boost(self.boost);

        for (term, _, _) in matching_terms {
            let term_query = TermQuery::new(self.field.clone(), term);
            boolean_query.add_clause(BooleanClause::new(Box::new(term_query), Occur::Should));
        }

        boolean_query.scorer(reader)
    }

    fn boost(&self) -> f32 {
        self.boost
    }

    fn set_boost(&mut self, boost: f32) {
        self.boost = boost;
    }

    fn description(&self) -> String {
        format!(
            "WildcardQuery(field: {}, pattern: {})",
            self.field, self.pattern
        )
    }

    fn clone_box(&self) -> Box<dyn Query> {
        Box::new(self.clone())
    }

    fn is_empty(&self, _reader: &dyn LexicalIndexReader) -> Result<bool> {
        Ok(self.pattern.is_empty())
    }

    fn cost(&self, reader: &dyn LexicalIndexReader) -> Result<u64> {
        // Wildcard queries can be expensive
        Ok(reader.doc_count() as u64)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_query_creation() {
        let query = WildcardQuery::new("content", "hello*").unwrap();

        assert_eq!(query.field(), "content");
        assert_eq!(query.pattern(), "hello*");
        assert_eq!(query.boost(), 1.0);
    }

    #[test]
    fn test_wildcard_query_with_boost() {
        let query = WildcardQuery::new("content", "test?")
            .unwrap()
            .with_boost(2.5);

        assert_eq!(query.boost(), 2.5);
    }

    #[test]
    fn test_wildcard_pattern_compilation() {
        // Test simple wildcard
        let query = WildcardQuery::new("field", "hello*").unwrap();
        assert!(query.matches("hello"));
        assert!(query.matches("helloworld"));
        assert!(!query.matches("hell"));

        // Test question mark
        let query = WildcardQuery::new("field", "h?llo").unwrap();
        assert!(query.matches("hello"));
        assert!(query.matches("hallo"));
        assert!(query.matches("hxllo"));
        assert!(!query.matches("heello"));

        // Test combination
        let query = WildcardQuery::new("field", "h*l?o").unwrap();
        assert!(query.matches("hello"));
        assert!(query.matches("hallo"));
        assert!(query.matches("heeello"));
        assert!(query.matches("hllo")); // Actually matches because ? can be 'l'
    }

    #[test]
    fn test_escaped_wildcards() {
        let query = WildcardQuery::new("field", "hello\\*world").unwrap();
        assert!(query.matches("hello*world"));
        assert!(!query.matches("helloworld"));
        assert!(!query.matches("hello123world"));

        let query = WildcardQuery::new("field", "hello\\?world").unwrap();
        assert!(query.matches("hello?world"));
        assert!(!query.matches("helloxworld"));
    }

    #[test]
    fn test_special_regex_characters() {
        let query = WildcardQuery::new("field", "hello.world").unwrap();
        assert!(query.matches("hello.world"));
        assert!(!query.matches("helloxworld"));

        let query = WildcardQuery::new("field", "hello+world").unwrap();
        assert!(query.matches("hello+world"));
        assert!(!query.matches("helloworld"));
    }
}
