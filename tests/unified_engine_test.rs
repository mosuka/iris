use tempfile::TempDir;

use iris::data::{DataValue, Document};
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::lexical::core::field::FieldOption as LexicalOption;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::field::VectorOption;

#[test]
fn test_unified_engine_indexing() -> iris::error::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines
    // We define a schema with:
    // - "title": Lexical only
    // - "embedding": Vector only
    // - "description": Both (Hybrid)

    // Note: VectorOption default uses Cosine, dim=128
    let vector_opt = VectorOption::default();
    let lexical_opt = LexicalOption::default();

    let config = IndexConfig::builder()
        .add_field(
            "title",
            FieldConfig {
                lexical: Some(lexical_opt.clone()),
                vector: None,
            },
        )
        .add_field(
            "embedding",
            FieldConfig {
                lexical: None,
                vector: Some(vector_opt.clone()),
            },
        )
        .add_field(
            "description",
            FieldConfig {
                lexical: Some(lexical_opt),
                vector: Some(vector_opt),
            },
        )
        .build();

    // 3. Initialize Engine
    let engine = Engine::new(storage.clone(), config)?;

    // 4. Index Documents
    let doc1 = Document::new_with_id("doc1")
        .add_field("title", DataValue::Text("Rust Programming".into()))
        .add_field("description", DataValue::Text("A systems language".into()))
        .add_field("embedding", DataValue::Vector(vec![0.1; 128])); // Valid vector

    let doc2 = Document::new_with_id("doc2")
        .add_field("title", DataValue::Text("Vector Search".into()))
        .add_field(
            "description",
            DataValue::Text("Searching with vectors".into()),
        )
        .add_field("embedding", DataValue::Vector(vec![0.2; 128]));

    engine.index(doc1)?;
    engine.index(doc2)?;

    // 5. Commit
    engine.commit()?;

    // 6. Verify Files Exist in Storage
    // Lexical engine should create files in "lexical/"
    // Vector engine should create files in "vector/"
    let files = storage.list_files()?;
    println!("Created files: {:?}", files);

    let has_lexical = files.iter().any(|f| f.starts_with("lexical/"));
    let has_vector = files.iter().any(|f| f.starts_with("vector/"));

    assert!(has_lexical, "Should have created lexical index files");
    assert!(has_vector, "Should have created vector index files");

    Ok(())
}
