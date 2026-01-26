use iris::data::{DataValue, Document};
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::error::Result;
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::distance::DistanceMetric;
use iris::vector::core::field::{FlatOption, VectorOption};

fn build_test_engine() -> Result<Engine> {
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    let field_config = FieldConfig {
        vector: Some(VectorOption::Flat(FlatOption {
            dimension: 3,
            distance: DistanceMetric::Cosine,
            base_weight: 1.0,
            quantizer: None,
        })),
        lexical: None,
    };

    let config = IndexConfig::builder()
        .add_field("body", field_config)
        .build();

    Engine::new(storage, config)
}

fn create_payload(id: &str, vector: Vec<f32>) -> Document {
    Document::new()
        .add_field("_id", DataValue::Text(id.into()))
        .add_field("body", DataValue::Vector(vector))
}

#[test]
fn test_chunk_addition() -> Result<()> {
    let engine = build_test_engine()?;

    // 1. Add first chunk for "doc_A"
    let p1 = create_payload("doc_A", vec![1.0, 0.0, 0.0]);
    let id1 = engine.index_chunk(p1)?;

    // 2. Add second chunk for "doc_A"
    let p2 = create_payload("doc_A", vec![0.0, 1.0, 0.0]);
    let id2 = engine.index_chunk(p2)?;

    // Verify IDs are different
    assert_ne!(id1, id2, "Internal IDs should be different for chunks");

    engine.commit()?;

    let stats = engine.stats()?;
    assert_eq!(stats.document_count, 2, "Should have 2 documents total");

    Ok(())
}

#[test]
fn test_chunk_deletion() -> Result<()> {
    let engine = build_test_engine()?;

    // Add 2 chunks
    let p1 = create_payload("doc_A", vec![1.0, 0.0, 0.0]);
    engine.index_chunk(p1)?;

    let p2 = create_payload("doc_A", vec![0.0, 1.0, 0.0]);
    engine.index_chunk(p2)?;

    engine.commit()?;

    let stats_before = engine.stats()?;
    assert_eq!(stats_before.document_count, 2);

    // Delete "doc_A"
    engine.delete("doc_A")?;
    engine.commit()?;

    // Verify deletion
    let stats_after = engine.stats()?;
    assert_eq!(
        stats_after.document_count, 0,
        "All chunks should be deleted"
    );

    Ok(())
}

#[test]
fn test_mixed_mode_behavior() -> Result<()> {
    let engine = build_test_engine()?;

    // Add chunk 1
    engine.index_chunk(create_payload("doc_B", vec![1.0, 0.0, 0.0]))?;

    // Add chunk 2
    engine.index_chunk(create_payload("doc_B", vec![0.0, 1.0, 0.0]))?;

    engine.commit()?;
    assert_eq!(engine.stats()?.document_count, 2);

    // Now index (upsert) "doc_B" (should overwrite ALL of them)
    engine.index(create_payload("doc_B", vec![0.0, 0.0, 1.0]))?;
    engine.commit()?;

    // All chunks replaced by a single doc. Total should be 1.
    assert_eq!(engine.stats()?.document_count, 1);

    // Delete "doc_B" -> Should delete the remaining doc.
    engine.delete("doc_B")?;
    engine.commit()?;
    assert_eq!(engine.stats()?.document_count, 0);

    Ok(())
}
