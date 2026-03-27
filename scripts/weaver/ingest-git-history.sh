#!/usr/bin/env bash
# ingest-git-history.sh — Parse git log into ECC-ready CausalGraph JSON
# Writes output to .weftos/graph/git-history.json
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")/../.." rev-parse --show-toplevel)"
OUT_DIR="$REPO_ROOT/.weftos/graph"
OUT_FILE="$OUT_DIR/git-history.json"

mkdir -p "$OUT_DIR"

# Dump raw git log in a parseable format, then hand off to Python for JSON building.
# Format: COMMIT|<full_hash>|<short_hash>|<author>|<iso_date>|<subject>
# Followed by file names (one per line), then a blank line.

git -C "$REPO_ROOT" log --pretty=format:'COMMIT|%H|%h|%an|%aI|%s' --name-only \
  | python3 "$REPO_ROOT/scripts/weaver/_build_git_graph.py" > "$OUT_FILE"

# Validate
python3 -c "import json, sys; d=json.load(open('$OUT_FILE')); print(f'OK: {d[\"stats\"][\"total_commits\"]} commits, {d[\"stats\"][\"total_nodes\"]} nodes, {d[\"stats\"][\"total_edges\"]} edges')"
