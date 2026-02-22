//! Field value types and field options for documents.
//!
//! This module defines:
//! - [`Field`] - A struct combining a value and its indexing options
//! - [`FieldValue`] - The value stored in a field (Text, Integer, etc.)
//!   **Note:** This is now an alias for [`crate::data::DataValue`].
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
//! - **Bytes** - Raw byte data or vectors
//! - **DateTime** - UTC timestamps with timezone
//! - **Geo** - Geographic coordinates (latitude/longitude)
//! - **Null** - Explicit null values
//!
//! # Type Conversion
//!
//! The `FieldValue` enum provides conversion methods for extracting typed values:
//!
//! ```
//! use laurus::lexical::core::field::FieldValue;
//!
//! let text_value = FieldValue::Text("hello".to_string());
//! assert_eq!(text_value.as_text(), Some("hello"));
//!
//! let int_value = FieldValue::Int64(42);
//! assert_eq!(int_value.as_integer(), Some(42));
//!
//! let bool_value = FieldValue::Bool(true);
//! assert_eq!(bool_value.as_boolean(), Some(true));
//! ```
//!
//! // Type inference is not supported on DataValue alias.
//! let text = FieldValue::Text("true".to_string());
//! assert_eq!(text.as_boolean(), None);

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

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
/// use laurus::lexical::core::field::{Field, FieldValue, FieldOption, TextOption};
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

/// Alias to the unified [`crate::data::DataValue`].
///
/// For backward compatibility, `FieldValue` is preserved as an alias.
pub type FieldValue = crate::data::DataValue;

fn default_true() -> bool {
    true
}

// FieldValue (alias to DataValue) methods moved to src/data.rs

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
    #[serde(default = "default_true")]
    pub term_vectors: bool,
}

impl TextOption {
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }

    pub fn term_vectors(mut self, term_vectors: bool) -> Self {
        self.term_vectors = term_vectors;
        self
    }
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

/// Option for Bytes field.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]

pub struct BytesOption {
    /// If true, the value is stored.
    #[serde(default = "default_true")]
    pub stored: bool,
}

impl BytesOption {
    /// Set whether the field is stored.
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }
}

impl Default for BytesOption {
    fn default() -> Self {
        Self { stored: true }
    }
}

impl BytesOption {
    /// Create a new bytes option.
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

impl IntegerOption {
    /// Set whether the field is indexed.
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Set whether the field is stored.
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }
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

impl FloatOption {
    /// Set whether the field is indexed.
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Set whether the field is stored.
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }
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

impl BooleanOption {
    /// Set whether the field is indexed.
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Set whether the field is stored.
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }
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

impl DateTimeOption {
    /// Set whether the field is indexed.
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Set whether the field is stored.
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }
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

impl GeoOption {
    /// Set whether the field is indexed.
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Set whether the field is stored.
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }
}

/// Unified field option type that wraps all field-specific options.
///
/// This enum provides a type-safe way to store configuration options
/// for different field types within a Document structure.
///
/// # Examples
///
/// ```
/// use laurus::lexical::core::field::{FieldOption, TextOption, BytesOption};
///
/// // Text field with custom options
/// let text_opt = FieldOption::Text(TextOption {
///     indexed: true,
///     stored: true,
///     term_vectors: true,
/// });
///
/// // Bytes field (e.g. for binary data)
/// let bytes_opt = FieldOption::Bytes(BytesOption::default());
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

    /// Options for bytes fields (binary data and vectors).
    Bytes(BytesOption),

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
            FieldValue::Int64(_) => FieldOption::Integer(IntegerOption::default()),
            FieldValue::Float64(_) => FieldOption::Float(FloatOption::default()),
            FieldValue::Bool(_) => FieldOption::Boolean(BooleanOption::default()),
            FieldValue::Vector(_) | FieldValue::Bytes(_, _) => {
                FieldOption::Bytes(BytesOption::default())
            }
            FieldValue::DateTime(_) => FieldOption::DateTime(DateTimeOption::default()),
            FieldValue::Geo(_, _) => FieldOption::Geo(GeoOption::default()),
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
