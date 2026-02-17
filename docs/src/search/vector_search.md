# Vector Search

Vector search finds documents by semantic similarity. Instead of matching keywords, it compares the meaning of the query against document embeddings in vector space.

## Basic Usage

### Builder API

```rust
use iris::SearchRequestBuilder;
use iris::vector::VectorSearchRequestBuilder;

let request = SearchRequestBuilder::new()
    .vector_search_request(
        VectorSearchRequestBuilder::new()
            .add_text("embedding", "systems programming language")
            .limit(10)
            .build()
    )
    .build();

let results = engine.search(request).await?;
```

The `add_text()` method stores the text as a query payload. At search time, the engine embeds it using the configured embedder and then searches the vector index.

### Query DSL

```rust
use iris::vector::VectorQueryParser;

let parser = VectorQueryParser::new(embedder.clone())
    .with_default_field("embedding");

let request = parser.parse(r#"embedding:~"systems programming""#).await?;
```

## VectorSearchRequestBuilder

The builder API provides fine-grained control:

```rust
use iris::vector::VectorSearchRequestBuilder;
use iris::vector::store::request::QueryVector;

let request = VectorSearchRequestBuilder::new()
    // Text query (will be embedded at search time)
    .add_text("text_vec", "machine learning")

    // Or use a pre-computed vector directly
    .add_vector("embedding", vec![0.1, 0.2, 0.3, /* ... */])

    // Search parameters
    .limit(20)

    .build();
```

### Methods

| Method | Description |
| :--- | :--- |
| `add_text(field, text)` | Add a text query for a specific field (embedded at search time) |
| `add_vector(field, vector)` | Add a pre-computed query vector for a specific field |
| `limit(n)` | Maximum number of results |

## Multi-Field Vector Search

You can search across multiple vector fields in a single request:

```rust
let request = VectorSearchRequestBuilder::new()
    .add_text("text_vec", "cute kitten")
    .add_text("image_vec", "fluffy cat")
    .build();
```

Each clause produces a vector that is searched against its respective field. Results are combined using the configured score mode.

### Score Modes

| Mode | Description |
| :--- | :--- |
| `WeightedSum` (default) | Sum of (similarity * weight) across all clauses |
| `MaxSim` | Maximum similarity score across clauses |
| `LateInteraction` | ColBERT-style late interaction scoring |

### Weights

Use the `^` boost syntax in DSL or `weight` in `QueryVector` to adjust how much each field contributes:

```
text_vec:~"cute kitten"^1.0 image_vec:~"fluffy cat"^0.5
```

This means text similarity counts twice as much as image similarity.

## Filtered Vector Search

You can apply lexical filters to narrow the vector search results:

```rust
use iris::{SearchRequestBuilder, LexicalSearchRequest};
use iris::lexical::TermQuery;
use iris::vector::VectorSearchRequestBuilder;

// Vector search with a category filter
let request = SearchRequestBuilder::new()
    .vector_search_request(
        VectorSearchRequestBuilder::new()
            .add_text("embedding", "machine learning")
            .build()
    )
    .filter_query(Box::new(TermQuery::new("category", "tutorial")))
    .limit(10)
    .build();

let results = engine.search(request).await?;
```

The filter query runs first on the lexical index to identify allowed document IDs, then the vector search is restricted to those IDs.

### Filter with Numeric Range

```rust
use iris::lexical::NumericRangeQuery;
use iris::lexical::core::field::NumericType;

let request = SearchRequestBuilder::new()
    .vector_search_request(
        VectorSearchRequestBuilder::new()
            .add_text("embedding", "type systems")
            .build()
    )
    .filter_query(Box::new(NumericRangeQuery::new(
        "year", NumericType::Integer,
        Some(2020.0), Some(2024.0), true, true
    )))
    .limit(10)
    .build();
```

## Distance Metrics

The distance metric is configured per field in the schema (see [Vector Indexing](../indexing/vector_indexing.md)):

| Metric | Description | Lower = More Similar |
| :--- | :--- | :--- |
| **Cosine** | 1 - cosine similarity | Yes |
| **Euclidean** | L2 distance | Yes |
| **Manhattan** | L1 distance | Yes |
| **DotProduct** | Negative inner product | Yes |
| **Angular** | Angular distance | Yes |

## Code Example: Complete Vector Search

```rust
use std::sync::Arc;
use iris::{Document, Engine, Schema, SearchRequestBuilder, PerFieldEmbedder};
use iris::lexical::TextOption;
use iris::vector::HnswOption;
use iris::vector::VectorSearchRequestBuilder;
use iris::storage::memory::MemoryStorage;

#[tokio::main]
async fn main() -> iris::Result<()> {
    let storage = Arc::new(MemoryStorage::new(Default::default()));

    let schema = Schema::builder()
        .add_text_field("title", TextOption::default())
        .add_hnsw_field("text_vec", HnswOption {
            dimension: 384,
            ..Default::default()
        })
        .build();

    // Set up per-field embedder
    let embedder = Arc::new(my_embedder);
    let mut pfe = PerFieldEmbedder::new(embedder.clone());
    pfe.add_embedder("text_vec", embedder.clone());

    let engine = Engine::builder(storage, schema)
        .embedder(Arc::new(pfe))
        .build()
        .await?;

    // Index documents (text in vector field is auto-embedded)
    engine.add_document("doc-1", Document::builder()
        .add_text("title", "Rust Programming")
        .add_text("text_vec", "Rust is a systems programming language.")
        .build()
    ).await?;
    engine.commit().await?;

    // Search by semantic similarity
    let results = engine.search(
        SearchRequestBuilder::new()
            .vector_search_request(
                VectorSearchRequestBuilder::new()
                    .add_text("text_vec", "systems language")
                    .build()
            )
            .limit(5)
            .build()
    ).await?;

    for r in &results {
        println!("{}: score={:.4}", r.id, r.score);
    }

    Ok(())
}
```

## Next Steps

- Combine with keyword search: [Hybrid Search](hybrid_search.md)
- DSL syntax for vector queries: [Query DSL](../advanced/query_dsl.md)
