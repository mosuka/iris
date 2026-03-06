# ID Management

Laurus uses a dual-tiered ID management strategy to ensure efficient document retrieval, updates, and aggregation in distributed environments.

## 1. External ID (String)

The External ID is a **logical identifier** used by users and applications to uniquely identify a document.

- **Type**: `String`
- **Role**: You can use any unique value, such as UUIDs, URLs, or database primary keys.
- **Storage**: Persisted transparently as a reserved system field name `_id` within the Lexical Index.
- **Uniqueness**: Expected to be unique across the entire system.
- **Updates**: Indexing a document with an existing `external_id` triggers an automatic "Delete-then-Insert" (Upsert) operation, replacing the old version with the newest.

## 2. Internal ID (u64 / Stable ID)

The Internal ID is a **physical handle** used internally by Laurus's engines (Lexical and Vector) for high-performance operations.

- **Type**: Unsigned 64-bit Integer (`u64`)
- **Role**: Used for bitmap operations, point references, and routing between distributed nodes.
- **Immutability (Stable)**: Once assigned, an Internal ID never changes due to index merges (segment compaction) or restarts. This prevents inconsistencies in deletion logs and caches.

### ID Structure (Shard-Prefixed)

Laurus employs a **Shard-Prefixed Stable ID** scheme designed for multi-node distributed environments.

| Bit Range | Name | Description |
| :--- | :--- | :--- |
| **48-63 bit** | **Shard ID** | Prefix identifying the node or partition (up to 65,535 shards). |
| **0-47 bit** | **Local ID** | Monotonically increasing document number within a shard (up to ~281 trillion documents). |

#### Why this structure?

1. **Zero-Cost Aggregation**: Since `u64` IDs are globally unique, the aggregator can perform fast sorting and deduplication without worrying about ID collisions between nodes.
2. **Fast Routing**: The aggregator can immediately identify the physical node responsible for a document just by looking at the upper bits, avoiding expensive hash lookups.
3. **High-Performance Fetching**: Internal IDs map directly to physical data structures. This allows Laurus to skip the "External-to-Internal ID" conversion step during retrieval, achieving **O(1)** access speed.

## ID Lifecycle

1. **Registration (`engine.put_document()` / `engine.add_document()`)**: User provides a document with an External ID.
2. **ID Assignment**: The `Engine` combines the current `shard_id` with a new Local ID to issue a Shard-Prefixed Internal ID.
3. **Mapping**: The engine maintains the relationship between the External ID and the new Internal ID.
4. **Search**: Search results return the External ID (`String`), resolved from the Internal ID.
5. **Retrieval/Deletion**: While the user-facing API accepts External IDs for convenience, the engine internally converts them to Internal IDs for near-instant processing.
