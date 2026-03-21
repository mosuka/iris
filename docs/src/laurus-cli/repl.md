# REPL (Interactive Mode)

The REPL provides an interactive session for exploring your index without typing the full `laurus` command each time.

## Starting the REPL

```bash
laurus --index-dir ./my_index repl
```

If an index already exists at the specified directory, it is opened automatically:

```text
Laurus REPL (type 'help' for commands, 'quit' to exit)
laurus>
```

If no index exists yet, the REPL starts without a loaded index and guides you to create one:

```text
Laurus REPL — no index found at ./my_index.
Use 'create index <schema_path>' to create one, or 'help' for commands.
laurus>
```

## Available Commands

Commands follow the same `<operation> <resource>` ordering as the CLI.

| Command | Description |
| :--- | :--- |
| `create index [schema_path]` | Create a new index (interactive wizard if no path given) |
| `create schema <output_path>` | Interactive schema generation wizard |
| `search <query>` | Search the index |
| `add field <name> <json>` | Add a field to the schema |
| `add doc <id> <json>` | Add a document |
| `get stats` | Show index statistics |
| `get schema` | Show the current schema |
| `get doc <id>` | Get a document by ID |
| `delete field <name>` | Remove a field from the schema |
| `delete doc <id>` | Delete a document by ID |
| `commit` | Commit pending changes |
| `help` | Show available commands |
| `quit` / `exit` | Exit the REPL |

> **Note:** Commands other than `create`, `help`, and `quit` require a loaded index. If no index is loaded, the REPL displays a message asking you to run `create index` first.

## Usage Examples

### Creating an Index

```text
laurus> create index ./schema.toml
Index created at ./my_index.
laurus> add doc doc1 {"title":"Hello","body":"World"}
Document 'doc1' added.
```

### Searching

```text
laurus> search body:rust
╭──────┬────────┬────────────────────────────────────╮
│ ID   │ Score  │ Fields                             │
├──────┼────────┼────────────────────────────────────┤
│ doc1 │ 0.8532 │ body: Rust is a systems..., title… │
╰──────┴────────┴────────────────────────────────────╯
```

### Managing Fields

```text
laurus> add field category {"Text": {"indexed": true, "stored": true}}
Field 'category' added.
laurus> delete field category
Field 'category' deleted.
```

### Adding and Committing Documents

```text
laurus> add doc doc4 {"title":"New Document","body":"Some content here."}
Document 'doc4' added.
laurus> commit
Changes committed.
```

### Retrieving Information

```text
laurus> get stats
Document count: 3

laurus> get schema
{
  "fields": { ... },
  "default_fields": ["title", "body"]
}

laurus> get doc doc4
╭──────┬───────────────────────────────────────────────╮
│ ID   │ Fields                                        │
├──────┼───────────────────────────────────────────────┤
│ doc4 │ body: Some content here., title: New Document │
╰──────┴───────────────────────────────────────────────╯
```

### Deleting Documents

```text
laurus> delete doc doc4
Document 'doc4' deleted.
laurus> commit
Changes committed.
```

## Features

- **Line editing** — Arrow keys, Home/End, and standard readline shortcuts
- **History** — Use Up/Down arrows to recall previous commands
- **Ctrl+C / Ctrl+D** — Exit the REPL gracefully
