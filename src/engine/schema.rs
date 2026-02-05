use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lexical::core::field::FieldOption as LexicalOption;
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

    pub fn add_vector_field(
        mut self,
        name: impl Into<String>,
        option: impl Into<VectorOption>,
    ) -> Self {
        let name = name.into();
        self.fields.insert(name, FieldOption::Vector(option.into()));
        self
    }

    pub fn add_lexical_field(
        mut self,
        name: impl Into<String>,
        option: impl Into<LexicalOption>,
    ) -> Self {
        let name = name.into();
        self.fields
            .insert(name, FieldOption::Lexical(option.into()));
        self
    }

    pub fn build(self) -> Schema {
        Schema {
            fields: self.fields,
            default_fields: self.default_fields,
        }
    }
}
