#!/usr/bin/env bash
# Evaluate-only score flywheel check (WEFT-730). Never promotes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "== refresh scores =="
scripts/metaharness/score.sh >/dev/null 2>&1 || true
scripts/metaharness/weftos-score.sh >/dev/null

FOUND="$ROOT/.metaharness/weftos-score-latest.json"
[[ -f "$FOUND" ]] || { echo "missing $FOUND"; exit 1; }

python3 - <<'PY'
import json
from pathlib import Path
doc = json.loads(Path(".metaharness/weftos-score-latest.json").read_text())
score = doc.get("weftosFoundationScore", 0)
comps = doc.get("components", {})
errs = []
if score < 80:
    errs.append(f"weftosFoundationScore {score} < 80")
if comps.get("tasks", 0) < 3:
    errs.append("tasks < 3")
if comps.get("views", 0) < 1:
    errs.append("views < 1")
if comps.get("patternsManifest", 0) < 5:
    errs.append("patterns < 5")
gate = Path(".metaharness/tasks/gate.md").read_text()
if "scripts/build.sh gate" not in gate:
    errs.append("gate task lost build.sh gate command")
if not Path("docs/adr/adr-090-lewm-ecc-decoupling-invariant.md").exists():
    errs.append("ADR-090 missing")
print(json.dumps({"weftosFoundationScore": score, "components": comps, "ok": not errs, "errors": errs}, indent=2))
if errs:
    raise SystemExit(1)
print("flywheel-score-eval: OK (evaluate_only)")
PY

echo "== validate views =="
scripts/metaharness/validate-views.sh

echo "== anchor file present =="
test -f .metaharness/flywheel-score/anchors/score-invariants.json
echo "flywheel-score-eval: complete"
