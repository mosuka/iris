"""Quickstart — Your first full-text search with Laurus.

This minimal example shows how to:
1. Create an in-memory search engine
2. Define a schema with text fields
3. Index documents
4. Search with a simple term query and a DSL string

Run with:
    maturin develop
    python examples/quickstart.py
"""

import laurus


def main() -> None:
    print("=== Laurus Quickstart ===\n")

    # 1. Define a schema
    schema = laurus.Schema()
    schema.add_text_field("title")
    schema.add_text_field("body")
    schema.set_default_fields(["title", "body"])

    # 2. Create an in-memory index
    index = laurus.Index(schema=schema)

    # 3. Index documents
    index.add_document(
        "doc1",
        {
            "title": "Introduction to Rust",
            "body": "Rust is a systems programming language focused on safety and performance.",
        },
    )
    index.add_document(
        "doc2",
        {
            "title": "Python for Data Science",
            "body": "Python is a versatile language widely used in data science and machine learning.",
        },
    )
    index.add_document(
        "doc3",
        {
            "title": "Web Development with JavaScript",
            "body": "JavaScript powers the modern web, from frontend frameworks to backend services.",
        },
    )
    index.commit()
    print("Indexed 3 documents.\n")

    # 4. Search with a TermQuery object
    print("[Search] TermQuery — 'rust' in body:")
    results = index.search(laurus.TermQuery("body", "rust"), limit=5)
    _print_results(results)

    # 5. Search with a DSL string
    print("\n[Search] DSL — 'language':")
    results = index.search("language", limit=5)
    _print_results(results)

    # 6. put_document replaces an existing document
    print("\n[Update] Replacing doc2 with put_document:")
    index.put_document(
        "doc2",
        {
            "title": "Python for Everything",
            "body": "Python is used in web, data science, scripting, and automation.",
        },
    )
    index.commit()

    print("[Search] DSL — 'automation' (should find updated doc2):")
    results = index.search("automation", limit=5)
    _print_results(results)

    print("\nQuickstart complete!")


def _print_results(results: list) -> None:
    if not results:
        print("  (no results)")
        return
    for r in results:
        title = (r.document or {}).get("title", "")
        print(f"  id={r.id!r:8s}  score={r.score:.4f}  title={title!r}")


if __name__ == "__main__":
    main()
