# Build & Test

## Prerequisites

- **Rust** 1.85 or later (edition 2024)
- **Cargo** (included with Rust)
- **protobuf compiler** (`protoc`) -- required for building `laurus-server`

## Building

```bash
# Build all crates
cargo build

# Build with specific features
cargo build --features embeddings-candle

# Build in release mode
cargo build --release
```

## Testing

```bash
# Run all tests
cargo test

# Run a specific test by name
cargo test <test_name>

# Run tests for a specific crate
cargo test -p laurus
cargo test -p laurus-cli
cargo test -p laurus-server
```

## Linting

```bash
# Run clippy with warnings as errors
cargo clippy -- -D warnings
```

## Formatting

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

## Documentation

### API Documentation

```bash
# Generate and open Rust API docs
cargo doc --no-deps --open
```

### mdBook Documentation

```bash
# Build the documentation site
mdbook build docs

# Start a local preview server (http://localhost:3000)
mdbook serve docs

# Lint markdown files
markdownlint-cli2 "docs/src/**/*.md"
```
