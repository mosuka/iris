//! Regular expression query implementation.

use std::fmt::Debug;
use std::sync::Arc;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SarissaError};
use crate::lexical::index::inverted::core::terms::{TermDictionaryAccess, TermsEnum};
use crate::lexical::index::inverted::query::Query;
use crate::lexical::index::inverted::query::matcher::{EmptyMatcher, Matcher};
use crate::lexical::index::inverted::query::multi_term::{MultiTermQuery, RewriteMethod};
use crate::lexical::index::inverted::query::scorer::Scorer;
use crate::lexical::index::inverted::reader::InvertedIndexReader;
use crate::lexical::reader::LexicalIndexReader;

/// A query that matches documents containing terms that match a regular expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexpQuery {
    /// The field to search in.
    field: String,
    /// The regular expression pattern.
    pattern: String,
    /// The compiled regex for matching.
    #[serde(skip)]
    regex: Option<Arc<Regex>>,
    /// The boost factor for this query.
    boost: f32,
    /// Rewrite method for multi-term expansion
    rewrite_method: RewriteMethod,
}

impl RegexpQuery {
    /// Create a new regexp query.
    pub fn new<S: Into<String>>(field: S, pattern: S) -> Result<Self> {
        let field = field.into();
        let pattern = pattern.into();
        let regex = Regex::new(&pattern)
            .map_err(|e| SarissaError::analysis(format!("Invalid regexp pattern: {e}")))?;

        Ok(RegexpQuery {
            field,
            pattern,
            regex: Some(Arc::new(regex)),
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

    /// Get the pattern.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get the rewrite method.
    pub fn rewrite_method(&self) -> RewriteMethod {
        self.rewrite_method
    }

    /// Get the compiled regex.
    /// Recompiles if missing (e.g. after deserialization).
    fn get_regex(&self) -> Result<Arc<Regex>> {
        if let Some(regex) = &self.regex {
            Ok(regex.clone())
        } else {
            let regex = Regex::new(&self.pattern)
                .map_err(|e| SarissaError::analysis(format!("Invalid regexp pattern: {e}")))?;
            Ok(Arc::new(regex))
        }
    }

    /// Attempt to extract a constant prefix from the regex pattern.
    /// This is a simple heuristic to optimize term dictionary constraints.
    fn extract_prefix(&self) -> Option<String> {
        let mut chars = self.pattern.chars();
        if chars.next() != Some('^') {
            return None;
        }

        let mut prefix = String::new();
        let mut escaped = false;

        for c in chars {
            if escaped {
                prefix.push(c);
                escaped = false;
            } else {
                match c {
                    '\\' => escaped = true,
                    '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$' => {
                        // Special char, stop prefix extraction
                        break;
                    }
                    _ => prefix.push(c),
                }
            }
        }

        if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        }
    }
}

impl MultiTermQuery for RegexpQuery {
    fn field(&self) -> &str {
        &self.field
    }

    fn enumerate_terms(&self, reader: &dyn LexicalIndexReader) -> Result<Vec<(String, u64, f32)>> {
        let mut results = Vec::new();
        let regex = self.get_regex()?;

        if let Some(inverted_reader) = reader.as_any().downcast_ref::<InvertedIndexReader>() {
            if let Some(terms) = inverted_reader.terms(&self.field)? {
                let mut iterator = terms.iterator()?;

                // Try to extract prefix for optimization
                if let Some(prefix) = self.extract_prefix() {
                    iterator.seek(&prefix)?;
                    // Check current term after seek
                    if let Some(term_stats) = iterator.current() {
                        if term_stats.term.starts_with(&prefix) && regex.is_match(&term_stats.term)
                        {
                            results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                        }
                    }

                    while let Some(term_stats) = iterator.next()? {
                        if !term_stats.term.starts_with(&prefix) {
                            break;
                        }
                        if regex.is_match(&term_stats.term) {
                            results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                        }
                        if let Some(limit) = self.rewrite_method.max_expansions() {
                            if results.len() >= limit {
                                break;
                            }
                        }
                    }
                } else {
                    // No prefix, scan all terms
                    while let Some(term_stats) = iterator.next()? {
                        if regex.is_match(&term_stats.term) {
                            results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                        }
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

impl Query for RegexpQuery {
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
            boolean_query.add_clause(BooleanClause::new(Box::new(term_query), Occur::Should));
        }

        boolean_query.matcher(reader)
    }

    fn scorer(&self, reader: &dyn LexicalIndexReader) -> Result<Box<dyn Scorer>> {
        let matching_terms = self.enumerate_terms(reader)?;

        if matching_terms.is_empty() {
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
            "RegexpQuery(field: {}, pattern: {})",
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
    fn test_regexp_query_creation() {
        let query = RegexpQuery::new("field", "^abc.*").unwrap();
        assert_eq!(query.field(), "field");
        assert_eq!(query.pattern(), "^abc.*");
        assert_eq!(query.boost(), 1.0);
    }

    #[test]
    fn test_prefix_extraction() {
        let query = RegexpQuery::new("f", "^abc.*").unwrap();
        assert_eq!(query.extract_prefix().as_deref(), Some("abc"));

        let query = RegexpQuery::new("f", "abc.*").unwrap();
        assert_eq!(query.extract_prefix(), None); // No anchor

        let query = RegexpQuery::new("f", "^abc\\.def").unwrap();
        assert_eq!(query.extract_prefix().as_deref(), Some("abc.def"));

        let query = RegexpQuery::new("f", "^a(b|c)").unwrap();
        assert_eq!(query.extract_prefix().as_deref(), Some("a"));
    }
}
