//! Vector store search request types.
//!
//! This module provides types for constructing vector search requests,
//! including query vectors, field selectors, and score combination modes.

use serde::{Deserialize, Serialize};

use crate::data::DataValue;

fn default_query_limit() -> usize {
    10
}

fn default_overfetch() -> f32 {
    1.0
}

/// Request model for collection-level search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    /// Query vectors to search with (already embedded).
    #[serde(default)]
    pub query_vectors: Vec<QueryVector>,
    /// Query payloads to embed and search with.
    /// These will be embedded internally using the configured embedder.
    /// Note: This field is skipped during serialization because `Payload`
    /// contains non-serializable data (e.g., `Arc<[u8]>`).
    #[serde(skip)]
    pub query_payloads: Vec<QueryPayload>,
    /// Fields to search in.
    ///
    /// **Note:** The current [`VectorStore::search()`](crate::vector::store::VectorStore::search)
    /// implementation does not use this field. All indexed vectors are searched
    /// regardless of this value. This field is reserved for future
    /// field-level filtering support.
    #[serde(default)]
    pub fields: Option<Vec<FieldSelector>>,
    /// Maximum number of results to return.
    #[serde(default = "default_query_limit")]
    pub limit: usize,
    /// How to combine scores from multiple query vectors.
    #[serde(default)]
    pub score_mode: VectorScoreMode,
    /// Overfetch factor for better result quality.
    ///
    /// **Note:** The current [`VectorStore::search()`](crate::vector::store::VectorStore::search)
    /// implementation does not use this field. Instead, it hardcodes a 2x overfetch
    /// (`limit.saturating_mul(2)`). This field is reserved for future use.
    #[serde(default = "default_overfetch")]
    pub overfetch: f32,
    /// Minimum score threshold. Results with scores below this value are filtered out.
    /// Default is 0.0 (no filtering).
    #[serde(default)]
    pub min_score: f32,

    /// List of allowed document IDs (for internal use by Engine filtering).
    #[serde(skip)]
    pub allowed_ids: Option<Vec<u64>>,
}

impl Default for VectorSearchRequest {
    fn default() -> Self {
        Self {
            query_vectors: Vec::new(),
            query_payloads: Vec::new(),
            fields: None,
            limit: default_query_limit(),
            score_mode: VectorScoreMode::default(),
            overfetch: default_overfetch(),
            min_score: 0.0,
            allowed_ids: None,
        }
    }
}

/// Selector for choosing which vector fields to include in a search.
///
/// Fields can be selected either by their exact name or by a name prefix,
/// allowing flexible targeting of specific vector fields within a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldSelector {
    /// Select a field by its exact name (e.g., `"title_embedding"`).
    Exact(String),
    /// Select all fields whose names start with the given prefix
    /// (e.g., `"image_"` matches `"image_thumbnail"`, `"image_full"`, etc.).
    Prefix(String),
}

/// Strategy for combining similarity scores when a search uses multiple query vectors.
///
/// Different modes suit different retrieval scenarios. For example,
/// [`WeightedSum`](Self::WeightedSum) works well when all query vectors contribute
/// additively, while [`MaxSim`](Self::MaxSim) is better for alternative-interpretation
/// queries and [`LateInteraction`](Self::LateInteraction) suits ColBERT-style multi-vector
/// representations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VectorScoreMode {
    /// Sum of `similarity * weight` across all query vectors.
    #[default]
    WeightedSum,
    /// Maximum `similarity * weight` across all query vectors.
    /// Useful when multiple query vectors represent alternative interpretations
    /// and only the best-matching one should determine the score.
    MaxSim,
    /// For each query vector, find the max similarity across all document vectors,
    /// then sum. Inspired by ColBERT's late interaction mechanism.
    /// Best suited for multi-vector document representations.
    LateInteraction,
}

/// A pre-embedded query vector with an optional weight and field restriction.
///
/// Each `QueryVector` carries a dense embedding that has already been produced
/// by an external embedding model. It is used directly for similarity
/// computation against the stored document vectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryVector {
    /// Dense floating-point embedding representing the query.
    pub vector: Vec<f32>,
    /// Multiplicative weight applied to the similarity score produced by this
    /// vector. Defaults to `1.0`.
    #[serde(default = "QueryVector::default_weight")]
    pub weight: f32,
    /// Optional list of fields to restrict this query vector to.
    /// If None, it applies to all target fields.
    #[serde(default)]
    pub fields: Option<Vec<String>>,
}

impl QueryVector {
    fn default_weight() -> f32 {
        1.0
    }
}

/// Query payload for a specific field (to be embedded internally).
///
/// This allows users to pass raw payloads (text, images, etc.) that will be
/// automatically embedded using the configured embedder during search.
///
/// Note: This type is not serializable because `Payload` contains
/// non-serializable data (e.g., `Arc<[u8]>`). Use `QueryVector` for
/// serialization scenarios with pre-embedded vectors.
#[derive(Debug, Clone)]
pub struct QueryPayload {
    /// The field name to search in.
    pub field: String,
    /// The payload to embed.
    pub payload: DataValue,
    /// Weight for this query vector (default: 1.0).
    pub weight: f32,
}

impl QueryPayload {
    /// Create a new query payload from a `DataValue`.
    pub fn new(field: impl Into<String>, payload: DataValue) -> Self {
        Self {
            field: field.into(),
            payload,
            weight: Self::default_weight(),
        }
    }

    /// Create a new query payload with a specific weight.
    pub fn with_weight(field: impl Into<String>, payload: DataValue, weight: f32) -> Self {
        Self {
            field: field.into(),
            payload,
            weight,
        }
    }

    fn default_weight() -> f32 {
        1.0
    }
}
