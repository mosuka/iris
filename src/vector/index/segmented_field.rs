use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use crate::error::{IrisError, Result};
use crate::maintenance::deletion::DeletionBitmap;
use crate::storage::Storage;
use crate::vector::core::field::FieldOption;
use crate::vector::core::vector::{StoredVector, Vector};
use crate::vector::index::VectorIndexWriter;
use crate::vector::index::config::HnswIndexConfig;
use crate::vector::index::field::{
    FieldHit, FieldSearchInput, FieldSearchResults, VectorField, VectorFieldReader,
    VectorFieldStats, VectorFieldWriter,
};
use crate::vector::index::hnsw::reader::HnswIndexReader;
use crate::vector::index::hnsw::searcher::HnswSearcher;
use crate::vector::index::hnsw::segment::manager::{ManagedSegmentInfo, SegmentManager};
use crate::vector::index::hnsw::segment::merge_engine::{MergeConfig, MergeEngine};
use crate::vector::index::hnsw::writer::HnswIndexWriter;
use crate::vector::search::searcher::{
    VectorIndexSearchParams, VectorIndexSearchRequest, VectorIndexSearcher,
};
use crate::vector::store::config::VectorFieldConfig;
use crate::vector::writer::VectorIndexWriterConfig;
use std::cmp::Ordering;

/// A vector field implementation that partitions data into segments.
///
/// This implementation allows for efficient ingestion and background merging
/// of HNSW vector segments.
#[derive(Debug, Clone)]
pub struct SegmentedVectorField {
    /// Field name.
    pub name: String,

    /// Field configuration.
    pub config: VectorFieldConfig,

    /// Manager for segments.
    pub segment_manager: Arc<SegmentManager>,

    /// Storage backend.
    pub storage: Arc<dyn Storage>,

    /// Active segment for current writes.
    pub active_segment: Arc<RwLock<Option<(String, HnswIndexWriter)>>>,

    /// Global deletion bitmap.
    pub deletion_bitmap: Option<Arc<DeletionBitmap>>,
}

impl SegmentedVectorField {
    pub fn create(
        name: impl Into<String>,
        config: VectorFieldConfig,
        segment_manager: Arc<SegmentManager>,
        storage: Arc<dyn Storage>,
        deletion_bitmap: Option<Arc<DeletionBitmap>>,
    ) -> Result<Self> {
        let name_str = name.into();

        // Validate config
        match &config.vector {
            Some(FieldOption::Hnsw(_)) => {}
            _ => {
                return Err(IrisError::invalid_config(
                    "SegmentedVectorField requires HNSW configuration",
                ));
            }
        }

        let field = Self {
            name: name_str,
            config,
            segment_manager,
            storage,
            active_segment: Arc::new(RwLock::new(None)),
            deletion_bitmap,
        };

        Ok(field)
    }

    fn ensure_active_segment(&self) -> Result<()> {
        // ... same as before

        // Optimistic check
        if self.active_segment.read().is_some() {
            return Ok(());
        }

        let mut active_lock = self.active_segment.write();
        if active_lock.is_some() {
            return Ok(());
        }

        // Create new active segment
        let segment_id = self.segment_manager.generate_segment_id();

        // Get HNSW parameters from config
        let (dimension, distance, m, ef_construction) = match &self.config.vector {
            Some(FieldOption::Hnsw(opt)) => {
                (opt.dimension, opt.distance, opt.m, opt.ef_construction)
            }
            _ => {
                return Err(IrisError::invalid_config(
                    "SegmentedVectorField requires HNSW configuration".to_string(),
                ));
            }
        };

        let hnsw_config = HnswIndexConfig {
            dimension,
            distance_metric: distance,
            m,
            ef_construction,
            normalize_vectors: distance == crate::vector::core::distance::DistanceMetric::Cosine,
            ..Default::default()
        };

        let writer_config = VectorIndexWriterConfig {
            ..Default::default()
        };

        let writer = HnswIndexWriter::with_storage(
            hnsw_config,
            writer_config,
            &segment_id,
            self.storage.clone(),
        )?;
        *active_lock = Some((segment_id, writer));

        Ok(())
    }

    /// Trigger a background merge of segments using various policies.
    pub fn perform_merge(&self) -> Result<()> {
        let policy = crate::vector::index::hnsw::segment::merge_policy::SimpleMergePolicy::new();
        self.perform_merge_with_policy(&policy)
    }

    /// Trigger a merge with a specific policy.
    pub fn perform_merge_with_policy(
        &self,
        policy: &dyn crate::vector::index::hnsw::segment::merge_policy::MergePolicy,
    ) -> Result<()> {
        if let Some(candidate) = self.segment_manager.check_merge(policy) {
            let (dimension, m, ef_construction) = match &self.config.vector {
                Some(FieldOption::Hnsw(opt)) => (opt.dimension, opt.m, opt.ef_construction),
                _ => {
                    return Err(IrisError::invalid_config(
                        "SegmentedVectorField requires HNSW configuration".to_string(),
                    ));
                }
            };

            let mut engine = MergeEngine::new(
                MergeConfig::default(),
                self.storage.clone(),
                HnswIndexConfig {
                    dimension,
                    m,
                    ef_construction,
                    ..Default::default()
                },
                VectorIndexWriterConfig {
                    ..Default::default()
                },
            );

            if let Some(bitmap) = &self.deletion_bitmap {
                engine.set_deletion_bitmap(bitmap.clone());
            }

            let new_segment_id = self.segment_manager.generate_segment_id();
            let result =
                engine.merge_segments(candidate.segments.clone(), new_segment_id.clone())?;

            let info = ManagedSegmentInfo::new(new_segment_id, result.stats.vectors_merged, 0, 0);

            self.segment_manager.apply_merge(candidate, info)?;
        }
        Ok(())
    }
}

#[async_trait]
impl VectorField for SegmentedVectorField {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &VectorFieldConfig {
        &self.config
    }

    fn writer(&self) -> &dyn VectorFieldWriter {
        self
    }

    fn reader(&self) -> &dyn VectorFieldReader {
        self
    }

    fn writer_handle(&self) -> Arc<dyn VectorFieldWriter> {
        Arc::new(self.clone())
    }

    fn reader_handle(&self) -> Arc<dyn VectorFieldReader> {
        Arc::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl VectorFieldWriter for SegmentedVectorField {
    async fn add_stored_vector(
        &self,
        doc_id: u64,
        vector: &StoredVector,
        _version: u64,
    ) -> Result<()> {
        let vec = vector.to_vector();

        self.ensure_active_segment()?;
        let mut active_opt = self.active_segment.write();
        if let Some((_, writer)) = active_opt.as_mut() {
            writer.add_vectors(vec![(doc_id, self.name.clone(), vec)])?;
        } else {
            return Err(IrisError::internal(
                "No active segment available".to_string(),
            ));
        }
        Ok(())
    }

    async fn has_storage(&self) -> bool {
        self.active_segment
            .read()
            .as_ref()
            .map(|(_, w)| w.has_storage())
            .unwrap_or(false)
    }

    async fn vectors(&self) -> Vec<(u64, String, Vector)> {
        if let Some((_, writer)) = self.active_segment.read().as_ref() {
            writer.vectors().to_vec()
        } else {
            Vec::new()
        }
    }

    async fn rebuild(&self, _vectors: Vec<(u64, String, Vector)>) -> Result<()> {
        Ok(())
    }

    async fn delete_document(&self, doc_id: u64, _version: u64) -> Result<()> {
        if let Some((_, writer)) = self.active_segment.write().as_mut() {
            let _ = writer.delete_document(doc_id);
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        let mut active_lock = self.active_segment.write();
        if let Some((segment_id, mut writer)) = active_lock.take() {
            writer.finalize()?;
            writer.write()?;

            let vector_count = writer.vectors().len() as u64;

            let info = ManagedSegmentInfo::new(
                segment_id,
                vector_count,
                0, // offset
                0, // generation
            );

            self.segment_manager.add_segment(info)?;
        }
        Ok(())
    }

    async fn optimize(&self) -> Result<()> {
        let policy = crate::vector::index::hnsw::segment::merge_policy::ForceMergePolicy::new();
        self.perform_merge_with_policy(&policy)
    }
}

impl SegmentedVectorField {
    fn search_active_segment(
        &self,
        query: &[f32],
        limit: usize,
        weight: f32,
    ) -> Result<Vec<FieldHit>> {
        let active_opt = self.active_segment.read();
        let writer = match active_opt.as_ref() {
            Some((_, w)) => w,
            None => return Ok(Vec::new()),
        };

        // Safe unwrap because verified in create()
        let distance_metric = match &self.config.vector {
            Some(FieldOption::Hnsw(opt)) => opt.distance,
            _ => return Ok(Vec::new()), // Should not happen
        };

        let vectors = writer.vectors();
        let mut candidates = Vec::with_capacity(vectors.len());

        for (doc_id, _field, vector) in vectors {
            let similarity = distance_metric.similarity(query, &vector.data)?;
            let distance = distance_metric.distance(query, &vector.data)?;
            candidates.push((*doc_id, similarity, distance));
        }

        // Sort by similarity descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        let hits = candidates
            .into_iter()
            .take(limit)
            .map(|(doc_id, similarity, distance)| FieldHit {
                doc_id,
                field: self.name.clone(),
                score: similarity * weight,
                distance,
            })
            .collect();

        Ok(hits)
    }

    fn search_managed_segments(
        &self,
        query: &[f32],
        limit: usize,
        weight: f32,
    ) -> Result<Vec<FieldHit>> {
        let mut all_hits = Vec::new();
        let segments = self.segment_manager.list_segments();

        // Safe unwrap because verified in create()
        let distance_metric = match &self.config.vector {
            Some(FieldOption::Hnsw(opt)) => opt.distance,
            _ => return Ok(Vec::new()),
        };

        for info in segments {
            // Load reader for segment
            let mut reader =
                HnswIndexReader::load(self.storage.as_ref(), &info.segment_id, distance_metric)?;

            // Inject deletion bitmap if available
            if let Some(bitmap) = &self.deletion_bitmap {
                reader.set_deletion_bitmap(bitmap.clone());
            }

            let mut searcher = HnswSearcher::new(Arc::new(reader))?;

            // Set ef_search based on config if available
            if let Some(FieldOption::Hnsw(opt)) = &self.config.vector {
                // Use a higher ef_search for better recall (increase search effort relative to construction)
                searcher.set_ef_search(opt.ef_construction.max(50) * 2);
            }

            let params = VectorIndexSearchParams {
                top_k: limit,
                ..Default::default()
            };

            let request = VectorIndexSearchRequest {
                query: Vector::new(query.to_vec()),
                params,
                field_name: Some(self.name.clone()),
            };

            let results = searcher.search(&request)?;
            for res in results.results {
                all_hits.push(FieldHit {
                    doc_id: res.doc_id,
                    field: self.name.clone(),
                    score: res.similarity * weight,
                    distance: res.distance,
                });
            }
        }

        Ok(all_hits)
    }
}

impl VectorFieldReader for SegmentedVectorField {
    fn search(&self, request: FieldSearchInput) -> Result<FieldSearchResults> {
        if request.field != self.name {
            return Err(IrisError::invalid_argument(format!(
                "field mismatch: expected '{}', got '{}'",
                self.name, request.field
            )));
        }

        if request.query_vectors.is_empty() {
            return Ok(FieldSearchResults::default());
        }

        let mut merged: HashMap<u64, FieldHit> = HashMap::new();

        for query in &request.query_vectors {
            let effective_weight = query.weight;
            let query_vec = &query.vector;

            // 1. Search Active
            let active_hits =
                self.search_active_segment(query_vec, request.limit, effective_weight)?;
            for hit in active_hits {
                match merged.entry(hit.doc_id) {
                    Entry::Vacant(e) => {
                        e.insert(hit);
                    }
                    Entry::Occupied(mut e) => {
                        let entry = e.get_mut();
                        entry.score += hit.score;
                        entry.distance = entry.distance.min(hit.distance);
                    }
                }
            }

            // 2. Search Managed
            let managed_hits =
                self.search_managed_segments(query_vec, request.limit, effective_weight)?;
            for hit in managed_hits {
                match merged.entry(hit.doc_id) {
                    Entry::Vacant(e) => {
                        e.insert(hit);
                    }
                    Entry::Occupied(mut e) => {
                        let entry = e.get_mut();
                        entry.score += hit.score;
                        entry.distance = entry.distance.min(hit.distance);
                    }
                }
            }
        }

        let mut hits: Vec<FieldHit> = merged.into_values().collect();
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        if hits.len() > request.limit {
            hits.truncate(request.limit);
        }

        Ok(FieldSearchResults { hits })
    }

    fn stats(&self) -> Result<VectorFieldStats> {
        let mut active_count = 0;
        if let Some((_, writer)) = self.active_segment.read().as_ref() {
            active_count = writer.vectors().len();
        }

        let manager_stats = self.segment_manager.stats();
        let managed_count = manager_stats.total_vectors;

        // Safe unwrap because verified in create()
        let dimension = match &self.config.vector {
            Some(FieldOption::Hnsw(opt)) => opt.dimension,
            _ => 0,
        };

        Ok(VectorFieldStats {
            vector_count: active_count + managed_count as usize,
            dimension,
        })
    }
}
