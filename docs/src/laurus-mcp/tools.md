# MCP Tools Reference

The laurus MCP server exposes the following tools.

## connect

Connect to a running laurus-server gRPC endpoint. Call this before using other
tools if the server was started without the `--endpoint` flag, or to switch to
a different laurus-server at runtime.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `endpoint` | string | Yes | gRPC endpoint URL (e.g. `http://localhost:50051`) |

### Example

```text
Tool: connect
endpoint: "http://localhost:50051"
```

Result: `Connected to laurus-server at http://localhost:50051.`

---

## create_index

Create a new search index with the provided schema.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `schema_json` | string | Yes | Schema definition as a JSON string |

### Schema JSON format

FieldOption uses serde's externally-tagged representation where the variant name is the key:

```json
{
  "fields": {
    "title":     { "Text":    { "indexed": true, "stored": true } },
    "body":      { "Text":    {} },
    "score":     { "Float":   {} },
    "count":     { "Integer": {} },
    "active":    { "Boolean": {} },
    "created":   { "DateTime": {} },
    "embedding": { "Hnsw":    { "dimension": 384 } }
  }
}
```

### Example

```text
Tool: create_index
schema_json: {"fields": {"title": {"Text": {}}, "body": {"Text": {}}}}
```

Result: `Index created successfully at /path/to/index.`

---

## add_field

Add a new field to the index.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `name` | string | Yes | The field name |
| `field_option_json` | string | Yes | Field configuration as JSON |

### Example

```json
{
  "name": "category",
  "field_option_json": "{\"Text\": {\"indexed\": true, \"stored\": true}}"
}
```

Result: `Field 'category' added successfully.`

---

## get_index

Get statistics for the current search index.

### Parameters

None.

### Result

```json
{
  "document_count": 42,
  "vector_fields": ["embedding"]
}
```

---

## add_document

Add or upsert a document in the index. Call `commit` after adding documents.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `id` | string | Yes | External document identifier |
| `document` | object | Yes | Document fields as a JSON object |
| `mode` | string | No | `"put"` (default, upsert) or `"add"` (append chunk) |

### Modes

- `put` (default): Delete any existing document with the same `id`, then index the new one.
- `add`: Append as a new chunk. Multiple chunks can share the same `id` (useful for splitting large documents).

### Example

```text
Tool: add_document
id: "doc-1"
document: {"title": "Hello World", "body": "This is a test document."}
```

Result: `Document 'doc-1' added. Call commit to persist changes.`

---

## get_document

Retrieve stored document(s) by external ID.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `id` | string | Yes | External document identifier |

### Result

```json
{
  "id": "doc-1",
  "documents": [
    { "title": "Hello World", "body": "This is a test document." }
  ]
}
```

---

## delete_document

Delete document(s) by external ID. Call `commit` after deletion.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `id` | string | Yes | External document identifier |

Result: `Document 'doc-1' deleted. Call commit to persist changes.`

---

## commit

Commit pending changes to disk. Must be called after `add_document` or `delete_document` to make changes searchable and durable.

### Parameters

None.

Result: `Changes committed successfully.`

---

## search

Search documents using the laurus query DSL.

### Parameters

| Name | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `query` | string | Yes | Search query in laurus query DSL |
| `limit` | integer | No | Maximum results (default: 10) |
| `offset` | integer | No | Results to skip for pagination (default: 0) |

### Query DSL examples

| Query | Description |
| :--- | :--- |
| `hello` | Term search across default fields |
| `title:hello` | Field-scoped term search |
| `title:hello AND body:world` | Boolean AND |
| `"exact phrase"` | Phrase search |
| `roam~2` | Fuzzy search (edit distance 2) |
| `count:[1 TO 10]` | Range search |
| `title:helo~1` | Fuzzy field search |

### Result

```json
{
  "total": 2,
  "results": [
    {
      "id": "doc-1",
      "score": 3.14,
      "document": { "title": "Hello World", "body": "..." }
    },
    {
      "id": "doc-2",
      "score": 1.57,
      "document": { "title": "Hello Again", "body": "..." }
    }
  ]
}
```

---

## Typical Workflow

```text
1. connect         → connect to a running laurus-server
2. create_index    → define the schema (if index does not exist)
3. add_field       → dynamically add fields (optional)
4. add_document    → index documents (repeat as needed)
5. commit          → persist changes to disk
6. search          → query the index
7. add_document    → update documents
8. delete_document → remove documents
9. commit          → persist changes
```
