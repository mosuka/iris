"""Vector Search Example — semantic similarity search with embeddings.

Demonstrates vector search using pre-computed embeddings:
- Basic vector search (semantic similarity)
- Filtered vector search (with lexical filters)

In this example the embeddings are produced by `sentence-transformers`.
If that package is not installed, the example falls back to random vectors
so you can verify the API shape without additional dependencies.

Run with:
    pip install sentence-transformers   # optional but recommended
    maturin develop
    python examples/vector_search.py
"""

from __future__ import annotations

import math
import os
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
    print("=== Laurus Vector Search Example ===\n")
    print(f"Embedding model dimension: {_DIM}\n")

    # ── Schema ─────────────────────────────────────────────────────────────
    schema = laurus.Schema()
    schema.add_text_field("title")
    schema.add_text_field("text")
    schema.add_text_field("category")
    schema.add_integer_field("page")
    schema.add_flat_field("text_vec", dimension=_DIM, distance="cosine")
    schema.set_default_fields(["text"])

    index = laurus.Index(schema=schema)

    # ── Index ──────────────────────────────────────────────────────────────
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
    print("[A] Basic Vector Search: 'memory safety'")
    print("=" * 60)
    query_vec = embed("memory safety")
    _print_results(
        index.search(laurus.VectorQuery("text_vec", query_vec), limit=3)
    )

    # =====================================================================
    # [B] Filtered Vector Search — category filter
    # =====================================================================
    print("\n" + "=" * 60)
    print("[B] Filtered Vector Search: 'memory safety' + category='concurrency'")
    print("=" * 60)
    request = laurus.SearchRequest(
        vector_query=laurus.VectorQuery("text_vec", embed("memory safety")),
        filter_query=laurus.TermQuery("category", "concurrency"),
        limit=3,
    )
    _print_results(index.search(request))

    # =====================================================================
    # [C] Filtered Vector Search — numeric range filter (page <= 3)
    # =====================================================================
    print("\n" + "=" * 60)
    print("[C] Filtered Vector Search: 'type system' + page <= 3")
    print("=" * 60)
    request = laurus.SearchRequest(
        vector_query=laurus.VectorQuery("text_vec", embed("type system")),
        filter_query=laurus.NumericRangeQuery("page", min=1, max=3),
        limit=3,
    )
    _print_results(index.search(request))

    print("\nVector search example completed!")


def _print_results(results: list) -> None:
    if not results:
        print("  (no results)")
        return
    for r in results:
        doc = r.document or {}
        print(f"  id={r.id!r:8s}  score={r.score:.4f}  text={doc.get('text', '')!r:.60s}")


if __name__ == "__main__":
    main()
