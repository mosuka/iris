use std::collections::HashMap;
use std::sync::Arc;

use iris::embedding::precomputed::PrecomputedEmbedder;
use iris::error::Result;
use iris::lexical::engine::config::LexicalIndexConfig;
use iris::storage::memory::{MemoryStorage, MemoryStorageConfig};
use iris::vector::core::distance::DistanceMetric;
use iris::vector::core::document::{DocumentPayload, Payload, PayloadSource};
use iris::vector::core::field::{FlatOption, VectorIndexKind, VectorOption};
use iris::vector::engine::VectorEngine;
use iris::vector::engine::config::{VectorFieldConfig, VectorIndexConfig};

fn build_test_engine() -> Result<VectorEngine> {
    let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));

    let field_config = VectorFieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 3,
            distance: DistanceMetric::Cosine,
            base_weight: 1.0,
            quantizer: None,
        })),
        lexical: None,
    };

    let config = VectorIndexConfig {
        fields: HashMap::from([("body".into(), field_config)]),
        default_fields: vec!["body".into()],
        metadata: HashMap::new(),
        default_distance: DistanceMetric::Cosine,
        default_dimension: Some(3),
        default_index_kind: VectorIndexKind::Flat,
        default_base_weight: 1.0,
        implicit_schema: false,
        embedder: Arc::new(PrecomputedEmbedder::new()),
        deletion_config: Default::default(),
        shard_id: 0,
        metadata_config: LexicalIndexConfig::builder()
            .analyzer(Arc::new(
                iris::analysis::analyzer::keyword::KeywordAnalyzer::default(),
            ))
            .build(),
    };

    VectorEngine::new(storage, config)
}

fn create_payload(id: &str, vector: Vec<f32>) -> DocumentPayload {
    let mut metadata = HashMap::new();
    metadata.insert("_id".to_string(), id.to_string());

    let mut fields = HashMap::new();
    fields.insert(
        "body".to_string(),
        Payload {
            source: PayloadSource::Vector {
                data: Arc::from(vector),
            },
        },
    );

    DocumentPayload { fields, metadata }
}

#[test]
fn test_chunk_addition() -> Result<()> {
    let engine = build_test_engine()?;

    // 1. Add first chunk for "doc_A"
    let p1 = create_payload("doc_A", vec![1.0, 0.0, 0.0]);
    let id1 = engine.index_payload_chunk(p1)?;

    // 2. Add second chunk for "doc_A"
    let p2 = create_payload("doc_A", vec![0.0, 1.0, 0.0]);
    let id2 = engine.index_payload_chunk(p2)?;

    // Verify IDs are different
    assert_ne!(id1, id2, "Internal IDs should be different for chunks");

    // 3. Verify Lexical Engine has both IDs
    // We need to access metadata_index via private field? No, VectorEngine doesn't expose metadata_index publicly.
    // We can use delete_document_by_id or other public methods to verify indirectlly,
    // OR Lexical search if exposed?
    // VectorEngine doesn't expose search on metadata explicitly other than during filter.
    // However, we can use `count`.
    let stats = engine.stats()?;
    assert_eq!(stats.document_count, 2, "Should have 2 documents total");

    Ok(())
}

#[test]
fn test_chunk_deletion() -> Result<()> {
    let engine = build_test_engine()?;

    // Add 2 chunks
    let p1 = create_payload("doc_A", vec![1.0, 0.0, 0.0]);
    engine.index_payload_chunk(p1)?;

    let p2 = create_payload("doc_A", vec![0.0, 1.0, 0.0]);
    engine.index_payload_chunk(p2)?;

    let stats_before = engine.stats()?;
    assert_eq!(stats_before.document_count, 2);

    // Delete "doc_A"
    let deleted = engine.delete_document_by_id("doc_A")?;
    assert!(deleted, "Should return true for deletion");

    // Verify deletion
    let stats_after = engine.stats()?;
    assert_eq!(
        stats_after.document_count, 0,
        "All chunks should be deleted"
    );

    // Verify idempotency
    let deleted_again = engine.delete_document_by_id("doc_A")?;
    assert!(!deleted_again, "Should return false if already deleted");

    Ok(())
}

#[test]
fn test_mixed_mode_behavior() -> Result<()> {
    // Verify that index_payloads (upsert) still works as expected (overwrite)
    // AND check interaction with chunks.
    // Note: Upserting with existing external ID currently OVERWRITES.
    // If we have MULTIPLE chunks, and we call `index_payloads` (upsert),
    // `index_document` in LexicalEngine finds ONE existing ID and updates it.
    // It does NOT delete others. This is "undefined behavior" or "partial overwrite" currently.
    // For this task we accept this behavior, but we should know what happens.

    let engine = build_test_engine()?;

    // Add chunk 1
    engine.index_payload_chunk(create_payload("doc_B", vec![1.0, 0.0, 0.0]))?;

    // Add chunk 2
    engine.index_payload_chunk(create_payload("doc_B", vec![0.0, 1.0, 0.0]))?;

    assert_eq!(engine.stats()?.document_count, 2);

    // Now Upsert "doc_B" (should overwrite one of them)
    engine.index_payloads("doc_B", create_payload("doc_B", vec![0.0, 0.0, 1.0]))?;

    // One doc Updated, one untouched. Total should be 2.
    // Because Upsert = Find ID -> Update Doc content.
    // New content replaces old content for THAT ID.
    // The other ID remains.
    assert_eq!(engine.stats()?.document_count, 2);

    // Delete "doc_B" -> Should delete BOTH.
    engine.delete_document_by_id("doc_B")?;
    assert_eq!(engine.stats()?.document_count, 0);

    Ok(())
}
