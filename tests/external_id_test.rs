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
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_external_id_operations() -> sarissa::error::Result<()> {
    // 1. Setup
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // Lexical setup
    let tokenizer = Arc::new(WhitespaceTokenizer::new());
    let analyzer = PipelineAnalyzer::new(tokenizer).add_filter(Arc::new(LowercaseFilter::new()));

    let lexical_config = LexicalIndexConfig::builder()
        .analyzer(Arc::new(analyzer))
        .default_field("title")
        .build();
    let lexical_engine = LexicalEngine::new(storage.clone(), lexical_config)?;

    // Vector setup
    let vector_config = VectorIndexConfig {
        fields: HashMap::from([(
            "vector".to_string(),
            VectorFieldConfig {
                dimension: 3,
                distance: DistanceMetric::Cosine,
                index: VectorIndexKind::Flat,
                metadata: HashMap::new(),
                base_weight: 1.0,
            },
        )]),
        default_fields: vec!["vector".into()],
        metadata: HashMap::new(),
        default_distance: DistanceMetric::Cosine,
        default_dimension: Some(3),
        default_index_kind: VectorIndexKind::Flat,
        default_base_weight: 1.0,
        implicit_schema: false,
        embedder: Arc::new(sarissa::embedding::precomputed::PrecomputedEmbedder::new()),
        deletion_config: Default::default(),
    };
    let vector_engine = VectorEngine::new(storage.clone(), vector_config)?;

    // Hybrid setup
    let mut engine = HybridEngine::new(storage.clone(), lexical_engine, vector_engine)?;

    // 2. Index first document with ID "doc1"
    let mut lex_doc1 = LexicalDocument::new();
    lex_doc1.add_field_value("title", FieldValue::Text("Rust Programming".to_string()));

    let mut vec_payload1 = DocumentPayload::new();
    vec_payload1.set_field("vector", Payload::vector(vec![1.0f32, 0.0, 0.0]));

    let doc1 = HybridDocument::builder()
        .add_lexical_doc(lex_doc1)
        .add_vector_payload(vec_payload1)
        .build();

    let id1 = engine.index_document("doc1", doc1).await?;
    assert_eq!(id1, 0);

    engine.commit()?;

    // 3. Search should find it
    let request = HybridSearchRequest::new().with_text("rust");
    let results = engine.search(request).await?;
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.results[0].doc_id, 0);

    // 4. Index updated document with SAME ID "doc1"
    // This should delete old one (0) and assign new one (1)
    let mut lex_doc2 = LexicalDocument::new();
    lex_doc2.add_field_value("title", FieldValue::Text("Rust Programming v2".to_string()));

    let mut vec_payload2 = DocumentPayload::new();
    vec_payload2.set_field("vector", Payload::vector(vec![1.0f32, 0.0, 0.0]));

    let doc1_v2 = HybridDocument::builder()
        .add_lexical_doc(lex_doc2)
        .add_vector_payload(vec_payload2)
        .build();

    let id2 = engine.index_document("doc1", doc1_v2).await?;
    assert_eq!(id2, 1);

    engine.commit()?;

    // 5. Search should find NEW one (and only one result)
    let request = HybridSearchRequest::new().with_text("rust");
    let results = engine.search(request).await?;
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.results[0].doc_id, 1);

    // 6. Delete "doc1"
    let deleted = engine.delete_document_by_id("doc1")?;
    assert!(deleted);

    engine.commit()?;

    // 7. Search should find NOTHING
    let request = HybridSearchRequest::new().with_text("rust");
    let results = engine.search(request).await?;
    assert_eq!(results.results.len(), 0);

    Ok(())
}
