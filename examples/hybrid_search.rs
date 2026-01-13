use std::error::Error;
use std::sync::Arc;

use sarissa::analysis::analyzer::pipeline::PipelineAnalyzer;
use sarissa::analysis::token_filter::lowercase::LowercaseFilter;
use sarissa::analysis::tokenizer::whitespace::WhitespaceTokenizer;
use sarissa::hybrid::core::document::HybridDocument;
use sarissa::hybrid::engine::HybridEngine;
use sarissa::hybrid::search::searcher::HybridSearchRequest;
use sarissa::lexical::core::document::Document;
use sarissa::lexical::core::field::TextOption;
use sarissa::lexical::engine::LexicalEngine;
use sarissa::lexical::engine::config::LexicalIndexConfig;
use sarissa::lexical::search::searcher::LexicalSearchRequest;
use sarissa::storage::memory::MemoryStorageConfig;
use sarissa::storage::{StorageConfig, StorageFactory};
use sarissa::vector::core::distance::DistanceMetric;
use sarissa::vector::core::document::{DocumentPayload, Payload, StoredVector};
use sarissa::vector::engine::VectorEngine;
use sarissa::vector::engine::config::{VectorFieldConfig, VectorIndexConfig, VectorIndexKind};
use sarissa::vector::engine::request::{QueryVector, VectorSearchRequest};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hybrid Search Example (Recommended Usage)");
    println!("=======================================");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // 1. Setup Storage
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure & Create Lexical Engine
    // We use a simple tokenizer/analyzer pipeline here
    let tokenizer = Arc::new(WhitespaceTokenizer::new());
    let analyzer = PipelineAnalyzer::new(tokenizer).add_filter(Arc::new(LowercaseFilter::new()));
    let lexical_config = LexicalIndexConfig::builder()
        .analyzer(Arc::new(analyzer))
        .default_field("content")
        .build();
    let lexical_engine = LexicalEngine::new(storage.clone(), lexical_config)?;

    // 3. Configure & Create Vector Engine
    let vector_config = VectorIndexConfig::builder()
        .field(
            "vector",
            VectorFieldConfig {
                dimension: 3,
                distance: DistanceMetric::Euclidean, // Matches example data flavor
                index: VectorIndexKind::Hnsw,
                ..Default::default()
            },
        )
        .default_field("vector")
        .build()?;
    let vector_engine = VectorEngine::new(storage.clone(), vector_config)?;

    // 4. Create Hybrid Engine (The recommended high-level entry point)
    let mut engine = HybridEngine::new(storage.clone(), lexical_engine, vector_engine)?;

    // 5. Index Documents with External IDs
    rt.block_on(async {
        // Doc 1: "apple banana", vector [1.0, 0.0, 0.0]
        // We give it a stable external ID: "product-1"
        println!("Indexing 'product-1'...");
        let doc1 = Document::builder()
            .add_text("content", "apple banana", TextOption::default())
            .build();
        let mut payload1 = DocumentPayload::new();
        payload1.set_field("vector", Payload::vector(vec![1.0f32, 0.0, 0.0]));

        let hybrid_doc1 = HybridDocument::builder()
            .add_lexical_doc(doc1)
            .add_vector_payload(payload1)
            .build();

        engine
            .index_document("product-1", hybrid_doc1)
            .await
            .unwrap();

        // Doc 2: "banana orange", vector [0.0, 1.0, 0.0]
        // External ID: "product-2"
        println!("Indexing 'product-2'...");
        let doc2 = Document::builder()
            .add_text("content", "banana orange", TextOption::default())
            .build();
        let mut payload2 = DocumentPayload::new();
        payload2.set_field("vector", Payload::vector(vec![0.0f32, 1.0, 0.0]));

        let hybrid_doc2 = HybridDocument::builder()
            .add_lexical_doc(doc2)
            .add_vector_payload(payload2)
            .build();

        engine
            .index_document("product-2", hybrid_doc2)
            .await
            .unwrap();
    });

    // 6. Commit
    engine.commit()?;
    println!("Committed changes.");

    // 7. Execute Hybrid Search
    println!("\n--- Hybrid Search for 'apple' near [0.95, 0.05, 0.0] ---");

    // Lexical Query: "content:apple"
    // Note: We search the field "content". System automatically indexed "_id" field too.
    let lexical_req = LexicalSearchRequest::new("content:apple");

    // Vector Query: Close to product-1
    let query_vec_data = vec![0.95, 0.05, 0.0];
    let vector_req = VectorSearchRequest {
        query_vectors: vec![QueryVector {
            vector: StoredVector {
                data: std::sync::Arc::from(query_vec_data.as_slice()),
                weight: 1.0,
                attributes: Default::default(),
            },
            fields: Some(vec!["vector".to_string()]),
            weight: 1.0,
        }],
        ..Default::default()
    };

    let search_request = HybridSearchRequest::new()
        .with_lexical_request(lexical_req)
        .with_vector_request(vector_req);

    let results = rt.block_on(engine.search(search_request))?;

    println!("Found {} results:", results.results.len());
    for (i, result) in results.results.iter().enumerate() {
        println!(
            "{}. Internal ID: {} (Score: {:.4})",
            i + 1,
            result.doc_id,
            result.hybrid_score
        );
        // We can retrieve the stored _id to verify
        // Ideally HybridSearchResults returns fields, but currently it returns metadata if requested.
        // For demonstration, we assume user trusts the ID.
    }

    // 8. Demonstrate Update via External ID
    println!("\n--- Updating 'product-1' (changing content to 'green apple') ---");
    rt.block_on(async {
        let doc1_v2 = Document::builder()
            .add_text("content", "green apple", TextOption::default())
            .build();
        // keeping same vector for simplicity
        let mut payload1_v2 = DocumentPayload::new();
        payload1_v2.set_field("vector", Payload::vector(vec![1.0f32, 0.0, 0.0]));

        let hybrid_doc1_v2 = HybridDocument::builder()
            .add_lexical_doc(doc1_v2)
            .add_vector_payload(payload1_v2)
            .build();

        // Re-index with SAME ID "product-1". This replaces the old document.
        engine
            .index_document("product-1", hybrid_doc1_v2)
            .await
            .unwrap();
    });
    engine.commit()?;

    // Search again for "green"
    println!("Searching for 'green'...");
    let req_update = HybridSearchRequest::new().with_text("content:green");
    let results_update = rt.block_on(engine.search(req_update))?;
    println!("Found {} results for 'green'", results_update.results.len());

    Ok(())
}
