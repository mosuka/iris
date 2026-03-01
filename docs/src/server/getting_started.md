# Getting Started with the gRPC Server

## Starting the Server

The gRPC server is started via the `serve` subcommand of the `laurus` CLI:

```bash
laurus serve [OPTIONS]
```

### Options

| Option | Short | Env Variable | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `--config <PATH>` | `-c` | `LAURUS_CONFIG` | — | Path to a TOML configuration file |
| `--host <HOST>` | `-H` | `LAURUS_HOST` | `0.0.0.0` | Listen address |
| `--port <PORT>` | `-p` | `LAURUS_PORT` | `50051` | Listen port |
| `--http-port <PORT>` | — | `LAURUS_HTTP_PORT` | — | HTTP Gateway port (enables HTTP gateway when set) |
| `--log-level <LEVEL>` | `-l` | `LAURUS_LOG_LEVEL` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |

The global `--data-dir` option (env: `LAURUS_DATA_DIR`) specifies the index data directory:

```bash
# Using CLI arguments
laurus --data-dir ./my_index serve --port 8080 --log-level debug

# Using environment variables
export LAURUS_DATA_DIR=./my_index
export LAURUS_PORT=8080
export LAURUS_LOG_LEVEL=debug
laurus serve
```

### Startup Behavior

On startup, the server attempts to open an existing index at the configured data directory. If no index exists, the server starts without one — you can create an index later via the `CreateIndex` RPC.

## Configuration File

You can use a TOML configuration file instead of (or in addition to) command-line options:

```bash
laurus serve --config config.toml
```

### Format

```toml
[server]
host = "0.0.0.0"
port = 50051
http_port = 8080  # Optional: enables HTTP Gateway

[index]
data_dir = "./laurus_data"

[log]
level = "info"
```

### Priority

Settings are resolved in the following order (highest priority first):

```text
CLI arguments > Environment variables > Config file > Defaults
```

For example, if `config.toml` sets `port = 50051`, the environment variable `LAURUS_PORT=4567` is set, and `--port 1234` is passed on the command line:

```bash
LAURUS_PORT=4567 laurus serve --config config.toml --port 1234
# → Listens on port 1234 (CLI argument wins)
```

If the CLI argument is omitted:

```bash
LAURUS_PORT=4567 laurus serve --config config.toml
# → Listens on port 4567 (environment variable wins over config file)
```

## Graceful Shutdown

When the server receives a shutdown signal (Ctrl+C / SIGINT), it automatically:

1. Stops accepting new connections
2. Commits any pending changes to the index
3. Exits cleanly

## HTTP Gateway

When `http_port` is set, an HTTP/JSON gateway starts alongside the gRPC server. The gateway proxies HTTP requests to the gRPC server internally:

```text
User Request (HTTP/JSON) → gRPC Gateway (axum) → gRPC Server (tonic) → Engine
```

If `http_port` is omitted, only the gRPC server starts (default behavior).

### Starting with HTTP Gateway

```bash
# Via CLI
laurus serve --http-port 8080

# Via config file (set http_port in [server] section)
laurus serve --config config.toml

# Via environment variable
LAURUS_HTTP_PORT=8080 laurus serve
```

### HTTP API Endpoints

| Method | Path | gRPC Method |
| :--- | :--- | :--- |
| GET | `/v1/health` | `HealthService/Check` |
| POST | `/v1/index` | `IndexService/CreateIndex` |
| GET | `/v1/index` | `IndexService/GetIndex` |
| GET | `/v1/schema` | `IndexService/GetSchema` |
| PUT | `/v1/documents/:id` | `DocumentService/PutDocument` |
| POST | `/v1/documents/:id` | `DocumentService/AddDocument` |
| GET | `/v1/documents/:id` | `DocumentService/GetDocuments` |
| DELETE | `/v1/documents/:id` | `DocumentService/DeleteDocuments` |
| POST | `/v1/commit` | `DocumentService/Commit` |
| POST | `/v1/search` | `SearchService/Search` |
| POST | `/v1/search/stream` | `SearchService/SearchStream` (SSE) |

### HTTP API Examples

```bash
# Health check
curl http://localhost:8080/v1/health

# Create an index
curl -X POST http://localhost:8080/v1/index \
  -H 'Content-Type: application/json' \
  -d '{
    "schema": {
      "fields": {
        "title": {"text": {"indexed": true, "stored": true}},
        "body": {"text": {"indexed": true, "stored": true}}
      },
      "default_fields": ["title", "body"]
    }
  }'

# Add a document
curl -X POST http://localhost:8080/v1/documents/doc1 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Hello World",
        "body": "This is a test document."
      }
    }
  }'

# Commit
curl -X POST http://localhost:8080/v1/commit

# Search
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "body:test", "limit": 10}'

# Streaming search (SSE)
curl -N -X POST http://localhost:8080/v1/search/stream \
  -H 'Content-Type: application/json' \
  -d '{"query": "body:test", "limit": 10}'
```

## Connecting via gRPC

Any gRPC client can connect to the server. For quick testing, [grpcurl](https://github.com/fullstorydev/grpcurl) is useful:

```bash
# Health check
grpcurl -plaintext localhost:50051 laurus.v1.HealthService/Check

# Create an index
grpcurl -plaintext -d '{
  "schema": {
    "fields": {
      "title": {"text": {"indexed": true, "stored": true, "term_vectors": true}},
      "body": {"text": {"indexed": true, "stored": true, "term_vectors": true}}
    },
    "default_fields": ["title", "body"]
  }
}' localhost:50051 laurus.v1.IndexService/CreateIndex

# Add a document
grpcurl -plaintext -d '{
  "id": "doc1",
  "document": {
    "fields": {
      "title": {"text_value": "Hello World"},
      "body": {"text_value": "This is a test document."}
    }
  }
}' localhost:50051 laurus.v1.DocumentService/AddDocument

# Commit
grpcurl -plaintext localhost:50051 laurus.v1.DocumentService/Commit

# Search
grpcurl -plaintext -d '{"query": "body:test", "limit": 10}' \
  localhost:50051 laurus.v1.SearchService/Search
```
