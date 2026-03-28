# Development Setup

This page covers how to set up a local development environment
for the `laurus-php` binding, build it, and run the test
suite.

## Prerequisites

- **Rust** 1.85 or later with Cargo
- **PHP** 8.1 or later with development headers (`php-dev` / `php-devel`)
- **Composer** for dependency management
- Repository cloned locally

```bash
git clone https://github.com/mosuka/laurus.git
cd laurus
```

## Build

### Development build

Compiles the Rust native extension in debug mode. Re-run after
any Rust source change.

```bash
cd laurus-php
cargo build
```

The resulting shared library is located at `../target/debug/liblaurus_php.so`.

### Release build

```bash
cd laurus-php
cargo build --release
```

The resulting shared library is located at `../target/release/liblaurus_php.so`.

### Verify the build

```bash
php -d extension=../target/release/liblaurus_php.so -r "
use Laurus\Index;
\$index = new Index();
print_r(\$index->stats());
"
# Array ( [document_count] => 0 [vector_fields] => Array ( ) )
```

## Testing

Tests use [PHPUnit](https://phpunit.de/) and are located in
`tests/`.

```bash
# Install test dependencies
composer install

# Run all tests
php -d extension=../target/release/liblaurus_php.so vendor/bin/phpunit tests/
```

To run a specific test file:

```bash
php -d extension=../target/release/liblaurus_php.so vendor/bin/phpunit tests/LaurusTest.php
```

## Linting and formatting

```bash
# Rust lint (Clippy)
cargo clippy -p laurus-php -- -D warnings

# Rust formatting
cargo fmt -p laurus-php --check

# Apply formatting
cargo fmt -p laurus-php
```

## Cleaning up

```bash
# Remove build artifacts
cargo clean

# Remove Composer dependencies
rm -rf vendor/
```

## Project layout

```text
laurus-php/
├── Cargo.toml          # Rust crate manifest
├── composer.json       # Composer package definition
├── composer.lock       # Locked dependency versions
├── src/                # Rust source (ext-php-rs binding)
│   ├── lib.rs          # Module registration
│   ├── index.rs        # Index class
│   ├── schema.rs       # Schema class
│   ├── query.rs        # Query classes
│   ├── search.rs       # SearchRequest / SearchResult / Fusion
│   ├── analysis.rs     # Tokenizer / Filter / Token
│   ├── convert.rs      # PHP <-> DataValue conversion
│   └── errors.rs       # Error mapping
├── tests/              # PHPUnit tests
│   └── LaurusTest.php
└── examples/           # Runnable PHP examples
```
