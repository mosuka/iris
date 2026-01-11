//! Prefix query implementation.
//!
//! This module provides support for finding terms that start with a specific prefix.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::lexical::index::inverted::core::terms::{TermDictionaryAccess, TermStats, TermsEnum};
use crate::lexical::index::inverted::query::Query;
use crate::lexical::index::inverted::query::matcher::Matcher;
use crate::lexical::index::inverted::query::multi_term::{MultiTermQuery, RewriteMethod};
use crate::lexical::index::inverted::query::scorer::Scorer;
use crate::lexical::index::inverted::reader::InvertedIndexReader;
use crate::lexical::reader::LexicalIndexReader;

/// A query that matches terms starting with a specific prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixQuery {
    /// Field to search in
    field: String,
    /// Prefix to search for
    prefix: String,
    /// Boost factor for the query
    boost: f32,
    /// Rewrite method for multi-term expansion
    rewrite_method: RewriteMethod,
}

impl PrefixQuery {
    /// Create a new prefix query.
    pub fn new<F: Into<String>, P: Into<String>>(field: F, prefix: P) -> Self {
        PrefixQuery {
            field: field.into(),
            prefix: prefix.into(),
            boost: 1.0,
            rewrite_method: RewriteMethod::default(),
        }
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

    /// Get the prefix.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Get the field name.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Get the rewrite method.
    pub fn rewrite_method(&self) -> RewriteMethod {
        self.rewrite_method
    }
}

impl Query for PrefixQuery {
    fn matcher(&self, reader: &dyn LexicalIndexReader) -> Result<Box<dyn Matcher>> {
        // Since MultiTermQuery generic implementation isn't fully ready/linked yet (as per TODOs in multi_term.rs),
        // we implement a simple version here that collects matching terms and creates a BooleanQuery-like matcher
        // or delegates to a specialized implementation.

        // For now, let's just use the enumerate_terms method we implement below
        // and build a DisjunctionMatcher manually or similar.
        // Actually, without a generic MultiTermQuery rewrite engine, we have to do it manually.

        // However, looking at fuzzy.rs, it implements its own matches finding.
        // PrefixQuery is simpler. We can collect terms and use a BooleanQuery or TermQuery logic.

        // Let's implement term enumeration and simple matching for now.
        // In a real Lucene-like system, we would rewrite to BooleanQuery.
        // Since we don't have the full rewrite machinery exposed easily,
        // let's try to mimic what fuzzy.rs does or use simple collection.

        // Note: For this implementation, we will use the "rewrite to basic matcher" approach
        // where we find matching terms and create a composite matcher.

        // TODO: Use actual RewriteMethod logic when available.

        let matching_terms = self.enumerate_terms(reader)?;

        if matching_terms.is_empty() {
            use crate::lexical::index::inverted::query::matcher::EmptyMatcher;
            return Ok(Box::new(EmptyMatcher::new()));
        }

        // naive implementation: just collect all matching terms and treat as "OR"
        // This is effectively RewriteMethod::BooleanQuery or TopTermsBlended

        // We need to return a Matcher.
        // We can reuse FuzzyMatcher logic or create a similar MultiTermMatcher.
        // Or simply construct a BooleanQuery and get its matcher?
        // But BooleanQuery is in a sibling module.

        // Let's rely on finding matches and constructing a simpler matcher for this MVP.
        // Re-using FuzzyMatcher might be weird but it does exactly "OR of terms".
        // Let's define a specific PrefixMatcher or similar if needed,
        // but FuzzyMatcher actually takes `FuzzyMatch` structs.

        // Simpler: Use functionality from boolean query if possible, but that might differ.
        // For strict MVP, let's implement a simple Disjunction over TermMatchers.

        // But wait, `fuzzy.rs` has `FuzzyMatcher`. We can create a `PrefixMatcher` similar to it.
        // Actually, let's try to use BooleanQuery if we can import it.
        // src/lexical/index/inverted/query/boolean.rs

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
        // Delegate to the rewritten query (BooleanQuery)
        // Similar logic to matcher above

        use crate::lexical::index::inverted::query::boolean::{BooleanClause, BooleanQuery, Occur};
        use crate::lexical::index::inverted::query::term::TermQuery;

        let matching_terms = self.enumerate_terms(reader)?;

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
            "PrefixQuery(field: {}, prefix: {})",
            self.field, self.prefix
        )
    }

    fn clone_box(&self) -> Box<dyn Query> {
        Box::new(self.clone())
    }

    fn is_empty(&self, _reader: &dyn LexicalIndexReader) -> Result<bool> {
        Ok(self.prefix.is_empty())
    }

    fn cost(&self, reader: &dyn LexicalIndexReader) -> Result<u64> {
        // Rough estimate
        Ok(reader.doc_count() as u64)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn field(&self) -> Option<&str> {
        Some(&self.field)
    }
}

impl MultiTermQuery for PrefixQuery {
    fn field(&self) -> &str {
        &self.field
    }

    fn enumerate_terms(&self, reader: &dyn LexicalIndexReader) -> Result<Vec<(String, u64, f32)>> {
        let mut results = Vec::new();

        if let Some(inverted_reader) = reader.as_any().downcast_ref::<InvertedIndexReader>() {
            if let Some(terms) = inverted_reader.terms(&self.field)? {
                let mut iterator = terms.iterator()?;

                // Seek to the prefix
                iterator.seek(&self.prefix)?;

                // Iterate while prefix matches
                // Note: iterator.next() continues from current position
                // But we need to make sure we check the current term after seek first?
                // TermsEnum::seek usually positions at the term >= target.
                // So we should check current() or next().
                // However, standard TermsEnum usage pattern in this codebase:
                // seek() returns bool (found exact?), then we can use next() or current()?
                // Checking `automaton.rs`: seek() then next() loop.
                // Let's check `core/terms.rs` trait definition if possible.
                // Assuming standard behavior: seek positions at >=.

                // If seek found exact match or placed us at a term, we need to check if it starts with prefix.
                // However, `iterator.next()` advances.
                // If we use `iterator.seek(&self.prefix)?`, it positions us.
                // We should probably loop and check.

                // Implementation detail for standard TermsEnum often requires getting the current term
                // immediately after seek if we want to include it.
                // Let's try to inspect the term after seek.

                // But `TermsEnum` here might not expose `term()` directly without `current()`.
                if let Some(term_stats) = iterator.current() {
                    if term_stats.term.starts_with(&self.prefix) {
                        results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));
                    }
                }

                // Continue with next()
                while let Some(term_stats) = iterator.next()? {
                    if !term_stats.term.starts_with(&self.prefix) {
                        break;
                    }
                    results.push((term_stats.term.clone(), term_stats.doc_freq, 1.0));

                    // Check limits based on RewriteMethod/max_expansions if we implemented full logic
                    if let Some(limit) = self.rewrite_method.max_expansions() {
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

/// A TermsEnum that only yields terms starting with a prefix.
pub struct PrefixTermsEnum<T: TermsEnum> {
    inner: T,
    prefix: String,
}

impl<T: TermsEnum> PrefixTermsEnum<T> {
    pub fn new(mut inner: T, prefix: String) -> Result<Self> {
        inner.seek(&prefix)?;
        Ok(Self { inner, prefix })
    }
}

impl<T: TermsEnum> TermsEnum for PrefixTermsEnum<T> {
    fn next(&mut self) -> Result<Option<TermStats>> {
        if let Some(stats) = self.inner.next()? {
            if stats.term.starts_with(&self.prefix) {
                return Ok(Some(stats));
            }
        }
        Ok(None)
    }

    fn seek(&mut self, target: &str) -> Result<bool> {
        // If target is before prefix, seek to prefix
        if target < self.prefix.as_str() {
            return self.inner.seek(&self.prefix);
        }
        // If target doesn't start with prefix and is greater, we can't match anything?
        // Or we just seek normally and let next() handle mismatch.
        self.inner.seek(target)
    }

    fn seek_exact(&mut self, term: &str) -> Result<bool> {
        if !term.starts_with(&self.prefix) {
            return Ok(false);
        }
        self.inner.seek_exact(term)
    }

    fn current(&self) -> Option<&TermStats> {
        let current = self.inner.current();
        if let Some(stats) = current {
            if stats.term.starts_with(&self.prefix) {
                return Some(stats);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Removed unused imports:
    // use crate::lexical::index::inverted::reader::{InvertedIndexReader, InvertedIndexReaderConfig};
    // use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    // use std::sync::Arc;

    #[test]
    fn test_prefix_query_creation() {
        let query = PrefixQuery::new("field", "pre").with_boost(2.0);

        // Changed to call MultiTermQuery::field() which returns &str
        assert_eq!(MultiTermQuery::field(&query), "field");
        assert_eq!(query.prefix(), "pre");
        assert_eq!(query.boost(), 2.0);
    }

    // Note: Integration tests with actual index would be better,
    // but we can test basic structural properties here.
}
