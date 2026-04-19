# M0 smoke test

Everything here should be doable in under five minutes once the extension
is compiled and installed. If a step below fails, M0 is not green.

## Prereqs

- `npm install && npm run compile` has been run in this directory.
- Extension has been installed into VSCode or Cursor (see `README.md`).

## Steps

1. **Start the daemon.**
   ```bash
   cargo run -p clawft-weave --bin weaver -- kernel start
   ```
   Leave it running.

2. **Open the workspace** in VSCode or Cursor with this repo as the root.

3. **Install the extension** if you have not already:
   ```bash
   code --install-extension extensions/vscode-weft-panel/vscode-weft-panel-0.0.1.vsix
   # or in Cursor:
   cursor --install-extension extensions/vscode-weft-panel/vscode-weft-panel-0.0.1.vsix
   ```
   Alternatively: palette -> "Developer: Install Extension from Location..."
   and pick `extensions/vscode-weft-panel/`.

4. **Open the panel.**
   Palette -> **WeftOS: Open Panel**. A dark panel opens in the editor
   column. The status line shows the resolved socket path.

5. **Fetch kernel.status.** Click the button. The response pane shows
   the JSON result (kernel state, uptime, process count, etc.). The
   radar log at the bottom gains one entry tagged `ok`.

6. **Fetch kernel.ps.** Click the other button. The response pane shows
   the JSON process list (possibly empty). The radar log gains a second
   entry.

7. **Reload the window.** Palette -> "Developer: Reload Window". The
   panel reappears because the serializer was registered. The response
   pane and radar log are empty (acceptable for M0; state rehydration
   is M1). The status line still shows the socket path.

8. **Kill the daemon.** Stop the `weaver kernel start` process.
   Click either button again. The response pane now shows the RPC
   error (`ENOENT` / `ECONNREFUSED` plus the "start with
   `weaver kernel start`" hint). The extension does not crash; the
   panel is still usable once the daemon returns.

If all eight steps pass, M0 is green.
