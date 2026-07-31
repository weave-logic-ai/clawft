#!/usr/bin/env bash
# WEFT-407 — Browser performance profiling baseline (load / init / first-msg / memory)
#
# Produces a schema-valid report (weftos.browser-perf.v1) for the browser WASM
# path. Default mode is CI-friendly: Node unit tests for the metric helpers
# plus a stub report when no live browser run is available.
#
# Usage:
#   scripts/bench/browser-perf-baseline.sh              # stub + unit tests
#   scripts/bench/browser-perf-baseline.sh --stub       # force stub (no tests)
#   scripts/bench/browser-perf-baseline.sh --test-only  # node --test helpers only
#   scripts/bench/browser-perf-baseline.sh --check      # validate last JSON report
#   scripts/bench/browser-perf-baseline.sh --json-out PATH
#
# Modes:
#   stub     — always (CI default when Chrome/pkg not exercised live)
#   partial  — reserved for harness scrapes with incomplete samples
#   live     — full load/init/first-msg after manual harness run
#
# Live capture (manual):
#   1. scripts/build.sh browser && scripts/build.sh serve
#   2. Open http://localhost:8080, Initialize, send a message
#   3. In DevTools: copy(JSON.stringify(window.__clawftPerf, null, 2))
#   4. Save to target/wasm-bench/browser-perf.json
#   5. scripts/bench/browser-perf-baseline.sh --check
#
# Outputs:
#   - Human-readable table on stdout
#   - JSON report to target/wasm-bench/browser-perf.json (or --json-out)
#
# Exit codes:
#   0 — schema ok and no failed target checks (skips do not fail)
#   1 — unit tests failed, schema invalid, or target check failed
#   2 — usage / tooling error

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WWW_DIR="$PROJECT_ROOT/crates/clawft-wasm/www"
BASELINE_JSON="$SCRIPT_DIR/browser-baseline.json"
RESULTS_DIR="$PROJECT_ROOT/target/wasm-bench"
DEFAULT_OUT="$RESULTS_DIR/browser-perf.json"

MODE="auto"          # auto | stub | test-only | check
JSON_OUT="$DEFAULT_OUT"
CHECK_PATH=""

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info() { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()   { printf "${GREEN}[PASS]${NC}  %s\n" "$*"; }
warn() { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
fail() { printf "${RED}[FAIL]${NC}  %s\n" "$*" >&2; }

usage() {
    sed -n '2,35p' "$0" | sed 's/^# \?//'
    exit 2
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --stub)       MODE="stub"; shift ;;
        --test-only)  MODE="test-only"; shift ;;
        --check)
            MODE="check"
            shift
            if [[ $# -gt 0 && "$1" != --* ]]; then
                CHECK_PATH="$1"
                shift
            fi
            ;;
        --json-out)
            JSON_OUT="${2:?--json-out requires a path}"
            shift 2
            ;;
        -h|--help) usage ;;
        *)
            fail "unknown argument: $1"
            usage
            ;;
    esac
done

if [[ "$MODE" == "check" && -z "$CHECK_PATH" ]]; then
    CHECK_PATH="$JSON_OUT"
fi

# ---------------------------------------------------------------------------
# Node unit tests (schema + target evaluation — always CI-safe)
# ---------------------------------------------------------------------------
run_unit_tests() {
    if ! command -v node >/dev/null 2>&1; then
        fail "node is required for perf-baseline unit tests"
        return 2
    fi
    info "Running Node unit tests: crates/clawft-wasm/www/perf-baseline.test.mjs"
    (cd "$PROJECT_ROOT" && node --test crates/clawft-wasm/www/perf-baseline.test.mjs)
}

# ---------------------------------------------------------------------------
# Emit stub report via Node (imports the same module as the harness)
# ---------------------------------------------------------------------------
emit_stub_report() {
    mkdir -p "$(dirname "$JSON_OUT")"
    local out_abs
    out_abs="$(cd "$(dirname "$JSON_OUT")" && pwd)/$(basename "$JSON_OUT")"

    WEFT_PERF_OUT="$out_abs" WEFT_PERF_WWW="$WWW_DIR" node --input-type=module <<'EOF'
import { writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { join } from "node:path";

const www = process.env.WEFT_PERF_WWW;
const out = process.env.WEFT_PERF_OUT;
const modUrl = pathToFileURL(join(www, "perf-baseline.js")).href;
const { stubReport, validateReport } = await import(modUrl);

const report = stubReport({
  issue: "WEFT-407",
  baseline_file: "scripts/bench/browser-baseline.json",
  host: process.platform,
  node: process.version,
});

const v = validateReport(report);
if (!v.ok) {
  console.error("stub report failed validation:", v.errors.join("; "));
  process.exit(1);
}

writeFileSync(out, JSON.stringify(report, null, 2) + "\n");
console.log(JSON.stringify(report, null, 2));
EOF
}

# ---------------------------------------------------------------------------
# Validate an existing report file
# ---------------------------------------------------------------------------
check_report() {
    local path="$1"
    if [[ ! -f "$path" ]]; then
        fail "report not found: $path"
        return 1
    fi
    local path_abs
    path_abs="$(cd "$(dirname "$path")" && pwd)/$(basename "$path")"

    WEFT_PERF_REPORT="$path_abs" WEFT_PERF_WWW="$WWW_DIR" node --input-type=module <<'EOF'
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { join } from "node:path";

const www = process.env.WEFT_PERF_WWW;
const reportPath = process.env.WEFT_PERF_REPORT;
const modUrl = pathToFileURL(join(www, "perf-baseline.js")).href;
const { validateReport } = await import(modUrl);

const report = JSON.parse(readFileSync(reportPath, "utf8"));
const v = validateReport(report);
if (!v.ok) {
  console.error("INVALID:", v.errors.join("; "));
  process.exit(1);
}

const s = report.summary || {};
console.log("schema:   ", report.schema);
console.log("mode:     ", report.mode);
console.log("summary:  ", JSON.stringify(s));
console.log("");
console.log("Metric               Value        Target   Status");
console.log("-------------------- ------------ -------- ------");
for (const c of report.checks || []) {
  const val = c.value == null ? "n/a" : String(c.value);
  console.log(
    c.name.padEnd(20),
    val.padStart(12),
    String(c.target).padStart(8),
    c.status
  );
}

if (s.ok === false) {
  console.error("\nOne or more target checks FAILED");
  process.exit(1);
}
console.log("\nReport OK");
EOF
}

# ---------------------------------------------------------------------------
# Print human-readable targets table from browser-baseline.json
# ---------------------------------------------------------------------------
print_targets() {
    echo "=== Browser Perf Baseline (WEFT-407) ==="
    echo ""
    if [[ -f "$BASELINE_JSON" ]]; then
        info "Targets from $BASELINE_JSON"
        if command -v python3 >/dev/null 2>&1; then
            python3 - <<'PY' "$BASELINE_JSON"
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)
print(f"  {'Metric':<28} {'Target':>10}  (lower is better)")
print(f"  {'-'*28} {'-'*10}")
for k, v in d.get("targets", {}).items():
    print(f"  {k:<28} {v:>10}")
print()
base = d.get("baseline", {})
print("  Checked-in baseline values:")
for k in ("load_ms", "init_ms", "first_msg_ms", "subsequent_msg_ms_avg", "memory_wasm_heap_mb"):
    print(f"    {k}: {base.get(k)}")
print()
PY
        else
            cat "$BASELINE_JSON"
            echo ""
        fi
    else
        warn "baseline file missing: $BASELINE_JSON"
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
print_targets

case "$MODE" in
    test-only)
        run_unit_tests
        ok "Unit tests passed"
        exit 0
        ;;
    check)
        info "Validating report: $CHECK_PATH"
        check_report "$CHECK_PATH"
        ok "Report valid: $CHECK_PATH"
        exit 0
        ;;
    stub)
        info "Emitting stub report → $JSON_OUT"
        emit_stub_report > /dev/null
        check_report "$JSON_OUT"
        ok "Stub report written: $JSON_OUT"
        exit 0
        ;;
    auto)
        run_unit_tests
        echo ""
        info "Emitting CI stub report → $JSON_OUT"
        info "(Live numbers: run harness, then --check on window.__clawftPerf dump)"
        emit_stub_report > /dev/null
        check_report "$JSON_OUT"
        ok "Browser perf baseline gate passed (stub + unit tests)"
        echo ""
        info "JSON: $JSON_OUT"
        exit 0
        ;;
    *)
        fail "internal: unknown mode $MODE"
        exit 2
        ;;
esac
