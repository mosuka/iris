//! Lexical search implementation using inverted indexes.
//!
//! This module provides lexical (keyword-based) search functionality through
//! inverted index structures, supporting BM25 scoring, phrase queries, and
//! various query types based on token matching.
//!
//! # Module Structure
//!
//! - `core`: Core data structures (posting, dictionary, segment, etc.)
//! - `index`: Index management (config, factory, traits, inverted, segment, maintenance)
//! - `search`: Search execution (scoring, features, result processing)
//! - `engine`: High-level engine interface
//! - `reader`: Index reader trait
//! - `writer`: Index writer trait

// Internal modules
// Internal modules
pub mod core;
pub mod index;
pub mod search;

pub mod reader;
pub mod store;
pub mod writer;

// Re-exports
pub use core::field::{FieldOption, NumericType, TextOption};
pub use core::parser::DocumentParser;
pub use index::config::InvertedIndexConfig;
pub use index::inverted::query::*;
pub use index::inverted::writer::{InvertedIndexWriter, InvertedIndexWriterConfig};
pub use reader::LexicalIndexReader;
pub use search::searcher::{LexicalSearchParams, LexicalSearchQuery, LexicalSearchRequest};
pub use store::LexicalStore;
pub use store::config::LexicalIndexConfig;
pub use writer::LexicalIndexWriter;
