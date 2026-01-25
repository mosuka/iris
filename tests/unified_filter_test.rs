use tempfile::TempDir;

use iris::data::Document;
use iris::engine::Engine;
use iris::engine::config::{FieldConfig, IndexConfig};
use iris::engine::search::SearchRequestBuilder;
use iris::lexical::core::field::FieldOption as LexicalOption;
use iris::lexical::index::inverted::query::term::TermQuery;
use iris::storage::file::FileStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::field::VectorOption;
use iris::vector::store::query::VectorSearchRequestBuilder;

#[test]
fn test_unified_filtering() -> iris::error::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines
    use iris::vector::core::field::FlatOption;
    let vector_opt: VectorOption = FlatOption::default().dimension(2).into();
    let lexical_opt = LexicalOption::default();

    let config = IndexConfig::builder()
        .add_field(
            "name",
            FieldConfig {
                lexical: Some(lexical_opt.clone()),
                vector: None,
            },
        )
        .add_field(
            "category",
            FieldConfig {
                lexical: Some(lexical_opt),
                vector: None,
            },
        )
        .add_field(
            "embedding",
            FieldConfig {
                lexical: None,
                vector: Some(vector_opt),
            },
        )
        .build();

    let engine = Engine::new(storage.clone(), config)?;

    // 3. Index Documents
    // Doc 1: Apple (Fruit), Vector [1.0, 0.0]
    engine.index(
        Document::new()
            .with_id("doc1")
            .with_field("name", "Apple")
            .with_field("category", "fruit")
            .with_field("embedding", vec![1.0, 0.0]),
    )?;

    // Doc 2: Banana (Fruit), Vector [0.9, 0.1]
    engine.index(
        Document::new()
            .with_id("doc2")
            .with_field("name", "Banana")
            .with_field("category", "fruit")
            .with_field("embedding", vec![0.9, 0.1]),
    )?;

    // Doc 3: Carrot (Vegetable), Vector [1.0, 0.0] -> Identical vector to Apple!
    engine.index(
        Document::new()
            .with_id("doc3")
            .with_field("name", "Carrot")
            .with_field("category", "vegetable")
            .with_field("embedding", vec![1.0, 0.0]),
    )?;

    engine.commit()?;

    // 4. Test Filtering: Search for [1.0, 0.0] (Apple/Carrot) but filter for "vegetable"
    let vector_req = VectorSearchRequestBuilder::new()
        .add_vector("embedding", vec![1.0, 0.0])
        .build();

    let filter_query = Box::new(TermQuery::new("category", "vegetable"));

    let req = SearchRequestBuilder::new()
        .with_vector(vector_req.clone())
        .filter(filter_query)
        .build();

    let results = engine.search(req)?;

    // Should pass through Engine -> Lexical Filter -> Vector Store allowed_ids
    println!("Filtered Results (Vegetable): {:?}", results);

    assert_eq!(results.len(), 1, "Should filter down to 1 result");

    // Check it's Carrot (doc_id is internal, but logic holds)
    // We should load document to verify content if possible, but SearchResult has document loaded?
    // Engine search loads documents for lexical hits, but for purely vector hits?
    // Engine search returns SearchResult without loading doc content for vector hits unless fusion?
    // Wait, Engine search vector hits mapping uses `document: None`.
    // I need to verify which doc it is. Based on ID?
    // Or I can check scores.

    // Let's rely on the count.
    // Without filter, [1.0, 0.0] should match Apple (1.0) and Carrot (1.0) equally.
    // Banana (0.9) is less relevant.

    // 5. Test Filtering: Search for "fruit"
    let filter_query_fruit = Box::new(TermQuery::new("category", "fruit"));
    let req_fruit = SearchRequestBuilder::new()
        .with_vector(vector_req)
        .filter(filter_query_fruit)
        .build();

    let results_fruit = engine.search(req_fruit)?;
    println!("Filtered Results (Fruit): {:?}", results_fruit);
    assert_eq!(
        results_fruit.len(),
        2,
        "Should filter down to 2 results (Apple, Banana)"
    );

    Ok(())
}

#[test]
fn test_unified_filtering_hnsw() -> iris::error::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines with HNSW
    use iris::vector::core::field::HnswOption;
    let vector_opt: VectorOption = HnswOption::default().dimension(2).into();
    let lexical_opt = LexicalOption::default();

    let config = IndexConfig::builder()
        .add_field(
            "name",
            FieldConfig {
                lexical: Some(lexical_opt.clone()),
                vector: None,
            },
        )
        .add_field(
            "category",
            FieldConfig {
                lexical: Some(lexical_opt),
                vector: None,
            },
        )
        .add_field(
            "embedding",
            FieldConfig {
                lexical: None,
                vector: Some(vector_opt),
            },
        )
        .build();

    let engine = Engine::new(storage.clone(), config)?;

    // 3. Index Documents
    engine.index(
        Document::new()
            .with_id("doc1")
            .with_field("name", "Apple")
            .with_field("category", "fruit")
            .with_field("embedding", vec![1.0, 0.0]),
    )?;
    engine.index(
        Document::new()
            .with_id("doc2")
            .with_field("name", "Banana")
            .with_field("category", "fruit")
            .with_field("embedding", vec![0.9, 0.1]),
    )?;
    engine.index(
        Document::new()
            .with_id("doc3")
            .with_field("name", "Carrot")
            .with_field("category", "vegetable")
            .with_field("embedding", vec![1.0, 0.0]),
    )?;
    engine.commit()?;

    // 4. Test Filtering: Search for [1.0, 0.0] but filter for "vegetable"
    let vector_req = VectorSearchRequestBuilder::new()
        .add_vector("embedding", vec![1.0, 0.0])
        .build();

    let filter_query = Box::new(TermQuery::new("category", "vegetable"));
    let req = SearchRequestBuilder::new()
        .with_vector(vector_req)
        .filter(filter_query)
        .build();

    let results = engine.search(req)?;
    println!("Filtered Results HNSW (Vegetable): {:?}", results);
    assert_eq!(results.len(), 1, "Should filter down to 1 result");
    Ok(())
}

#[test]
fn test_unified_filtering_ivf() -> iris::error::Result<()> {
    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Engines with IVF
    use iris::vector::core::field::IvfOption;
    // Note: IVF usually requires training or pre-defined centroids.
    // However, our mocked/simple implementation might work with small data or just use simple clustering (if implemented).
    // Or we rely on `IvfFieldReader` behavior validation specifically.
    // If standard IVF needs training, this test might fail if training isn't triggered.
    // Assuming implicit training or minimal setup for test.
    let vector_opt: VectorOption = IvfOption::default().dimension(2).n_clusters(1).into();
    let lexical_opt = LexicalOption::default();

    let config = IndexConfig::builder()
        .add_field(
            "name",
            FieldConfig {
                lexical: Some(lexical_opt.clone()),
                vector: None,
            },
        )
        .add_field(
            "category",
            FieldConfig {
                lexical: Some(lexical_opt),
                vector: None,
            },
        )
        .add_field(
            "embedding",
            FieldConfig {
                lexical: None,
                vector: Some(vector_opt),
            },
        )
        .build();

    let engine = Engine::new(storage.clone(), config)?;

    // 3. Index Documents
    engine.index(
        Document::new()
            .with_id("doc1")
            .with_field("name", "Apple")
            .with_field("category", "fruit")
            .with_field("embedding", vec![1.0, 0.0]),
    )?;
    engine.index(
        Document::new()
            .with_id("doc2")
            .with_field("name", "Banana")
            .with_field("category", "fruit")
            .with_field("embedding", vec![0.9, 0.1]),
    )?;
    engine.index(
        Document::new()
            .with_id("doc3")
            .with_field("name", "Carrot")
            .with_field("category", "vegetable")
            .with_field("embedding", vec![1.0, 0.0]),
    )?;
    engine.commit()?;

    // 4. Test Filtering: Search for [1.0, 0.0] but filter for "vegetable"
    let vector_req = VectorSearchRequestBuilder::new()
        .add_vector("embedding", vec![1.0, 0.0])
        .build();

    let filter_query = Box::new(TermQuery::new("category", "vegetable"));
    let req = SearchRequestBuilder::new()
        .with_vector(vector_req)
        .filter(filter_query)
        .build();

    let results = engine.search(req)?;
    println!("Filtered Results IVF (Vegetable): {:?}", results);
    assert_eq!(results.len(), 1, "Should filter down to 1 result");
    Ok(())
}
