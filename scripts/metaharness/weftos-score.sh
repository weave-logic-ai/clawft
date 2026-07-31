#!/usr/bin/env bash
# Extended WeftOS MetaHarness score (complements upstream ADR-041 scorecard).
# Upstream inventory is shallow (HIGH_SIGNAL only); this scores *our* harness
# assets: tasks, patterns, views, anchors, ADRs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT="${METAHARNESS_OUT:-$ROOT/.metaharness}/weftos-score-latest.json"

tasks=$(find "$ROOT/.metaharness/tasks" -name '*.md' 2>/dev/null | wc -l | tr -d ' ')
views=$(find "$ROOT/config/views" "$ROOT/.metaharness/weftos/views" -name '*.yaml' -o -name '*.yml' 2>/dev/null | wc -l | tr -d ' ')
anchors=$(find "$ROOT/.metaharness/eval" -type f 2>/dev/null | wc -l | tr -d ' ')
patterns=0
[[ -f "$ROOT/.metaharness/patterns-manifest.md" ]] && \
  patterns=$(grep -c '^- `' "$ROOT/.metaharness/patterns-manifest.md" 2>/dev/null || echo 0)
adrs=0
[[ -f "$ROOT/docs/adr/adr-096-metaharness-foundation.md" ]] && adrs=$((adrs+1))
[[ -f "$ROOT/docs/adr/adr-097-metaharness-data-governance.md" ]] && adrs=$((adrs+1))
cmds=$(ls "$ROOT/.claude/commands"/weft-*.md 2>/dev/null | wc -l | tr -d ' ')

# Heuristic 0–100: weight tasks/views/patterns/anchors/adrs/commands
score_task=$(( tasks >= 3 ? 25 : tasks * 8 ))
score_view=$(( views >= 1 ? 20 : 0 ))
score_anchor=$(( anchors >= 2 ? 15 : anchors * 7 ))
score_pat=$(( patterns >= 5 ? 20 : patterns * 4 ))
score_adr=$(( adrs * 5 ))
score_cmd=$(( cmds >= 3 ? 15 : cmds * 5 ))
total=$(( score_task + score_view + score_anchor + score_pat + score_adr + score_cmd ))
[[ $total -gt 100 ]] && total=100

# Upstream score if present
up_fit=""; up_cov=""; up_mem=""
if [[ -f "$ROOT/.metaharness/score-latest.json" ]]; then
  up_fit=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('harnessFit',''))" 2>/dev/null || true)
  up_cov=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('taskCoverage',''))" 2>/dev/null || true)
  up_mem=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('memoryUsefulness',''))" 2>/dev/null || true)
fi

python3 - <<PY
import json, datetime
doc = {
  "schema": 1,
  "kind": "weftos-metaharness-score",
  "generatedAt": datetime.datetime.utcnow().isoformat() + "Z",
  "weftosFoundationScore": $total,
  "components": {
    "tasks": $tasks,
    "views": $views,
    "anchors": $anchors,
    "patternsManifest": $patterns,
    "adrs": $adrs,
    "claudeCommands": $cmds,
  },
  "componentScores": {
    "tasks": $score_task,
    "views": $score_view,
    "anchors": $score_anchor,
    "patterns": $score_pat,
    "adrs": $score_adr,
    "commands": $score_cmd,
  },
  "upstreamAdr041": {
    "harnessFit": "$up_fit" or None,
    "taskCoverage": "$up_cov" or None,
    "memoryUsefulness": "$up_mem" or None,
    "note": "Upstream scorecard uses shallow inventory; weftosFoundationScore tracks real harness assets."
  },
  "targets": {
    "weftosFoundationScore": 80,
    "tasksMin": 3,
    "patternsMin": 5,
    "viewsMin": 1
  },
  "plane": ["WEFT-724", "WEFT-725", "WEFT-726", "WEFT-727", "WEFT-728"]
}
# fix nulls
for k,v in list(doc["upstreamAdr041"].items()):
  if v == "" or v is None:
    doc["upstreamAdr041"][k] = None
open("$OUT","w").write(json.dumps(doc, indent=2) + "\n")
print(json.dumps(doc, indent=2))
PY
