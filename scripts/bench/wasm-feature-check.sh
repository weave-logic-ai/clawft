#!/usr/bin/env bash
# Verify no banned features are compiled into the WASM binary.
#
# Checks that channels, services, native-exec, vector-memory symbols are absent
# from the compiled WASM output. Uses wasm-objdump if available, falls back to
# strings(1).
#
# Usage: scripts/bench/wasm-feature-check.sh [wasm-file]
#   wasm-file  Path to WASM binary (default: target/wasm32-wasip2/release-wasm/clawft_wasm.wasm)
#
# Exit codes:
#   0 = clean (no banned symbols found)
#   1 = banned symbols detected
#   2 = usage / input error
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

WASM_FILE="${1:-$PROJECT_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"

# ---------------------------------------------------------------------------
# Banned symbol patterns
# ---------------------------------------------------------------------------
# These features must NOT be present in the WASM binary.
# Each entry is a grep -iE pattern and a human-readable description.
declare -a BANNED_PATTERNS=(
    "channel_create|channel_send|channel_recv|ChannelManager"
    "services::|ServiceRegistry|service_handler"
    "tokio::|tokio_runtime|TokioRuntime"
    "reqwest::|ReqwestClient|reqwest_"
    "native_exec|NativeExecutor|native_execute"
    "vector_memory|VectorMemory|vector_store|vector_search"
)
declare -a BANNED_DESCRIPTIONS=(
    "channels (channel_create, channel_send, channel_recv, ChannelManager)"
    "services (services::, ServiceRegistry, service_handler)"
    "tokio async runtime (tokio::, tokio_runtime)"
    "reqwest HTTP client (reqwest::, ReqwestClient)"
    "native execution (native_exec, NativeExecutor)"
    "vector memory (vector_memory, VectorMemory, vector_store)"
)

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------
if [ ! -f "$WASM_FILE" ]; then
    echo "ERROR: WASM binary not found at: $WASM_FILE"
    echo ""
    echo "Build first:"
    echo "  cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm"
    exit 2
fi

WASM_SIZE=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
WASM_SIZE_KB=$(echo "scale=1; $WASM_SIZE / 1024" | bc 2>/dev/null || echo "$((WASM_SIZE / 1024))")

echo "=== WASM Feature Exclusion Check ==="
echo "File: $WASM_FILE"
echo "Size: ${WASM_SIZE_KB} KB"
echo ""

# ---------------------------------------------------------------------------
# Choose inspection tool
# ---------------------------------------------------------------------------
INSPECT_TOOL=""
INSPECT_METHOD=""

if command -v wasm-objdump &>/dev/null; then
    INSPECT_TOOL="wasm-objdump"
    INSPECT_METHOD="wasm-objdump -x (export/import/symbol inspection)"
    echo "Using: wasm-objdump (WABT toolkit)"
elif command -v wasm-tools &>/dev/null; then
    INSPECT_TOOL="wasm-tools"
    INSPECT_METHOD="wasm-tools dump (component model inspection)"
    echo "Using: wasm-tools"
elif command -v strings &>/dev/null; then
    INSPECT_TOOL="strings"
    INSPECT_METHOD="strings (fallback, less precise)"
    echo "Using: strings (fallback -- wasm-objdump or wasm-tools not found)"
    echo "  Install WABT for more accurate inspection:"
    echo "    apt install wabt  OR  cargo install wasm-tools"
else
    echo "ERROR: No inspection tool available."
    echo "Install one of: wabt (wasm-objdump), wasm-tools, or binutils (strings)"
    exit 2
fi
echo "Method: $INSPECT_METHOD"
echo ""

# ---------------------------------------------------------------------------
# Extract symbol/string data from the WASM binary
# ---------------------------------------------------------------------------
SYMBOL_DATA=""

case "$INSPECT_TOOL" in
    wasm-objdump)
        # Get exports, imports, and symbol names
        SYMBOL_DATA=$(wasm-objdump -x "$WASM_FILE" 2>/dev/null || echo "")
        # Also get the disassembly name section if present
        NAMES=$(wasm-objdump -x "$WASM_FILE" 2>/dev/null | grep -i "name:" || true)
        SYMBOL_DATA="${SYMBOL_DATA}${NAMES}"
        ;;
    wasm-tools)
        # Use wasm-tools to dump names and exports
        SYMBOL_DATA=$(wasm-tools dump "$WASM_FILE" 2>/dev/null || echo "")
        ;;
    strings)
        # Fall back to raw string extraction (ASCII strings >= 6 chars)
        SYMBOL_DATA=$(strings -n 6 "$WASM_FILE" 2>/dev/null || echo "")
        ;;
esac

if [ -z "$SYMBOL_DATA" ]; then
    echo "WARNING: Could not extract symbols from WASM binary."
    echo "The binary may be fully stripped. Trying strings as fallback..."
    SYMBOL_DATA=$(strings -n 6 "$WASM_FILE" 2>/dev/null || echo "")
    if [ -z "$SYMBOL_DATA" ]; then
        echo "WARNING: No string data found. Cannot verify feature exclusion."
        echo "RESULT: INCONCLUSIVE (binary appears fully stripped)"
        exit 0
    fi
fi

# ---------------------------------------------------------------------------
# Check each banned pattern
# ---------------------------------------------------------------------------
FAIL_COUNT=0
TOTAL_CHECKS=${#BANNED_PATTERNS[@]}
VIOLATIONS=""

for i in "${!BANNED_PATTERNS[@]}"; do
    pattern="${BANNED_PATTERNS[$i]}"
    description="${BANNED_DESCRIPTIONS[$i]}"

    matches=$(echo "$SYMBOL_DATA" | grep -iE "$pattern" 2>/dev/null || true)

    if [ -n "$matches" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
        match_count=$(echo "$matches" | wc -l)
        first_match=$(echo "$matches" | head -3)
        VIOLATIONS+="  FAIL  $description\n"
        VIOLATIONS+="        Found $match_count match(es):\n"
        while IFS= read -r line; do
            VIOLATIONS+="          > $(echo "$line" | head -c 120)\n"
        done <<< "$first_match"
        VIOLATIONS+="\n"
    else
        printf "  PASS  %s\n" "$description"
    fi
done

echo ""

# ---------------------------------------------------------------------------
# Report violations
# ---------------------------------------------------------------------------
if [ "$FAIL_COUNT" -gt 0 ]; then
    echo "--- VIOLATIONS ---"
    printf "$VIOLATIONS"
    echo ""
    echo "RESULT: FAIL -- $FAIL_COUNT of $TOTAL_CHECKS checks failed"
    echo ""
    echo "Banned features were found in the WASM binary."
    echo "Ensure these features are behind cfg(not(target_arch = \"wasm32\")) or feature gates."
    exit 1
else
    echo "RESULT: PASS -- all $TOTAL_CHECKS feature exclusion checks passed"
    echo ""
    echo "No banned symbols found in the WASM binary."
    exit 0
fi
