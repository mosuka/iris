use crate::lexical::query::Query;
use crate::lexical::search::searcher::LexicalSearchRequest;
use crate::vector::store::request::VectorSearchRequest;

/// Unified search request that can contain lexical, vector, or both queries.
///
/// Populate only `lexical_search_request` for pure lexical search, only
/// `vector_search_request` for pure vector search, or both for hybrid
/// search with fusion.
///
/// Use [`SearchRequestBuilder`] for a fluent construction API.
pub struct SearchRequest {
    /// Optional lexical search component (BM25-scored inverted index search).
    pub lexical_search_request: Option<LexicalSearchRequest>,

    /// Optional vector search component (nearest-neighbor search).
    pub vector_search_request: Option<VectorSearchRequest>,

    /// Maximum number of results to return. Defaults to `10`.
    pub limit: usize,

    /// Number of results to skip before returning (for pagination). Defaults to `0`.
    pub offset: usize,

    /// Fusion algorithm for combining lexical and vector scores.
    ///
    /// Only used when **both** `lexical_search_request` and
    /// `vector_search_request` are present. Defaults to
    /// [`FusionAlgorithm::RRF { k: 60.0 }`](FusionAlgorithm::RRF) when
    /// `None`.
    pub fusion_algorithm: Option<FusionAlgorithm>,

    /// Optional filter query (lexical) to restrict the search space.
    ///
    /// When set, the filter is evaluated first and **both** lexical and
    /// vector searches are restricted to documents matching this filter.
    /// For lexical search, the filter is combined with the user query via a
    /// boolean `must` + `filter` clause. For vector search, it produces an
    /// `allowed_ids` list that restricts candidate scoring.
    ///
    /// If the filter matches zero documents, the search returns an empty
    /// result immediately.
    pub filter_query: Option<Box<dyn Query>>,
}

/// Algorithm used to combine lexical and vector scores in hybrid search.
///
/// The default fusion algorithm (when none is specified in a
/// [`SearchRequest`]) is [`RRF`](Self::RRF) with `k = 60.0`.
#[derive(Debug, Clone, Copy)]
pub enum FusionAlgorithm {
    /// Reciprocal Rank Fusion (RRF).
    ///
    /// Combines results based on rank position rather than raw scores,
    /// making it effective when score magnitudes are not comparable
    /// (e.g. BM25 vs cosine similarity). The score for each document is
    /// `sum(1 / (k + rank))` across the result lists.
    RRF {
        /// Smoothing constant `k`. Higher values reduce the influence of
        /// top-ranked documents. Typical default is `60.0`.
        k: f64,
    },

    /// Weighted Sum with automatic min-max score normalization.
    ///
    /// Before weighting, the engine independently normalizes lexical and
    /// vector scores to the `[0.0, 1.0]` range using min-max normalization
    /// over their respective result sets. This means raw score magnitudes
    /// do not need to be comparable.
    WeightedSum {
        /// Weight for the normalized lexical score (clamped to `0.0..=1.0`
        /// by [`SearchRequestBuilder`]).
        lexical_weight: f32,
        /// Weight for the normalized vector score (clamped to `0.0..=1.0`
        /// by [`SearchRequestBuilder`]).
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

/// Fluent builder for constructing a [`SearchRequest`].
pub struct SearchRequestBuilder {
    request: SearchRequest,
}

impl Default for SearchRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchRequestBuilder {
    /// Create a new builder with default settings (limit 10, offset 0, no queries).
    pub fn new() -> Self {
        Self {
            request: SearchRequest::default(),
        }
    }

    /// Set the lexical search component.
    pub fn lexical_search_request(mut self, request: LexicalSearchRequest) -> Self {
        self.request.lexical_search_request = Some(request);
        self
    }

    /// Set the vector search component.
    pub fn vector_search_request(mut self, request: VectorSearchRequest) -> Self {
        self.request.vector_search_request = Some(request);
        self
    }

    /// Set the maximum number of results to return.
    pub fn limit(mut self, limit: usize) -> Self {
        self.request.limit = limit;
        self
    }

    /// Set the number of results to skip (for pagination).
    pub fn offset(mut self, offset: usize) -> Self {
        self.request.offset = offset;
        self
    }

    /// Set the fusion algorithm for hybrid search.
    ///
    /// For [`FusionAlgorithm::WeightedSum`], the `lexical_weight` and
    /// `vector_weight` values are clamped to the `0.0..=1.0` range to
    /// prevent NaN/Inf propagation.
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

    /// Set a filter query to restrict the search space.
    ///
    /// The filter applies to **both** lexical and vector searches when
    /// present. See [`SearchRequest::filter_query`] for details.
    pub fn filter_query(mut self, query: Box<dyn Query>) -> Self {
        self.request.filter_query = Some(query);
        self
    }

    /// Add a field-level boost for lexical search.
    ///
    /// Requires [`lexical_search_request`](Self::lexical_search_request) to
    /// have been called first. If no lexical query has been set, this is a
    /// no-op.
    pub fn add_field_boost(mut self, field: impl Into<String>, boost: f32) -> Self {
        if let Some(ref mut lex) = self.request.lexical_search_request {
            lex.field_boosts.insert(field.into(), boost);
        }
        self
    }

    /// Consume the builder and return the constructed [`SearchRequest`].
    pub fn build(self) -> SearchRequest {
        self.request
    }
}

/// A single result from an [`Engine`](super::Engine) search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// External document ID (the `_id` field value).
    pub id: String,
    /// Relevance score. The meaning depends on the search mode:
    /// - Lexical only: BM25 score.
    /// - Vector only: similarity score (e.g. cosine similarity).
    /// - Hybrid: fused score produced by the [`FusionAlgorithm`].
    pub score: f32,
    /// The stored fields of the document, or `None` if the document could
    /// not be retrieved (e.g. it was deleted between scoring and retrieval).
    pub document: Option<crate::data::Document>,
}
