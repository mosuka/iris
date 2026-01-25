//! Hybrid search module combining lexical and vector search.
//!
//! This module provides unified search capabilities that combine:
//! - Lexical (keyword-based) search with BM25 scoring
//! - Vector (semantic) search with embeddings
//! - Configurable fusion algorithms (RRF, weighted sum, etc.)
//!
//! # Architecture
//!
//! The hybrid search module follows the same pattern as the lexical module:
//!
//! - **Core data structure**: `index` - Combines lexical and vector indexes
//! - **Configuration and types**: Configuration, statistics, and type definitions
//! - **Engine**: High-level interface for hybrid search operations
//! - **Writer**: Hybrid index writing functionality
//! - **Search submodule**: Search execution, scoring, and result merging
//!
//! # Example
//!
//! ```no_run
//! use iris::hybrid::engine::HybridEngine;
//! use iris::hybrid::search::searcher::{HybridSearchRequest, HybridSearchParams};
//! use iris::lexical::store::LexicalStore;
//! use iris::vector::store::VectorStore;
//! use iris::storage::memory::MemoryStorage;
//! use iris::error::Result;
//! use std::sync::Arc;
//!
//! async fn example(lexical_engine: LexicalStore, vector_engine: VectorStore) -> Result<()> {
//!     // Create storage
//!     let storage = Arc::new(MemoryStorage::default());
//!
//!     // Create hybrid search engine
//!     let engine = HybridEngine::new(storage, lexical_engine, vector_engine)?;
//!
//!     // Create search request
//!     let params = HybridSearchParams {
//!         keyword_weight: 0.6,
//!         vector_weight: 0.4,
//!         ..Default::default()
//!     };
//!     let request = HybridSearchRequest::new()
//!         .with_text("rust programming")
//!         .with_params(params);
//!
//!     // Execute search
//!     let results = engine.search(request).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! For a complete working example, see `examples/hybrid_search.rs`.

// Core data structure
pub mod core;
pub mod index; // Core hybrid index combining lexical and vector indexes

// Configuration and types
pub mod stats;

// High-level interface
pub mod engine;

// Writer and search modules
pub mod search; // Search execution submodule (contains request, params, results)
pub mod writer; // Index writer
