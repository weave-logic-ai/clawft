#!/usr/bin/env bash
# MetaHarness readiness probes for WeftOS (ADR-096). Graceful if CLI missing.
# Captures BOTH:
#   - ADR-041 scorecard (`metaharness score`) — shallow inventory canary
#   - ADR-041 genome (`metaharness genome`) — 7-section readiness (often the
#     missing signal: score alone mis-ranks monorepos as "MCP server")
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT_DIR="${METAHARNESS_OUT:-$ROOT/.metaharness}"
mkdir -p "$OUT_DIR"
JSON_OUT="$OUT_DIR/score-latest.json"
GENOME_OUT="$OUT_DIR/genome-latest.json"

run_score() {
  if command -v npx >/dev/null 2>&1; then
    if npx --yes metaharness score . --json >"$JSON_OUT" 2>/tmp/metaharness-score.err; then
      return 0
    fi
  fi
  cat >"$JSON_OUT" <<'JSON'
{"success":false,"degraded":true,"reason":"metaharness-cli-unavailable","hint":"npm i && npx metaharness score . --json (or use MCP metaharness_score)"}
JSON
  return 1
}

run_genome() {
  if command -v npx >/dev/null 2>&1; then
    if npx --yes metaharness genome . --json >"$GENOME_OUT" 2>/tmp/metaharness-genome.err; then
      return 0
    fi
  fi
  cat >"$GENOME_OUT" <<'JSON'
{"success":false,"degraded":true,"reason":"metaharness-genome-unavailable"}
JSON
  return 1
}

score_ok=0
genome_ok=0
run_score && score_ok=1 || true
run_genome && genome_ok=1 || true

echo "=== ADR-041 scorecard → $JSON_OUT ==="
cat "$JSON_OUT"
echo ""
echo "=== ADR-041 genome → $GENOME_OUT ==="
cat "$GENOME_OUT"

if [[ "$score_ok" -eq 0 && "$genome_ok" -eq 0 ]]; then
  echo "metaharness score+genome degraded" >&2
fi
exit 0
