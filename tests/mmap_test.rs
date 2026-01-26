use iris::embedding::precomputed::PrecomputedEmbedder;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};

use iris::data::{DataValue, Document};
use iris::vector::core::distance::DistanceMetric;
use iris::vector::core::field::{FlatOption, VectorOption};
use iris::vector::store::VectorStore;
use iris::vector::store::config::{VectorFieldConfig, VectorIndexConfig};
use iris::vector::store::query::VectorSearchRequestBuilder;

use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_mmap_mode_basic_search() {
    let dir = tempdir().unwrap();
    let storage_path = dir.path().to_owned();

    let storage_config = StorageConfig::File(FileStorageConfig::new(storage_path.clone()));
    let storage = StorageFactory::create(storage_config).unwrap();

    // Configure a fields with Mmap loading
    let field_config = VectorFieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 3,
            distance: DistanceMetric::Cosine,
            base_weight: 1.0,
            quantizer: None,
        })),
        lexical: None,
    };

    let config = VectorIndexConfig::builder()
        .embedder(Arc::new(PrecomputedEmbedder::new()))
        .field("mmap_field", field_config)
        .build()
        .unwrap();

    let engine = VectorStore::new(storage, config).unwrap();

    // Add vectors
    let vectors = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];

    for vec_data in vectors {
        let doc = Document::new().add_field("mmap_field", DataValue::Vector(vec_data));
        engine.add_payloads(doc).unwrap();
    }
    engine.commit().unwrap();

    let query_vector = vec![1.0, 0.1, 0.0];
    let request = VectorSearchRequestBuilder::new()
        .add_vector("mmap_field", query_vector)
        .limit(2)
        .build();

    let results = engine.search(request).unwrap();

    assert_eq!(results.hits.len(), 2);
}

#[test]
fn test_mmap_mode_persistence_reload() {
    let dir = tempdir().unwrap();
    let storage_path = dir.path().to_owned();

    {
        let storage_config = StorageConfig::File(FileStorageConfig::new(storage_path.clone()));
        let storage = StorageFactory::create(storage_config).unwrap();

        let field_config = VectorFieldConfig {
            vector: Some(VectorOption::Flat(FlatOption {
                dimension: 3,
                distance: DistanceMetric::Cosine,
                base_weight: 1.0,
                quantizer: None,
            })),
            lexical: None,
        };

        let config = VectorIndexConfig::builder()
            .embedder(Arc::new(PrecomputedEmbedder::new()))
            .field("mmap_field", field_config)
            .build()
            .unwrap();

        let engine = VectorStore::new(storage, config).unwrap();

        let vectors = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];

        for vec_data in vectors {
            let doc = Document::new().add_field("mmap_field", DataValue::Vector(vec_data));
            engine.add_payloads(doc).unwrap();
        }
        engine.commit().unwrap();
    }

    // Re-open
    {
        let storage_config = StorageConfig::File(FileStorageConfig::new(storage_path.clone()));
        let storage = StorageFactory::create(storage_config).unwrap();

        let field_config = VectorFieldConfig {
            vector: Some(VectorOption::Flat(FlatOption {
                dimension: 3,
                distance: DistanceMetric::Cosine,
                base_weight: 1.0,
                quantizer: None,
            })),
            lexical: None,
        };

        let config = VectorIndexConfig::builder()
            .embedder(Arc::new(PrecomputedEmbedder::new()))
            .field("mmap_field", field_config)
            .build()
            .unwrap();

        let engine = VectorStore::new(storage, config).unwrap();

        // IMPORTANT: In Mmap mode, vectors are LOADED from file on demand.
        // If file persistence works, search should find them.

        let query_vector = vec![0.0, 1.0, 0.0];
        let request = VectorSearchRequestBuilder::new()
            .add_vector("mmap_field", query_vector)
            .limit(1)
            .build();

        let results = engine.search(request).unwrap();

        assert_eq!(results.hits.len(), 1);
        // We expect it to match the second vector.
    }
}
