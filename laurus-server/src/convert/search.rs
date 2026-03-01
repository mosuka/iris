use std::collections::HashMap;

use laurus::{
    FusionAlgorithm, LexicalSearchParams, LexicalSearchQuery, LexicalSearchRequest, QueryVector,
    SearchRequestBuilder, SearchResult, SortField, SortOrder, VectorScoreMode, VectorSearchRequest,
};

use crate::convert::document;
use crate::proto::laurus::v1;

/// Build a laurus SearchRequest from a proto SearchRequest.
#[allow(clippy::result_large_err)]
pub fn from_proto(proto: &v1::SearchRequest) -> Result<laurus::SearchRequest, tonic::Status> {
    let mut builder = SearchRequestBuilder::new();

    // Lexical query
    if !proto.query.is_empty() {
        let lexical_params = build_lexical_params(proto.lexical_params.as_ref());
        let field_boosts: HashMap<String, f32> = proto.field_boosts.clone();

        let lexical_request = LexicalSearchRequest {
            query: LexicalSearchQuery::Dsl(proto.query.clone()),
            params: lexical_params,
            field_boosts,
        };
        builder = builder.lexical_search_request(lexical_request);
    }

    // Vector query
    if !proto.query_vectors.is_empty() {
        let vector_request = build_vector_request(proto)?;
        builder = builder.vector_search_request(vector_request);
    }

    // Limit and offset
    if proto.limit > 0 {
        builder = builder.limit(proto.limit as usize);
    }
    builder = builder.offset(proto.offset as usize);

    // Fusion
    if let Some(fusion) = &proto.fusion
        && let Some(alg) = &fusion.algorithm
    {
        let fusion_alg = match alg {
            v1::fusion_algorithm::Algorithm::Rrf(rrf) => FusionAlgorithm::RRF { k: rrf.k },
            v1::fusion_algorithm::Algorithm::WeightedSum(ws) => FusionAlgorithm::WeightedSum {
                lexical_weight: ws.lexical_weight,
                vector_weight: ws.vector_weight,
            },
        };
        builder = builder.fusion_algorithm(fusion_alg);
    }

    // Field boosts (also applied at builder level)
    for (field, boost) in &proto.field_boosts {
        builder = builder.add_field_boost(field.clone(), *boost);
    }

    Ok(builder.build())
}

fn build_lexical_params(params: Option<&v1::LexicalParams>) -> LexicalSearchParams {
    match params {
        Some(p) => {
            let sort_by = match &p.sort_by {
                Some(spec) if !spec.field.is_empty() => {
                    let order = match v1::SortOrder::try_from(spec.order) {
                        Ok(v1::SortOrder::Desc) => SortOrder::Desc,
                        _ => SortOrder::Asc,
                    };
                    SortField::Field {
                        name: spec.field.clone(),
                        order,
                    }
                }
                _ => SortField::Score,
            };
            LexicalSearchParams {
                limit: 0, // Controlled by top-level limit
                min_score: p.min_score,
                load_documents: true,
                timeout_ms: p.timeout_ms,
                parallel: p.parallel,
                sort_by,
            }
        }
        None => LexicalSearchParams::default(),
    }
}

#[allow(clippy::result_large_err)]
fn build_vector_request(proto: &v1::SearchRequest) -> Result<VectorSearchRequest, tonic::Status> {
    let query_vectors: Vec<QueryVector> = proto
        .query_vectors
        .iter()
        .map(|qv| QueryVector {
            vector: qv.vector.clone(),
            weight: if qv.weight == 0.0 { 1.0 } else { qv.weight },
            fields: if qv.fields.is_empty() {
                None
            } else {
                Some(qv.fields.clone())
            },
        })
        .collect();

    let (score_mode, overfetch, min_score) = match &proto.vector_params {
        Some(vp) => {
            let score_mode = match v1::VectorScoreMode::try_from(vp.score_mode) {
                Ok(v1::VectorScoreMode::MaxSim) => VectorScoreMode::MaxSim,
                Ok(v1::VectorScoreMode::LateInteraction) => VectorScoreMode::LateInteraction,
                _ => VectorScoreMode::WeightedSum,
            };
            let overfetch = if vp.overfetch == 0.0 {
                2.0
            } else {
                vp.overfetch
            };
            (score_mode, overfetch, vp.min_score)
        }
        None => (VectorScoreMode::WeightedSum, 2.0, 0.0),
    };

    Ok(VectorSearchRequest {
        query_vectors,
        query_payloads: Vec::new(),
        fields: None,
        limit: proto.limit as usize,
        score_mode,
        overfetch,
        min_score,
        allowed_ids: None,
    })
}

/// Convert a laurus SearchResult into a proto SearchResult.
pub fn result_to_proto(result: &SearchResult) -> v1::SearchResult {
    v1::SearchResult {
        id: result.id.clone(),
        score: result.score,
        document: result.document.as_ref().map(document::to_proto),
    }
}
