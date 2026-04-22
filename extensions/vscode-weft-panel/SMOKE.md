# Smoke test — WeftOS VSCode / Cursor panel (M1)

End-to-end path: **daemon ↔ VSCode extension ↔ webview ↔ egui-wasm surface**.

## 1. Build the wasm bundle

```bash
# from repo root
extensions/vscode-weft-panel/scripts/build-wasm.sh
```

Prerequisites: `rustup target add wasm32-unknown-unknown`,
`cargo install wasm-pack` (one-time). Expected output:

```
✓ Wasm bundle at extensions/vscode-weft-panel/webview/wasm
… clawft_gui_egui.js  (~80 KB)
… clawft_gui_egui_bg.wasm  (~4.2 MB unoptimized)
```

## 2. Compile the extension

```bash
cd extensions/vscode-weft-panel
npm install      # first time only
npm run compile  # tsc -p .
```

## 3. Start the daemon

```bash
cargo run -p clawft-weave --bin weaver -- kernel start
```

## 4. Install + open the panel

**In VSCode / Cursor**:

- Palette → `Developer: Install Extension from Location…`
- Point at `extensions/vscode-weft-panel/`.
- Palette → `WeftOS: Open Panel`.

You should see:

1. Loading screen: `loading egui shell…` on black for ≤1s.
2. The **WeftOS boot splash** (gold mark on black, halo, `weave the machine`) fading in and out over ~4s.
3. The **desktop shell** — warped-grid wallpaper + tray + kernel pill — *inside the Cursor editor pane*.
4. Within ~1s the sidebar pill flips to green: `connected`. The Status block shows live `kernel.status` values (uptime, processes, services, Poll #N · Nms ago, Poll RTT).

If the wasm bundle is missing (step 1 skipped), the panel shows a fallback card with the build-script path and the expected webview URI.

## 5. Reload survival

- `Developer: Reload Window` — the panel reappears (ready-blank); click around to re-fetch.

## 6. Daemon-offline behaviour

- Kill the daemon (`^C` on the `weaver kernel start` shell).
- Within ~1s the sidebar pill turns red (`offline`); block data holds its last snapshot plus `last_error`.
- Restart the daemon — next tick turns green again, no reload needed.

## 7. Allowed RPC surface

The extension allowlists four methods for the panel:

- `kernel.status`
- `kernel.ps`
- `kernel.services`
- `kernel.logs`

Any other method request from the webview is rejected with
`method not allowed: <method>`. Extending the allowlist happens in
`extensions/vscode-weft-panel/src/extension.ts` (`ALLOWED_METHODS`).

## Known gaps (deferred to M2 / M3)

- No voice input — VSCode webviews can't expose `allow="microphone"`
  yet (microsoft/vscode#303293). Capture sidecar lands next.
- No typed active-radar return schema — webview currently posts
  plain RPC-request / RPC-response messages only, no `variant-id`
  yet.
- No `ThreadDock` primitive for per-agent parallel output.
- Panel does not yet speak WSP-0.1 verbs (protocol-spec.md); raw
  kernel.* RPC only. WSP verbs land in M3.

References:
- Architecture & rationale: ADR-011, session-7 findings.
- Protocol: `.planning/symposiums/compositional-ui/protocol-spec.md`.
