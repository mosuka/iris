# frozen_string_literal: true

# Lexical Search Example — all query types.
#
# Demonstrates every lexical query type Laurus supports:
#
# 1. TermQuery         — exact single-term matching
# 2. PhraseQuery       — exact word sequence matching
# 3. FuzzyQuery        — approximate matching (typo tolerance)
# 4. WildcardQuery     — pattern matching with * and ?
# 5. NumericRangeQuery — numeric range filtering (int and float)
# 6. GeoQuery          — geographic radius / bounding box
# 7. BooleanQuery      — AND / OR / NOT combinations
# 8. SpanQuery         — positional / proximity search
#
# Usage:
#   cd laurus-ruby
#   bundle exec rake compile
#   ruby -Ilib examples/lexical_search.rb

require "laurus"

def print_results(results)
  if results.empty?
    puts "  (no results)"
    return
  end
  results.each do |r|
    title = r.document&.fetch("title", "") || ""
    printf "  id=%-8s  score=%.4f  title=%s\n", r.id.inspect, r.score, title.inspect
  end
end

def main
  puts "=== Laurus Lexical Search Example ===\n\n"

  # ── Setup ──────────────────────────────────────────────────────────────
  schema = Laurus::Schema.new
  schema.add_text_field("title")
  schema.add_text_field("body")
  schema.add_text_field("category", analyzer: "keyword")
  schema.add_text_field("filename", analyzer: "keyword")
  schema.add_boolean_field("in_print")
  schema.add_float_field("price")
  schema.add_integer_field("year")
  schema.add_geo_field("location")
  schema.set_default_fields(%w[body])

  index = Laurus::Index.new(schema: schema)

  # ── Index documents ────────────────────────────────────────────────────
  docs = [
    ["book1", {
      "title" => "The Rust Programming Language",
      "body" => "Rust is a systems programming language focused on safety, speed, and concurrency",
      "category" => "programming",
      "filename" => "rust_book.pdf",
      "in_print" => true,
      "price" => 49.99,
      "year" => 2019,
      "location" => { "lat" => 37.7749, "lon" => -122.4194 } # San Francisco
    }],
    ["book2", {
      "title" => "Python for Data Science",
      "body" => "Python is a versatile programming language widely used in data science and machine learning",
      "category" => "data-science",
      "filename" => "python_data.epub",
      "in_print" => true,
      "price" => 39.99,
      "year" => 2021,
      "location" => { "lat" => 40.7128, "lon" => -74.0060 } # New York
    }],
    ["book3", {
      "title" => "JavaScript Web Development",
      "body" => "JavaScript powers the modern web from frontend frameworks to backend services",
      "category" => "web-development",
      "filename" => "javascript_web.pdf",
      "in_print" => true,
      "price" => 54.99,
      "year" => 2022,
      "location" => { "lat" => 51.5074, "lon" => -0.1278 } # London
    }],
    ["book4", {
      "title" => "Machine Learning Algorithms",
      "body" => "Understanding algorithms used in machine learning and artificial intelligence applications",
      "category" => "data-science",
      "filename" => "ml_algorithms.docx",
      "in_print" => true,
      "price" => 72.99,
      "year" => 2020,
      "location" => { "lat" => 37.4419, "lon" => -122.1430 } # Palo Alto
    }],
    ["book5", {
      "title" => "Database Design Principles",
      "body" => "Learn database design, SQL queries, and data management for modern applications",
      "category" => "database",
      "filename" => "db_design.pdf",
      "in_print" => false,
      "price" => 45.50,
      "year" => 2018,
      "location" => { "lat" => 47.6062, "lon" => -122.3321 } # Seattle
    }],
    ["book6", {
      "title" => "The quick brown fox",
      "body" => "The quick brown fox jumped over the lazy dog in a sunny meadow",
      "category" => "fiction",
      "filename" => "fox_story.txt",
      "in_print" => false,
      "price" => 12.99,
      "year" => 2023,
      "location" => { "lat" => 34.0522, "lon" => -118.2437 } # Los Angeles
    }]
  ]

  puts "  Indexing #{docs.size} documents..."
  docs.each { |doc_id, doc| index.add_document(doc_id, doc) }
  index.commit
  puts "  Done.\n\n"

  # =====================================================================
  # PART 1: TermQuery — exact single-term matching
  # =====================================================================
  puts "=" * 60
  puts "PART 1: TermQuery"
  puts "=" * 60

  puts "\n[1a] Search for 'rust' in body:"
  print_results(index.search(Laurus::TermQuery.new("body", "rust"), limit: 5))

  puts "\n[1b] Search for 'programming' in category (exact):"
  print_results(index.search(Laurus::TermQuery.new("category", "programming"), limit: 5))

  puts "\n[1c] Search for in_print=true (boolean field):"
  print_results(index.search(Laurus::TermQuery.new("in_print", "true"), limit: 5))

  puts "\n[1d] DSL: 'body:rust':"
  print_results(index.search("body:rust", limit: 5))

  # =====================================================================
  # PART 2: PhraseQuery — exact word sequence
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 2: PhraseQuery"
  puts "=" * 60

  puts "\n[2a] Phrase 'machine learning' in body:"
  print_results(index.search(Laurus::PhraseQuery.new("body", %w[machine learning]), limit: 5))

  puts "\n[2b] Phrase 'systems programming language' in body:"
  print_results(index.search(Laurus::PhraseQuery.new("body", %w[systems programming language]), limit: 5))

  puts "\n[2c] DSL: 'body:\"machine learning\"':"
  print_results(index.search('body:"machine learning"', limit: 5))

  # =====================================================================
  # PART 3: FuzzyQuery — approximate matching (typo tolerance)
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 3: FuzzyQuery"
  puts "=" * 60

  puts "\n[3a] Fuzzy 'programing' (missing 'm', max_edits=2):"
  print_results(index.search(Laurus::FuzzyQuery.new("body", "programing", max_edits: 2), limit: 5))

  puts "\n[3b] Fuzzy 'javascritp' (transposed, max_edits=1):"
  print_results(index.search(Laurus::FuzzyQuery.new("body", "javascritp", max_edits: 1), limit: 5))

  puts "\n[3c] DSL: 'programing~2':"
  print_results(index.search("programing~2", limit: 5))

  # =====================================================================
  # PART 4: WildcardQuery — pattern matching with * and ?
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 4: WildcardQuery"
  puts "=" * 60

  puts "\n[4a] Wildcard '*.pdf' in filename:"
  print_results(index.search(Laurus::WildcardQuery.new("filename", "*.pdf"), limit: 5))

  puts "\n[4b] Wildcard 'pro*' in body:"
  print_results(index.search(Laurus::WildcardQuery.new("body", "pro*"), limit: 5))

  puts "\n[4c] DSL: 'body:pro*':"
  print_results(index.search("body:pro*", limit: 5))

  # =====================================================================
  # PART 5: NumericRangeQuery — numeric range filtering
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 5: NumericRangeQuery"
  puts "=" * 60

  puts "\n[5a] Books with price $40-$60 (float range):"
  print_results(index.search(Laurus::NumericRangeQuery.new("price", min: 40.0, max: 60.0), limit: 5))

  puts "\n[5b] Books published from 2021 onwards (integer range):"
  print_results(index.search(Laurus::NumericRangeQuery.new("year", min: 2021), limit: 5))

  puts "\n[5c] DSL: 'price:[40 TO 60]':"
  print_results(index.search("price:[40 TO 60]", limit: 5))

  # =====================================================================
  # PART 6: GeoQuery — geographic search
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 6: GeoQuery (no DSL equivalent)"
  puts "=" * 60

  puts "\n[6a] Within 100 km of San Francisco (37.77, -122.42):"
  print_results(index.search(Laurus::GeoQuery.within_radius("location", 37.7749, -122.4194, 100.0), limit: 5))

  puts "\n[6b] Bounding box — US West Coast (33, -123) to (48, -117):"
  print_results(index.search(Laurus::GeoQuery.within_bounding_box("location", 33.0, -123.0, 48.0, -117.0), limit: 5))

  # =====================================================================
  # PART 7: BooleanQuery — AND / OR / NOT combinations
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 7: BooleanQuery"
  puts "=" * 60

  puts "\n[7a] AND: 'programming' in body AND category='data-science':"
  bq = Laurus::BooleanQuery.new
  bq.must(Laurus::TermQuery.new("body", "programming"))
  bq.must(Laurus::TermQuery.new("category", "data-science"))
  print_results(index.search(bq, limit: 5))

  puts "\n[7b] OR: category='programming' OR category='web-development':"
  bq = Laurus::BooleanQuery.new
  bq.should(Laurus::TermQuery.new("category", "programming"))
  bq.should(Laurus::TermQuery.new("category", "web-development"))
  print_results(index.search(bq, limit: 5))

  puts "\n[7c] NOT: 'programming' in body, NOT 'python':"
  bq = Laurus::BooleanQuery.new
  bq.must(Laurus::TermQuery.new("body", "programming"))
  bq.must_not(Laurus::TermQuery.new("body", "python"))
  print_results(index.search(bq, limit: 5))

  puts "\n[7d] DSL: '+body:programming -body:python':"
  print_results(index.search("+body:programming -body:python", limit: 5))

  # =====================================================================
  # PART 8: SpanQuery — positional / proximity search
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "PART 8: SpanQuery (no DSL equivalent)"
  puts "=" * 60

  puts "\n[8a] SpanNear: 'quick' near 'fox' (slop=1, ordered):"
  span_q = Laurus::SpanQuery.near("body", %w[quick fox], slop: 1, ordered: true)
  print_results(index.search(span_q, limit: 5))

  puts "\n[8b] SpanContaining: 'quick..fox' containing 'brown':"
  big = Laurus::SpanQuery.near("body", %w[quick fox], slop: 1, ordered: true)
  little = Laurus::SpanQuery.term("body", "brown")
  containing = Laurus::SpanQuery.containing("body", big, little)
  print_results(index.search(containing, limit: 5))

  puts "\nLexical search example completed!"
end

main
