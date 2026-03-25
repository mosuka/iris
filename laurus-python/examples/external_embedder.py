"""External Embedder Example — vector and hybrid search with a user-managed embedder.

Demonstrates vector and hybrid search where embeddings are produced outside
laurus and passed as pre-computed vectors via ``VectorQuery``.

This approach is useful when you want to:
- Use any embedding library (sentence-transformers, HuggingFace, etc.)
- Control the embedding model independently of the index schema
- Reuse embeddings across multiple indexes

For an alternative that lets laurus handle embeddings automatically, see
``vector_search.py`` and ``hybrid_search.py`` which use the built-in
``candle_bert`` embedder via ``schema.add_embedder()``.

Run with:
    pip install sentence-transformers   # optional but recommended
    maturin develop
    python examples/external_embedder.py
"""

from __future__ import annotations

import math
import random

import laurus

# ---------------------------------------------------------------------------
# Embedding helper
# ---------------------------------------------------------------------------

try:
    from sentence_transformers import SentenceTransformer  # type: ignore

    _model = SentenceTransformer("all-MiniLM-L6-v2")
    _DIM = 384

    def embed(text: str) -> list[float]:
        return _model.encode(text, normalize_embeddings=True).tolist()

    print("Using sentence-transformers/all-MiniLM-L6-v2 for embeddings.\n")

except ImportError:
    # Fallback: deterministic pseudo-embeddings for demo purposes only.
    # Real similarity is not meaningful with these vectors.
    _DIM = 64

    def embed(text: str) -> list[float]:  # type: ignore[misc]
        rng = random.Random(hash(text) & 0xFFFFFFFF)
        raw = [rng.gauss(0, 1) for _ in range(_DIM)]
        norm = math.sqrt(sum(x * x for x in raw)) or 1.0
        return [x / norm for x in raw]

    print(
        "[NOTE] sentence-transformers not found — using random fallback vectors.\n"
        "       Results will NOT reflect semantic similarity.\n"
        "       Install with: pip install sentence-transformers\n"
    )

# ---------------------------------------------------------------------------
# Dataset
# ---------------------------------------------------------------------------

CHUNKS = [
    ("book_a", "The Rust Programming Language", "Chapter 1: Getting Started", 1, "basics"),
    ("book_a", "The Rust Programming Language", "Cargo is the Rust build system and package manager. Use cargo new to create a crate.", 2, "basics"),
    ("book_a", "The Rust Programming Language", "Every value in Rust has an owner. Ownership rules prevent data races at compile time.", 3, "memory"),
    ("book_a", "The Rust Programming Language", "References and borrowing let you use values without taking ownership of them.", 4, "memory"),
    ("book_a", "The Rust Programming Language", "Generic types and trait bounds enable polymorphism without runtime overhead.", 5, "type-system"),
    ("book_a", "The Rust Programming Language", "Async functions and tokio provide concurrent programming with lightweight tasks.", 6, "concurrency"),
    ("book_b", "Programming in Rust", "Rust's type system catches many bugs at compile time. Trait objects enable dynamic dispatch.", 1, "type-system"),
    ("book_b", "Programming in Rust", "The borrow checker ensures memory safety without garbage collection. Lifetime annotations help.", 2, "memory"),
    ("book_b", "Programming in Rust", "Rust async/await provides zero-cost concurrency for building scalable concurrent services.", 3, "concurrency"),
]


def main() -> None:
    print("=== Laurus External Embedder Example ===\n")
    print(f"Embedding model dimension: {_DIM}\n")

    # ── Schema ─────────────────────────────────────────────────────────────
    # No embedder is registered in the schema; vectors are provided by the
    # caller at index and query time via embed().
    schema = laurus.Schema()
    schema.add_text_field("title")
    schema.add_text_field("text")
    # keyword analyzer: treats hyphenated values like "type-system" as a single token
    schema.add_text_field("category", analyzer="keyword")
    schema.add_integer_field("page")
    schema.add_flat_field("text_vec", dimension=_DIM, distance="cosine")
    schema.set_default_fields(["text"])

    index = laurus.Index(schema=schema)

    # ── Index ──────────────────────────────────────────────────────────────
    # Embeddings are computed here in Python and stored as float vectors.
    print("--- Indexing chunked documents ---\n")
    for doc_id, title, text, page, category in CHUNKS:
        index.add_document(
            doc_id,
            {
                "title": title,
                "text": text,
                "category": category,
                "page": page,
                "text_vec": embed(text),
            },
        )
    index.commit()
    print(f"Indexed {len(CHUNKS)} chunks.\n")

    # =====================================================================
    # [A] Basic Vector Search
    # =====================================================================
    print("=" * 60)
    print("[A] Vector-only: 'memory safety'")
    print("=" * 60)
    _print_results(
        index.search(laurus.VectorQuery("text_vec", embed("memory safety")), limit=3)
    )

    # =====================================================================
    # [B] Filtered Vector Search — category filter
    # =====================================================================
    print("\n" + "=" * 60)
    print("[B] Filtered vector: 'memory safety' + category='concurrency'")
    print("=" * 60)
    request = laurus.SearchRequest(
        vector_query=laurus.VectorQuery("text_vec", embed("memory safety")),
        filter_query=laurus.TermQuery("category", "concurrency"),
        limit=3,
    )
    _print_results(index.search(request))

    # =====================================================================
    # [C] Hybrid search — RRF Fusion
    # =====================================================================
    print("\n" + "=" * 60)
    print("[C] Hybrid (RRF k=60): vector='concurrent' + lexical='async'")
    print("=" * 60)
    request = laurus.SearchRequest(
        lexical_query=laurus.TermQuery("text", "async"),
        vector_query=laurus.VectorQuery("text_vec", embed("concurrent")),
        fusion=laurus.RRF(k=60.0),
        limit=3,
    )
    _print_results(index.search(request))

    # =====================================================================
    # [D] Hybrid search — WeightedSum Fusion
    # =====================================================================
    print("\n" + "=" * 60)
    print("[D] Hybrid (WeightedSum 0.3/0.7): vector='memory safety' + lexical='safety'")
    print("=" * 60)
    request = laurus.SearchRequest(
        lexical_query=laurus.TermQuery("text", "safety"),
        vector_query=laurus.VectorQuery("text_vec", embed("memory safety")),
        fusion=laurus.WeightedSum(lexical_weight=0.3, vector_weight=0.7),
        limit=3,
    )
    _print_results(index.search(request))

    # =====================================================================
    # [E] Hybrid search with filter
    # =====================================================================
    print("\n" + "=" * 60)
    print("[E] Hybrid + filter: vector='type system' + lexical='trait' + category='type-system'")
    print("=" * 60)
    request = laurus.SearchRequest(
        lexical_query=laurus.TermQuery("text", "trait"),
        vector_query=laurus.VectorQuery("text_vec", embed("type system")),
        filter_query=laurus.TermQuery("category", "type-system"),
        fusion=laurus.RRF(k=60.0),
        limit=3,
    )
    _print_results(index.search(request))

    print("\nExternal embedder example completed!")


def _print_results(results: list) -> None:
    if not results:
        print("  (no results)")
        return
    for r in results:
        doc = r.document or {}
        print(f"  id={r.id!r:8s}  score={r.score:.4f}  text={doc.get('text', '')!r:.60s}")


if __name__ == "__main__":
    main()
