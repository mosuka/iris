//! Unified Search Example
//!
//! This example demonstrates the flexibility of the unified Engine API, showing how to:
//! 1. Configure a schema where fields can be indexed lexically or vectorially.
//! 2. Perform purely Lexical Search (keywords only).
//! 3. Perform purely Vector Search (semantic only).
//! 4. Perform Hybrid Search (combining both) with different fusion strategies.
//!
//! This replaces the separate lexical_search.rs, vector_search.rs, and hybrid_search.rs examples
//! to show how everything works together in one cohesive system.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use iris::Document;
use iris::Engine;
use iris::Result;
use iris::analysis::analyzer::keyword::KeywordAnalyzer;
use iris::analysis::analyzer::per_field::PerFieldAnalyzer;
use iris::analysis::analyzer::standard::StandardAnalyzer;
use iris::lexical::{FieldOption as LexicalFieldOption, TextOption};
use iris::{EmbedInput, EmbedInputType, Embedder, PerFieldEmbedder};
use iris::{FieldOption, Schema};
use iris::{FusionAlgorithm, SearchRequestBuilder};

use iris::lexical::TermQuery;
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::Vector;
use iris::vector::VectorSearchRequestBuilder;
use iris::vector::{FieldOption as VectorOption, FlatOption};

// --- Mock Embedder Setup ---
// Two embedders that deterministically convert keywords into vectors.
// FruitEmbedder focuses on fruit-related terms; ConceptEmbedder on abstract concepts.

#[derive(Debug, Clone)]
struct FruitEmbedder;

#[async_trait]
impl Embedder for FruitEmbedder {
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
                    Ok(Vector::new(vec![0.0, 0.0, 0.0, 0.0]))
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
        "FruitEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
struct ConceptEmbedder;

#[async_trait]
impl Embedder for ConceptEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(t) => {
                let t = t.to_lowercase();
                if t.contains("tech") {
                    Ok(Vector::new(vec![0.0, 0.0, 0.0, 1.0]))
                } else if t.contains("nature") {
                    Ok(Vector::new(vec![0.0, 0.0, 1.0, 0.0]))
                } else {
                    Ok(Vector::new(vec![0.0, 0.0, 0.0, 0.0]))
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
        "ConceptEmbedder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn main() -> Result<()> {
    println!("=== Iris Unified Search Example ===\n");

    // 1. Setup Storage (In-memory for this example)
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config)?;

    // 2. Configure the Engine
    // We define a flexible schema:
    // - "title":       Lexical Only (Keyword search is best for precise title matching)
    // - "content":     Lexical (For keyword search on content)
    // - "content_vec": Vector (For semantic search on content)
    // - "embedding":   Vector Only (Hidden semantic features)
    let schema = Schema::builder()
        // Field 1: Lexical content field
        .add_field(
            "content",
            FieldOption::Lexical(LexicalFieldOption::Text(TextOption::default())),
        )
        // Field 2: Vector content field (for semantic search)
        .add_field(
            "content_vec",
            FieldOption::Vector(VectorOption::Flat(FlatOption {
                dimension: 4,
                ..Default::default()
            })),
        )
        // Field 3: Lexical Only
        .add_lexical_field("title", LexicalFieldOption::Text(TextOption::default()))
        // Field 4: Vector Only
        .add_vector_field(
            "embedding",
            VectorOption::Flat(FlatOption {
                dimension: 4,
                ..Default::default()
            }),
        )
        .build();

    // Setup PerFieldAnalyzer:
    // - "title" uses KeywordAnalyzer (no tokenization, exact match)
    // - All other fields use StandardAnalyzer (tokenization + lowercasing)
    let std_analyzer = Arc::new(StandardAnalyzer::default());
    let kw_analyzer = Arc::new(KeywordAnalyzer::new());

    let mut per_field_analyzer = PerFieldAnalyzer::new(std_analyzer);
    per_field_analyzer.add_analyzer("title", kw_analyzer);

    // Setup PerFieldEmbedder:
    // - "content_vec" uses FruitEmbedder (fruit-focused semantic space)
    // - "embedding"   uses ConceptEmbedder (concept-focused semantic space)
    // - Default fallback is FruitEmbedder
    let mut per_field_embedder = PerFieldEmbedder::new(Arc::new(FruitEmbedder));
    per_field_embedder.add_embedder("content_vec", Arc::new(FruitEmbedder));
    per_field_embedder.add_embedder("embedding", Arc::new(ConceptEmbedder));

    let engine = Engine::builder(storage, schema)
        .analyzer(Arc::new(per_field_analyzer))
        .embedder(Arc::new(per_field_embedder))
        .build()?;

    // 3. Index Data
    let docs = vec![
        // ID, Title, Content
        (
            "doc1",
            "Fruit Guide",
            "An apple a day keeps the doctor away.",
        ),
        (
            "doc2",
            "Tech Daily",
            "The latest tech news about silicon chips.",
        ),
        (
            "doc3",
            "Orange Juice",
            "Freshly squeezed orange juice is great.",
        ),
        (
            "doc4",
            "Banana Split",
            "Dessert made with banana and ice cream.",
        ),
        ("doc5", "Hybrid Theory", "Technology and nature combined."),
    ];

    println!("Indexing {} documents...", docs.len());
    for (id, title, content) in docs {
        let doc = Document::new()
            .add_text("title", title)
            .add_text("content", content) // Lexical field
            .add_text("content_vec", content) // Vector field (auto-embedded)
            .add_text("embedding", content); // Separate vector field
        // Note: 'content_vec' and 'embedding' text will be automatically embedded
        // because they have vector configs and we registered a global embedder.

        engine.put_document(id, doc)?;
    }
    engine.commit()?;

    // 4. Case A: Lexical Search Only (Keyword Match)
    // Scenario: User types a specific word and expects exact matches.
    println!("\n[Case A] Lexical Search Only (Query: 'apple')");
    let request_lexical = SearchRequestBuilder::new()
        // We only provide .with_lexical(), so vector index is ignored (even though available)
        .with_lexical(Box::new(TermQuery::new("content", "apple")))
        .limit(3)
        .build();

    let results = engine.search(request_lexical)?;
    print_results(&results);

    // 5. Case B: Vector Search Only (Semantic Match)
    // Scenario: User searches for 'fruit' (concept), but 'apple' doc doesn't contain the word 'fruit'.
    // Our MockEmbedder maps 'apple' -> vector [1,0,0,0].
    println!("\n[Case B] Vector Search Only (Query: 'apple' semantically)");
    let request_vector = SearchRequestBuilder::new()
        // We only provide .with_vector(), so lexical index is ignored
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("content_vec", "apple") // Embeds 'apple' to search vector space
                .build(),
        )
        .limit(3)
        .build();

    let results = engine.search(request_vector)?;
    print_results(&results);

    // 6. Case C: Hybrid Search (RRF Fusion)
    // Scenario: Provide the best of both worlds.
    // Searching for "tech" (semantic) AND "news" (keyword).
    println!("\n[Case C] Hybrid Search (RRF Fusion)");
    println!("Query: Vector('tech') + Lexical('news')");

    let request_hybrid = SearchRequestBuilder::new()
        // Vector part: finds broad "tech" concepts (e.g., 'doc2' and 'doc5')
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("content_vec", "tech")
                .build(),
        )
        // Lexical part: specifically looks for the word "news" (only in 'doc2')
        .with_lexical(Box::new(TermQuery::new("content", "news")))
        // Fusion: RRF boosts documents that rank high in BOTH lists
        .fusion(FusionAlgorithm::RRF { k: 60.0 })
        .limit(3)
        .build();

    let results = engine.search(request_hybrid)?;
    print_results(&results);

    // 7. Case D: Hybrid Search (Weighted Sum)
    // Scenario: We trust Semantic search more (70%) than keyword search (30%).
    println!("\n[Case D] Hybrid Search (Weighted Sum: 70% Vector, 30% Lexical)");

    let request_weighted = SearchRequestBuilder::new()
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("content_vec", "orange")
                .build(),
        )
        .with_lexical(Box::new(TermQuery::new("content", "juice")))
        .fusion(FusionAlgorithm::WeightedSum {
            vector_weight: 0.7,
            lexical_weight: 0.3,
        })
        .limit(3)
        .build();

    let results = engine.search(request_weighted)?;
    print_results(&results);

    // 8. Case E: Search on "embedding" field (Vector Only)
    // Scenario: We explicitly search the "embedding" field, which is separate from "content".
    println!("\n[Case E] Vector Search on 'embedding' field (Query: 'tech')");
    let request_embedding = SearchRequestBuilder::new()
        .with_vector(
            VectorSearchRequestBuilder::new()
                .add_text("embedding", "tech")
                .build(),
        )
        .limit(3)
        .build();

    let results = engine.search(request_embedding)?;
    print_results(&results);

    Ok(())
}

// Helper to print results cleanly
fn print_results(results: &[iris::SearchResult]) {
    if results.is_empty() {
        println!("  (No results found)");
        return;
    }
    for (i, hit) in results.iter().enumerate() {
        let id = &hit.id;
        let title = hit
            .document
            .as_ref()
            .and_then(|doc| doc.fields.get("title"))
            .and_then(|v| v.as_text())
            .unwrap_or("No Title");
        println!("  {}. [{}] {} (Score: {:.4})", i + 1, id, title, hit.score);
    }
}
