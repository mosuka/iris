# Persistence & WAL

To ensure data durability and fast recovery, Sarissa implements a **Write-Ahead Log (WAL)** system.

## Write-Ahead Log (WAL)
- All incoming write operations (Add, Delete) are immediately appended to a disk-based log file.
- This happens **before** memory structures (like HNSW graph or Inverted Index) are updated.
- In case of a crash, Sarissa replays the WAL on startup to restore the in-memory state.

## Segments
Indexes can be split into segments (though currently, the implementation focuses on a global segment model with potential for expansion).
- Larger indexes are safer to manage as smaller, immutable segments that are periodically merged.

## Checkpointing
Currently, explicit commits flush the in-memory state to durable index files.
- `lexical_engine.commit()`
- `hybrid_engine.commit()`
