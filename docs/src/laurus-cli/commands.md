# Command Reference

## Global Options

Every command accepts these options:

| Option | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--index-dir <PATH>` | `LAURUS_INDEX_DIR` | `./laurus_index` | Path to the index data directory |
| `--format <FORMAT>` | — | `table` | Output format: `table` or `json` |

```bash
# Example: use JSON output with a custom data directory
laurus --index-dir /var/data/my_index --format json search "title:rust"
```

---

## `create` — Create a Resource

### `create index`

Create a new index. If `--schema` is given, uses that TOML file; otherwise launches the interactive schema wizard.

```bash
laurus create index [--schema <FILE>]
```

**Arguments:**

| Flag | Required | Description |
| :--- | :--- | :--- |
| `--schema <FILE>` | No | Path to a TOML file defining the index schema. When omitted, the command checks if a `schema.toml` already exists in the index directory and uses it; otherwise the interactive wizard is launched. |

**Schema file format:**

The schema file follows the same structure as the `Schema` type in the Laurus library. See [Schema Format Reference](schema_format.md) for full details. Example:

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

**Examples:**

```bash
# From a schema file
laurus --index-dir ./my_index create index --schema schema.toml
# Index created at ./my_index.

# Interactive wizard (no --schema flag)
laurus --index-dir ./my_index create index
# === Laurus Schema Generator ===
# Field name: title
# ...
# Index created at ./my_index.
```

> **Note:** If both `schema.toml` and `store/` already exist, an error is returned. Delete the index directory to recreate. If only `schema.toml` exists (e.g. after an interrupted creation), running `create index` without `--schema` recovers the index by creating the missing storage from the existing schema.

### `create schema`

Interactively generate a schema TOML file through a guided wizard.

```bash
laurus create schema [--output <FILE>]
```

**Arguments:**

| Flag | Required | Default | Description |
| :--- | :--- | :--- | :--- |
| `--output <FILE>` | No | `schema.toml` | Output file path for the generated schema |

The wizard guides you through:

1. **Field definition** — Enter a field name, select the type, and configure type-specific options
2. **Repeat** — Add as many fields as needed
3. **Default fields** — Select which lexical fields to use as default search fields
4. **Preview** — Review the generated TOML before saving
5. **Save** — Write the schema file

**Supported field types:**

| Type | Category | Options |
| :--- | :--- | :--- |
| `Text` | Lexical | `indexed`, `stored`, `term_vectors` |
| `Integer` | Lexical | `indexed`, `stored` |
| `Float` | Lexical | `indexed`, `stored` |
| `Boolean` | Lexical | `indexed`, `stored` |
| `DateTime` | Lexical | `indexed`, `stored` |
| `Geo` | Lexical | `indexed`, `stored` |
| `Bytes` | Lexical | `stored` |
| `Hnsw` | Vector | `dimension`, `distance`, `m`, `ef_construction` |
| `Flat` | Vector | `dimension`, `distance` |
| `Ivf` | Vector | `dimension`, `distance`, `n_clusters`, `n_probe` |

**Example:**

```bash
# Generate schema.toml interactively
laurus create schema

# Specify output path
laurus create schema --output my_schema.toml

# Then create an index from the generated schema
laurus create index --schema schema.toml
```

---

## `get` — Get a Resource

### `get stats`

Display statistics about the index.

```bash
laurus get stats
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
laurus --format json get stats
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

### `get schema`

Display the current index schema as JSON.

```bash
laurus get schema
```

**Example:**

```bash
laurus get schema
# {
#   "fields": { ... },
#   "default_fields": ["title", "body"],
#   ...
# }
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

### `add field`

Dynamically add a new field to an existing index.

```bash
laurus add field --index-dir ./data \
    --name category \
    --field-option '{"Text": {"indexed": true, "stored": true}}'
```

The `--field-option` argument accepts a JSON string using the same
externally-tagged format as the schema file. The schema is automatically
persisted after the field is added.

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

### `delete field`

Remove a field from the index schema.

```bash
laurus delete field --name <FIELD_NAME>
```

**Example:**

```bash
laurus delete field --name category
# Field 'category' deleted.
```

Existing indexed data for the field remains in storage but becomes
inaccessible. Per-field analyzers and embedders are unregistered.

---

## `commit`

Commit pending changes (additions and deletions) to the index. Until committed, changes are not visible to search.

```bash
laurus commit
```

**Example:**

```bash
laurus --index-dir ./my_index commit
# Changes committed successfully.
```

---

## `search`

Execute a search query using the [Query DSL](../concepts/query_dsl.md).

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

---

## `serve`

Start the gRPC server (and optionally the HTTP Gateway).

```bash
laurus serve [OPTIONS]
```

For startup options, configuration, and usage examples, see the [laurus-server documentation](../laurus-server.md):

- [Getting Started](../laurus-server/getting_started.md) — startup options and gRPC connection examples
- [Configuration](../laurus-server/configuration.md) — TOML config file, environment variables, and priority rules
- [Hands-on Tutorial](../laurus-server/tutorial.md) — step-by-step walkthrough
