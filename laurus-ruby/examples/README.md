# laurus-ruby Examples

This directory contains runnable examples for the
`laurus` Ruby bindings.

## Prerequisites

- Rust toolchain (`rustup` — <https://rustup.rs>)
- Ruby 3.2+ (<https://www.ruby-lang.org>)
- Bundler

## Setup

All examples must be run from the `laurus-ruby/` directory
after building the native extension.

```bash
cd laurus-ruby

# Install dependencies
bundle install

# Build the native extension
bundle exec rake compile
```

## Examples

### Basic examples (no extra dependencies)

Build once, then run any of the examples below:

| Example | Description |
| :--- | :--- |
| [quickstart.rb](quickstart.rb) | Minimal full-text search: index, search, update |
| [lexical_search.rb](lexical_search.rb) | All lexical query types: Term, Phrase, Fuzzy, Wildcard, NumericRange, Geo, Boolean, Span |
| [synonym_graph_filter.rb](synonym_graph_filter.rb) | Synonym expansion in the analysis pipeline |

```bash
ruby -Ilib examples/quickstart.rb
ruby -Ilib examples/lexical_search.rb
ruby -Ilib examples/synonym_graph_filter.rb
```

---

### Vector search — built-in embedder

Uses laurus's built-in `CandleBert` embedder (via [Candle](https://github.com/huggingface/candle)).
Text is embedded automatically by the Rust engine at index and query time — **no external
embedding library is needed**.

Build with the `embeddings-candle` feature:

```bash
bundle exec rake compile  # ensure Cargo.toml includes embeddings-candle feature
```

| Example | Description |
| :--- | :--- |
| [vector_search.rb](vector_search.rb) | Semantic similarity search with laurus's built-in BERT embedder |
| [hybrid_search.rb](hybrid_search.rb) | Hybrid lexical + vector search with RRF and WeightedSum fusion |

```bash
ruby -Ilib examples/vector_search.rb
ruby -Ilib examples/hybrid_search.rb
```

> **Note:** The first run downloads the model weights from Hugging Face Hub
> (`sentence-transformers/all-MiniLM-L6-v2`, ~90 MB). Subsequent runs use
> the local cache.

---

## Release build

For production performance, build with the release profile:

```bash
bundle exec rake compile:release
```
