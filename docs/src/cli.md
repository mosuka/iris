# CLI (Command-Line Interface)

Laurus provides a command-line tool `laurus` that lets you create indexes, manage documents, and run search queries without writing code.

## Features

- **Index management** — Create and inspect indexes from TOML schema files
- **Document CRUD** — Add, retrieve, and delete documents via JSON
- **Search** — Execute queries using the [Query DSL](advanced/query_dsl.md)
- **Dual output** — Human-readable tables or machine-parseable JSON
- **Interactive REPL** — Explore your index in a live session

## Getting Started

```bash
# Install
cargo install laurus-cli

# Create an index
laurus --data-dir ./my_index create index --schema schema.toml

# Add a document
laurus --data-dir ./my_index add doc --id doc1 --data '{"title":"Hello","body":"World"}'

# Commit changes
laurus --data-dir ./my_index commit

# Search
laurus --data-dir ./my_index search "body:world"
```

See the sub-sections for detailed documentation:

- [Installation](cli/installation.md) — How to install the CLI
- [Commands](cli/commands.md) — Full command reference
- [REPL](cli/repl.md) — Interactive mode
