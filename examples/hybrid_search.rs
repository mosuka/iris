//! Hybrid Search Example - Basic usage guide via the unified Engine API.
//!
//! This example demonstrates how to combine lexical and vector search:
//! 1. Setup storage and Engine with both lexical and vector fields
//! 2. Combine results using various fusion algorithms (RRF, WeightedSum)

use std::sync::Arc;

use async_trait::async_trait;
use iris::data::Document;
use iris::embedding::embedder::{EmbedInput, EmbedInputType, Embedder};
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::engine::search::{FusionAlgorithm, SearchRequestBuilder};
use iris::error::Result;
use iris::lexical::core::field::{FieldOption, TextOption};
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::field::{FlatOption, VectorOption};
use iris::vector::core::vector::Vector;
use iris::vector::store::query::VectorSearchRequestBuilder;
use std::any::Any;

// Simple Mock Embedder for the example
#[derive(Debug, Clone)]
struct SimpleEmbedder;

#[async_trait]
impl Embedder for SimpleEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(t) => {
                let t = t.to_lowercase();
                if t.contains("apple") {
                    Ok(Vector::new(vec![1.0, 0.0, 0.0, 0.0]))
                } else if t.contains("banana") {
                    Ok(Vector::new(vec![0.0, 1.0, 0.0, 0.0]))
                } else if t.contains("orange") {
                    Ok(Vector::new(vec![0.0, 0.0, 1.0, 0.0]))
                } else {
                    Ok(Vector::new(vec![0.0, 0.0, 0.0, 1.0]))
                }
            }
            _ => Ok(Vector::new(vec![0.0; 4])),
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
        "SimpleEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn main() -> Result<()> {
    println!("=== Hybrid Search Example (Unified Engine) ===\n");

    // 1. Setup Storage
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engine
    // We define "content" as a field that has BOTH vector and lexical indexing.
    let config = IndexConfig::builder()
        .embedder(Arc::new(SimpleEmbedder))
        .add_field(
            "content",
            FieldConfig {
                vector: Some(VectorOption::Flat(FlatOption {
                    dimension: 4,
                    ..Default::default()
                })),
                lexical: Some(FieldOption::Text(TextOption::default())),
            },
        )
        .build();

    let engine = Engine::new(storage, config)?;

    // 3. Index Documents
    let docs = vec![
        ("doc1", "apple banana"),
        ("doc2", "banana orange"),
        ("doc3", "orange grape"),
    ];

    println!("Indexing {} documents...", docs.len());
    for (id, content) in docs {
        let doc = Document::new().with_id(id).with_field("content", content);
        engine.index(doc)?;
    }
    engine.commit()?;

    // 4. Perform Hybrid Searches

    println!("\n--- Hybrid Search 1: Vector('apple') + Lexical('orange') with RRF ---");
    // RRF (Reciprocal Rank Fusion) is good for combining results from different scorers
    let request_rrf = SearchRequestBuilder::new()
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("content", "apple")
                .build(),
        )
        .with_lexical(Box::new(
            iris::lexical::index::inverted::query::term::TermQuery::new("content", "orange"),
        ))
        .fusion(FusionAlgorithm::RRF { k: 60.0 })
        .build();

    let results_rrf = engine.search(request_rrf)?;
    for (i, hit) in results_rrf.iter().enumerate() {
        if let Ok(Some(doc)) = engine.get_document(hit.doc_id) {
            println!(
                "{}. ID: {}, Score: {:.4}",
                i + 1,
                doc.id.as_deref().unwrap_or("unknown"),
                hit.score
            );
        }
    }

    println!("\n--- Hybrid Search 2: Vector('apple') + Lexical('orange') with WeightedSum ---");
    // WeightedSum is good when you want to control the influence of each search type
    let request_ws = SearchRequestBuilder::new()
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("content", "apple")
                .build(),
        )
        .with_lexical(Box::new(
            iris::lexical::index::inverted::query::term::TermQuery::new("content", "orange"),
        ))
        .fusion(FusionAlgorithm::WeightedSum {
            lexical_weight: 0.5,
            vector_weight: 0.5,
        })
        .build();

    let results_ws = engine.search(request_ws)?;
    for (i, hit) in results_ws.iter().enumerate() {
        if let Ok(Some(doc)) = engine.get_document(hit.doc_id) {
            println!(
                "{}. ID: {}, Score: {:.4}",
                i + 1,
                doc.id.as_deref().unwrap_or("unknown"),
                hit.score
            );
        }
    }

    Ok(())
}
