use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sarissa::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use sarissa::embedding::per_field::PerFieldEmbedder;
use sarissa::error::SarissaError;
use sarissa::lexical::core::field::{FieldOption, TextOption};
use sarissa::storage::memory::MemoryStorage;
use sarissa::vector::core::document::{DocumentPayload, Payload, StoredVector};
use sarissa::vector::core::vector::Vector;
use sarissa::vector::engine::VectorEngine;
use sarissa::vector::engine::config::{
    FlatOption, VectorFieldConfig, VectorOption, VectorIndexConfig,
};
use sarissa::vector::engine::request::{
    FusionConfig, LexicalQuery, QueryVector, TermQueryOptions, VectorSearchRequest,
};

#[derive(Debug, Clone)]
struct MockEmbedder {
    vectors: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    dimension: usize,
}

impl MockEmbedder {
    fn new(dimension: usize) -> Self {
        Self {
            vectors: Arc::new(Mutex::new(HashMap::new())),
            dimension,
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
    async fn embed(&self, input: &EmbedInput<'_>) -> std::result::Result<Vector, SarissaError> {
        match input {
            EmbedInput::Text(text) => {
                let map = self.vectors.lock().unwrap();
                if let Some(vec) = map.get(*text) {
                    Ok(Vector::new(vec.clone()))
                } else {
                    Err(SarissaError::invalid_argument(format!(
                        "MockEmbedder: unknown text '{}'",
                        text
                    )))
                }
            }
            _ => Err(SarissaError::invalid_argument(
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
    let embedder = Arc::new(MockEmbedder::new(3));
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
