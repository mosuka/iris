# Development

## Prerequisites

- [Rust](https://rustup.rs/) (stable, with `wasm32-unknown-unknown` target)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) (for testing and npm publish)

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

## Build

```bash
cd laurus-wasm

# Debug build (faster compilation)
wasm-pack build --target web --dev

# Release build (optimized)
wasm-pack build --target web --release

# For bundler targets (webpack, vite, etc.)
wasm-pack build --target bundler --release
```

## Project Structure

```text
laurus-wasm/
├── Cargo.toml          # Rust dependencies (wasm-bindgen, laurus core)
├── package.json        # npm package metadata
├── src/
│   ├── lib.rs          # Module declarations
│   ├── index.rs        # Index class (CRUD + search)
│   ├── schema.rs       # Schema builder
│   ├── search.rs       # SearchRequest / SearchResult
│   ├── query.rs        # Query type definitions
│   ├── convert.rs      # JsValue ↔ Document conversion
│   ├── analysis.rs     # Tokenizer / Filter wrappers
│   ├── errors.rs       # LaurusError → JsValue conversion
│   └── storage.rs      # OPFS persistence layer
└── js/
    └── opfs_bridge.js  # JS glue for Origin Private File System
```

## Architecture Notes

### Storage Strategy

laurus-wasm uses a two-layer storage approach:

1. **MemoryStorage** (runtime) -- All read/write operations go
   through Laurus's in-memory storage, which satisfies the
   `Storage` trait's `Send + Sync` requirement.

2. **OPFS** (persistence) -- On `commit()`, the entire
   MemoryStorage state is serialized to OPFS files. On
   `Index.open()`, OPFS files are loaded back into MemoryStorage.

This avoids the `Send + Sync` incompatibility of JS handles
while keeping the core engine unchanged.

### Feature Flags

The `laurus` core uses feature flags to support WASM:

```toml
# laurus-wasm depends on laurus without default features
laurus = { workspace = true, default-features = false }
```

This excludes native-only dependencies (tokio/full, rayon,
memmap2, etc.) and uses `#[cfg(target_arch = "wasm32")]`
fallbacks for parallelism.

## Testing

```bash
# Build check
cargo build -p laurus-wasm --target wasm32-unknown-unknown

# Clippy
cargo clippy -p laurus-wasm --target wasm32-unknown-unknown -- -D warnings
```

Browser tests can be run with `wasm-pack test`:

```bash
wasm-pack test --headless --chrome
```
