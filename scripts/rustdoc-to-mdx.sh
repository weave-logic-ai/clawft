#!/usr/bin/env bash
# Generate MDX API docs from Rust source for a single crate.
#
# Usage: scripts/rustdoc-to-mdx.sh <crate-dir> [output-dir]
#
# If a nightly toolchain with rustdoc JSON output is available, this script
# could be extended to run:
#   cargo +nightly doc --output-format json -p <crate>
# For now it delegates to the rustdoc-mdx tool which parses source directly.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CRATE_DIR="${1:?Usage: rustdoc-to-mdx.sh <crate-dir> [output-dir]}"
OUTPUT_DIR="${2:-$ROOT/docs/src/content/docs/api}"

TOOL="$ROOT/tools/rustdoc-mdx/target/release/rustdoc-mdx"
if [[ ! -x "$TOOL" ]]; then
    echo "Building rustdoc-mdx tool..."
    cargo build --release --manifest-path "$ROOT/tools/rustdoc-mdx/Cargo.toml"
fi

mkdir -p "$OUTPUT_DIR"
"$TOOL" --crate-dir "$CRATE_DIR" --output "$OUTPUT_DIR"
