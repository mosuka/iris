use std::collections::HashMap;
use std::sync::Arc;

use sarissa::analysis::analyzer::pipeline::PipelineAnalyzer;
use sarissa::analysis::token_filter::lowercase::LowercaseFilter;
use sarissa::analysis::tokenizer::whitespace::WhitespaceTokenizer;
use sarissa::hybrid::core::document::HybridDocument;
use sarissa::hybrid::engine::HybridEngine;
use sarissa::hybrid::search::searcher::HybridSearchRequest;
use sarissa::lexical::core::document::Document as LexicalDocument;
use sarissa::lexical::core::field::FieldValue;
use sarissa::lexical::engine::LexicalEngine;
use sarissa::lexical::engine::config::LexicalIndexConfig;
use sarissa::storage::memory::MemoryStorageConfig;
use sarissa::storage::{StorageConfig, StorageFactory};
use sarissa::vector::DistanceMetric;
use sarissa::vector::core::document::{DocumentPayload, Payload};
use sarissa::vector::engine::VectorEngine;
use sarissa::vector::engine::config::{VectorFieldConfig, VectorIndexConfig, VectorIndexKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== External ID Support Demo ===\n");

    // 1. Initialize Engines (In-Memory for demo)
    println!("-> Initializing Hybrid Engine...");
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // Lexical Config
    let tokenizer = Arc::new(WhitespaceTokenizer::new());
    let analyzer = PipelineAnalyzer::new(tokenizer).add_filter(Arc::new(LowercaseFilter::new()));
    let lexical_config = LexicalIndexConfig::builder()
        .analyzer(Arc::new(analyzer))
        .default_field("title")
        .build();
    let lexical_engine = LexicalEngine::new(storage.clone(), lexical_config)?;

    // Vector Config
    let vector_config = VectorIndexConfig {
        fields: HashMap::from([(
            "description_vector".to_string(),
            VectorFieldConfig {
                dimension: 3,
                distance: DistanceMetric::Cosine,
                index: VectorIndexKind::Flat,
                metadata: HashMap::new(),
                base_weight: 1.0,
            },
        )]),
        default_fields: vec!["description_vector".into()],
        metadata: HashMap::new(),
        default_distance: DistanceMetric::Cosine,
        default_dimension: Some(3), // 3D for demo simplicity
        default_index_kind: VectorIndexKind::Flat,
        default_base_weight: 1.0,
        implicit_schema: false,
        embedder: Arc::new(sarissa::embedding::precomputed::PrecomputedEmbedder::new()),
        deletion_config: Default::default(),
    };
    let vector_engine = VectorEngine::new(storage.clone(), vector_config)?;

    // Hybrid Engine
    let mut engine = HybridEngine::new(storage.clone(), lexical_engine, vector_engine)?;

    // 2. Index a Document with External ID "product_123"
    println!("\n-> Indexing document with ID 'product_123'...");

    let mut lex_doc = LexicalDocument::new();
    lex_doc.add_field_value("title", FieldValue::Text("Smartphone Pro Max".to_string()));
    lex_doc.add_field_value("category", FieldValue::Text("electronics".to_string()));

    let mut vec_payload = DocumentPayload::new();
    // Simplified vector for "high tech phone"
    vec_payload.set_field("description_vector", Payload::vector(vec![0.9, 0.8, 0.1]));

    let doc = HybridDocument::builder()
        .add_lexical_doc(lex_doc)
        .add_vector_payload(vec_payload)
        .build();

    let internal_id_1 = engine.index_document("product_123", doc).await?;
    println!("   Assigned Internal ID: {}", internal_id_1);
    engine.commit()?;

    // 3. Search to verify
    println!("\n-> Searching for 'smartphone'...");
    let request = HybridSearchRequest::new().with_text("smartphone");
    let results = engine.search(request).await?;
    if let Some(hit) = results.results.first() {
        println!(
            "   Found match! Internal ID: {}, Score: {}",
            hit.doc_id, hit.hybrid_score
        );
    } else {
        println!("   No match found.");
    }

    // 4. Update the Document (Overwriting "product_123")
    println!("\n-> Updating document 'product_123' (changed category to 'mobile')...");

    let mut lex_doc_v2 = LexicalDocument::new();
    lex_doc_v2.add_field_value("title", FieldValue::Text("Smartphone Pro Max".to_string()));
    lex_doc_v2.add_field_value("category", FieldValue::Text("mobile".to_string())); // Changed

    let mut vec_payload_v2 = DocumentPayload::new();
    vec_payload_v2.set_field("description_vector", Payload::vector(vec![0.9, 0.85, 0.15])); // Slightly tuned vector

    let doc_v2 = HybridDocument::builder()
        .add_lexical_doc(lex_doc_v2)
        .add_vector_payload(vec_payload_v2)
        .build();

    let internal_id_2 = engine.index_document("product_123", doc_v2).await?;
    println!("   New Assiged Internal ID: {}", internal_id_2);
    engine.commit()?;

    // 5. Verify Update (Old ID should be gone)
    println!("\n-> Verifying update...");
    // We'll rely on search results.
    // Note: 'category' field was added but not explicitly configured in LexicalConfig as a default field.
    // However, HybridSearchRequest::new().with_text("mobile") uses default field ("title").
    // We should search specifically in the category field OR ensure category is indexed.
    // For this demo, let's search for "Pro" which is in the title, and check the ID.
    // Or better, search "category:mobile" if query parser supports it (it should).
    let request = HybridSearchRequest::new().with_text("category:mobile");
    let results = engine.search(request).await?;
    println!(
        "   Search for 'mobile' found {} results.",
        results.results.len()
    );
    if let Some(hit) = results.results.first() {
        println!(
            "   Hit Internal ID: {} (Expected: {})",
            hit.doc_id, internal_id_2
        );
        assert_eq!(hit.doc_id, internal_id_2);
    }

    // 6. Delete Document
    println!("\n-> Deleting document 'product_123'...");
    let deleted = engine.delete_document_by_id("product_123")?;
    println!("   Deleted: {}", deleted);
    engine.commit()?;

    // 7. Verify Deletion
    println!("\n-> Verifying deletion (Search for 'smartphone')...");
    let request = HybridSearchRequest::new().with_text("smartphone");
    let results = engine.search(request).await?;
    println!("   Found {} results.", results.results.len());
    assert_eq!(results.results.len(), 0);

    println!("\n=== Demo Complete ===");
    Ok(())
}
