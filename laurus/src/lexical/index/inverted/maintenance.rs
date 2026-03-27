//! Index maintenance operations for inverted indexes.
//!
//! This module provides maintenance functionality for inverted indexes:
//! - Background tasks for async operations
//! - Deletion management
//! - Optimization strategies
//! - Transaction support

#[cfg(not(target_arch = "wasm32"))]
pub mod background_tasks;
// pub mod deletion; // Moved to separate crate-level module
pub mod optimization;
pub mod transaction;
