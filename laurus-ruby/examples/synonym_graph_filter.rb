# frozen_string_literal: true

# Synonym Graph Filter Example — token expansion with synonyms.
#
# Demonstrates:
# - Creating a synonym dictionary with bidirectional synonym groups
# - Tokenizing text with WhitespaceTokenizer
# - Applying SynonymGraphFilter without boost
# - Applying SynonymGraphFilter with boost < 1.0 (synonyms weighted down)
#
# Usage:
#   cd laurus-ruby
#   bundle exec rake compile
#   ruby -Ilib examples/synonym_graph_filter.rb

require "laurus"

def main
  puts "=== SynonymGraphFilter Usage Example ===\n\n"

  # ── Step 1: Create synonym dictionary ─────────────────────────────────
  puts "Step 1: Creating synonym dictionary"
  syn_dict = Laurus::SynonymDictionary.new
  syn_dict.add_synonym_group(%w[ml machine\ learning])
  syn_dict.add_synonym_group(%w[ai artificial\ intelligence])

  puts "  Added synonyms:"
  puts "    - 'ml' <-> 'machine learning'"
  puts "    - 'ai' <-> 'artificial intelligence'\n\n"

  tokenizer = Laurus::WhitespaceTokenizer.new

  # ── Step 2: Apply filter WITHOUT boost ────────────────────────────────
  puts "Step 2: Applying filter WITHOUT boost"
  filt = Laurus::SynonymGraphFilter.new(syn_dict, keep_original: true, boost: 1.0)

  input_text = "ml tutorial"
  puts "  Input: \"#{input_text}\"\n\n"

  tokens = tokenizer.tokenize(input_text)
  result_tokens = filt.apply(tokens)

  puts "  Output tokens:"
  result_tokens.each_with_index do |tok, i|
    printf "    [%d] %-20s  pos=%d  pos_inc=%d  pos_len=%d  boost=%.2f\n",
           i, tok.text.inspect, tok.position, tok.position_increment, tok.position_length, tok.boost
  end

  puts
  puts "  Explanation:"
  puts "    - All tokens have boost=1.0 (default)"
  puts "    - Synonyms have equal weight to original tokens\n\n"

  # ── Step 3: Apply filter WITH boost=0.8 ───────────────────────────────
  puts "Step 3: Applying filter WITH boost=0.8"
  filt_boosted = Laurus::SynonymGraphFilter.new(syn_dict, keep_original: true, boost: 0.8)

  puts "  Input: \"#{input_text}\"\n\n"

  tokens = tokenizer.tokenize(input_text)
  result_tokens = filt_boosted.apply(tokens)

  puts "  Output tokens:"
  result_tokens.each_with_index do |tok, i|
    printf "    [%d] %-20s  pos=%d  pos_inc=%d  pos_len=%d  boost=%.2f\n",
           i, tok.text.inspect, tok.position, tok.position_increment, tok.position_length, tok.boost
  end

  puts
  puts "  Explanation:"
  puts "    - Original token 'ml' has boost=1.0"
  puts "    - Synonym tokens have boost < 1.0"
  puts "    - Lower boost means synonyms contribute less to the final score"
  puts "    - This helps prioritize exact matches over synonym matches\n\n"

  # ── Step 4: Multi-word synonym expansion ──────────────────────────────
  puts "Step 4: Multi-word synonym expansion (ai -> artificial intelligence)"
  filt2 = Laurus::SynonymGraphFilter.new(syn_dict, keep_original: true, boost: 0.9)

  input_text2 = "ai research"
  puts "  Input: \"#{input_text2}\"\n\n"

  tokens2 = tokenizer.tokenize(input_text2)
  result_tokens2 = filt2.apply(tokens2)

  puts "  Output tokens (position graph):"
  result_tokens2.each_with_index do |tok, i|
    printf "    [%d] %-25s  pos=%d  pos_inc=%d  pos_len=%d  boost=%.2f\n",
           i, tok.text.inspect, tok.position, tok.position_increment, tok.position_length, tok.boost
  end

  puts
  puts "  Explanation:"
  puts "    - 'artificial intelligence' spans positions 0 and 1"
  puts "    - position_length > 1 on 'artificial' indicates a multi-word synonym"
  puts "    - Downstream phrase indexing uses this graph to support phrase queries\n\n"

  puts "Use cases for boost:"
  puts "  - boost=0.8: Synonyms have 80% weight (common default)"
  puts "  - boost=0.5: Synonyms have 50% weight (conservative)"
  puts "  - boost=1.0: Synonyms equal to originals (no adjustment)\n\n"

  puts "SynonymGraphFilter example completed!"
end

main
