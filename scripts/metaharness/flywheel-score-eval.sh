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
if comps.get("workspaceMembers", 0) < 20:
    errs.append(f"workspaceMembers {comps.get('workspaceMembers')} < 20")
if comps.get("distTargetCount", 0) < 5:
    errs.append(f"distTargetCount {comps.get('distTargetCount')} < 5")
if comps.get("domainsHit", 0) < 4:
    errs.append(f"domainsHit {comps.get('domainsHit')} < 4")
gate = Path(".metaharness/tasks/gate.md").read_text()
if "scripts/build.sh gate" not in gate:
    errs.append("gate task lost build.sh gate command")
if not Path("docs/adr/adr-090-lewm-ecc-decoupling-invariant.md").exists():
    errs.append("ADR-090 missing")
# Genome (when present): expect publish-ready OS, not "empty scaffold"
genome = doc.get("upstreamGenome") or {}
if genome and not genome.get("degraded"):
    if genome.get("publish_readiness", 1) < 0.8:
        errs.append(f"genome publish_readiness {genome.get('publish_readiness')} < 0.8")
    if genome.get("test_confidence", 1) < 0.8:
        errs.append(f"genome test_confidence {genome.get('test_confidence')} < 0.8")
plat = doc.get("componentScores", {}).get("platformSurface", 0)
print(json.dumps({
    "weftosFoundationScore": score,
    "platformSurfacePts": plat,
    "components": comps,
    "genome": {k: genome.get(k) for k in ("repo_type", "publish_readiness", "test_confidence", "risk_score", "mcp_surface") if genome},
    "ok": not errs,
    "errors": errs,
}, indent=2))
if errs:
    raise SystemExit(1)
print("flywheel-score-eval: OK (evaluate_only)")
PY

echo "== validate views =="
scripts/metaharness/validate-views.sh

echo "== anchor file present =="
test -f .metaharness/flywheel-score/anchors/score-invariants.json

echo "== flywheel measure receipt =="
# Signed measure via @metaharness/flywheel (no promote / no Darwin)
node "$ROOT/scripts/metaharness/flywheel-measure.mjs" measure

echo "flywheel-score-eval: complete"
