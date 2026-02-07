use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lexical::TextOption;
use crate::lexical::core::field::{
    BooleanOption, DateTimeOption, FieldOption as LexicalOption, FloatOption, IntegerOption,
};
use crate::vector::HnswOption;
use crate::vector::core::field::FieldOption as VectorOption;

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
/// A field is indexed either as a vector or lexically, but not both.
/// For hybrid search, define separate fields for vector and lexical indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "options", rename_all = "snake_case")]
pub enum FieldOption {
    /// Vector index options (e.g. HNSW parameters, dimension).
    Vector(VectorOption),
    /// Lexical index options (e.g. text, integer, etc.).
    Lexical(LexicalOption),
}

impl FieldOption {
    /// Returns true if this is a vector field.
    pub fn is_vector(&self) -> bool {
        matches!(self, FieldOption::Vector(_))
    }

    /// Returns true if this is a lexical field.
    pub fn is_lexical(&self) -> bool {
        matches!(self, FieldOption::Lexical(_))
    }

    /// Returns the vector option if this is a vector field.
    pub fn as_vector(&self) -> Option<&VectorOption> {
        match self {
            FieldOption::Vector(opt) => Some(opt),
            _ => None,
        }
    }

    /// Returns the lexical option if this is a lexical field.
    pub fn as_lexical(&self) -> Option<&LexicalOption> {
        match self {
            FieldOption::Lexical(opt) => Some(opt),
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

    pub fn add_lexical_field(
        self,
        name: impl Into<String>,
        option: impl Into<LexicalOption>,
    ) -> Self {
        self.add_field(name, FieldOption::Lexical(option.into()))
    }

    pub fn add_text_field(self, name: impl Into<String>, option: impl Into<TextOption>) -> Self {
        self.add_lexical_field(name, LexicalOption::Text(option.into()))
    }

    pub fn add_integer_field(
        self,
        name: impl Into<String>,
        option: impl Into<IntegerOption>,
    ) -> Self {
        self.add_lexical_field(name, LexicalOption::Integer(option.into()))
    }

    pub fn add_float_field(self, name: impl Into<String>, option: impl Into<FloatOption>) -> Self {
        self.add_lexical_field(name, LexicalOption::Float(option.into()))
    }

    pub fn add_boolean_field(
        self,
        name: impl Into<String>,
        option: impl Into<BooleanOption>,
    ) -> Self {
        self.add_lexical_field(name, LexicalOption::Boolean(option.into()))
    }

    pub fn add_datetime_field(
        self,
        name: impl Into<String>,
        option: impl Into<DateTimeOption>,
    ) -> Self {
        self.add_lexical_field(name, LexicalOption::DateTime(option.into()))
    }

    pub fn add_geo_field(
        self,
        name: impl Into<String>,
        option: impl Into<crate::lexical::core::field::GeoOption>,
    ) -> Self {
        self.add_lexical_field(name, LexicalOption::Geo(option.into()))
    }

    pub fn add_blob_field(
        self,
        name: impl Into<String>,
        option: impl Into<crate::lexical::core::field::BlobOption>,
    ) -> Self {
        self.add_lexical_field(name, LexicalOption::Blob(option.into()))
    }

    pub fn add_vector_field(
        self,
        name: impl Into<String>,
        option: impl Into<VectorOption>,
    ) -> Self {
        self.add_field(name, FieldOption::Vector(option.into()))
    }

    pub fn add_hnsw_field(self, name: impl Into<String>, option: impl Into<HnswOption>) -> Self {
        self.add_vector_field(name, VectorOption::Hnsw(option.into()))
    }

    pub fn add_flat_field(
        self,
        name: impl Into<String>,
        option: impl Into<crate::vector::FlatOption>,
    ) -> Self {
        self.add_vector_field(name, VectorOption::Flat(option.into()))
    }

    pub fn add_ivf_field(
        self,
        name: impl Into<String>,
        option: impl Into<crate::vector::IvfOption>,
    ) -> Self {
        self.add_vector_field(name, VectorOption::Ivf(option.into()))
    }

    pub fn build(self) -> Schema {
        Schema {
            fields: self.fields,
            default_fields: self.default_fields,
        }
    }
}
