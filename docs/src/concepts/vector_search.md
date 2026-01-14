# Vector Search

Vector search (finding "nearest neighbors") allows for semantic retrieval where matches are based on meaning rather than exact keywords.

## HNSW (Hierarchical Navigable Small World)
Sarissa primarily uses HNSW for fast approximate nearest neighbor search. It builds a multi-layered graph structure to navigate the vector space efficiently.

## Flat (Brute Force)
For scenarios requiring 100% recall (exact nearest neighbors) or for small datasets, Sarissa supports a Flat index.
- Performs a brute-force scan of all vectors.
- No approximation error (Recall = 1.0).
- Slower search speed compared to HNSW as dataset grows (O(N)).

## IVF (Inverted File Index)
IVF accelerates search by clustering vectors into partitions (Voronoi cells).
- **Training**: Requires a training phase to compute cluster centroids (e.g., K-Means).
- **Indexing**: Assigns vectors to the nearest centroid.
- **Search**: Restricts the search to the nearest `nprobe` clusters, significantly reducing the search space.
- Good balance between speed and recall, efficient for very large datasets where HNSW graph size might be an issue.

## Payloads
Vectors are stored as **Payloads**.
- **Vector Source**: You can provide pre-computed `Vec<f32>`.
- **Text Source**: You can provide raw text, and if an embedder is configured, Sarissa will compute the vector for you.

## Distance Metrics
- **Cosine**: Measures the angle between vectors (normalized dot product). Good for semantic text similarity.
- **L2 (Euclidean)**: Measures the straight-line distance.
- **DotProduct**: Optimized for normalized vectors.
