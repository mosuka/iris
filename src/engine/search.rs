use crate::lexical::query::Query;
use crate::vector::store::request::VectorSearchRequest;

/// Unified search request.
pub struct SearchRequest {
    /// Lexical query (e.g. BoolQuery, TermQuery).
    pub lexical: Option<Box<dyn Query>>,

    /// Vector search request.
    pub vector: Option<VectorSearchRequest>,

    /// Maximum number of results to return.
    pub limit: usize,

    /// Hybrid fusion algorithm to use (if both queries are present).
    pub fusion: Option<FusionAlgorithm>,

    /// Filter query (lexical) to restrict search space.
    /// Documents matching this query will be candidates for vector search.
    pub filter: Option<Box<dyn Query>>,

    /// Field-level boosts for lexical search.
    pub field_boosts: std::collections::HashMap<String, f32>,
}

/// Algorithm used to combine lexical and vector scores.
#[derive(Debug, Clone, Copy)]
pub enum FusionAlgorithm {
    /// Reciprocal Rank Fusion (RRF).
    /// Good when scores are not comparable (e.g. BM25 vs Cosine).
    RRF {
        /// Constant k (default 60).
        k: f64,
    },

    /// Weighted Sum.
    /// Requires normalized scores.
    WeightedSum {
        /// Weight for lexical score (0.0 - 1.0).
        lexical_weight: f32,
        /// Weight for vector score (0.0 - 1.0).
        vector_weight: f32,
    },
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            lexical: None,
            vector: None,
            limit: 10,
            fusion: None,
            filter: None,
            field_boosts: std::collections::HashMap::new(),
        }
    }
}

pub struct SearchRequestBuilder {
    request: SearchRequest,
}

impl Default for SearchRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchRequestBuilder {
    pub fn new() -> Self {
        Self {
            request: SearchRequest::default(),
        }
    }

    pub fn with_lexical(mut self, query: Box<dyn Query>) -> Self {
        self.request.lexical = Some(query);
        self
    }

    pub fn with_vector(mut self, request: VectorSearchRequest) -> Self {
        self.request.vector = Some(request);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.request.limit = limit;
        self
    }

    pub fn fusion(mut self, fusion: FusionAlgorithm) -> Self {
        self.request.fusion = Some(fusion);
        self
    }

    pub fn filter(mut self, query: Box<dyn Query>) -> Self {
        self.request.filter = Some(query);
        self
    }

    pub fn add_field_boost(mut self, field: impl Into<String>, boost: f32) -> Self {
        self.request.field_boosts.insert(field.into(), boost);
        self
    }

    pub fn build(self) -> SearchRequest {
        self.request
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    /// External document ID.
    pub id: String,
    pub score: f32,
    pub document: Option<crate::data::Document>,
}
