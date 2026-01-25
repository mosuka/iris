//! # Iris
//!
//! A fast, featureful full-text search library for Rust, inspired by Whoosh.
//!
//! ## Features
//!
//! - Pure Rust implementation
//! - Fast indexing and searching
//! - Flexible text analysis pipeline
//! - Pluggable storage backends
//! - Multiple query types
//! - BM25 scoring
// Core modules
pub mod analysis;
pub mod data;
pub mod embedding;
pub mod engine;
pub mod error;
pub mod lexical;
pub mod maintenance;
pub mod spelling;
pub mod storage;
pub mod util;
pub mod vector;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
