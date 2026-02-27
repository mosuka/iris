# Command Reference

## Global Options

Every command accepts these options:

| Option | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--data-dir <PATH>` | `LAURUS_DATA_DIR` | `./laurus_data` | Path to the index data directory |
| `--format <FORMAT>` | — | `table` | Output format: `table` or `json` |

```bash
# Example: use JSON output with a custom data directory
laurus --data-dir /var/data/my_index --format json search "title:rust"
```

---

## `create` — Create a Resource

### `create index`

Create a new index from a schema TOML file.

```bash
laurus create index --schema <FILE>
```

**Arguments:**

| Flag | Required | Description |
| :--- | :--- | :--- |
| `--schema <FILE>` | Yes | Path to a TOML file defining the index schema |

**Schema file format:**

The schema file follows the same structure as the `Schema` type in the Laurus library. Example:

```toml
default_fields = ["title", "body"]

[fields.title.Text]
stored = true
indexed = true

[fields.body.Text]
stored = true
indexed = true

[fields.category.Text]
stored = true
indexed = true
```

**Example:**

```bash
laurus --data-dir ./my_index create index --schema schema.toml
# Index created at ./my_index.
```

> **Note:** An error is returned if the index already exists. Delete the data directory to recreate.

---

## `get` — Get a Resource

### `get index`

Display statistics about the index.

```bash
laurus get index
```

**Table output example:**

```text
Document count: 42

Vector fields:
╭──────────┬─────────┬───────────╮
│ Field    │ Vectors │ Dimension │
├──────────┼─────────┼───────────┤
│ text_vec │ 42      │ 384       │
╰──────────┴─────────┴───────────╯
```

**JSON output example:**

```bash
laurus --format json get index
```

```json
{
  "document_count": 42,
  "fields": {
    "text_vec": {
      "vector_count": 42,
      "dimension": 384
    }
  }
}
```

### `get doc`

Retrieve a document (and all its chunks) by external ID.

```bash
laurus get doc --id <ID>
```

**Table output example:**

```text
╭──────┬─────────────────────────────────────────╮
│ ID   │ Fields                                  │
├──────┼─────────────────────────────────────────┤
│ doc1 │ body: This is a test, title: Hello World │
╰──────┴─────────────────────────────────────────╯
```

**JSON output example:**

```bash
laurus --format json get doc --id doc1
```

```json
[
  {
    "id": "doc1",
    "document": {
      "title": "Hello World",
      "body": "This is a test document."
    }
  }
]
```

---

## `add` — Add a Resource

### `add doc`

Add a document to the index. Documents are not searchable until `commit` is called.

```bash
laurus add doc --id <ID> --data <JSON>
```

**Arguments:**

| Flag | Required | Description |
| :--- | :--- | :--- |
| `--id <ID>` | Yes | External document ID (string) |
| `--data <JSON>` | Yes | Document fields as a JSON string |

The JSON format is a flat object mapping field names to values:

```json
{
  "title": "Introduction to Rust",
  "body": "Rust is a systems programming language.",
  "category": "programming"
}
```

**Example:**

```bash
laurus add doc --id doc1 --data '{"title":"Hello World","body":"This is a test document."}'
# Document 'doc1' added. Run 'commit' to persist changes.
```

> **Tip:** Multiple documents can share the same external ID (chunking pattern). Use `add doc` for each chunk.

---

## `delete` — Delete a Resource

### `delete doc`

Delete a document (and all its chunks) by external ID.

```bash
laurus delete doc --id <ID>
```

**Example:**

```bash
laurus delete doc --id doc1
# Document 'doc1' deleted. Run 'commit' to persist changes.
```

---

## `commit`

Commit pending changes (additions and deletions) to the index. Until committed, changes are not visible to search.

```bash
laurus commit
```

**Example:**

```bash
laurus --data-dir ./my_index commit
# Changes committed successfully.
```

---

## `search`

Execute a search query using the [Query DSL](../advanced/query_dsl.md).

```bash
laurus search <QUERY> [--limit <N>] [--offset <N>]
```

**Arguments:**

| Argument / Flag | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| `<QUERY>` | Yes | — | Query string in Laurus Query DSL |
| `--limit <N>` | No | `10` | Maximum number of results |
| `--offset <N>` | No | `0` | Number of results to skip |

**Query syntax examples:**

```bash
# Term query
laurus search "body:rust"

# Phrase query
laurus search 'body:"machine learning"'

# Boolean query
laurus search "+body:programming -body:python"

# Fuzzy query (typo tolerance)
laurus search "body:programing~2"

# Wildcard query
laurus search "title:intro*"

# Range query
laurus search "price:[10 TO 50]"
```

**Table output example:**

```text
╭──────┬────────┬─────────────────────────────────────────╮
│ ID   │ Score  │ Fields                                  │
├──────┼────────┼─────────────────────────────────────────┤
│ doc1 │ 0.8532 │ body: Rust is a systems..., title: Intr │
│ doc3 │ 0.4210 │ body: JavaScript powers..., title: Web  │
╰──────┴────────┴─────────────────────────────────────────╯
```

**JSON output example:**

```bash
laurus --format json search "body:rust" --limit 5
```

```json
[
  {
    "id": "doc1",
    "score": 0.8532,
    "document": {
      "title": "Introduction to Rust",
      "body": "Rust is a systems programming language."
    }
  }
]
```

---

## `repl`

Start an interactive REPL session. See [REPL](repl.md) for details.

```bash
laurus repl
```
