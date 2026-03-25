//! Python wrappers for the Laurus analysis pipeline.
//!
//! Mirrors the `synonym_graph_filter.rs` example:
//!
//! ```python
//! syn_dict = laurus.SynonymDictionary()
//! syn_dict.add_synonym_group(["ml", "machine learning"])
//!
//! tokenizer = laurus.WhitespaceTokenizer()
//! tokens    = tokenizer.tokenize("ml tutorial")
//!
//! filt = laurus.SynonymGraphFilter(syn_dict, keep_original=True, boost=0.8)
//! for token in filt.apply(tokens):
//!     print(token.text, token.position, token.boost)
//! ```

use crate::errors::laurus_err;
use laurus::analysis::synonym::dictionary::SynonymDictionary;
use laurus::analysis::token_filter::Filter;
use laurus::analysis::token_filter::synonym_graph::SynonymGraphFilter;
use laurus::analysis::tokenizer::Tokenizer;
use laurus::analysis::tokenizer::whitespace::WhitespaceTokenizer;
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// A single token produced by the analysis pipeline.
///
/// Attributes:
///     text (str): The token text.
///     position (int): Position in the token stream.
///     start_offset (int): Character start offset in the original text.
///     end_offset (int): Character end offset in the original text.
///     boost (float): Score boost factor (1.0 = no adjustment).
///     stopped (bool): Whether this token has been removed by a stop filter.
///     position_increment (int): Difference from the previous token's position.
///     position_length (int): Number of positions spanned by this token.
#[pyclass(name = "Token")]
pub struct PyToken {
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub position: usize,
    #[pyo3(get)]
    pub start_offset: usize,
    #[pyo3(get)]
    pub end_offset: usize,
    #[pyo3(get)]
    pub boost: f32,
    #[pyo3(get)]
    pub stopped: bool,
    #[pyo3(get)]
    pub position_increment: usize,
    #[pyo3(get)]
    pub position_length: usize,
}

#[pymethods]
impl PyToken {
    fn __repr__(&self) -> String {
        format!(
            "Token(text='{}', position={}, boost={:.2}, pos_inc={}, pos_len={})",
            self.text, self.position, self.boost, self.position_increment, self.position_length
        )
    }
}

impl From<laurus::analysis::token::Token> for PyToken {
    fn from(t: laurus::analysis::token::Token) -> Self {
        Self {
            text: t.text,
            position: t.position,
            start_offset: t.start_offset,
            end_offset: t.end_offset,
            boost: t.boost,
            stopped: t.stopped,
            position_increment: t.position_increment,
            position_length: t.position_length,
        }
    }
}

// ---------------------------------------------------------------------------
// SynonymDictionary
// ---------------------------------------------------------------------------

/// A dictionary of synonym groups used by [`SynonymGraphFilter`].
///
/// ## Example
///
/// ```python
/// syn_dict = laurus.SynonymDictionary()
/// syn_dict.add_synonym_group(["ml", "machine learning"])
/// syn_dict.add_synonym_group(["ai", "artificial intelligence"])
/// ```
#[pyclass(name = "SynonymDictionary")]
pub struct PySynonymDictionary {
    pub inner: SynonymDictionary,
}

#[pymethods]
impl PySynonymDictionary {
    /// Create an empty synonym dictionary.
    #[new]
    pub fn new() -> PyResult<Self> {
        SynonymDictionary::new(None)
            .map(|inner| Self { inner })
            .map_err(laurus_err)
    }

    /// Add a bidirectional synonym group.
    ///
    /// All terms in the group are treated as synonyms of each other.
    ///
    /// Args:
    ///     terms: List of synonym strings (e.g. `["ml", "machine learning"]`).
    pub fn add_synonym_group(&mut self, terms: Vec<String>) {
        self.inner.add_synonym_group(terms);
    }

    fn __repr__(&self) -> String {
        format!(
            "SynonymDictionary(max_phrase_length={})",
            self.inner.max_phrase_length()
        )
    }
}

// ---------------------------------------------------------------------------
// WhitespaceTokenizer
// ---------------------------------------------------------------------------

/// Splits text on whitespace boundaries.
///
/// ## Example
///
/// ```python
/// tokenizer = laurus.WhitespaceTokenizer()
/// tokens = tokenizer.tokenize("ml tutorial")
/// ```
#[pyclass(name = "WhitespaceTokenizer")]
pub struct PyWhitespaceTokenizer {
    inner: WhitespaceTokenizer,
}

#[pymethods]
impl PyWhitespaceTokenizer {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: WhitespaceTokenizer,
        }
    }

    /// Tokenize a text string and return a list of [`Token`] objects.
    pub fn tokenize(&self, text: &str) -> PyResult<Vec<PyToken>> {
        self.inner
            .tokenize(text)
            .map(|stream| stream.map(PyToken::from).collect())
            .map_err(laurus_err)
    }

    fn __repr__(&self) -> String {
        "WhitespaceTokenizer()".to_string()
    }
}

// ---------------------------------------------------------------------------
// SynonymGraphFilter
// ---------------------------------------------------------------------------

/// Token filter that expands tokens with their synonyms from a
/// [`SynonymDictionary`].
///
/// Multi-word synonyms are represented as a position graph so that downstream
/// indexing and searching correctly handles phrase queries.
///
/// ## Example
///
/// ```python
/// syn_dict = laurus.SynonymDictionary()
/// syn_dict.add_synonym_group(["ml", "machine learning"])
///
/// # Without boost — all synonyms have the same weight
/// filt = laurus.SynonymGraphFilter(syn_dict)
/// for token in filt.apply(tokenizer.tokenize("ml tutorial")):
///     print(token.text, token.boost)
///
/// # With boost — synonyms have reduced weight (0.8)
/// filt_boosted = laurus.SynonymGraphFilter(syn_dict, keep_original=True, boost=0.8)
/// ```
#[pyclass(name = "SynonymGraphFilter")]
pub struct PySynonymGraphFilter {
    inner: SynonymGraphFilter,
}

#[pymethods]
impl PySynonymGraphFilter {
    /// Create a new synonym graph filter.
    ///
    /// Args:
    ///     dictionary: The [`SynonymDictionary`] to use for expansion.
    ///     keep_original: Whether to retain the original token alongside synonyms (default True).
    ///     boost: Weight multiplier for synonym tokens (0.0–1.0, default 1.0 = no adjustment).
    #[new]
    #[pyo3(signature = (dictionary, *, keep_original=true, boost=1.0))]
    pub fn new(dictionary: &PySynonymDictionary, keep_original: bool, boost: f32) -> Self {
        let mut filt = SynonymGraphFilter::new(dictionary.inner.clone(), keep_original);
        if (boost - 1.0f32).abs() > f32::EPSILON {
            filt = filt.with_boost(boost);
        }
        Self { inner: filt }
    }

    /// Apply the synonym filter to a list of tokens.
    ///
    /// Args:
    ///     tokens: A list of [`Token`] objects (e.g. from `WhitespaceTokenizer.tokenize()`).
    ///
    /// Returns:
    ///     A list of expanded [`Token`] objects.
    pub fn apply(&self, tokens: Vec<PyRef<PyToken>>) -> PyResult<Vec<PyToken>> {
        // Reconstruct Rust Tokens from PyToken references
        let rust_tokens: Vec<laurus::analysis::token::Token> = tokens
            .iter()
            .map(|pt| {
                laurus::analysis::token::Token::new(pt.text.clone(), pt.position)
                    .with_boost(pt.boost)
                    .with_position_increment(pt.position_increment)
                    .with_position_length(pt.position_length)
            })
            .collect();

        let stream: Box<dyn Iterator<Item = laurus::analysis::token::Token> + Send> =
            Box::new(rust_tokens.into_iter());

        self.inner
            .filter(stream)
            .map(|out| out.map(PyToken::from).collect())
            .map_err(laurus_err)
    }

    fn __repr__(&self) -> String {
        "SynonymGraphFilter()".to_string()
    }
}
