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
`method not allowed: <method>` (or `method denied for webview: …` for
entries on `WEBVIEW_DENIED_METHODS`, e.g. `substrate.publish` — WEFT-496 /
ADR-071). Extending the allowlist happens in
`extensions/vscode-weft-panel/src/allowlist.ts` (`STATIC_ALLOWED_METHODS`);
raw substrate writes go on `WEBVIEW_DENIED_METHODS`, never the allowlist.

## 8. Multi-user panel auth (WEFT-495 / ADR-071)

Default is **single-user** (0.7 posture): allowlist only, no panel token.

Enable multi-user with either:

```bash
export WEFTOS_MULTI_USER=1
# or VSCode setting: weft.multiUser = true
```

When multi-user is on, each panel gets a host-held session with default
scopes `{read, chat}`. Methods requiring `write` or `admin` are rejected
by identity before the UDS call (`permission denied: panel scopes …`).
The token is never sent into the webview.

Unit tests (no VSCode host):

```bash
cd extensions/vscode-weft-panel && npm run test:panel-auth
```

## 9. Chip-icon DOM assertion (WEFT-558)

Tray chips paint **inside the egui canvas** (wasm), so a raw
`webview.html` greper cannot see them. WEFT-558 closes that gap with a
DOM-side a11y / E2E mirror:

| Piece | Path |
|-------|------|
| Pure helpers | `src/chipA11y.ts` |
| Host inject + snapshot | command `weft._test.chipStripSnapshot` (not in palette) |
| E2E case | `test/suite/extension.test.ts` → `chip strip exposes >=1 chip element with stable id` |
| Unit tests | `npm run test:chip-a11y` |

Stable identifiers match tray `ChipId`s: `kernel`, `mesh`, `exochain`,
`explorer`. Each chip is a visually-hidden

```html
<span data-weft-chip data-chip-id="kernel" data-chip-tone="ok" data-chip-label="Kernel">Kernel</span>
```

inside `#weft-chip-a11y[data-weft-status-strip]`.

**How the E2E path works**

1. `runTest.ts` sets `WEFT_PANEL_E2E=1` in the extension host env so the
   initial HTML can seed the mock set.
2. The suite opens the panel, then calls `weft._test.chipStripSnapshot`,
   which re-injects `MOCK_E2E_CHIPS` into `panel.webview.html` and
   parses them back.
3. Asserts `chips.length >= 1` and a chip with `id === "kernel"`.

**Local runs**

```bash
cd extensions/vscode-weft-panel
npm run test:chip-a11y          # pure unit (no VSCode host)
npm run pretest && npm test     # full headless host (needs xvfb on Linux)
```

CI: `vscode-panel-build` job in `.github/workflows/pr-gates.yml`
(`xvfb-run -a npm test`).

Production panels leave the strip **empty** (container only) unless
`WEFT_PANEL_E2E=1`. The test command is host-only and never contributed
to the command palette.

## 10. WSP-0.1 verb surface (WEFT-285)

The extension host accepts **WSP-0.1 verbs** alongside raw RPC.

| Webview → host | Host → webview |
|----------------|----------------|
| `{ type: "wsp-request", id, method, params }` | `{ type: "wsp-response", id, method, ok, result?, error? }` |
| (auto on `ready`) | `{ type: "wsp-hello", result: server-caps }` |
| | `{ type: "wsp-notification", method, params }` |

Raw `{ type: "rpc-request", … }` is **unchanged** (backward compatible).

### Verb set (17 — ADR-005 hard cap)

`session.initialize`, `session.shutdown`, `surface.compose`,
`surface.get`, `surface.update`, `surface.dispose`, `subscribe`,
`unsubscribe`, `observe`, `observe.close`, `invoke`, `mutate`,
`gate.check`, `consent.request`, `consent.revoke`,
`ontology.describe`, `cancel`.

### Common panel ops → daemon RPC

| WSP path | Daemon RPC |
|----------|------------|
| `subscribe` `resource://kernel/status` | `kernel.status` |
| `subscribe` `resource://kernel/ps` | `kernel.ps` |
| `subscribe` `resource://kernel/services` | `kernel.services` |
| `subscribe` `resource://kernel/logs` | `kernel.logs` |
| `subscribe` `resource://cluster/*` | `cluster.status` / `cluster.nodes` |
| `subscribe` `resource://chain/*` | `chain.status` / `chain.tail` |
| `subscribe` `substrate://…` | `substrate.subscribe` (or `.read`) |
| `invoke` affordance `chat` | `agent.chat` |
| `invoke` affordance `chat_stream` | `agent.chat_stream` |
| `invoke` affordance `terminal_*` | `terminal.*` |
| `invoke` affordance `service_*` | `service.*` |
| `invoke` affordance `control_set_enabled` | `control.set_enabled` |

Full tables live in `src/wsp.ts` (`RESOURCE_RPC_MAP`,
`AFFORDANCE_RPC_MAP`, `verbMappingTable()`).

### Migrated surface

On webview `ready` the host:

1. Runs `session.initialize` (persona `persona://dev-panel`).
2. Posts `wsp-hello` with server-caps (verbs, resources, flags).
3. `subscribe`s `resource://kernel/status` → `kernel.status` and
   posts a `substrate.update` notification (`source: "wsp-bootstrap"`).

The wasm Live loop may keep using raw RPC; both paths share the
allowlist + multi-user gates.

### Unit tests (no VSCode host)

```bash
cd extensions/vscode-weft-panel
npm run test:wsp
# or: npm run test:unit
```

## Known gaps (deferred)

- No voice input — VSCode webviews can't expose `allow="microphone"`
  yet (microsoft/vscode#303293). Capture sidecar lands next.
- No typed active-radar return schema on the observation stream
  (`observe` opens a local id; full radar is M2).
- No `ThreadDock` primitive for per-agent parallel output.
- Live tone sync from wasm → DOM a11y strip (today the E2E path uses
  a mock inject; production canvas tones are still canvas-only).

References:
- Architecture & rationale: ADR-011, session-7 findings.
- Protocol: `.planning/symposiums/compositional-ui/protocol-spec.md`.
- Verb set: `.planning/symposiums/compositional-ui/adrs/adr-005-wsp-verb-set.md`.
- Chip DOM followup: WEFT-558 / WEFT-486.
- WEFT-285: `docs/plans/wave-0-WEFT-285-result.md`.

## WEFT-283 active-radar

Typed `radar-return` / `variant-id` echo — see `src/activeRadar.ts` and `npm run test:active-radar`.
