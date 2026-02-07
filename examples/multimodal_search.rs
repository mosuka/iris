//! Multimodal Search Example - Search across text and images
//!
//! This example demonstrates:
//! 1. Configuring a vector index with CandleClipEmbedder (CLIP)
//! 2. Indexing images from local file system
//! 3. Indexing text descriptions
//! 4. Performing multimodal search:
//!    - Text query finding images
//!    - Image query finding images
//!    - Image query finding text
//!
//! To run this example:
//! ```bash
//! cargo run --example multimodal_search --features embeddings-multimodal
//! ```

#[cfg(feature = "embeddings-multimodal")]
#[cfg(feature = "embeddings-multimodal")]
use std::path::Path;

#[cfg(feature = "embeddings-multimodal")]
use iris::CandleClipEmbedder;
#[cfg(feature = "embeddings-multimodal")]
use iris::Embedder;
#[cfg(feature = "embeddings-multimodal")]
use iris::Result;
use iris::parking_lot::RwLock;
#[cfg(feature = "embeddings-multimodal")]
use iris::storage::file::FileStorageConfig;
use iris::storage::prefixed::PrefixedStorage;
#[cfg(feature = "embeddings-multimodal")]
use iris::storage::{StorageConfig, StorageFactory};
use iris::store::document::UnifiedDocumentStore;
#[cfg(feature = "embeddings-multimodal")]
use iris::vector::DistanceMetric;
#[cfg(feature = "embeddings-multimodal")]
use iris::vector::VectorSearchRequestBuilder;
#[cfg(feature = "embeddings-multimodal")]
use iris::vector::VectorStore;
#[cfg(feature = "embeddings-multimodal")]
use iris::vector::{FlatOption, FieldOption};
use iris::vector::{VectorFieldConfig, VectorIndexConfig};
#[cfg(feature = "embeddings-multimodal")]
use iris::{DataValue, Document};
#[cfg(feature = "embeddings-multimodal")]
use tempfile::TempDir;

#[cfg(feature = "embeddings-multimodal")]
fn main() -> Result<()> {
    use std::sync::Arc;

    println!("=== Multimodal Search Example (CLIP) ===\n");

    // 1. Setup Storage
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig::File(FileStorageConfig::new(temp_dir.path()));
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure Index with CandleClipEmbedder (CLIP)
    // We use "openai/clip-vit-base-patch32" which is a good balance of speed and quality
    println!("Loading CLIP model (this may take a while on first run)...");
    let embedder = CandleClipEmbedder::new("openai/clip-vit-base-patch32")?;
    println!("Model loaded: {}", embedder.name());

    let field_config = VectorFieldConfig {
        vector: Some(FieldOption::Flat(FlatOption {
            dimension: 3,
            distance: DistanceMetric::Cosine,
            base_weight: 1.0,
            quantizer: None,
        })),
        lexical: None,
    };

    let index_config = VectorIndexConfig::builder()
        .embedder(Arc::new(embedder))
        .field("content", field_config)
        .build()?;

    // 3. Create Engine
    let doc_storage = Arc::new(PrefixedStorage::new("documents", storage.clone()));
    let doc_store = Arc::new(RwLock::new(UnifiedDocumentStore::open(doc_storage)?));
    let engine = VectorStore::new(storage, index_config, doc_store.clone())?;

    // 4. Index Images
    println!("\n--- Indexing Images ---");
    let resources_dir = Path::new("resources");
    let images_dir = resources_dir.join("images");

    if !images_dir.exists() {
        eprintln!("Error: resources/images directory not found.");
        eprintln!(
            "Please ensure you are running from the project root and resources/images exists."
        );
        return Ok(());
    }

    let mut indexed_count = 0;
    // Iterate over jpg files
    let entries = std::fs::read_dir(&images_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "jpg" || ext == "jpeg" || ext == "png" {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    println!("Indexing image: {}", filename);

                    // Read image file into bytes
                    let bytes = std::fs::read(&path)?;

                    let doc = Document::builder()
                        .add_field("content", DataValue::Bytes(bytes, None))
                        .add_field("filename", DataValue::Text(filename.clone()))
                        .add_field("type", DataValue::Text("image".into()))
                        .build();

                    engine.add_document(doc)?;
                    indexed_count += 1;
                }
            }
        }
    }
    println!("Indexed {} images.", indexed_count);

    // 5. Index Text Descriptions
    // Start doc_ids after images (though automatic assignment handles it, we assume sequential for output)
    println!("\n--- Indexing Text ---");
    let texts = vec![
        "A cute kitten looking at the camera",
        "A loyal dog standing in the grass",
        "Two dogs playing together",
        "A landscape with mountains and a lake", // Distractor
    ];

    for text in &texts {
        println!("Indexing text: \"{}\"", text);
        let doc = Document::builder()
            .add_field("content", DataValue::Text((*text).into()))
            .add_field("text", DataValue::Text((*text).into()))
            .add_field("type", DataValue::Text("text".into()))
            .build();
        engine.add_document(doc)?;
    }

    // Commit to make documents searchable
    engine.commit()?;
    doc_store.write().commit()?;

    // 6. Search Demonstrations

    // Demo 1: Text-to-Image
    println!("\n--- Search 1: Text-to-Image ---");
    let query_text = "a photo of a cat";
    println!("Query: \"{}\"", query_text);

    let request = VectorSearchRequestBuilder::new()
        .add_text("content", query_text)
        .limit(3)
        .build();

    let results = engine.search(request)?;
    print_results(&results);

    // Demo 2: Image-to-Image
    println!("\n--- Search 2: Image-to-Image ---");
    let query_image_path = resources_dir.join("query_image.jpg");
    if query_image_path.exists() {
        println!("Query Image: {}", query_image_path.display());
        // Read query image bytes
        let query_bytes = std::fs::read(&query_image_path)?;

        // Create the request
        // We use add_bytes for the image query.
        // In a real app, you might want to detect mime type (e.g. "image/jpeg").
        let request = VectorSearchRequestBuilder::new()
            .add_bytes("content", query_bytes, Some("image/jpeg"))
            .limit(3)
            .build();

        let results = engine.search(request)?;
        print_results(&results);
    } else {
        println!(
            "Skipping Image-to-Image demo: {} not found",
            query_image_path.display()
        );
    }

    Ok(())
}

#[cfg(feature = "embeddings-multimodal")]
fn print_results(results: &iris::vector::VectorSearchResults) {
    for (i, hit) in results.hits.iter().enumerate() {
        println!("{}. Doc ID: {}, Score: {:.4}", i + 1, hit.doc_id, hit.score);
    }
}

#[cfg(not(feature = "embeddings-multimodal"))]
fn main() {
    eprintln!("This example requires the 'embeddings-multimodal' feature.");
    eprintln!(
        "Please run with: cargo run --example multimodal_search --features embeddings-multimodal"
    );
    std::process::exit(1);
}
