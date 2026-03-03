//! Document storage module.
//!
//! This module provides the unified document store, write-ahead log (WAL), and segment
//! management infrastructure used to persistently store and retrieve documents.
//!
//! # Submodules
//!
//! - [`document`] -- Segmented document storage (`UnifiedDocumentStore`) with binary segment
//!   files, a JSON manifest for segment metadata, and readers/writers for individual segments.
//! - [`log`] -- Write-ahead log for crash-safe document ingestion.

pub mod document;
pub mod log;
