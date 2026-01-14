# Deletions & Compaction

## Logical Deletion
When a document is deleted:
1. It is **not** immediately removed from the physical files.
2. Its ID is added to a **Deletion Bitmap**.
3. Subsequent searches check this bitmap and filter out deleted IDs from results.
4. This operation is fast O(1).

## Physical Deletion (Compaction)
Over time, deleted documents accumulate and waste space.
- **Compaction (Vacuuming)** is the process of rewriting the index files to exclude logically deleted data.
- It rebuilds the HNSW graph or Inverted Index segments without the deleted entries.
- This is an expensive operation and should be run periodically (e.g., nightly).

```rust
// Example of triggering manual compaction
vector_engine.force_merge()?;
```
