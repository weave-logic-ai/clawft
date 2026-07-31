#!/usr/bin/env bash
# Extended WeftOS MetaHarness score (complements upstream ADR-041 scorecard).
# Upstream inventory is shallow (HIGH_SIGNAL only); this scores *our* harness
# assets: tasks, patterns, views, anchors, ADRs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT_DIR="${METAHARNESS_OUT:-$ROOT/.metaharness}"
mkdir -p "$OUT_DIR"
OUT="$OUT_DIR/weftos-score-latest.json"

count_files() {
  local n
  n=$(find "$@" -type f 2>/dev/null | wc -l | tr -d ' ')
  echo "${n:-0}"
}

tasks=$(count_files "$ROOT/.metaharness/tasks" -name '*.md')
views=$(find "$ROOT/config/views" "$ROOT/.metaharness/weftos/views" \( -name '*.yaml' -o -name '*.yml' \) -type f 2>/dev/null | wc -l | tr -d ' ')
views=${views:-0}
anchors=$(count_files "$ROOT/.metaharness/eval")
patterns=0
if [[ -f "$ROOT/.metaharness/patterns-manifest.md" ]]; then
  patterns=$(grep -E '^\| `pattern-' "$ROOT/.metaharness/patterns-manifest.md" 2>/dev/null | wc -l | tr -d ' ')
  patterns=${patterns:-0}
fi
adrs=0
[[ -f "$ROOT/docs/adr/adr-096-metaharness-foundation.md" ]] && adrs=$((adrs + 1))
[[ -f "$ROOT/docs/adr/adr-097-metaharness-data-governance.md" ]] && adrs=$((adrs + 1))
cmds=$(find "$ROOT/.metaharness/commands" -name 'weft-*.md' -type f 2>/dev/null | wc -l | tr -d ' ')
cmds=${cmds:-0}
surfaces=0
[[ -f "$ROOT/.metaharness/weftos/surfaces.yaml" ]] && surfaces=1

score_task=$(( tasks >= 3 ? 25 : tasks * 8 ))
score_view=$(( views >= 1 ? 20 : 0 ))
score_anchor=$(( anchors >= 2 ? 15 : anchors * 7 ))
score_pat=$(( patterns >= 5 ? 20 : patterns * 4 ))
score_adr=$(( adrs * 5 ))
score_cmd=$(( cmds >= 3 ? 10 : cmds * 3 ))
score_surf=$(( surfaces * 5 ))
total=$(( score_task + score_view + score_anchor + score_pat + score_adr + score_cmd + score_surf ))
[[ $total -gt 100 ]] && total=100

up_fit=""
up_cov=""
up_mem=""
if [[ -f "$ROOT/.metaharness/score-latest.json" ]]; then
  up_fit=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('harnessFit',''))" 2>/dev/null || true)
  up_cov=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('taskCoverage',''))" 2>/dev/null || true)
  up_mem=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('memoryUsefulness',''))" 2>/dev/null || true)
fi

python3 - "$OUT" "$total" "$tasks" "$views" "$anchors" "$patterns" "$adrs" "$cmds" "$surfaces" \
  "$score_task" "$score_view" "$score_anchor" "$score_pat" "$score_adr" "$score_cmd" "$score_surf" \
  "$up_fit" "$up_cov" "$up_mem" <<'PY'
import json, datetime, sys
out, total, tasks, views, anchors, patterns, adrs, cmds, surfaces = sys.argv[1:10]
st, sv, sa, sp, sadr, sc, ss = sys.argv[10:17]
up_fit, up_cov, up_mem = sys.argv[17:20]

def ni(x):
    try:
        return int(x)
    except Exception:
        return 0

def maybe(x):
    return int(x) if x not in ("", "None", None) else None

doc = {
  "schema": 1,
  "kind": "weftos-metaharness-score",
  "generatedAt": datetime.datetime.utcnow().isoformat() + "Z",
  "weftosFoundationScore": ni(total),
  "components": {
    "tasks": ni(tasks),
    "views": ni(views),
    "anchors": ni(anchors),
    "patternsManifest": ni(patterns),
    "adrs": ni(adrs),
    "commands": ni(cmds),
    "surfacesYaml": ni(surfaces),
  },
  "componentScores": {
    "tasks": ni(st),
    "views": ni(sv),
    "anchors": ni(sa),
    "patterns": ni(sp),
    "adrs": ni(sadr),
    "commands": ni(sc),
    "surfaces": ni(ss),
  },
  "upstreamAdr041": {
    "harnessFit": maybe(up_fit),
    "taskCoverage": maybe(up_cov),
    "memoryUsefulness": maybe(up_mem),
    "note": "Upstream scorecard uses shallow HIGH_SIGNAL inventory; weftosFoundationScore tracks real harness assets + AgentDB patterns via seed script.",
  },
  "targets": {
    "weftosFoundationScore": 80,
    "tasksMin": 3,
    "patternsMin": 5,
    "viewsMin": 1,
  },
  "plane": ["WEFT-724", "WEFT-725", "WEFT-726", "WEFT-727", "WEFT-728"],
}
open(out, "w").write(json.dumps(doc, indent=2) + "\n")
print(json.dumps(doc, indent=2))
PY
