//! VectorStore: Simplified vector storage following LexicalStore pattern.
//!
//! This module provides a vector storage component with a simple 3-member structure:
//! - `index`: The underlying vector index
//! - `writer_cache`: Cached writer for write operations
//! - `searcher_cache`: Cached searcher for search operations
//!
//! # Module Structure
//!
//! - [`config`] - Configuration types (VectorIndexConfig, VectorFieldConfig)
//! - [`embedder`] - Embedding utilities (EmbedderExecutor)
//! - [`embedding_writer`] - Embedding writer wrapper
//! - [`query`] - Search query builder
//! - [`request`] - Search request types
//! - [`response`] - Search response types

pub mod config;
pub mod embedder;
pub mod embedding_writer;
pub mod memory;
pub mod request;
pub mod response;

use std::sync::Arc;

use parking_lot::Mutex;

use crate::data::Document;
use crate::embedding::embedder::Embedder;
use crate::error::Result;
use crate::storage::Storage;
use crate::vector::core::vector::Vector;
use crate::vector::index::config::VectorIndexTypeConfig;
use crate::vector::index::factory::VectorIndexFactory;
use crate::vector::index::VectorIndex;
use crate::vector::search::searcher::{VectorIndexSearchRequest, VectorIndexSearcher};
use crate::vector::writer::VectorIndexWriter;

use self::config::VectorIndexConfig;
use self::request::VectorSearchRequest;
use self::response::{VectorHit, VectorSearchResults, VectorStats};

/// A simplified vector storage component following the LexicalStore pattern.
///
/// This structure mirrors `LexicalStore` with only 3 members:
/// - `index`: The underlying vector index
/// - `writer_cache`: Cached writer for write operations
/// - `searcher_cache`: Cached searcher for search operations
pub struct VectorStore {
    /// The underlying vector index.
    index: Box<dyn VectorIndex>,
    /// Cached writer (created on-demand).
    writer_cache: Mutex<Option<Box<dyn VectorIndexWriter>>>,
    /// Cached searcher (invalidated after commit/optimize).
    searcher_cache: parking_lot::RwLock<Option<Box<dyn VectorIndexSearcher>>>,
}

impl std::fmt::Debug for VectorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorStore")
            .field("index", &self.index)
            .finish()
    }
}

impl VectorStore {
    /// Create a new vector store with the given storage and high-level configuration.
    ///
    /// This constructor is compatible with Engine and accepts VectorIndexConfig.
    /// It extracts the index type configuration from the first field.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend for persisting index data
    /// * `config` - High-level configuration (compatible with Engine)
    ///
    /// # Returns
    ///
    /// Returns a new `VectorStore` instance.
    pub fn new(storage: Arc<dyn Storage>, config: VectorIndexConfig) -> Result<Self> {
        // Extract index type config from the first field, or use default
        let index_type_config = Self::extract_index_type_config(&config);
        Self::with_index_type_config(storage, index_type_config)
    }

    /// Create a new vector store with explicit index type configuration.
    ///
    /// This is a lower-level constructor for when you have a specific
    /// VectorIndexTypeConfig.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend for persisting index data
    /// * `config` - Configuration for the vector index (Flat, HNSW, or IVF)
    ///
    /// # Returns
    ///
    /// Returns a new `VectorStore` instance.
    pub fn with_index_type_config(
        storage: Arc<dyn Storage>,
        config: VectorIndexTypeConfig,
    ) -> Result<Self> {
        let index = VectorIndexFactory::open_or_create(storage, "vector_index", config)?;
        Ok(Self {
            index,
            writer_cache: Mutex::new(None),
            searcher_cache: parking_lot::RwLock::new(None),
        })
    }

    /// Extract VectorIndexTypeConfig from VectorIndexConfig.
    ///
    /// Uses the first field's configuration if available, otherwise returns default.
    fn extract_index_type_config(config: &VectorIndexConfig) -> VectorIndexTypeConfig {
        use crate::vector::core::field::FieldOption;
        use crate::vector::index::config::{FlatIndexConfig, HnswIndexConfig, IvfIndexConfig};

        // Try to get config from the first field with vector configuration
        for field_config in config.fields.values() {
            if let Some(ref vector_opt) = field_config.vector {
                return match vector_opt {
                    FieldOption::Flat(opt) => VectorIndexTypeConfig::Flat(FlatIndexConfig {
                        dimension: opt.dimension,
                        distance_metric: opt.distance,
                        embedder: config.embedder.clone(),
                        ..Default::default()
                    }),
                    FieldOption::Hnsw(opt) => VectorIndexTypeConfig::HNSW(HnswIndexConfig {
                        dimension: opt.dimension,
                        distance_metric: opt.distance,
                        m: opt.m,
                        ef_construction: opt.ef_construction,
                        embedder: config.embedder.clone(),
                        ..Default::default()
                    }),
                    FieldOption::Ivf(opt) => VectorIndexTypeConfig::IVF(IvfIndexConfig {
                        dimension: opt.dimension,
                        distance_metric: opt.distance,
                        n_clusters: opt.n_clusters,
                        n_probe: opt.n_probe,
                        embedder: config.embedder.clone(),
                        ..Default::default()
                    }),
                };
            }
        }

        // Default to HNSW with config's embedder
        VectorIndexTypeConfig::HNSW(HnswIndexConfig {
            embedder: config.embedder.clone(),
            ..Default::default()
        })
    }

    /// Upsert a document by its internal ID (used for WAL recovery).
    ///
    /// This method is primarily used during WAL recovery where the internal ID
    /// is already known.
    pub fn upsert_document_by_internal_id(&self, doc_id: u64, doc: Document) -> Result<()> {
        // Get or create writer
        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }

        // First, delete any existing vectors for this doc_id
        let writer = guard.as_mut().unwrap();
        let _ = writer.delete_document(doc_id);

        // Add values to index (writer handles embedding automatically)
        for (field_name, value) in &doc.fields {
            writer.add_value(doc_id, field_name.clone(), value.clone())?;
        }

        Ok(())
    }

    /// Delete a document by its internal ID.
    pub fn delete_document_by_internal_id(&self, doc_id: u64) -> Result<()> {
        let mut guard = self.writer_cache.lock();
        if guard.is_none() {
            *guard = Some(self.index.writer()?);
        }
        let writer = guard.as_mut().unwrap();

        writer.delete_document(doc_id)?;

        Ok(())
    }

    /// Commit any pending changes to the index.
    pub fn commit(&self) -> Result<()> {
        if let Some(mut writer) = self.writer_cache.lock().take() {
            // commit() calls finalize() then write() to persist to storage
            writer.commit()?;
        }
        self.index.refresh()?;
        *self.searcher_cache.write() = None;
        Ok(())
    }

    /// Optimize the index.
    pub fn optimize(&self) -> Result<()> {
        self.index.optimize()?;
        *self.searcher_cache.write() = None;
        Ok(())
    }

    /// Refresh the searcher cache.
    pub fn refresh(&self) -> Result<()> {
        *self.searcher_cache.write() = None;
        Ok(())
    }

    /// Get or create a searcher.
    fn get_searcher(&self) -> Result<Box<dyn VectorIndexSearcher>> {
        // For now, always create a new searcher.
        // The cache can be used for optimization later.
        self.index.searcher()
    }

    /// Execute a low-level vector similarity search.
    pub fn search_index(
        &self,
        request: &VectorIndexSearchRequest,
    ) -> Result<crate::vector::search::searcher::VectorIndexSearchResults> {
        let searcher = self.get_searcher()?;
        searcher.search(request)
    }

    /// Execute a high-level vector search (compatible with Engine).
    ///
    /// This method handles multiple query vectors and aggregates results.
    /// Note: query_payloads must be pre-embedded before calling this method.
    pub fn search(&self, request: VectorSearchRequest) -> Result<VectorSearchResults> {
        if request.query_vectors.is_empty() {
            return Ok(VectorSearchResults::default());
        }

        let searcher = self.get_searcher()?;
        let mut all_hits: std::collections::HashMap<u64, f32> = std::collections::HashMap::new();

        // Process each query vector
        for qv in &request.query_vectors {
            let index_request = VectorIndexSearchRequest::new(Vector::new(qv.vector.clone()))
                .top_k(request.limit * 2); // Overfetch for better results

            let results = searcher.search(&index_request)?;

            // Aggregate scores
            for result in results.results {
                // Apply allowed_ids filter if present
                if request
                    .allowed_ids
                    .as_ref()
                    .is_some_and(|allowed| !allowed.contains(&result.doc_id))
                {
                    continue;
                }

                // Apply min_score filter
                if result.similarity < request.min_score {
                    continue;
                }

                let entry = all_hits.entry(result.doc_id).or_insert(0.0);
                *entry += result.similarity * qv.weight;
            }
        }

        // Convert to VectorHit and sort by score
        let mut hits: Vec<VectorHit> = all_hits
            .into_iter()
            .map(|(doc_id, score)| VectorHit {
                doc_id,
                score,
                field_hits: vec![],
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if hits.len() > request.limit {
            hits.truncate(request.limit);
        }

        Ok(VectorSearchResults { hits })
    }

    /// Count the number of vectors matching the query.
    pub fn count(&self, request: VectorIndexSearchRequest) -> Result<u64> {
        let searcher = self.get_searcher()?;
        searcher.count(request)
    }

    /// Get index statistics.
    pub fn stats(&self) -> Result<VectorStats> {
        // Use the reader to get accurate vector count
        let reader = self.index.reader()?;
        let doc_count = reader.vector_count();

        Ok(VectorStats {
            document_count: doc_count,
            fields: std::collections::HashMap::new(), // Simplified - no per-field stats
        })
    }

    /// Get the storage backend.
    pub fn storage(&self) -> &Arc<dyn Storage> {
        self.index.storage()
    }

    /// Close the store.
    pub fn close(&self) -> Result<()> {
        *self.writer_cache.lock() = None;
        *self.searcher_cache.write() = None;
        self.index.close()
    }

    /// Check if the store is closed.
    pub fn is_closed(&self) -> bool {
        self.index.is_closed()
    }

    /// Get the embedder.
    pub fn embedder(&self) -> Arc<dyn Embedder> {
        self.index.embedder()
    }

    /// Get the last processed WAL sequence number.
    pub fn last_wal_seq(&self) -> u64 {
        self.index.last_wal_seq()
    }

    /// Set the last processed WAL sequence number.
    ///
    /// Note: This method doesn't return Result for Engine compatibility.
    /// Errors are silently ignored.
    pub fn set_last_wal_seq(&self, seq: u64) {
        let _ = self.index.set_last_wal_seq(seq);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};

    #[test]
    fn test_vectorstore_creation() {
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));

        let config = VectorIndexTypeConfig::default();
        let store = VectorStore::with_index_type_config(storage, config).unwrap();

        assert!(!store.is_closed());
    }

    #[test]
    fn test_vectorstore_close() {
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));

        let config = VectorIndexTypeConfig::default();
        let store = VectorStore::with_index_type_config(storage, config).unwrap();

        assert!(!store.is_closed());
        store.close().unwrap();
        assert!(store.is_closed());
    }
}
