# ADR-011: Dev-Panel Embedding — egui-wasm-in-webview + Capture Sidecar

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Session 7 (RQ7, priority-one prototype) evaluated four embedding
architectures for a WeftOS dev-panel inside Cursor / VSCode:
(A) egui compiled to `wasm32-unknown-unknown`, rendered into a
VSCode `WebviewView`; (B) a native sidecar spawned as an OS window;
(C) hybrid — A for the surface, a small native sidecar for what the
webview cannot do; (D) X11/Wayland or Tauri-in-webview gymnastics.
The dev-panel is the priority-one prototype target per AGENDA; it
must satisfy all four predicates, voice must be first-class
(predicate 4), and the active-radar loop must run end-to-end
(ADR-007). Two blockers drove the choice: (i) VSCode webviews
cannot `getUserMedia` (microsoft/vscode#303293 unresolved as of
April 2026); (ii) native windows embedded in VSCode are fragile on
Wayland/macOS/Windows. Session 2's twelve-system teardown showed no
competitor has even tried the full dev-panel the canon requires —
voice, multi-agent columns without interleaving, goals outliving
sessions, return signals. Getting this embedded cleanly is the
product.

## Decision

Ship Option C (hybrid). The dev-panel is an egui-wasm module
running inside a VSCode `WebviewView`; a small `weft-capture`
native sidecar owns the microphone and other capture wrappers and
publishes to substrate topics. The sidecar is a *capture plane*,
not a GUI plane. The VSCode extension is a zero-business-logic
proxy: socket + workspace-adapter + sidecar supervisor.

**Canonical host**: a VSCode `WebviewView` in the activity bar for
the sovereign-posture dev-panel (`retainContextWhenHidden: true`,
Cooper sovereign → "memory is cheaper than reboot"). Optional
`WebviewPanel` in the editor area for full Mission Console posture.

**Surface plane** (webview + egui-wasm):
- `clawft-gui-egui` compiled to `wasm32-unknown-unknown` via
  `wasm-bindgen`; loads from `localResourceRoots` with a tight
  CSP (`default-src 'none'`, nonce'd wasm loader, no inline JS).
- Native-only paths (Unix socket `DaemonClient`, tokio threads)
  gated behind `#[cfg(not(target_arch = "wasm32"))]`.
- A wasm-flavoured `Live` implements the same `Snapshot` API but
  forwards RPC requests via `acquireVsCodeApi().postMessage` to
  the extension host, which pipes them through the daemon's
  UDS JSON-RPC (ADR-003 local rail).
- `WebviewPanelSerializer` + `getState()/setState()` restore
  scroll, active thread, composer draft, subscribed thread ids
  across reload.

**Capture plane** (`weft-capture` sidecar):
- Owns mic (always), camera + screen (gated); runs ASR (whisper.cpp
  default) and publishes `substrate/voice/waveform`,
  `substrate/voice/transcript`, `substrate/voice/segments`.
- Connects to the daemon as a normal client; not a renderer.
- Refuses to start without a signed capability grant from
  `governance-counsel`; consent is per-goal (ADR-008, ADR-012).
- Renders a persistent tray `ui://chip{kind="capture"}` composed by
  the kernel whenever any capture is live (ADR-012).
- Crash-recoverable: extension respawns with backoff; webview
  notices only that transcripts pause.

**VSCode extension** — zero business logic, five responsibilities
and no more:
1. Host the webview(s).
2. Proxy daemon RPC over UDS.
3. Mirror workspace state as substrate topics
   (`substrate/editor/{buffer,diagnostics,roots,tasks,focus}`);
   focus/selection under governance as capture data.
4. Forward substrate → webview via `postMessage` tagged with
   topic + stream verb (`chunk` / `commit` / `cancel`).
5. Supervise the capture sidecar (spawn, restart, kill; Windows
   uses fork-then-spawn from vscode#46002 and `taskkill /T`
   on cleanup).

**Milestone ordering** (Session 7 §"Concrete prototype plan"):
- **M0** — webview host + static HTML + daemon RPC proxy +
  `WebviewPanelSerializer`. Passes if the panel survives window
  reload and can invoke a daemon RPC.
- **M1** — egui-wasm in the webview; wasm-flavoured `Live`; boot
  splash → desktop shell → blocks invokable from real daemon.
  Demonstrates predicates 1 + 2 + 3 in one surface.
- **M2** — `weft-capture` sidecar + `ui://thread-dock` composition
  + workspace-bridge topics + `variant-id` / return-signal
  emission on `ux/returns` (ADR-007). Demonstrates predicate 4 +
  closes the active-radar loop.

**Multi-agent output discipline** (Session 7 §"Multi-agent parallel
output"): a `ThreadDock` composition holds N `AgentThread`
primitive instances; ordering inside a column is monotonic per
agent; no cross-thread interleaving in any single widget. One
focus lane at a time; non-focused columns animate but do not steal
focus. Agent columns cannot raise modals; "act on this" becomes a
tray chip.

**Interruption is a verb on every streaming primitive** (Session 7
§"Interrupt handling", foundations predicate 3). If a primitive
emits streaming chunks, it answers an `interrupt` on the same
stream — no exceptions. Agent-to-user interruption is never
synchronous; agents queue via interruption-priority scoring
(ECC-weighted).

**Cursor treated equivalently to VSCode**. Cursor's own chat pane
is not replaced. Our value proposition (ontology-addressable,
voice, multi-agent, goal persistence) is orthogonal to Cursor's
single-thread LLM chat; both can run together, and our ECC reads
Cursor's pane as a `foreign://cursor-chat` signal.

**When vscode#303293 lands**, the sidecar collapses into the
webview with no change to the protocol. The seam is the substrate
topic boundary; the process boundary is allowed to move.

## Consequences

### Positive
- Option A's UX and portability (works in `vscode.dev`,
  codespaces, macOS/Linux/Windows) plus Option B's mic access
  without porting the GUI.
- Zero business logic in the extension means the Desktop shell and
  the dev-panel are substrate peers — both read `substrate/editor/*`
  topics; parity is the test.
- Sidecar failure is cheap to recover — it is one process, one
  state, one consent grant; the rest of the panel is unaffected.
- Return-signal emission is on the renderer side (we own the
  canvas), so topology / doppler / range / bearing reconstruct
  with no process boundary crossed on every interaction.
- `ThreadDock` + `AgentThread` columns close Session 2 gap #4
  (parallel output serialised or visualised-only) by making each
  thread an ontology-addressable primitive another agent can
  subscribe to.

### Negative
- Two artefacts to ship and version-lockstep: the wasm bundle and
  the sidecar binary. Cross-platform signing / notarisation adds
  overhead.
- Sidecar ↔ webview transition path (when vscode#303293 lands)
  must be planned for now, not later; protocol seams must stay
  stable across the collapse.
- WebGPU availability on Linux VSCode is uneven; we plan to keep
  animated primitives under 300 draw calls/frame to keep WebGL2
  parity.
- Windows named-pipe transport for RPC is a gap (Session 7 open
  Q); milestone 2+ unblocks Windows users.
- `vscode.dev` posture is ambiguous without a local daemon —
  degrade gracefully to read-only remote substrate view or require
  mesh reachability. ADR material.

### Neutral
- `retainContextWhenHidden: true` only on the sovereign dev-panel;
  transient Mission Console panels use `getState()`-driven
  restore. Cooper posture, directly expressed.

## Alternatives considered

1. **Option A alone (webview-only)** — rejected: vscode#303293
   blocks voice-as-first-class (predicate 4). Without mic, the
   active-radar loop cannot receive voice returns; a core
   predicate fails.
2. **Option B alone (native sidecar)** — rejected: native-window
   embedding is fragile on macOS/Windows/Wayland (no cross-platform
   XEmbed), no `vscode.dev`/remote-SSH support, child-process
   orphan risk on Windows (vscode#46002). Option B fails the
   "actually embedded" bar.
3. **Option D (Tauri-in-webview)** — rejected: webviews cannot
   nest another WebView2/WKWebView; Tauri is a windowed app
   runtime, not a web component. Architecturally impossible.
4. **Full native with X11 XEmbed only** — rejected: Linux-only
   path; foundations requires cross-platform.
5. **Chat Participant API (`vscode.chat.createChatParticipant`)
   as primary surface** — rejected as primary; reserved as a
   shallow complementary entry (Session 7 open Q). The panel is
   the product; a `@weft` participant may punch the user into it.
6. **Ship after vscode#303293 lands** — rejected: timeline is
   unknown; blocking priority-one prototype on external fix is
   unacceptable. Sidecar is transitional; the seam persists.

## Related

- Sessions: `session-7-dev-panel-embedding.md` (entire doc —
  especially four-option teardown, voice/interrupt/multi-agent
  strategy, bridge design, milestone plan, 10 recs),
  `session-2-agentic-ui-sota.md` (Cursor / Continue / Claude Code
  teardowns, gap 4 multi-agent, gap 6 voice, gap 8 return
  signals), `session-6-protocol-design.md` (§12 open Q:
  browser/wasm encoding), `session-9-agentic-os-canon.md`
  (rec. 4 ForeignSurface, rec. 6 voice as peer).
- Foundation elements: predicate 4 (dual-operator, voice
  first-class), predicate 3 (streaming-native interruption),
  §"Two tiers" (ambient capture wrappers), §"active-radar loop"
  (variant-id on every render).
- ADRs: ADR-001 (canon includes `ui://thread-dock` composition,
  `ForeignSurface`), ADR-003 (transport — WebSocket browser path
  also available here), ADR-007 (`ux/returns`),
  ADR-008 (goals bound to consent), ADR-012 (capture privacy tray
  chip).
