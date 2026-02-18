#!/usr/bin/env bash
# Build clawft-wasm with each allocator and compare binary sizes.
#
# Usage:
#   scripts/bench/alloc-compare.sh [--wasm-opt]

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="wasm32-wasip2"
PROFILE="release-wasm"
APPLY_WASM_OPT=false

if [ "${1:-}" = "--wasm-opt" ]; then
    APPLY_WASM_OPT=true
fi

OUTPUT_DIR="$WORKSPACE_ROOT/target/$TARGET/$PROFILE"

echo "=== Allocator Comparison ==="
echo "Target:   $TARGET"
echo "Profile:  $PROFILE"
echo "wasm-opt: $APPLY_WASM_OPT"
echo ""
printf "%-12s  %8s  %8s  %8s\n" "Allocator" "Raw (KB)" "Opt (KB)" "Gzip (KB)"
printf "%-12s  %8s  %8s  %8s\n" "---------" "--------" "--------" "---------"

for ALLOC_NAME in dlmalloc talc lol_alloc; do
    case "$ALLOC_NAME" in
        dlmalloc)  FLAGS="" ;;
        talc)      FLAGS="--features alloc-talc" ;;
        lol_alloc) FLAGS="--features alloc-lol" ;;
    esac

    # Build
    # shellcheck disable=SC2086
    if ! cargo build -p clawft-wasm \
        --target "$TARGET" \
        --profile "$PROFILE" \
        $FLAGS \
        2>/dev/null; then
        printf "%-12s  %8s  %8s  %8s\n" "$ALLOC_NAME" "FAIL" "-" "-"
        continue
    fi

    RAW_FILE="$OUTPUT_DIR/clawft_wasm.wasm"
    if [ ! -f "$RAW_FILE" ]; then
        printf "%-12s  %8s  %8s  %8s\n" "$ALLOC_NAME" "NO FILE" "-" "-"
        continue
    fi

    RAW_BYTES=$(stat -c%s "$RAW_FILE" 2>/dev/null || stat -f%z "$RAW_FILE")
    RAW_KB=$(( RAW_BYTES / 1024 ))

    OPT_KB="-"
    GZIP_KB="-"

    if [ "$APPLY_WASM_OPT" = true ] && command -v wasm-opt &>/dev/null; then
        OPT_FILE="$OUTPUT_DIR/clawft_wasm.opt.wasm"
        wasm-opt -Oz \
            --strip-debug --strip-dwarf --strip-producers \
            --zero-filled-memory --converge \
            -o "$OPT_FILE" "$RAW_FILE" 2>/dev/null
        OPT_BYTES=$(stat -c%s "$OPT_FILE" 2>/dev/null || stat -f%z "$OPT_FILE")
        OPT_KB=$(( OPT_BYTES / 1024 ))
        GZIP_BYTES=$(gzip -9 -c "$OPT_FILE" | wc -c)
        GZIP_KB=$(( GZIP_BYTES / 1024 ))
    fi

    printf "%-12s  %8s  %8s  %8s\n" "$ALLOC_NAME" "$RAW_KB" "$OPT_KB" "$GZIP_KB"
done

echo ""
echo "=== Comparison Complete ==="
