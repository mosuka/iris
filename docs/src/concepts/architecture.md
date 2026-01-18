# Architecture

Sarissa is built on a modular architecture composed of three main engines.

## 1. Lexical Engine
Handles keyword-based full-text search.
- **Inverted Index**: Standard posting lists for term lookups.
- **Analyzers**: Tokenization and normalization pipeline.
- **Query Parser**: Supports boolean, phrase, and structured queries.

## 2. Vector Engine
Handles semantic search using dense vectors.
- **HNSW / IVF**: Pluggable vector indexing algorithms.
- **Embeddings**: Optional integration with models to convert text/images to vectors.
- **Metadata Store**: Integrated Lexical Engine for ID management and metadata filtering.

## 3. Hybrid Engine
The unifying layer that coordinates Lexical and Vector engines.
- **ID Management**: Synchronizes Shard-Prefixed Stable IDs across sub-engines. Ensures consistency between Lexical and Vector indexes.
- **Result Merging**: Combines search results using algorithms like RRF (Reciprocal Rank Fusion) or Weighted Sum.
- **Manifest**: Persists global and shard-specific state, including the next available Local ID for each shard.

```mermaid
graph TD
    subgraph "Application Layer"
        User[User / App]
        Req[HybridSearchRequest]
    end

    subgraph "Sarissa Hybrid Engine"
        HE[HybridEngine]
        RM[ResultMerger]
        
        subgraph "Coordination"
            ID[Stable ID Management]
            Manifest["Manifest<br>(Next Local ID)"]
        end
    end

    subgraph "Lexical Subsystem"
        LE[LexicalEngine]
        Analyzer["Analyzers<br>(Standard, Lindera, etc)"]
        InvIdx[Inverted Index]
        
        LE --> Analyzer
        LE --> InvIdx
    end

    subgraph "Vector Subsystem"
        VE[VectorEngine]
        Embedder["Embedder<br>(Optional: OpenAI/Candle)"]
        HNSW[HNSW Graph]
        WAL[Write-Ahead Log]
        DelMgr["DeletionManager<br>(Bitmaps)"]

        VE --> Embedder
        VE --> HNSW
        VE --> WAL
        VE --> DelMgr
        VE --> Meta[Lexical Engine (Metadata)]
    end

    subgraph "Storage Layer"
        FS[FileStorage / Mmap]
    end

    %% Flows
    User -->|index_document| HE
    User -->|search| Req
    Req --> HE

    %% Indexing Flow
    HE -->|1. Check _id| LE
    HE -->|2. Assign Shard-Prefixed ID| ID
    HE -->|3. Upsert| LE
    HE -->|4. Upsert Payload| VE

    %% Search Flow
    HE -->|Hybrid Search| LE
    HE -->|Hybrid Search| VE
    LE -->|Top K Docs| RM
    VE -->|Top K Vectors| RM
    RM -->|"Fusion (RRF/Weighted)"| User

    %% Storage connections
    InvIdx -.-> FS
    HNSW -.-> FS
    WAL -.-> FS
    Manifest -.-> FS
```
