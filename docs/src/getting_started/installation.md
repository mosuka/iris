# Installation

Add iris to your `Cargo.toml`:

```toml
[dependencies]
iris = "0.1.0"
```

## Feature Flags

Iris provides several feature flags to enable optional functionalities, particularly for embedding generation:

- `embeddings-candle`: Enables Hugging Face Candle integration for running models locally.
- `embeddings-openai`: Enables OpenAI API integration.
- `embeddings-multimodal`: Enables multimodal embedding support (image + text) via Candle.
- `embeddings-all`: Enables all embedding features.

```toml
# Example: interacting with OpenAI
iris = { version = "0.1.0", features = ["embeddings-openai"] }
```
