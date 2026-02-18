#!/usr/bin/env bash
# Profiles WASM binary with twiggy top, dominators, and monos.
#
# Usage: scripts/bench/wasm-twiggy.sh [wasm-file]
#
# Default: target/wasm32-wasip2/release-wasm/clawft_wasm.wasm
#
# For wasip2 component model binaries, this script extracts the core module
# using wasm-tools before profiling (twiggy only supports core modules).
#
# Output: Reports written to target/wasm-profile/
#
# Requires: twiggy
#   Install: cargo install twiggy
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

# --- Detect WASM binary type ---
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

# --- Resolve workspace root ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# --- Parse arguments ---
WASM_FILE="${1:-$WORKSPACE_ROOT/target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"

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
    err "WASM file not found: $WASM_FILE"
    err ""
    err "Build the WASM binary first:"
    err "  cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm"
    exit 1
fi

# --- Check twiggy is available ---
if ! command -v twiggy >/dev/null 2>&1; then
    err "twiggy is not installed."
    err ""
    err "Install with: cargo install twiggy"
    exit 1
fi

# --- Create output directory ---
PROFILE_DIR="$WORKSPACE_ROOT/target/wasm-profile"
mkdir -p "$PROFILE_DIR"

WASM_BASENAME="$(basename "$WASM_FILE" .wasm)"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

info "Profiling: $WASM_FILE"
info "Output:    $PROFILE_DIR/"

# --- Handle component model binaries ---
# twiggy only supports core WASM modules, not component model binaries.
# For wasip2 components, extract the core module first.
WASM_TYPE=$(detect_wasm_type "$WASM_FILE")
PROFILE_TARGET="$WASM_FILE"
CLEANUP_TMP=""

if [ "$WASM_TYPE" = "component" ]; then
    info "Binary type: component model (wasip2)"

    if command -v wasm-tools >/dev/null 2>&1; then
        TMPDIR_PROFILE=$(mktemp -d)
        CLEANUP_TMP="$TMPDIR_PROFILE"

        info "Extracting core module with wasm-tools for twiggy analysis..."
        if wasm-tools component unbundle \
            --module-dir "$TMPDIR_PROFILE" \
            -o "$TMPDIR_PROFILE/component-unbundled.wasm" \
            "$WASM_FILE" 2>/dev/null; then

            # Use the largest core module (the main code)
            CORE_MODULE=$(find "$TMPDIR_PROFILE" -name "unbundled-module*.wasm" -type f \
                -exec wc -c {} + 2>/dev/null | sort -rn | head -1 | awk '{print $2}')

            if [ -n "$CORE_MODULE" ] && [ -f "$CORE_MODULE" ]; then
                CORE_SIZE=$(wc -c < "$CORE_MODULE")
                CORE_KB=$(echo "scale=1; $CORE_SIZE / 1024" | bc)
                info "Extracted core module: $CORE_KB KB"
                PROFILE_TARGET="$CORE_MODULE"
                WASM_BASENAME="${WASM_BASENAME}-core"
            else
                warn "No core modules found; profiling original binary (may fail)"
            fi
        else
            warn "wasm-tools component unbundle failed; profiling original binary (may fail)"
        fi
    else
        warn "Component model binary detected but wasm-tools not installed"
        warn "twiggy may fail on component model binaries"
        warn "Install wasm-tools: cargo install wasm-tools"
    fi
elif [ "$WASM_TYPE" = "core" ]; then
    info "Binary type: core module"
else
    warn "Unknown binary type; twiggy may fail"
fi

printf "\n"

# --- Run twiggy top ---
TOP_FILE="$PROFILE_DIR/${WASM_BASENAME}-top-${TIMESTAMP}.txt"
info "Running: twiggy top (top 20 size contributors)..."
if twiggy top "$PROFILE_TARGET" -n 20 2>/dev/null | tee "$TOP_FILE"; then
    ok "Saved: $TOP_FILE"
else
    warn "twiggy top failed (binary format may not be supported)"
    echo "(twiggy top failed)" > "$TOP_FILE"
fi
printf "\n"

# --- Run twiggy dominators ---
DOM_FILE="$PROFILE_DIR/${WASM_BASENAME}-dominators-${TIMESTAMP}.txt"
info "Running: twiggy dominators..."
if twiggy dominators "$PROFILE_TARGET" > "$DOM_FILE" 2>/dev/null; then
    ok "Saved: $DOM_FILE"
else
    warn "twiggy dominators failed"
    echo "(twiggy dominators failed)" > "$DOM_FILE"
fi

# --- Run twiggy monos ---
MONOS_FILE="$PROFILE_DIR/${WASM_BASENAME}-monos-${TIMESTAMP}.txt"
info "Running: twiggy monos (monomorphization bloat)..."
if twiggy monos "$PROFILE_TARGET" > "$MONOS_FILE" 2>/dev/null; then
    ok "Saved: $MONOS_FILE"
else
    warn "twiggy monos failed"
    echo "(twiggy monos failed)" > "$MONOS_FILE"
fi

# --- Cleanup temp directory ---
if [ -n "$CLEANUP_TMP" ] && [ -d "$CLEANUP_TMP" ]; then
    rm -rf "$CLEANUP_TMP"
fi

# --- Summary ---
printf "\n"
echo "=== Twiggy Profile Summary ==="
echo "WASM file: $WASM_FILE"

SIZE_BYTES=$(wc -c < "$WASM_FILE")
SIZE_KB=$(echo "scale=1; $SIZE_BYTES / 1024" | bc)
echo "Total size: $SIZE_KB KB ($SIZE_BYTES bytes)"

if [ "$WASM_TYPE" = "component" ]; then
    echo "Note: profiled the extracted core module, not the full component"
fi

printf "\n"
echo "--- Top 5 Size Contributors ---"
head -8 "$TOP_FILE"

printf "\n"
echo "--- Reports ---"
echo "  Top:        $TOP_FILE"
echo "  Dominators: $DOM_FILE"
echo "  Monos:      $MONOS_FILE"

printf "\n"
ok "Twiggy profiling complete."
