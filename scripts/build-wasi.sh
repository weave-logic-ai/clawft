#!/usr/bin/env bash
# Build WeftOS crates for the wasm32-wasip2 target.
#
# Usage:
#   scripts/build-wasi.sh              # release-wasm profile (default)
#   scripts/build-wasi.sh --debug      # dev profile (faster iteration)
#   scripts/build-wasi.sh --optimize   # release-wasm + wasm-opt
#
# The wasm32-wasip2 target is HP-16's canonical WASM target. cargo-dist v0.31.0
# does not support it in its target matrix, so this script (and the companion
# release-wasi.yml workflow) handle the build independently.
#
# What compiles for wasip2 (--no-default-features):
#   clawft-types, clawft-platform, clawft-plugin, clawft-llm,
#   clawft-security, exo-resource-tree, clawft-core, clawft-kernel,
#   weftos (lib), clawft-wasm (cdylib)
#
# The highest-level library crate is weftos (the full kernel facade).
# The highest-level binary artefact is clawft-wasm (cdylib .wasm module).

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${CYAN}[wasi]${NC} %s\n" "$*"; }
ok()    { printf "${GREEN}[wasi]${NC} %s\n" "$*"; }
err()   { printf "${RED}[wasi]${NC} %s\n" "$*" >&2; }

# --- Resolve workspace root ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"

# --- Parse arguments ---
PROFILE="release-wasm"
PROFILE_DIR="release-wasm"
OPTIMIZE=false

for arg in "$@"; do
    case "$arg" in
        --debug)
            PROFILE="dev"
            PROFILE_DIR="debug"
            ;;
        --optimize)
            OPTIMIZE=true
            ;;
        --help|-h)
            echo "Usage: scripts/build-wasi.sh [--debug] [--optimize]"
            echo ""
            echo "  --debug      Use dev profile (faster, larger binary)"
            echo "  --optimize   Run wasm-opt after build (requires binaryen)"
            exit 0
            ;;
        *)
            err "Unknown argument: $arg"
            exit 1
            ;;
    esac
done

TARGET="wasm32-wasip2"
WASM_FILE="target/${TARGET}/${PROFILE_DIR}/clawft_wasm.wasm"

# --- Ensure target is installed ---
info "Ensuring ${TARGET} target is available..."
rustup target add "$TARGET" 2>/dev/null || true

# --- Build weftos (full kernel facade, lib only) ---
if [ "$PROFILE" = "dev" ]; then
    info "Building weftos (lib) for ${TARGET} (dev profile)..."
    cargo build --target "$TARGET" -p weftos --no-default-features --lib
else
    info "Building weftos (lib) for ${TARGET} (release-wasm profile)..."
    cargo build --target "$TARGET" -p weftos --no-default-features --lib --profile release-wasm
fi
ok "weftos lib compiled for ${TARGET}"

# --- Build clawft-wasm (cdylib binary artefact) ---
if [ "$PROFILE" = "dev" ]; then
    info "Building clawft-wasm for ${TARGET} (dev profile)..."
    cargo build --target "$TARGET" -p clawft-wasm --no-default-features
else
    info "Building clawft-wasm for ${TARGET} (release-wasm profile)..."
    cargo build --target "$TARGET" -p clawft-wasm --no-default-features --profile release-wasm
fi

# --- Verify output ---
if [ ! -f "$WASM_FILE" ]; then
    err "Expected WASM binary not found: $WASM_FILE"
    exit 1
fi

SIZE_BYTES=$(wc -c < "$WASM_FILE")
SIZE_KB=$(echo "scale=1; $SIZE_BYTES / 1024" | bc 2>/dev/null || echo "$((SIZE_BYTES / 1024))")
ok "Built: $WASM_FILE (${SIZE_KB} KB)"

# --- Optional wasm-opt ---
if [ "$OPTIMIZE" = true ]; then
    if [ -f "$WORKSPACE_ROOT/scripts/build/wasm-opt.sh" ]; then
        info "Running wasm-opt..."
        bash "$WORKSPACE_ROOT/scripts/build/wasm-opt.sh" "$WASM_FILE"
    else
        err "wasm-opt.sh not found; skipping optimization"
    fi
fi

# --- Summary ---
ls -la "$WASM_FILE"
GZIP_SIZE=$(gzip -9 -c "$WASM_FILE" | wc -c)
GZIP_KB=$(echo "scale=1; $GZIP_SIZE / 1024" | bc 2>/dev/null || echo "$((GZIP_SIZE / 1024))")
info "Gzipped: ${GZIP_KB} KB"
