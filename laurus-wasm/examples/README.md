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

- Creates an in-memory search index with `title` and `body` fields
- Loads 5 sample documents about Rust, WASM, and search
- Provides a search box with DSL query support (e.g. `"title:rust"`,
  `"browser programming"`)
- Allows adding new documents interactively
- Shows search results with relevance scores
- Logs all operations in the console panel
