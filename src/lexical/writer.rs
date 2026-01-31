//! Lexical index writer trait and common types.
//!
//! This module defines the `LexicalIndexWriter` trait which all lexical index writer
//! implementations must follow. The primary implementation is `InvertedIndexWriter`.

use crate::error::Result;
use crate::lexical::core::analyzed::AnalyzedDocument;
use crate::lexical::core::document::Document;

/// Trait for lexical index writers.
///
/// This trait defines the common interface that all lexical index writer implementations
/// must follow. The primary implementation is `InvertedIndexWriter`.
///
/// # Example
///
/// ```rust,no_run
/// use iris::lexical::index::inverted::writer::{InvertedIndexWriter, InvertedIndexWriterConfig};
/// use iris::lexical::writer::LexicalIndexWriter;
/// use iris::storage::memory::{MemoryStorage, MemoryStorageConfig};
/// use iris::storage::StorageConfig;
/// use std::sync::Arc;
///
/// let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
/// let config = InvertedIndexWriterConfig::default();
/// let mut writer = InvertedIndexWriter::new(storage, config).unwrap();
///
/// // Use LexicalIndexWriter trait methods
/// // writer.add_document(doc).unwrap();
/// // writer.commit().unwrap();
/// ```
pub trait LexicalIndexWriter: Send + Sync + std::fmt::Debug {
    /// Add a document to the index with automatic ID assignment.
    /// Returns the assigned document ID.
    fn add_document(&mut self, doc: Document) -> Result<u64>;

    /// Upsert a document to the index with a specific document ID.
    fn upsert_document(&mut self, doc_id: u64, doc: Document) -> Result<()>;

    /// Add an already analyzed document to the index with automatic ID assignment.
    /// Returns the assigned document ID.
    ///
    /// This allows adding pre-analyzed documents that were processed
    /// using DocumentParser or from external tokenization systems.
    fn add_analyzed_document(&mut self, doc: AnalyzedDocument) -> Result<u64>;

    /// Upsert an already analyzed document to the index with a specific document ID.
    fn upsert_analyzed_document(&mut self, doc_id: u64, doc: AnalyzedDocument) -> Result<()>;

    /// Delete a document by ID.
    fn delete_document(&mut self, doc_id: u64) -> Result<()>;

    /// Commit all pending changes to the index.
    fn commit(&mut self) -> Result<()>;

    /// Rollback all pending changes.
    fn rollback(&mut self) -> Result<()>;

    /// Get the number of documents added since the last commit.
    fn pending_docs(&self) -> u64;

    /// Close the writer and release resources.
    fn close(&mut self) -> Result<()>;

    /// Check if the writer is closed.
    fn is_closed(&self) -> bool;

    /// Build a reader from the written index.
    ///
    /// This method allows creating a reader directly from the writer,
    /// enabling the "write-then-read" workflow used in hybrid search.
    fn build_reader(
        &self,
    ) -> Result<std::sync::Arc<dyn crate::lexical::reader::LexicalIndexReader>>;

    /// Get the next available document ID.
    fn next_doc_id(&self) -> u64;

    /// Find the internal document ID for a given term (field:value).
    ///
    /// This allows NRT (Near Real-Time) lookups for ID management.
    fn find_doc_id_by_term(&self, field: &str, term: &str) -> Result<Option<u64>>;

    /// Find all internal document IDs for a given term (field:value).
    ///
    /// This allows NRT lookups for multiple documents (chunks).
    fn find_doc_ids_by_term(&self, field: &str, term: &str) -> Result<Option<Vec<u64>>>;

    /// Set the last processed WAL sequence number.
    fn set_last_wal_seq(&mut self, _seq: u64) -> Result<()> {
        Ok(())
    }

    /// Check if a document is marked as deleted in the pending set.
    fn is_updated_deleted(&self, _doc_id: u64) -> bool {
        false
    }
}
