# Quick Start

## Basic Usage (In-memory)

```javascript
import init, { Index, Schema } from 'laurus-wasm';

// Initialize the WASM module
await init();

// Define a schema
const schema = new Schema();
schema.addTextField("title");
schema.addTextField("body");
schema.setDefaultFields(["title", "body"]);

// Create an in-memory index
const index = await Index.create(schema);

// Add documents
await index.putDocument("doc1", {
  title: "Introduction to Rust",
  body: "Rust is a systems programming language"
});
await index.putDocument("doc2", {
  title: "WebAssembly Guide",
  body: "WASM enables near-native performance in the browser"
});
await index.commit();

// Search
const results = await index.search("rust programming");
for (const result of results) {
  console.log(`${result.id}: ${result.score}`);
  console.log(result.document);
}
```

## Persistent Storage (OPFS)

```javascript
import init, { Index, Schema } from 'laurus-wasm';

await init();

const schema = new Schema();
schema.addTextField("title");
schema.addTextField("body");

// Open a persistent index (data survives page reloads)
const index = await Index.open("my-index", schema);

// Add documents
await index.putDocument("doc1", {
  title: "Hello",
  body: "World"
});

// commit() persists to OPFS automatically
await index.commit();

// On next page load, Index.open("my-index") will restore the data
```

## Vector Search

```javascript
import init, { Index, Schema } from 'laurus-wasm';

await init();

const schema = new Schema();
schema.addTextField("title");
schema.addHnswField("embedding", 3); // 3-dimensional vectors

const index = await Index.create(schema);

await index.putDocument("doc1", {
  title: "Rust",
  embedding: [1.0, 0.0, 0.0]
});
await index.putDocument("doc2", {
  title: "Python",
  embedding: [0.0, 1.0, 0.0]
});
await index.commit();

// Search by vector similarity
const results = await index.searchVector("embedding", [0.9, 0.1, 0.0]);
console.log(results[0].document.title); // "Rust"
```

## Usage with Bundlers

### Vite

```javascript
// vite.config.js
import wasm from 'vite-plugin-wasm';

export default {
  plugins: [wasm()]
};
```

### Webpack 5

Webpack 5 supports WASM natively with `asyncWebAssembly`:

```javascript
// webpack.config.js
module.exports = {
  experiments: {
    asyncWebAssembly: true
  }
};
```
