#!/usr/bin/env bash
# Runs wasm-opt -Oz on the clawft WASM binary.
#
# Usage: scripts/build/wasm-opt.sh [input.wasm] [output.wasm]
#
# Defaults:
#   input:  target/wasm32-wasip2/release-wasm/clawft_wasm.wasm
#   output: <input>.opt.wasm (sibling of input)
#
# For wasip1 core modules: runs wasm-opt -Oz directly (typically 15-30% reduction).
# For wasip2 component model binaries: extracts core modules with wasm-tools,
#   optimizes them, and reports size reduction. The output is the optimized
#   core module (not a full component) since reliable recomposition is not
#   yet possible with current tooling.
#
# Requires: wasm-opt (from binaryen)
#   Install: cargo install wasm-opt
#       or:  apt-get install binaryen
#       or:  brew install binaryen
#       or:  download from https://github.com/WebAssembly/binaryen/releases
#
# Optional: wasm-tools (for wasip2 component model support)
#   Install: cargo install wasm-tools

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()    { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
err()   { printf "${RED}[ERROR]${NC} %s\n" "$*" >&2; }

# --- Resolve workspace root ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# --- Parse arguments ---
INPUT="${1:-$WORKSPACE_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"

# If input doesn't exist at primary location, try wasip1 fallback
if [ ! -f "$INPUT" ] && [ "$INPUT" = "$WORKSPACE_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm" ]; then
    FALLBACK="$WORKSPACE_ROOT/target/wasm32-wasip1/release-wasm/clawft_wasm.wasm"
    if [ -f "$FALLBACK" ]; then
        warn "Primary target not found, falling back to wasip1: $FALLBACK"
        INPUT="$FALLBACK"
    fi
fi

# Derive output path: same directory, .opt.wasm suffix
INPUT_DIR="$(dirname "$INPUT")"
INPUT_BASE="$(basename "$INPUT" .wasm)"
OUTPUT="${2:-$INPUT_DIR/${INPUT_BASE}.opt.wasm}"

# --- Validate input ---
if [ ! -f "$INPUT" ]; then
    err "Input WASM file not found: $INPUT"
    err ""
    err "Build the WASM binary first:"
    err "  cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm"
    exit 1
fi

# --- Check wasm-opt is available ---
if ! command -v wasm-opt >/dev/null 2>&1; then
    err "wasm-opt is not installed."
    err ""
    err "Install binaryen (which provides wasm-opt) via one of:"
    err "  cargo install wasm-opt"
    err "  apt-get install binaryen"
    err "  brew install binaryen"
    err "  https://github.com/WebAssembly/binaryen/releases"
    exit 1
fi

# --- Detect WASM binary type ---
# Core module magic: \0asm\1\0\0\0 (version 1)
# Component model magic: \0asm\r\0\1\0 (version 0d/13)
detect_wasm_type() {
    local file="$1"
    local header
    header=$(od -A n -t x1 -N 8 "$file" | tr -d ' ')
    case "$header" in
        0061736d01000000) echo "core" ;;
        0061736d0d000100) echo "component" ;;
        *) echo "unknown" ;;
    esac
}

WASM_TYPE=$(detect_wasm_type "$INPUT")

# --- Measure input size ---
INPUT_SIZE=$(wc -c < "$INPUT")
INPUT_KB=$(echo "scale=1; $INPUT_SIZE / 1024" | bc)
info "Input:  $INPUT ($INPUT_KB KB / $INPUT_SIZE bytes)"
info "Binary type: $WASM_TYPE"

# --- Run wasm-opt based on binary type ---
OPTIMIZED=false

if [ "$WASM_TYPE" = "core" ]; then
    # Standard core module -- optimize directly
    info "Running: wasm-opt -Oz --enable-bulk-memory --enable-sign-ext"
    wasm-opt -Oz \
        --enable-bulk-memory \
        --enable-sign-ext \
        -o "$OUTPUT" \
        "$INPUT"
    OPTIMIZED=true

elif [ "$WASM_TYPE" = "component" ]; then
    info "Detected component model binary (wasip2)"

    if command -v wasm-tools >/dev/null 2>&1; then
        # Extract core modules from the component, optimize them, report savings
        TMPDIR_OPT=$(mktemp -d)
        trap "rm -rf '$TMPDIR_OPT'" EXIT

        info "Extracting core modules with wasm-tools component unbundle..."
        if wasm-tools component unbundle \
            --module-dir "$TMPDIR_OPT" \
            -o "$TMPDIR_OPT/component-unbundled.wasm" \
            "$INPUT" 2>/dev/null; then

            # Find all unbundled core modules
            CORE_MODULES=()
            while IFS= read -r -d '' f; do
                CORE_MODULES+=("$f")
            done < <(find "$TMPDIR_OPT" -name "unbundled-module*.wasm" -print0 | sort -z)

            if [ "${#CORE_MODULES[@]}" -gt 0 ]; then
                TOTAL_BEFORE=0
                TOTAL_AFTER=0

                for MODULE in "${CORE_MODULES[@]}"; do
                    MODULE_NAME=$(basename "$MODULE")
                    MODULE_SIZE=$(wc -c < "$MODULE")
                    TOTAL_BEFORE=$((TOTAL_BEFORE + MODULE_SIZE))

                    OPT_MODULE="${MODULE%.wasm}.opt.wasm"
                    if wasm-opt -Oz --enable-bulk-memory --enable-sign-ext \
                        -o "$OPT_MODULE" "$MODULE" 2>/dev/null; then
                        OPT_SIZE=$(wc -c < "$OPT_MODULE")
                        TOTAL_AFTER=$((TOTAL_AFTER + OPT_SIZE))
                        MODULE_KB=$(echo "scale=1; $MODULE_SIZE / 1024" | bc)
                        OPT_KB=$(echo "scale=1; $OPT_SIZE / 1024" | bc)
                        info "  $MODULE_NAME: $MODULE_KB KB -> $OPT_KB KB"
                    else
                        TOTAL_AFTER=$((TOTAL_AFTER + MODULE_SIZE))
                        warn "  $MODULE_NAME: wasm-opt failed, keeping original"
                    fi
                done

                CORE_REDUCTION=$(echo "scale=1; (1 - $TOTAL_AFTER / $TOTAL_BEFORE) * 100" | bc)
                ok "Core module optimization: ${CORE_REDUCTION}% reduction"
                TOTAL_BEFORE_KB=$(echo "scale=1; $TOTAL_BEFORE / 1024" | bc)
                TOTAL_AFTER_KB=$(echo "scale=1; $TOTAL_AFTER / 1024" | bc)
                info "Core modules: $TOTAL_BEFORE_KB KB -> $TOTAL_AFTER_KB KB"

                # Estimate what the full component would be with optimized core
                COMPONENT_OVERHEAD=$((INPUT_SIZE - TOTAL_BEFORE))
                ESTIMATED_OPT=$((COMPONENT_OVERHEAD + TOTAL_AFTER))
                ESTIMATED_KB=$(echo "scale=1; $ESTIMATED_OPT / 1024" | bc)
                info "Estimated optimized component size: $ESTIMATED_KB KB"

                # Output: copy the first optimized core module as representative
                # The size gate should run on the original component since we
                # cannot reliably recompose it yet
                if [ "${#CORE_MODULES[@]}" -eq 1 ]; then
                    OPT_FIRST="${CORE_MODULES[0]%.wasm}.opt.wasm"
                    if [ -f "$OPT_FIRST" ]; then
                        cp "$OPT_FIRST" "$OUTPUT"
                        OPTIMIZED=true
                        warn "Output is the optimized core module, not a full component"
                        warn "The size gate should run on the original component binary"
                    fi
                fi
            else
                warn "No core modules extracted from component"
            fi
        else
            warn "wasm-tools component unbundle failed for this binary"
        fi

        # If we couldn't produce optimized output, copy original
        if [ "$OPTIMIZED" = false ]; then
            cp "$INPUT" "$OUTPUT"
            warn "Could not optimize; output is a copy of input"
        fi
    else
        warn "Component model binary detected but wasm-tools is not installed"
        warn "wasm-opt does not support WASM component model binaries directly"
        warn "Install wasm-tools for component decomposition: cargo install wasm-tools"
        cp "$INPUT" "$OUTPUT"
    fi
else
    warn "Unknown WASM binary type"
    if wasm-opt -Oz -o "$OUTPUT" "$INPUT" 2>/dev/null; then
        OPTIMIZED=true
        ok "Optimization succeeded despite unknown type"
    else
        err "wasm-opt failed on unknown binary type"
        cp "$INPUT" "$OUTPUT"
    fi
fi

# --- Measure output size ---
OUTPUT_SIZE=$(wc -c < "$OUTPUT")
OUTPUT_KB=$(echo "scale=1; $OUTPUT_SIZE / 1024" | bc)

# --- Calculate reduction ---
if [ "$INPUT_SIZE" -gt 0 ]; then
    REDUCTION_PCT=$(echo "scale=1; (1 - $OUTPUT_SIZE / $INPUT_SIZE) * 100" | bc)
else
    REDUCTION_PCT="0"
fi

info "Output: $OUTPUT ($OUTPUT_KB KB / $OUTPUT_SIZE bytes)"
if [ "$OPTIMIZED" = true ]; then
    ok "Size reduction: ${REDUCTION_PCT}% ($INPUT_KB KB -> $OUTPUT_KB KB)"
else
    info "No optimization applied (0% reduction)"
fi

# --- Gzipped size ---
GZIP_SIZE=$(gzip -9 -c "$OUTPUT" | wc -c)
GZIP_KB=$(echo "scale=1; $GZIP_SIZE / 1024" | bc)
info "Gzipped (level 9): $GZIP_KB KB ($GZIP_SIZE bytes)"

# --- Optional: validate with wasmtime ---
if command -v wasmtime >/dev/null 2>&1; then
    info "Validating optimized WASM with wasmtime compile..."
    if wasmtime compile "$OUTPUT" -o /dev/null 2>/dev/null; then
        ok "wasmtime validation passed"
    else
        warn "wasmtime compile validation failed (may be expected for extracted core modules)"
    fi
else
    info "wasmtime not found; skipping validation (install wasmtime to enable)"
fi

# --- Summary ---
printf "\n"
echo "=== wasm-opt Summary ==="
echo "Type:      $WASM_TYPE"
echo "Input:     $INPUT_KB KB"
echo "Optimized: $OUTPUT_KB KB (${REDUCTION_PCT}% smaller)"
echo "Gzipped:   $GZIP_KB KB"
echo "Output:    $OUTPUT"
