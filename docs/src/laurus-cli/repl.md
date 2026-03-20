# REPL (Interactive Mode)

The REPL provides an interactive session for exploring your index without typing the full `laurus` command each time.

## Starting the REPL

```bash
laurus --index-dir ./my_index repl
```

```text
Laurus REPL (type 'help' for commands, 'quit' to exit)
laurus>
```

The REPL opens the index at startup and keeps it loaded throughout the session.

## Available Commands

Commands follow the same `<operation> <resource>` ordering as the CLI.

| Command | Description |
| :--- | :--- |
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

## Usage Examples

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
