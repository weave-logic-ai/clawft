#!/usr/bin/env bash
# Run benchmarks and save results as a JSON file.
#
# Usage: save-results.sh [binary] [output_file] [iterations]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="${1:-clawft/target/release/weft}"
OUTPUT="${2:-bench-results.json}"
ITERATIONS="${3:-50}"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    echo "Build first: cd clawft && cargo build --release"
    exit 1
fi

echo "=== Saving Benchmark Results ==="
echo "Binary:     $BINARY"
echo "Output:     $OUTPUT"
echo "Iterations: $ITERATIONS"
echo ""

# ---------------------------------------------------------------------------
# 1. Startup time (mean over N iterations)
# ---------------------------------------------------------------------------
echo "Measuring startup time ($ITERATIONS iterations)..."
TOTAL_MS=0
for i in $(seq 1 "$ITERATIONS"); do
    START=$(date +%s%N)
    "$BINARY" --version > /dev/null 2>&1 || true
    END=$(date +%s%N)
    ELAPSED_NS=$((END - START))
    ELAPSED_MS=$((ELAPSED_NS / 1000000))
    TOTAL_MS=$((TOTAL_MS + ELAPSED_MS))
done
# Use bc for fractional average
STARTUP_AVG=$(echo "scale=2; $TOTAL_MS / $ITERATIONS" | bc 2>/dev/null || echo "$((TOTAL_MS / ITERATIONS))")
echo "  Startup average: ${STARTUP_AVG} ms"

# ---------------------------------------------------------------------------
# 2. Binary size
# ---------------------------------------------------------------------------
SIZE_BYTES=$(stat -c%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY" 2>/dev/null || echo "0")
SIZE_KB=$((SIZE_BYTES / 1024))
echo "  Binary size: ${SIZE_KB} KB"

# ---------------------------------------------------------------------------
# 3. Throughput (invocations/sec over 100 calls)
# ---------------------------------------------------------------------------
THROUGHPUT_COUNT=100
echo "Measuring throughput ($THROUGHPUT_COUNT invocations)..."
T_START=$(date +%s%N)
for i in $(seq 1 "$THROUGHPUT_COUNT"); do
    "$BINARY" --version > /dev/null 2>&1 || true
done
T_END=$(date +%s%N)
T_ELAPSED_MS=$(( (T_END - T_START) / 1000000 ))
if [ "$T_ELAPSED_MS" -gt 0 ]; then
    THROUGHPUT=$(echo "scale=1; $THROUGHPUT_COUNT * 1000 / $T_ELAPSED_MS" | bc 2>/dev/null || echo "0")
else
    THROUGHPUT="0"
fi
echo "  Throughput: ${THROUGHPUT} invocations/sec"

# ---------------------------------------------------------------------------
# 4. Write JSON
# ---------------------------------------------------------------------------
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

if command -v jq &>/dev/null; then
    jq -n \
        --arg st "$STARTUP_AVG" \
        --arg sz "$SIZE_KB" \
        --arg tp "$THROUGHPUT" \
        --arg ts "$TIMESTAMP" \
        '{
            startup_time_ms: ($st | tonumber),
            binary_size_kb: ($sz | tonumber),
            throughput_invocations_per_sec: ($tp | tonumber),
            timestamp: $ts
        }' > "$OUTPUT"
elif command -v python3 &>/dev/null; then
    python3 -c "
import json, sys
data = {
    'startup_time_ms': float('$STARTUP_AVG'),
    'binary_size_kb': int('$SIZE_KB'),
    'throughput_invocations_per_sec': float('$THROUGHPUT'),
    'timestamp': '$TIMESTAMP'
}
with open('$OUTPUT', 'w') as f:
    json.dump(data, f, indent=2)
    f.write('\n')
"
else
    # Fallback: hand-write JSON
    cat > "$OUTPUT" <<EOJSON
{
  "startup_time_ms": $STARTUP_AVG,
  "binary_size_kb": $SIZE_KB,
  "throughput_invocations_per_sec": $THROUGHPUT,
  "timestamp": "$TIMESTAMP"
}
EOJSON
fi

echo ""
echo "Results saved to: $OUTPUT"
echo "Contents:"
cat "$OUTPUT"
