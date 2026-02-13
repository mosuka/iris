//! GeoQuery example - demonstrates geographic location-based searches.

use std::sync::Arc;

use tempfile::TempDir;

use iris::Result;
use iris::analysis::analyzer::analyzer::Analyzer;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::GeoQuery;
use iris::lexical::InvertedIndexConfig;
use iris::lexical::LexicalIndexConfig;
use iris::lexical::LexicalSearchRequest;
use iris::lexical::LexicalStore;
use iris::lexical::Query;
use iris::storage::StorageConfig;
use iris::storage::StorageFactory;
use iris::storage::file::FileStorageConfig;
use iris::{DataValue, Document};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== GeoQuery Example - Geographic Location-Based Search ===\n");

    // Create a storage backend
    let temp_dir = TempDir::new().unwrap();
    let storage =
        StorageFactory::create(StorageConfig::File(FileStorageConfig::new(temp_dir.path())))?;

    // Create an analyzer
    let standard_analyzer: Arc<dyn Analyzer> = Arc::new(StandardAnalyzer::new()?);
    let keyword_analyzer: Arc<dyn Analyzer> = Arc::new(KeywordAnalyzer::new());
    let mut per_field_analyzer = PerFieldAnalyzer::new(Arc::clone(&standard_analyzer));
    per_field_analyzer.add_analyzer("id", Arc::clone(&keyword_analyzer));

    // Create a lexical engine
    let lexical_index_config = LexicalIndexConfig::Inverted(InvertedIndexConfig {
        analyzer: Arc::new(per_field_analyzer.clone()),
        ..InvertedIndexConfig::default()
    });
    let lexical_engine = LexicalStore::new(storage, lexical_index_config)?;

    // Add documents with geographic coordinates
    // Using famous locations around the world
    let documents = vec![
        Document::builder()
            .add_field("name", DataValue::Text("Central Park".into()))
            .add_field(
                "description",
                DataValue::Text("Large public park in Manhattan, New York City".into()),
            )
            .add_field("category", DataValue::Text("park".into()))
            .add_field("location", DataValue::Geo(40.7829, -73.9654))
            .add_field("city", DataValue::Text("New York".into()))
            .add_field("id", DataValue::Text("loc001".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Statue of Liberty".into()))
            .add_field(
                "description",
                DataValue::Text("Iconic statue on Liberty Island".into()),
            )
            .add_field("category", DataValue::Text("monument".into()))
            .add_field("location", DataValue::Geo(40.6892, -74.0445))
            .add_field("city", DataValue::Text("New York".into()))
            .add_field("id", DataValue::Text("loc002".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Golden Gate Bridge".into()))
            .add_field(
                "description",
                DataValue::Text("Suspension bridge in San Francisco".into()),
            )
            .add_field("category", DataValue::Text("bridge".into()))
            .add_field("location", DataValue::Geo(37.8199, -122.4783))
            .add_field("city", DataValue::Text("San Francisco".into()))
            .add_field("id", DataValue::Text("loc003".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Alcatraz Island".into()))
            .add_field(
                "description",
                DataValue::Text("Former federal prison on island in San Francisco Bay".into()),
            )
            .add_field("category", DataValue::Text("historical".into()))
            .add_field("location", DataValue::Geo(37.8267, -122.4233))
            .add_field("city", DataValue::Text("San Francisco".into()))
            .add_field("id", DataValue::Text("loc004".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Hollywood Sign".into()))
            .add_field(
                "description",
                DataValue::Text("Landmark sign in Hollywood Hills".into()),
            )
            .add_field("category", DataValue::Text("landmark".into()))
            .add_field("location", DataValue::Geo(34.1341, -118.3215))
            .add_field("city", DataValue::Text("Los Angeles".into()))
            .add_field("id", DataValue::Text("loc005".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Santa Monica Pier".into()))
            .add_field(
                "description",
                DataValue::Text("Amusement park and pier on Santa Monica Beach".into()),
            )
            .add_field("category", DataValue::Text("entertainment".into()))
            .add_field("location", DataValue::Geo(34.0084, -118.4966))
            .add_field("city", DataValue::Text("Los Angeles".into()))
            .add_field("id", DataValue::Text("loc006".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Space Needle".into()))
            .add_field(
                "description",
                DataValue::Text("Observation tower in Seattle Center".into()),
            )
            .add_field("category", DataValue::Text("tower".into()))
            .add_field("location", DataValue::Geo(47.6205, -122.3493))
            .add_field("city", DataValue::Text("Seattle".into()))
            .add_field("id", DataValue::Text("loc007".into()))
            .build(),
        Document::builder()
            .add_field("name", DataValue::Text("Pike Place Market".into()))
            .add_field(
                "description",
                DataValue::Text("Public market overlooking Elliott Bay".into()),
            )
            .add_field("category", DataValue::Text("market".into()))
            .add_field("location", DataValue::Geo(47.6101, -122.3421))
            .add_field("city", DataValue::Text("Seattle".into()))
            .add_field("id", DataValue::Text("loc008".into()))
            .build(),
    ];

    println!("Adding {} documents to the index...", documents.len());
    for (i, doc) in documents.into_iter().enumerate() {
        lexical_engine.upsert_document((i + 1) as u64, doc)?;
    }

    // Commit changes to engine
    lexical_engine.commit()?;

    println!("\n=== GeoQuery Examples ===\n");

    // Example 1: Find locations within radius of Times Square, NYC
    println!("1. Locations within 5km of Times Square (40.7580° N, 73.9855° W):");
    let query = GeoQuery::within_radius("location", 40.7580, -73.9855, 5.0)?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("name")
                && let DataValue::Text(name) = field
            {
                println!("      Name: {name}");
            }
            if let Some(field) = doc.get_field("city")
                && let DataValue::Text(city) = field
            {
                println!("      City: {city}");
            }
        }
    }

    // Example 2: Find locations within radius of downtown San Francisco
    println!("\n2. Locations within 10km of downtown San Francisco (37.7749° N, 122.4194° W):");
    let query = GeoQuery::within_radius("location", 37.7749, -122.4194, 10.0)?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("name")
                && let DataValue::Text(name) = field
            {
                println!("      Name: {name}");
            }
            if let Some(field) = doc.get_field("description")
                && let DataValue::Text(description) = field
            {
                println!("      Description: {description}");
            }
        }
    }

    // Example 3: Find locations within a bounding box (Los Angeles area)
    println!("\n3. Locations within bounding box of Los Angeles area:");
    println!("   (33.9° N, 118.6° W) to (34.3° N, 118.1° W)");
    let query = GeoQuery::within_bounding_box("location", 33.9, -118.6, 34.3, -118.1)?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("name")
                && let DataValue::Text(name) = field
            {
                println!("      Name: {name}");
            }
            if let Some(field) = doc.get_field("category")
                && let DataValue::Text(category) = field
            {
                println!("      Category: {category}");
            }
        }
    }

    // Example 4: Find locations within a large radius to include multiple cities
    println!("\n4. All West Coast locations within 1000km of San Francisco:");
    let query = GeoQuery::within_radius("location", 37.7749, -122.4194, 1000.0)?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("name")
                && let DataValue::Text(name) = field
            {
                println!("      Name: {name}");
            }
            if let Some(field) = doc.get_field("city")
                && let DataValue::Text(city) = field
            {
                println!("      City: {city}");
            }
        }
    }

    // Example 5: Find locations within radius of Seattle
    println!("\n5. Locations within 2km of downtown Seattle (47.6062° N, 122.3321° W):");
    let query = GeoQuery::within_radius("location", 47.6062, -122.3321, 2.0)?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("name")
                && let DataValue::Text(name) = field
            {
                println!("      Name: {name}");
            }
            if let Some(field) = doc.get_field("description")
                && let DataValue::Text(description) = field
            {
                println!("      Description: {description}");
            }
        }
    }

    // Example 6: Find locations within a tight radius (should find few/no results)
    println!("\n6. Locations within 1km of a specific point in the ocean:");
    let query = GeoQuery::within_radius("location", 36.0, -125.0, 1.0)?; // Pacific Ocean
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);

    // Example 7: Bounding box covering the entire continental US (wide search)
    println!("\n7. All locations within US continental bounding box:");
    println!("   (25° N, 125° W) to (49° N, 66° W)");
    let query = GeoQuery::within_bounding_box("location", 25.0, -125.0, 49.0, -66.0)?;
    let request = LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>).load_documents(true);
    let results = lexical_engine.search(request)?;

    println!("   Found {} results", results.total_hits);
    for (i, hit) in results.hits.iter().enumerate() {
        println!(
            "   {}. Score: {:.4}, Doc ID: {}",
            i + 1,
            hit.score,
            hit.doc_id
        );
        if let Some(doc) = &hit.document {
            if let Some(field) = doc.get_field("name")
                && let DataValue::Text(name) = field
            {
                println!("      Name: {name}");
            }
            if let Some(field) = doc.get_field("city")
                && let DataValue::Text(city) = field
            {
                println!("      City: {city}");
            }
        }
    }

    // Example 8: Count locations within a specific area
    println!("\n8. Counting locations within 50km of Los Angeles center:");
    let query = GeoQuery::within_radius("location", 34.0522, -118.2437, 50.0)?;
    let count =
        lexical_engine.count(LexicalSearchRequest::new(Box::new(query) as Box<dyn Query>))?;
    println!("   Count: {count} locations");

    lexical_engine.close()?;
    println!("\nGeoQuery example completed successfully!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_geo_query_example() {
        let result = main().await;
        assert!(result.is_ok());
    }
}
