//! VectorEngine 検索リクエスト関連の型定義
//!
//! このモジュールは検索リクエスト、クエリベクトル、フィールドセレクタを提供する。

use serde::{Deserialize, Serialize};

use crate::vector::core::document::{Payload, StoredVector};
use crate::vector::engine::filter::VectorFilter;

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
    /// Metadata filter to apply.
    #[serde(default)]
    pub filter: Option<VectorFilter>,
    /// Minimum score threshold. Results with scores below this value are filtered out.
    /// Default is 0.0 (no filtering).
    #[serde(default)]
    pub min_score: f32,

    /// Lexical query (keyword search).
    #[serde(default)]
    pub lexical_query: Option<LexicalQuery>,

    /// Rank fusion configuration.
    #[serde(default)]
    pub fusion_config: Option<FusionConfig>,
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
            filter: None,
            min_score: 0.0,
            lexical_query: None,
            fusion_config: None,
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
    pub vector: StoredVector,
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
    pub payload: Payload,
    /// Weight for this query vector (default: 1.0).
    pub weight: f32,
}

impl QueryPayload {
    /// Create a new query payload from a `Payload`.
    pub fn new(field: impl Into<String>, payload: Payload) -> Self {
        Self {
            field: field.into(),
            payload,
            weight: Self::default_weight(),
        }
    }

    /// Create a new query payload with a specific weight.
    pub fn with_weight(field: impl Into<String>, payload: Payload, weight: f32) -> Self {
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

/// Lexical query (keyword search).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "options", rename_all = "snake_case")]
pub enum LexicalQuery {
    /// Match all documents (useful for purely fetching or combining with filters).
    MatchAll,
    /// Term query (exact match).
    Term(TermQueryOptions),
    /// Match query (analyzed text search).
    Match(MatchQueryOptions),
    /// Boolean query (compound).
    Boolean(BooleanQueryOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermQueryOptions {
    pub field: String,
    pub term: String,
    #[serde(default = "default_boost")]
    pub boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchQueryOptions {
    pub field: String,
    pub query: String,
    #[serde(default = "default_operator_or")]
    pub operator: MatchOperator,
    #[serde(default = "default_boost")]
    pub boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchOperator {
    Or,
    And,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BooleanQueryOptions {
    #[serde(default)]
    pub must: Vec<LexicalQuery>,
    #[serde(default)]
    pub must_not: Vec<LexicalQuery>,
    #[serde(default)]
    pub should: Vec<LexicalQuery>,
    #[serde(default = "default_boost")]
    pub boost: f32,
}

/// Configuration for Rank Fusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum FusionConfig {
    /// Reciprocal Rank Fusion
    Rrf {
        /// constant k in formula: score = 1.0 / (k + rank)
        #[serde(default = "default_rrf_k")]
        k: usize,
    },
    /// Weighted Sum (Linear Combination)
    /// vector_score * vector_weight + lexical_score * lexical_weight
    WeightedSum {
        #[serde(default = "default_fusion_weight")]
        vector_weight: f32,
        #[serde(default = "default_fusion_weight")]
        lexical_weight: f32,
    },
}

impl Default for FusionConfig {
    fn default() -> Self {
        FusionConfig::Rrf { k: 60 }
    }
}

fn default_boost() -> f32 {
    1.0
}

fn default_operator_or() -> MatchOperator {
    MatchOperator::Or
}

fn default_rrf_k() -> usize {
    60
}

fn default_fusion_weight() -> f32 {
    1.0
}
