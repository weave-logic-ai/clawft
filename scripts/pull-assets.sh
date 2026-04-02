#!/usr/bin/env bash
# Pull CDN assets (WASM + KB) into docs/src/public/ for local development.
#
# Usage:
#   scripts/pull-assets.sh              # pull from GitHub Releases (cdn-assets tag)
#   scripts/pull-assets.sh --local      # copy from local build artifacts
#
# The docs site .gitignore excludes public/wasm/ and public/kb/.
# In production these are served from NEXT_PUBLIC_CDN_URL.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

WASM_DIR="$ROOT/docs/src/public/wasm"
KB_DIR="$ROOT/docs/src/public/kb"

mkdir -p "$WASM_DIR" "$KB_DIR"

if [[ "${1:-}" == "--local" ]]; then
    echo "Copying from local build artifacts..."

    # WASM from crates/clawft-wasm/pkg/
    PKG_DIR="$ROOT/crates/clawft-wasm/pkg"
    if [[ -f "$PKG_DIR/clawft_wasm_bg.wasm" ]]; then
        cp "$PKG_DIR/clawft_wasm_bg.wasm" "$WASM_DIR/"
        cp "$PKG_DIR/clawft_wasm.js" "$WASM_DIR/"
        echo "  WASM: copied from $PKG_DIR"
    else
        echo "  WASM: not found at $PKG_DIR — run browser WASM build first"
        exit 1
    fi

    # KB from build-kb output or worktree
    if [[ -f "$ROOT/docs/src/public/kb/weftos-docs.rvf" ]]; then
        echo "  KB: already exists"
    else
        echo "  KB: running build-kb..."
        "$ROOT/scripts/build-kb.sh"
    fi
else
    REPO="${GITHUB_REPOSITORY:-weave-logic-ai/weftos}"
    TAG="cdn-assets"
    BASE="https://github.com/$REPO/releases/download/$TAG"

    echo "Pulling assets from GitHub Releases ($REPO@$TAG)..."

    echo "  Downloading clawft_wasm_bg.wasm..."
    curl -fsSL "$BASE/clawft_wasm_bg.wasm" -o "$WASM_DIR/clawft_wasm_bg.wasm"

    echo "  Downloading clawft_wasm.js..."
    curl -fsSL "$BASE/clawft_wasm.js" -o "$WASM_DIR/clawft_wasm.js"

    echo "  Downloading weftos-docs.rvf..."
    curl -fsSL "$BASE/weftos-docs.rvf" -o "$KB_DIR/weftos-docs.rvf"
fi

echo ""
echo "Assets ready:"
ls -lh "$WASM_DIR/"
ls -lh "$KB_DIR/"
