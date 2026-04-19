#!/usr/bin/env bash
# Build the clawft-gui-egui crate for wasm32-unknown-unknown and drop the
# wasm-bindgen bundle under `webview/wasm/` so the VSCode / Cursor
# extension's webview can load it.
#
# Artifacts are .gitignore'd; run this before packaging the extension.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../../.." && pwd)"
out_dir="$here/../webview/wasm"

cd "$repo_root"

command -v wasm-pack >/dev/null || {
    echo "wasm-pack not on PATH. Install with:"
    echo "  curl https://rustup.rs -sSf | sh"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
}

echo "→ Building clawft-gui-egui → wasm32-unknown-unknown → $out_dir"
wasm-pack build crates/clawft-gui-egui \
    --target web \
    --out-dir "$out_dir" \
    --no-typescript \
    -- --no-default-features

echo "✓ Wasm bundle at $out_dir"
ls -l "$out_dir"
