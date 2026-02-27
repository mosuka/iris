use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lexical::core::field::{
    BooleanOption, BytesOption, DateTimeOption, FloatOption, GeoOption, IntegerOption, TextOption,
};
use crate::vector::core::field::{FlatOption, HnswOption, IvfOption};

/// Schema for the unified engine.
///
/// Declares what fields exist and their index types (lexical or vector).
/// Runtime configuration such as analyzers and embedders are provided
/// separately via [`super::EngineBuilder`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Options for each field.
    pub fields: HashMap<String, FieldOption>,
    /// Default fields for search.
    #[serde(default)]
    pub default_fields: Vec<String>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            default_fields: Vec::new(),
        }
    }

    pub fn builder() -> SchemaBuilder {
        SchemaBuilder::default()
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for a single field in the unified schema.
///
/// Each variant directly represents a concrete field type.
/// For hybrid search, define separate fields for vector and lexical indexing.
///
/// Serializes using serde's externally tagged representation:
/// ```json
/// { "Text": { "indexed": true, "stored": true, "term_vectors": false } }
/// { "Hnsw": { "dimension": 384, "distance": "Cosine" } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldOption {
    /// Text field options (lexical search).
    Text(TextOption),
    /// Integer field options.
    Integer(IntegerOption),
    /// Float field options.
    Float(FloatOption),
    /// Boolean field options.
    Boolean(BooleanOption),
    /// DateTime field options.
    DateTime(DateTimeOption),
    /// Geo field options.
    Geo(GeoOption),
    /// Bytes field options.
    Bytes(BytesOption),
    /// HNSW vector index options.
    Hnsw(HnswOption),
    /// Flat vector index options.
    Flat(FlatOption),
    /// IVF vector index options.
    Ivf(IvfOption),
}

impl FieldOption {
    /// Returns true if this is a vector field.
    pub fn is_vector(&self) -> bool {
        matches!(self, Self::Hnsw(_) | Self::Flat(_) | Self::Ivf(_))
    }

    /// Returns true if this is a lexical field.
    pub fn is_lexical(&self) -> bool {
        matches!(
            self,
            Self::Text(_)
                | Self::Integer(_)
                | Self::Float(_)
                | Self::Boolean(_)
                | Self::DateTime(_)
                | Self::Geo(_)
                | Self::Bytes(_)
        )
    }

    /// Converts to the vector-subsystem's `FieldOption` if this is a vector field.
    pub fn to_vector(&self) -> Option<crate::vector::core::field::FieldOption> {
        match self {
            Self::Hnsw(o) => Some(crate::vector::core::field::FieldOption::Hnsw(o.clone())),
            Self::Flat(o) => Some(crate::vector::core::field::FieldOption::Flat(o.clone())),
            Self::Ivf(o) => Some(crate::vector::core::field::FieldOption::Ivf(o.clone())),
            _ => None,
        }
    }

    /// Converts to the lexical-subsystem's `FieldOption` if this is a lexical field.
    pub fn to_lexical(&self) -> Option<crate::lexical::core::field::FieldOption> {
        match self {
            Self::Text(o) => Some(crate::lexical::core::field::FieldOption::Text(o.clone())),
            Self::Integer(o) => {
                Some(crate::lexical::core::field::FieldOption::Integer(o.clone()))
            }
            Self::Float(o) => Some(crate::lexical::core::field::FieldOption::Float(o.clone())),
            Self::Boolean(o) => {
                Some(crate::lexical::core::field::FieldOption::Boolean(o.clone()))
            }
            Self::DateTime(o) => {
                Some(crate::lexical::core::field::FieldOption::DateTime(o.clone()))
            }
            Self::Geo(o) => Some(crate::lexical::core::field::FieldOption::Geo(o.clone())),
            Self::Bytes(o) => Some(crate::lexical::core::field::FieldOption::Bytes(o.clone())),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct SchemaBuilder {
    fields: HashMap<String, FieldOption>,
    default_fields: Vec<String>,
}

impl SchemaBuilder {
    pub fn add_field(mut self, name: impl Into<String>, option: FieldOption) -> Self {
        let name = name.into();
        self.fields.insert(name, option);
        self
    }

    pub fn add_text_field(self, name: impl Into<String>, option: impl Into<TextOption>) -> Self {
        self.add_field(name, FieldOption::Text(option.into()))
    }

    pub fn add_integer_field(
        self,
        name: impl Into<String>,
        option: impl Into<IntegerOption>,
    ) -> Self {
        self.add_field(name, FieldOption::Integer(option.into()))
    }

    pub fn add_float_field(self, name: impl Into<String>, option: impl Into<FloatOption>) -> Self {
        self.add_field(name, FieldOption::Float(option.into()))
    }

    pub fn add_boolean_field(
        self,
        name: impl Into<String>,
        option: impl Into<BooleanOption>,
    ) -> Self {
        self.add_field(name, FieldOption::Boolean(option.into()))
    }

    pub fn add_datetime_field(
        self,
        name: impl Into<String>,
        option: impl Into<DateTimeOption>,
    ) -> Self {
        self.add_field(name, FieldOption::DateTime(option.into()))
    }

    pub fn add_geo_field(self, name: impl Into<String>, option: impl Into<GeoOption>) -> Self {
        self.add_field(name, FieldOption::Geo(option.into()))
    }

    pub fn add_bytes_field(self, name: impl Into<String>, option: impl Into<BytesOption>) -> Self {
        self.add_field(name, FieldOption::Bytes(option.into()))
    }

    pub fn add_hnsw_field(self, name: impl Into<String>, option: impl Into<HnswOption>) -> Self {
        self.add_field(name, FieldOption::Hnsw(option.into()))
    }

    pub fn add_flat_field(self, name: impl Into<String>, option: impl Into<FlatOption>) -> Self {
        self.add_field(name, FieldOption::Flat(option.into()))
    }

    pub fn add_ivf_field(self, name: impl Into<String>, option: impl Into<IvfOption>) -> Self {
        self.add_field(name, FieldOption::Ivf(option.into()))
    }

    pub fn add_default_field(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.default_fields.push(name);
        self
    }

    pub fn build(self) -> Schema {
        Schema {
            fields: self.fields,
            default_fields: self.default_fields,
        }
    }
}
