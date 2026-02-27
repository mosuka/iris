# REPL (Interactive Mode)

The REPL provides an interactive session for exploring your index without typing the full `laurus` command each time.

## Starting the REPL

```bash
laurus --data-dir ./my_index repl
```

```text
Laurus REPL (type 'help' for commands, 'quit' to exit)
laurus>
```

The REPL opens the index at startup and keeps it loaded throughout the session.

## Available Commands

| Command | Description |
| :--- | :--- |
| `search <query> [limit]` | Search the index |
| `doc add <id> <json>` | Add a document |
| `doc get <id>` | Get a document by ID |
| `doc delete <id>` | Delete a document by ID |
| `commit` | Commit pending changes |
| `stats` | Show index statistics |
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

### Adding and Committing Documents

```text
laurus> doc add doc4 {"title":"New Document","body":"Some content here."}
Document 'doc4' added.
laurus> commit
Changes committed.
```

### Retrieving Documents

```text
laurus> doc get doc4
╭──────┬───────────────────────────────────────────────╮
│ ID   │ Fields                                        │
├──────┼───────────────────────────────────────────────┤
│ doc4 │ body: Some content here., title: New Document │
╰──────┴───────────────────────────────────────────────╯
```

### Deleting Documents

```text
laurus> doc delete doc4
Document 'doc4' deleted.
laurus> commit
Changes committed.
```

### Viewing Statistics

```text
laurus> stats
Document count: 3
```

## Features

- **Line editing** — Arrow keys, Home/End, and standard readline shortcuts
- **History** — Use Up/Down arrows to recall previous commands
- **Ctrl+C / Ctrl+D** — Exit the REPL gracefully
