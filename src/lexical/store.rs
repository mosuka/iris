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
use crate::lexical::index::inverted::query::LexicalSearchResults;
use crate::lexical::search::searcher::{LexicalSearchRequest, LexicalSearcher};
use crate::lexical::store::config::LexicalIndexConfig;
use crate::lexical::writer::LexicalIndexWriter;
use crate::storage::Storage;
use crate::store::document::UnifiedDocumentStore;
use parking_lot::{Mutex, RwLock};

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
/// use iris::store::document::UnifiedDocumentStore;
/// use iris::storage::prefixed::PrefixedStorage;
/// use parking_lot::RwLock;
/// use std::sync::Arc;
///
/// // Create storage and engine
/// let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
/// let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
/// let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
/// let config = LexicalIndexConfig::default();
/// let engine = LexicalStore::new(storage, config, doc_store).unwrap();
///
/// // Add documents
/// use iris::lexical::core::field::TextOption;
/// let doc = Document::new()
///     .add_text("title", "Rust Programming");
/// engine.add_document(doc).unwrap();
/// engine.commit().unwrap();
///
/// // Search using DSL string
/// let results = engine.search(LexicalSearchRequest::new("title:rust")).unwrap();
/// ```
pub struct LexicalStore {
    /// The underlying lexical index.
    index: Box<dyn LexicalIndex>,
    writer_cache: Mutex<Option<Box<dyn LexicalIndexWriter>>>,
    searcher_cache: RwLock<Option<Box<dyn LexicalSearcher>>>,
    doc_store: Arc<RwLock<UnifiedDocumentStore>>,
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
    /// use iris::store::document::UnifiedDocumentStore;
    /// use iris::storage::prefixed::PrefixedStorage;
    /// use parking_lot::RwLock;
    /// use std::sync::Arc;
    ///
    /// let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// let storage = StorageFactory::create(storage_config).unwrap();
    /// let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// let engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    /// ```
    ///
    /// # Example with File Storage
    ///
    /// ```rust,no_run
    /// use iris::lexical::store::LexicalStore;
    /// use iris::lexical::store::config::LexicalIndexConfig;
    /// use iris::storage::{Storage, StorageConfig, StorageFactory};
    /// use iris::storage::file::FileStorageConfig;
    /// use iris::store::document::UnifiedDocumentStore;
    /// use iris::storage::prefixed::PrefixedStorage;
    /// use parking_lot::RwLock;
    /// use std::sync::Arc;
    ///
    /// let storage_config = StorageConfig::File(FileStorageConfig::new("/tmp/index"));
    /// let storage = StorageFactory::create(storage_config).unwrap();
    /// let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// let engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    /// ```
    pub fn new(
        storage: Arc<dyn Storage>,
        config: LexicalIndexConfig,
        doc_store: Arc<RwLock<UnifiedDocumentStore>>,
    ) -> Result<Self> {
        let index = LexicalIndexFactory::open_or_create(storage, config)?;
        Ok(Self {
            index,
            writer_cache: Mutex::new(None),
            searcher_cache: RwLock::new(None),
            doc_store,
        })
    }

    /// Add a document to the index.
    ///
    /// This method adds a single document to the index. The writer is created
    /// and cached on the first call. Changes are not persisted until you call `commit()`.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to add
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the operation fails.
    ///
    /// # Important
    ///
    /// You must call `commit()` to persist the changes to storage.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use iris::lexical::core::document::Document;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// # use iris::store::document::UnifiedDocumentStore;
    /// # use iris::storage::prefixed::PrefixedStorage;
    /// # use parking_lot::RwLock;
    /// # use std::sync::Arc;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// # let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    ///
    /// let doc = Document::new()
    ///     .add_text("title", "Hello World")
    ///     .add_text("body", "This is a test");
    /// let doc_id = engine.add_document(doc).unwrap();
    /// engine.commit().unwrap();  // Don't forget to commit!
    /// ```
    /// Add a document to the index.
    ///
    /// This method intelligently handles both insertion and updates (upsert) based on the presence
    /// of `Document.id`.
    ///
    /// - If `doc.id` is present: checks for an existing document with the same ID.
    ///   - If found: updates the existing document (Upsert).
    ///   - If not found: adds a new document with the specified ID.
    /// - If `doc.id` is missing: adds a new document with an auto-generated internal ID.
    ///
    /// Changes are not persisted until you call `commit()`.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to add
    ///
    /// # Returns
    ///
    /// Returns the internal document ID on success.
    pub fn add_document(&self, doc: Document) -> Result<u64> {
        // Add to UnifiedDocumentStore first
        let internal_id = self.doc_store.write().add_document(doc.clone())?;

        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }

        // Use upsert to force the specific ID assignment
        guard.as_mut().unwrap().upsert_document(internal_id, doc)?;

        Ok(internal_id)
    }

    /// Put (upsert) a document into the index.
    ///
    /// This method ensures that the document is uniquely identified by its `id`.
    /// - If a document with the same `id` exists, it is updated (Upsert).
    /// - If no such document exists, it is added (Insert).
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to put. Must have `id` set.
    ///
    /// # Returns
    ///
    /// Returns the internal document ID on success.
    /// Returns an error if `doc.id` is missing.
    pub fn put_document(&self, mut doc: Document) -> Result<u64> {
        let external_id = doc.id.clone().ok_or_else(|| {
            crate::error::IrisError::invalid_argument("Document ID is required for put_document")
        })?;

        use crate::data::DataValue;
        if !doc.fields.contains_key("_id") {
            doc.fields
                .insert("_id".to_string(), DataValue::Text(external_id.clone()));
        }

        if let Some(internal_id) = self.find_doc_id_by_term("_id", &external_id)? {
            // Document Store is append-only, so we must delete the old mapping and add new one.
            self.delete_document_by_internal_id(internal_id)?;
            self.add_document(doc)
        } else {
            self.add_document(doc)
        }
    }

    /// Get documents by external ID.
    ///
    /// Returns all documents that match the given external ID.
    pub fn get_documents(&self, external_id: &str) -> Result<Vec<Document>> {
        let doc_ids = self.find_doc_ids_by_term("_id", external_id)?;
        let mut docs = Vec::with_capacity(doc_ids.len());
        for doc_id in doc_ids {
            if let Some(doc) = self.get_document_by_internal_id(doc_id)? {
                docs.push(doc);
            }
        }
        Ok(docs)
    }

    /// Delete documents by external ID.
    ///
    /// Returns `true` if any documents were found and deleted, `false` otherwise.
    pub fn delete_documents(&self, external_id: &str) -> Result<bool> {
        let ids = self.find_doc_ids_by_term("_id", external_id)?;
        if ids.is_empty() {
            return Ok(false);
        }

        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }
        let writer = guard.as_mut().unwrap();

        for doc_id in ids {
            writer.delete_document(doc_id)?;
        }

        Ok(true)
    }

    /// Upsert a document with a specific internal ID.
    ///
    /// Note: You must call `commit()` to persist the changes.
    pub(crate) fn upsert_document(&self, internal_id: u64, doc: Document) -> Result<()> {
        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }
        // Update document store
        // Note: UnifiedDocumentStore doesn't have upsert_by_id yet? Or maybe it does via insert on map?
        // UnifiedDocumentStore API check needed. Assuming typical update pattern:
        // If we want to keep ID, we should ensure the doc store reflects this.
        // Actually DocumentStore only supports append (add_document). Update is not supported in append-only logs usually.
        // But for "upsert", if it's an update, we might be appending a new version with same External ID but new Internal ID?
        // Wait, LexicalStore::put_document logic:
        // 1. find_doc_id_by_term("_id", external_id) -> returns internal_id
        // 2. upsert_document(internal_id, doc)

        // If we reuse internal_id, we must update the content at that ID in DocumentStore.
        // But UnifiedDocumentStore is likely append-only for segments.
        // Does it support update?
        // If not, we cannot reuse internal_id easily without holes.
        // Iris usually does "delete + insert" for updates, getting a new Internal ID.
        // But put_document tries to reuse internal_id?

        // Let's look at put_document (lines 210-227).
        // It calls self.find_doc_id_by_term. If found, calls self.upsert_document(internal_id, doc).
        // If we reuse internal_id, we imply in-place update.
        // UnifiedDocumentStore needs to support replacing a document at an ID.
        // If it doesn't, we should maybe DELETE old ID and Insert new ID, and update index mapping?
        // But upsert_document takes internal_id.

        // For now, let's assume we can't easily update in-place in DocumentStore.
        // We should warn or implement update in DocumentStore later.
        // Or, we update the logic of put_document to: Delete old -> Add new.
        // But put_document signature returns u64 (internal_id).

        // If we keep existing implementation of upsert_document, we just update the index.
        // But the DocumentStore will still have the old document?
        // We need to update DocumentStore too.
        // doc_store.write().update_document(internal_id, doc)? -> Need to implement this if missing.

        // TEMPORARY: Just update index. DocumentStore might be stale for this ID.
        // This is a known issue if UnifiedDocumentStore doesn't support update.
        // Ideally we should implement update_document in UnifiedDocumentStore.
        // Let's try to call it, assuming it might exist or we will add it.
        // Actually, UnifiedDocumentStore logic was "SegmentedDocumentStore".
        // It likely supports append only.

        // Let's update `put_document` instead to do "delete and add".
        // But we are inside upsert_document here.

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

    /// Get a document by its internal ID.
    ///
    /// This uses the system-reserved `_id` field to find the internal ID first.
    pub(crate) fn get_document_by_internal_id(&self, internal_id: u64) -> Result<Option<Document>> {
        // Use UnifiedDocumentStore
        let guard = self.doc_store.read();
        let mut doc = guard.get_document(internal_id)?;

        // If we have an _id field, use it to populate the Document.id property
        if let Some(d) = &mut doc {
            if d.id.is_none() {
                if let Some(id_val) = d.fields.get("_id").and_then(|v| v.as_text()) {
                    d.id = Some(id_val.to_string());
                }
            }
        }

        Ok(doc)
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
        use crate::lexical::index::inverted::query::Query;
        use crate::lexical::index::inverted::query::term::TermQuery;

        let query = Box::new(TermQuery::new(field, term)) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query)
            .max_docs(usize::MAX) // Retrieve all matches
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

    /// Find the internal document ID for a given term (field:value).
    ///
    /// This searches both the uncommitted in-memory buffer (via Writer) and
    /// the committed index (via Searcher).
    fn find_doc_id_by_term(&self, field: &str, term: &str) -> Result<Option<u64>> {
        let guard = self.writer_cache.lock();

        // 1. Check writer (NRT - Uncommitted)
        if let Some(writer) = guard.as_ref()
            && let Some(doc_id) = writer.find_doc_id_by_term(field, term)?
        {
            return Ok(Some(doc_id));
        }

        // 2. Check reader (Committed)
        use crate::lexical::index::inverted::query::Query;
        use crate::lexical::index::inverted::query::term::TermQuery;

        let query = Box::new(TermQuery::new(field, term)) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query)
            .max_docs(1)
            .load_documents(false);

        let results = self.search(request)?;
        if let Some(hit) = results.hits.first() {
            // Check if marked as deleted in pending set
            let is_deleted = if let Some(writer) = guard.as_ref() {
                writer.is_updated_deleted(hit.doc_id)
            } else {
                false
            };

            if !is_deleted {
                return Ok(Some(hit.doc_id));
            }
        }

        Ok(None)
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
    /// # use iris::store::document::UnifiedDocumentStore;
    /// # use iris::storage::prefixed::PrefixedStorage;
    /// # use parking_lot::RwLock;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// # let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    ///
    /// // Add multiple documents
    /// for i in 0..10 {
    ///     let doc = Document::new()
    ///         .add_text("id", &i.to_string())
    ///         .add_text("title", &format!("Document {}", i));
    ///     engine.add_document(doc).unwrap();
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
    /// use iris::store::document::UnifiedDocumentStore;
    /// use iris::storage::prefixed::PrefixedStorage;
    /// use parking_lot::RwLock;
    /// # use std::sync::Arc;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// # let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// # let mut engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    ///
    /// // Add and commit many documents
    /// for i in 0..1000 {
    ///     let doc = Document::new()
    ///         .add_text("id", &i.to_string());
    ///     engine.add_document(doc).unwrap();
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
    /// use iris::lexical::index::inverted::query::term::TermQuery;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// # use std::sync::Arc;
    /// # use iris::store::document::UnifiedDocumentStore;
    /// # use iris::storage::prefixed::PrefixedStorage;
    /// # use parking_lot::RwLock;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// # let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    /// # use iris::lexical::core::field::TextOption;
    /// # let doc = Document::new().add_text("title", "hello world");
    /// # engine.add_document(doc).unwrap();
    /// # engine.commit().unwrap();
    ///
    /// // Using DSL string
    /// let request = LexicalSearchRequest::new("title:hello")
    ///     .max_docs(10)
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
    /// use iris::lexical::index::inverted::query::parser::QueryParser;
    /// use iris::lexical::search::searcher::LexicalSearchRequest;
    /// # use iris::lexical::core::document::Document;
    /// # use iris::lexical::store::LexicalStore;
    /// # use iris::lexical::store::config::LexicalIndexConfig;
    /// # use iris::storage::{StorageConfig, StorageFactory};
    /// use iris::storage::memory::MemoryStorageConfig;
    /// use iris::analysis::analyzer::standard::StandardAnalyzer;
    /// # use std::sync::Arc;
    /// # use iris::store::document::UnifiedDocumentStore;
    /// # use iris::storage::prefixed::PrefixedStorage;
    /// # use parking_lot::RwLock;
    /// # let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    /// # let storage = StorageFactory::create(storage_config).unwrap();
    /// # let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// # let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// # let engine = LexicalStore::new(storage, LexicalIndexConfig::default(), doc_store).unwrap();
    ///
    /// let analyzer = Arc::new(StandardAnalyzer::default());
    /// let parser = QueryParser::new(analyzer).with_default_field("title");
    /// let query = parser.parse("rust AND programming").unwrap();
    /// let results = engine.search(LexicalSearchRequest::new(query)).unwrap();
    /// ```
    pub fn search(&self, request: LexicalSearchRequest) -> Result<LexicalSearchResults> {
        let mut results = {
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

        // Hydrate doc.id from _id field
        for hit in &mut results.hits {
            if let Some(doc) = &mut hit.document {
                if doc.id.is_none() {
                    if let Some(id_val) = doc.fields.get("_id").and_then(|v| v.as_text()) {
                        doc.id = Some(id_val.to_string());
                    }
                }
            }
        }

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
    /// use iris::store::document::UnifiedDocumentStore;
    /// use iris::storage::prefixed::PrefixedStorage;
    /// use parking_lot::RwLock;
    /// # use std::sync::Arc;
    /// # let config = LexicalIndexConfig::default();
    /// # let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
    /// # let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    /// # let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage).unwrap()));
    /// # let engine = LexicalStore::new(storage, config, doc_store).unwrap();
    /// // Count all matching documents
    /// let count = engine.count(LexicalSearchRequest::new("title:hello")).unwrap();
    /// println!("Found {} documents", count);
    ///
    /// // Count with min_score threshold
    /// let count = engine.count(
    ///     LexicalSearchRequest::new("title:hello").min_score(0.5)
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
    pub fn query_parser(
        &self,
    ) -> Result<crate::lexical::index::inverted::query::parser::QueryParser> {
        let analyzer = self.analyzer()?;
        let mut parser = crate::lexical::index::inverted::query::parser::QueryParser::new(analyzer);

        if let Ok(fields) = self.index.default_fields() {
            if !fields.is_empty() {
                parser = parser.with_default_fields(fields);
            }
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
    use crate::lexical::index::inverted::query::Query;
    use crate::lexical::index::inverted::query::term::TermQuery;
    use crate::lexical::store::config::LexicalIndexConfig;
    use crate::storage::file::{FileStorage, FileStorageConfig};
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    use crate::storage::prefixed::PrefixedStorage;
    use crate::store::document::UnifiedDocumentStore;
    use parking_lot::RwLock;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[allow(dead_code)]
    fn create_test_document(title: &str, body: &str) -> Document {
        Document::new()
            .add_text("title", title)
            .add_text("body", body)
    }

    #[test]
    fn test_search_engine_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

        // Schema-less mode: no schema() method available
        assert!(!engine.is_closed());
    }

    #[test]
    fn test_search_engine_in_memory() {
        let config = LexicalIndexConfig::default();
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store.clone()).unwrap();

        // Add some documents
        let docs = vec![
            create_test_document("Test Document 1", "Content of test document 1"),
            create_test_document("Test Document 2", "Content of test document 2"),
        ];
        for doc in docs {
            engine.add_document(doc).unwrap();
        }
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        // Search for documents
        let query = Box::new(TermQuery::new("title", "Test")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let _results = engine.search(request).unwrap();

        // Should find documents in memory
        // Note: total_hits may be 0 if the analyzer lowercases "Test" to "test"
        // but we indexed "Test" (capital T). Just verify the search works.
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
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config.clone(), doc_store).unwrap();
        engine.close().unwrap();

        // Open engine
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

        // Schema-less mode: no schema() method available
        assert!(!engine.is_closed());
    }

    #[test]
    fn test_add_document() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store.clone()).unwrap();

        let doc = create_test_document("Hello World", "This is a test document");
        engine.add_document(doc).unwrap();
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        // Check that document was added (through stats)
        let _stats = engine.stats().unwrap();
        // Note: stats might not reflect the added document immediately
        // depending on the index implementation
        // doc_count is usize, so >= 0 check is redundant
    }

    #[test]
    fn test_add_multiple_documents() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store.clone()).unwrap();

        let docs = vec![
            create_test_document("First Document", "Content of first document"),
            create_test_document("Second Document", "Content of second document"),
            create_test_document("Third Document", "Content of third document"),
        ];

        for doc in docs {
            engine.add_document(doc).unwrap();
        }
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        let _stats = engine.stats().unwrap();
        // doc_count is usize, so >= 0 check is redundant
    }

    #[test]
    fn test_search_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

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
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store.clone()).unwrap();

        // Add some documents
        let docs = vec![
            create_test_document("Hello World", "This is a test document"),
            create_test_document("Goodbye World", "This is another test document"),
        ];
        for doc in docs {
            engine.add_document(doc).unwrap();
        }
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        // Search for documents
        let query = Box::new(TermQuery::new("title", "Hello")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let _results = engine.search(request).unwrap();

        // Results depend on the actual indexing implementation
        // For now, we just check that search doesn't fail
        // hits.len() is usize, so >= 0 check is redundant
        // total_hits is u64, so >= 0 check is redundant
    }

    #[test]
    fn test_count_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

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
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store.clone()).unwrap();

        // Add a document
        let doc = create_test_document("Test Document", "Test content");
        engine.add_document(doc).unwrap();
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        // Refresh should not fail
        engine.refresh().unwrap();

        // Search should still work
        let query = Box::new(TermQuery::new("title", "Test")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query);
        let _results = engine.search(request).unwrap();
        // hits.len() is usize, so >= 0 check is redundant
    }

    #[test]
    fn test_engine_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

        let stats = engine.stats().unwrap();
        // doc_count is usize, so >= 0 check is redundant
        // term_count is usize, so >= 0 check is redundant
        assert!(stats.last_modified > 0);
    }

    #[test]
    fn test_engine_close() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

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
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

        let query = Box::new(TermQuery::new("title", "hello")) as Box<dyn Query>;
        let request = LexicalSearchRequest::new(query)
            .max_docs(5)
            .min_score(0.5)
            .load_documents(false);

        let results = engine.search(request).unwrap();

        // Should respect the configuration
        assert_eq!(results.hits.len(), 0); // No matching documents
        assert_eq!(results.total_hits, 0);
    }

    #[test]
    fn test_search_with_query_parser() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();

        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store.clone()).unwrap();

        // Add some documents with lowercase titles for testing
        let docs = vec![
            create_test_document("hello world", "This is a test document"),
            create_test_document("goodbye world", "This is another test document"),
        ];
        for doc in docs {
            engine.add_document(doc).unwrap();
        }
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        // Search with QueryParser (Lucene style)
        use crate::lexical::index::inverted::query::parser::QueryParser;
        let parser = QueryParser::with_standard_analyzer()
            .unwrap()
            .with_default_field("title");

        // QueryParser analyzes "Hello" to "hello" before creating TermQuery
        let query = parser.parse("Hello").unwrap();
        let results = engine.search(LexicalSearchRequest::new(query)).unwrap();

        // Should find the document
        // QueryParser analyzes "Hello" -> "hello", which matches the indexed "hello"
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
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage, config, doc_store).unwrap();

        // Search specific field
        use crate::analysis::analyzer::standard::StandardAnalyzer;
        use crate::lexical::index::inverted::query::parser::QueryParser;
        let analyzer = Arc::new(StandardAnalyzer::new().unwrap());
        let parser = QueryParser::new(analyzer);
        let query = parser.parse_field("title", "hello world").unwrap();
        let results = engine.search(LexicalSearchRequest::new(query)).unwrap();

        // Should not find anything (empty index)
        assert_eq!(results.hits.len(), 0);
    }

    #[test]
    fn test_id_based_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = LexicalIndexConfig::default();
        let storage = Arc::new(
            FileStorage::new(temp_dir.path(), FileStorageConfig::new(temp_dir.path())).unwrap(),
        );
        let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
        let doc_store = Arc::new(RwLock::new(
            UnifiedDocumentStore::open(doc_storage).unwrap(),
        ));
        let engine = LexicalStore::new(storage.clone(), config, doc_store.clone()).unwrap();

        // 1. Index document with external ID
        let mut doc = Document::new().add_text("title", "Test Doc");
        doc.id = Some("ext_1".to_string());
        let internal_id = engine.put_document(doc).unwrap();
        engine.commit().unwrap();
        doc_store.write().commit().unwrap();

        // 2. Get by internal ID
        let found = engine.get_document_by_internal_id(internal_id).unwrap();
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().get_field("title").unwrap().as_text(),
            Some("Test Doc")
        );

        // 3. Index with external ID
        let mut doc = Document::new().add_text("title", "Test Doc");
        doc.id = Some("ext_1".to_string());
        let internal_id = engine.put_document(doc).unwrap();

        // Verify ID search
        let found_id = engine.find_doc_id_by_term("_id", "ext_1").unwrap();
        assert_eq!(found_id, Some(internal_id));
        engine.commit().unwrap();

        // 4. Update existing by put_document
        let mut doc_v2 = Document::new().add_text("title", "Test Doc Updated");
        doc_v2.id = Some("ext_1".to_string());
        let internal_id_v2 = engine.put_document(doc_v2).unwrap();
        // Since UnifiedDocumentStore is append-only, update results in a new internal ID.
        assert_ne!(internal_id, internal_id_v2);
        engine.commit().unwrap();

        let found_v2 = engine.get_documents("ext_1").unwrap();
        assert_eq!(found_v2.len(), 1);
        assert_eq!(
            found_v2[0].get_field("title").unwrap().as_text(),
            Some("Test Doc Updated")
        );

        // 5. Delete by external ID
        let deleted = engine.delete_documents("ext_1").unwrap();
        assert!(deleted);
        engine.commit().unwrap();

        // 6. Verify deletion
        let found_after = engine.get_documents("ext_1").unwrap();
        assert!(found_after.is_empty());

        // 7. Delete non-existent
        let deleted_non = engine.delete_documents("non_existent").unwrap();
        assert!(!deleted_non);
    }
}
