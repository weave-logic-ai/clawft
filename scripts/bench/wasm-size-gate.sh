#!/usr/bin/env bash
# Checks WASM binary against size budget.
#
# Usage: scripts/bench/wasm-size-gate.sh [wasm-file] [max-raw-kb] [max-gz-kb]
#
# Defaults:
#   wasm-file:  target/wasm32-wasip2/release-wasm/clawft_wasm.wasm
#   max-raw-kb: 300
#   max-gz-kb:  120
#
# Exit codes:
#   0 - Binary is within budget
#   1 - Binary exceeds budget

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()    { printf "${GREEN}[PASS]${NC}  %s\n" "$*"; }
warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
fail()  { printf "${RED}[FAIL]${NC}  %s\n" "$*"; }

# --- Resolve workspace root ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# --- Parse arguments ---
WASM_FILE="${1:-$WORKSPACE_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"
MAX_RAW_KB="${2:-300}"
MAX_GZ_KB="${3:-120}"

# Fallback to wasip1 if primary target not found
if [ ! -f "$WASM_FILE" ] && [ "$WASM_FILE" = "$WORKSPACE_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm" ]; then
    FALLBACK="$WORKSPACE_ROOT/target/wasm32-wasip1/release-wasm/clawft_wasm.wasm"
    if [ -f "$FALLBACK" ]; then
        warn "Primary target not found, falling back to wasip1: $FALLBACK"
        WASM_FILE="$FALLBACK"
    fi
fi

# Also try .opt.wasm variant
if [ ! -f "$WASM_FILE" ]; then
    OPT_VARIANT="${WASM_FILE%.wasm}.opt.wasm"
    if [ -f "$OPT_VARIANT" ]; then
        info "Using optimized variant: $OPT_VARIANT"
        WASM_FILE="$OPT_VARIANT"
    fi
fi

# --- Validate input ---
if [ ! -f "$WASM_FILE" ]; then
    fail "WASM file not found: $WASM_FILE"
    echo ""
    echo "Build the WASM binary first:"
    echo "  cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm"
    exit 1
fi

# --- Measure raw size ---
RAW_BYTES=$(wc -c < "$WASM_FILE")
RAW_KB=$(echo "scale=1; $RAW_BYTES / 1024" | bc)

# --- Measure gzipped size ---
GZ_BYTES=$(gzip -9 -c "$WASM_FILE" | wc -c)
GZ_KB=$(echo "scale=1; $GZ_BYTES / 1024" | bc)

# --- Report ---
echo "=== WASM Size Gate ==="
echo "File:    $WASM_FILE"
echo "Raw:     $RAW_KB KB ($RAW_BYTES bytes)"
echo "Gzipped: $GZ_KB KB ($GZ_BYTES bytes)"
echo "Budget:  $MAX_RAW_KB KB raw / $MAX_GZ_KB KB gzipped"
echo ""

# --- Check raw size ---
# Use integer comparison: multiply by 10 to handle one decimal place from bc
RAW_KB_INT=$(echo "$RAW_BYTES / 1024" | bc)
GZ_KB_INT=$(echo "$GZ_BYTES / 1024" | bc)

PASSED=true

if [ "$RAW_KB_INT" -le "$MAX_RAW_KB" ]; then
    ok "Raw size: ${RAW_KB} KB <= ${MAX_RAW_KB} KB budget"
else
    fail "Raw size: ${RAW_KB} KB > ${MAX_RAW_KB} KB budget (${RAW_KB_INT} KB over by $((RAW_KB_INT - MAX_RAW_KB)) KB)"
    PASSED=false
fi

if [ "$GZ_KB_INT" -le "$MAX_GZ_KB" ]; then
    ok "Gzipped:  ${GZ_KB} KB <= ${MAX_GZ_KB} KB budget"
else
    fail "Gzipped:  ${GZ_KB} KB > ${MAX_GZ_KB} KB budget (${GZ_KB_INT} KB over by $((GZ_KB_INT - MAX_GZ_KB)) KB)"
    PASSED=false
fi

echo ""

if [ "$PASSED" = true ]; then
    ok "WASM binary is within size budget."
    exit 0
else
    fail "WASM binary exceeds size budget!"
    echo ""
    echo "To investigate, run:"
    echo "  scripts/bench/wasm-twiggy.sh $WASM_FILE"
    echo ""
    echo "To adjust thresholds (if intentional growth):"
    echo "  scripts/bench/wasm-size-gate.sh $WASM_FILE <new-raw-kb> <new-gz-kb>"
    exit 1
fi
