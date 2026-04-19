# vscode-weft-panel (M0)

Smallest thing that proves the loop `daemon <-> extension <-> webview`
end to end. Background and design rationale live in
`.planning/symposiums/compositional-ui/session-7-dev-panel-embedding.md`;
predicate framing in `.planning/symposiums/compositional-ui/foundations.md`.

This is an M0 scaffold. It deliberately stops short of egui-wasm (M1),
voice/capture sidecar (M2), and the substrate topic layer (WSP-0.1).

## What it does

- Registers the command `WeftOS: Open Panel` (`weft.openPanel`).
- Opens a `WebviewPanel` in the editor area with `retainContextWhenHidden`.
- Implements `WebviewPanelSerializer` so the shell survives window reloads.
- Proxies `kernel.status` and `kernel.ps` from the webview to the daemon
  over the Unix socket described in `clawft_rpc::protocol::socket_path()`.
- Records each click + return in a small on-screen log — a placeholder
  for the active-radar return channel in WSP.

## Build

```bash
cd extensions/vscode-weft-panel
npm install
npm run compile      # tsc -p .
# optional: npm run watch
```

## Install (VSCode)

```bash
npm install -g @vscode/vsce
npm run package      # produces vscode-weft-panel-0.0.1.vsix
code --install-extension vscode-weft-panel-0.0.1.vsix
```

Or, for unpacked iteration: in VSCode run
`Developer: Install Extension from Location...` and point it at
this directory.

## Install (Cursor)

Cursor is a VSCode fork and accepts the same `.vsix`:

```bash
cursor --install-extension vscode-weft-panel-0.0.1.vsix
```

Or `Developer: Install Extension from Location...` in the Cursor
command palette.

## Use

1. Start the daemon: `cargo run -p clawft-weave --bin weaver -- kernel start`.
2. Open the palette and run **WeftOS: Open Panel**.
3. Click **Fetch kernel.status** or **Fetch kernel.ps** — the JSON
   response appears in the response pane; the radar log records the
   round-trip with a latency.
4. If the daemon is not running, the response pane shows the error
   (`ENOENT` or `ECONNREFUSED` plus the "start with `weaver kernel start`"
   hint); the extension does not crash.

See `SMOKE.md` for the full smoke test.
