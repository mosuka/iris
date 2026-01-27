pub mod config;
pub mod search;

use std::sync::Arc;

use crate::analysis::analyzer::keyword::KeywordAnalyzer;
use crate::analysis::analyzer::per_field::PerFieldAnalyzer;
use crate::analysis::analyzer::standard::StandardAnalyzer;
use crate::data::Document;
use crate::error::Result;
use crate::lexical::store::LexicalStore;
use crate::lexical::store::config::LexicalIndexConfig;
use crate::storage::Storage;
use crate::storage::prefixed::PrefixedStorage;
use crate::vector::index::wal::{WalEntry, WalManager};
use crate::vector::store::VectorStore;
use crate::vector::store::config::VectorIndexConfig;

use self::config::IndexConfig;

/// Unified Engine that manages both Lexical and Vector indices.
///
/// This engine acts as a facade, coordinating document ingestion and search
/// across the underlying specialized engines.
pub struct Engine {
    config: IndexConfig,
    lexical: LexicalStore,
    vector: VectorStore,
    wal: Arc<WalManager>,
}

use crate::engine::search::{FusionAlgorithm, SearchResult};
use std::collections::HashMap;

impl Engine {
    /// Create a new Unified Engine.
    ///
    /// # Arguments
    ///
    /// * `storage` - The root storage for the index.
    /// * `config` - The unified index configuration.
    ///
    /// The engine will create two namespaces within the storage:
    /// - "lexical" for the inverted index
    /// - "vector" for the vector index
    pub fn new(storage: Arc<dyn Storage>, config: IndexConfig) -> Result<Self> {
        // 1. Split configuration
        let (lexical_config, vector_config) = Self::split_config(&config);

        // 2. Create namespaced storage
        let lexical_storage = Arc::new(PrefixedStorage::new("lexical", storage.clone()));
        let vector_storage = Arc::new(PrefixedStorage::new("vector", storage.clone()));

        // 3. Initialize engines
        let lexical = LexicalStore::new(lexical_storage, lexical_config)?;
        let vector = VectorStore::new(vector_storage, vector_config)?;

        // 4. Initialize Unified WAL (at root)
        let wal = Arc::new(WalManager::new(storage, "engine.wal")?);

        let engine = Self {
            config,
            lexical,
            vector,
            wal,
        };

        // 5. Recover state from WAL
        engine.recover()?;

        Ok(engine)
    }

    /// Recover index state from the Write-Ahead Log.
    fn recover(&self) -> Result<()> {
        let records = self.wal.read_all()?;
        if records.is_empty() {
            return Ok(());
        }

        let vector_last_seq = self.vector.last_wal_seq();
        let lexical_last_seq = self.lexical.last_wal_seq();

        for record in records {
            if record.seq <= vector_last_seq && record.seq <= lexical_last_seq {
                continue;
            }

            match record.entry {
                WalEntry::Upsert { doc_id, document } => {
                    // Re-index into both stores using the recorded doc_id
                    if record.seq > lexical_last_seq {
                        self.lexical.upsert_document(doc_id, document.clone())?;
                        self.lexical.set_last_wal_seq(record.seq)?;
                    }

                    if record.seq > vector_last_seq {
                        // Filter for vector fields
                        let mut vector_doc = Document::new();
                        if let Some(id) = &document.id {
                            vector_doc.id = Some(id.clone());
                        }
                        for (name, val) in &document.fields {
                            if let Some(field_config) = self.config.fields.get(name) {
                                if field_config.vector.is_some() {
                                    vector_doc.fields.insert(name.clone(), val.clone());
                                }
                            }
                        }
                        self.vector.upsert_vectors(doc_id, vector_doc)?;
                        self.vector.set_last_wal_seq(record.seq);
                    }
                }
                WalEntry::Delete { doc_id } => {
                    if record.seq > lexical_last_seq {
                        self.lexical.delete_document(doc_id)?;
                        self.lexical.set_last_wal_seq(record.seq)?;
                    }
                    if record.seq > vector_last_seq {
                        self.vector.delete_vectors(doc_id)?;
                        self.vector.set_last_wal_seq(record.seq);
                    }
                }
            }
        }
        Ok(())
    }

    /// Index a document.
    ///
    /// The document will be routed to the appropriate underlying engines based on
    /// the field configuration.
    ///
    /// - Fields with `lexical` config are added to the LexicalStore.
    /// - Fields with `vector` config are added to the VectorStore.
    /// - The document ID is preserved across both engines.
    pub fn index(&self, doc: Document) -> Result<()> {
        let _ = self.index_internal(doc, false)?;
        Ok(())
    }

    /// Index a document as a new chunk (always appends).
    ///
    /// This allows multiple internal documents (chunks) to share the same external ID.
    pub fn index_chunk(&self, doc: Document) -> Result<u64> {
        self.index_internal(doc, true)
    }

    fn index_internal(&self, doc: Document, as_chunk: bool) -> Result<u64> {
        // 1. Extract External ID
        let external_id = doc
            .id
            .clone()
            .or_else(|| {
                doc.fields
                    .get("_id")
                    .and_then(|v| v.as_text())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        if !as_chunk {
            self.delete(&external_id)?;
        }

        // 2. Index into Lexical Store
        let doc_id = if as_chunk {
            self.lexical.add_document(doc.clone())?
        } else {
            self.lexical.index_document(&external_id, doc.clone())?
        };

        // 3. Write to Centralized WAL
        let seq = self.wal.append(&WalEntry::Upsert {
            doc_id,
            document: doc.clone(),
        })?;

        // 4. Update sub-stores sequence tracker
        self.lexical.set_last_wal_seq(seq)?;
        self.vector.set_last_wal_seq(seq);

        // 4. Index into Vector Store
        let mut vector_doc = Document::new();
        vector_doc.id = Some(external_id.clone());

        for (name, val) in &doc.fields {
            if let Some(field_config) = self.config.fields.get(name) {
                if field_config.vector.is_some() {
                    vector_doc.fields.insert(name.clone(), val.clone());
                }
            }
        }

        self.vector.upsert_vectors(doc_id, vector_doc)?;

        Ok(doc_id)
    }

    /// Delete a document by its external ID.
    pub fn delete(&self, external_id: &str) -> Result<()> {
        let doc_ids = self.lexical.find_doc_ids_by_term("_id", external_id)?;
        for doc_id in doc_ids {
            // 1. Write to WAL
            let seq = self.wal.append(&WalEntry::Delete { doc_id })?;
            // 2. Update trackers
            self.lexical.set_last_wal_seq(seq)?;
            self.vector.set_last_wal_seq(seq);
            // 3. Delete from Lexical
            self.lexical.delete_document(doc_id)?;
            // 4. Delete from Vector
            self.vector.delete_vectors(doc_id)?;
        }
        Ok(())
    }

    /// Commit changes to both engines.
    pub fn commit(&self) -> Result<()> {
        self.lexical.commit()?;
        self.vector.commit()?;
        // After successful commit to both stores, we can truncate the WAL
        self.wal.truncate()?;
        Ok(())
    }

    /// Get index statistics.
    pub fn stats(&self) -> Result<crate::vector::store::response::VectorStats> {
        self.vector.stats()
    }

    /// Get a document by its internal ID.
    pub fn get_document(&self, doc_id: u64) -> Result<Option<Document>> {
        self.lexical.get_document(doc_id)
    }

    /// Split the unified config into specialized configs.
    fn split_config(config: &IndexConfig) -> (LexicalIndexConfig, VectorIndexConfig) {
        // Construct Lexical Config
        let default_analyzer = config
            .analyzer
            .clone()
            .unwrap_or_else(|| Arc::new(StandardAnalyzer::new().unwrap()));

        let mut per_field_analyzer = PerFieldAnalyzer::new(default_analyzer);
        per_field_analyzer.add_analyzer("_id", Arc::new(KeywordAnalyzer::new()));

        let mut lexical_builder =
            LexicalIndexConfig::builder().analyzer(Arc::new(per_field_analyzer));

        for (name, field_config) in &config.fields {
            if let Some(lexical_opt) = &field_config.lexical {
                lexical_builder = lexical_builder.add_field(name, lexical_opt.clone());
            }
        }

        let lexical_config = lexical_builder.build();

        // Construct Vector Config
        let mut vector_builder = VectorIndexConfig::builder();
        if let Some(embedder) = &config.embedder {
            vector_builder = vector_builder.embedder(embedder.clone());
        }

        for (name, field_config) in &config.fields {
            if let Some(vector_opt) = &field_config.vector {
                // VectorIndexConfig builder assumes we add fields one by one
                // But we need to verify how `add_field` works with `VectorOption`
                // The current API might need adjustment or we use `add_field` with explicit options.
                vector_builder = vector_builder
                    .add_field(name, vector_opt.clone())
                    .unwrap_or_else(|e| panic!("Failed to add field '{}': {}", name, e));
            }
        }

        // For now, return default/partial configs.
        // Real implementation requires more detailed mapping.
        let vector_config = vector_builder
            .build()
            .unwrap_or_else(|_| VectorIndexConfig::default()); // Fallback

        (lexical_config, vector_config)
    }

    /// Search the index.
    ///
    /// Executes hybrid search combining lexical and vector results.
    pub fn search(
        &self,
        request: self::search::SearchRequest,
    ) -> Result<Vec<self::search::SearchResult>> {
        // 0. Pre-process Filter
        let (allowed_ids, lexical_query_override) = if let Some(filter_query) = &request.filter {
            // A. Execute filter query to get allowed IDs for Vector Store
            use crate::lexical::search::searcher::LexicalSearchRequest;
            // Use a large limit for filtering. In production, this should be streaming or bitset-based.
            let req = LexicalSearchRequest::new(filter_query.clone_box())
                .max_docs(1_000_000)
                .load_documents(false);

            let filter_hits = self.lexical.search(req)?.hits;
            let ids: Vec<u64> = filter_hits.into_iter().map(|h| h.doc_id).collect();

            // If filter matches nothing, we can early return empty results?
            // Unless we want to return empty vector results but lexical results might vary? No, filter applies to everything.
            if ids.is_empty() {
                return Ok(Vec::new());
            }

            // B. Combine filter with lexical query if present
            let new_lexical_query: Option<Box<dyn crate::lexical::index::inverted::query::Query>> =
                if let Some(user_query) = &request.lexical {
                    use crate::lexical::index::inverted::query::boolean::BooleanQueryBuilder;
                    // Use builder for chaining
                    let bool_query = BooleanQueryBuilder::new()
                        .must(user_query.clone_box())
                        .must(filter_query.clone_box())
                        .build();
                    Some(Box::new(bool_query))
                } else {
                    // If only filter is present, we don't necessarily want to run lexical search
                    // unless explicitly requested (lexical=None means no lexical search).
                    // But wait, if filter is present, it acts as a constraint.
                    // If lexical is None, we just do Vector Search with filter.
                    None
                };

            (Some(ids), new_lexical_query)
        } else {
            (None, None)
        };

        // 1. Execute Lexical Search
        let mut lexical_query_to_use =
            lexical_query_override.or_else(|| request.lexical.as_ref().map(|q| q.clone_box()));

        if let Some(query) = &mut lexical_query_to_use {
            if !request.field_boosts.is_empty() {
                query.apply_field_boosts(&request.field_boosts);
            }
        }

        let lexical_hits = if let Some(query) = &lexical_query_to_use {
            use crate::lexical::search::searcher::LexicalSearchRequest;
            let q = query.clone_box();
            // Fetch up to limit * 2 to give fusion more data if both queries are present
            let overfetch_limit = if request.vector.is_some() {
                request.limit * 2
            } else {
                request.limit
            };
            let req = LexicalSearchRequest::new(q).max_docs(overfetch_limit);

            self.lexical.search(req)?.hits
        } else {
            Vec::new()
        };

        // 2. Execute Vector Search
        let vector_hits = if let Some(vector_req) = &request.vector {
            // Similarly overfetch for vector if lexical is present
            let mut vreq = vector_req.clone();
            if request.lexical.is_some() && vreq.limit < request.limit * 2 {
                vreq.limit = request.limit * 2;
            }
            // Inject allowed_ids
            if let Some(ids) = &allowed_ids {
                vreq.allowed_ids = Some(ids.clone());
            }
            self.vector.search(vreq)?.hits
        } else {
            Vec::new()
        };

        // 3. Fusion
        if request.lexical.is_some() && request.vector.is_some() {
            let algorithm = request.fusion.unwrap_or(FusionAlgorithm::RRF { k: 60.0 });
            self.fuse_results(lexical_hits, vector_hits, algorithm, request.limit)
        } else if !vector_hits.is_empty() {
            // Only vector results
            Ok(vector_hits
                .into_iter()
                .take(request.limit)
                .map(|hit| SearchResult {
                    doc_id: hit.doc_id,
                    score: hit.score,
                    document: None,
                })
                .collect())
        } else {
            // Only lexical results (or both empty)
            Ok(lexical_hits
                .into_iter()
                .take(request.limit)
                .map(|hit| SearchResult {
                    doc_id: hit.doc_id,
                    score: hit.score,
                    document: hit.document,
                })
                .collect())
        }
    }

    /// Combine results from lexical and vector engines.
    fn fuse_results(
        &self,
        lexical_hits: Vec<crate::lexical::index::inverted::query::SearchHit>,
        vector_hits: Vec<crate::vector::store::response::VectorHit>,
        fusion: FusionAlgorithm,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut fused_scores: HashMap<u64, (f32, Option<crate::data::Document>)> = HashMap::new();

        match fusion {
            FusionAlgorithm::RRF { k } => {
                // RRF: score = sum(1.0 / (k + rank))
                // Lexical ranks
                for (rank, hit) in lexical_hits.into_iter().enumerate() {
                    let rrf_score = 1.0 / (k + (rank + 1) as f64);
                    let entry = fused_scores
                        .entry(hit.doc_id)
                        .or_insert((0.0, hit.document));
                    entry.0 += rrf_score as f32;
                }
                // Vector ranks
                for (rank, hit) in vector_hits.into_iter().enumerate() {
                    let rrf_score = 1.0 / (k + (rank + 1) as f64);
                    let entry = fused_scores.entry(hit.doc_id).or_insert((0.0, None));
                    entry.0 += rrf_score as f32;
                }
            }
            FusionAlgorithm::WeightedSum {
                lexical_weight,
                vector_weight,
            } => {
                // Weighted Sum: normalized_lexical * w1 + normalized_vector * w2

                // 1. Normalize Lexical Scores
                let lexical_min = lexical_hits
                    .iter()
                    .map(|h| h.score)
                    .fold(f32::INFINITY, f32::min);
                let lexical_max = lexical_hits
                    .iter()
                    .map(|h| h.score)
                    .fold(f32::NEG_INFINITY, f32::max);

                for hit in lexical_hits {
                    let norm_score = if lexical_max > lexical_min {
                        (hit.score - lexical_min) / (lexical_max - lexical_min)
                    } else {
                        1.0 // If all scores are same, treat as 1.0 (or 0.0? 1.0 is more common for single hits)
                    };
                    let entry = fused_scores
                        .entry(hit.doc_id)
                        .or_insert((0.0, hit.document));
                    entry.0 += norm_score * lexical_weight;
                }

                // 2. Normalize Vector Scores
                let vector_min = vector_hits
                    .iter()
                    .map(|h| h.score)
                    .fold(f32::INFINITY, f32::min);
                let vector_max = vector_hits
                    .iter()
                    .map(|h| h.score)
                    .fold(f32::NEG_INFINITY, f32::max);

                for hit in vector_hits {
                    let norm_score = if vector_max > vector_min {
                        (hit.score - vector_min) / (vector_max - vector_min)
                    } else {
                        1.0
                    };
                    let entry = fused_scores.entry(hit.doc_id).or_insert((0.0, None));
                    entry.0 += norm_score * vector_weight;
                }
            }
        }

        let mut results: Vec<SearchResult> = fused_scores
            .into_iter()
            .map(|(doc_id, (score, document))| SearchResult {
                doc_id,
                score,
                document,
            })
            .collect();

        // Sort by fused score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        if results.len() > limit {
            results.truncate(limit);
        }

        // Fill missing documents from Lexical Store if needed
        for result in &mut results {
            if result.document.is_none() {
                if let Ok(Some(doc)) = self.lexical.get_document(result.doc_id) {
                    result.document = Some(doc);
                }
            }
        }

        Ok(results)
    }
}
