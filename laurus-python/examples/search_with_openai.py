"""Search with OpenAI Embedder — real vector search using OpenAI's API.

This example mirrors `search_with_openai.rs` but produces embeddings on the
Python side using the `openai` SDK, then passes the resulting vectors to
Laurus as `VectorQuery`.

Prerequisites:
    pip install openai
    export OPENAI_API_KEY=your-api-key-here
    maturin develop
    python examples/search_with_openai.py
"""

from __future__ import annotations

import os
import sys

import laurus

# ---------------------------------------------------------------------------
# OpenAI embedding helper
# ---------------------------------------------------------------------------

try:
    from openai import OpenAI  # type: ignore
except ImportError:
    print("ERROR: openai package not installed.")
    print("       Install with: pip install openai")
    sys.exit(1)

_MODEL = "text-embedding-3-small"
_DIM = 1536


def embed(client: "OpenAI", text: str) -> list[float]:
    """Call OpenAI Embeddings API and return the vector."""
    response = client.embeddings.create(input=text, model=_MODEL)
    return response.data[0].embedding


# ---------------------------------------------------------------------------
# Dataset (same as the Rust example)
# ---------------------------------------------------------------------------

CHUNKS = [
    ("book_a", "The Rust Programming Language", "Chapter 1: Getting Started", 1, "basics"),
    ("book_a", "The Rust Programming Language", "Cargo is the Rust build system and package manager. Use cargo new to create a crate.", 2, "basics"),
    ("book_a", "The Rust Programming Language", "Every value in Rust has an owner. Ownership rules prevent data races at compile time.", 3, "memory"),
    ("book_a", "The Rust Programming Language", "References and borrowing let you use values without taking ownership of them.", 4, "memory"),
    ("book_a", "The Rust Programming Language", "Generic types and trait bounds enable polymorphism without runtime overhead.", 5, "type-system"),
    ("book_a", "The Rust Programming Language", "Async functions and tokio provide concurrent programming with lightweight tasks and threads.", 6, "concurrency"),
    ("book_b", "Programming in Rust", "Rust's type system catches many bugs at compile time. Trait objects enable dynamic dispatch.", 1, "type-system"),
    ("book_b", "Programming in Rust", "The borrow checker ensures memory safety without garbage collection. Lifetime annotations help.", 2, "memory"),
    ("book_b", "Programming in Rust", "Rust async/await provides zero-cost concurrency for building scalable concurrent network services.", 3, "concurrency"),
]


def main() -> None:
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        print("ERROR: OPENAI_API_KEY environment variable not set.")
        print("       export OPENAI_API_KEY=your-api-key-here")
        sys.exit(1)

    print("=== Laurus Search with OpenAI Embedder ===\n")

    client = OpenAI(api_key=api_key)
    print(f"OpenAI embedder ready (model={_MODEL}, dim={_DIM})\n")

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
        vec = embed(client, text)
        index.add_document(
            doc_id,
            {
                "title": title,
                "text": text,
                "category": category,
                "page": page,
                "text_vec": vec,
            },
        )
        print(f"  Indexed {doc_id} page {page}: {text[:50]!r}...")
    index.commit()
    print(f"\nIndexed {len(CHUNKS)} chunks.\n")

    # =====================================================================
    # [A] Vector Search
    # =====================================================================
    print("=" * 60)
    print("[A] Vector Search: 'memory safety'")
    print("=" * 60)
    _print_results(
        index.search(laurus.VectorQuery("text_vec", embed(client, "memory safety")), limit=3)
    )

    # =====================================================================
    # [B] Filtered Vector Search — category filter
    # =====================================================================
    print("\n" + "=" * 60)
    print("[B] Filtered Vector Search: 'memory safety' + category='concurrency'")
    print("=" * 60)
    request = laurus.SearchRequest(
        vector_query=laurus.VectorQuery("text_vec", embed(client, "memory safety")),
        filter_query=laurus.TermQuery("category", "concurrency"),
        limit=3,
    )
    _print_results(index.search(request))

    # =====================================================================
    # [C] Filtered Vector Search — numeric range filter
    # =====================================================================
    print("\n" + "=" * 60)
    print("[C] Filtered Vector Search: 'type system' + page <= 3")
    print("=" * 60)
    request = laurus.SearchRequest(
        vector_query=laurus.VectorQuery("text_vec", embed(client, "type system")),
        filter_query=laurus.NumericRangeQuery("page", min=1, max=3),
        limit=3,
    )
    _print_results(index.search(request))

    # =====================================================================
    # [D] Lexical Search
    # =====================================================================
    print("\n" + "=" * 60)
    print("[D] Lexical Search: 'ownership'")
    print("=" * 60)
    _print_results(index.search(laurus.TermQuery("text", "ownership"), limit=3))

    # =====================================================================
    # [E] Hybrid Search (RRF)
    # =====================================================================
    print("\n" + "=" * 60)
    print("[E] Hybrid Search (RRF): vector='concurrent' + lexical='async'")
    print("=" * 60)
    request = laurus.SearchRequest(
        lexical_query=laurus.TermQuery("text", "async"),
        vector_query=laurus.VectorQuery("text_vec", embed(client, "concurrent")),
        fusion=laurus.RRF(k=60.0),
        limit=3,
    )
    _print_results(index.search(request))

    print("\nSearch with OpenAI example completed!")


def _print_results(results: list) -> None:
    if not results:
        print("  (no results)")
        return
    for r in results:
        doc = r.document or {}
        text = doc.get("text", "")
        print(f"  id={r.id!r:8s}  score={r.score:.4f}  text={text!r:.60s}")


if __name__ == "__main__":
    main()
