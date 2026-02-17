#!/bin/bash
# Benchmark: Startup time measurement
# Measures cold start time for `weft --version`
set -euo pipefail

BINARY="${1:-clawft/target/release/weft}"
ITERATIONS="${2:-10}"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    echo "Build first: cd clawft && cargo build --release"
    exit 1
fi

echo "=== Startup Time Benchmark ==="
echo "Binary: $BINARY"
echo "Iterations: $ITERATIONS"
echo ""

TOTAL_MS=0
MIN_MS=999999
MAX_MS=0

for i in $(seq 1 "$ITERATIONS"); do
    # Clear filesystem cache (best-effort, needs root)
    sync 2>/dev/null || true

    START=$(date +%s%N)
    "$BINARY" --version > /dev/null 2>&1 || true
    END=$(date +%s%N)

    ELAPSED_NS=$((END - START))
    ELAPSED_MS=$((ELAPSED_NS / 1000000))

    TOTAL_MS=$((TOTAL_MS + ELAPSED_MS))
    if [ "$ELAPSED_MS" -lt "$MIN_MS" ]; then MIN_MS=$ELAPSED_MS; fi
    if [ "$ELAPSED_MS" -gt "$MAX_MS" ]; then MAX_MS=$ELAPSED_MS; fi

    printf "  Run %2d: %d ms\n" "$i" "$ELAPSED_MS"
done

AVG_MS=$((TOTAL_MS / ITERATIONS))

echo ""
echo "Results:"
echo "  Average: ${AVG_MS} ms"
echo "  Min:     ${MIN_MS} ms"
echo "  Max:     ${MAX_MS} ms"
echo ""

# Output as machine-readable JSON
echo "{\"benchmark\": \"startup_time\", \"avg_ms\": $AVG_MS, \"min_ms\": $MIN_MS, \"max_ms\": $MAX_MS, \"iterations\": $ITERATIONS}"
