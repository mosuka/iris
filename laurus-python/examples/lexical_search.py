"""Lexical Search Example — all query types.

Demonstrates every lexical query type Laurus supports:

1. TermQuery         — exact single-term matching
2. PhraseQuery       — exact word sequence matching
3. FuzzyQuery        — approximate matching (typo tolerance)
4. WildcardQuery     — pattern matching with * and ?
5. NumericRangeQuery — numeric range filtering (int and float)
6. GeoQuery          — geographic radius / bounding box
7. BooleanQuery      — AND / OR / NOT combinations
8. SpanQuery         — positional / proximity search

Run with:
    maturin develop
    python examples/lexical_search.py
"""

import laurus


def main() -> None:
    print("=== Laurus Lexical Search Example ===\n")

    # ── Setup ──────────────────────────────────────────────────────────────
    schema = laurus.Schema()
    schema.add_text_field("title")
    schema.add_text_field("body")
    schema.add_text_field("category", analyzer="keyword")
    schema.add_text_field("filename", analyzer="keyword")
    schema.add_boolean_field("in_print")
    schema.add_float_field("price")
    schema.add_integer_field("year")
    schema.add_geo_field("location")
    schema.set_default_fields(["body"])

    index = laurus.Index(schema=schema)

    # ── Index documents ────────────────────────────────────────────────────
    docs = [
        (
            "book1",
            {
                "title": "The Rust Programming Language",
                "body": "Rust is a systems programming language focused on safety, speed, and concurrency",
                "category": "programming",
                "filename": "rust_book.pdf",
                "in_print": True,
                "price": 49.99,
                "year": 2019,
                "location": (37.7749, -122.4194),  # San Francisco
            },
        ),
        (
            "book2",
            {
                "title": "Python for Data Science",
                "body": "Python is a versatile programming language widely used in data science and machine learning",
                "category": "data-science",
                "filename": "python_data.epub",
                "in_print": True,
                "price": 39.99,
                "year": 2021,
                "location": (40.7128, -74.0060),  # New York
            },
        ),
        (
            "book3",
            {
                "title": "JavaScript Web Development",
                "body": "JavaScript powers the modern web from frontend frameworks to backend services",
                "category": "web-development",
                "filename": "javascript_web.pdf",
                "in_print": True,
                "price": 54.99,
                "year": 2022,
                "location": (51.5074, -0.1278),  # London
            },
        ),
        (
            "book4",
            {
                "title": "Machine Learning Algorithms",
                "body": "Understanding algorithms used in machine learning and artificial intelligence applications",
                "category": "data-science",
                "filename": "ml_algorithms.docx",
                "in_print": True,
                "price": 72.99,
                "year": 2020,
                "location": (37.4419, -122.1430),  # Palo Alto
            },
        ),
        (
            "book5",
            {
                "title": "Database Design Principles",
                "body": "Learn database design, SQL queries, and data management for modern applications",
                "category": "database",
                "filename": "db_design.pdf",
                "in_print": False,
                "price": 45.50,
                "year": 2018,
                "location": (47.6062, -122.3321),  # Seattle
            },
        ),
        (
            "book6",
            {
                "title": "The quick brown fox",
                "body": "The quick brown fox jumped over the lazy dog in a sunny meadow",
                "category": "fiction",
                "filename": "fox_story.txt",
                "in_print": False,
                "price": 12.99,
                "year": 2023,
                "location": (34.0522, -118.2437),  # Los Angeles
            },
        ),
    ]

    print(f"  Indexing {len(docs)} documents...")
    for doc_id, doc in docs:
        index.add_document(doc_id, doc)
    index.commit()
    print("  Done.\n")

    # =====================================================================
    # PART 1: TermQuery — exact single-term matching
    # =====================================================================
    print("=" * 60)
    print("PART 1: TermQuery")
    print("=" * 60)

    print("\n[1a] Search for 'rust' in body:")
    _print_results(index.search(laurus.TermQuery("body", "rust"), limit=5))

    print("\n[1b] Search for 'programming' in category (exact):")
    _print_results(index.search(laurus.TermQuery("category", "programming"), limit=5))

    print("\n[1c] Search for in_print=true (boolean field):")
    _print_results(index.search(laurus.TermQuery("in_print", "true"), limit=5))

    print("\n[1d] DSL: 'body:rust':")
    _print_results(index.search("body:rust", limit=5))

    # =====================================================================
    # PART 2: PhraseQuery — exact word sequence
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 2: PhraseQuery")
    print("=" * 60)

    print("\n[2a] Phrase 'machine learning' in body:")
    _print_results(
        index.search(laurus.PhraseQuery("body", ["machine", "learning"]), limit=5)
    )

    print("\n[2b] Phrase 'systems programming language' in body:")
    _print_results(
        index.search(
            laurus.PhraseQuery("body", ["systems", "programming", "language"]),
            limit=5,
        )
    )

    print("\n[2c] DSL: 'body:\"machine learning\"':")
    _print_results(index.search('body:"machine learning"', limit=5))

    # =====================================================================
    # PART 3: FuzzyQuery — approximate matching (typo tolerance)
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 3: FuzzyQuery")
    print("=" * 60)

    print("\n[3a] Fuzzy 'programing' (missing 'm', max_edits=2):")
    _print_results(
        index.search(laurus.FuzzyQuery("body", "programing", max_edits=2), limit=5)
    )

    print("\n[3b] Fuzzy 'javascritp' (transposed, max_edits=1):")
    _print_results(
        index.search(laurus.FuzzyQuery("body", "javascritp", max_edits=1), limit=5)
    )

    print("\n[3c] DSL: 'programing~2':")
    _print_results(index.search("programing~2", limit=5))

    # =====================================================================
    # PART 4: WildcardQuery — pattern matching with * and ?
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 4: WildcardQuery")
    print("=" * 60)

    print("\n[4a] Wildcard '*.pdf' in filename:")
    _print_results(index.search(laurus.WildcardQuery("filename", "*.pdf"), limit=5))

    print("\n[4b] Wildcard 'pro*' in body:")
    _print_results(index.search(laurus.WildcardQuery("body", "pro*"), limit=5))

    print("\n[4c] DSL: 'body:pro*':")
    _print_results(index.search("body:pro*", limit=5))

    # =====================================================================
    # PART 5: NumericRangeQuery — numeric range filtering
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 5: NumericRangeQuery")
    print("=" * 60)

    print("\n[5a] Books with price $40–$60 (float range):")
    _print_results(
        index.search(
            laurus.NumericRangeQuery("price", min=40.0, max=60.0), limit=5
        )
    )

    print("\n[5b] Books published from 2021 onwards (integer range):")
    _print_results(
        index.search(laurus.NumericRangeQuery("year", min=2021), limit=5)
    )

    print("\n[5c] DSL: 'price:[40 TO 60]':")
    _print_results(index.search("price:[40 TO 60]", limit=5))

    # =====================================================================
    # PART 6: GeoQuery — geographic search
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 6: GeoQuery (no DSL equivalent)")
    print("=" * 60)

    print("\n[6a] Within 100 km of San Francisco (37.77, -122.42):")
    _print_results(
        index.search(
            laurus.GeoQuery.within_radius("location", 37.7749, -122.4194, 100.0),
            limit=5,
        )
    )

    print("\n[6b] Bounding box — US West Coast (33, -123) to (48, -117):")
    _print_results(
        index.search(
            laurus.GeoQuery.within_bounding_box(
                "location", 33.0, -123.0, 48.0, -117.0
            ),
            limit=5,
        )
    )

    # =====================================================================
    # PART 7: BooleanQuery — AND / OR / NOT combinations
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 7: BooleanQuery")
    print("=" * 60)

    print("\n[7a] AND: 'programming' in body AND category='data-science':")
    bq = laurus.BooleanQuery()
    bq.must(laurus.TermQuery("body", "programming"))
    bq.must(laurus.TermQuery("category", "data-science"))
    _print_results(index.search(bq, limit=5))

    print("\n[7b] OR: category='programming' OR category='web-development':")
    bq = laurus.BooleanQuery()
    bq.should(laurus.TermQuery("category", "programming"))
    bq.should(laurus.TermQuery("category", "web-development"))
    _print_results(index.search(bq, limit=5))

    print("\n[7c] NOT: 'programming' in body, NOT 'python':")
    bq = laurus.BooleanQuery()
    bq.must(laurus.TermQuery("body", "programming"))
    bq.must_not(laurus.TermQuery("body", "python"))
    _print_results(index.search(bq, limit=5))

    print("\n[7d] DSL: '+body:programming -body:python':")
    _print_results(index.search("+body:programming -body:python", limit=5))

    # =====================================================================
    # PART 8: SpanQuery — positional / proximity search
    # =====================================================================
    print("\n" + "=" * 60)
    print("PART 8: SpanQuery (no DSL equivalent)")
    print("=" * 60)

    print("\n[8a] SpanNear: 'quick' near 'fox' (slop=1, ordered):")
    span_q = laurus.SpanQuery.near("body", ["quick", "fox"], slop=1, ordered=True)
    _print_results(index.search(span_q, limit=5))

    print("\n[8b] SpanContaining: 'quick..fox' containing 'brown':")
    big = laurus.SpanQuery.near("body", ["quick", "fox"], slop=1, ordered=True)
    little = laurus.SpanQuery.term("body", "brown")
    containing = laurus.SpanQuery.containing("body", big, little)
    _print_results(index.search(containing, limit=5))

    print("\nLexical search example completed!")


def _print_results(results: list) -> None:
    if not results:
        print("  (no results)")
        return
    for r in results:
        title = (r.document or {}).get("title", "")
        print(f"  id={r.id!r:8s}  score={r.score:.4f}  title={title!r}")


if __name__ == "__main__":
    main()
