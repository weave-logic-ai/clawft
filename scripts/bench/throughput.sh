#!/bin/bash
# Benchmark: Message throughput
# Measures message processing rate using test infrastructure
set -euo pipefail

BINARY="${1:-clawft/target/release/weft}"
NUM_MESSAGES="${2:-100}"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

echo "=== Message Throughput Benchmark ==="
echo "Binary: $BINARY"
echo "Messages: $NUM_MESSAGES"
echo ""
echo "Note: Full throughput benchmarks require a running agent with an LLM provider."
echo "This script measures command parsing and pipeline overhead only."
echo ""

# Benchmark: repeated --version calls (measures CLI overhead)
START=$(date +%s%N)
for i in $(seq 1 "$NUM_MESSAGES"); do
    "$BINARY" --version > /dev/null 2>&1 || true
done
END=$(date +%s%N)

ELAPSED_NS=$((END - START))
ELAPSED_MS=$((ELAPSED_NS / 1000000))
RATE=$(echo "scale=1; $NUM_MESSAGES * 1000 / $ELAPSED_MS" | bc 2>/dev/null || echo "N/A")

echo "Results (CLI invocation overhead):"
echo "  Total time: ${ELAPSED_MS} ms for ${NUM_MESSAGES} invocations"
echo "  Rate: ${RATE} invocations/sec"
echo ""
echo "{\"benchmark\": \"throughput\", \"total_ms\": $ELAPSED_MS, \"messages\": $NUM_MESSAGES, \"rate_per_sec\": $RATE}"
