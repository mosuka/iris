use sarissa::vector::core::vector::Vector;
use sarissa::vector::index::config::{FlatIndexConfig, HnswIndexConfig, IvfIndexConfig};
use sarissa::vector::index::flat::writer::FlatIndexWriter;
use sarissa::vector::index::hnsw::writer::HnswIndexWriter;
use sarissa::vector::index::ivf::writer::IvfIndexWriter;
use sarissa::vector::writer::{VectorIndexWriter, VectorIndexWriterConfig};
use std::collections::HashMap;

fn create_vector(id: u64, category: &str) -> (u64, String, Vector) {
    let mut metadata = HashMap::new();
    metadata.insert("category".to_string(), category.to_string());

    // 2D vector for simplicity
    let data = vec![if category == "A" { 1.0 } else { 2.0 }, 0.0];

    (id, "vec".to_string(), Vector::with_metadata(data, metadata))
}

fn test_flat() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Flat Index Deletion...");
    let config = FlatIndexConfig {
        dimension: 2,
        distance_metric: sarissa::vector::core::distance::DistanceMetric::Euclidean,
        ..Default::default()
    };
    let writer_config = VectorIndexWriterConfig::default();
    let mut writer = FlatIndexWriter::new(config, writer_config, "test_flat")?;

    let vectors = vec![
        create_vector(1, "A"),
        create_vector(2, "B"),
        create_vector(3, "A"),
    ];

    writer.build(vectors)?;

    // Delete category A
    let deleted = writer.delete_documents("category", "A")?;
    println!("Deleted {} documents from Flat Index", deleted);
    assert_eq!(deleted, 2);

    writer.finalize()?;

    assert_eq!(writer.vectors().len(), 1);
    assert_eq!(writer.vectors()[0].0, 2); // ID 2 should remain

    println!("Flat Index Deletion OK");
    Ok(())
}

fn test_hnsw() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing HNSW Index Deletion...");
    let config = HnswIndexConfig {
        dimension: 2,
        distance_metric: sarissa::vector::core::distance::DistanceMetric::Euclidean,
        m: 16,
        ef_construction: 100,
        ..Default::default()
    };
    let writer_config = VectorIndexWriterConfig::default();
    let mut writer = HnswIndexWriter::new(config, writer_config, "test_hnsw")?;

    let vectors = vec![
        create_vector(1, "A"),
        create_vector(2, "B"),
        create_vector(3, "A"),
    ];

    writer.build(vectors)?;

    // Delete category A
    let deleted = writer.delete_documents("category", "A")?;
    println!("Deleted {} documents from HNSW Index", deleted);
    assert_eq!(deleted, 2);

    writer.finalize()?;

    assert_eq!(writer.vectors().len(), 1);
    assert_eq!(writer.vectors()[0].0, 2);

    println!("HNSW Index Deletion OK");
    Ok(())
}

fn test_ivf() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing IVF Index Deletion...");
    let config = IvfIndexConfig {
        dimension: 2,
        distance_metric: sarissa::vector::core::distance::DistanceMetric::Euclidean,
        n_clusters: 1, // small for test
        n_probe: 1,
        normalize_vectors: false,
        ..Default::default()
    };
    let writer_config = VectorIndexWriterConfig::default();
    let mut writer = IvfIndexWriter::new(config, writer_config, "test_ivf")?;

    let vectors = vec![
        create_vector(1, "A"),
        create_vector(2, "B"),
        create_vector(3, "A"),
    ];

    writer.build(vectors)?;

    // Delete category A
    let deleted = writer.delete_documents("category", "A")?;
    println!("Deleted {} documents from IVF Index", deleted);
    assert_eq!(deleted, 2);

    writer.finalize()?;

    assert_eq!(writer.vectors().len(), 1);
    assert_eq!(writer.vectors()[0].0, 2);

    println!("IVF Index Deletion OK");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    test_flat()?;
    test_hnsw()?;
    test_ivf()?;
    Ok(())
}
