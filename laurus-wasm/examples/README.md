# Laurus WASM Demo

A single-page application demonstrating full-text search in the browser
using laurus-wasm.

## How to Run

1. Build the WASM package:

   ```bash
   cd laurus-wasm
   wasm-pack build --target web --dev
   ```

2. Serve the files with any HTTP server (WASM requires HTTP, not `file://`):

   ```bash
   # Python
   python3 -m http.server 8080

   # Node.js (npx)
   npx serve .
   ```

3. Open <http://localhost:8080/examples/> in your browser.

## What the Demo Does

- Creates an OPFS-persistent search index with `title` and `body` fields
  (data survives page reloads)
- Seeds 8 sample documents on first visit; skips seeding when existing
  data is loaded from OPFS
- Uses Transformers.js (all-MiniLM-L6-v2) for real 384-dim semantic
  embeddings via the callback embedder
- Provides a search box with unified query DSL support:
  - Lexical: `rust`, `title:wasm`, `"memory safety"`
  - Vector: `embedding:"how to make code faster"`, `embedding:python`
  - Hybrid: `rust embedding:"systems programming"`
- Allows adding new documents interactively
- Shows search results with relevance scores
- Logs all operations in the console panel
