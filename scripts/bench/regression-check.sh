#!/usr/bin/env bash
# Benchmark regression detection
# Compares benchmark results against a known-good baseline and flags regressions.
#
# Usage: regression-check.sh <results.json> [baseline.json] [threshold_pct]
#
# Exit codes:
#   0 = all metrics within threshold (pass)
#   1 = one or more regressions detected (fail)
#   2 = usage / input error
set -euo pipefail

RESULTS="${1:?Usage: regression-check.sh <results.json> [baseline.json] [threshold_pct]}"
BASELINE="${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/baseline.json}"
THRESHOLD="${3:-10}"

# ---------------------------------------------------------------------------
# Validate inputs
# ---------------------------------------------------------------------------
for f in "$RESULTS" "$BASELINE"; do
    if [ ! -f "$f" ]; then
        echo "ERROR: file not found: $f"
        exit 2
    fi
done

# We need jq or python3 for JSON parsing
if command -v jq &>/dev/null; then
    JSON_TOOL="jq"
elif command -v python3 &>/dev/null; then
    JSON_TOOL="python3"
else
    echo "ERROR: jq or python3 is required for JSON parsing"
    exit 2
fi

# ---------------------------------------------------------------------------
# JSON helpers
# ---------------------------------------------------------------------------
json_get() {
    local file="$1" key="$2"
    if [ "$JSON_TOOL" = "jq" ]; then
        jq -r ".$key // empty" "$file"
    else
        python3 -c "
import json, sys
with open('$file') as f:
    d = json.load(f)
v = d.get('$key')
if v is not None:
    print(v)
"
    fi
}

# ---------------------------------------------------------------------------
# Compare a single metric.
# For "higher is better" metrics (throughput) a *decrease* is a regression.
# For "lower is better" metrics (startup_time, binary_size) an *increase* is.
#
# compare_metric <name> <direction> <result_val> <baseline_val>
#   direction: "lower" = lower is better, "higher" = higher is better
# ---------------------------------------------------------------------------
FAIL_COUNT=0
DETAILS=""

compare_metric() {
    local name="$1" direction="$2" result_val="$3" baseline_val="$4"

    if [ -z "$result_val" ] || [ -z "$baseline_val" ]; then
        DETAILS+="  SKIP  $name (missing data)\n"
        return
    fi

    # Compute pct change using bc for floating-point math
    local pct_change
    pct_change=$(echo "scale=4; (($result_val - $baseline_val) / $baseline_val) * 100" | bc 2>/dev/null)

    if [ -z "$pct_change" ]; then
        DETAILS+="  SKIP  $name (calculation error)\n"
        return
    fi

    # Determine if this is a regression based on direction
    local is_regression=0
    if [ "$direction" = "lower" ]; then
        # Lower is better -> regression when result is HIGHER (positive pct)
        is_regression=$(echo "$pct_change > $THRESHOLD" | bc 2>/dev/null || echo 0)
    else
        # Higher is better -> regression when result is LOWER (negative pct)
        # We check if the *decrease* exceeds threshold
        local neg_change
        neg_change=$(echo "scale=4; $pct_change * -1" | bc 2>/dev/null)
        is_regression=$(echo "$neg_change > $THRESHOLD" | bc 2>/dev/null || echo 0)
    fi

    # Format for display
    local sign=""
    local abs_pct
    abs_pct=$(echo "$pct_change" | sed 's/^-//')
    if echo "$pct_change" | grep -q '^-'; then
        sign="-"
    else
        sign="+"
    fi

    if [ "$is_regression" = "1" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
        DETAILS+="  FAIL  $name: baseline=$baseline_val result=$result_val (${sign}${abs_pct}%% change, threshold=${THRESHOLD}%%)\n"
    else
        DETAILS+="  PASS  $name: baseline=$baseline_val result=$result_val (${sign}${abs_pct}%% change)\n"
    fi
}

# ---------------------------------------------------------------------------
# Extract metrics
# ---------------------------------------------------------------------------
R_STARTUP=$(json_get "$RESULTS" "startup_time_ms")
B_STARTUP=$(json_get "$BASELINE" "startup_time_ms")

R_SIZE=$(json_get "$RESULTS" "binary_size_kb")
B_SIZE=$(json_get "$BASELINE" "binary_size_kb")

R_THROUGHPUT=$(json_get "$RESULTS" "throughput_invocations_per_sec")
B_THROUGHPUT=$(json_get "$BASELINE" "throughput_invocations_per_sec")

# ---------------------------------------------------------------------------
# Run comparisons
# ---------------------------------------------------------------------------
echo "=== Benchmark Regression Check ==="
echo "Results:   $RESULTS"
echo "Baseline:  $BASELINE"
echo "Threshold: ${THRESHOLD}%"
echo ""

compare_metric "startup_time_ms"              "lower"  "$R_STARTUP"    "$B_STARTUP"
compare_metric "binary_size_kb"               "lower"  "$R_SIZE"       "$B_SIZE"
compare_metric "throughput_invocations_per_sec" "higher" "$R_THROUGHPUT" "$B_THROUGHPUT"

echo "--- Metric Details ---"
printf "$DETAILS"
echo ""

if [ "$FAIL_COUNT" -gt 0 ]; then
    echo "RESULT: FAIL -- $FAIL_COUNT regression(s) detected"
    exit 1
else
    echo "RESULT: PASS -- all metrics within ${THRESHOLD}% threshold"
    exit 0
fi
