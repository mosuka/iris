//! Vector store search response types.
//!
//! This module provides types for representing search results, hit information,
//! and collection-level statistics returned from vector search operations.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::vector::index::field::{FieldHit, VectorFieldStats};

/// Results returned from a vector search operation.
///
/// Contains a list of [`VectorHit`] entries ranked by relevance score.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorSearchResults {
    /// Ranked list of document hits matching the query vectors.
    #[serde(default)]
    pub hits: Vec<VectorHit>,
}

/// Aggregated statistics describing a collection and its fields.
#[derive(Debug, Clone, Default)]
pub struct VectorStats {
    /// Total number of documents in the collection.
    pub document_count: usize,
    /// Per-field statistics, keyed by field name.
    pub fields: HashMap<String, VectorFieldStats>,
}

/// A single document hit from a vector search.
///
/// Represents a matched document together with its aggregated similarity
/// score and the per-field hit details that contributed to the match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    /// Unique identifier of the matched document.
    pub doc_id: u64,
    /// Aggregated similarity score for this document across all query vectors
    /// and matched fields, computed according to the chosen [`VectorScoreMode`](crate::vector::store::request::VectorScoreMode).
    pub score: f32,
    /// Per-field hit details showing which fields matched and their individual scores.
    #[serde(default)]
    pub field_hits: Vec<FieldHit>,
}
