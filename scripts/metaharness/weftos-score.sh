#!/usr/bin/env bash
# Extended WeftOS MetaHarness score (complements upstream ADR-041 scorecard).
# Upstream inventory is shallow (HIGH_SIGNAL only); this scores *our* harness
# assets AND real OS surface (workspace crates, cargo-dist targets, edge tree).
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

# Platform surface: real OS breadth (invisible to ADR-041 inventory).
PLATFORM_JSON=$(python3 - <<'PY'
import json, re
from pathlib import Path

root = Path(".")
try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore

with open(root / "Cargo.toml", "rb") as f:
    cargo = tomllib.load(f)
ws = cargo.get("workspace", {})
members = list(ws.get("members", []))
exclude = list(ws.get("exclude", []))
dist_targets = list(ws.get("metadata", {}).get("dist", {}).get("targets", []))

on_disk = sorted(
    p.name
    for p in (root / "crates").iterdir()
    if p.is_dir() and (p / "Cargo.toml").exists()
)
member_names = {Path(m).name for m in members}
edge_tree = sorted(set(on_disk) - member_names)

domains = {
    "edge_wasm": r"wasm|wasi|browser|android-edge|edge-pad|edge-bench",
    "firmware_hw": r"edge-pad-idf|leaf-touch|leaf-display|lgfx|gt911|esp|leaf-",
    "world_sensor": r"worldmodel|sensor|bvh|splat|sonobuoy",
    "voice": r"voice|whisper|aec|tts|onnx",
    "kernel_mesh": r"kernel|weave|mesh|rpc|security|substrate|exo-",
    "agent_llm": r"llm|channels|tools|delegation|service-agent|plugin|service-",
    "memory_graph": r"canon|cow-memory|graphify|treecalc",
    "gui": r"gui|surface|clawft-app|leaf-canvas|leaf-renderer|leaf-scene|scene-builder",
}
domain_counts = {}
for name, pat in domains.items():
    hits = [m for m in members if re.search(pat, m, re.I)]
    # also count edge tree for firmware
    if name == "firmware_hw":
        hits += [c for c in edge_tree if re.search(pat, c, re.I)]
    domain_counts[name] = len(set(hits))

domains_hit = sum(1 for v in domain_counts.values() if v > 0)

# Score components (0-100 axes for surface)
# crateSurface: workspace members (full credit at 40+)
n_members = len(members)
crate_pts = min(12, (n_members * 12) // 40) if n_members else 0
# edge firmware / non-workspace crates (full credit at 8+)
edge_pts = min(4, (len(edge_tree) * 4) // 8) if edge_tree else 0
# dist targets (full credit at 5+)
target_pts = min(6, (len(dist_targets) * 6) // 5) if dist_targets else 0
# domain breadth (full credit at 6 distinct domains)
domain_pts = min(6, domains_hit) if domains_hit else 0
# extra signals
if (root / "Dockerfile").exists():
    target_pts = min(6, target_pts + 1)
if (root / "rust-toolchain.toml").exists():
    crate_pts = min(12, crate_pts)  # already capped; toolchain noted in json

surface_pts = crate_pts + edge_pts + target_pts + domain_pts  # max 28

print(json.dumps({
    "workspaceMembers": n_members,
    "workspaceExclude": len(exclude),
    "onDiskCrates": len(on_disk),
    "edgeFirmwareCrates": edge_tree,
    "edgeFirmwareCount": len(edge_tree),
    "distTargets": dist_targets,
    "distTargetCount": len(dist_targets),
    "domains": domain_counts,
    "domainsHit": domains_hit,
    "scores": {
        "crateSurface": crate_pts,
        "edgeFirmware": edge_pts,
        "targetSurface": target_pts,
        "domainBreadth": domain_pts,
        "platformSurfaceTotal": surface_pts,
    },
}))
PY
)

# Harness asset points (rebalanced so platform surface can matter)
# Max: tasks20 + views12 + anchors10 + patterns12 + adrs8 + cmds8 + surfYaml4 + platform28 = 102 → clamp 100
score_task=$(( tasks >= 3 ? 20 : tasks * 6 ))
score_view=$(( views >= 1 ? 12 : 0 ))
score_anchor=$(( anchors >= 2 ? 10 : anchors * 5 ))
score_pat=$(( patterns >= 5 ? 12 : patterns * 2 ))
score_adr=$(( adrs * 4 ))
score_cmd=$(( cmds >= 3 ? 8 : cmds * 2 ))
score_surf=$(( surfaces * 4 ))

score_crate=$(python3 -c "import json;print(json.loads('''$PLATFORM_JSON''')['scores']['crateSurface'])")
score_edge=$(python3 -c "import json;print(json.loads('''$PLATFORM_JSON''')['scores']['edgeFirmware'])")
score_tgt=$(python3 -c "import json;print(json.loads('''$PLATFORM_JSON''')['scores']['targetSurface'])")
score_dom=$(python3 -c "import json;print(json.loads('''$PLATFORM_JSON''')['scores']['domainBreadth'])")
score_plat=$(python3 -c "import json;print(json.loads('''$PLATFORM_JSON''')['scores']['platformSurfaceTotal'])")

total=$(( score_task + score_view + score_anchor + score_pat + score_adr + score_cmd + score_surf + score_plat ))
[[ $total -gt 100 ]] && total=100

up_fit=""
up_cov=""
up_mem=""
up_compile=""
up_safety=""
if [[ -f "$ROOT/.metaharness/score-latest.json" ]]; then
  up_fit=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('harnessFit',''))" 2>/dev/null || true)
  up_cov=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('taskCoverage',''))" 2>/dev/null || true)
  up_mem=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('memoryUsefulness',''))" 2>/dev/null || true)
  up_compile=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('compileConfidence',''))" 2>/dev/null || true)
  up_safety=$(python3 -c "import json;print(json.load(open('$ROOT/.metaharness/score-latest.json')).get('toolSafety',''))" 2>/dev/null || true)
fi

# Genome (ADR-041 7-section readiness) — the signal scorecard alone misses
genome_json="{}"
if [[ -f "$ROOT/.metaharness/genome-latest.json" ]]; then
  genome_json=$(cat "$ROOT/.metaharness/genome-latest.json")
fi

python3 - "$OUT" "$total" "$tasks" "$views" "$anchors" "$patterns" "$adrs" "$cmds" "$surfaces" \
  "$score_task" "$score_view" "$score_anchor" "$score_pat" "$score_adr" "$score_cmd" "$score_surf" \
  "$score_crate" "$score_edge" "$score_tgt" "$score_dom" "$score_plat" \
  "$up_fit" "$up_cov" "$up_mem" "$up_compile" "$up_safety" \
  "$PLATFORM_JSON" "$genome_json" <<'PY'
import json, datetime, sys
(
    out, total, tasks, views, anchors, patterns, adrs, cmds, surfaces,
    st, sv, sa, sp, sadr, sc, ss,
    scrate, sedge, stgt, sdom, splat,
    up_fit, up_cov, up_mem, up_compile, up_safety,
    platform_raw, genome_raw,
) = sys.argv[1:29]

def ni(x):
    try:
        return int(x)
    except Exception:
        return 0

def maybe(x):
    return int(x) if x not in ("", "None", None) else None

platform = json.loads(platform_raw)
try:
    genome = json.loads(genome_raw) if genome_raw and genome_raw != "{}" else None
except Exception:
    genome = None

doc = {
  "schema": 2,
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
    "workspaceMembers": platform.get("workspaceMembers", 0),
    "onDiskCrates": platform.get("onDiskCrates", 0),
    "edgeFirmwareCount": platform.get("edgeFirmwareCount", 0),
    "distTargetCount": platform.get("distTargetCount", 0),
    "domainsHit": platform.get("domainsHit", 0),
  },
  "componentScores": {
    "tasks": ni(st),
    "views": ni(sv),
    "anchors": ni(sa),
    "patterns": ni(sp),
    "adrs": ni(sadr),
    "commands": ni(sc),
    "surfacesYaml": ni(ss),
    "crateSurface": ni(scrate),
    "edgeFirmware": ni(sedge),
    "targetSurface": ni(stgt),
    "domainBreadth": ni(sdom),
    "platformSurface": ni(splat),
  },
  "platformSurface": platform,
  "upstreamAdr041": {
    "harnessFit": maybe(up_fit),
    "compileConfidence": maybe(up_compile),
    "taskCoverage": maybe(up_cov),
    "toolSafety": maybe(up_safety),
    "memoryUsefulness": maybe(up_mem),
    "note": (
      "ADR-041 scorecard uses shallow HIGH_SIGNAL inventory + recommended "
      "archetype surface — not workspace member count. Prefer metaharness "
      "genome (publish/test readiness) + weftosFoundationScore for OS maturity."
    ),
  },
  "upstreamGenome": genome,
  "targets": {
    "weftosFoundationScore": 80,
    "tasksMin": 3,
    "patternsMin": 5,
    "viewsMin": 1,
    "workspaceMembersMin": 20,
    "distTargetsMin": 5,
    "domainsHitMin": 4,
  },
  "plane": ["WEFT-724", "WEFT-725", "WEFT-726", "WEFT-727", "WEFT-728", "WEFT-730"],
  "missingFromUpstreamScorecard": [
    "workspace Cargo.toml members (crates) — only crates/:dir stub",
    "edge/firmware crates outside workspace",
    "cargo-dist multi-OS targets",
    "metaharness genome 7-section readiness (use genome, not score alone)",
    "minted .harness/ bundle + manifest witness (optional Darwin surface)",
  ],
}
open(out, "w").write(json.dumps(doc, indent=2) + "\n")
print(json.dumps(doc, indent=2))
PY
