use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iris::Engine;
use iris::IrisError;
use iris::lexical::TermQuery;
use iris::lexical::{FieldOption as LexicalFieldOption, TextOption};
use iris::storage::memory::MemoryStorage;
use iris::vector::{FlatOption, FieldOption as VectorOption};
use iris::vector::{QueryVector, VectorSearchRequest};
use iris::{DataValue, Document};
use iris::{EmbedInput, EmbedInputType, Embedder};
use iris::{FieldOption, Schema};
use iris::{FusionAlgorithm, SearchRequestBuilder};

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
    // Embedder setup
    let embedder = Arc::new(MockEmbedder::new());
    embedder.add("apple", vec![0.9, 0.1, 0.0]);
    embedder.add("banana", vec![0.0, 1.0, 0.0]);

    // Schema with separate fields for lexical and vector
    let schema = Schema::builder()
        .add_field(
            "title",
            FieldOption::Lexical(LexicalFieldOption::Text(TextOption::default())),
        )
        .add_field(
            "title_vec",
            FieldOption::Vector(VectorOption::Flat(FlatOption {
                dimension: 3,
                ..Default::default()
            })),
        )
        .build();

    let storage = Arc::new(MemoryStorage::new(Default::default()));

    // Use simple embedder directly (PerFieldEmbedder is not supported)
    let engine = Engine::builder(storage, schema)
        .embedder(embedder)
        .build()?;
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
    // Index Doc 1: "apple" - both lexical and vector fields
    let payload1 = Document::new()
        .add_field("title", DataValue::Text("apple".into()))
        .add_field("title_vec", DataValue::Text("apple".into()))
        .add_field("_id", DataValue::Text("1".into()));

    engine.index_chunk(payload1)?;

    // Index Doc 2: "banana" - both lexical and vector fields
    let payload2 = Document::new()
        .add_field("title", DataValue::Text("banana".into()))
        .add_field("title_vec", DataValue::Text("banana".into()))
        .add_field("_id", DataValue::Text("2".into()));

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
