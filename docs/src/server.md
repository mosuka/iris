# gRPC Server

Laurus includes a built-in gRPC server that keeps the search engine resident in memory, eliminating the per-command startup overhead of the CLI. This is the recommended way to run Laurus in production or when integrating with other services.

## Features

- **Persistent engine** — The index stays open across requests; no WAL replay on every call
- **Full gRPC API** — Index management, document CRUD, commit, and search (unary + streaming)
- **Health checking** — Standard health check endpoint for load balancers and orchestrators
- **Graceful shutdown** — Pending changes are committed automatically on Ctrl+C / SIGINT
- **TOML configuration** — Optional config file with CLI override support

## Quick Start

```bash
# Start the server with default settings
laurus serve

# Start with a custom data directory and port
laurus --data-dir ./my_index serve --port 8080

# Start with a configuration file
laurus serve --config config.toml
```

## Sections

- [Getting Started](server/getting_started.md) — Installation, startup options, and configuration
- [gRPC API Reference](server/api.md) — Full API documentation for all services and RPCs
