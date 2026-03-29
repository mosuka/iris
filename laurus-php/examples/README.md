# laurus-php Examples

This directory contains runnable examples for the
`laurus` PHP bindings.

## Prerequisites

- Rust toolchain (`rustup` -- <https://rustup.rs>)
- PHP 8.1+ (<https://www.php.net>)
- Composer (<https://getcomposer.org>) (optional, for tests only)

## Setup

All examples must be run from the `laurus-php/` directory
after building the native extension.

```bash
cd laurus-php

# Build the PHP extension (release mode)
cargo build --release
```

## Examples

### Basic examples (no extra dependencies)

Build once, then run any of the examples below:

| Example | Description |
| :--- | :--- |
| [quickstart.php](quickstart.php) | Minimal full-text search: index, search, update |
| [lexical_search.php](lexical_search.php) | All lexical query types: Term, Phrase, Fuzzy, Wildcard, NumericRange, Geo, Boolean, Span |
| [synonym_graph_filter.php](synonym_graph_filter.php) | Synonym expansion in the analysis pipeline |

```bash
php -d extension=target/release/liblaurus_php.so examples/quickstart.php
php -d extension=target/release/liblaurus_php.so examples/lexical_search.php
php -d extension=target/release/liblaurus_php.so examples/synonym_graph_filter.php
```

> **Tip:** To avoid passing `-d extension=...` every time, add
> `extension=liblaurus_php.so` to your `php.ini` and copy the `.so` file
> to the PHP extensions directory (`php -i | grep extension_dir`).

---

### Vector search -- built-in embedder

Uses laurus's built-in `CandleBert` embedder (via [Candle](https://github.com/huggingface/candle)).
Text is embedded automatically by the Rust engine at index and query time -- **no external
embedding library is needed**.

Build with the `embeddings-candle` feature:

```bash
cargo build --release --features embeddings-candle
```

| Example | Description |
| :--- | :--- |
| [vector_search.php](vector_search.php) | Semantic similarity search with laurus's built-in BERT embedder |
| [hybrid_search.php](hybrid_search.php) | Hybrid lexical + vector search with RRF and WeightedSum fusion |
| [search_app.php](search_app.php) | Browser-based hybrid search UI with PHP built-in server |

```bash
php -d extension=target/release/liblaurus_php.so examples/vector_search.php
php -d extension=target/release/liblaurus_php.so examples/hybrid_search.php
```

The web-based search app launches a local server with Lexical / Vector / Hybrid mode switching:

```bash
php -d extension=target/release/liblaurus_php.so -S localhost:8080 examples/search_app.php
```

Then open <http://localhost:8080> in your browser.

> **Note:** The first run downloads the model weights from Hugging Face Hub
> (`sentence-transformers/all-MiniLM-L6-v2`, ~90 MB). Subsequent runs use
> the local cache.

---

### Vector search -- external embedder

Uses pre-computed vectors passed via `VectorQuery`. No embedder is registered
in the schema -- the caller manages embeddings externally.

Build without extra features (standard release build):

```bash
cargo build --release
```

| Example | Description |
| :--- | :--- |
| [external_embedder.php](external_embedder.php) | Pre-computed vector search with random fallback embeddings (no external dependencies) |
| [search_with_openai.php](search_with_openai.php) | Real vector search using the OpenAI Embeddings API via raw curl |
| [multimodal_search.php](multimodal_search.php) | Multimodal search: store image bytes + vector embeddings, query across text and images |

```bash
php -d extension=target/release/liblaurus_php.so examples/external_embedder.php
php -d extension=target/release/liblaurus_php.so examples/multimodal_search.php
```

The OpenAI example requires an API key:

```bash
export OPENAI_API_KEY=your-api-key-here
php -d extension=target/release/liblaurus_php.so examples/search_with_openai.php
```
