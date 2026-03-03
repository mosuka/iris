//! Char filter implementations for text normalization.
//!
//! This module provides filters that pre-process the text string before it is
//! passed to the tokenizer. This allows for normalization operations like
//! Unicode normalization or regex replacement.
//!
//! # Available Filters
//!
//! - [`unicode_normalize::UnicodeNormalizationCharFilter`] - Unicode normalization (NFC, NFD, etc.)
//! - [`pattern_replace::PatternReplaceCharFilter`] - Regex-based replacement
//! - [`japanese_iteration_mark::JapaneseIterationMarkCharFilter`] - Japanese iteration mark normalization
//! - [`mapping::MappingCharFilter`] - Character mapping replacement
//!
//! # Examples
//!
//! ```
//! use laurus::analysis::char_filter::CharFilter;
//! use laurus::analysis::char_filter::unicode_normalize::{UnicodeNormalizationCharFilter, NormalizationForm};
//!
//! let filter = UnicodeNormalizationCharFilter::new(NormalizationForm::NFKC);
//! let (normalized, _transformations) = filter.filter("ﬁne");
//! assert_eq!(normalized, "fine");
//! ```

/// Represents a character offset mapping between original and filtered text.
///
/// When a [`CharFilter`] modifies text (e.g., replacing characters, expanding ligatures,
/// or removing diacritics), the character positions in the filtered output no longer
/// correspond 1:1 to positions in the original input. A `Transformation` records one
/// such positional shift so that downstream components (tokenizer, highlighter, etc.)
/// can map offsets back to the original text.
///
/// Each transformation describes a contiguous region in the original text and the
/// corresponding region in the new (filtered) text that replaced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transformation {
    /// Byte offset of the start of the affected range in the original text (inclusive).
    pub original_start: usize,
    /// Byte offset of the end of the affected range in the original text (exclusive).
    pub original_end: usize,
    /// Byte offset of the start of the replacement range in the filtered text (inclusive).
    pub new_start: usize,
    /// Byte offset of the end of the replacement range in the filtered text (exclusive).
    pub new_end: usize,
}

impl Transformation {
    /// Creates a new `Transformation` that maps a range in the original text
    /// to a range in the filtered text.
    ///
    /// # Arguments
    ///
    /// * `original_start` - Byte offset of the start of the affected range in the
    ///   original text (inclusive).
    /// * `original_end` - Byte offset of the end of the affected range in the
    ///   original text (exclusive).
    /// * `new_start` - Byte offset of the start of the replacement range in the
    ///   filtered text (inclusive).
    /// * `new_end` - Byte offset of the end of the replacement range in the
    ///   filtered text (exclusive).
    ///
    /// # Returns
    ///
    /// A new `Transformation` recording the positional mapping.
    pub fn new(
        original_start: usize,
        original_end: usize,
        new_start: usize,
        new_end: usize,
    ) -> Self {
        Self {
            original_start,
            original_end,
            new_start,
            new_end,
        }
    }
}

/// Trait for character filters that transform text before tokenization.
///
/// Implementations can modify the text content and returns the modified text
/// along with a list of transformations that occurred.
pub trait CharFilter: Send + Sync {
    /// Apply this filter to the input text.
    ///
    /// # Arguments
    ///
    /// * `input` - The input text to filter
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - The filtered text.
    /// - A vector of `Transformation`s describing changes made.
    fn filter(&self, input: &str) -> (String, Vec<Transformation>);

    /// Get the name of this char filter.
    fn name(&self) -> &'static str;
}

pub mod japanese_iteration_mark;
pub mod mapping;
pub mod pattern_replace;
pub mod unicode_normalize;
