# frozen_string_literal: true

# Hybrid Search Example — combining lexical and vector search.
#
# Demonstrates:
# - Lexical-only search (for comparison)
# - Vector-only search (for comparison)
# - Hybrid search with RRF fusion
# - Hybrid search with WeightedSum fusion
# - Hybrid search with a filter query
#
# The embedder is registered in the schema and laurus automatically converts
# text to vectors at index and query time — no external embedding library needed.
#
# Usage:
#   cd laurus-ruby
#   bundle exec rake compile  # build with: --features embeddings-candle
#   ruby -Ilib examples/hybrid_search.rb

require "laurus"

EMBEDDER_NAME = "bert"
EMBEDDER_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
DIM = 384 # dimension for all-MiniLM-L6-v2

CHUNKS = [
  ["book_a", "The Rust Programming Language", "Chapter 1: Getting Started", 1, "basics"],
  ["book_a", "The Rust Programming Language", "Cargo is the Rust build system and package manager.", 2, "basics"],
  ["book_a", "The Rust Programming Language", "Every value in Rust has an owner. Ownership rules prevent data races at compile time.", 3, "memory"],
  ["book_a", "The Rust Programming Language", "References and borrowing let you use values without taking ownership of them.", 4, "memory"],
  ["book_a", "The Rust Programming Language", "Generic types and trait bounds enable polymorphism without runtime overhead.", 5, "type-system"],
  ["book_a", "The Rust Programming Language", "Async functions and tokio provide concurrent programming with lightweight tasks and threads.", 6, "concurrency"],
  ["book_b", "Programming in Rust", "Rust's type system catches many bugs at compile time. Trait objects enable dynamic dispatch.", 1, "type-system"],
  ["book_b", "Programming in Rust", "The borrow checker ensures memory safety without garbage collection.", 2, "memory"],
  ["book_b", "Programming in Rust", "Rust async/await provides zero-cost concurrency for scalable concurrent network services.", 3, "concurrency"]
].freeze

def print_results(results)
  if results.empty?
    puts "  (no results)"
    return
  end
  results.each do |r|
    doc = r.document || {}
    text = (doc["text"] || "")[0, 60]
    printf "  id=%-8s  score=%.4f  text=%s\n", r.id.inspect, r.score, text.inspect
  end
end

def main
  puts "=== Laurus Hybrid Search Example ===\n\n"
  puts "Embedder: #{EMBEDDER_MODEL} (dim=#{DIM})\n\n"

  # ── Schema ─────────────────────────────────────────────────────────────
  schema = Laurus::Schema.new
  schema.add_embedder(EMBEDDER_NAME, { "type" => "candle_bert", "model" => EMBEDDER_MODEL })
  schema.add_text_field("title")
  schema.add_text_field("text")
  schema.add_text_field("category")
  schema.add_integer_field("page")
  schema.add_flat_field("text_vec", DIM, distance: "cosine", embedder: EMBEDDER_NAME)
  schema.set_default_fields(%w[text])

  index = Laurus::Index.new(schema: schema)

  # ── Index ──────────────────────────────────────────────────────────────
  puts "--- Indexing chunked documents ---\n\n"
  CHUNKS.each do |doc_id, title, text, page, category|
    index.add_document(doc_id, {
      "title" => title,
      "text" => text,
      "category" => category,
      "page" => page,
      "text_vec" => text
    })
  end
  index.commit
  puts "Indexed #{CHUNKS.size} chunks.\n\n"

  # =====================================================================
  # [A] Lexical-only search (baseline)
  # =====================================================================
  puts "=" * 60
  puts "[A] Lexical-only: term 'ownership' in text"
  puts "=" * 60
  print_results(index.search(Laurus::TermQuery.new("text", "ownership"), limit: 3))

  # =====================================================================
  # [B] Vector-only search (baseline)
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[B] Vector-only: 'memory safety'"
  puts "=" * 60
  print_results(index.search(Laurus::VectorTextQuery.new("text_vec", "memory safety"), limit: 3))

  # =====================================================================
  # [C] Hybrid search — RRF Fusion
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[C] Hybrid (RRF k=60): vector='concurrent' + lexical='async'"
  puts "=" * 60
  request = Laurus::SearchRequest.new(
    lexical_query: Laurus::TermQuery.new("text", "async"),
    vector_query: Laurus::VectorTextQuery.new("text_vec", "concurrent"),
    fusion: Laurus::RRF.new(k: 60.0),
    limit: 3
  )
  print_results(index.search(request))

  # =====================================================================
  # [D] Hybrid search — WeightedSum Fusion
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[D] Hybrid (WeightedSum 0.3/0.7): vector='memory safety' + lexical='safety'"
  puts "=" * 60
  request = Laurus::SearchRequest.new(
    lexical_query: Laurus::TermQuery.new("text", "safety"),
    vector_query: Laurus::VectorTextQuery.new("text_vec", "memory safety"),
    fusion: Laurus::WeightedSum.new(lexical_weight: 0.3, vector_weight: 0.7),
    limit: 3
  )
  print_results(index.search(request))

  # =====================================================================
  # [E] Hybrid search with filter query
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[E] Hybrid + filter: vector='type system' + lexical='trait' + category='type-system'"
  puts "=" * 60
  request = Laurus::SearchRequest.new(
    lexical_query: Laurus::TermQuery.new("text", "trait"),
    vector_query: Laurus::VectorTextQuery.new("text_vec", "type system"),
    filter_query: Laurus::TermQuery.new("category", "type-system"),
    fusion: Laurus::RRF.new(k: 60.0),
    limit: 3
  )
  print_results(index.search(request))

  # =====================================================================
  # [F] DSL string query
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[F] DSL query string: 'text:async'"
  puts "=" * 60
  print_results(index.search("text:async", limit: 3))

  puts "\nHybrid search example completed!"
end

main
