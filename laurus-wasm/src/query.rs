//! WASM wrappers for all Laurus query types.
//!
//! Each query struct stores the data needed to construct the Rust query.

use laurus::lexical::span::{SpanQueryBuilder, SpanQueryWrapper};
use laurus::lexical::{
    BooleanQuery, FuzzyQuery, GeoQuery, NumericRangeQuery, PhraseQuery, TermQuery, WildcardQuery,
};
use laurus::vector::Vector;
use laurus::vector::store::request::QueryVector;
use laurus::{DataValue, LexicalSearchQuery, QueryPayload, VectorSearchQuery};
use wasm_bindgen::JsValue;

// ---------------------------------------------------------------------------
// Internal enum types (not exported to JS)
// ---------------------------------------------------------------------------

/// Enum wrapping all supported lexical query types.
pub enum JsQuery {
    TermQuery(JsTermQuery),
    PhraseQuery(JsPhraseQuery),
    FuzzyQuery(JsFuzzyQuery),
    WildcardQuery(JsWildcardQuery),
    NumericRangeQuery(JsNumericRangeQuery),
    GeoQuery(JsGeoQuery),
    BooleanQuery(JsBooleanQuery),
    SpanQuery(JsSpanQuery),
}

/// Enum wrapping all supported vector query types.
pub enum JsVectorQuery {
    VectorQuery(JsVectorQueryInner),
    VectorTextQuery(JsVectorTextQuery),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a Laurus lexical query from a [`JsQuery`] enum variant.
pub fn extract_lexical_query(query: &JsQuery) -> Result<Box<dyn laurus::lexical::Query>, JsValue> {
    match query {
        JsQuery::TermQuery(q) => Ok(Box::new(TermQuery::new(&q.field, &q.term))),
        JsQuery::PhraseQuery(q) => Ok(Box::new(PhraseQuery::new(&q.field, q.terms.clone()))),
        JsQuery::FuzzyQuery(q) => Ok(Box::new(
            FuzzyQuery::new(&q.field, &q.term).max_edits(q.max_edits),
        )),
        JsQuery::WildcardQuery(q) => Ok(Box::new(
            WildcardQuery::new(&q.field, &q.pattern)
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        )),
        JsQuery::NumericRangeQuery(q) => Ok(q.build()),
        JsQuery::GeoQuery(q) => q.build().map_err(|e| JsValue::from_str(&e.to_string())),
        JsQuery::BooleanQuery(q) => q.build_query(),
        JsQuery::SpanQuery(q) => Ok(Box::new(SpanQueryWrapper::new(q.kind.build(&q.field)))),
    }
}

/// Convert a [`JsQuery`] into a [`LexicalSearchQuery`].
pub fn query_to_lexical_search_query(query: &JsQuery) -> Result<LexicalSearchQuery, JsValue> {
    Ok(LexicalSearchQuery::Obj(extract_lexical_query(query)?))
}

/// Convert a [`JsVectorQuery`] into a [`VectorSearchQuery`].
pub fn vector_query_to_search_query(query: &JsVectorQuery) -> VectorSearchQuery {
    match query {
        JsVectorQuery::VectorQuery(q) => VectorSearchQuery::Vectors(vec![QueryVector {
            vector: Vector::new(q.vector.clone()),
            weight: 1.0,
            fields: Some(vec![q.field.clone()]),
        }]),
        JsVectorQuery::VectorTextQuery(q) => VectorSearchQuery::Payloads(vec![QueryPayload::new(
            &q.field,
            DataValue::Text(q.text.clone()),
        )]),
    }
}

// ---------------------------------------------------------------------------
// Span-query recipe enum (Clone so it can be nested)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum SpanKind {
    Term(String),
    Near(Vec<SpanKind>, u32, bool),
    Containing(Box<SpanKind>, Box<SpanKind>),
    Within(Box<SpanKind>, Box<SpanKind>, u32),
}

impl SpanKind {
    pub fn build(&self, field: &str) -> Box<dyn laurus::lexical::span::SpanQuery> {
        let sb = SpanQueryBuilder::new(field);
        match self {
            SpanKind::Term(t) => Box::new(sb.term(t)),
            SpanKind::Near(clauses, slop, ordered) => {
                let built: Vec<Box<dyn laurus::lexical::span::SpanQuery>> =
                    clauses.iter().map(|c| c.build(field)).collect();
                Box::new(sb.near(built, *slop, *ordered))
            }
            SpanKind::Containing(big, little) => {
                Box::new(sb.containing(big.build(field), little.build(field)))
            }
            SpanKind::Within(include, exclude, dist) => {
                Box::new(sb.within(include.build(field), exclude.build(field), *dist))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Query data structs (used internally by search.rs and index.rs)
// ---------------------------------------------------------------------------

pub struct JsTermQuery {
    pub field: String,
    pub term: String,
}

pub struct JsPhraseQuery {
    pub field: String,
    pub terms: Vec<String>,
}

pub struct JsFuzzyQuery {
    pub field: String,
    pub term: String,
    pub max_edits: u32,
}

pub struct JsWildcardQuery {
    pub field: String,
    pub pattern: String,
}

pub struct JsNumericRangeQuery {
    pub field: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub is_float: bool,
}

impl JsNumericRangeQuery {
    pub fn build(&self) -> Box<dyn laurus::lexical::Query> {
        if self.is_float {
            Box::new(NumericRangeQuery::f64_range(
                &self.field,
                self.min,
                self.max,
            ))
        } else {
            Box::new(NumericRangeQuery::i64_range(
                &self.field,
                self.min.map(|v| v as i64),
                self.max.map(|v| v as i64),
            ))
        }
    }
}

pub struct JsGeoQuery {
    pub field: String,
    pub kind: GeoKind,
}

#[derive(Clone)]
pub enum GeoKind {
    Radius {
        lat: f64,
        lon: f64,
        distance_km: f64,
    },
    BoundingBox {
        min_lat: f64,
        min_lon: f64,
        max_lat: f64,
        max_lon: f64,
    },
}

impl JsGeoQuery {
    pub fn build(&self) -> laurus::Result<Box<dyn laurus::lexical::Query>> {
        match &self.kind {
            GeoKind::Radius {
                lat,
                lon,
                distance_km,
            } => Ok(Box::new(GeoQuery::within_radius(
                &self.field,
                *lat,
                *lon,
                *distance_km,
            )?)),
            GeoKind::BoundingBox {
                min_lat,
                min_lon,
                max_lat,
                max_lon,
            } => Ok(Box::new(GeoQuery::within_bounding_box(
                &self.field,
                *min_lat,
                *min_lon,
                *max_lat,
                *max_lon,
            )?)),
        }
    }
}

pub struct JsBooleanQuery {
    pub musts: Vec<JsQuery>,
    pub shoulds: Vec<JsQuery>,
    pub must_nots: Vec<JsQuery>,
}

impl JsBooleanQuery {
    pub fn build_query(&self) -> Result<Box<dyn laurus::lexical::Query>, JsValue> {
        let mut bq = BooleanQuery::new();
        for q in &self.musts {
            bq.add_must(extract_lexical_query(q)?);
        }
        for q in &self.shoulds {
            bq.add_should(extract_lexical_query(q)?);
        }
        for q in &self.must_nots {
            bq.add_must_not(extract_lexical_query(q)?);
        }
        Ok(Box::new(bq))
    }
}

pub struct JsSpanQuery {
    pub field: String,
    pub kind: SpanKind,
}

pub struct JsVectorQueryInner {
    pub field: String,
    pub vector: Vec<f32>,
}

pub struct JsVectorTextQuery {
    pub field: String,
    pub text: String,
}
