#!/usr/bin/env bash
# ingest-decisions.sh — Parse SPARC plans + symposium results into ECC-ready graph JSON
# Writes output to .weftos/graph/decisions.json
#
# Sources:
#   docs/weftos/k2-symposium/08-symposium-results-report.md
#   docs/weftos/k3-symposium/07-symposium-results-report.md
#   docs/weftos/k5-symposium/05-symposium-results.md
#   docs/weftos/ecc-symposium/05-symposium-results-report.md
#   .planning/sparc/weftos/0.1/*.md
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")/../.." rev-parse --show-toplevel)"
SCRIPT_DIR="$REPO_ROOT/scripts/weaver"
OUT_DIR="$REPO_ROOT/.weftos/graph"
OUT_FILE="$OUT_DIR/decisions.json"

mkdir -p "$OUT_DIR"

echo "==> Extracting decisions and phases from symposium results + SPARC plans..."

# Step 1: Generate structured data from the Python extraction module
# Step 2: Pipe through the graph builder
python3 "$SCRIPT_DIR/_decisions_data.py" \
  | python3 "$SCRIPT_DIR/_build_decisions_graph.py" \
  > "$OUT_FILE"

# Step 3: Validate JSON and print summary
python3 -c "
import json, sys
d = json.load(open('$OUT_FILE'))
s = d['stats']
print(f'OK: {s[\"total_decisions\"]} decisions, {s[\"total_phases\"]} phases, {s[\"total_commitments\"]} commitments')
print(f'    {s[\"total_nodes\"]} nodes, {s[\"total_edges\"]} edges')
print(f'    implemented={s[\"implemented\"]} pending={s[\"pending\"]} deferred={s[\"deferred\"]} blocked={s[\"blocked\"]} partial={s[\"partial\"]}')
print(f'    generated_at={d[\"generated_at\"]}')
"

echo "==> Written to $OUT_FILE"
