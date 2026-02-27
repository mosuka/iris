pub mod query;
pub mod schema;
pub mod search;

use std::sync::Arc;

use crate::analysis::analyzer::analyzer::Analyzer;
use crate::analysis::analyzer::keyword::KeywordAnalyzer;
use crate::analysis::analyzer::per_field::PerFieldAnalyzer;
use crate::analysis::analyzer::standard::StandardAnalyzer;
use crate::data::Document;
use crate::embedding::embedder::Embedder;
use crate::error::Result;
use crate::lexical::store::LexicalStore;
use crate::lexical::store::config::LexicalIndexConfig;
use crate::storage::Storage;
use crate::storage::prefixed::PrefixedStorage;
use crate::store::log::{DocumentLog, LogEntry};
use crate::vector::store::VectorStore;
use crate::vector::store::config::VectorIndexConfig;

use self::schema::Schema;

/// Unified Engine that manages both Lexical and Vector indices.
///
/// This engine acts as a facade, coordinating document ingestion and search
/// across the underlying specialized engines.
pub struct Engine {
    schema: Schema,
    lexical: LexicalStore,
    vector: VectorStore,
    log: Arc<DocumentLog>,
}

use crate::engine::search::{FusionAlgorithm, SearchResult};
use std::collections::HashMap;

impl Engine {
    /// Create a new Unified Engine with default analyzer and no embedder.
    ///
    /// For custom analyzer or embedder configuration, use [`Engine::builder`].
    pub async fn new(storage: Arc<dyn Storage>, schema: Schema) -> Result<Self> {
        EngineBuilder::new(storage, schema).build().await
    }

    /// Create an [`EngineBuilder`] for custom configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = Engine::builder(storage, schema)
    ///     .analyzer(Arc::new(StandardAnalyzer::default()))
    ///     .embedder(Arc::new(MyEmbedder))
    ///     .build()?;
    /// ```
    pub fn builder(storage: Arc<dyn Storage>, schema: Schema) -> EngineBuilder {
        EngineBuilder::new(storage, schema)
    }

    /// Recover index state from the document log.
    async fn recover(&self) -> Result<()> {
        // read_all() internally syncs next_doc_id with doc_store segments.
        let records = self.log.read_all()?;

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
                LogEntry::Upsert {
                    doc_id,
                    external_id: _,
                    document,
                } => {
                    // Restore document into document store
                    let stored_doc = self.filter_stored_fields(&document);
                    self.log.store_document(doc_id, stored_doc);

                    // Re-index into both stores using the recorded doc_id.
                    // Update seq only after BOTH stores succeed to maintain atomicity.
                    if record.seq > lexical_last_seq {
                        self.lexical.upsert_document(doc_id, document.clone())?;
                    }

                    if record.seq > vector_last_seq {
                        // Filter for vector fields
                        let mut vector_doc = Document::new();
                        for (name, val) in &document.fields {
                            if self
                                .schema
                                .fields
                                .get(name)
                                .is_some_and(|fc| fc.is_vector())
                            {
                                vector_doc.fields.insert(name.clone(), val.clone());
                            }
                        }
                        self.vector
                            .upsert_document_by_internal_id(doc_id, vector_doc)
                            .await?;
                    }

                    // Both stores succeeded — now update seq trackers
                    if record.seq > lexical_last_seq {
                        self.lexical.set_last_wal_seq(record.seq)?;
                    }
                    if record.seq > vector_last_seq {
                        self.vector.set_last_wal_seq(record.seq);
                    }
                }
                LogEntry::Delete {
                    doc_id,
                    external_id: _,
                } => {
                    if record.seq > lexical_last_seq {
                        self.lexical.delete_document_by_internal_id(doc_id)?;
                    }
                    if record.seq > vector_last_seq {
                        self.vector.delete_document_by_internal_id(doc_id).await?;
                    }

                    // Both stores succeeded — now update seq trackers
                    if record.seq > lexical_last_seq {
                        self.lexical.set_last_wal_seq(record.seq)?;
                    }
                    if record.seq > vector_last_seq {
                        self.vector.set_last_wal_seq(record.seq);
                    }
                }
            }
        }
        Ok(())
    }

    /// Put (upsert) a document.
    ///
    /// If a document with the same external ID exists, it is replaced.
    /// The document will be routed to the appropriate underlying engines
    /// based on the schema field configuration.
    pub async fn put_document(&self, id: &str, doc: Document) -> Result<()> {
        let _ = self.index_internal(id, doc, false).await?;
        Ok(())
    }

    /// Add a document as a new chunk (always appends).
    ///
    /// This allows multiple documents (chunks) to share the same external ID.
    pub async fn add_document(&self, id: &str, doc: Document) -> Result<()> {
        let _ = self.index_internal(id, doc, true).await?;
        Ok(())
    }

    async fn index_internal(&self, id: &str, mut doc: Document, as_chunk: bool) -> Result<u64> {
        // 1. Inject _id field
        use crate::data::DataValue;
        doc.fields
            .insert("_id".to_string(), DataValue::Text(id.to_string()));

        if !as_chunk {
            self.delete_documents(id).await?;
        }

        // 2. Write-Ahead Log: assign doc_id + persist (before any index updates)
        let (doc_id, seq) = self.log.append(id, doc.clone())?;

        // 3. Store only stored fields for retrieval (WAL has full data for recovery)
        let stored_doc = self.filter_stored_fields(&doc);
        self.log.store_document(doc_id, stored_doc);

        // 4. Prepare vector document (extract vector fields only)
        let mut vector_doc = Document::new();
        for (name, val) in &doc.fields {
            if self
                .schema
                .fields
                .get(name)
                .is_some_and(|fc| fc.is_vector())
            {
                vector_doc.fields.insert(name.clone(), val.clone());
            }
        }

        // 5. Index into Lexical and Vector stores
        self.lexical.upsert_document(doc_id, doc)?;
        if let Err(e) = self
            .vector
            .upsert_document_by_internal_id(doc_id, vector_doc)
            .await
        {
            // Rollback lexical insert to maintain consistency
            let _ = self.lexical.delete_document_by_internal_id(doc_id);
            return Err(e);
        }

        // 6. Update sub-stores sequence tracker AFTER both stores succeed.
        // This ensures failed index operations are retried on recovery.
        self.lexical.set_last_wal_seq(seq)?;
        self.vector.set_last_wal_seq(seq);

        Ok(doc_id)
    }

    /// Delete all documents (including chunks) by external ID.
    pub async fn delete_documents(&self, id: &str) -> Result<()> {
        let doc_ids = self.lexical.find_doc_ids_by_term("_id", id)?;
        for doc_id in doc_ids {
            // 1. Write to log
            let seq = self.log.append_delete(doc_id, id)?;
            // 2. Delete from Lexical
            self.lexical.delete_document_by_internal_id(doc_id)?;
            // 3. Delete from Vector
            self.vector.delete_document_by_internal_id(doc_id).await?;
            // 4. Update trackers AFTER both deletes succeed.
            // This ensures failed deletes are retried on recovery.
            self.lexical.set_last_wal_seq(seq)?;
            self.vector.set_last_wal_seq(seq);
        }
        Ok(())
    }

    /// Commit changes to both engines.
    pub async fn commit(&self) -> Result<()> {
        self.lexical.commit()?;
        self.vector.commit().await?;
        self.log.commit_documents()?;
        // After successful commit to all stores, truncate the log
        self.log.truncate()?;
        Ok(())
    }

    /// Get index statistics.
    pub fn stats(&self) -> Result<crate::vector::store::response::VectorStats> {
        self.vector.stats()
    }

    /// Get all documents (including chunks) by external ID.
    pub async fn get_documents(&self, id: &str) -> Result<Vec<Document>> {
        let doc_ids = self.lexical.find_doc_ids_by_term("_id", id)?;
        let mut docs = Vec::with_capacity(doc_ids.len());
        for doc_id in doc_ids {
            if let Some(doc) = self.get_document_by_internal_id(doc_id)? {
                docs.push(doc);
            }
        }
        Ok(docs)
    }

    /// Check if a field should be stored based on the schema.
    ///
    /// - `_id`: always stored (system field)
    /// - Lexical fields: stored only if `stored=true`
    /// - Vector fields: always stored
    /// - Unknown fields: not stored
    fn is_field_stored(&self, name: &str) -> bool {
        use crate::engine::schema::FieldOption;

        if name == "_id" {
            return true;
        }
        if let Some(field_opt) = self.schema.fields.get(name) {
            match field_opt {
                FieldOption::Text(o) => o.stored,
                FieldOption::Integer(o) => o.stored,
                FieldOption::Float(o) => o.stored,
                FieldOption::Boolean(o) => o.stored,
                FieldOption::DateTime(o) => o.stored,
                FieldOption::Geo(o) => o.stored,
                FieldOption::Bytes(o) => o.stored,
                // Vector fields are always stored
                FieldOption::Hnsw(_) | FieldOption::Flat(_) | FieldOption::Ivf(_) => true,
            }
        } else {
            false
        }
    }

    /// Filter a document to only include fields that should be stored.
    ///
    /// The document log (WAL) stores ALL fields for recovery, but the
    /// document store only keeps stored fields to save space.
    fn filter_stored_fields(&self, doc: &Document) -> Document {
        let mut stored_doc = Document::new();
        for (name, val) in &doc.fields {
            if self.is_field_stored(name) {
                stored_doc.fields.insert(name.clone(), val.clone());
            }
        }
        stored_doc
    }

    /// Get a document by its internal ID (private helper).
    ///
    /// Retrieves from the document log and filters out non-stored fields.
    fn get_document_by_internal_id(&self, doc_id: u64) -> Result<Option<Document>> {
        let doc = self.log.get_document(doc_id)?;

        if let Some(doc) = doc {
            Ok(Some(self.filter_stored_fields(&doc)))
        } else {
            Ok(None)
        }
    }

    /// Resolve external ID from internal doc_id.
    fn resolve_external_id(&self, internal_id: u64) -> Result<String> {
        if let Some(doc) = self.log.get_document(internal_id)?
            && let Some(id) = doc.fields.get("_id").and_then(|v| v.as_text())
        {
            return Ok(id.to_string());
        }
        Ok(format!("unknown_{}", internal_id))
    }

    /// Split the unified schema into specialized configs.
    fn split_schema(
        schema: &Schema,
        analyzer: Option<Arc<dyn Analyzer>>,
        embedder: Option<Arc<dyn Embedder>>,
    ) -> Result<(LexicalIndexConfig, VectorIndexConfig)> {
        // Construct Lexical Config
        let analyzer = match analyzer {
            Some(a) => a,
            None => Arc::new(StandardAnalyzer::new()?),
        };

        // If the user passed a PerFieldAnalyzer, clone it and ensure _id uses KeywordAnalyzer.
        // Otherwise, wrap the simple analyzer in a new PerFieldAnalyzer.
        let per_field_analyzer =
            if let Some(existing) = analyzer.as_any().downcast_ref::<PerFieldAnalyzer>() {
                let mut pfa = existing.clone();
                pfa.add_analyzer("_id", Arc::new(KeywordAnalyzer::new()));
                pfa
            } else {
                let mut pfa = PerFieldAnalyzer::new(analyzer);
                pfa.add_analyzer("_id", Arc::new(KeywordAnalyzer::new()));
                pfa
            };

        let mut lexical_builder =
            LexicalIndexConfig::builder().analyzer(Arc::new(per_field_analyzer));

        for (name, field_option) in &schema.fields {
            if let Some(lexical_opt) = field_option.to_lexical() {
                lexical_builder = lexical_builder.add_field(name, lexical_opt);
            }
        }

        let lexical_config = lexical_builder.build();

        // Construct Vector Config
        let mut vector_builder = VectorIndexConfig::builder();
        if let Some(embedder) = &embedder {
            vector_builder = vector_builder.embedder(embedder.clone());
        }

        for (name, field_option) in &schema.fields {
            if let Some(vector_opt) = field_option.to_vector() {
                vector_builder = vector_builder.add_field(name, vector_opt)?;
            }
        }

        let vector_config = vector_builder.build()?;

        Ok((lexical_config, vector_config))
    }

    /// Search the index.
    ///
    /// Executes hybrid search combining lexical and vector results.
    pub async fn search(
        &self,
        request: self::search::SearchRequest,
    ) -> Result<Vec<self::search::SearchResult>> {
        // 0. Pre-process Filter
        let (allowed_ids, lexical_query_override) =
            if let Some(filter_query) = &request.filter_query {
                let req = crate::lexical::search::searcher::LexicalSearchRequest::new(
                    filter_query.clone_box(),
                )
                .limit(1_000_000)
                .load_documents(false);

                let filter_hits = self.lexical.search(req)?.hits;
                let ids: Vec<u64> = filter_hits.into_iter().map(|h| h.doc_id).collect();

                if ids.is_empty() {
                    return Ok(Vec::new());
                }

                let new_lexical_query: Option<Box<dyn crate::lexical::query::Query>> =
                    if let Some(lex_req) = &request.lexical_search_request {
                        use crate::lexical::query::boolean::BooleanQueryBuilder;
                        let user_query = lex_req.query.clone().unwrap_query()?;
                        let bool_query = BooleanQueryBuilder::new()
                            .must(user_query)
                            .filter(filter_query.clone_box())
                            .build();
                        Some(Box::new(bool_query))
                    } else {
                        None
                    };

                (Some(ids), new_lexical_query)
            } else {
                (None, None)
            };

        // 1. Execute Lexical Search
        let mut lexical_query_to_use = if lexical_query_override.is_some() {
            lexical_query_override
        } else if let Some(lex_req) = &request.lexical_search_request {
            Some(lex_req.query.clone().unwrap_query()?)
        } else {
            None
        };

        if let Some(query) = &mut lexical_query_to_use
            && let Some(lex_req) = &request.lexical_search_request
            && !lex_req.field_boosts.is_empty()
        {
            query.apply_field_boosts(&lex_req.field_boosts);
        }

        let fetch_count = request.offset.saturating_add(request.limit);

        let lexical_hits = if let Some(query) = &lexical_query_to_use {
            let q = query.clone_box();
            let overfetch_limit = if request.vector_search_request.is_some() {
                fetch_count.saturating_mul(2)
            } else {
                fetch_count
            };
            let req = crate::lexical::search::searcher::LexicalSearchRequest::new(q)
                .limit(overfetch_limit);

            self.lexical.search(req)?.hits
        } else {
            Vec::new()
        };

        // 2. Execute Vector Search
        let vector_hits = if let Some(vector_req) = &request.vector_search_request {
            let mut vreq = vector_req.clone();
            if request.lexical_search_request.is_some()
                && vreq.limit < fetch_count.saturating_mul(2)
            {
                vreq.limit = fetch_count.saturating_mul(2);
            }
            if let Some(ids) = &allowed_ids {
                vreq.allowed_ids = Some(ids.clone());
            }
            // Embed query_payloads into query_vectors before searching.
            // NOTE: When using VectorQueryParser, query_vectors are already populated
            // at parse time, so this block is skipped. This fallback remains for
            // VectorSearchRequestBuilder users who populate query_payloads directly.
            if !vreq.query_payloads.is_empty() {
                use crate::data::DataValue;
                use crate::embedding::embedder::EmbedInput;
                use crate::embedding::per_field::PerFieldEmbedder;
                use crate::vector::store::request::QueryVector;

                let embedder = self.vector.embedder();
                for payload in vreq.query_payloads.drain(..) {
                    let (text_owned, bytes_owned, mime_owned) = match payload.payload {
                        DataValue::Text(t) => (Some(t), None, None),
                        DataValue::Bytes(b, m) => (None, Some(b), m),
                        _ => continue,
                    };
                    let field_name = payload.field.clone();
                    let input = if let Some(ref text) = text_owned {
                        EmbedInput::Text(text)
                    } else if let Some(ref bytes) = bytes_owned {
                        EmbedInput::Bytes(bytes, mime_owned.as_deref())
                    } else {
                        unreachable!()
                    };
                    let vector =
                        if let Some(pf) = embedder.as_any().downcast_ref::<PerFieldEmbedder>() {
                            pf.embed_field(&field_name, &input).await?
                        } else {
                            embedder.embed(&input).await?
                        };
                    vreq.query_vectors.push(QueryVector {
                        vector: vector.data,
                        weight: payload.weight,
                        fields: Some(vec![payload.field]),
                    });
                }
            }
            self.vector.search(vreq)?.hits
        } else {
            Vec::new()
        };

        // 3. Fusion
        if request.lexical_search_request.is_some() && request.vector_search_request.is_some() {
            let algorithm = request
                .fusion_algorithm
                .unwrap_or(FusionAlgorithm::RRF { k: 60.0 });
            let mut results =
                self.fuse_results(lexical_hits, vector_hits, algorithm, fetch_count)?;
            if request.offset > 0 {
                results = results.into_iter().skip(request.offset).collect();
            }
            results.truncate(request.limit);
            Ok(results)
        } else if !vector_hits.is_empty() {
            // Only vector results — resolve external IDs and load documents
            let mut results = Vec::with_capacity(request.limit);
            for hit in vector_hits
                .into_iter()
                .skip(request.offset)
                .take(request.limit)
            {
                let external_id = self.resolve_external_id(hit.doc_id)?;
                let document = self.get_document_by_internal_id(hit.doc_id)?;
                results.push(SearchResult {
                    id: external_id,
                    score: hit.score,
                    document,
                });
            }
            Ok(results)
        } else {
            // Only lexical results (or both empty)
            let mut results = Vec::with_capacity(request.limit);
            for hit in lexical_hits
                .into_iter()
                .skip(request.offset)
                .take(request.limit)
            {
                let external_id = self.resolve_external_id(hit.doc_id)?;
                results.push(SearchResult {
                    id: external_id,
                    score: hit.score,
                    document: hit.document,
                });
            }
            Ok(results)
        }
    }

    /// Combine results from lexical and vector engines.
    fn fuse_results(
        &self,
        lexical_hits: Vec<crate::lexical::query::SearchHit>,
        vector_hits: Vec<crate::vector::store::response::VectorHit>,
        fusion: FusionAlgorithm,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut fused_scores: HashMap<u64, (f32, Option<crate::data::Document>)> = HashMap::new();

        match fusion {
            FusionAlgorithm::RRF { k } => {
                for (rank, hit) in lexical_hits.into_iter().enumerate() {
                    let rrf_score = 1.0 / (k + (rank + 1) as f64);
                    let entry = fused_scores
                        .entry(hit.doc_id)
                        .or_insert((0.0, hit.document));
                    entry.0 += rrf_score as f32;
                }
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
                        1.0
                    };
                    let entry = fused_scores
                        .entry(hit.doc_id)
                        .or_insert((0.0, hit.document));
                    entry.0 += norm_score * lexical_weight;
                }

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

        let mut intermediate: Vec<(u64, f32, Option<crate::data::Document>)> = fused_scores
            .into_iter()
            .map(|(doc_id, (score, document))| (doc_id, score, document))
            .collect();

        // Sort by fused score descending
        intermediate.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        if intermediate.len() > limit {
            intermediate.truncate(limit);
        }

        // Resolve external IDs and fill missing documents
        let mut results = Vec::with_capacity(intermediate.len());
        for (doc_id, score, document) in intermediate {
            let external_id = self.resolve_external_id(doc_id)?;
            let document = if document.is_some() {
                document
            } else {
                self.get_document_by_internal_id(doc_id)?
            };
            results.push(SearchResult {
                id: external_id,
                score,
                document,
            });
        }

        Ok(results)
    }
}

/// Builder for constructing an [`Engine`] with custom configuration.
///
/// Use this when you need to specify a custom analyzer or embedder.
/// For simple cases with default settings, use [`Engine::new`] directly.
///
/// # Example
///
/// ```ignore
/// let schema = Schema::builder()
///     .add_field("content", FieldOption::Lexical(LexicalFieldOption::Text(TextOption::default())))
///     .add_field("content_vec", FieldOption::Vector(VectorOption::Flat(FlatOption { dimension: 384, ..Default::default() })))
///     .build();
///
/// let engine = Engine::builder(storage, schema)
///     .analyzer(Arc::new(StandardAnalyzer::default()))
///     .embedder(Arc::new(MyEmbedder))
///     .build()?;
/// ```
pub struct EngineBuilder {
    storage: Arc<dyn Storage>,
    schema: Schema,
    analyzer: Option<Arc<dyn Analyzer>>,
    embedder: Option<Arc<dyn Embedder>>,
}

impl EngineBuilder {
    /// Create a new builder with the given storage and schema.
    pub fn new(storage: Arc<dyn Storage>, schema: Schema) -> Self {
        Self {
            storage,
            schema,
            analyzer: None,
            embedder: None,
        }
    }

    /// Set the analyzer for text fields.
    ///
    /// Both simple analyzers (e.g., [`StandardAnalyzer`]) and [`PerFieldAnalyzer`] are
    /// supported. When a `PerFieldAnalyzer` is passed, it is used directly (with `_id`
    /// automatically set to `KeywordAnalyzer` if not already configured).
    ///
    /// If not set, [`StandardAnalyzer`] is used as the default.
    pub fn analyzer(mut self, analyzer: Arc<dyn Analyzer>) -> Self {
        self.analyzer = Some(analyzer);
        self
    }

    /// Set the embedder for vector fields.
    ///
    /// Both simple embedders and [`PerFieldEmbedder`](crate::embedding::per_field::PerFieldEmbedder)
    /// are supported. When a `PerFieldEmbedder` is passed, each vector field will use
    /// the embedder registered for that field name, falling back to the default.
    ///
    /// If not set, no embedder is configured.
    pub fn embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Build the [`Engine`].
    ///
    /// # Errors
    ///
    /// Returns an error if the storage or index initialization fails.
    pub async fn build(self) -> Result<Engine> {
        let (lexical_config, vector_config) =
            Engine::split_schema(&self.schema, self.analyzer, self.embedder)?;

        let lexical_storage = Arc::new(PrefixedStorage::new("lexical", self.storage.clone()));
        let vector_storage = Arc::new(PrefixedStorage::new("vector", self.storage.clone()));
        let document_storage: Arc<dyn Storage> =
            Arc::new(PrefixedStorage::new("documents", self.storage.clone()));

        let lexical = LexicalStore::new(lexical_storage, lexical_config)?;
        let vector = VectorStore::new(vector_storage, vector_config)?;

        let log = Arc::new(DocumentLog::new(
            self.storage,
            "engine.wal",
            document_storage,
        )?);

        let engine = Engine {
            schema: self.schema,
            lexical,
            vector,
            log,
        };

        engine.recover().await?;

        Ok(engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::per_field::PerFieldEmbedder;
    use crate::embedding::precomputed::PrecomputedEmbedder;
    use crate::storage::memory::MemoryStorage;

    #[tokio::test]
    async fn test_accepts_per_field_analyzer() {
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(Default::default()));
        let schema = Schema::new();

        let per_field = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::default()));

        let result = Engine::builder(storage, schema)
            .analyzer(Arc::new(per_field))
            .build()
            .await;

        assert!(result.is_ok(), "Should accept PerFieldAnalyzer");
    }

    #[tokio::test]
    async fn test_accepts_per_field_embedder() {
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(Default::default()));
        let schema = Schema::new();

        let dummy_embedder = Arc::new(PrecomputedEmbedder::new());
        let per_field = PerFieldEmbedder::new(dummy_embedder);

        let result = Engine::builder(storage, schema)
            .embedder(Arc::new(per_field))
            .build()
            .await;

        assert!(result.is_ok(), "Should accept PerFieldEmbedder");
    }

    #[tokio::test]
    async fn test_accepts_simple_analyzer() {
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(Default::default()));
        let schema = Schema::new();

        let result = Engine::builder(storage, schema)
            .analyzer(Arc::new(StandardAnalyzer::default()))
            .build()
            .await;

        assert!(result.is_ok(), "Should accept StandardAnalyzer");
    }

    #[tokio::test]
    async fn test_accepts_simple_embedder() {
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new(Default::default()));
        let schema = Schema::new();

        let dummy_embedder = Arc::new(PrecomputedEmbedder::new());

        let result = Engine::builder(storage, schema)
            .embedder(dummy_embedder)
            .build()
            .await;

        assert!(result.is_ok(), "Should accept simple embedder");
    }
}
