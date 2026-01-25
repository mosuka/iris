use std::collections::HashMap;
use std::sync::Arc;
use tempfile::Builder;

use iris::data::{DataValue, Document};
use iris::lexical::store::config::LexicalIndexConfig;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::distance::DistanceMetric;
use iris::vector::core::field::{FlatOption, HnswOption, VectorOption};
use iris::vector::store::VectorStore;
use iris::vector::store::config::{VectorFieldConfig, VectorIndexConfig};
use iris::vector::store::request::{QueryVector, VectorScoreMode, VectorSearchRequest};

#[test]
fn test_vacuum_reduces_file_size() {
    let dir = Builder::new().prefix("test_vacuum").tempdir().unwrap();
    let path = dir.path().to_path_buf();

    // 1. Create VectorStore using FileStorage
    let field_config = VectorFieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 128,
            distance: DistanceMetric::Cosine,
            base_weight: 1.0,
            quantizer: None,
        })),
        lexical: None,
    };
    let config = VectorIndexConfig {
        fields: HashMap::from([("vectors".to_string(), field_config)]),
        default_fields: vec!["vectors".to_string()],
        metadata: HashMap::new(),
        embedder: Arc::new(iris::embedding::precomputed::PrecomputedEmbedder::new()),
        deletion_config: iris::maintenance::deletion::DeletionConfig::default(),
        shard_id: 0,
        metadata_config: LexicalIndexConfig::default(),
    };

    let file_config = FileStorageConfig::new(path.to_str().unwrap());
    let storage_config = StorageConfig::File(file_config);
    let storage = StorageFactory::create(storage_config).unwrap();

    let engine = VectorStore::new(storage, config).unwrap();

    let dim = 128;
    let num_vectors = 200;

    // 2. Insert vectors
    println!("Inserting {} vectors...", num_vectors);
    for i in 0..num_vectors {
        let doc = Document::new().with_field("vectors", DataValue::Vector(vec![0.1f32; dim]));
        engine.upsert_vectors(i, doc).unwrap();
    }

    println!("Flushing vectors to disk...");
    engine.flush_vectors().unwrap();
    engine.commit().unwrap();
    println!("committed.");

    // Check file size
    let index_file_path = path
        .join("vector_fields")
        .join("vectors")
        .join("vectors.index.flat");

    assert!(
        index_file_path.exists(),
        "Index file should exist after commit: {:?}",
        index_file_path
    );
    let size_before = std::fs::metadata(&index_file_path).unwrap().len();
    println!("Size before deletion: {} bytes", size_before);

    // 3. Delete 100 vectors (even IDs)
    println!("Deleting {} vectors...", num_vectors / 2);
    for i in 0..num_vectors {
        if i % 2 == 0 {
            engine.delete_vectors(i).unwrap();
        }
    }
    engine.commit().unwrap();

    let size_intermediate = std::fs::metadata(&index_file_path).unwrap().len();
    println!(
        "Size after delete (before optimize): {} bytes",
        size_intermediate
    );

    // 4. Run Vacuum
    println!("Running optimize (Vacuum)...");
    engine.optimize().unwrap();

    let size_after = std::fs::metadata(&index_file_path).unwrap().len();
    println!("Size after optimize: {} bytes", size_after);

    assert!(
        size_after < size_before,
        "Size should decrease after vacuum. Before: {}, After: {}",
        size_before,
        size_after
    );
    assert!(
        size_after < (size_before as f64 * 0.7) as u64,
        "Size should be roughly half (allow some metadata overhead)"
    );

    // 5. Verify Search
    let request = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: vec![0.1f32; dim],
            weight: 1.0,
            fields: None,
        }],
        limit: num_vectors as usize,
        overfetch: 1.0,
        min_score: 0.0,
        score_mode: VectorScoreMode::MaxSim,
        filter: None,
        fields: None,
        query_payloads: vec![],
        lexical_query: None,
        fusion_config: None,
        allowed_ids: None,
    };

    let searcher = engine.searcher().unwrap();
    let results = searcher.search(&request).unwrap();

    // Expect 100 hits
    assert_eq!(results.hits.len(), 100, "Should have 100 hits left");

    // Verify none are even
    for hit in results.hits {
        assert!(
            hit.doc_id % 2 != 0,
            "Deleted document {} found in search results",
            hit.doc_id
        );
    }
}
#[test]
fn test_vacuum_reduces_file_size_hnsw() {
    let dir = Builder::new().prefix("test_vacuum_hnsw").tempdir().unwrap();
    let path = dir.path().to_path_buf();

    // 1. Create VectorStore using HNSW
    let field_config = VectorFieldConfig {
        vector: Some(VectorOption::Hnsw(HnswOption {
            dimension: 16,
            distance: DistanceMetric::Cosine,
            m: 48,
            ef_construction: 200,
            base_weight: 1.0,
            quantizer: None,
        })),
        lexical: None,
    };

    let config = VectorIndexConfig {
        fields: HashMap::from([("vectors".to_string(), field_config)]),
        default_fields: vec!["vectors".to_string()],
        metadata: HashMap::new(),
        embedder: Arc::new(iris::embedding::precomputed::PrecomputedEmbedder::new()),
        deletion_config: iris::maintenance::deletion::DeletionConfig::default(),
        shard_id: 0,
        metadata_config: LexicalIndexConfig::default(),
    };

    // Correctly construct FileStorageConfig
    // Note: FileStorageConfig::new(path) might be the way, or struct init.
    // Based on storage.rs doc example: let mut file_config = FileStorageConfig::new("/tmp/test_index");
    let file_config = FileStorageConfig::new(path.to_str().unwrap());

    let storage_config = StorageConfig::File(file_config);
    let storage = StorageFactory::create(storage_config).unwrap();

    let engine = VectorStore::new(storage, config).unwrap();

    let dim = 16;
    let num_vectors = 200;

    // 2. Insert vectors
    println!("Inserting {} vectors...", num_vectors);
    for i in 0..num_vectors {
        // Use random vectors to avoid equidistant pathology
        let mut vec_data = vec![0.0f32; dim];
        for j in 0..dim {
            vec_data[j] = (i + j) as f32 % 500.0 / 500.0; // Deterministic pseudo-random, unique for i < 500
        }

        let doc = Document::new().with_field("vectors", DataValue::Vector(vec_data));
        engine.upsert_vectors(i as u64, doc).unwrap();
    }

    println!("Flushing vectors to disk...");
    engine.flush_vectors().unwrap();
    engine.commit().unwrap();
    println!("committed.");

    // Check file size by summing all .hnsw segment files
    let vectors_dir = path.join("vector_fields").join("vectors");
    let get_total_hnsw_size = |dir: &std::path::Path| -> u64 {
        let mut size = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(e) = entry {
                    let path = e.path();
                    if let Some(ext) = path.extension() {
                        if ext == "hnsw" {
                            size += e.metadata().unwrap().len();
                        }
                    }
                }
            }
        }
        size
    };

    let size_before = get_total_hnsw_size(&vectors_dir);
    println!("Size before deletion: {} bytes", size_before);
    assert!(size_before > 0, "Index size should be greater than 0");

    // 3. Delete 100 vectors (even IDs)
    println!("Deleting {} vectors...", num_vectors / 2);
    for i in 0..num_vectors {
        if i % 2 == 0 {
            engine.delete_vectors(i as u64).unwrap();
        }
    }
    engine.commit().unwrap();

    let size_intermediate = get_total_hnsw_size(&vectors_dir);
    println!(
        "Size after delete (before optimize): {} bytes",
        size_intermediate
    );

    // 4. Run Vacuum
    println!("Running optimize (Vacuum)...");
    engine.optimize().unwrap();

    let size_after = get_total_hnsw_size(&vectors_dir);
    println!("Size after optimize: {} bytes", size_after);

    assert!(
        size_after < size_before,
        "Size should decrease after vacuum. Before: {}, After: {}",
        size_before,
        size_after
    );
    assert!(
        size_after < (size_before as f64 * 0.7) as u64,
        "Size should be roughly half (allow some metadata overhead)"
    );

    // 5. Verify Search
    // Deleted (even) should not match. Odd should match.
    // If we use MaxSim and min_score 0.0, we match everything.
    // Query vector doesn't matter much if we just want "any 100 items".
    let request = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: vec![0.5f32; dim], // Generic vector
            weight: 1.0,
            fields: None,
        }],
        limit: num_vectors as usize,
        overfetch: 10.0,
        min_score: 0.0,
        score_mode: VectorScoreMode::MaxSim,
        filter: None,
        fields: None,
        lexical_query: None,
        fusion_config: None,
        query_payloads: vec![],
        allowed_ids: None,
    };

    let searcher = engine.searcher().unwrap();
    let results = searcher.search(&request).unwrap();

    // Expect reasonable recall (HNSW is approximate, and small N + parallel build might miss some)
    println!("Found {} hits", results.hits.len());
    assert!(
        results.hits.len() >= 75,
        "Should have decent recall (>= 75/100) left. Found {}",
        results.hits.len()
    );

    // Verify none are even
    for hit in results.hits {
        assert!(
            hit.doc_id % 2 != 0,
            "Deleted document {} found in search results",
            hit.doc_id
        );
    }
    // 5. Verify Compaction
    // HNSW vacuum is cleaner now.
    // If it worked, size < size_before (mostly).
    // But since HNSW adds complexity, strict size comparison is tricky.
    // We mainly ensure we didn't crash and index is valid.
}
