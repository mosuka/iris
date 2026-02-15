use crate::lexical::query::Query;
use crate::lexical::search::searcher::LexicalSearchRequest;
use crate::vector::store::request::VectorSearchRequest;

/// Unified search request.
pub struct SearchRequest {
    /// Lexical search request.
    pub lexical_search_request: Option<LexicalSearchRequest>,

    /// Vector search request.
    pub vector_search_request: Option<VectorSearchRequest>,

    /// Maximum number of results to return.
    pub limit: usize,

    /// Number of results to skip before returning (for pagination).
    pub offset: usize,

    /// Hybrid fusion algorithm to use (if both queries are present).
    pub fusion_algorithm: Option<FusionAlgorithm>,

    /// Filter query (lexical) to restrict search space.
    /// Documents matching this query will be candidates for vector search.
    pub filter_query: Option<Box<dyn Query>>,
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
            lexical_search_request: None,
            vector_search_request: None,
            limit: 10,
            offset: 0,
            fusion_algorithm: None,
            filter_query: None,
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

    /// Set the lexical search request.
    pub fn lexical_search_request(mut self, request: LexicalSearchRequest) -> Self {
        self.request.lexical_search_request = Some(request);
        self
    }

    /// Set the vector search request.
    pub fn vector_search_request(mut self, request: VectorSearchRequest) -> Self {
        self.request.vector_search_request = Some(request);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.request.limit = limit;
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.request.offset = offset;
        self
    }

    pub fn fusion_algorithm(mut self, fusion: FusionAlgorithm) -> Self {
        // Clamp weights to valid range to prevent NaN/Inf propagation.
        let fusion = match fusion {
            FusionAlgorithm::WeightedSum {
                lexical_weight,
                vector_weight,
            } => FusionAlgorithm::WeightedSum {
                lexical_weight: lexical_weight.clamp(0.0, 1.0),
                vector_weight: vector_weight.clamp(0.0, 1.0),
            },
            other => other,
        };
        self.request.fusion_algorithm = Some(fusion);
        self
    }

    pub fn filter_query(mut self, query: Box<dyn Query>) -> Self {
        self.request.filter_query = Some(query);
        self
    }

    /// Add a field-level boost for lexical search.
    ///
    /// Note: This requires `with_lexical()` to have been called first.
    /// If no lexical query has been set, this is a no-op.
    pub fn add_field_boost(mut self, field: impl Into<String>, boost: f32) -> Self {
        if let Some(ref mut lex) = self.request.lexical_search_request {
            lex.field_boosts.insert(field.into(), boost);
        }
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
