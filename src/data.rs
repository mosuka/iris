use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Helper for archiving DateTime as micros timestamp (i64)
pub struct MicroSeconds;

impl rkyv::with::ArchiveWith<chrono::DateTime<chrono::Utc>> for MicroSeconds {
    type Archived = rkyv::Archived<i64>;
    type Resolver = ();

    fn resolve_with(
        field: &chrono::DateTime<chrono::Utc>,
        _: (),
        out: rkyv::Place<Self::Archived>,
    ) {
        let ts = field.timestamp_micros();
        ts.resolve((), out);
    }
}

impl<S: rkyv::rancor::Fallible + ?Sized> rkyv::with::SerializeWith<chrono::DateTime<chrono::Utc>, S>
    for MicroSeconds
{
    fn serialize_with(
        field: &chrono::DateTime<chrono::Utc>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        RkyvSerialize::serialize(&field.timestamp_micros(), serializer)
    }
}

impl<D: rkyv::rancor::Fallible + ?Sized>
    rkyv::with::DeserializeWith<rkyv::Archived<i64>, chrono::DateTime<chrono::Utc>, D>
    for MicroSeconds
{
    fn deserialize_with(
        archived: &rkyv::Archived<i64>,
        _deserializer: &mut D,
    ) -> Result<chrono::DateTime<chrono::Utc>, D::Error> {
        use chrono::TimeZone;
        let ts: i64 = (*archived).into();
        Ok(chrono::Utc.timestamp_micros(ts).single().unwrap())
    }
}

/// The unified value type for fields in a document.
///
/// This enum merges the concepts of `FieldValue` (from Lexical Index) and
/// `VectorValue` (from Vector Index).
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub enum DataValue {
    // --- Primitive Types (Metadata / Lexical / Storage) ---
    Null,
    Bool(bool),
    Int64(i64),
    Float64(f64),

    /// String content typically used for keywords, IDs, or non-tokenized metadata.
    String(String),

    // --- Complex / Searchable Types ---
    /// Text content to be full-text indexed and/or embedded into a vector.
    Text(String),

    /// Binary content (image, audio, etc.) to be embedded.
    /// Contains the raw bytes and an optional MIME type.
    Bytes(Vec<u8>, Option<String>),

    /// Pre-computed vector.
    Vector(Vec<f32>),

    /// List of values (e.g. tags).
    List(Vec<String>),

    /// Date and time in UTC.
    DateTime(#[rkyv(with = MicroSeconds)] chrono::DateTime<chrono::Utc>),

    /// Geographical point (latitude, longitude).
    Geo(f64, f64),
}

impl DataValue {
    /// Returns the text value if this is a Text variant.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            DataValue::Text(s) | DataValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the integer value if this is an Int64 variant.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DataValue::Int64(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the float value if this is a Float64 variant.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            DataValue::Float64(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the boolean value if this is a Bool variant.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the datetime value if this is a DateTime variant.
    pub fn as_datetime(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match self {
            DataValue::DateTime(dt) => Some(*dt),
            _ => None,
        }
    }

    /// Returns the vector data if this is a Vector variant.
    pub fn as_vector_ref(&self) -> Option<&Vec<f32>> {
        match self {
            DataValue::Vector(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the bytes data if this is a Bytes variant.
    pub fn as_bytes_ref(&self) -> Option<&[u8]> {
        match self {
            DataValue::Bytes(b, _) => Some(b),
            _ => None,
        }
    }

    /// Returns the geographical point if this is a Geo variant.
    pub fn as_geo(&self) -> Option<(f64, f64)> {
        match self {
            DataValue::Geo(lat, lon) => Some((*lat, *lon)),
            _ => None,
        }
    }
}

// --- Conversions ---

impl From<String> for DataValue {
    fn from(v: String) -> Self {
        DataValue::Text(v)
    }
}

impl From<&str> for DataValue {
    fn from(v: &str) -> Self {
        DataValue::Text(v.to_string())
    }
}

impl From<i64> for DataValue {
    fn from(v: i64) -> Self {
        DataValue::Int64(v)
    }
}

impl From<i32> for DataValue {
    fn from(v: i32) -> Self {
        DataValue::Int64(v as i64)
    }
}

impl From<f64> for DataValue {
    fn from(v: f64) -> Self {
        DataValue::Float64(v)
    }
}

impl From<f32> for DataValue {
    fn from(v: f32) -> Self {
        DataValue::Float64(v as f64)
    }
}

impl From<bool> for DataValue {
    fn from(v: bool) -> Self {
        DataValue::Bool(v)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for DataValue {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        DataValue::DateTime(dt)
    }
}

impl From<Vec<f32>> for DataValue {
    fn from(v: Vec<f32>) -> Self {
        DataValue::Vector(v.into())
    }
}

/// Unified Document structure.
///
/// A document is a collection of named fields, each containing a `DataValue`.
/// This structure is used across both Lexical and Vector engines.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Document {
    /// Optional unique identifier for the document.
    pub id: Option<String>,

    /// Field data.
    pub fields: HashMap<String, DataValue>,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            id: None,
            fields: HashMap::new(),
        }
    }

    /// Set the document ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a field to the document.
    pub fn with_field(mut self, name: impl Into<String>, value: impl Into<DataValue>) -> Self {
        self.fields.insert(name.into(), value.into());
        self
    }

    /// Alias for with_field (compatibility with Lexical).
    pub fn add_field(self, name: impl Into<String>, value: impl Into<DataValue>) -> Self {
        self.with_field(name, value)
    }

    /// Get a reference to a field's value.
    pub fn get(&self, name: &str) -> Option<&DataValue> {
        self.fields.get(name)
    }

    /// Alias for get (compatibility with Lexical).
    pub fn get_field(&self, name: &str) -> Option<&DataValue> {
        self.get(name)
    }

    /// Check if the document has a field.
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    /// Get all field names.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of fields.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if the document is empty.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Builder for constructing documents (compat with Lexical).
    pub fn builder() -> DocumentBuilder {
        DocumentBuilder::new()
    }
}

/// A simple builder for constructing documents.
#[derive(Debug, Clone, Default)]
pub struct DocumentBuilder {
    document: Document,
}

impl DocumentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.document.id = Some(id.into());
        self
    }

    pub fn with_field(mut self, name: impl Into<String>, value: impl Into<DataValue>) -> Self {
        self.document.fields.insert(name.into(), value.into());
        self
    }

    pub fn add_text(
        mut self,
        name: &str,
        text: &str,
        _opt: crate::lexical::core::field::TextOption,
    ) -> Self {
        self.document
            .fields
            .insert(name.to_string(), DataValue::Text(text.to_string()));
        self
    }

    pub fn add_integer(
        mut self,
        name: &str,
        value: i64,
        _opt: crate::lexical::core::field::IntegerOption,
    ) -> Self {
        self.document
            .fields
            .insert(name.to_string(), DataValue::Int64(value));
        self
    }

    pub fn add_float(
        mut self,
        name: &str,
        value: f64,
        _opt: crate::lexical::core::field::FloatOption,
    ) -> Self {
        self.document
            .fields
            .insert(name.to_string(), DataValue::Float64(value));
        self
    }

    pub fn add_boolean(
        mut self,
        name: &str,
        value: bool,
        _opt: crate::lexical::core::field::BooleanOption,
    ) -> Self {
        self.document
            .fields
            .insert(name.to_string(), DataValue::Bool(value));
        self
    }

    pub fn add_datetime(
        mut self,
        name: &str,
        value: chrono::DateTime<chrono::Utc>,
        _opt: crate::lexical::core::field::DateTimeOption,
    ) -> Self {
        self.document
            .fields
            .insert(name.to_string(), DataValue::DateTime(value));
        self
    }

    pub fn add_field(mut self, name: &str, value: DataValue) -> Self {
        self.document.fields.insert(name.to_string(), value);
        self
    }

    pub fn build(self) -> Document {
        self.document
    }
}
