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
| [synonym-graph-filter example](../README.md#text-analysis) | Synonym expansion (see README) |

```bash
node examples/quickstart.mjs
node examples/lexical-search.mjs
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

## Release build

For production performance, build with the release profile:

```bash
npm run build
```
