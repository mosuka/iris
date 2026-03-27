# frozen_string_literal: true

# Vector Search Example — semantic similarity search with embeddings.
#
# Demonstrates vector search using laurus's built-in CandleBert embedder:
# - Basic vector search (semantic similarity)
# - Filtered vector search (with lexical filters)
#
# The embedder is registered in the schema and laurus automatically converts
# text to vectors at index and query time — no external embedding library needed.
#
# Usage:
#   cd laurus-ruby
#   bundle exec rake compile  # build with: --features embeddings-candle
#   ruby -Ilib examples/vector_search.rb

require "laurus"

EMBEDDER_NAME = "bert"
EMBEDDER_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
DIM = 384 # dimension for all-MiniLM-L6-v2

CHUNKS = [
  ["book_a", "The Rust Programming Language", "Chapter 1: Getting Started", 1, "basics"],
  ["book_a", "The Rust Programming Language", "Cargo is the Rust build system and package manager. Use cargo new to create a crate.", 2, "basics"],
  ["book_a", "The Rust Programming Language", "Every value in Rust has an owner. Ownership rules prevent data races at compile time.", 3, "memory"],
  ["book_a", "The Rust Programming Language", "References and borrowing let you use values without taking ownership of them.", 4, "memory"],
  ["book_a", "The Rust Programming Language", "Generic types and trait bounds enable polymorphism without runtime overhead.", 5, "type-system"],
  ["book_a", "The Rust Programming Language", "Async functions and tokio provide concurrent programming with lightweight tasks.", 6, "concurrency"],
  ["book_b", "Programming in Rust", "Rust's type system catches many bugs at compile time. Trait objects enable dynamic dispatch.", 1, "type-system"],
  ["book_b", "Programming in Rust", "The borrow checker ensures memory safety without garbage collection. Lifetime annotations help.", 2, "memory"],
  ["book_b", "Programming in Rust", "Rust async/await provides zero-cost concurrency for building scalable concurrent services.", 3, "concurrency"]
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
  puts "=== Laurus Vector Search Example ===\n\n"
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
  # [A] Basic Vector Search
  # =====================================================================
  puts "=" * 60
  puts "[A] Basic Vector Search: 'memory safety'"
  puts "=" * 60
  print_results(index.search(Laurus::VectorTextQuery.new("text_vec", "memory safety"), limit: 3))

  # =====================================================================
  # [B] Filtered Vector Search — category filter
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[B] Filtered Vector Search: 'memory safety' + category='concurrency'"
  puts "=" * 60
  request = Laurus::SearchRequest.new(
    vector_query: Laurus::VectorTextQuery.new("text_vec", "memory safety"),
    filter_query: Laurus::TermQuery.new("category", "concurrency"),
    limit: 3
  )
  print_results(index.search(request))

  # =====================================================================
  # [C] Filtered Vector Search — numeric range filter (page <= 3)
  # =====================================================================
  puts "\n#{"=" * 60}"
  puts "[C] Filtered Vector Search: 'type system' + page <= 3"
  puts "=" * 60
  request = Laurus::SearchRequest.new(
    vector_query: Laurus::VectorTextQuery.new("text_vec", "type system"),
    filter_query: Laurus::NumericRangeQuery.new("page", min: 1, max: 3),
    limit: 3
  )
  print_results(index.search(request))

  puts "\nVector search example completed!"
end

main
