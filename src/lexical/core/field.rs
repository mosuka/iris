//! Field value types and field options for documents.
//!
//! This module defines:
//! - [`Field`] - A struct combining a value and its indexing options
//! - [`FieldValue`] - The value stored in a field (Text, Integer, etc.)
//! - [`FieldOption`] - Type-specific indexing options (TextOption, VectorOption, etc.)
//!
//! # Field Structure
//!
//! Each field consists of:
//! - **value**: The actual data (FieldValue)
//! - **option**: How the field should be indexed (FieldOption)
//!
//! # Supported Types
//!
//! - **Text** - String data for full-text search
//! - **Blob** - Raw byte data or vectors
//! - **DateTime** - UTC timestamps with timezone
//! - **Geo** - Geographic coordinates (latitude/longitude)
//! - **Null** - Explicit null values
//!
//! # Type Conversion
//!
//! The `FieldValue` enum provides conversion methods for extracting typed values:
//!
//! ```
//! use iris::lexical::core::field::FieldValue;
//!
//! let text_value = FieldValue::Text("hello".to_string());
//! assert_eq!(text_value.as_text(), Some("hello"));
//!
//! let int_value = FieldValue::Integer(42);
//! assert_eq!(int_value.as_numeric(), Some("42".to_string()));
//!
//! let bool_value = FieldValue::Boolean(true);
//! assert_eq!(bool_value.as_boolean(), Some(true));
//! ```
//!
//! # Type Inference
//!
//! String values can be interpreted as different types:
//!
//! ```
//! use iris::lexical::core::field::FieldValue;
//!
//! // Boolean inference from text
//! let text = FieldValue::Text("true".to_string());
//! assert_eq!(text.as_boolean(), Some(true));
//!
//! let text2 = FieldValue::Text("yes".to_string());
//! assert_eq!(text2.as_boolean(), Some(true));
//! ```

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::lexical::index::inverted::query::geo::GeoPoint;

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
        // unsafe block not strictly needed if we don't call unsafe fns, but resolve might be unsafe?
        // In 0.8 traits resolve is safe? No ArchiveWith::resolve_with might not be unsafe?
        // Actually ArchiveWith::resolve_with is NOT unsafe in 0.8? Check docs or assume consistent with signature.
        // Wait, standard trait is: fn resolve_with(field: &F, resolver: <Self as ArchiveWith<F>>::Resolver, out: Place<<Self as ArchiveWith<F>>::Archived>)
        // It is not unsafe.
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

/// A field combines a value with indexing options.
///
/// This struct represents a complete field in a document, containing both
/// the data (value) and metadata about how it should be indexed (option).
///
/// # Examples
///
/// ```
/// use iris::lexical::core::field::{Field, FieldValue, FieldOption, TextOption};
///
/// // Create a text field with custom options
/// let field = Field {
///     value: FieldValue::Text("Rust Programming".to_string()),
///     option: FieldOption::Text(TextOption {
///         indexed: true,
///         stored: true,
///         term_vectors: true,
///     }),
/// };
/// ```
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct Field {
    /// The field value.
    pub value: FieldValue,

    /// The field indexing options.
    pub option: FieldOption,
}

impl Field {
    /// Create a new field with a value and option.
    pub fn new(value: FieldValue, option: FieldOption) -> Self {
        Self { value, option }
    }

    /// Create a field with the option inferred from the value type.
    pub fn with_default_option(value: FieldValue) -> Self {
        let option = FieldOption::from_field_value(&value);
        Self { value, option }
    }
}

/// Numeric type classification for numeric range queries.
///
/// This enum is used internally to distinguish between integer and
/// floating-point numeric types when performing range queries.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]

pub enum NumericType {
    /// Integer type (i64).
    Integer,
    /// Float type (f64).
    Float,
}

/// Represents a value for a field in a document.
///
/// This enum provides a flexible type system for document fields, supporting
/// various data types commonly used in search and indexing applications.
///
/// # Serialization
///
/// DateTime values are serialized using their UTC timestamp representation
/// for compatibility with bincode and other binary formats.
///
/// # Examples
///
/// Creating field values:
///
/// ```
/// use iris::lexical::core::field::FieldValue;
///
/// let text = FieldValue::Text("Rust Programming".to_string());
/// let number = FieldValue::Integer(2024);
/// let price = FieldValue::Float(39.99);
/// let active = FieldValue::Boolean(true);
/// let data = FieldValue::Blob("application/octet-stream".to_string(), vec![0x00, 0x01, 0x02]);
/// ```
///
/// Extracting typed values:
///
/// ```
/// use iris::lexical::core::field::FieldValue;
///
/// let value = FieldValue::Integer(100);
/// assert_eq!(value.as_numeric(), Some("100".to_string()));
///
/// let text = FieldValue::Text("42".to_string());
/// assert_eq!(text.as_text(), Some("42"));
/// ```
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub enum FieldValue {
    /// Text value
    Text(String),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// DateTime value
    DateTime(#[rkyv(with = MicroSeconds)] chrono::DateTime<chrono::Utc>),
    /// Geographic point value
    Geo(GeoPoint),
    /// Blob value (MIME type, Data)
    Blob(String, Vec<u8>),
    /// Null value
    Null,
}

impl FieldValue {
    /// Convert to text if this is a text value.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            FieldValue::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Convert to numeric string representation.
    pub fn as_numeric(&self) -> Option<String> {
        match self {
            FieldValue::Integer(i) => Some(i.to_string()),
            FieldValue::Float(f) => Some(f.to_string()),
            _ => None,
        }
    }

    /// Convert to datetime string representation (RFC3339).
    pub fn as_datetime(&self) -> Option<String> {
        match self {
            FieldValue::Text(s) => {
                // Try to parse as datetime and return as string if valid
                if s.parse::<chrono::DateTime<chrono::Utc>>().is_ok() {
                    Some(s.clone())
                } else {
                    None
                }
            }
            FieldValue::Integer(timestamp) => {
                // Treat as Unix timestamp
                chrono::DateTime::from_timestamp(*timestamp, 0).map(|dt| dt.to_rfc3339())
            }
            _ => None,
        }
    }

    /// Convert to boolean.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            FieldValue::Boolean(b) => Some(*b),
            FieldValue::Text(s) => match s.to_lowercase().as_str() {
                "true" | "t" | "yes" | "y" | "1" | "on" => Some(true),
                "false" | "f" | "no" | "n" | "0" | "off" => Some(false),
                _ => None,
            },
            FieldValue::Integer(i) => Some(*i != 0),
            _ => None,
        }
    }

    /// Get the value as binary data, if possible.
    pub fn as_blob(&self) -> Option<(&str, &[u8])> {
        match self {
            FieldValue::Blob(mime, data) => Some((mime, data)),
            _ => None,
        }
    }

    /// Convert to GeoPoint if this is a geo value.
    pub fn as_geo(&self) -> Option<&GeoPoint> {
        match self {
            FieldValue::Geo(point) => Some(point),
            _ => None,
        }
    }
}

// ============================================================================
// Field Options - Configuration for indexing and storage
// ============================================================================

/// Options for Text fields (used by Lexical indexing).
///
/// Controls how text fields are analyzed, indexed, and stored.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct TextOption {
    /// Whether to index this field for search.
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether to store the original field value.
    #[serde(default = "default_true")]
    pub stored: bool,

    /// Whether to store term vectors (enables highlighting, more-like-this).
    #[serde(default)]
    pub term_vectors: bool,
}

impl Default for TextOption {
    fn default() -> Self {
        Self {
            indexed: true,
            stored: true,
            term_vectors: false,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Option for Blob field.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]

pub struct BlobOption {
    /// If true, the value is stored.
    pub stored: bool,
}

impl Default for BlobOption {
    fn default() -> Self {
        Self { stored: true }
    }
}

impl BlobOption {
    /// Create a new blob option.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Options for Integer fields.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct IntegerOption {
    /// Whether to index this field for range queries.
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether to store the original value.
    #[serde(default = "default_true")]
    pub stored: bool,
}

impl Default for IntegerOption {
    fn default() -> Self {
        Self {
            indexed: true,
            stored: true,
        }
    }
}

/// Options for Float fields.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct FloatOption {
    /// Whether to index this field for range queries.
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether to store the original value.
    #[serde(default = "default_true")]
    pub stored: bool,
}

impl Default for FloatOption {
    fn default() -> Self {
        Self {
            indexed: true,
            stored: true,
        }
    }
}

/// Options for Boolean fields.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct BooleanOption {
    /// Whether to index this field.
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether to store the original value.
    #[serde(default = "default_true")]
    pub stored: bool,
}

impl Default for BooleanOption {
    fn default() -> Self {
        Self {
            indexed: true,
            stored: true,
        }
    }
}

/// Options for DateTime fields.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct DateTimeOption {
    /// Whether to index this field for range queries.
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether to store the original value.
    #[serde(default = "default_true")]
    pub stored: bool,
}

impl Default for DateTimeOption {
    fn default() -> Self {
        Self {
            indexed: true,
            stored: true,
        }
    }
}

/// Options for Geo (geographic point) fields.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct GeoOption {
    /// Whether to index this field for geo queries.
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether to store the original value.
    #[serde(default = "default_true")]
    pub stored: bool,
}

/// Unified field option type that wraps all field-specific options.
///
/// This enum provides a type-safe way to store configuration options
/// for different field types within a Document structure.
///
/// # Examples
///
/// ```
/// use iris::lexical::core::field::{FieldOption, TextOption, BlobOption};
///
/// // Text field with custom options
/// let text_opt = FieldOption::Text(TextOption {
///     indexed: true,
///     stored: true,
///     term_vectors: true,
/// });
///
/// // Blob field (e.g. for vector source)
/// let blob_opt = FieldOption::Blob(BlobOption::default());
/// ```
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub enum FieldOption {
    /// Options for text fields (lexical search).
    Text(TextOption),

    /// Options for integer fields.
    Integer(IntegerOption),

    /// Options for float fields.
    Float(FloatOption),

    /// Options for boolean fields.
    Boolean(BooleanOption),

    /// Options for blob fields (binary data and vectors).
    Blob(BlobOption),

    /// Options for datetime fields.
    DateTime(DateTimeOption),

    /// Options for geographic point fields.
    Geo(GeoOption),
}

impl Default for FieldOption {
    fn default() -> Self {
        FieldOption::Text(TextOption::default())
    }
}

impl FieldOption {
    /// Create a default option based on the field value type.
    ///
    /// This method infers appropriate default options based on the
    /// type of field value.
    pub fn from_field_value(value: &FieldValue) -> Self {
        match value {
            FieldValue::Text(_) => FieldOption::Text(TextOption::default()),
            FieldValue::Integer(_) => FieldOption::Integer(IntegerOption::default()),
            FieldValue::Float(_) => FieldOption::Float(FloatOption::default()),
            FieldValue::Boolean(_) => FieldOption::Boolean(BooleanOption::default()),
            FieldValue::DateTime(_) => FieldOption::DateTime(DateTimeOption::default()),
            FieldValue::Geo(_) => FieldOption::Geo(GeoOption::default()),
            FieldValue::Blob(_, _) => FieldOption::Blob(BlobOption::default()), // Default to BlobOption
            FieldValue::Null => FieldOption::Text(TextOption::default()),
        }
    }
}

impl Default for GeoOption {
    fn default() -> Self {
        Self {
            indexed: true,
            stored: true,
        }
    }
}
