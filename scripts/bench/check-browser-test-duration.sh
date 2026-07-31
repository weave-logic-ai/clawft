#!/usr/bin/env bash
# Browser WASM test-duration regression gate (WEFT-408 / P6.7).
#
# Compares a measured suite wall-clock duration against a committed (or
# prior-run) baseline and fails when the result is more than THRESHOLD%
# slower. Lower is better — speedups always pass.
#
# Usage:
#   check-browser-test-duration.sh <results.json> [baseline.json] [threshold_pct]
#   check-browser-test-duration.sh --from-sec <seconds> [--out results.json]
#                                  [--baseline baseline.json] [--threshold N]
#
# Environment:
#   BROWSER_DURATION_SOFT=1           warn only (exit 0) on regression
#   BROWSER_DURATION_UPDATE_BASELINE=1  rewrite baseline from results (exit 0)
#   BROWSER_DURATION_BASELINE=path    override baseline path
#   BROWSER_DURATION_PRIOR=path       prior-run artefact (CI download)
#
# Exit codes:
#   0 — within threshold (or soft/update mode)
#   1 — duration regression beyond threshold
#   2 — usage / input error

set -euo pipefail

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
CYAN=$'\033[0;36m'
NC=$'\033[0m'

info() { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()   { printf "${GREEN}[PASS]${NC}  %s\n" "$*"; }
warn() { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
fail() { printf "${RED}[FAIL]${NC}  %s\n" "$*" >&2; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEFAULT_BASELINE="$SCRIPT_DIR/browser-test-duration-baseline.json"
DEFAULT_OUT="$WORKSPACE_ROOT/browser-test-duration.json"

FROM_SEC=""
RESULTS=""
BASELINE="${BROWSER_DURATION_BASELINE:-}"
THRESHOLD=""
OUT=""
WRITE_ONLY=false
POSITIONAL=()

usage() {
    cat <<'EOF'
Usage:
  check-browser-test-duration.sh <results.json> [baseline.json] [threshold_pct]
  check-browser-test-duration.sh --from-sec <seconds> [--out results.json]
                                 [--baseline baseline.json] [--threshold N]
                                 [--write-only]

  --write-only   Record duration artefact only; skip the regression gate.
                 Used by scripts/build.sh test-browser after the suite runs.

Results JSON schema (weftos.browser-test-duration.v1):
  {
    "schema": "weftos.browser-test-duration.v1",
    "suite": "browser_pipeline",
    "duration_sec": 123.4,
    "duration_ms": 123400,
    "timestamp": "2026-07-31T00:00:00Z",
    "features": "browser",
    "git_sha": "…",
    "pass": true
  }
EOF
    exit 2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --from-sec)   FROM_SEC="${2:?--from-sec requires a value}"; shift 2 ;;
        --out)        OUT="${2:?--out requires a path}"; shift 2 ;;
        --baseline)   BASELINE="${2:?--baseline requires a path}"; shift 2 ;;
        --threshold)  THRESHOLD="${2:?--threshold requires a value}"; shift 2 ;;
        --write-only) WRITE_ONLY=true; shift ;;
        -h|--help)    usage ;;
        --*)          fail "unknown flag: $1"; usage ;;
        *)            POSITIONAL+=("$1"); shift ;;
    esac
done

if [ ${#POSITIONAL[@]} -ge 1 ]; then
    RESULTS="${POSITIONAL[0]}"
fi
if [ ${#POSITIONAL[@]} -ge 2 ] && [ -z "$BASELINE" ]; then
    BASELINE="${POSITIONAL[1]}"
fi
if [ ${#POSITIONAL[@]} -ge 3 ] && [ -z "$THRESHOLD" ]; then
    THRESHOLD="${POSITIONAL[2]}"
fi

BASELINE="${BASELINE:-$DEFAULT_BASELINE}"

json_get() {
    local file="$1" key="$2"
    if command -v jq >/dev/null 2>&1; then
        jq -r --arg k "$key" '.[$k] // empty' "$file"
    elif command -v python3 >/dev/null 2>&1; then
        python3 -c "
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)
v = d.get(sys.argv[2])
if v is not None:
    print(v)
" "$file" "$key"
    else
        fail "jq or python3 is required"
        exit 2
    fi
}

write_results() {
    local duration_sec="$1"
    local out="${2:-$DEFAULT_OUT}"
    local duration_ms
    duration_ms=$(python3 -c "print(int(float('$duration_sec') * 1000))" 2>/dev/null \
        || echo $(( ${duration_sec%.*} * 1000 )))
    local ts
    ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    local sha="${GITHUB_SHA:-$(git -C "$WORKSPACE_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)}"
    local features="${BROWSER_TEST_FEATURES:-browser}"
    local suite="${BROWSER_TEST_SUITE:-browser_pipeline}"
    local pass_flag="${BROWSER_TEST_PASS:-true}"
    local runner="${RUNNER_OS:-$(uname -s 2>/dev/null || echo unknown)}"

    python3 -c "
import json
data = {
    'schema': 'weftos.browser-test-duration.v1',
    'suite': '''$suite''',
    'duration_sec': float('''$duration_sec'''),
    'duration_ms': int('''$duration_ms'''),
    'timestamp': '''$ts''',
    'features': '''$features''',
    'git_sha': '''$sha''',
    'runner_os': '''$runner''',
    'pass': ('''$pass_flag'''.lower() in ('1','true','yes')),
}
with open('''$out''', 'w') as f:
    json.dump(data, f, indent=2)
    f.write('\n')
"
    echo "$out"
}

if [ -n "$FROM_SEC" ]; then
    OUT="${OUT:-$DEFAULT_OUT}"
    RESULTS="$(write_results "$FROM_SEC" "$OUT")"
    info "Wrote duration results → $RESULTS"
    if [ "$WRITE_ONLY" = true ]; then
        ok "write-only mode — skipping regression comparison"
        exit 0
    fi
fi

if [ -z "$RESULTS" ]; then
    fail "provide <results.json> or --from-sec <seconds>"
    usage
fi

if [ ! -f "$RESULTS" ]; then
    fail "results file not found: $RESULTS"
    exit 2
fi

R_SEC=$(json_get "$RESULTS" "duration_sec")
if [ -z "$R_SEC" ]; then
    R_MS=$(json_get "$RESULTS" "duration_ms")
    if [ -z "$R_MS" ]; then
        fail "results missing duration_sec / duration_ms: $RESULTS"
        exit 2
    fi
    R_SEC=$(python3 -c "print(float('$R_MS') / 1000.0)")
fi

if [ ! -f "$BASELINE" ]; then
    warn "baseline not found: $BASELINE"
    warn "Seeding a local baseline from current results (no gate applied)."
    mkdir -p "$(dirname "$BASELINE")"
    cp "$RESULTS" "$BASELINE"
    ok "Wrote seed baseline → $BASELINE"
    exit 0
fi

B_SEC=$(json_get "$BASELINE" "duration_sec")
if [ -z "$B_SEC" ]; then
    B_MS=$(json_get "$BASELINE" "duration_ms")
    if [ -z "$B_MS" ]; then
        fail "baseline missing duration_sec / duration_ms: $BASELINE"
        exit 2
    fi
    B_SEC=$(python3 -c "print(float('$B_MS') / 1000.0)")
fi

if [ -z "$THRESHOLD" ]; then
    THRESHOLD=$(json_get "$BASELINE" "threshold_pct")
fi
THRESHOLD="${THRESHOLD:-10}"

# Prefer a prior-run artefact when present (true "compare against prior
# runs"); fall back to the committed baseline otherwise. A committed
# absolute ceiling can still be enforced separately via
# BROWSER_DURATION_CEILING_SEC.
REF_SOURCE="committed baseline"
if [ -n "${BROWSER_DURATION_PRIOR:-}" ] && [ -f "$BROWSER_DURATION_PRIOR" ]; then
    P_SEC=$(json_get "$BROWSER_DURATION_PRIOR" "duration_sec")
    if [ -n "$P_SEC" ]; then
        info "Prior-run artefact: ${P_SEC}s (prefer over committed ${B_SEC}s)"
        B_SEC="$P_SEC"
        REF_SOURCE="prior-run artefact"
    fi
fi
info "Reference source: $REF_SOURCE (${B_SEC}s)"

if [ -n "${BROWSER_DURATION_CEILING_SEC:-}" ]; then
    CEIL="${BROWSER_DURATION_CEILING_SEC}"
    OVER_CEIL=$(python3 -c "print(1 if float('$R_SEC') > float('$CEIL') else 0)")
    if [ "$OVER_CEIL" = "1" ]; then
        msg="duration ${R_SEC}s exceeds absolute ceiling ${CEIL}s"
        if [ "${BROWSER_DURATION_SOFT:-0}" = "1" ]; then
            warn "$msg (BROWSER_DURATION_SOFT=1 — not failing)"
        else
            fail "$msg"
            exit 1
        fi
    else
        ok "under absolute ceiling ${CEIL}s"
    fi
fi

if [ "${BROWSER_DURATION_UPDATE_BASELINE:-0}" = "1" ]; then
    info "BROWSER_DURATION_UPDATE_BASELINE=1 — rewriting baseline from results"
    python3 -c "
import json
with open('$RESULTS') as f:
    r = json.load(f)
with open('$BASELINE') as f:
    b = json.load(f)
b['duration_sec'] = float(r['duration_sec'])
b['duration_ms'] = int(r.get('duration_ms', float(r['duration_sec']) * 1000))
b['timestamp'] = r.get('timestamp', b.get('timestamp'))
b['git_sha'] = r.get('git_sha', b.get('git_sha'))
b['features'] = r.get('features', b.get('features', 'browser'))
b['threshold_pct'] = b.get('threshold_pct', int('$THRESHOLD'))
b['schema'] = 'weftos.browser-test-duration.v1'
with open('$BASELINE', 'w') as f:
    json.dump(b, f, indent=2)
    f.write('\n')
"
    ok "Baseline updated → $BASELINE"
    exit 0
fi

PCT=$(python3 -c "
b=float('$B_SEC'); r=float('$R_SEC')
print('0' if b <= 0 else f'{((r - b) / b) * 100:.4f}')
")
IS_REGRESSION=$(python3 -c "print(1 if float('$PCT') > float('$THRESHOLD') else 0)")

echo "=== Browser Test Duration Gate (WEFT-408 / P6.7) ==="
echo "Results:   $RESULTS"
echo "Baseline:  $BASELINE"
echo "Threshold: ${THRESHOLD}% (fail when slower)"
echo "Measured:  ${R_SEC}s"
echo "Reference: ${B_SEC}s"
echo "Delta:     ${PCT}%"
echo ""

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "## Browser Test Duration (WEFT-408)"
        echo ""
        echo "| Metric | Value |"
        echo "|--------|-------|"
        echo "| Measured | ${R_SEC}s |"
        echo "| Baseline | ${B_SEC}s |"
        echo "| Delta | ${PCT}% |"
        echo "| Threshold | ${THRESHOLD}% |"
        if [ "$IS_REGRESSION" = "1" ]; then
            echo "| Status | FAIL |"
        else
            echo "| Status | PASS |"
        fi
        echo ""
    } >> "$GITHUB_STEP_SUMMARY"
fi

if [ "$IS_REGRESSION" = "1" ]; then
    msg="duration regression: ${R_SEC}s is ${PCT}% slower than ${B_SEC}s (threshold ${THRESHOLD}%)"
    if [ "${BROWSER_DURATION_SOFT:-0}" = "1" ]; then
        warn "$msg (BROWSER_DURATION_SOFT=1 — not failing)"
        exit 0
    fi
    fail "$msg"
    fail ""
    fail "Investigate suite growth / flaky Chrome / cold cache."
    fail "If intentional, re-seed the baseline:"
    fail "  BROWSER_DURATION_UPDATE_BASELINE=1 \\"
    fail "    scripts/bench/check-browser-test-duration.sh $RESULTS"
    exit 1
fi

ok "duration within ${THRESHOLD}% of baseline (${PCT}% delta)"
exit 0
