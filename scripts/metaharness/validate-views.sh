#!/usr/bin/env bash
# Validate Graph ViewSpec fixtures + flywheel anchors (WEFT-727). Evaluate-only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
ERR=0

echo "== ViewSpec fixtures =="
shopt -s nullglob
views=(config/views/*.yaml config/views/*.yml .metaharness/weftos/views/*.yaml)
if [[ ${#views[@]} -eq 0 ]]; then
  echo "FAIL: no ViewSpec yaml under config/views/"
  ERR=1
else
  for f in "${views[@]}"; do
    [[ -f "$f" ]] || continue
    echo "check $f"
    python3 - "$f" <<'PY' || ERR=1
import sys, re
path = sys.argv[1]
text = open(path).read()
# Minimal structural checks (YAML may be comment-heavy; avoid full PyYAML dep)
required = ["view_id:", "sources:", "caps:", "promote:"]
missing = [r for r in required if r not in text]
if missing:
    print(f"FAIL {path}: missing {missing}")
    sys.exit(1)
if "schema_version:" not in text:
    print(f"FAIL {path}: missing schema_version")
    sys.exit(1)
print(f"OK {path}")
PY
  done
fi

echo "== Flywheel anchors =="
MAN="$ROOT/.metaharness/eval/flywheel-anchor.manifest.json"
if [[ ! -f "$MAN" ]]; then
  echo "FAIL: missing $MAN"
  ERR=1
else
  python3 - "$MAN" "$ROOT" <<'PY' || ERR=1
import json, sys, os
man_path, root = sys.argv[1], sys.argv[2]
man = json.load(open(man_path))
assert man.get("mode") == "evaluate_only", "manifest mode must be evaluate_only"
for a in man.get("anchors", []):
    p = os.path.join(root, ".metaharness/eval", a["path"])
    if not os.path.isfile(p):
        print(f"FAIL missing anchor {p}")
        sys.exit(1)
    json.load(open(p))
    print(f"OK anchor {a['id']}")
print("OK manifest", man_path)
PY
fi

echo "== Room-12 identity invariants =="
python3 - <<'PY' || ERR=1
from pathlib import Path
p = Path("config/views/room-12-identity.yaml")
t = p.read_text()
checks = [
    ("view_id: room-12-identity" in t, "view_id"),
    ("kind: bvh" in t, "bvh source"),
    ("live: true" in t, "live source"),
    ("require_chain_event: true" in t, "chain promote"),
    ("min_score: 0.82" in t or "min_score:" in t, "soft edge score"),
]
for ok, name in checks:
    print(("OK" if ok else "FAIL"), name)
    if not ok:
        raise SystemExit(1)
PY

if [[ $ERR -ne 0 ]]; then
  echo "validate-views: FAILED"
  exit 1
fi
echo "validate-views: OK (evaluate-only; no promote)"
