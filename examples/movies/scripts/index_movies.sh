#!/usr/bin/env bash
# Index the Meilisearch movies dataset into laurus.
# Uses the REPL with piped stdin to avoid per-document process startup overhead.
# Requires: jq
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
INDEX_DIR="$SCRIPT_DIR/../index"
DATASET="$PROJECT_ROOT/../datasets/datasets/movies/movies.json"

if [ ! -f "$DATASET" ]; then
  echo "Error: Dataset not found at $DATASET" >&2
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "Error: jq is required but not installed." >&2
  exit 1
fi

# Build the release binary first for faster execution.
echo "==> Building laurus (release)..."
cargo build --manifest-path "$PROJECT_ROOT/Cargo.toml" --release --bin laurus
LAURUS="$PROJECT_ROOT/target/release/laurus"

TOTAL=$(jq length "$DATASET")
COMMIT_INTERVAL=1000

echo "==> Indexing $TOTAL movies into $INDEX_DIR"

# Generate REPL commands from the dataset and pipe them into a single laurus process.
generate_commands() {
  local count=0
  jq -c '.[]' "$DATASET" | while IFS= read -r movie; do
    ID=$(echo "$movie" | jq -r '.id')
    DOC=$(echo "$movie" | jq -c '{fields: {title: {Text: .title}, overview: {Text: .overview}, genres: {Text: (.genres | join(", "))}, poster: {Text: .poster}, release_date: {Int64: .release_date}}}')

    echo "doc add $ID $DOC"

    count=$((count + 1))
    if [ $((count % COMMIT_INTERVAL)) -eq 0 ]; then
      echo "commit"
      echo "    Committed $count / $TOTAL documents" >&2
    fi
  done

  # Final commit and exit.
  echo "commit"
  echo "quit"
}

generate_commands | "$LAURUS" --data-dir "$INDEX_DIR" repl >/dev/null

echo "==> Done. Indexed $TOTAL movies."
