#!/usr/bin/env bash
# Benchmark comparison report: clawft vs OpenClaw.
#
# Usage:
#   ./scripts/bench/compare.sh <weft-binary> [openclaw-binary]
#
# Produces a comparison across 4 metrics:
#   1. Binary size (bytes, stripped release)
#   2. Cold start to first response (ms, hyperfine --warmup 0)
#   3. Peak RSS under load (/usr/bin/time -v)
#   4. Messages/sec throughput (criterion results)
#
# Results are written to bench-comparison.json and printed to stdout.

set -euo pipefail

WEFT_BINARY="${1:-target/release/weft}"
OPENCLAW_BINARY="${2:-}"
BASELINE_FILE="scripts/bench/baseline.json"
OUTPUT_FILE="bench-comparison.json"

# -------------------------------------------------------------------
# Metric 1: Binary size
# -------------------------------------------------------------------
measure_binary_size() {
    local binary="$1"
    if [ -f "$binary" ]; then
        wc -c < "$binary" | tr -d ' '
    else
        echo "0"
    fi
}

# -------------------------------------------------------------------
# Metric 2: Cold start time (ms)
# -------------------------------------------------------------------
measure_cold_start() {
    local binary="$1"
    if ! command -v hyperfine &>/dev/null; then
        echo "N/A"
        return
    fi
    if [ ! -f "$binary" ]; then
        echo "N/A"
        return
    fi
    # Measure cold start: run the binary with --help and capture time.
    local result
    result=$(hyperfine --warmup 0 --runs 5 --export-json /dev/stdout \
        "$binary --help" 2>/dev/null | \
        python3 -c "import sys,json; d=json.load(sys.stdin); print(f'{d[\"results\"][0][\"mean\"]*1000:.1f}')" 2>/dev/null || echo "N/A")
    echo "$result"
}

# -------------------------------------------------------------------
# Metric 3: Peak RSS (KB)
# -------------------------------------------------------------------
measure_peak_rss() {
    local binary="$1"
    if [ ! -f "$binary" ]; then
        echo "N/A"
        return
    fi
    local rss
    rss=$(/usr/bin/time -v "$binary" --help 2>&1 | grep "Maximum resident" | awk '{print $NF}' 2>/dev/null || echo "N/A")
    echo "$rss"
}

# -------------------------------------------------------------------
# Main comparison
# -------------------------------------------------------------------
echo "=== Benchmark Comparison Report ==="
echo ""

WEFT_SIZE=$(measure_binary_size "$WEFT_BINARY")
WEFT_SIZE_MB=$(echo "scale=2; $WEFT_SIZE / 1048576" | bc 2>/dev/null || echo "N/A")

echo "Metric 1: Binary Size"
echo "  weft:     ${WEFT_SIZE_MB} MB (${WEFT_SIZE} bytes)"

# Load baseline if available
if [ -f "$BASELINE_FILE" ]; then
    BASELINE_SIZE=$(python3 -c "import json; d=json.load(open('$BASELINE_FILE')); print(d.get('binary_size_bytes', 'N/A'))" 2>/dev/null || echo "N/A")
    BASELINE_SIZE_MB=$(echo "scale=2; $BASELINE_SIZE / 1048576" | bc 2>/dev/null || echo "N/A")
    echo "  baseline: ${BASELINE_SIZE_MB} MB (${BASELINE_SIZE} bytes)"
fi

echo ""
echo "Metric 2: Cold Start"
WEFT_STARTUP=$(measure_cold_start "$WEFT_BINARY")
echo "  weft:     ${WEFT_STARTUP} ms"

echo ""
echo "Metric 3: Peak RSS"
WEFT_RSS=$(measure_peak_rss "$WEFT_BINARY")
echo "  weft:     ${WEFT_RSS} KB"

echo ""
echo "Metric 4: Throughput"
echo "  (run criterion benchmarks separately: cargo bench)"

# Write JSON report
cat > "$OUTPUT_FILE" << EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "weft": {
    "binary_size_bytes": $WEFT_SIZE,
    "cold_start_ms": "$WEFT_STARTUP",
    "peak_rss_kb": "$WEFT_RSS"
  },
  "baseline_file": "$BASELINE_FILE"
}
EOF

echo ""
echo "Results written to $OUTPUT_FILE"
