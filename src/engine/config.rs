use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::analysis::analyzer::analyzer::Analyzer;
use crate::lexical::core::field::FieldOption as LexicalOption;
use crate::vector::core::field::VectorOption;

use crate::embedding::embedder::Embedder;

/// Configuration for the unified engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Configuration for each field.
    pub fields: HashMap<String, FieldConfig>,
    /// Default fields for search.
    #[serde(default)]
    pub default_fields: Vec<String>,
    /// Global analyzer (fallback if not specified per field).
    #[serde(skip)]
    pub analyzer: Option<Arc<dyn Analyzer>>,
    /// Global embedder (fallback if not specified per field).
    #[serde(skip)]
    pub embedder: Option<Arc<dyn Embedder>>,
}

impl IndexConfig {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            default_fields: Vec::new(),
            analyzer: None,
            embedder: None,
        }
    }

    pub fn builder() -> IndexConfigBuilder {
        IndexConfigBuilder::default()
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for a single field in the unified schema.
///
/// A field can be indexed as a vector, lexically, or both.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldConfig {
    /// Vector index options (e.g. HNSW parameters, dimension).
    /// If None, this field is not indexed as a vector.
    pub vector: Option<VectorOption>,

    /// Lexical index options (e.g. detailed analyzer).
    /// If None, this field is not indexed lexically.
    pub lexical: Option<LexicalOption>,
}

#[derive(Default)]
pub struct IndexConfigBuilder {
    fields: HashMap<String, FieldConfig>,
    default_fields: Vec<String>,
    embedder: Option<Arc<dyn Embedder>>,
    analyzer: Option<Arc<dyn Analyzer>>,
}

impl IndexConfigBuilder {
    pub fn add_field(mut self, name: impl Into<String>, config: FieldConfig) -> Self {
        let name = name.into();
        self.fields.insert(name, config);
        self
    }

    pub fn add_vector_field(
        mut self,
        name: impl Into<String>,
        option: impl Into<VectorOption>,
    ) -> Self {
        let name = name.into();
        let config = FieldConfig {
            vector: Some(option.into()),
            lexical: None,
        };
        self.fields.insert(name, config);
        self
    }

    pub fn add_lexical_field(
        mut self,
        name: impl Into<String>,
        option: impl Into<LexicalOption>,
    ) -> Self {
        let name = name.into();
        let config = FieldConfig {
            vector: None,
            lexical: Some(option.into()),
        };
        self.fields.insert(name, config);
        self
    }

    pub fn add_hybrid_field(
        mut self,
        name: impl Into<String>,
        vector_option: impl Into<VectorOption>,
        lexical_option: impl Into<LexicalOption>,
    ) -> Self {
        let name = name.into();
        let config = FieldConfig {
            vector: Some(vector_option.into()),
            lexical: Some(lexical_option.into()),
        };
        self.fields.insert(name, config);
        self
    }

    pub fn embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    pub fn analyzer(mut self, analyzer: Arc<dyn Analyzer>) -> Self {
        self.analyzer = Some(analyzer);
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            fields: self.fields,
            default_fields: self.default_fields,
            analyzer: self.analyzer,
            embedder: self.embedder,
        }
    }
}
