pub mod analyzer;
pub mod embedder;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use self::analyzer::AnalyzerDefinition;
use self::embedder::EmbedderDefinition;

use crate::lexical::core::field::{
    BooleanOption, BytesOption, DateTimeOption, FloatOption, GeoOption, IntegerOption, TextOption,
};
use crate::vector::core::field::{FlatOption, HnswOption, IvfOption};

/// Schema for the unified engine.
///
/// Declares what fields exist, their index types (lexical or vector),
/// and optional custom analyzer definitions. Custom analyzers are
/// referenced by name from [`TextOption::analyzer`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Custom analyzer definitions, keyed by name.
    /// These can be referenced from text field `analyzer` settings.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub analyzers: HashMap<String, AnalyzerDefinition>,
    /// Embedder definitions, keyed by name.
    /// These can be referenced from vector field `embedder` settings.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub embedders: HashMap<String, EmbedderDefinition>,
    /// Options for each field.
    pub fields: HashMap<String, FieldOption>,
    /// Default fields for search.
    #[serde(default)]
    pub default_fields: Vec<String>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            analyzers: HashMap::new(),
            embedders: HashMap::new(),
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

    /// Returns the embedder name if this is a vector field with an embedder configured.
    pub fn embedder_name(&self) -> Option<&str> {
        match self {
            Self::Hnsw(o) => o.embedder.as_deref(),
            Self::Flat(o) => o.embedder.as_deref(),
            Self::Ivf(o) => o.embedder.as_deref(),
            _ => None,
        }
    }

    /// Converts to the lexical-subsystem's `FieldOption` if this is a lexical field.
    pub fn to_lexical(&self) -> Option<crate::lexical::core::field::FieldOption> {
        match self {
            Self::Text(o) => Some(crate::lexical::core::field::FieldOption::Text(o.clone())),
            Self::Integer(o) => Some(crate::lexical::core::field::FieldOption::Integer(o.clone())),
            Self::Float(o) => Some(crate::lexical::core::field::FieldOption::Float(o.clone())),
            Self::Boolean(o) => Some(crate::lexical::core::field::FieldOption::Boolean(o.clone())),
            Self::DateTime(o) => Some(crate::lexical::core::field::FieldOption::DateTime(
                o.clone(),
            )),
            Self::Geo(o) => Some(crate::lexical::core::field::FieldOption::Geo(o.clone())),
            Self::Bytes(o) => Some(crate::lexical::core::field::FieldOption::Bytes(o.clone())),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct SchemaBuilder {
    analyzers: HashMap<String, AnalyzerDefinition>,
    embedders: HashMap<String, EmbedderDefinition>,
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

    /// Add a custom analyzer definition to the schema.
    ///
    /// # Arguments
    ///
    /// * `name` - The analyzer name (referenced from `TextOption::analyzer`).
    /// * `definition` - The analyzer definition.
    pub fn add_analyzer(mut self, name: impl Into<String>, definition: AnalyzerDefinition) -> Self {
        self.analyzers.insert(name.into(), definition);
        self
    }

    /// Add an embedder definition to the schema.
    ///
    /// # Arguments
    ///
    /// * `name` - The embedder name (referenced from vector field `embedder`).
    /// * `definition` - The embedder definition.
    pub fn add_embedder(mut self, name: impl Into<String>, definition: EmbedderDefinition) -> Self {
        self.embedders.insert(name.into(), definition);
        self
    }

    pub fn build(self) -> Schema {
        Schema {
            analyzers: self.analyzers,
            embedders: self.embedders,
            fields: self.fields,
            default_fields: self.default_fields,
        }
    }
}
