//! Regular expression query implementation.

use std::fmt::Debug;
use std::sync::Arc;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SarissaError};
use crate::lexical::index::inverted::core::automaton::{AutomatonTermsEnum, RegexAutomaton};
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
}

impl MultiTermQuery for RegexpQuery {
    fn field(&self) -> &str {
        &self.field
    }

    fn enumerate_terms(&self, reader: &dyn LexicalIndexReader) -> Result<Vec<(String, u64, f32)>> {
        let mut results = Vec::new();

        if let Some(inverted_reader) = reader.as_any().downcast_ref::<InvertedIndexReader>() {
            if let Some(terms) = inverted_reader.terms(&self.field)? {
                // Use RegexAutomaton and AutomatonTermsEnum
                // Re-use our existing regex logic via RegexAutomaton
                // Note: We need to handle the case where self.regex is Some or None
                let regex_automaton = if let Some(regex) = &self.regex {
                    RegexAutomaton::from_regex(regex.as_ref().clone(), self.pattern.clone())
                } else {
                    RegexAutomaton::new(&self.pattern)?
                };

                let mut terms_enum = AutomatonTermsEnum::new(terms.iterator()?, regex_automaton);

                if let Some(max) = self.rewrite_method.max_expansions() {
                    terms_enum = terms_enum.with_max_matches(max);
                }

                while let Some(term_stats) = terms_enum.next()? {
                    results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
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
        use crate::lexical::index::inverted::core::automaton::{Automaton, RegexAutomaton};

        // Test via RegexAutomaton
        let automaton = RegexAutomaton::new("^abc.*").unwrap();
        assert_eq!(automaton.initial_seek_term().as_deref(), Some("abc"));

        let automaton = RegexAutomaton::new("abc.*").unwrap();
        assert_eq!(automaton.initial_seek_term(), None); // No anchor

        let automaton = RegexAutomaton::new("^abc\\.def").unwrap();
        assert_eq!(automaton.initial_seek_term().as_deref(), Some("abc.def"));

        let automaton = RegexAutomaton::new("^a(b|c)").unwrap();
        assert_eq!(automaton.initial_seek_term().as_deref(), Some("a"));
    }
}
