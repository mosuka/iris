#!/usr/bin/env bash
# Example search queries against the movies index.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
INDEX_DIR="$SCRIPT_DIR/../index"

echo "==> Building laurus (release)..."
cargo build --manifest-path "$PROJECT_ROOT/Cargo.toml" --release --bin laurus
LAURUS="$PROJECT_ROOT/target/release/laurus"

echo "=== Search: 'star wars' ==="
"$LAURUS" --data-dir "$INDEX_DIR" search "star wars" --limit 5
echo

echo "=== Search: title:nemo ==="
"$LAURUS" --data-dir "$INDEX_DIR" search "title:nemo" --limit 5
echo

echo "=== Search: genres:comedy ==="
"$LAURUS" --data-dir "$INDEX_DIR" search "genres:comedy" --limit 5
echo

echo '=== Search: overview:"robot" ==='
"$LAURUS" --data-dir "$INDEX_DIR" search "overview:robot" --limit 5
echo

echo "=== JSON output: 'star wars' ==="
"$LAURUS" --data-dir "$INDEX_DIR" --format json search "star wars" --limit 3
