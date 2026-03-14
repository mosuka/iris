# Hands-on Tutorial

This tutorial walks you through a complete workflow with laurus-server: starting the server, creating an index, adding documents, searching, updating, and deleting. All examples use `curl` via the HTTP Gateway.

## Prerequisites

- laurus CLI installed (see [Installation](../getting_started/installation.md))
- `curl` available on your system

## Step 1: Start the Server

Start laurus-server with the HTTP Gateway enabled:

```bash
laurus --data-dir ./tutorial_data serve --http-port 8080
```

You should see log output indicating the gRPC server (port 50051) and the HTTP Gateway (port 8080) have started.

Verify the server is running:

```bash
curl http://localhost:8080/v1/health
```

Expected response:

```json
{"status":"SERVING_STATUS_SERVING"}
```

## Step 2: Create an Index

Create an index with a schema that defines text fields for lexical search:

```bash
curl -X POST http://localhost:8080/v1/index \
  -H 'Content-Type: application/json' \
  -d '{
    "schema": {
      "fields": {
        "title": {"text": {"indexed": true, "stored": true, "term_vectors": false}},
        "body": {"text": {"indexed": true, "stored": true, "term_vectors": false}},
        "category": {"text": {"indexed": true, "stored": true, "term_vectors": false}}
      },
      "default_fields": ["title", "body"]
    }
  }'
```

This creates an index with three text fields. The `default_fields` setting means that queries without a field prefix will search both `title` and `body`.

Verify the index was created:

```bash
curl http://localhost:8080/v1/index
```

Expected response:

```json
{"documentCount":"0","vectorFields":{}}
```

## Step 3: Add Documents

Add a few documents to the index. Use `PUT` to upsert documents by ID:

```bash
curl -X PUT http://localhost:8080/v1/documents/doc001 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Introduction to Rust Programming",
        "body": "Rust is a modern systems programming language that focuses on safety, speed, and concurrency.",
        "category": "programming"
      }
    }
  }'
```

```bash
curl -X PUT http://localhost:8080/v1/documents/doc002 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Web Development with Rust",
        "body": "Building web applications with Rust has become increasingly popular. Frameworks like Actix and Rocket make it easy to create fast and secure web services.",
        "category": "web-development"
      }
    }
  }'
```

```bash
curl -X PUT http://localhost:8080/v1/documents/doc003 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Python for Data Science",
        "body": "Python is the most popular language for data science and machine learning. Libraries like NumPy and Pandas provide powerful tools for data analysis.",
        "category": "data-science"
      }
    }
  }'
```

## Step 4: Commit Changes

Documents are not searchable until committed. Commit the pending changes:

```bash
curl -X POST http://localhost:8080/v1/commit
```

## Step 5: Search Documents

### Basic Search

Search for documents containing "rust":

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "rust", "limit": 10}'
```

This searches the default fields (`title` and `body`). Expected result: `doc001` and `doc002` are returned.

### Field-Specific Search

Search only in the `title` field:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "title:python", "limit": 10}'
```

Expected result: only `doc003` is returned.

### Search by Category

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "category:programming", "limit": 10}'
```

Expected result: only `doc001` is returned.

### Boolean Queries

Combine conditions with `AND`, `OR`, and `NOT`:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "rust AND web", "limit": 10}'
```

Expected result: only `doc002` is returned (contains both "rust" and "web").

### Field Boosting

Boost the `title` field to prioritize title matches:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "rust",
    "limit": 10,
    "field_boosts": {"title": 2.0}
  }'
```

## Step 6: Retrieve a Document

Fetch a specific document by its ID:

```bash
curl http://localhost:8080/v1/documents/doc001
```

Expected response:

```json
{
  "documents": [
    {
      "fields": {
        "title": "Introduction to Rust Programming",
        "body": "Rust is a modern systems programming language that focuses on safety, speed, and concurrency.",
        "category": "programming"
      }
    }
  ]
}
```

## Step 7: Update a Document

Update a document by `PUT`-ing with the same ID. This replaces the entire document:

```bash
curl -X PUT http://localhost:8080/v1/documents/doc001 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Introduction to Rust Programming",
        "body": "Rust is a modern systems programming language that focuses on safety, speed, and concurrency. It provides memory safety without garbage collection.",
        "category": "programming"
      }
    }
  }'
```

Commit and verify:

```bash
curl -X POST http://localhost:8080/v1/commit
curl http://localhost:8080/v1/documents/doc001
```

The updated body text is now stored.

## Step 8: Delete a Document

Delete a document by its ID:

```bash
curl -X DELETE http://localhost:8080/v1/documents/doc003
```

Commit and verify:

```bash
curl -X POST http://localhost:8080/v1/commit
```

Confirm the document was deleted:

```bash
curl http://localhost:8080/v1/documents/doc003
```

Expected response:

```json
{"documents":[]}
```

Search results will no longer include the deleted document:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "python", "limit": 10}'
```

Expected result: no results returned.

## Step 9: Check Index Statistics

View the current index statistics:

```bash
curl http://localhost:8080/v1/index
```

The `documentCount` should reflect the remaining documents after the deletion.

## Step 10: Clean Up

Stop the server with `Ctrl+C`. The server performs a graceful shutdown, committing any pending changes before exiting.

To remove the tutorial data:

```bash
rm -rf ./tutorial_data
```

## Next Steps

- Learn about [vector search and hybrid search](../concepts/search/hybrid_search.md) for semantic similarity queries
- Explore the [gRPC API Reference](grpc_api.md) for the full API specification
- Configure the server for production using [Configuration](configuration.md)
- Use `grpcurl` or a gRPC client library for programmatic access — see [Getting Started](getting_started.md)
