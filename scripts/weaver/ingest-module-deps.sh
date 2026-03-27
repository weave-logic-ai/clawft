#!/usr/bin/env bash
# ingest-module-deps.sh — Parse Rust crate/module dependency structure into
# ECC-ready JSON for the WeftOS causal graph.
# Writes output to .weftos/graph/module-deps.json
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")/../.." rev-parse --show-toplevel)"
OUT_DIR="$REPO_ROOT/.weftos/graph"
OUT_FILE="$OUT_DIR/module-deps.json"

mkdir -p "$OUT_DIR"

python3 "$REPO_ROOT/scripts/weaver/_build_module_graph.py" "$REPO_ROOT" > "$OUT_FILE"

# Validate
python3 -c "
import json, sys
d = json.load(open('$OUT_FILE'))
s = d['stats']
print(f\"OK: {s['total_crates']} crates, {s['total_modules']} modules, {s['total_edges']} edges, avg_degree={s['avg_degree']:.1f}, most_connected={s['most_connected']}\")
"
