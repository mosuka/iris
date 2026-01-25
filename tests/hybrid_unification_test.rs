use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iris::data::{DataValue, Document};
use iris::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use iris::embedding::per_field::PerFieldEmbedder;
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::engine::search::{FusionAlgorithm, SearchRequestBuilder};
use iris::error::IrisError;
use iris::lexical::core::field::{FieldOption, TextOption};
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::storage::memory::MemoryStorage;
use iris::vector::core::field::{FlatOption, VectorOption};
use iris::vector::store::request::{QueryVector, VectorSearchRequest};

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
    async fn embed(
        &self,
        input: &EmbedInput<'_>,
    ) -> std::result::Result<iris::vector::core::vector::Vector, IrisError> {
        match input {
            EmbedInput::Text(text) => {
                let map = self.vectors.lock().unwrap();
                if let Some(vec) = map.get(*text) {
                    Ok(iris::vector::core::vector::Vector::new(vec.clone()))
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

fn create_hybrid_engine() -> std::result::Result<Engine, Box<dyn std::error::Error>> {
    // Field "title": Lexical + Vector
    let title_config = FieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 3,
            ..Default::default()
        })),
        lexical: Some(FieldOption::Text(TextOption::default())),
    };

    // Embedder setup
    let embedder = Arc::new(MockEmbedder::new());
    embedder.add("apple", vec![0.9, 0.1, 0.0]);
    embedder.add("banana", vec![0.0, 1.0, 0.0]);

    // PerFieldEmbedder
    let mut per_field = PerFieldEmbedder::new(embedder.clone());
    per_field.add_embedder("title", embedder.clone());

    let config = IndexConfig::builder()
        .embedder(Arc::new(per_field))
        .add_field("title", title_config)
        .build();

    let storage = Arc::new(MemoryStorage::new(Default::default()));

    let engine = Engine::new(storage, config)?;
    Ok(engine)
}

#[test]
fn test_hybrid_search_unification() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    let engine = create_hybrid_engine()?;

    rt.block_on(async { test_hybrid_search_unification_impl(&engine).await })?;

    drop(engine);
    Ok(())
}

async fn test_hybrid_search_unification_impl(
    engine: &Engine,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Index Doc 1: "apple"
    let payload1 = Document::new()
        .with_field("title", DataValue::Text("apple".into()))
        .with_field("_id", DataValue::Text("1".into()));

    engine.index_chunk(payload1)?;

    // Index Doc 2: "banana"
    let payload2 = Document::new()
        .with_field("title", DataValue::Text("banana".into()))
        .with_field("_id", DataValue::Text("2".into()));

    engine.index_chunk(payload2)?;

    engine.commit()?;

    // Test 1: Vector Search (query closest to apple [0.9, 0.1, 0.0])
    let req_vector = SearchRequestBuilder::new()
        .with_vector(VectorSearchRequest {
            query_vectors: vec![QueryVector {
                vector: vec![0.9, 0.1, 0.0],
                weight: 1.0,
                fields: None,
            }],
            limit: 10,
            ..Default::default()
        })
        .limit(10)
        .build();

    let res_vector = engine.search(req_vector)?;
    assert!(!res_vector.is_empty(), "Vector search should return hits");
    let top_doc = res_vector[0].doc_id;

    // Test 2: Lexical Search ("banana")
    let req_lexical = SearchRequestBuilder::new()
        .with_lexical(Box::new(TermQuery::new("title", "banana")))
        .limit(10)
        .build();

    let res_lexical = engine.search(req_lexical)?;
    assert!(!res_lexical.is_empty(), "Lexical search should return hits");
    assert_ne!(
        res_lexical[0].doc_id, top_doc,
        "Banana should be different from Apple"
    );

    // Test 3: Hybrid Search (RRF)
    let req_hybrid = SearchRequestBuilder::new()
        .with_vector(VectorSearchRequest {
            query_vectors: vec![QueryVector {
                vector: vec![0.0, 1.0, 0.0],
                weight: 1.0,
                fields: None,
            }],
            limit: 10,
            ..Default::default()
        })
        .with_lexical(Box::new(TermQuery::new("title", "banana")))
        .fusion(FusionAlgorithm::RRF { k: 60.0 })
        .limit(10)
        .build();

    let res_hybrid = engine.search(req_hybrid)?;
    assert!(!res_hybrid.is_empty());

    assert_eq!(
        res_hybrid[0].doc_id, res_lexical[0].doc_id,
        "Banana should win in hybrid search"
    );

    Ok(())
}
