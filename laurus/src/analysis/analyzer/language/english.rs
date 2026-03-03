//! English language analyzer implementation.
//!
//! This analyzer provides English-specific text analysis with regex-based
//! tokenization and English stop word filtering.
//!
//! # Pipeline
//!
//! 1. RegexTokenizer (`\w+` pattern — matches sequences of word characters)
//! 2. LowercaseFilter
//! 3. StopFilter (33 common English stop words)
//!
//! # Examples
//!
//! ```ignore
//! use laurus::analysis::analyzer::analyzer::Analyzer;
//! use laurus::analysis::analyzer::language::english::EnglishAnalyzer;
//!
//! let analyzer = EnglishAnalyzer::new().unwrap();
//! let tokens: Vec<_> = analyzer.analyze("Hello the world and test").unwrap().collect();
//!
//! // "the" and "and" are filtered out
//! assert_eq!(tokens.len(), 3);
//! assert_eq!(tokens[0].text, "hello");
//! assert_eq!(tokens[1].text, "world");
//! assert_eq!(tokens[2].text, "test");
//! ```
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;

use crate::analysis::analyzer::analyzer::Analyzer;
use crate::analysis::analyzer::pipeline::PipelineAnalyzer;
use crate::analysis::token::TokenStream;
use crate::analysis::token_filter::lowercase::LowercaseFilter;
use crate::analysis::token_filter::stop::StopFilter;
use crate::analysis::tokenizer::regex::RegexTokenizer;
use crate::error::Result;

/// Analyzer optimized for English language text.
///
/// This analyzer uses regex-based tokenization to split English text using
/// the `\w+` pattern (matching sequences of word characters: letters, digits,
/// and underscores) and applies lowercase normalization and stop word removal
/// to produce normalized tokens suitable for indexing and search.
///
/// # Components
///
/// - **Tokenizer**: RegexTokenizer (`\w+` pattern — matches word characters)
/// - **Filters**: Lowercase + English stop words (33 common articles/prepositions/conjunctions)
///
/// # Examples
///
/// ```ignore
/// use laurus::analysis::analyzer::analyzer::Analyzer;
/// use laurus::analysis::analyzer::language::english::EnglishAnalyzer;
///
/// let analyzer = EnglishAnalyzer::new().unwrap();
/// let tokens: Vec<_> = analyzer.analyze("Hello the world and test").unwrap().collect();
///
/// // "the" and "and" are filtered out
/// assert_eq!(tokens.len(), 3);
/// assert_eq!(tokens[0].text, "hello");
/// ```
pub struct EnglishAnalyzer {
    inner: PipelineAnalyzer,
}

impl EnglishAnalyzer {
    /// Creates a new `EnglishAnalyzer` with the default English analysis pipeline.
    ///
    /// The pipeline consists of a [`RegexTokenizer`] for word boundary splitting,
    /// a [`LowercaseFilter`] for case normalization, and a [`StopFilter`] loaded
    /// with common English stop words.
    ///
    /// # Returns
    ///
    /// A new `EnglishAnalyzer` instance, or an error if the regex tokenizer
    /// pattern fails to compile.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying [`RegexTokenizer`] cannot be created
    /// (e.g., due to an invalid regex pattern).
    pub fn new() -> Result<Self> {
        let tokenizer = Arc::new(RegexTokenizer::new()?);
        let analyzer = PipelineAnalyzer::new(tokenizer)
            .add_filter(Arc::new(LowercaseFilter::new()))
            .add_filter(Arc::new(StopFilter::default()))
            .with_name("english".to_string());

        Ok(Self { inner: analyzer })
    }
}

impl Default for EnglishAnalyzer {
    fn default() -> Self {
        Self::new().expect("English analyzer should be creatable with default settings")
    }
}

impl Analyzer for EnglishAnalyzer {
    fn analyze(&self, text: &str) -> Result<TokenStream> {
        self.inner.analyze(text)
    }

    fn name(&self) -> &'static str {
        "english"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Debug for EnglishAnalyzer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnglishAnalyzer")
            .field("inner", &self.inner)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::token::Token;

    #[test]
    fn test_english_analyzer() {
        let analyzer = EnglishAnalyzer::new().unwrap();

        let tokens: Vec<Token> = analyzer
            .analyze("Hello the world and test")
            .unwrap()
            .collect();

        // "the" and "and" should be filtered out
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "hello");
        assert_eq!(tokens[1].text, "world");
        assert_eq!(tokens[2].text, "test");
    }

    #[test]
    fn test_english_analyzer_name() {
        let analyzer = EnglishAnalyzer::new().unwrap();

        assert_eq!(analyzer.name(), "english");
    }
}
