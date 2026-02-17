#!/usr/bin/env bash
# Profile WASM binary size with section breakdown
set -euo pipefail

WASM_FILE="${1:-target/wasm32-wasip1/release/clawft_wasm.wasm}"

if [ ! -f "$WASM_FILE" ]; then
    echo "WASM file not found: $WASM_FILE"
    echo "Build with: cargo build --target wasm32-wasip1 --release -p clawft-wasm"
    exit 1
fi

SIZE_BYTES=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
SIZE_KB=$((SIZE_BYTES / 1024))

echo "=== WASM Size Report ==="
echo "File: $WASM_FILE"
echo "Size: ${SIZE_KB} KB (${SIZE_BYTES} bytes)"
echo ""

# Check target
TARGET_KB=300
if [ "$SIZE_KB" -le "$TARGET_KB" ]; then
    echo "PASS: ${SIZE_KB} KB <= ${TARGET_KB} KB target"
else
    echo "FAIL: ${SIZE_KB} KB > ${TARGET_KB} KB target"
    exit 1
fi

# Gzip size
GZIP_SIZE=$(gzip -c "$WASM_FILE" | wc -c)
GZIP_KB=$((GZIP_SIZE / 1024))
echo "Gzipped: ${GZIP_KB} KB (target: <= 120 KB)"

if [ "$GZIP_KB" -le 120 ]; then
    echo "GZIP PASS"
else
    echo "GZIP FAIL"
fi

# Section analysis if wasm-objdump available
if command -v wasm-objdump &>/dev/null; then
    echo ""
    echo "=== Section Breakdown ==="
    wasm-objdump -h "$WASM_FILE"
fi
