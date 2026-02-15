//! High-level lexical search engine that combines indexing and searching.
//!
//! This module provides the core `LexicalStore` implementation.

pub mod config;

use std::sync::Arc;

use crate::analysis::analyzer::analyzer::Analyzer;
use crate::error::Result;
use crate::lexical::core::document::Document;
use crate::lexical::index::LexicalIndex;
use crate::lexical::index::factory::LexicalIndexFactory;
use crate::lexical::index::inverted::InvertedIndexStats;
use crate::lexical::query::LexicalSearchResults;
use crate::lexical::search::searcher::{LexicalSearchRequest, LexicalSearcher};
use crate::lexical::store::config::LexicalIndexConfig;
use crate::lexical::writer::LexicalIndexWriter;
use crate::storage::Storage;
use parking_lot::Mutex;
use parking_lot::RwLock;

/// A high-level lexical search engine that provides both indexing and searching capabilities.
///
/// The `LexicalStore` wraps a `LexicalIndex` trait object and provides a simplified,
/// unified interface for all lexical search operations. It manages the complexity of
/// coordinating between readers and writers while maintaining efficiency through caching.
///
/// # Features
///
/// - **Writer caching**: The writer is created on-demand and cached until commit
/// - **Reader invalidation**: Readers are automatically invalidated after commits/optimizations
/// - **Index abstraction**: Works with any `LexicalIndex` implementation (Inverted, etc.)
/// - **Simplified workflow**: Handles the lifecycle of readers and writers automatically
///
/// # Caching Strategy
///
/// - **Writer**: Created on first write operation, cached until `commit()` is called
/// - **Reader**: Invalidated after `commit()` or `optimize()` to ensure fresh data
/// - This design ensures that you always read committed data while minimizing object creation
///
/// # Usage Example
///
/// ```rust,no_run
/// use iris::lexical::core::document::Document;
/// use iris::lexical::store::LexicalStore;
/// use iris::lexical::store::config::LexicalIndexConfig;
/// use iris::lexical::search::searcher::LexicalSearchRequest;
/// use iris::storage::memory::{MemoryStorage, MemoryStorageConfig};
/// use std::sync::Arc;
///
/// // Create storage and engine
/// let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
/// let config = LexicalIndexConfig::default();
/// let engine = LexicalStore::new(storage, config).unwrap();
///
/// // Add documents
/// let doc = Document::builder()
///     .add_text("title", "Rust Programming")
///     .build();
/// engine.upsert_document(1, doc).unwrap();
/// engine.commit().unwrap();
///
/// // Search using DSL string
/// let results = engine.search(LexicalSearchRequest::from_dsl("title:rust")).unwrap();
/// ```
pub struct LexicalStore {
    /// The underlying lexical index.
    index: Box<dyn LexicalIndex>,
    writer_cache: Mutex<Option<Box<dyn LexicalIndexWriter>>>,
    searcher_cache: RwLock<Option<Box<dyn LexicalSearcher>>>,
}

impl std::fmt::Debug for LexicalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LexicalStore")
            .field("index", &self.index)
            .finish()
    }
}

impl LexicalStore {
    /// Create a new lexical search engine with the given storage and configuration.
    ///
    /// This constructor creates a `LexicalIndex` internally using the provided storage
    /// and configuration, then wraps it with lazy-initialized caches for the reader,
    /// writer, and searcher.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend for persisting index data
    /// * `config` - Configuration for the lexical index (schema, analyzer, etc.)
    ///
    /// # Returns
    ///
    /// Returns a new `LexicalStore` instance.
    ///
    /// # Example with Memory Storage
    ///
    /// ```rust,no_run
    /// use iris::lexical::store::LexicalStore;
    /// use iris::lexical::store::config::LexicalIndexConfig;
    /// use iris::storage::{Storage, StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// use std::sync::Arc;
    ///
    /// let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// let storage = StorageFactory::create(storage_config).unwrap();
    /// let engine = LexicalStore::new(storage, LexicalIndexConfig::default()).unwrap();
    /// ```
    ///
    /// # Example with File Storage
    ///
    /// ```rust,no_run
    /// use iris::lexical::store::LexicalStore;
    /// use iris::lexical::store::config::LexicalIndexConfig;
    /// use iris::storage::{Storage, StorageConfig, StorageFactory};
    /// use iris::storage::file::FileStorageConfig;
    /// use std::sync::Arc;
    ///
    /// let storage_config = StorageConfig::File(FileStorageConfig::new("/tmp/index"));
    /// let storage = StorageFactory::create(storage_config).unwrap();
    /// let engine = LexicalStore::new(storage, LexicalIndexConfig::default()).unwrap();
    /// ```
    pub fn new(storage: Arc<dyn Storage>, config: LexicalIndexConfig) -> Result<Self> {
        let index = LexicalIndexFactory::open_or_create(storage, config)?;
        Ok(Self {
            index,
            writer_cache: Mutex::new(None),
            searcher_cache: RwLock::new(None),
        })
    }

    /// Upsert a document with a specific internal ID.
    ///
    /// The caller is responsible for doc_id generation (via [`DocumentLog`](crate::store::log::DocumentLog)).
    /// Changes are not persisted until you call `commit()`.
    pub fn upsert_document(&self, internal_id: u64, doc: Document) -> Result<()> {
        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }
        guard.as_mut().unwrap().upsert_document(internal_id, doc)
    }

    /// Delete a document by internal ID.
    ///
    /// Note: You must call `commit()` to persist the changes.
    pub(crate) fn delete_document_by_internal_id(&self, internal_id: u64) -> Result<()> {
        // Delete from doc store
        // UnifiedDocumentStore doesn't have delete? It might have a bitmap or tombstone.
        // Actually deletion is usually handled by a separate DeletionPolicy / Bitmap.
        // But if DocumentStore is source of truth, maybe it has delete?
        // Let's leave DocumentStore alone for deletion for now (soft delete/vacuum handled elsewhere).
        // Or check if it has delete.
        // Assuming no delete in doc store for now (compaction handles it).

        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }
        guard.as_mut().unwrap().delete_document(internal_id)
    }

    /// Find all internal document IDs for a given term (field:value).
    ///
    /// This searches both the uncommitted in-memory buffer (via Writer) and
    /// the committed index (via Searcher).
    pub(crate) fn find_doc_ids_by_term(&self, field: &str, term: &str) -> Result<Vec<u64>> {
        let mut ids = Vec::new();
        let guard = self.writer_cache.lock();

        // 1. Check writer (NRT - Uncommitted)
        if let Some(writer) = guard.as_ref()
            && let Some(writer_ids) = writer.find_doc_ids_by_term(field, term)?
        {
            ids.extend(writer_ids);
        }

        // 2. Check reader (Committed)
        use crate::lexical::query::Query;
        use crate::lexical::query::term::TermQuery;

        let query = Box::new(TermQuery::new(field, term)) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query)
            .limit(usize::MAX) // Retrieve all matches
            .load_documents(false);

        // Safe to call search while holding writer lock as long as lock order is respected (Writer -> Searcher)
        // search() acquires searcher_cache lock.
        // commit() acquires writer_cache lock THEN searcher_cache lock (via refresh).
        // So we are consistent.
        let results = self.search(request)?;
        for hit in results.hits {
            if !ids.contains(&hit.doc_id) {
                // Check if marked as deleted in pending set
                let is_deleted = if let Some(writer) = guard.as_ref() {
                    writer.is_updated_deleted(hit.doc_id)
                } else {
                    false
                };

                if !is_deleted {
                    ids.push(hit.doc_id);
                }
            }
        }

        Ok(ids)
    }

    /// Commit any pending changes to the index.
    ///
    /// This method flushes all pending write operations to storage and makes them
    /// visible to subsequent searches. The cached writer is consumed and the reader
    /// cache is invalidated to ensure fresh data on the next search.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the commit fails.
    ///
    /// # Important
    ///
    /// - All write operations (add, update, delete) are not persisted until commit
    /// - After commit, the reader cache is invalidated automatically
    /// - The writer cache is cleared and will be recreated on the next write operation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use iris::lexical::core::document::Document;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// # use std::sync::Arc;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default()).unwrap();
    ///
    /// // Add multiple documents
    /// for i in 0..10 {
    ///     let doc = Document::builder()
    ///         .add_text("id", &i.to_string())
    ///         .add_text("title", &format!("Document {}", i))
    ///         .build();
    ///     engine.upsert_document(i + 1, doc).unwrap();
    /// }
    ///
    /// // Commit all changes at once
    /// engine.commit().unwrap();
    /// ```
    pub fn commit(&self) -> Result<()> {
        if let Some(mut writer) = self.writer_cache.lock().take() {
            writer.commit()?;
        }
        self.index.refresh()?;
        *self.searcher_cache.write() = None;
        Ok(())
    }

    /// Optimize the index.
    ///
    /// This method triggers index optimization, which typically involves merging smaller
    /// index segments into larger ones to improve search performance and reduce storage overhead.
    /// After optimization, the reader and searcher caches are invalidated to reflect the optimized structure.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if optimization fails.
    ///
    /// # Performance Considerations
    ///
    /// - Optimization can be I/O and CPU intensive for large indexes
    /// - It's typically performed during off-peak hours or maintenance windows
    /// - The benefits include faster search performance and reduced storage space
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use iris::lexical::core::document::Document;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// # use std::sync::Arc;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let mut engine = LexicalStore::new(storage, LexicalIndexConfig::default()).unwrap();
    ///
    /// // Add and commit many documents
    /// for i in 0..1000 {
    ///     let doc = Document::builder()
    ///         .add_text("id", &i.to_string())
    ///         .build();
    ///     engine.upsert_document(i + 1, doc).unwrap();
    /// }
    /// engine.commit().unwrap();
    ///
    /// // Optimize the index for better performance
    /// engine.optimize().unwrap();
    /// ```
    pub fn optimize(&self) -> Result<()> {
        self.index.optimize()?;
        *self.searcher_cache.write() = None;
        Ok(())
    }

    /// Refresh the reader to see latest changes.
    pub fn refresh(&self) -> Result<()> {
        *self.searcher_cache.write() = None;
        Ok(())
    }

    /// Get index statistics.
    pub fn stats(&self) -> Result<InvertedIndexStats> {
        let mut stats = self.index.stats()?;

        // Add pending docs from writer cache
        let guard = self.writer_cache.lock();
        if let Some(writer) = guard.as_ref() {
            stats.doc_count += writer.pending_docs();
        }

        Ok(stats)
    }

    /// Get the storage backend.
    pub fn storage(&self) -> &Arc<dyn Storage> {
        self.index.storage()
    }

    /// Search with the given request.
    ///
    /// This method executes a search query against the index using a cached searcher
    /// for improved performance.
    ///
    /// # Arguments
    ///
    /// * `request` - The search request containing the query and search parameters
    ///
    /// # Returns
    ///
    /// Returns `SearchResults` containing matching documents, scores, and metadata.
    ///
    /// # Example with TermQuery
    ///
    /// ```rust,no_run
    /// use iris::lexical::core::document::Document;
    /// use iris::lexical::search::searcher::LexicalSearchRequest;
    /// use iris::lexical::query::term::TermQuery;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// # use std::sync::Arc;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default()).unwrap();
    /// # let doc = Document::builder().add_text("title", "hello world").build();
    /// # engine.upsert_document(1, doc).unwrap();
    /// # engine.commit().unwrap();
    ///
    /// // Using DSL string
    /// let request = LexicalSearchRequest::from_dsl("title:hello")
    ///     .limit(10)
    ///     .min_score(0.5);
    /// let results = engine.search(request).unwrap();
    ///
    /// println!("Found {} documents", results.total_hits);
    /// for hit in results.hits {
    ///     println!("Doc ID: {}, Score: {}", hit.doc_id, hit.score);
    /// }
    /// ```
    ///
    /// # Example with QueryParser
    ///
    /// ```rust,no_run
    /// use iris::lexical::query::parser::QueryParser;
    /// use iris::lexical::search::searcher::LexicalSearchRequest;
    /// # use iris::lexical::core::document::Document;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// use iris::analysis::analyzer::standard::StandardAnalyzer;
    /// # use std::sync::Arc;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default()).unwrap();
    ///
    /// let analyzer = Arc::new(StandardAnalyzer::default());
    /// let parser = QueryParser::new(analyzer).with_default_field("title");
    /// let query = parser.parse("rust AND programming").unwrap();
    /// let results = engine.search(LexicalSearchRequest::new(query)).unwrap();
    /// ```
    pub fn search(&self, request: LexicalSearchRequest) -> Result<LexicalSearchResults> {
        let results = {
            let guard = self.searcher_cache.read();
            if let Some(ref searcher) = *guard {
                searcher.search(request)?
            } else {
                drop(guard);
                let mut guard = self.searcher_cache.write();
                if guard.is_none() {
                    *guard = Some(self.index.searcher()?);
                }
                guard.as_ref().unwrap().search(request)?
            }
        };

        Ok(results)
    }

    /// Count documents matching the request.
    ///
    /// Uses a cached searcher for improved performance.
    /// If `min_score` is specified in the request parameters, only documents
    /// with a score equal to or greater than the threshold are counted.
    ///
    /// # Arguments
    ///
    /// * `request` - Search request containing the query and search parameters.
    ///   Use `LexicalSearchRequest::new(query)` to create a request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::lexical::search::searcher::LexicalSearchRequest;
    /// # use iris::storage::memory::MemoryStorage;
    /// # use iris::storage::memory::MemoryStorageConfig;
    /// # use std::sync::Arc;
    /// # let config = LexicalIndexConfig::default();
    /// # let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
    /// # let engine = LexicalStore::new(storage, config).unwrap();
    /// // Count all matching documents
    /// let count = engine.count(LexicalSearchRequest::from_dsl("title:hello")).unwrap();
    /// println!("Found {} documents", count);
    ///
    /// // Count with min_score threshold
    /// let count = engine.count(
    ///     LexicalSearchRequest::from_dsl("title:hello").min_score(0.5)
    /// ).unwrap();
    /// println!("Found {} documents with score >= 0.5", count);
    /// ```
    pub fn count(&self, request: LexicalSearchRequest) -> Result<u64> {
        {
            let guard = self.searcher_cache.read();
            if let Some(ref searcher) = *guard {
                return searcher.count(request);
            }
        }
        let mut guard = self.searcher_cache.write();
        if guard.is_none() {
            *guard = Some(self.index.searcher()?);
        }
        guard.as_ref().unwrap().count(request)
    }

    /// Close the search engine.
    pub fn close(&self) -> Result<()> {
        *self.writer_cache.lock() = None;
        *self.searcher_cache.write() = None;
        self.index.close()
    }

    /// Check if the engine is closed.
    pub fn is_closed(&self) -> bool {
        self.index.is_closed()
    }

    /// Get the analyzer used by this engine.
    ///
    /// Returns the analyzer from the underlying index reader.
    /// This is useful for query parsing and term normalization.
    ///
    /// # Returns
    ///
    /// Returns `Result<Arc<dyn Analyzer>>` containing the analyzer.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader cannot be created or the index type
    /// doesn't support analyzers.
    pub fn analyzer(&self) -> Result<Arc<dyn Analyzer>> {
        use crate::lexical::index::inverted::reader::InvertedIndexReader;

        let reader = self.index.reader()?;

        // Downcast to InvertedIndexReader to access analyzer
        if let Some(inverted_reader) = reader.as_any().downcast_ref::<InvertedIndexReader>() {
            Ok(Arc::clone(inverted_reader.analyzer()))
        } else {
            // For other index types, return StandardAnalyzer as default
            use crate::analysis::analyzer::standard::StandardAnalyzer;
            Ok(Arc::new(StandardAnalyzer::new()?))
        }
    }

    /// Create a query parser configured for this index.
    ///
    /// The parser uses the index's analyzer and default fields configuration.
    ///
    /// # Returns
    ///
    /// Returns `Result<QueryParser>` containing the configured parser.
    pub fn query_parser(&self) -> Result<crate::lexical::query::parser::QueryParser> {
        let analyzer = self.analyzer()?;
        let mut parser = crate::lexical::query::parser::QueryParser::new(analyzer);

        if let Ok(fields) = self.index.default_fields()
            && !fields.is_empty()
        {
            parser = parser.with_default_fields(fields);
        }

        Ok(parser)
    }

    /// Get the last processed WAL sequence number.
    pub fn last_wal_seq(&self) -> u64 {
        self.index.last_wal_seq()
    }

    /// Set the last processed WAL sequence number.
    ///
    /// If a writer is cached, it sets the sequence on the writer.
    /// Otherwise, it sets it on the underlying index.
    pub fn set_last_wal_seq(&self, seq: u64) -> Result<()> {
        if let Some(writer) = self.writer_cache.lock().as_mut() {
            writer.set_last_wal_seq(seq)?;
        } else {
            self.index.set_last_wal_seq(seq)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexical::query::Query;
    use crate::lexical::query::term::TermQuery;
    use crate::lexical::store::config::LexicalIndexConfig;
    use crate::storage::file::{FileStorage, FileStorageConfig};
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_document(title: &str, body: &str) -> Document {
        Document::builder()
            .add_text("title", title)
            .add_text("body", body)
            .build()
    }

    #[test]
    fn test_search_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        assert!(!engine.is_closed());
    }

    #[test]
    fn test_search_engine_in_memory() {
        let config = LexicalIndexConfig::default();
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let engine = LexicalStore::new(storage, config).unwrap();

        // Add some documents
        let docs = vec![
            create_test_document("Test Document 1", "Content of test document 1"),
            create_test_document("Test Document 2", "Content of test document 2"),
        ];
        for (i, doc) in docs.into_iter().enumerate() {
            engine.upsert_document((i + 1) as u64, doc).unwrap();
        }
        engine.commit().unwrap();

        // Search for documents
        let query = Box::new(TermQuery::new("title", "Test")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let _results = engine.search(request).unwrap();

        assert!(!engine.is_closed());
    }

    #[test]
    fn test_search_engine_open() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        // Create engine
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config.clone()).unwrap();
        engine.close().unwrap();

        // Open engine
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        assert!(!engine.is_closed());
    }

    #[test]
    fn test_upsert_document() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        let doc = create_test_document("Hello World", "This is a test document");
        engine.upsert_document(1, doc).unwrap();
        engine.commit().unwrap();

        let _stats = engine.stats().unwrap();
    }

    #[test]
    fn test_upsert_multiple_documents() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        let docs = vec![
            create_test_document("First Document", "Content of first document"),
            create_test_document("Second Document", "Content of second document"),
            create_test_document("Third Document", "Content of third document"),
        ];

        for (i, doc) in docs.into_iter().enumerate() {
            engine.upsert_document((i + 1) as u64, doc).unwrap();
        }
        engine.commit().unwrap();

        let _stats = engine.stats().unwrap();
    }

    #[test]
    fn test_search_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        let query = Box::new(TermQuery::new("title", "hello")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let results = engine.search(request).unwrap();

        assert_eq!(results.hits.len(), 0);
        assert_eq!(results.total_hits, 0);
        assert_eq!(results.max_score, 0.0);
    }

    #[test]
    fn test_search_with_documents() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        // Add some documents
        let docs = vec![
            create_test_document("Hello World", "This is a test document"),
            create_test_document("Goodbye World", "This is another test document"),
        ];
        for (i, doc) in docs.into_iter().enumerate() {
            engine.upsert_document((i + 1) as u64, doc).unwrap();
        }
        engine.commit().unwrap();

        // Search for documents
        let query = Box::new(TermQuery::new("title", "Hello")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let _results = engine.search(request).unwrap();
    }

    #[test]
    fn test_count_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        let query = Box::new(TermQuery::new("title", "hello")) as Box<dyn Query>;
        let count = engine.count(LexicalSearchRequest::new(query)).unwrap();

        // Should return 0 for empty index
        assert_eq!(count, 0);
    }

    #[test]
    fn test_engine_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        // Add a document
        let doc = create_test_document("Test Document", "Test content");
        engine.upsert_document(1, doc).unwrap();
        engine.commit().unwrap();

        // Refresh should not fail
        engine.refresh().unwrap();

        // Search should still work
        let query = Box::new(TermQuery::new("title", "Test")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let _results = engine.search(request).unwrap();
    }

    #[test]
    fn test_engine_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        let stats = engine.stats().unwrap();
        assert!(stats.last_modified > 0);
    }

    #[test]
    fn test_engine_close() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        assert!(!engine.is_closed());

        engine.close().unwrap();

        assert!(engine.is_closed());
    }

    #[test]
    fn test_search_request_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        let query = Box::new(TermQuery::new("title", "hello")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query)
            .limit(5)
            .min_score(0.5)
            .load_documents(false);

        let results = engine.search(request).unwrap();

        assert_eq!(results.hits.len(), 0);
        assert_eq!(results.total_hits, 0);
    }

    #[test]
    fn test_search_with_query_parser() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        // Add some documents with lowercase titles for testing
        let docs = vec![
            create_test_document("hello world", "This is a test document"),
            create_test_document("goodbye world", "This is another test document"),
        ];
        for (i, doc) in docs.into_iter().enumerate() {
            engine.upsert_document((i + 1) as u64, doc).unwrap();
        }
        engine.commit().unwrap();

        // Search with QueryParser (Lucene style)
        use crate::lexical::query::parser::QueryParser;
        let parser = QueryParser::with_standard_analyzer()
            .unwrap()
            .with_default_field("title");

        // QueryParser analyzes "Hello" to "hello" before creating TermQuery
        let query = parser.parse("Hello").unwrap();
        let results = engine.search(LexicalSearchRequest::new(query)).unwrap();

        // Should find the document
        assert_eq!(results.hits.len(), 1);
        assert_eq!(results.total_hits, 1);
    }

    #[test]
    fn test_search_field_with_string() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        // Search specific field
        use crate::analysis::analyzer::standard::StandardAnalyzer;
        use crate::lexical::query::parser::QueryParser;
        let analyzer = Arc::new(StandardAnalyzer::new().unwrap());
        let parser = QueryParser::new(analyzer);
        let query = parser.parse_field("title", "hello world").unwrap();
        let results = engine.search(LexicalSearchRequest::new(query)).unwrap();

        // Should not find anything (empty index)
        assert_eq!(results.hits.len(), 0);
    }

    #[test]
    fn test_find_doc_ids_by_term() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let engine = LexicalStore::new(storage, config).unwrap();

        // Index document with external ID
        let doc = Document::builder()
            .add_text("title", "Test Doc")
            .add_text("_id", "ext_1")
            .build();
        engine.upsert_document(1, doc).unwrap();
        engine.commit().unwrap();

        // Verify find_doc_ids_by_term
        let found_ids = engine.find_doc_ids_by_term("_id", "ext_1").unwrap();
        assert_eq!(found_ids, vec![1]);

        // Non-existent
        let not_found = engine.find_doc_ids_by_term("_id", "ext_999").unwrap();
        assert!(not_found.is_empty());
    }
}
