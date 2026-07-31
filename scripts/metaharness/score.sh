#!/usr/bin/env bash
# MetaHarness readiness score for WeftOS (ADR-096). Graceful if CLI missing.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT_DIR="${METAHARNESS_OUT:-$ROOT/.metaharness}"
mkdir -p "$OUT_DIR"
JSON_OUT="$OUT_DIR/score-latest.json"

if command -v npx >/dev/null 2>&1; then
  if npx --yes metaharness score . --json >"$JSON_OUT" 2>/tmp/metaharness-score.err; then
    cat "$JSON_OUT"
    exit 0
  fi
fi

# Fallback: print degraded envelope
cat >"$JSON_OUT" <<'JSON'
{"success":false,"degraded":true,"reason":"metaharness-cli-unavailable","hint":"npm i && npx metaharness score . --json (or use MCP metaharness_score)"}
JSON
cat "$JSON_OUT"
echo "metaharness score degraded — see $JSON_OUT" >&2
exit 0
