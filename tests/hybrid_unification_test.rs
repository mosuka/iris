use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iris::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use iris::embedding::per_field::PerFieldEmbedder;
use iris::error::IrisError;
use iris::lexical::core::field::{FieldOption, TextOption};
use iris::storage::memory::MemoryStorage;
use iris::vector::core::document::{DocumentPayload, Payload, StoredVector};
use iris::vector::core::field::{FlatOption, VectorOption};
use iris::vector::core::vector::Vector;
use iris::vector::engine::VectorEngine;
use iris::vector::engine::config::{VectorFieldConfig, VectorIndexConfig};
use iris::vector::engine::request::{
    FusionConfig, LexicalQuery, QueryVector, TermQueryOptions, VectorSearchRequest,
};

#[derive(Debug, Clone)]
struct MockEmbedder {
    vectors: Arc<Mutex<HashMap<String, Vec<f32>>>>,
}

impl MockEmbedder {
    fn new() -> Self {
        Self {
            vectors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn add(&self, text: &str, vector: Vec<f32>) {
        self.vectors
            .lock()
            .unwrap()
            .insert(text.to_string(), vector);
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> std::result::Result<Vector, IrisError> {
        match input {
            EmbedInput::Text(text) => {
                let map = self.vectors.lock().unwrap();
                if let Some(vec) = map.get(*text) {
                    Ok(Vector::new(vec.clone()))
                } else {
                    Err(IrisError::invalid_argument(format!(
                        "MockEmbedder: unknown text '{}'",
                        text
                    )))
                }
            }
            _ => Err(IrisError::invalid_argument(
                "MockEmbedder only supports text",
            )),
        }
    }

    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        vec![EmbedInputType::Text]
    }

    fn supports_text(&self) -> bool {
        true
    }
    fn supports_image(&self) -> bool {
        false
    }
    fn name(&self) -> &str {
        "MockEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_hybrid_engine() -> std::result::Result<VectorEngine, Box<dyn std::error::Error>> {
    // Field "title": Lexical + Vector
    let title_config = VectorFieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 3,
            ..Default::default()
        })),
        lexical: Some(FieldOption::Text(TextOption::default())),
    };

    // Embedder setup
    let embedder = Arc::new(MockEmbedder::new());
    embedder.add("apple", vec![1.0, 0.0, 0.0]);
    embedder.add("banana", vec![0.0, 1.0, 0.0]);

    // PerFieldEmbedder
    let mut per_field = PerFieldEmbedder::new(embedder.clone());
    per_field.add_embedder("title", embedder.clone());

    let config = VectorIndexConfig::builder()
        .embedder(per_field) // Pass directly
        .field("title", title_config)
        .build()?;

    let storage = Arc::new(MemoryStorage::new(Default::default()));

    // new(storage, config)
    let engine = VectorEngine::new(storage, config)?;
    Ok(engine)
}

#[test]
fn test_hybrid_search_unification() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let engine = create_hybrid_engine()?;

    // Index Doc 1: "apple"
    let mut payload1 = DocumentPayload::new();
    payload1.set_field("title", Payload::text("apple"));
    payload1.metadata.insert("_id".to_string(), "1".to_string());

    engine.index_payload_chunk(payload1)?;

    // Index Doc 2: "banana"
    let mut payload2 = DocumentPayload::new();
    payload2.set_field("title", Payload::text("banana"));
    payload2.metadata.insert("_id".to_string(), "2".to_string());

    engine.index_payload_chunk(payload2)?;

    engine.commit()?;

    // Test 1: Vector Search (query closest to apple [0.9, 0.1, 0.0])
    let query_vector = StoredVector::new(Arc::new([0.9, 0.1, 0.0]));

    let req_vector = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: query_vector.clone(),
            weight: 1.0,
            fields: None,
        }],
        limit: 10,
        ..Default::default()
    };
    let res_vector = engine.search(req_vector)?;
    assert!(
        !res_vector.hits.is_empty(),
        "Vector search should return hits"
    );
    let top_doc = res_vector.hits[0].doc_id;

    // Test 2: Lexical Search ("banana")
    let req_lexical = VectorSearchRequest {
        lexical_query: Some(LexicalQuery::Term(TermQueryOptions {
            field: "title".to_string(),
            term: "banana".to_string(),
            boost: 1.0,
        })),
        limit: 10,
        ..Default::default()
    };
    let res_lexical = engine.search(req_lexical)?;
    assert!(
        !res_lexical.hits.is_empty(),
        "Lexical search should return hits"
    );
    assert_ne!(
        res_lexical.hits[0].doc_id, top_doc,
        "Banana should be different from Apple"
    );

    // Test 3: Hybrid Search (RRF)
    let req_hybrid = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: query_vector.clone(),
            weight: 1.0,
            fields: None,
        }],
        lexical_query: Some(LexicalQuery::Term(TermQueryOptions {
            field: "title".to_string(),
            term: "banana".to_string(),
            boost: 1.0,
        })),
        fusion_config: Some(FusionConfig::Rrf { k: 60 }),
        limit: 10,
        ..Default::default()
    };
    let res_hybrid = engine.search(req_hybrid)?;
    assert!(!res_hybrid.hits.is_empty());

    assert_eq!(
        res_hybrid.hits[0].doc_id, res_lexical.hits[0].doc_id,
        "Banana should win in hybrid search"
    );

    Ok(())
}
