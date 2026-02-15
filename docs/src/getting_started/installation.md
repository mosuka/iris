# Installation

## Add Iris to Your Project

Add `iris` and `tokio` (async runtime) to your `Cargo.toml`:

```toml
[dependencies]
iris = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## Feature Flags

Iris ships with a minimal default feature set. Enable additional features as needed:

| Feature | Description | Use Case |
| :--- | :--- | :--- |
| *(default)* | Lexical search, in-memory storage, standard analyzer | Keyword search only |
| `embeddings-candle` | Local BERT embeddings via Hugging Face Candle | Vector search without external API |
| `embeddings-openai` | OpenAI API embeddings (text-embedding-3-small, etc.) | Cloud-based vector search |
| `embeddings-multimodal` | CLIP embeddings for text + image via Candle | Multimodal (text-to-image) search |
| `embeddings-all` | All embedding features above | Full embedding support |

### Examples

**Lexical search only** (no embeddings needed):

```toml
[dependencies]
iris = "0.1.0"
```

**Vector search with local model** (no API key required):

```toml
[dependencies]
iris = { version = "0.1.0", features = ["embeddings-candle"] }
```

**Vector search with OpenAI**:

```toml
[dependencies]
iris = { version = "0.1.0", features = ["embeddings-openai"] }
```

**Everything**:

```toml
[dependencies]
iris = { version = "0.1.0", features = ["embeddings-all"] }
```

## Verify Installation

Create a minimal program to verify that Iris compiles:

```rust
use iris::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Iris version: {}", iris::VERSION);
    Ok(())
}
```

```bash
cargo run
```

If you see the version printed, you are ready to proceed to the [Quick Start](quickstart.md).
