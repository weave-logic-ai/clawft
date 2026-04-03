#!/usr/bin/env bash
# Generate MDX API pages for all publishable crates.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building rustdoc-mdx tool..."
cargo build --release --manifest-path "$ROOT/tools/rustdoc-mdx/Cargo.toml"

TOOL="$ROOT/tools/rustdoc-mdx/target/release/rustdoc-mdx"
OUTPUT="$ROOT/docs/src/content/docs/api"
mkdir -p "$OUTPUT"

for crate_dir in "$ROOT"/crates/clawft-*/; do
    crate_name=$(basename "$crate_dir")
    echo "Processing $crate_name..."
    "$TOOL" --crate-dir "$crate_dir" --output "$OUTPUT"
done

# Also process the top-level weftos and exo-resource-tree crates.
for extra in weftos exo-resource-tree; do
    crate_dir="$ROOT/crates/$extra"
    if [[ -d "$crate_dir/src" ]]; then
        echo "Processing $extra..."
        "$TOOL" --crate-dir "$crate_dir" --output "$OUTPUT"
    fi
done

# Regenerate index page.
echo "Generating index..."
"$TOOL" --output "$OUTPUT" --index-only

echo "Generated MDX API docs at $OUTPUT"
