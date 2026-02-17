#!/bin/bash
# Run all benchmarks and output combined results
#
# Usage: run-all.sh [binary] [report_file] [options]
#
# Options:
#   --save-results <file>   Save results as JSON to <file>
#   --check-regression      Run regression check against baseline after benchmarks
#   --baseline <file>       Baseline JSON file (default: scripts/bench/baseline.json)
#   --threshold <pct>       Regression threshold percentage (default: 10)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
BINARY="clawft/target/release/weft"
REPORT_FILE=".planning/development_notes/report_benchmarks.md"
SAVE_RESULTS=""
CHECK_REGRESSION=0
BASELINE_FILE="$SCRIPT_DIR/baseline.json"
THRESHOLD=10

POSITIONAL_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --save-results)
            SAVE_RESULTS="$2"
            shift 2
            ;;
        --check-regression)
            CHECK_REGRESSION=1
            shift
            ;;
        --baseline)
            BASELINE_FILE="$2"
            shift 2
            ;;
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        *)
            POSITIONAL_ARGS+=("$1")
            shift
            ;;
    esac
done

# Restore positional args
if [ "${#POSITIONAL_ARGS[@]}" -ge 1 ]; then
    BINARY="${POSITIONAL_ARGS[0]}"
fi
if [ "${#POSITIONAL_ARGS[@]}" -ge 2 ]; then
    REPORT_FILE="${POSITIONAL_ARGS[1]}"
fi

echo "Running all benchmarks..."
echo "Binary: $BINARY"
echo "Report: $REPORT_FILE"
echo ""

# Collect results
STARTUP_JSON=$("$SCRIPT_DIR/startup-time.sh" "$BINARY" 10 2>/dev/null | tail -1)
MEMORY_JSON=$("$SCRIPT_DIR/memory-usage.sh" "$BINARY" 2>/dev/null | tail -1)
THROUGHPUT_JSON=$("$SCRIPT_DIR/throughput.sh" "$BINARY" 100 2>/dev/null | tail -1)

# Extract values
STARTUP_AVG=$(echo "$STARTUP_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('avg_ms','N/A'))" 2>/dev/null || echo "N/A")
RSS_MB=$(echo "$MEMORY_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('peak_rss_mb','N/A'))" 2>/dev/null || echo "N/A")
RATE=$(echo "$THROUGHPUT_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin).get('rate_per_sec','N/A'))" 2>/dev/null || echo "N/A")

# Get binary size
if [ -f "$BINARY" ]; then
    SIZE_BYTES=$(stat -c%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY" 2>/dev/null || echo "0")
    SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc 2>/dev/null || echo "N/A")
    SIZE_KB=$((SIZE_BYTES / 1024))
else
    SIZE_MB="N/A"
    SIZE_KB=0
fi

DATE=$(date -u +"%Y-%m-%d %H:%M UTC")

echo ""
echo "=== Summary ==="
echo "Startup: ${STARTUP_AVG} ms"
echo "RSS: ${RSS_MB} MB"
echo "Throughput: ${RATE}/s"
echo "Binary size: ${SIZE_MB} MB"

# ---------------------------------------------------------------------------
# Save results as JSON (optional)
# ---------------------------------------------------------------------------
if [ -n "$SAVE_RESULTS" ]; then
    echo ""
    echo "Saving results to $SAVE_RESULTS ..."
    "$SCRIPT_DIR/save-results.sh" "$BINARY" "$SAVE_RESULTS"
fi

# ---------------------------------------------------------------------------
# Regression check (optional)
# ---------------------------------------------------------------------------
if [ "$CHECK_REGRESSION" = "1" ]; then
    echo ""
    # If save-results was used, check against that file; otherwise generate one
    RESULTS_FILE="$SAVE_RESULTS"
    if [ -z "$RESULTS_FILE" ]; then
        RESULTS_FILE=$(mktemp /tmp/bench-results-XXXXXX.json)
        "$SCRIPT_DIR/save-results.sh" "$BINARY" "$RESULTS_FILE" > /dev/null 2>&1
        trap "rm -f '$RESULTS_FILE'" EXIT
    fi
    "$SCRIPT_DIR/regression-check.sh" "$RESULTS_FILE" "$BASELINE_FILE" "$THRESHOLD"
fi
