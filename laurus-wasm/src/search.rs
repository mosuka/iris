//! WASM wrappers for search request/result and fusion algorithm types.

use crate::convert::data_value_to_json;
use crate::query::{
    JsQuery, JsVectorQuery, extract_lexical_query, query_to_lexical_search_query,
    vector_query_to_search_query,
};
use laurus::{FusionAlgorithm, LexicalSearchQuery, SearchRequestBuilder, SearchResult};
use serde::Serialize;
use wasm_bindgen::JsValue;

// ---------------------------------------------------------------------------
// SearchResult
// ---------------------------------------------------------------------------

/// A single search result.
#[derive(Serialize)]
pub struct WasmSearchResult {
    /// External document identifier.
    pub id: String,
    /// Relevance score.
    pub score: f64,
    /// Retrieved document fields as a key-value object, or null.
    pub document: Option<serde_json::Value>,
}

/// Convert a [`SearchResult`] from the engine into a [`WasmSearchResult`].
pub fn to_wasm_search_result(r: SearchResult) -> WasmSearchResult {
    let document = r.document.map(|doc| {
        let mut map = serde_json::Map::new();
        for (field, value) in doc.fields {
            map.insert(field, data_value_to_json(&value));
        }
        serde_json::Value::Object(map)
    });
    WasmSearchResult {
        id: r.id,
        score: r.score as f64,
        document,
    }
}

// ---------------------------------------------------------------------------
// SearchRequest internal types
// ---------------------------------------------------------------------------

pub enum FusionChoice {
    RRF(f64),
    WeightedSum(f32, f32),
}

/// Internal search request state.
pub struct WasmSearchRequestInner {
    pub query_dsl: Option<String>,
    pub lexical_query: Option<JsQuery>,
    pub vector_query: Option<JsVectorQuery>,
    pub filter_query: Option<JsQuery>,
    pub fusion: Option<FusionChoice>,
    pub limit: usize,
    pub offset: usize,
}

impl WasmSearchRequestInner {
    pub fn new(limit: usize, offset: usize) -> Self {
        Self {
            query_dsl: None,
            lexical_query: None,
            vector_query: None,
            filter_query: None,
            fusion: None,
            limit,
            offset,
        }
    }

    /// Build the Laurus [`laurus::SearchRequest`] from this wrapper.
    pub fn build(&self) -> Result<laurus::SearchRequest, JsValue> {
        let mut builder = SearchRequestBuilder::new()
            .limit(self.limit)
            .offset(self.offset);

        // Fusion algorithm
        if let Some(fusion) = &self.fusion {
            match fusion {
                FusionChoice::RRF(k) => {
                    builder = builder.fusion_algorithm(FusionAlgorithm::RRF { k: *k });
                }
                FusionChoice::WeightedSum(lw, vw) => {
                    builder = builder.fusion_algorithm(FusionAlgorithm::WeightedSum {
                        lexical_weight: *lw,
                        vector_weight: *vw,
                    });
                }
            }
        }

        // Filter query
        if let Some(fq) = &self.filter_query {
            builder = builder.filter_query(extract_lexical_query(fq)?);
        }

        // Explicit hybrid: lexical_query + vector_query both set
        if let (Some(lq), Some(vq)) = (&self.lexical_query, &self.vector_query) {
            builder = builder
                .lexical_query(query_to_lexical_search_query(lq)?)
                .vector_query(vector_query_to_search_query(vq));
            if self.fusion.is_none() {
                builder = builder.fusion_algorithm(FusionAlgorithm::RRF { k: 60.0 });
            }
            return Ok(builder.build());
        }

        // Only lexical_query set
        if let Some(lq) = &self.lexical_query {
            builder = builder.lexical_query(query_to_lexical_search_query(lq)?);
            return Ok(builder.build());
        }

        // Only vector_query set
        if let Some(vq) = &self.vector_query {
            builder = builder.vector_query(vector_query_to_search_query(vq));
            return Ok(builder.build());
        }

        // DSL string
        if let Some(dsl) = &self.query_dsl {
            builder = builder.query_dsl(dsl.clone());
            return Ok(builder.build());
        }

        Ok(builder.build())
    }
}

// ---------------------------------------------------------------------------
// Helper: build a SearchRequest from index.search() arguments
// ---------------------------------------------------------------------------

/// Build a [`laurus::SearchRequest`] from a DSL string with limit/offset.
pub fn build_dsl_request(dsl: String, limit: usize, offset: usize) -> laurus::SearchRequest {
    SearchRequestBuilder::new()
        .limit(limit)
        .offset(offset)
        .query_dsl(dsl)
        .build()
}

/// Build a [`laurus::SearchRequest`] from a lexical query.
pub fn build_lexical_request(
    query: &JsQuery,
    limit: usize,
    offset: usize,
) -> Result<laurus::SearchRequest, JsValue> {
    Ok(SearchRequestBuilder::new()
        .limit(limit)
        .offset(offset)
        .lexical_query(LexicalSearchQuery::Obj(extract_lexical_query(query)?))
        .build())
}

/// Build a [`laurus::SearchRequest`] from a vector query.
pub fn build_vector_request(
    query: &JsVectorQuery,
    limit: usize,
    offset: usize,
) -> laurus::SearchRequest {
    SearchRequestBuilder::new()
        .limit(limit)
        .offset(offset)
        .vector_query(vector_query_to_search_query(query))
        .build()
}
