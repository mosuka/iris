//! gRPC service implementations.
//!
//! This module re-exports the concrete service types that implement the
//! generated tonic server traits:
//!
//! * [`document::DocumentService`] – document CRUD and commit operations.
//! * [`health::HealthService`]     – health-check endpoint.
//! * [`index::IndexService`]       – index creation and schema management.
//! * [`search::SearchService`]     – lexical, vector, and hybrid search.

pub mod document;
pub mod health;
pub mod index;
pub mod search;
