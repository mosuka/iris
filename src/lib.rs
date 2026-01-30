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
// Core modules
pub mod analysis;
mod data;
pub mod embedding;
mod engine;
mod error;
pub mod lexical;
mod maintenance;
pub mod spelling;
pub mod storage;
mod util;
pub mod vector;

// Re-exports for the public API
pub use analysis::analyzer::analyzer::Analyzer;
pub use data::{DataValue, Document};
#[cfg(feature = "embeddings-candle")]
pub use embedding::candle_bert_embedder::CandleBertEmbedder;
#[cfg(feature = "embeddings-multimodal")]
pub use embedding::candle_clip_embedder::CandleClipEmbedder;
pub use embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
#[cfg(feature = "embeddings-openai")]
pub use embedding::openai_embedder::OpenAIEmbedder;
pub use embedding::per_field::PerFieldEmbedder;
pub use embedding::precomputed::PrecomputedEmbedder;
pub use engine::Engine;
pub use engine::config::{FieldConfig, IndexConfig};
pub use engine::search::{FusionAlgorithm, SearchRequest, SearchRequestBuilder, SearchResult};
pub use error::{IrisError, Result};
pub use maintenance::deletion::DeletionConfig;
pub use storage::{Storage, StorageConfig, StorageFactory};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
