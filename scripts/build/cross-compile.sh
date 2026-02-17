#!/usr/bin/env bash
# Cross-compilation wrapper for clawft/weft binary.
# Usage: ./scripts/build/cross-compile.sh <target> [--use-cross]
#
# Examples:
#   ./scripts/build/cross-compile.sh x86_64-unknown-linux-musl --use-cross
#   ./scripts/build/cross-compile.sh aarch64-apple-darwin
#   ./scripts/build/cross-compile.sh x86_64-pc-windows-msvc
#   ./scripts/build/cross-compile.sh wasm32-wasip1

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# --- Helpers ---
info()  { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()    { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
err()   { printf "${RED}[ERROR]${NC} %s\n" "$*" >&2; }

usage() {
    cat <<'EOF'
Cross-compilation wrapper for clawft/weft binary.

Usage: cross-compile.sh <target> [--use-cross]

Arguments:
  <target>       Rust target triple (required)
                 e.g. x86_64-unknown-linux-musl, aarch64-apple-darwin

Options:
  --use-cross    Use `cross` instead of `cargo` for compilation.
                 Recommended for Linux musl targets on non-Linux hosts.
  --help         Show this help message.

Examples:
  cross-compile.sh x86_64-unknown-linux-musl --use-cross
  cross-compile.sh aarch64-apple-darwin
  cross-compile.sh x86_64-pc-windows-msvc
  cross-compile.sh wasm32-wasip1
EOF
    exit 0
}

# --- Parse arguments ---
TARGET=""
USE_CROSS=false

for arg in "$@"; do
    case "$arg" in
        --use-cross) USE_CROSS=true ;;
        --help|-h)   usage ;;
        -*)          err "Unknown option: $arg"; usage ;;
        *)
            if [ -z "$TARGET" ]; then
                TARGET="$arg"
            else
                err "Unexpected argument: $arg"
                exit 1
            fi
            ;;
    esac
done

if [ -z "$TARGET" ]; then
    err "Target triple is required."
    printf "\n"
    usage
fi

# --- Resolve workspace root ---
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

info "Workspace root: $WORKSPACE_ROOT"
info "Target:         $TARGET"

# --- Determine build tool ---
if [ "$USE_CROSS" = true ]; then
    if ! command -v cross >/dev/null 2>&1; then
        err "'cross' is not installed. Install with: cargo install cross"
        exit 1
    fi
    BUILD_CMD="cross"
else
    BUILD_CMD="cargo"
fi

info "Build tool:     $BUILD_CMD"

# --- Determine crate and binary based on target ---
IS_WASM=false
case "$TARGET" in
    wasm32-*)
        IS_WASM=true
        CRATE="clawft-wasm"
        BINARY_NAME="clawft_wasm.wasm"
        info "WASM target detected, building crate: $CRATE"
        ;;
    *)
        CRATE="clawft-cli"
        BINARY_NAME="weft"
        ;;
esac

# Windows targets produce .exe binaries
case "$TARGET" in
    *-windows-*)
        BINARY_NAME="weft.exe"
        ;;
esac

# --- Build ---
BUILD_ARGS=(
    --release
    --target "$TARGET"
    -p "$CRATE"
)

# WASM targets use the release-wasm profile if available
if [ "$IS_WASM" = true ]; then
    BUILD_ARGS=(
        --profile release-wasm
        --target "$TARGET"
        -p "$CRATE"
    )
fi

info "Running: $BUILD_CMD build ${BUILD_ARGS[*]}"

(cd "$WORKSPACE_ROOT" && "$BUILD_CMD" build "${BUILD_ARGS[@]}")

# --- Locate output binary ---
if [ "$IS_WASM" = true ]; then
    # WASM output path
    BINARY_PATH="$WORKSPACE_ROOT/target/$TARGET/release-wasm/$BINARY_NAME"
    # Also check the wasm32-wasip1 alternate output layout
    if [ ! -f "$BINARY_PATH" ]; then
        BINARY_PATH="$WORKSPACE_ROOT/target/$TARGET/release-wasm/clawft-wasm.wasm"
    fi
    if [ ! -f "$BINARY_PATH" ]; then
        # Try the deps directory for wasm artifacts
        BINARY_PATH=$(find "$WORKSPACE_ROOT/target/$TARGET/release-wasm" -name "*.wasm" -type f 2>/dev/null | head -1 || true)
    fi
else
    BINARY_PATH="$WORKSPACE_ROOT/target/$TARGET/release/$BINARY_NAME"
fi

if [ ! -f "$BINARY_PATH" ]; then
    err "Build succeeded but binary not found at expected path."
    err "Expected: $BINARY_PATH"
    err "Listing target directory:"
    ls -la "$WORKSPACE_ROOT/target/$TARGET/release"* 2>/dev/null || true
    exit 1
fi

# --- Report size ---
SIZE_BYTES=$(stat -f%z "$BINARY_PATH" 2>/dev/null || stat -c%s "$BINARY_PATH" 2>/dev/null)
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)

ok "Build complete!"
printf "\n"
info "Binary: $BINARY_PATH"
info "Size:   ${SIZE_MB} MB (${SIZE_BYTES} bytes)"

# --- Run size check if available ---
SIZE_CHECK="$SCRIPT_DIR/size-check.sh"
if [ -f "$SIZE_CHECK" ] && [ "$IS_WASM" = false ]; then
    printf "\n"
    info "Running size check..."
    bash "$SIZE_CHECK" "$BINARY_PATH"
fi

printf "\n"
ok "Cross-compilation for $TARGET finished successfully."
