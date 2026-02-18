#!/usr/bin/env bash
# Benchmark: WASM instantiation time measurement
# Measures cold-start and warm-start times for the clawft WASM binary using wasmtime.
#
# Requires: wasmtime CLI (https://wasmtime.dev)
#
# Usage: scripts/bench/wasm-startup.sh [wasm-file] [iterations]
#   wasm-file   Path to WASM binary (default: target/wasm32-wasip2/release-wasm/clawft_wasm.wasm)
#   iterations  Number of warm-start iterations (default: 50)
#
# Outputs:
#   - Human-readable table to stdout
#   - JSON results to target/wasm-bench/startup.json
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

WASM_FILE="${1:-$PROJECT_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"
ITERATIONS="${2:-50}"
RESULTS_DIR="$PROJECT_ROOT/target/wasm-bench"

# ---------------------------------------------------------------------------
# Preflight checks
# ---------------------------------------------------------------------------
if ! command -v wasmtime &>/dev/null; then
    echo "ERROR: wasmtime is not installed."
    echo "Install: curl https://wasmtime.dev/install.sh -sSf | bash"
    echo ""
    echo "Alternatively, install via your package manager:"
    echo "  cargo install wasmtime-cli"
    exit 1
fi

echo "=== WASM Startup Benchmark ==="
echo "wasmtime: $(wasmtime --version 2>/dev/null || echo 'unknown')"
echo ""

# ---------------------------------------------------------------------------
# Build WASM binary if not present
# ---------------------------------------------------------------------------
if [ ! -f "$WASM_FILE" ]; then
    echo "WASM binary not found at: $WASM_FILE"
    echo "Building clawft-wasm (this may take a minute)..."
    echo ""

    cd "$PROJECT_ROOT"
    cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm

    if [ ! -f "$WASM_FILE" ]; then
        echo "ERROR: Build succeeded but WASM binary not found at expected location."
        echo "Searching for .wasm files..."
        find "$PROJECT_ROOT/target" -name "*.wasm" -type f 2>/dev/null | head -5
        exit 1
    fi
    echo ""
fi

WASM_SIZE=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
WASM_SIZE_KB=$(echo "scale=1; $WASM_SIZE / 1024" | bc 2>/dev/null || echo "$((WASM_SIZE / 1024))")

echo "WASM file: $WASM_FILE"
echo "WASM size: ${WASM_SIZE_KB} KB"
echo "Iterations: $ITERATIONS (warm start)"
echo ""

# ---------------------------------------------------------------------------
# Timing helper: returns elapsed time in nanoseconds
# Uses /proc/uptime as a high-resolution monotonic source on Linux,
# falls back to date +%s%N.
# ---------------------------------------------------------------------------
now_ns() {
    if [ -r /proc/uptime ]; then
        # /proc/uptime gives seconds with centisecond resolution; use date for ns
        date +%s%N
    else
        date +%s%N
    fi
}

# ---------------------------------------------------------------------------
# Cold start: first invocation (includes module compilation)
# wasmtime compiles the module on first run; no cache exists yet.
# ---------------------------------------------------------------------------
echo "--- Cold Start (first invocation, includes compilation) ---"

# Clear wasmtime cache to ensure a true cold start
WASMTIME_CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/wasmtime"
if [ -d "$WASMTIME_CACHE_DIR" ]; then
    rm -rf "$WASMTIME_CACHE_DIR" 2>/dev/null || true
fi

COLD_START=$(now_ns)
wasmtime run "$WASM_FILE" -- capabilities >/dev/null 2>&1 || true
COLD_END=$(now_ns)

COLD_NS=$((COLD_END - COLD_START))
COLD_MS=$(echo "scale=3; $COLD_NS / 1000000" | bc 2>/dev/null || echo "$((COLD_NS / 1000000))")

echo "  Cold start: ${COLD_MS} ms"
echo ""

# ---------------------------------------------------------------------------
# Warm starts: subsequent invocations (wasmtime caches the compiled module)
# ---------------------------------------------------------------------------
echo "--- Warm Start (${ITERATIONS} iterations, cached compilation) ---"

WARM_TIMES=()
WARM_TOTAL_NS=0

for i in $(seq 1 "$ITERATIONS"); do
    START=$(now_ns)
    wasmtime run "$WASM_FILE" -- capabilities >/dev/null 2>&1 || true
    END=$(now_ns)

    ELAPSED_NS=$((END - START))
    WARM_TIMES+=("$ELAPSED_NS")
    WARM_TOTAL_NS=$((WARM_TOTAL_NS + ELAPSED_NS))

    # Print progress every 10 iterations
    if [ $((i % 10)) -eq 0 ] || [ "$i" -eq 1 ]; then
        ELAPSED_MS=$(echo "scale=3; $ELAPSED_NS / 1000000" | bc 2>/dev/null || echo "$((ELAPSED_NS / 1000000))")
        printf "  Run %3d/%d: %s ms\n" "$i" "$ITERATIONS" "$ELAPSED_MS"
    fi
done

echo ""

# ---------------------------------------------------------------------------
# Statistics calculation
# ---------------------------------------------------------------------------

# Sort times for percentile calculation
IFS=$'\n' SORTED_TIMES=($(printf '%s\n' "${WARM_TIMES[@]}" | sort -n))
unset IFS

WARM_COUNT=${#SORTED_TIMES[@]}

# Min
WARM_MIN_NS=${SORTED_TIMES[0]}
WARM_MIN_MS=$(echo "scale=3; $WARM_MIN_NS / 1000000" | bc 2>/dev/null || echo "$((WARM_MIN_NS / 1000000))")

# Max
WARM_MAX_NS=${SORTED_TIMES[$((WARM_COUNT - 1))]}
WARM_MAX_MS=$(echo "scale=3; $WARM_MAX_NS / 1000000" | bc 2>/dev/null || echo "$((WARM_MAX_NS / 1000000))")

# Mean
WARM_MEAN_NS=$((WARM_TOTAL_NS / WARM_COUNT))
WARM_MEAN_MS=$(echo "scale=3; $WARM_MEAN_NS / 1000000" | bc 2>/dev/null || echo "$((WARM_MEAN_NS / 1000000))")

# P50 (median)
P50_IDX=$((WARM_COUNT / 2))
WARM_P50_NS=${SORTED_TIMES[$P50_IDX]}
WARM_P50_MS=$(echo "scale=3; $WARM_P50_NS / 1000000" | bc 2>/dev/null || echo "$((WARM_P50_NS / 1000000))")

# P90
P90_IDX=$(( (WARM_COUNT * 90) / 100 ))
if [ "$P90_IDX" -ge "$WARM_COUNT" ]; then P90_IDX=$((WARM_COUNT - 1)); fi
WARM_P90_NS=${SORTED_TIMES[$P90_IDX]}
WARM_P90_MS=$(echo "scale=3; $WARM_P90_NS / 1000000" | bc 2>/dev/null || echo "$((WARM_P90_NS / 1000000))")

# P99
P99_IDX=$(( (WARM_COUNT * 99) / 100 ))
if [ "$P99_IDX" -ge "$WARM_COUNT" ]; then P99_IDX=$((WARM_COUNT - 1)); fi
WARM_P99_NS=${SORTED_TIMES[$P99_IDX]}
WARM_P99_MS=$(echo "scale=3; $WARM_P99_NS / 1000000" | bc 2>/dev/null || echo "$((WARM_P99_NS / 1000000))")

# ---------------------------------------------------------------------------
# Results table
# ---------------------------------------------------------------------------
echo "=== Results ==="
echo ""
printf "  %-20s %s\n" "Metric" "Value"
printf "  %-20s %s\n" "--------------------" "-----------"
printf "  %-20s %s ms\n" "Cold start" "$COLD_MS"
printf "  %-20s %s ms\n" "Warm mean" "$WARM_MEAN_MS"
printf "  %-20s %s ms\n" "Warm P50 (median)" "$WARM_P50_MS"
printf "  %-20s %s ms\n" "Warm P90" "$WARM_P90_MS"
printf "  %-20s %s ms\n" "Warm P99" "$WARM_P99_MS"
printf "  %-20s %s ms\n" "Warm min" "$WARM_MIN_MS"
printf "  %-20s %s ms\n" "Warm max" "$WARM_MAX_MS"
printf "  %-20s %d\n"    "Iterations" "$ITERATIONS"
echo ""

# Target checks
echo "--- Target Checks ---"
COLD_PASS="PASS"
if [ "$(echo "$COLD_MS > 5" | bc 2>/dev/null || echo 0)" = "1" ]; then
    COLD_PASS="FAIL"
fi
echo "  Cold start < 5 ms:  ${COLD_PASS} (${COLD_MS} ms)"

WARM_PASS="PASS"
if [ "$(echo "$WARM_P50_MS > 1" | bc 2>/dev/null || echo 0)" = "1" ]; then
    WARM_PASS="FAIL"
fi
echo "  Warm P50 < 1 ms:    ${WARM_PASS} (${WARM_P50_MS} ms)"
echo ""

# ---------------------------------------------------------------------------
# Save JSON results
# ---------------------------------------------------------------------------
mkdir -p "$RESULTS_DIR"
RESULTS_FILE="$RESULTS_DIR/startup.json"

cat > "$RESULTS_FILE" <<EOF
{
  "benchmark": "wasm_startup",
  "wasm_file": "$WASM_FILE",
  "wasm_size_bytes": $WASM_SIZE,
  "wasmtime_version": "$(wasmtime --version 2>/dev/null || echo 'unknown')",
  "cold_start_ms": $COLD_MS,
  "warm_start": {
    "iterations": $ITERATIONS,
    "mean_ms": $WARM_MEAN_MS,
    "p50_ms": $WARM_P50_MS,
    "p90_ms": $WARM_P90_MS,
    "p99_ms": $WARM_P99_MS,
    "min_ms": $WARM_MIN_MS,
    "max_ms": $WARM_MAX_MS
  },
  "targets": {
    "cold_start_under_5ms": "$COLD_PASS",
    "warm_p50_under_1ms": "$WARM_PASS"
  },
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

echo "Results saved to: $RESULTS_FILE"

# Also output final JSON to stdout (machine-readable last line)
echo ""
cat "$RESULTS_FILE"
