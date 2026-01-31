use std::collections::HashMap;
use std::sync::Arc;
use tempfile::Builder;

use crate::data::{DataValue, Document};
use crate::lexical::LexicalIndexConfig;
use crate::storage::file::FileStorageConfig;
use crate::storage::prefixed::PrefixedStorage;
use crate::storage::{StorageConfig, StorageFactory};
use crate::store::document::UnifiedDocumentStore;
use crate::vector::DistanceMetric;
use crate::vector::store::VectorStore;
use crate::vector::{FlatOption, HnswOption, VectorOption};
use crate::vector::{QueryVector, VectorScoreMode, VectorSearchRequest};
use crate::vector::{VectorFieldConfig, VectorIndexConfig};
use parking_lot::RwLock;

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
        embedder: Arc::new(crate::embedding::precomputed::PrecomputedEmbedder::new()),
        deletion_config: crate::maintenance::deletion::DeletionConfig::default(),
        shard_id: 0,
        metadata_config: LexicalIndexConfig::default(),
    };

    let file_config = FileStorageConfig::new(path.to_str().unwrap());
    let storage_config = StorageConfig::File(file_config);
    let storage = StorageFactory::create(storage_config).unwrap();
    let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    let doc_store = Arc::new(RwLock::new(
        UnifiedDocumentStore::open(doc_storage).unwrap(),
    ));

    let engine = VectorStore::new(storage, config, doc_store.clone()).unwrap();

    let dim = 128;
    let num_vectors = 200;

    // 2. Insert vectors
    println!("Inserting {} vectors...", num_vectors);
    let mut batch_docs = HashMap::new();
    for i in 0..num_vectors {
        let doc = Document::new().add_field("vectors", DataValue::Vector(vec![0.1f32; dim]));
        batch_docs.insert(i as u64, doc.clone());
        engine
            .upsert_document_by_internal_id(i as u64, doc)
            .unwrap();
    }
    doc_store.write().add_segment(&batch_docs).unwrap();

    println!("Flushing vectors to disk...");
    engine.flush().unwrap();
    engine.commit().unwrap();
    println!("committed.");

    // Check file size
    let index_file_path = path
        .join("vector_fields")
        .join("vectors")
        .join("field.flat");

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
            engine.delete_document_by_internal_id(i).unwrap();
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
        embedder: Arc::new(crate::embedding::precomputed::PrecomputedEmbedder::new()),
        deletion_config: crate::maintenance::deletion::DeletionConfig::default(),
        shard_id: 0,
        metadata_config: LexicalIndexConfig::default(),
    };

    let file_config = FileStorageConfig::new(path.to_str().unwrap());
    let storage_config = StorageConfig::File(file_config);
    let storage = StorageFactory::create(storage_config).unwrap();
    let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    let doc_store = Arc::new(RwLock::new(
        UnifiedDocumentStore::open(doc_storage).unwrap(),
    ));

    let engine = VectorStore::new(storage, config, doc_store.clone()).unwrap();

    let dim = 16;
    let num_vectors = 200;

    // 2. Insert vectors
    println!("Inserting {} vectors...", num_vectors);
    let mut batch_docs = HashMap::new();
    for i in 0..num_vectors {
        // Use random vectors to avoid equidistant pathology
        let mut vec_data = vec![0.0f32; dim];
        for j in 0..dim {
            vec_data[j] = (i + j) as f32 % 500.0 / 500.0; // Deterministic pseudo-random, unique for i < 500
        }

        let doc = Document::new().add_field("vectors", DataValue::Vector(vec_data));
        batch_docs.insert(i as u64, doc.clone());
        engine
            .upsert_document_by_internal_id(i as u64, doc)
            .unwrap();
    }
    doc_store.write().add_segment(&batch_docs).unwrap();

    println!("Flushing vectors to disk...");
    engine.flush().unwrap();
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
            engine.delete_document_by_internal_id(i as u64).unwrap();
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
        results.hits.len() >= 60,
        "Should have decent recall (>= 60/100) left. Found {}",
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
