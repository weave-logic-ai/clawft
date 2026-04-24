#!/usr/bin/env bash
# Build the clawft-gui-egui crate for wasm32-unknown-unknown and drop the
# wasm-bindgen bundle under `webview/wasm/` so the VSCode / Cursor
# extension's webview can load it.
#
# Artifacts are .gitignore'd; run this before packaging the extension.
#
# Prefers `wasm-pack` when available and healthy. Falls back to `cargo
# build --target wasm32-unknown-unknown` + a separately-installed
# `wasm-bindgen` CLI — covers environments where wasm-pack's bundled
# bindgen downloader is broken (observed: "invalid type: map, expected a
# string" when parsing the GitHub releases manifest).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../../.." && pwd)"
out_dir="$here/../webview/wasm"

cd "$repo_root"
mkdir -p "$out_dir"

# ── Preferred path: wasm-pack ────────────────────────────────────────
if command -v wasm-pack >/dev/null; then
    echo "→ Trying wasm-pack build clawft-gui-egui → $out_dir"
    if wasm-pack build crates/clawft-gui-egui \
            --target web \
            --out-dir "$out_dir" \
            --no-typescript \
            -- --no-default-features; then
        echo "✓ Wasm bundle at $out_dir"
        ls -l "$out_dir"
        exit 0
    fi
    echo "! wasm-pack failed — falling back to cargo + wasm-bindgen"
fi

# ── Fallback: cargo build + wasm-bindgen ─────────────────────────────
command -v wasm-bindgen >/dev/null || {
    echo "Neither wasm-pack (working) nor wasm-bindgen are available."
    echo "Install either:"
    echo "  cargo install wasm-bindgen-cli"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
}

echo "→ cargo build --target wasm32-unknown-unknown --release"
cargo build \
    -p clawft-gui-egui \
    --target wasm32-unknown-unknown \
    --release \
    --no-default-features

wasm_in="target/wasm32-unknown-unknown/release/clawft_gui_egui.wasm"
[ -f "$wasm_in" ] || {
    echo "wasm artifact missing at $wasm_in"
    exit 1
}

echo "→ wasm-bindgen → $out_dir"
wasm-bindgen \
    --target web \
    --out-dir "$out_dir" \
    --no-typescript \
    "$wasm_in"

echo "✓ Wasm bundle at $out_dir"
ls -l "$out_dir"
