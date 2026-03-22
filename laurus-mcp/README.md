# laurus-mcp

[![Crates.io](https://img.shields.io/crates/v/laurus-mcp.svg)](https://crates.io/crates/laurus-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server for the [Laurus](https://github.com/mosuka/laurus) search engine. Enables AI assistants such as Claude to index documents and perform searches through the standard MCP stdio transport.

## Features

- **MCP stdio transport** -- Runs as a subprocess; communicates with the AI client via stdin/stdout
- **gRPC client** -- Proxies all tool calls to a running `laurus-server` instance
- **All search modes** -- Lexical (BM25), vector (HNSW/Flat/IVF), and hybrid search
- **Dynamic connection** -- Connect to any laurus-server endpoint via the `connect` tool
- **Document lifecycle** -- Put, add, get, delete, and search documents through MCP tools

## Architecture

```text
AI Client (Claude, etc.)
  └─ stdio (JSON-RPC) ─→ laurus-mcp
                            └─ gRPC ─→ laurus-server
                                         └─ Index on Disk
```

## Quick Start

```bash
# Start laurus-server
laurus serve --port 50051

# Configure Claude Code
claude mcp add laurus -- laurus mcp --endpoint http://localhost:50051
```

### Claude Desktop

Add the following to your Claude Desktop configuration file (`claude_desktop_config.json`):

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "laurus": {
      "command": "laurus",
      "args": ["mcp", "--endpoint", "http://localhost:50051"]
    }
  }
}
```

## MCP Tools

| Tool | Description |
| :--- | :--- |
| `connect` | Connect to a laurus-server gRPC endpoint |
| `create_index` | Create a new index with a schema |
| `get_stats` | Get index statistics (document count, vector fields) |
| `get_schema` | Get the current index schema |
| `add_field` | Dynamically add a field to the index |
| `delete_field` | Remove a field from the schema |
| `put_document` | Put (upsert) a document |
| `add_document` | Add a document as a new chunk (append) |
| `get_documents` | Retrieve all documents by ID |
| `delete_documents` | Delete all documents by ID |
| `commit` | Commit pending changes to disk |
| `search` | Search documents using the Laurus query DSL |

## Documentation

- [MCP Server Guide](https://mosuka.github.io/laurus/laurus-mcp.html)
- [Getting Started](https://mosuka.github.io/laurus/laurus-mcp/getting_started.html)
- [Tools Reference](https://mosuka.github.io/laurus/laurus-mcp/tools.html)

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
