//! Configuration types for embedder definitions within a schema.
//!
//! These types allow users to declaratively define embedding models
//! in the schema's `embedders` section. Each definition is referenced
//! by name from vector field options (e.g. `HnswOption::embedder`).
//!
//! # JSON Format
//!
//! ```json
//! {
//!   "type": "candle_bert",
//!   "model": "sentence-transformers/all-MiniLM-L6-v2"
//! }
//! ```

use serde::{Deserialize, Serialize};

/// A declarative embedder definition stored in the schema.
///
/// Each variant maps to a concrete [`Embedder`](crate::embedding::embedder::Embedder)
/// implementation. The `type` tag selects the variant; additional fields
/// provide type-specific configuration.
///
/// # API Key Handling
///
/// For embedders that require API keys (e.g. OpenAI), the key is read
/// from an environment variable at engine initialization time, **not**
/// stored in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbedderDefinition {
    /// Pre-computed vectors — no embedding is performed.
    /// Use this when vectors are computed externally and passed directly.
    Precomputed,

    /// Candle-based BERT embedder for text embedding.
    /// Requires the `embeddings-candle` feature.
    CandleBert {
        /// HuggingFace model ID
        /// (e.g. `"sentence-transformers/all-MiniLM-L6-v2"`).
        model: String,
    },

    /// Candle-based CLIP multimodal embedder for text and image embedding.
    /// Requires the `embeddings-multimodal` feature.
    CandleClip {
        /// HuggingFace model ID
        /// (e.g. `"openai/clip-vit-base-patch32"`).
        model: String,
    },

    /// OpenAI API embedder for text embedding.
    /// Requires the `embeddings-openai` feature.
    /// The API key is read from the `OPENAI_API_KEY` environment variable.
    Openai {
        /// OpenAI model name (e.g. `"text-embedding-3-small"`).
        model: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precomputed_serde_roundtrip() {
        let json = r#"{"type": "precomputed"}"#;
        let def: EmbedderDefinition = serde_json::from_str(json).unwrap();
        assert!(matches!(def, EmbedderDefinition::Precomputed));
        let serialized = serde_json::to_string(&def).unwrap();
        let _roundtrip: EmbedderDefinition = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn test_candle_bert_serde_roundtrip() {
        let json = r#"{"type": "candle_bert", "model": "sentence-transformers/all-MiniLM-L6-v2"}"#;
        let def: EmbedderDefinition = serde_json::from_str(json).unwrap();
        if let EmbedderDefinition::CandleBert { model } = &def {
            assert_eq!(model, "sentence-transformers/all-MiniLM-L6-v2");
        } else {
            panic!("Expected CandleBert");
        }
        let serialized = serde_json::to_string(&def).unwrap();
        let _roundtrip: EmbedderDefinition = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn test_candle_clip_serde_roundtrip() {
        let json = r#"{"type": "candle_clip", "model": "openai/clip-vit-base-patch32"}"#;
        let def: EmbedderDefinition = serde_json::from_str(json).unwrap();
        assert!(matches!(def, EmbedderDefinition::CandleClip { .. }));
    }

    #[test]
    fn test_openai_serde_roundtrip() {
        let json = r#"{"type": "openai", "model": "text-embedding-3-small"}"#;
        let def: EmbedderDefinition = serde_json::from_str(json).unwrap();
        if let EmbedderDefinition::Openai { model } = &def {
            assert_eq!(model, "text-embedding-3-small");
        } else {
            panic!("Expected Openai");
        }
    }
}
