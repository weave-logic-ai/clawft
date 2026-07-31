#!/usr/bin/env bash
# Flywheel status helper (ADR-096). Prefer MCP metaharness_flywheel when available.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
echo "MetaHarness flywheel status (ADR-096)"
echo "Prefer: claude-flow / ruflo MCP tool metaharness_flywheel operation=status"
echo "Receipts dir (if present): $ROOT/.metaharness/"
if [[ -d "$ROOT/.metaharness" ]]; then
  ls -la "$ROOT/.metaharness" 2>/dev/null | head -30
else
  echo "(no .metaharness directory yet)"
fi
if [[ -f "$HOME/.claude/metaharness/routing-receipts.jsonl" ]]; then
  echo "Routing receipts: $HOME/.claude/metaharness/routing-receipts.jsonl ($(wc -l < "$HOME/.claude/metaharness/routing-receipts.jsonl") lines)"
else
  echo "Routing receipts: none at ~/.claude/metaharness/routing-receipts.jsonl"
fi
