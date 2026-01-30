use async_trait::async_trait;
use iris::lexical::LexicalIndexConfig;
use iris::storage::memory::{MemoryStorage, MemoryStorageConfig};
use iris::vector::DistanceMetric;
use iris::vector::Vector;
use iris::vector::{HnswOption, VectorOption};
use iris::vector::{VectorFieldConfig, VectorIndexConfig};
use iris::{DataValue, Document};
use iris::{EmbedInput, EmbedInputType, Embedder};
use iris::{IrisError, Result};
use std::any::Any;
use std::sync::Arc;

#[derive(Debug)]
struct MockTextEmbedder {
    dimension: usize,
}

#[async_trait]
impl Embedder for MockTextEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(_) => Ok(Vector::new(vec![0.0; self.dimension])),
            _ => Err(IrisError::invalid_argument(
                "this embedder only supports text input",
            )),
        }
    }

    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        vec![EmbedInputType::Text]
    }

    fn name(&self) -> &str {
        "mock-text"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[tokio::test]
async fn test_vector_segment_integration() {
    // 1. Setup storage and config
    let storage_config = MemoryStorageConfig::default();
    let storage = Arc::new(MemoryStorage::new(storage_config));

    let mut field_configs = std::collections::HashMap::new();
    field_configs.insert(
        "vector_field".to_string(),
        VectorFieldConfig {
            vector: Some(VectorOption::Hnsw(HnswOption {
                dimension: 4,
                distance: DistanceMetric::Euclidean,
                m: 16,
                ef_construction: 200,
                base_weight: 1.0,
                quantizer: None,
            })),
            lexical: None,
        },
    );

    let collection_config = VectorIndexConfig {
        fields: field_configs.clone(),
        embedder: Arc::new(MockTextEmbedder { dimension: 4 }),
        default_fields: vec!["vector_field".to_string()],
        metadata: std::collections::HashMap::new(),
        deletion_config: iris::DeletionConfig::default(),
        shard_id: 0,
        metadata_config: LexicalIndexConfig::default(),
    };

    // We construct engine manually to inject storage
    let engine =
        iris::vector::VectorStore::new(storage.clone(), collection_config.clone()).unwrap();

    // 2. Insert vectors
    let vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];

    for (i, vec_data) in vectors.iter().enumerate() {
        let doc_id = (i as u64) + 1;
        let doc = Document::new().add_field("vector_field", DataValue::Vector(vec_data.clone()));

        // Use upsert_payloads.
        engine.upsert_payloads(doc_id, doc).unwrap();
    }

    // 3. Flush/Persist explicitly
    engine.commit().unwrap();

    // 4. Persistence check
    // We drop engine and recreates it.
    drop(engine);

    let engine_2 =
        iris::vector::VectorStore::new(storage.clone(), collection_config.clone()).unwrap();

    // We verify stats.
    // Recovery should load segments.
    // SegmentedVectorField::stats() sums active (new) + managed (sealed).
    // Sealed should be 3 (one per upsert), or less if mocked?
    // Assuming upsert flushes each time.

    let stats = engine_2.field_stats("vector_field").unwrap();

    // We use assert!(stats.vector_count > 0) to be safe against flush optimizations.
    // But given implementation, it should be 3.
    println!("Stats vector count: {}", stats.vector_count);
    assert_eq!(stats.vector_count, 3);
}
