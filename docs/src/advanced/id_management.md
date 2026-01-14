# ID Management

Sarissa employs a robust tiered ID management strategy to handle document stability and retrieval efficiency.

## 1. Internal ID (u64)
- An auto-incrementing integer assigned internally by Sarissa.
- Used as the primary key in forward and backward indexes.
- Efficient for bitmap operations.
- **NOT** guaranteed to be stable across deletions and compactions (if ID reuse is enabled in future versions).

## 2. External ID (String)
- Application-level identifier (e.g., UUID, URL, primary key).
- Stored as a reserved field `_id` in the Lexical Index.
- `HybridEngine` maintains a mapping between External and Internal IDs.

## Upsert Logic
When using `index_document(external_id, doc)`:
1. Sarissa checks if `external_id` already exists in the index.
2. If found, the old internal ID is marked for deletion.
3. A new internal ID is generated for the new document version.
4. The document is indexed with the new ID.

This "Delete-then-Insert" pattern ensures atomic-like updates.
