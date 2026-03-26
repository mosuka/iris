//! Node.js wrappers for all Laurus query types.
//!
//! Each query class stores the data needed to construct the Rust query.
//! Vector query classes produce `VectorSearchQuery` instead of lexical queries.

use laurus::lexical::span::{SpanQueryBuilder, SpanQueryWrapper};
use laurus::lexical::{
    BooleanQuery, FuzzyQuery, GeoQuery, NumericRangeQuery, PhraseQuery, TermQuery, WildcardQuery,
};
use laurus::vector::Vector;
use laurus::vector::store::request::QueryVector;
use laurus::{DataValue, LexicalSearchQuery, QueryPayload, VectorSearchQuery};
use napi::bindgen_prelude::*;
use napi_derive::napi;

// ---------------------------------------------------------------------------
// Helper: extract a lexical query from a JsQuery enum
// ---------------------------------------------------------------------------

/// Extract a Laurus lexical query from a [`JsQuery`] enum variant.
///
/// # Arguments
///
/// * `query` - A reference to a `JsQuery` enum.
///
/// # Returns
///
/// A boxed Laurus lexical query trait object.
pub fn extract_lexical_query(query: &JsQuery) -> Result<Box<dyn laurus::lexical::Query>> {
    match query {
        JsQuery::TermQuery(q) => Ok(Box::new(TermQuery::new(&q.field, &q.term))),
        JsQuery::PhraseQuery(q) => Ok(Box::new(PhraseQuery::new(&q.field, q.terms.clone()))),
        JsQuery::FuzzyQuery(q) => Ok(Box::new(
            FuzzyQuery::new(&q.field, &q.term).max_edits(q.max_edits),
        )),
        JsQuery::WildcardQuery(q) => Ok(Box::new(
            WildcardQuery::new(&q.field, &q.pattern)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?,
        )),
        JsQuery::NumericRangeQuery(q) => Ok(q.build()),
        JsQuery::GeoQuery(q) => q
            .build()
            .map_err(|e| napi::Error::from_reason(e.to_string())),
        JsQuery::BooleanQuery(q) => q.build_query(),
        JsQuery::SpanQuery(q) => Ok(Box::new(SpanQueryWrapper::new(q.kind.build(&q.field)))),
    }
}

/// Convert a [`JsQuery`] into a [`LexicalSearchQuery`].
///
/// # Arguments
///
/// * `query` - A reference to a `JsQuery` enum.
///
/// # Returns
///
/// A `LexicalSearchQuery::Obj` wrapping the extracted query.
pub fn query_to_lexical_search_query(query: &JsQuery) -> Result<LexicalSearchQuery> {
    Ok(LexicalSearchQuery::Obj(extract_lexical_query(query)?))
}

/// Convert a [`JsVectorQuery`] into a [`VectorSearchQuery`].
///
/// # Arguments
///
/// * `query` - A reference to a `JsVectorQuery` enum.
///
/// # Returns
///
/// A `VectorSearchQuery` for the engine.
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
// Query union types (napi does not support trait objects, so we use enums)
// ---------------------------------------------------------------------------

/// Enum wrapping all supported lexical query types.
///
/// Used internally to pass query objects across the JS/Rust boundary.
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
// Internal span-query recipe enum (Clone so it can be nested)
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
// TermQuery
// ---------------------------------------------------------------------------

/// Exact single-term lexical query.
///
/// ## Example
///
/// ```javascript
/// const { TermQuery } = require("@laurus/nodejs");
/// const q = new TermQuery("body", "rust");
/// const results = await index.search(q, { limit: 5 });
/// ```
#[napi(js_name = "TermQuery")]
pub struct JsTermQuery {
    pub(crate) field: String,
    pub(crate) term: String,
}

#[napi]
impl JsTermQuery {
    /// Create a new term query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to search in.
    /// * `term` - The exact term to match.
    #[napi(constructor)]
    pub fn new(field: String, term: String) -> Self {
        Self { field, term }
    }
}

// ---------------------------------------------------------------------------
// PhraseQuery
// ---------------------------------------------------------------------------

/// Exact phrase (word-sequence) lexical query.
///
/// ## Example
///
/// ```javascript
/// const { PhraseQuery } = require("@laurus/nodejs");
/// const q = new PhraseQuery("body", ["machine", "learning"]);
/// ```
#[napi(js_name = "PhraseQuery")]
pub struct JsPhraseQuery {
    pub(crate) field: String,
    pub(crate) terms: Vec<String>,
}

#[napi]
impl JsPhraseQuery {
    /// Create a new phrase query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to search in.
    /// * `terms` - The ordered list of terms that form the phrase.
    #[napi(constructor)]
    pub fn new(field: String, terms: Vec<String>) -> Self {
        Self { field, terms }
    }
}

// ---------------------------------------------------------------------------
// FuzzyQuery
// ---------------------------------------------------------------------------

/// Approximate (typo-tolerant) lexical query.
///
/// ## Example
///
/// ```javascript
/// const { FuzzyQuery } = require("@laurus/nodejs");
/// const q = new FuzzyQuery("body", "programing", 2);
/// ```
#[napi(js_name = "FuzzyQuery")]
pub struct JsFuzzyQuery {
    pub(crate) field: String,
    pub(crate) term: String,
    pub(crate) max_edits: u32,
}

#[napi]
impl JsFuzzyQuery {
    /// Create a new fuzzy query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to search in.
    /// * `term` - The approximate term to match.
    /// * `max_edits` - Maximum edit distance (default 2).
    #[napi(constructor)]
    pub fn new(field: String, term: String, max_edits: Option<u32>) -> Self {
        Self {
            field,
            term,
            max_edits: max_edits.unwrap_or(2),
        }
    }
}

// ---------------------------------------------------------------------------
// WildcardQuery
// ---------------------------------------------------------------------------

/// Wildcard pattern lexical query (`*` = any sequence, `?` = any character).
///
/// ## Example
///
/// ```javascript
/// const { WildcardQuery } = require("@laurus/nodejs");
/// const q = new WildcardQuery("filename", "*.pdf");
/// ```
#[napi(js_name = "WildcardQuery")]
pub struct JsWildcardQuery {
    pub(crate) field: String,
    pub(crate) pattern: String,
}

#[napi]
impl JsWildcardQuery {
    /// Create a new wildcard query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to search in.
    /// * `pattern` - The wildcard pattern (`*` = any sequence, `?` = single character).
    #[napi(constructor)]
    pub fn new(field: String, pattern: String) -> Self {
        Self { field, pattern }
    }
}

// ---------------------------------------------------------------------------
// NumericRangeQuery
// ---------------------------------------------------------------------------

/// Numeric range filter query (integer or float).
///
/// ## Example
///
/// ```javascript
/// const { NumericRangeQuery } = require("@laurus/nodejs");
/// const q = new NumericRangeQuery("year", 2020, 2023);
/// ```
#[napi(js_name = "NumericRangeQuery")]
pub struct JsNumericRangeQuery {
    pub(crate) field: String,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) is_float: bool,
}

#[napi]
impl JsNumericRangeQuery {
    /// Create a new numeric range query.
    ///
    /// Pass integer values for integer range, or use `isFloat: true` for float range.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to filter on.
    /// * `min` - Minimum value (inclusive), or `null` for unbounded.
    /// * `max` - Maximum value (inclusive), or `null` for unbounded.
    /// * `is_float` - Whether to treat values as float (default `false`, integer).
    #[napi(constructor)]
    pub fn new(field: String, min: Option<f64>, max: Option<f64>, is_float: Option<bool>) -> Self {
        Self {
            field,
            min,
            max,
            is_float: is_float.unwrap_or(false),
        }
    }
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

// ---------------------------------------------------------------------------
// GeoQuery
// ---------------------------------------------------------------------------

/// Geographic search query (radius or bounding box).
///
/// Use the static factory methods to create queries:
///
/// ## Example
///
/// ```javascript
/// const { GeoQuery } = require("@laurus/nodejs");
///
/// // Radius search: within 100 km of San Francisco
/// const q = GeoQuery.withinRadius("location", 37.77, -122.42, 100.0);
///
/// // Bounding box search
/// const q2 = GeoQuery.withinBoundingBox("location", 33.0, -123.0, 48.0, -117.0);
/// ```
#[napi(js_name = "GeoQuery")]
pub struct JsGeoQuery {
    pub(crate) field: String,
    pub(crate) kind: GeoKind,
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

#[napi]
impl JsGeoQuery {
    /// Create a radius-based geo query.
    ///
    /// # Arguments
    ///
    /// * `field` - Geo field name.
    /// * `lat` - Center latitude.
    /// * `lon` - Center longitude.
    /// * `distance_km` - Search radius in kilometers.
    #[napi(factory)]
    pub fn within_radius(field: String, lat: f64, lon: f64, distance_km: f64) -> Self {
        Self {
            field,
            kind: GeoKind::Radius {
                lat,
                lon,
                distance_km,
            },
        }
    }

    /// Create a bounding-box geo query.
    ///
    /// # Arguments
    ///
    /// * `field` - Geo field name.
    /// * `min_lat` - Southern boundary.
    /// * `min_lon` - Western boundary.
    /// * `max_lat` - Northern boundary.
    /// * `max_lon` - Eastern boundary.
    #[napi(factory)]
    pub fn within_bounding_box(
        field: String,
        min_lat: f64,
        min_lon: f64,
        max_lat: f64,
        max_lon: f64,
    ) -> Self {
        Self {
            field,
            kind: GeoKind::BoundingBox {
                min_lat,
                min_lon,
                max_lat,
                max_lon,
            },
        }
    }
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

// ---------------------------------------------------------------------------
// BooleanQuery
// ---------------------------------------------------------------------------

/// Boolean combination query (AND / OR / NOT).
///
/// ## Example
///
/// ```javascript
/// const { BooleanQuery, TermQuery } = require("@laurus/nodejs");
///
/// const bq = new BooleanQuery();
/// bq.must(new TermQuery("body", "programming"));
/// bq.mustNot(new TermQuery("body", "python"));
/// bq.should(new TermQuery("category", "data-science"));
/// const results = await index.search(bq, { limit: 5 });
/// ```
#[napi(js_name = "BooleanQuery")]
pub struct JsBooleanQuery {
    pub(crate) musts: Vec<JsQuery>,
    pub(crate) shoulds: Vec<JsQuery>,
    pub(crate) must_nots: Vec<JsQuery>,
}

#[napi]
impl JsBooleanQuery {
    /// Create a new empty boolean query.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            musts: Vec::new(),
            shoulds: Vec::new(),
            must_nots: Vec::new(),
        }
    }

    /// Add a MUST (required) clause with a term query.
    ///
    /// # Arguments
    ///
    /// * `query` - The lexical query to add as a required clause.
    #[napi]
    pub fn must_term(&mut self, field: String, term: String) {
        self.musts
            .push(JsQuery::TermQuery(JsTermQuery { field, term }));
    }

    /// Add a SHOULD (optional, boosts score) clause with a term query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name.
    /// * `term` - The term to match.
    #[napi]
    pub fn should_term(&mut self, field: String, term: String) {
        self.shoulds
            .push(JsQuery::TermQuery(JsTermQuery { field, term }));
    }

    /// Add a MUST_NOT (exclusion) clause with a term query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name.
    /// * `term` - The term to exclude.
    #[napi]
    pub fn must_not_term(&mut self, field: String, term: String) {
        self.must_nots
            .push(JsQuery::TermQuery(JsTermQuery { field, term }));
    }
}

impl JsBooleanQuery {
    /// Build the underlying Rust [`BooleanQuery`].
    pub fn build_query(&self) -> Result<Box<dyn laurus::lexical::Query>> {
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

// ---------------------------------------------------------------------------
// SpanQuery
// ---------------------------------------------------------------------------

/// Positional / proximity span query.
///
/// Use the static factory methods to construct span queries.
///
/// ## Example
///
/// ```javascript
/// const { SpanQuery } = require("@laurus/nodejs");
///
/// // SpanNear: "quick" within 1 position of "fox", in order
/// const q = SpanQuery.near("body", ["quick", "fox"], 1, true);
/// ```
#[napi(js_name = "SpanQuery")]
pub struct JsSpanQuery {
    pub(crate) field: String,
    pub(crate) kind: SpanKind,
}

#[napi]
impl JsSpanQuery {
    /// Single-term span query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to search in.
    /// * `term` - The term to match.
    #[napi(factory)]
    pub fn term(field: String, term: String) -> Self {
        Self {
            field,
            kind: SpanKind::Term(term),
        }
    }

    /// SpanNear: terms appearing within `slop` positions of each other.
    ///
    /// # Arguments
    ///
    /// * `field` - Field to search.
    /// * `terms` - List of term strings.
    /// * `slop` - Maximum token distance between terms (default 0).
    /// * `ordered` - Whether terms must appear in the given order (default `true`).
    #[napi(factory)]
    pub fn near(
        field: String,
        terms: Vec<String>,
        slop: Option<u32>,
        ordered: Option<bool>,
    ) -> Self {
        let kinds = terms.into_iter().map(SpanKind::Term).collect();
        Self {
            field,
            kind: SpanKind::Near(kinds, slop.unwrap_or(0), ordered.unwrap_or(true)),
        }
    }

    /// SpanNear with nested SpanQuery clauses instead of plain terms.
    ///
    /// # Arguments
    ///
    /// * `field` - Field to search.
    /// * `clauses` - List of SpanQuery objects.
    /// * `slop` - Maximum token distance (default 0).
    /// * `ordered` - Whether clauses must appear in order (default `true`).
    #[napi(factory)]
    pub fn near_spans(
        field: String,
        clauses: Vec<&JsSpanQuery>,
        slop: Option<u32>,
        ordered: Option<bool>,
    ) -> Self {
        let kinds: Vec<SpanKind> = clauses.iter().map(|c| c.kind.clone()).collect();
        Self {
            field,
            kind: SpanKind::Near(kinds, slop.unwrap_or(0), ordered.unwrap_or(true)),
        }
    }

    /// SpanContaining: a span that contains another span.
    ///
    /// # Arguments
    ///
    /// * `field` - Field to search.
    /// * `big` - The outer span query.
    /// * `little` - The inner span query that must be contained.
    #[napi(factory)]
    pub fn containing(field: String, big: &JsSpanQuery, little: &JsSpanQuery) -> Self {
        Self {
            field,
            kind: SpanKind::Containing(Box::new(big.kind.clone()), Box::new(little.kind.clone())),
        }
    }

    /// SpanWithin: a span included within another span, at a maximum distance.
    ///
    /// # Arguments
    ///
    /// * `field` - Field to search.
    /// * `include` - The span to include.
    /// * `exclude` - The span to measure distance from.
    /// * `distance` - Maximum distance.
    #[napi(factory)]
    pub fn within(
        field: String,
        include: &JsSpanQuery,
        exclude: &JsSpanQuery,
        distance: u32,
    ) -> Self {
        Self {
            field,
            kind: SpanKind::Within(
                Box::new(include.kind.clone()),
                Box::new(exclude.kind.clone()),
                distance,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// VectorQuery (pre-computed vector)
// ---------------------------------------------------------------------------

/// Vector search query using a pre-computed embedding vector.
///
/// ## Example
///
/// ```javascript
/// const { VectorQuery } = require("@laurus/nodejs");
/// const results = await index.search(new VectorQuery("text_vec", [0.1, 0.2, ...]));
/// ```
#[napi(js_name = "VectorQuery")]
pub struct JsVectorQueryInner {
    pub(crate) field: String,
    pub(crate) vector: Vec<f32>,
}

#[napi]
impl JsVectorQueryInner {
    /// Create a new vector query with a pre-computed embedding.
    ///
    /// # Arguments
    ///
    /// * `field` - The vector field name.
    /// * `vector` - The embedding vector as an array of numbers.
    #[napi(constructor)]
    pub fn new(field: String, vector: Vec<f64>) -> Self {
        Self {
            field,
            vector: vector.into_iter().map(|v| v as f32).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// VectorTextQuery (text → Rust Embedder → vector)
// ---------------------------------------------------------------------------

/// Vector search query where the text is embedded by the Rust-side Embedder.
///
/// ## Example
///
/// ```javascript
/// const { VectorTextQuery } = require("@laurus/nodejs");
/// const results = await index.search(new VectorTextQuery("text_vec", "memory safety"));
/// ```
#[napi(js_name = "VectorTextQuery")]
pub struct JsVectorTextQuery {
    pub(crate) field: String,
    pub(crate) text: String,
}

#[napi]
impl JsVectorTextQuery {
    /// Create a new text-based vector query.
    ///
    /// The text will be automatically embedded by the registered embedder.
    ///
    /// # Arguments
    ///
    /// * `field` - The vector field name.
    /// * `text` - The text to embed and search with.
    #[napi(constructor)]
    pub fn new(field: String, text: String) -> Self {
        Self { field, text }
    }
}
