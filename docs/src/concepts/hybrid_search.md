# Hybrid Search

Hybrid search combines the precision of Lexical Search (keyword matching) with the semantic understanding of Vector Search.

## Fusion Strategies

### 1. Weighted Sum
Scores from both engines are normalized and combined linearly.
`FinalScore = (LexicalScore * alpha) + (VectorScore * beta)`

### 2. RRF (Reciprocal Rank Fusion)
Relies on the **rank** of documents in each result set rather than their absolute scores. This is robust when score distributions vary wildly between engines.
`Score = 1 / (k + Rank_lexical) + 1 / (k + Rank_vector)`

### 3. Harmonic Mean
Combines scores using the harmonic mean formula, penalizing documents that only perform well in one engine.
