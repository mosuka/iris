//! Per-field analyzer (Lucene-compatible).

use std::sync::Arc;

use ahash::AHashMap;
use parking_lot::RwLock;

use crate::analysis::analyzer::analyzer::Analyzer;
use crate::analysis::token::TokenStream;
use crate::error::Result;

/// A per-field analyzer that applies different analyzers to different fields.
///
/// This is similar to Lucene's PerFieldAnalyzerWrapper. It allows you to specify
/// a different analyzer for each field, with a default analyzer for fields not
/// explicitly configured.
///
/// Field-specific analyzers can be added at any time via [`add_analyzer`](Self::add_analyzer),
/// even after the analyzer has been wrapped in an `Arc`. This enables dynamic
/// field addition at runtime.
///
/// # Memory Efficiency
///
/// When using the same analyzer for multiple fields, reuse a single instance
/// with `Arc::clone` to save memory. This is especially important for analyzers
/// with large dictionaries (e.g., Lindera for Japanese).
///
/// # Example
///
/// ```
/// use laurus::analysis::analyzer::analyzer::Analyzer;
/// use laurus::analysis::analyzer::per_field::PerFieldAnalyzer;
/// use laurus::analysis::analyzer::standard::StandardAnalyzer;
/// use laurus::analysis::analyzer::keyword::KeywordAnalyzer;
/// use std::sync::Arc;
///
/// // Reuse analyzer instances to save memory
/// let keyword_analyzer: Arc<dyn Analyzer> = Arc::new(KeywordAnalyzer::new());
/// let analyzer = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new().unwrap()));
/// analyzer.add_analyzer("id", Arc::clone(&keyword_analyzer));
/// analyzer.add_analyzer("category", Arc::clone(&keyword_analyzer));
/// // "title" and "body" will use StandardAnalyzer
/// // "id" and "category" will use the same KeywordAnalyzer instance
/// ```
#[derive(Debug)]
pub struct PerFieldAnalyzer {
    /// Default analyzer for fields not in the map.
    default_analyzer: Arc<dyn Analyzer>,

    /// Map of field names to their specific analyzers.
    /// Wrapped in `RwLock` to allow adding analyzers at runtime via `&self`.
    field_analyzers: RwLock<AHashMap<String, Arc<dyn Analyzer>>>,
}

impl Clone for PerFieldAnalyzer {
    fn clone(&self) -> Self {
        Self {
            default_analyzer: self.default_analyzer.clone(),
            field_analyzers: RwLock::new(self.field_analyzers.read().clone()),
        }
    }
}

impl PerFieldAnalyzer {
    /// Create a new per-field analyzer with a default analyzer.
    ///
    /// # Arguments
    ///
    /// * `default_analyzer` - The analyzer to use for fields not explicitly configured
    pub fn new(default_analyzer: Arc<dyn Analyzer>) -> Self {
        Self {
            default_analyzer,
            field_analyzers: RwLock::new(AHashMap::new()),
        }
    }

    /// Add a field-specific analyzer.
    ///
    /// This method takes `&self` (not `&mut self`) and uses interior mutability,
    /// so it can be called even after the analyzer has been wrapped in an `Arc`.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `analyzer` - The analyzer to use for this field
    pub fn add_analyzer(&self, field: impl Into<String>, analyzer: Arc<dyn Analyzer>) {
        self.field_analyzers.write().insert(field.into(), analyzer);
    }

    /// Remove the field-specific analyzer for the given field.
    ///
    /// After removal, the field will fall back to the default analyzer.
    /// This method is a no-op if the field has no specific analyzer configured.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name whose analyzer should be removed
    pub fn remove_analyzer(&self, field: &str) {
        self.field_analyzers.write().remove(field);
    }

    /// Get the analyzer for a specific field.
    ///
    /// Returns the field-specific analyzer if configured, otherwise returns the default.
    /// The returned `Arc` is cloned from under the internal read lock.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    pub fn get_analyzer(&self, field: &str) -> Arc<dyn Analyzer> {
        let guard = self.field_analyzers.read();
        guard
            .get(field)
            .cloned()
            .unwrap_or_else(|| self.default_analyzer.clone())
    }

    /// Get the default analyzer.
    pub fn default_analyzer(&self) -> &Arc<dyn Analyzer> {
        &self.default_analyzer
    }

    /// Analyze text with the analyzer for the given field.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to determine which analyzer to use
    /// * `text` - The text to analyze
    pub fn analyze_field(&self, field: &str, text: &str) -> Result<TokenStream> {
        self.get_analyzer(field).analyze(text)
    }
}

impl Analyzer for PerFieldAnalyzer {
    fn analyze(&self, text: &str) -> Result<TokenStream> {
        // When used as a regular Analyzer, use the default analyzer
        self.default_analyzer.analyze(text)
    }

    fn name(&self) -> &'static str {
        "PerFieldAnalyzer"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::analyzer::keyword::KeywordAnalyzer;
    use crate::analysis::analyzer::standard::StandardAnalyzer;

    #[test]
    fn test_per_field_analyzer() {
        let analyzer = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new().unwrap()));
        analyzer.add_analyzer("id", Arc::new(KeywordAnalyzer::new()));
        analyzer.add_analyzer("category", Arc::new(KeywordAnalyzer::new()));

        // Test that different fields use different analyzers
        let text = "Hello World";

        // Default analyzer (StandardAnalyzer) lowercases and tokenizes
        let tokens: Vec<_> = analyzer.analyze_field("title", text).unwrap().collect();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "hello");
        assert_eq!(tokens[1].text, "world");

        // KeywordAnalyzer keeps as single token (not lowercased by default)
        let tokens: Vec<_> = analyzer.analyze_field("id", text).unwrap().collect();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "Hello World");

        // Another field with KeywordAnalyzer
        let tokens: Vec<_> = analyzer.analyze_field("category", text).unwrap().collect();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "Hello World");
    }

    #[test]
    fn test_default_analyzer_when_field_not_configured() {
        let analyzer = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new().unwrap()));

        let text = "Hello World";
        let tokens: Vec<_> = analyzer
            .analyze_field("unknown_field", text)
            .unwrap()
            .collect();

        // Should use default StandardAnalyzer
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "hello");
        assert_eq!(tokens[1].text, "world");
    }

    #[test]
    fn test_as_analyzer_trait() {
        let analyzer = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new().unwrap()));
        analyzer.add_analyzer("id", Arc::new(KeywordAnalyzer::new()));

        // When used as Analyzer trait, should use default analyzer
        let text = "Hello World";
        let tokens: Vec<_> = analyzer.analyze(text).unwrap().collect();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "hello");
        assert_eq!(tokens[1].text, "world");
    }
}
