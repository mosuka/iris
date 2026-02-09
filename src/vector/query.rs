//! Vector query module.
//!
//! This module provides query building and parsing for vector search:
//! - [`builder`] - Fluent API for constructing VectorSearchRequest
//! - [`parser`] - DSL parser for `field:~"text"` syntax

pub mod builder;
pub mod parser;

pub use builder::VectorSearchRequestBuilder;
pub use parser::VectorQueryParser;
