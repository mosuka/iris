use tempfile::TempDir;

use iris::Engine;
use iris::lexical::FieldOption as LexicalOption;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::FieldOption as VectorOption;
use iris::{DataValue, Document};
use iris::{FieldOption, Schema};

#[test]
fn test_unified_engine_indexing() -> iris::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines
    // We define a schema with:
    // - "title": Lexical only
    // - "embedding": Vector only
    // - "description": Lexical only

    // Note: VectorOption default uses Cosine, dim=128
    let vector_opt = VectorOption::default();
    let lexical_opt = LexicalOption::default();

    let config = Schema::builder()
        .add_field("title", FieldOption::Lexical(lexical_opt.clone()))
        .add_field("embedding", FieldOption::Vector(vector_opt.clone()))
        .add_field("description", FieldOption::Lexical(lexical_opt))
        .build();

    // 3. Initialize Engine
    let engine = Engine::new(storage.clone(), config)?;

    // 4. Index Documents
    let doc1 = Document::new()
        .add_field("title", DataValue::Text("Rust Programming".into()))
        .add_field("description", DataValue::Text("A systems language".into()))
        .add_field("embedding", DataValue::Vector(vec![0.1; 128])); // Valid vector

    let doc2 = Document::new()
        .add_field("title", DataValue::Text("Vector Search".into()))
        .add_field(
            "description",
            DataValue::Text("Searching with vectors".into()),
        )
        .add_field("embedding", DataValue::Vector(vec![0.2; 128]));

    engine.put_document("doc1", doc1)?;
    engine.put_document("doc2", doc2)?;

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
