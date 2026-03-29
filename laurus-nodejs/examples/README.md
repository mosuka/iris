# laurus-nodejs Examples

This directory contains runnable examples for the
`laurus-nodejs` Node.js bindings.

## Prerequisites

- Rust toolchain (`rustup` — <https://rustup.rs>)
- Node.js 18+ (<https://nodejs.org>)
- npm

## Setup

All examples must be run from the `laurus-nodejs/` directory
after building the native extension.

```bash
cd laurus-nodejs

# Install dependencies
npm install

# Build the native module (debug mode for faster builds)
npm run build:debug
```

## Examples

### Basic examples (no extra dependencies)

Build once, then run any of the examples below:

| Example | Description |
| :--- | :--- |
| [quickstart.mjs](quickstart.mjs) | Minimal search: index, search, stats |
| [lexical-search.mjs](lexical-search.mjs) | Lexical query types: Term, Phrase, Fuzzy, Wildcard, DSL |
| [synonym-graph-filter.mjs](synonym-graph-filter.mjs) | Synonym expansion with SynonymDictionary, WhitespaceTokenizer, and SynonymGraphFilter |

```bash
node examples/quickstart.mjs
node examples/lexical-search.mjs
node examples/synonym-graph-filter.mjs
```

---

### Vector search — pre-computed embeddings

Uses pre-computed embedding vectors passed directly
to the HNSW index. No external embedding library is needed.

| Example | Description |
| :--- | :--- |
| [vector-search.mjs](vector-search.mjs) | Similarity search with pre-computed vectors |

```bash
node examples/vector-search.mjs
```

---

### Hybrid search

Combines lexical and vector search using RRF
(Reciprocal Rank Fusion) or WeightedSum fusion.

| Example | Description |
| :--- | :--- |
| [hybrid-search.mjs](hybrid-search.mjs) | Hybrid lexical + vector with RRF and WeightedSum |

```bash
node examples/hybrid-search.mjs
```

---

### External embedder

Vector and hybrid search with embeddings produced outside
laurus (e.g. `@xenova/transformers`). Falls back to random
vectors if the library is not installed.

| Example | Description |
| :--- | :--- |
| [external-embedder.mjs](external-embedder.mjs) | Pre-computed vector search with external embedding library |

```bash
npm install @xenova/transformers   # optional
node examples/external-embedder.mjs
```

---

### OpenAI embeddings

Uses the `openai` npm package to produce real embeddings
via the OpenAI API. Requires an API key.

| Example | Description |
| :--- | :--- |
| [search-with-openai.mjs](search-with-openai.mjs) | Vector and hybrid search with OpenAI embeddings |

```bash
npm install openai
export OPENAI_API_KEY=your-api-key-here
node examples/search-with-openai.mjs
```

---

### Multimodal search

Stores raw image bytes in a `bytes` field alongside CLIP
embeddings for cross-modal (text-to-image, image-to-text)
search.

| Example | Description |
| :--- | :--- |
| [multimodal-search.mjs](multimodal-search.mjs) | Bytes field + CLIP embeddings for multimodal search |

```bash
npm install @xenova/transformers   # optional
node examples/multimodal-search.mjs
```

---

## Release build

For production performance, build with the release profile:

```bash
npm run build
```
