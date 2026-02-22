//! Text analysis module for Laurus.
//!
//! This module provides comprehensive text analysis functionality for processing
//! and transforming text before indexing or searching. It includes:
//!
//! - **Tokenizers**: Break text into individual tokens
//! - **Token Filters**: Transform, filter, or augment token streams
//! - **Analyzers**: Combine tokenizers and filters into analysis pipelines
//! - **Synonyms**: Support for synonym expansion during analysis
//!
//! # Architecture
//!
//! The analysis pipeline follows a simple flow:
//!
//! ```text
//! Text → Tokenizer → Token Stream → Token Filters → Analyzed Tokens
//! ```
//!
//! # Examples
//!
//! ```
//! use laurus::analysis::analyzer::standard::StandardAnalyzer;
//! use laurus::analysis::analyzer::analyzer::Analyzer;
//!
//! let analyzer = StandardAnalyzer::new().unwrap();
//! let tokens: Vec<_> = analyzer.analyze("Hello World!").unwrap().collect();
//! // Tokens: ["hello", "world"]
//! ```
//!
//! # Modules
//!
//! - [`analyzer`]: Pre-built and custom text analyzers
//! - [`tokenizer`]: Text tokenization strategies
//! - [`token_filter`]: Token transformation and filtering
//! - [`token`]: Token representation and manipulation
//! - [`synonym`]: Synonym dictionary and graph building

pub mod analyzer;
pub mod char_filter;
pub mod synonym;
pub mod token;
pub mod token_filter;
pub mod tokenizer;

// Re-exports
pub use analyzer::analyzer::Analyzer;
pub use analyzer::keyword::KeywordAnalyzer;
pub use analyzer::per_field::PerFieldAnalyzer;
pub use analyzer::simple::SimpleAnalyzer;
pub use analyzer::standard::StandardAnalyzer;
pub use token_filter::Filter as TokenFilter;
pub use tokenizer::Tokenizer;
