//! VectorStore 検索リクエスト関連の型定義
//!
//! このモジュールは検索リクエスト、クエリベクトル、フィールドセレクタを提供する。

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
    /// Fields to search in. If None, searches all default fields.
    #[serde(default)]
    pub fields: Option<Vec<FieldSelector>>,
    /// Maximum number of results to return.
    #[serde(default = "default_query_limit")]
    pub limit: usize,
    /// How to combine scores from multiple query vectors.
    #[serde(default)]
    pub score_mode: VectorScoreMode,
    /// Overfetch factor for better result quality.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldSelector {
    Exact(String),
    Prefix(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VectorScoreMode {
    #[default]
    WeightedSum,
    MaxSim,
    LateInteraction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryVector {
    pub vector: Vec<f32>,
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

