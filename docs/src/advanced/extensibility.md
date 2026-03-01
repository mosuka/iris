# Extensibility

Laurus uses trait-based abstractions for its core components. You can implement these traits to provide custom analyzers, embedders, and storage backends.

## Custom Analyzer

Implement the `Analyzer` trait to create a custom text analysis pipeline:

```rust
use laurus::analysis::analyzer::analyzer::Analyzer;
use laurus::analysis::token::{Token, TokenStream};
use laurus::Result;

#[derive(Debug)]
struct ReverseAnalyzer;

impl Analyzer for ReverseAnalyzer {
    fn analyze(&self, text: &str) -> Result<TokenStream> {
        let tokens: Vec<Token> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, word)| Token {
                text: word.chars().rev().collect(),
                position: i,
                ..Default::default()
            })
            .collect();
        Ok(Box::new(tokens.into_iter()))
    }

    fn name(&self) -> &str {
        "reverse"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### Required Methods

| Method | Description |
| :--- | :--- |
| `analyze(&self, text: &str) -> Result<TokenStream>` | Process text into a stream of tokens |
| `name(&self) -> &str` | Return a unique identifier for this analyzer |
| `as_any(&self) -> &dyn Any` | Enable downcasting to the concrete type |

### Using a Custom Analyzer

Pass your analyzer to `EngineBuilder`:

```rust
use std::sync::Arc;

let analyzer = Arc::new(ReverseAnalyzer);
let engine = Engine::builder(storage, schema)
    .analyzer(analyzer)
    .build()
    .await?;
```

For per-field analyzers, wrap with `PerFieldAnalyzer`:

```rust
use laurus::analysis::analyzer::per_field::PerFieldAnalyzer;
use laurus::analysis::analyzer::standard::StandardAnalyzer;

let mut per_field = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new()?));
per_field.add_analyzer("custom_field", Arc::new(ReverseAnalyzer));

let engine = Engine::builder(storage, schema)
    .analyzer(Arc::new(per_field))
    .build()
    .await?;
```

## Custom Embedder

Implement the `Embedder` trait to integrate your own vector embedding model:

```rust
use async_trait::async_trait;
use laurus::embedding::embedder::{Embedder, EmbedInput, EmbedInputType};
use laurus::vector::core::vector::Vector;
use laurus::{LaurusError, Result};

#[derive(Debug)]
struct MyEmbedder {
    dimension: usize,
}

#[async_trait]
impl Embedder for MyEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(text) => {
                // Your embedding logic here
                let vector = vec![0.0f32; self.dimension];
                Ok(Vector::new(vector))
            }
            _ => Err(LaurusError::invalid_argument(
                "this embedder only supports text input",
            )),
        }
    }

    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        vec![EmbedInputType::Text]
    }

    fn name(&self) -> &str {
        "my-embedder"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### Required Methods

| Method | Description |
| :--- | :--- |
| `async embed(&self, input: &EmbedInput) -> Result<Vector>` | Generate an embedding vector for the given input |
| `supported_input_types(&self) -> Vec<EmbedInputType>` | Declare supported input types (`Text`, `Image`) |
| `as_any(&self) -> &dyn Any` | Enable downcasting |

### Optional Methods

| Method | Default | Description |
| :--- | :--- | :--- |
| `async embed_batch(&self, inputs) -> Result<Vec<Vector>>` | Sequential calls to `embed` | Override for batch optimization |
| `name(&self) -> &str` | `"unknown"` | Identifier for logging |
| `supports(&self, input_type) -> bool` | Checks `supported_input_types` | Input type support check |
| `supports_text() -> bool` | Checks for `Text` | Text support shorthand |
| `supports_image() -> bool` | Checks for `Image` | Image support shorthand |
| `is_multimodal() -> bool` | Both text and image | Multimodal check |

### Using a Custom Embedder

```rust
let embedder = Arc::new(MyEmbedder { dimension: 384 });
let engine = Engine::builder(storage, schema)
    .embedder(embedder)
    .build()
    .await?;
```

For per-field embedders, wrap with `PerFieldEmbedder`:

```rust
use laurus::embedding::per_field::PerFieldEmbedder;

let mut per_field = PerFieldEmbedder::new(Arc::new(MyEmbedder { dimension: 384 }));
per_field.add_embedder("image_vec", Arc::new(ClipEmbedder::new()?));

let engine = Engine::builder(storage, schema)
    .embedder(Arc::new(per_field))
    .build()
    .await?;
```

## Custom Storage

Implement the `Storage` trait to add a new storage backend:

```rust
use laurus::storage::{Storage, StorageInput, StorageOutput, LoadingMode, FileMetadata};
use laurus::Result;

#[derive(Debug)]
struct S3Storage {
    bucket: String,
    prefix: String,
}

impl Storage for S3Storage {
    fn loading_mode(&self) -> LoadingMode {
        LoadingMode::Eager  // S3 requires full download
    }

    fn open_input(&self, name: &str) -> Result<Box<dyn StorageInput>> {
        // Download from S3 and return a reader
        todo!()
    }

    fn create_output(&self, name: &str) -> Result<Box<dyn StorageOutput>> {
        // Create an upload stream to S3
        todo!()
    }

    fn create_output_append(&self, name: &str) -> Result<Box<dyn StorageOutput>> {
        todo!()
    }

    fn file_exists(&self, name: &str) -> bool {
        todo!()
    }

    fn delete_file(&self, name: &str) -> Result<()> {
        todo!()
    }

    fn list_files(&self) -> Result<Vec<String>> {
        todo!()
    }

    fn file_size(&self, name: &str) -> Result<u64> {
        todo!()
    }

    fn metadata(&self, name: &str) -> Result<FileMetadata> {
        todo!()
    }

    fn rename_file(&self, old_name: &str, new_name: &str) -> Result<()> {
        todo!()
    }

    fn create_temp_output(&self, prefix: &str) -> Result<(String, Box<dyn StorageOutput>)> {
        todo!()
    }

    fn sync(&self) -> Result<()> {
        todo!()
    }

    fn close(&mut self) -> Result<()> {
        todo!()
    }
}
```

### Required Methods

| Method | Description |
| :--- | :--- |
| `open_input(name) -> Result<Box<dyn StorageInput>>` | Open a file for reading |
| `create_output(name) -> Result<Box<dyn StorageOutput>>` | Create a file for writing |
| `create_output_append(name) -> Result<Box<dyn StorageOutput>>` | Open a file for appending |
| `file_exists(name) -> bool` | Check if a file exists |
| `delete_file(name) -> Result<()>` | Delete a file |
| `list_files() -> Result<Vec<String>>` | List all files |
| `file_size(name) -> Result<u64>` | Get file size in bytes |
| `metadata(name) -> Result<FileMetadata>` | Get file metadata |
| `rename_file(old, new) -> Result<()>` | Rename a file |
| `create_temp_output(prefix) -> Result<(String, Box<dyn StorageOutput>)>` | Create a temporary file |
| `sync() -> Result<()>` | Flush all pending writes |
| `close(&mut self) -> Result<()>` | Close storage and release resources |

### Optional Methods

| Method | Default | Description |
| :--- | :--- | :--- |
| `loading_mode() -> LoadingMode` | `LoadingMode::Eager` | Preferred data loading mode |

## Thread Safety

All three traits require `Send + Sync`. This means your implementations must be safe to share across threads. Use `Arc<Mutex<_>>` or lock-free data structures for shared mutable state.

## Next Steps

- [Error Handling](error_handling.md) — handle errors in custom implementations
- [Text Analysis](../concepts/analysis.md) — built-in analyzers and pipeline components
- [Embeddings](../concepts/embedding.md) — built-in embedder options
- [Storage](../concepts/storage.md) — built-in storage backends
