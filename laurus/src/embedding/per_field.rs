//! Per-field embedder for applying different embedders to different fields.
//!
//! This module provides `PerFieldEmbedder`, which allows specifying different
//! embedders for different vector fields. This is analogous to `PerFieldAnalyzer`
//! in the lexical module.
//!
//! # Design Symmetry with PerFieldAnalyzer
//!
//! | PerFieldAnalyzer | PerFieldEmbedder |
//! |-----------------|------------------|
//! | `new(default_analyzer)` | `new(default_embedder)` |
//! | `add_analyzer(field, analyzer)` | `add_embedder(field, embedder)` |
//! | `get_analyzer(field)` | `get_embedder(field)` |
//! | `default_analyzer()` | `default_embedder()` |
//! | `analyze_field(field, text)` | `embed_field(field, input)` |
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "embeddings-candle")]
//! # {
//! use laurus::embedding::per_field::PerFieldEmbedder;
//! use laurus::embedding::embedder::{Embedder, EmbedInput};
//! use laurus::embedding::candle_bert_embedder::CandleBertEmbedder;
//! use std::sync::Arc;
//!
//! # async fn example() -> laurus::Result<()> {
//! // Create default embedder
//! let default_embedder: Arc<dyn Embedder> = Arc::new(
//!     CandleBertEmbedder::new("sentence-transformers/all-MiniLM-L6-v2")?
//! );
//!
//! // Create per-field embedder with default
//! let per_field = PerFieldEmbedder::new(default_embedder);
//!
//! // Add specialized embedder for title field
//! let title_embedder: Arc<dyn Embedder> = Arc::new(
//!     CandleBertEmbedder::new("sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")?
//! );
//! per_field.add_embedder("title_embedding", Arc::clone(&title_embedder));
//!
//! // "content_embedding" will use default_embedder
//! // "title_embedding" will use the specialized title_embedder
//!
//! // Embed with field context
//! let input = EmbedInput::Text("Hello, world!");
//! let vector = per_field.embed_field("title_embedding", &input).await?;
//! # Ok(())
//! # }
//! # }
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use crate::error::Result;
use crate::vector::core::vector::Vector;

/// A per-field embedder that applies different embedders to different fields.
///
/// This is similar to `PerFieldAnalyzer` in the lexical module. It allows you
/// to specify a different embedder for each field, with a default embedder
/// for fields not explicitly configured.
///
/// Field-specific embedders can be added at any time via [`add_embedder`](Self::add_embedder),
/// even after the embedder has been wrapped in an `Arc`. This enables dynamic
/// field addition at runtime.
///
/// # Memory Efficiency
///
/// When using the same embedder for multiple fields, reuse a single instance
/// with `Arc::clone` to save memory. This is especially important for embedders
/// with large models.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "embeddings-candle")]
/// # {
/// use laurus::embedding::embedder::Embedder;
/// use laurus::embedding::per_field::PerFieldEmbedder;
/// use laurus::embedding::candle_bert_embedder::CandleBertEmbedder;
/// use std::sync::Arc;
///
/// # fn example() -> laurus::Result<()> {
/// // Create default embedder
/// let default_embedder: Arc<dyn Embedder> = Arc::new(
///     CandleBertEmbedder::new("sentence-transformers/all-MiniLM-L6-v2")?
/// );
///
/// // Create per-field embedder
/// let per_field = PerFieldEmbedder::new(default_embedder);
///
/// // Reuse embedder instances to save memory
/// let keyword_embedder: Arc<dyn Embedder> = Arc::new(
///     CandleBertEmbedder::new("sentence-transformers/all-MiniLM-L6-v2")?
/// );
/// per_field.add_embedder("id", Arc::clone(&keyword_embedder));
/// per_field.add_embedder("category", Arc::clone(&keyword_embedder));
/// # Ok(())
/// # }
/// # }
/// ```
pub struct PerFieldEmbedder {
    /// Default embedder for fields not in the map.
    default_embedder: Arc<dyn Embedder>,

    /// Map of field names to their specific embedders.
    /// Wrapped in `RwLock` to allow adding embedders at runtime via `&self`.
    field_embedders: RwLock<HashMap<String, Arc<dyn Embedder>>>,
}

impl Clone for PerFieldEmbedder {
    fn clone(&self) -> Self {
        Self {
            default_embedder: self.default_embedder.clone(),
            field_embedders: RwLock::new(self.field_embedders.read().clone()),
        }
    }
}

impl std::fmt::Debug for PerFieldEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerFieldEmbedder")
            .field("default_embedder", &self.default_embedder.name())
            .field(
                "fields",
                &self.field_embedders.read().keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl PerFieldEmbedder {
    /// Create a new per-field embedder with a default embedder.
    ///
    /// # Arguments
    ///
    /// * `default_embedder` - The embedder to use for fields not explicitly configured
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "embeddings-candle")]
    /// # {
    /// use laurus::embedding::per_field::PerFieldEmbedder;
    /// use laurus::embedding::embedder::Embedder;
    /// use laurus::embedding::candle_bert_embedder::CandleBertEmbedder;
    /// use std::sync::Arc;
    ///
    /// # fn example() -> laurus::Result<()> {
    /// let default: Arc<dyn Embedder> = Arc::new(
    ///     CandleBertEmbedder::new("sentence-transformers/all-MiniLM-L6-v2")?
    /// );
    /// let per_field = PerFieldEmbedder::new(default);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new(default_embedder: Arc<dyn Embedder>) -> Self {
        Self {
            default_embedder,
            field_embedders: RwLock::new(HashMap::new()),
        }
    }

    /// Add a field-specific embedder.
    ///
    /// This method takes `&self` (not `&mut self`) and uses interior mutability,
    /// so it can be called even after the embedder has been wrapped in an `Arc`.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `embedder` - The embedder to use for this field
    pub fn add_embedder(&self, field: impl Into<String>, embedder: Arc<dyn Embedder>) {
        self.field_embedders.write().insert(field.into(), embedder);
    }

    /// Remove the field-specific embedder for the given field.
    ///
    /// After removal, the field will fall back to the default embedder.
    /// This method is a no-op if the field has no specific embedder configured.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name whose embedder should be removed
    pub fn remove_embedder(&self, field: &str) {
        self.field_embedders.write().remove(field);
    }

    /// Get the embedder for a specific field.
    ///
    /// Returns the field-specific embedder if configured, otherwise returns the default.
    /// The returned `Arc` is cloned from under the internal read lock.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    pub fn get_embedder(&self, field: &str) -> Arc<dyn Embedder> {
        let guard = self.field_embedders.read();
        guard
            .get(field)
            .cloned()
            .unwrap_or_else(|| self.default_embedder.clone())
    }

    /// Get the default embedder.
    pub fn default_embedder(&self) -> &Arc<dyn Embedder> {
        &self.default_embedder
    }

    /// Embed with the embedder for the given field.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to determine which embedder to use
    /// * `input` - The input to embed
    ///
    /// # Returns
    ///
    /// The embedding vector for the input.
    pub async fn embed_field(&self, field: &str, input: &EmbedInput<'_>) -> Result<Vector> {
        self.get_embedder(field).embed(input).await
    }

    /// List all configured field names.
    pub fn configured_fields(&self) -> Vec<String> {
        self.field_embedders.read().keys().cloned().collect()
    }

    /// Check if a specific field supports the given input type.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name
    /// * `input_type` - The input type to check
    pub fn field_supports(&self, field: &str, input_type: EmbedInputType) -> bool {
        self.get_embedder(field).supports(input_type)
    }
}

#[async_trait]
impl Embedder for PerFieldEmbedder {
    /// Embed with the default embedder.
    ///
    /// Note: When using PerFieldEmbedder, it's recommended to use `embed_field()`
    /// to explicitly specify which field's embedder to use.
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        self.default_embedder.embed(input).await
    }

    async fn embed_batch(&self, inputs: &[EmbedInput<'_>]) -> Result<Vec<Vector>> {
        self.default_embedder.embed_batch(inputs).await
    }

    /// Returns the union of supported input types across the default
    /// embedder and all field-specific embedders.
    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        use std::collections::HashSet;
        let mut types: HashSet<EmbedInputType> = self
            .default_embedder
            .supported_input_types()
            .into_iter()
            .collect();
        let guard = self.field_embedders.read();
        for emb in guard.values() {
            for t in emb.supported_input_types() {
                types.insert(t);
            }
        }
        types.into_iter().collect()
    }

    fn name(&self) -> &str {
        "PerFieldEmbedder"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LaurusError;

    #[derive(Debug)]
    struct MockEmbedder {
        name: String,
        dim: usize,
    }

    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
            match input {
                EmbedInput::Text(_) => Ok(Vector::new(vec![0.0; self.dim])),
                _ => Err(LaurusError::invalid_argument("only text supported")),
            }
        }

        fn supported_input_types(&self) -> Vec<EmbedInputType> {
            vec![EmbedInputType::Text]
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_per_field_embedder() {
        let default: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "default".into(),
            dim: 384,
        });
        let per_field = PerFieldEmbedder::new(default);

        let title_embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "title".into(),
            dim: 768,
        });
        per_field.add_embedder("title", Arc::clone(&title_embedder));
        per_field.add_embedder("description", title_embedder);

        let input = EmbedInput::Text("hello");
        let title_vec = per_field.embed_field("title", &input).await.unwrap();
        assert_eq!(title_vec.dimension(), 768);

        let desc_vec = per_field.embed_field("description", &input).await.unwrap();
        assert_eq!(desc_vec.dimension(), 768);

        let content_vec = per_field.embed_field("content", &input).await.unwrap();
        assert_eq!(content_vec.dimension(), 384);
    }

    #[tokio::test]
    async fn test_default_embedder_when_field_not_configured() {
        let default: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "default".into(),
            dim: 384,
        });
        let per_field = PerFieldEmbedder::new(default);

        let input = EmbedInput::Text("hello");
        let vec = per_field
            .embed_field("unknown_field", &input)
            .await
            .unwrap();
        assert_eq!(vec.dimension(), 384);
        assert_eq!(per_field.get_embedder("unknown_field").name(), "default");
    }

    #[tokio::test]
    async fn test_as_embedder_trait() {
        let default: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "default".into(),
            dim: 384,
        });
        let per_field = PerFieldEmbedder::new(default);

        let embedder: &dyn Embedder = &per_field;
        assert!(embedder.supports_text());

        let vec = embedder.embed(&EmbedInput::Text("hello")).await.unwrap();
        assert_eq!(vec.dimension(), 384);
    }

    #[tokio::test]
    async fn test_embed_field() {
        let default: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "default".into(),
            dim: 384,
        });
        let per_field = PerFieldEmbedder::new(default);

        let title_embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "title".into(),
            dim: 768,
        });
        per_field.add_embedder("title", title_embedder);

        // Embed with specific field
        let input = EmbedInput::Text("hello");
        let vec = per_field.embed_field("title", &input).await.unwrap();
        assert_eq!(vec.dimension(), 768);

        // Embed with default field
        let vec = per_field.embed_field("unknown", &input).await.unwrap();
        assert_eq!(vec.dimension(), 384);
    }

    #[test]
    fn test_configured_fields() {
        let default: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "default".into(),
            dim: 384,
        });
        let per_field = PerFieldEmbedder::new(default);

        let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "special".into(),
            dim: 512,
        });
        per_field.add_embedder("title", Arc::clone(&embedder));
        per_field.add_embedder("body", embedder);

        let fields = per_field.configured_fields();
        assert!(fields.contains(&"title".to_string()));
        assert!(fields.contains(&"body".to_string()));
        assert!(!fields.contains(&"unknown".to_string()));
    }

    #[test]
    fn test_field_supports() {
        let default: Arc<dyn Embedder> = Arc::new(MockEmbedder {
            name: "default".into(),
            dim: 384,
        });
        let per_field = PerFieldEmbedder::new(default);

        assert!(per_field.field_supports("any", EmbedInputType::Text));
        assert!(!per_field.field_supports("any", EmbedInputType::Image));
    }
}
