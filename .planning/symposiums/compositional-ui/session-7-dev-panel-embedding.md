# Session 7 — Dev-panel embedding (RQ7, priority-one)

**Status**: Research findings, April 2026.
**Scope**: How to land a first-class WeftOS dev-panel inside Cursor / VSCode
that satisfies the four predicates and exercises the active-radar loop end
to end. This is the shippable prototype target; every recommendation here
is biased toward "what gets a useful thing in front of us this sprint."

---

## Scope

We want a panel inside Cursor/VSCode that:

1. Renders a compositional egui surface (our existing `clawft-gui-egui`
   crate — boot splash, desktop shell, 12 blocks, `Live` poller against
   the kernel daemon).
2. Talks to the local WeftOS daemon (`clawft-kernel` over its Unix
   JSON-RPC socket) with the same ontology-addressable substrate the
   Desktop shell subscribes to, so both can be open at once.
3. Accepts **voice input** as a first-class entry mode (four-predicates
   rule 4: dual-operator).
4. Shows **multiple concurrent agent threads** with reasoning visible
   and does not steal focus or scramble ordering on streamed output.
5. **Survives** VSCode reloads, extension crashes, and daemon restarts
   without losing goal state.
6. Bridges VSCode workspace state (open files, diagnostics, LSP) into
   the substrate so agents can reason about what the dev is doing.

Out of scope for this session: the protocol spec itself (RQ6), the
primitive set beyond what we already have blocks for (RQ5), the
desktop-only shell posture.

---

## Embedding option teardown

Four candidate architectures. I evaluate each against: four-predicates
fit, voice path, multi-agent surfacing, reload survival, build/ship
cost, and active-radar loop support.

### Option A — Webview + WASM (egui compiled to `wasm32-unknown-unknown`)

Compile `clawft-gui-egui` as a `cdylib`, generate JS glue with
`wasm-bindgen`, render into a `<canvas>` inside a
`WebviewView` (sidebar) or `WebviewPanel` (editor area). Extension host
loads the wasm bundle from `localResourceRoots`, the webview posts
messages through `acquireVsCodeApi()` to a stdio bridge the extension
maintains to the WeftOS daemon.

**Feasibility**: high. eframe 0.34+ fully supports
`wasm32-unknown-unknown` with WebGPU (preferred) or WebGL2 fallback,
and is known to run inside a hosted canvas via a `WebRunner`. The
web-components pattern ([ramblings.0x53a.com][egui-wc]) shows egui
working inside a shadow DOM, which is exactly what a VSCode webview
provides. No `wasm-eval` CSP is required; eframe compiles to plain
`wasm32` modules that load via `new WebAssembly.Module(bytes)`-style
glue from wasm-bindgen.

**Pros**
- Single-process: no sidecar to ship, sign, or orphan. Works on
  macOS, Linux, Windows, and (importantly) `vscode.dev` / codespaces.
- VSCode-native lifecycle: `retainContextWhenHidden: true` keeps the
  egui frame alive across tab switches; `getState`/`setState` +
  `WebviewPanelSerializer` survives window reloads; webview receives
  focus/blur/visibility events we can wire into Live's poller.
- Lego pieces already exist: `clawft-gui-egui/src/lib.rs::run()` is
  already structured for "embedder" use, and the shell/blocks don't
  depend on native threads.

**Cons / gotchas**
- **No native tokio on wasm.** Our `Live` poller currently
  `std::thread::spawn`s a current-thread tokio runtime and calls
  `DaemonClient::connect()` over a Unix socket. That whole path
  has to be replaced on wasm: the webview cannot open sockets
  directly. The substitute is `postMessage` to the extension host,
  which forwards to the daemon (see §"VSCode ↔ substrate bridge").
- **Microphone is gated.** VSCode webviews are sandboxed iframes
  without `allow="microphone"`; `getUserMedia` currently fails
  inside them ([issue 303293][mic-issue]). As of April 2026 this is
  still an open feature request. Voice input has to route via a
  sidecar or the OS-level mic (see §"Voice path").
- **Clipboard, file dialogs, OS notifications** route through
  VSCode APIs, not through eframe's native implementations. We need
  a thin adapter in the extension host.
- **CSP must be tight.** `default-src 'none'`, use `webview.cspSource`
  and a nonce for the wasm loader script; no inline JS.
- **Single `acquireVsCodeApi()` per session** — wrap it in a
  Rust-side singleton, do not re-acquire.
- **WebGPU availability is uneven.** Linux VSCode still falls back
  to WebGL2 frequently; our blocks render fine under WebGL2, but
  the `oscilloscope` and animated grid want to stay under 300 draw
  calls/frame to keep parity.

**Active-radar loop**: Native. We own the canvas, so `variant-id` +
topology/doppler/range/bearing are all observable without crossing
a process boundary. This is the loop's lowest-friction host.

### Option B — Native sidecar + thin extension shim

Ship `clawft-gui-egui` as a native binary. The extension spawns it as
a `child_process.spawn()` child on activation, passes a handshake
token, and the GUI opens its own OS window positioned near VSCode.
The panel shown in VSCode itself is either (i) a placeholder webview
with a "GUI is running in its own window" hint and a few quick
controls, or (ii) absent entirely.

**Pros**
- No wasm port. Our existing code runs unchanged.
- Full native access: mic, camera, OS notifications, file dialogs,
  Unix-socket daemon IPC, the lot.
- GPU performance identical to desktop.

**Cons**
- **It's not actually embedded.** The window floats beside the
  editor, which defeats the session's goal. On Linux/X11 we can
  reparent the native window into the VSCode frame via XEmbed,
  but it's fragile, breaks on Wayland, and doesn't exist on
  macOS/Windows. This is RQ7's "too fragile but worth stating"
  option — we state it, and we reject it as the primary path.
- Extension deactivation / reload does not reliably kill child
  processes on Windows ([issue 46002][child-proc]); orphan GUIs
  are a real risk. We would need a supervisor.
- No `vscode.dev` / remote-SSH support — the sidecar lives on the
  wrong host.
- Active-radar loop still works but has to ferry events across a
  process boundary on every interaction. Doable, not free.

### Option C — Hybrid (webview for pixels, sidecar for what webview can't do)

Primary surface is Option A (egui-wasm in webview). Alongside it, the
extension spawns a small **native audio/capture sidecar** whose only
job is to own the mic, run streaming speech recognition, and push
transcripts into the substrate. The sidecar is tiny, stateless, and
restartable. The webview subscribes to the substrate's
`substrate/voice/transcript` topic like any other surface.

**Pros**
- Gets Option A's UX and portability.
- Gets Option B's mic access — **without** having to port the GUI
  itself.
- The sidecar is a pure data producer, not a renderer, so crashes
  are cheap to recover from (the extension respawns; the webview
  never notices except that transcripts pause).
- Cleanly satisfies the foundations doc's "observation is per-user,
  governance-gated, observable to the user" constraint — the mic
  sidecar is one process, its state is one tray chip, consent is
  one ADR.

**Cons**
- Two things to build instead of one.
- A second process to version-lockstep with the daemon.

### Option D — "Real window" via X11/Wayland or Tauri-in-webview

Listed for completeness. X11 embedding was already addressed under
Option B. "Tauri inside a VSCode webview" is not viable: Tauri is
a windowed app runtime, not a web component, and VSCode webviews
cannot host another WebView2/WKWebView as a nested chrome. Reject.

### Recommendation

**Ship Option C (Hybrid).** The WASM-in-webview path is the UX and
portability win; a dedicated audio/capture sidecar is the only part
that genuinely can't live in the webview today (the microphone gate
is unresolved in VSCode as of April 2026 and is not something we
want to be blocked on). Treat the sidecar as a *capture plane*, not
a GUI plane — its only job is to put raw signals on the substrate.
Everything else, including the entire four-predicates surface, runs
inside the webview wasm.

When VSCode ships the mic-permissions feature (tracked in
[microsoft/vscode#303293][mic-issue]), the sidecar can collapse into
the webview with no change to the protocol. That's the canonical
form of "transitional architecture": the seams are the same, only
the process boundary moves.

---

## Voice / interrupt / multi-agent surface strategy

### Voice input path

1. The extension activates on startup and spawns the capture sidecar
   (`weft-capture`, new bin crate). The sidecar owns mic + optional
   screen-OCR + optional camera frame capture — i.e., the
   `foreign://mic`, `foreign://screen`, `foreign://camera` wrappers
   from `foundations.md`.
2. The sidecar connects to the daemon's Unix socket exactly like
   `DaemonClient` does today, and publishes to substrate topics:
   `substrate/voice/waveform`, `substrate/voice/transcript`,
   `substrate/voice/segments`. ASR can be local (whisper.cpp) or
   remote, configured via `clawft-types/src/config/kernel.rs`.
3. The webview, running egui-wasm, subscribes to the same topics
   through the extension-host message bridge. A voice chip in the
   tray (tier-A primitive with `state=listening|muted`, affordances
   `mute|unmute|push-to-talk`, confidence = ASR confidence,
   streaming = `chunk` verbs on every partial) renders the live
   state. Partial transcripts flow into the active composer with
   progressive disclosure so the user and any agent can read them
   as they arrive.
4. Consent / governance: the sidecar refuses to start without a
   signed capability grant from `governance-counsel`. The tray chip
   is **non-dismissible while capturing** — foundations' "no dark
   recording, ever" is a hard rule, not a nicety.

### Interrupt handling

Three real interrupt events:

- **User starts typing while an agent is streaming a reply.**
  Composer is a tier-A primitive with an `interrupt` affordance. On
  first keystroke we emit `substrate/intent/interrupt { target:
  agent-id }`. The agent's streaming token is honoured the same way
  MCP/LSP cancellations are — it keeps the prefix it already
  emitted, stops, and the ECC records both the interruption
  (bearing = user chose to override) and the interruption latency
  (range = how long the user let the agent run before cutting in).
- **User speaks over TTS.** Same mechanism. The ASR sidecar detects
  user voice while TTS is playing and emits an interrupt. The TTS
  primitive ducks within 80ms. No half-duplex nonsense.
- **Agent wants to interrupt the user.** Not allowed synchronously.
  Agents can *queue* for attention via the substrate's
  interruption-priority scoring (Rasmus probabilistic kernel
  crosswalk); the panel surfaces a pending-notifications chip. ECC
  picks the moment.

Treat interruption as a first-class verb on the protocol, not an
application feature. Every composer, TTS surface, and long-running
chip must answer `interrupt`.

### Multi-agent parallel output

CLI chats scramble because they're a single 1-D scroll of
interleaved tokens from N agents. Our answer is structural: the
panel is a **column-per-agent-thread dock**, each column is a
streaming-native composer, and the ordering inside a column is
monotonic per agent. No cross-thread interleaving in any single
widget.

Concretely:
- A `ThreadDock` primitive holds N `AgentThread` primitives.
- Each `AgentThread` has its own `tick` / variant-id / freshness;
  it is a subscription-attached streaming view.
- A single **focus lane** (the active thread) gets keyboard focus
  and the composer. All other threads animate but do not grab
  focus. Users move between threads via Tab / voice verb
  ("switch to the planner") / click.
- Ephemeral **presence pips** in each non-focused column show
  token rate and state (thinking / streaming / awaiting-input /
  done) without demanding attention.
- Governance rule: an agent column cannot raise a modal. Period.
  Anything that would be modal becomes a chip in the tray with an
  "act on this" affordance.

The four-predicates win here is that each thread is
ontology-addressable; an agent can subscribe to another agent's
thread as a primitive, enabling supervision / delegation surfaces
without hand-coded glue.

---

## VSCode ↔ substrate bridge design

A thin extension (TypeScript) that does five things and no more:

1. **Host the webview(s).** One `WebviewView` in the activity-bar
   for the dev-panel (Cursor's chat-tab convention — see §Cursor
   notes). Optionally a `WebviewPanel` in the editor area for the
   "full Mission Console" posture.
2. **Proxy daemon RPC.** Open one long-lived Unix-socket
   connection to the kernel daemon, and forward webview
   `postMessage` requests through it. The existing
   `clawft_rpc::protocol::{Request, Response}` newline-delimited
   JSON format transfers unchanged — extension-side it's a
   `net.createConnection(path).pipe(newlineSplitter)` and an
   id→reply map.
3. **Mirror workspace state as substrate topics.** The extension
   is the adapter RQ7 hints at: it subscribes to VSCode events
   and publishes them to the daemon:
   - `workspace.onDidChangeTextDocument` → `substrate/editor/buffer/<uri>`
   - `languages.onDidChangeDiagnostics` + `languages.getDiagnostics()`
     → `substrate/editor/diagnostics`
   - `workspace.workspaceFolders` → `substrate/editor/roots`
   - `tasks.onDidStartTask` / `onDidEndTask` → `substrate/editor/tasks`
   - `window.activeTextEditor.selection` → `substrate/editor/focus`
     (opt-in under governance; treat as capture data)
   - `env.clipboard` reads are pull-only via an RPC from the webview,
     not a continuous topic — foundations' privacy rule.
   The daemon just sees more topics; every other surface (the
   Desktop shell, another Cursor window, a remote operator) can
   subscribe to the same stream.
4. **Forward substrate → webview.** Subscriptions flow the other
   way over the `acquireVsCodeApi().postMessage` channel. Messages
   are tagged with their substrate topic + streaming verb
   (`chunk`/`commit`/`cancel`) straight from the protocol.
5. **Supervise the capture sidecar.** Spawn, restart, kill on
   deactivate. On Windows, use the "fork-then-spawn" pattern from
   [vscode#46002][child-proc] to survive session end, and shell out
   to `taskkill /T` on cleanup.

The extension keeps **no business logic** of its own. If it knows
something the daemon doesn't, that's a bug. This is what makes the
Desktop shell and the dev-panel equivalent substrate citizens —
both read the same topics; the only difference is their posture.

### Reload / crash survival

- **Daemon restart**: the webview's `Live`-equivalent keeps the
  connection state as `Connecting|Connected|Disconnected` (we
  already do this in `live.rs`). The extension reconnects
  transparently with exponential backoff; the webview shows a
  subdued "daemon reconnecting" chip, does not clear state.
- **Extension reload / VSCode window reload**: the webview is
  destroyed. `WebviewPanelSerializer` + `getState()` restore its
  scroll position, active thread, composer draft, and any
  in-flight agent-thread IDs. On restore, the webview reopens
  subscriptions for the threads it remembers; the agents' state
  is durable in the daemon (ECC + ReasoningBank), so the user
  sees their conversation resume mid-thought.
- **Extension host crash**: same as above, but the extension also
  reconnects the daemon socket on activation. The capture sidecar
  survives the crash (it's a daemon child, not an extension
  child) and continues publishing; the webview re-subscribes.
- **Capture sidecar crash**: extension respawns with backoff. Tray
  chip flips to "capture offline"; governance log records the
  event; no state loss anywhere else.
- **`retainContextWhenHidden: true`** only for the dev-panel
  webview — it's our "sovereign posture" surface per Cooper. The
  memory cost is worth it to avoid re-booting egui on every tab
  switch. Narrower WebviewPanels (Mission Console ephemeral
  modes) use `getState()`-driven restore instead.

---

## Cursor-specific notes

- Cursor is a VSCode fork; the extension API is compatible —
  `WebviewView`, `createWebviewPanel`, `authentication`,
  `languageModel`, `tasks` all work identically, and the
  `.vsix` format is the same one we ship to VSCode.
- Cursor's own chat pane **cannot be replaced by an extension**.
  That's fine: we are not the chat pane. We are a *different*
  panel that sits alongside it, and our value proposition
  (ontology-addressable, streaming, voice, multi-agent, project
  goals that outlive a Cursor thread) is orthogonal to Cursor's
  single-thread LLM chat. If a user runs both, that's great —
  our ECC reads Cursor-chat activity as yet another substrate
  signal (`foreign://cursor-chat`) and times its interruptions
  around it.
- Cursor 3.0's "chat tabs" (`⌘T` for a new tab) are a UI-level
  feature, not an API. We do not need to mimic them; our
  ThreadDock achieves the same in-panel multi-thread effect
  inside our own webview.
- Proprietary extensions (Live Share, C++, some internal Copilot
  bits) ship a check that disables them in Cursor. We are not one
  of those, and we should not rely on any extension-host API that
  Microsoft has lately started fencing (`vscode.chat` LM provider
  registration is the watch-spot; we use it only defensively).

---

## Concrete prototype plan

Three milestones. Each should be ~2-3 days of focused work. Each
ends on a thing that demonstrates at least one of the four
predicates end-to-end.

### Milestone 0 — Webview host, no egui yet (predicate 4 smoke-test)

Goal: prove the pipe is real.

1. `clawft-vscode-ext/` — new extension crate (TS, in the
   workspace but outside `crates/`, since it isn't Rust). Ship a
   `WebviewView` in the activity bar with a static HTML page:
   one text input, one log pane.
2. Extension opens the daemon socket on activate, forwards
   `kernel.status` as a JSON message every second, renders it in
   the log pane.
3. Typing in the input and hitting enter sends a `kernel.ps`
   request; response fills the log.
4. Add `getState()`/`setState()` + a `WebviewPanelSerializer` so
   a window reload preserves the input's draft text.

Passes if: the panel survives a window reload without losing
state, and the user can invoke a daemon RPC by typing. That's
already a working "dual-operator" primitive, even if it's
ugly.

### Milestone 1 — egui-wasm in the panel (predicates 1 + 2 + 3)

Goal: our real surface, inside the panel.

1. Add `wasm32-unknown-unknown` as a target to
   `clawft-gui-egui`, gate native-only code (`Live`, Unix
   `DaemonClient`) behind `#[cfg(not(target_arch = "wasm32"))]`.
2. Implement a wasm-flavoured `Live`: same `Snapshot` struct, same
   API, but its poller is a JS-glue function that calls
   `postMessage` with the RPC request and registers a callback
   on the reply. From egui's point of view nothing changes.
3. Build with `wasm-pack` (or `trunk` for dev speed), load the
   `.wasm` + glue from the extension via `asWebviewUri`, attach
   to a single `<canvas>` with an id — this is the known-good
   web-components recipe. Use `retainContextWhenHidden: true`.
4. Wire up `acquireVsCodeApi()` once, thread the handle into the
   wasm module via `#[wasm_bindgen]`.

Passes if: we see the boot splash → desktop shell inside Cursor,
Live polling works through the bridge, and we can invoke a block
(Terminal, Tree, Table) and see it update from the real daemon.
That is all three of ontology-addressable, self-describing, and
streaming-native demonstrated in one surface.

### Milestone 2 — Voice + threads + workspace bridge (active-radar)

Goal: turn on the loop.

1. Ship `weft-capture` bin crate: a mic-only sidecar with a
   `substrate/voice/transcript` publisher, whisper.cpp as the
   default ASR, governance gate on the capability grant, and
   a persistent tray chip in the egui surface.
2. Add a `ThreadDock` primitive and two `AgentThread` columns
   (predicate: each thread is ontology-addressable and
   streamable). One thread is the user's main conversation; the
   second is a supervisor thread watching the first.
3. Extension-side workspace adapter: publish
   `onDidChangeTextDocument`, `onDidChangeDiagnostics`, and
   `workspaceFolders` as substrate topics. An agent can now
   answer "what file am I in?" by subscribing, not by being
   told.
4. Add the `variant-id` + return-signal schema to every block's
   render call; log to `substrate/ux/returns`. This is the
   active-radar loop, live, running.

Passes if: we can speak a request, see two agents work on it in
parallel without fighting for focus, see one of them cite a file
it learned about from the workspace bridge, and have ECC log the
return signal against the emitted variant. That closes the loop.

### What is **not** in the prototype

- No new primitives beyond what `clawft-gui-egui/src/blocks/`
  already has. Those 12 blocks are enough to demonstrate all
  four predicates; adding more belongs in RQ5.
- No Windows support in milestone 0–2 (we're Unix-socket only
  today). Windows named pipes are a follow-up.
- No Mission Console "shared war-room" mode. That's RQ3 and
  needs the protocol spec from RQ6 to be meaningful.

---

## Recommendations for canon

1. **Canonical dev-panel host is a VSCode `WebviewView` running
   an egui-wasm module.** Native-window embedding is rejected as
   primary. (ADR-worthy.)

2. **A single capture sidecar per user session owns the mic,
   camera, and screen wrappers.** It is a substrate publisher,
   never a renderer. Governance gates it at capability-grant
   time, not per-tick. Consent is enforced by the daemon, not by
   the extension.

3. **The VSCode extension has zero business logic.** It is a
   socket proxy + workspace-adapter + sidecar supervisor. If it
   gains opinions, they belong in the daemon.

4. **Workspace signals are substrate topics, not ad-hoc
   messages.** `substrate/editor/*` is reserved; every field
   (buffer, diagnostics, selection, tasks) is a topic any surface
   can subscribe to. The Desktop shell and the dev-panel read the
   same topics; parity is the test that the bridge is honest.

5. **Interruption is a verb on every streaming primitive.** If a
   primitive emits streaming chunks, it **must** answer an
   `interrupt` on the same stream. No exceptions.

6. **Multi-agent output is column-per-thread, never
   interleaved.** A `ThreadDock` containing N `AgentThread`s is
   the canonical shape; anything that needs a cross-thread view
   is a separate supervisor surface.

7. **Reload survival is a daemon concern, not a webview
   concern.** Agent state, ECC state, and goal state live in the
   daemon; `getState()`/`setState()` only remembers view
   position + composer draft + subscribed thread IDs.

8. **`retainContextWhenHidden: true` for the dev-panel, not for
   transient panels.** Memory is cheaper than re-boot; flicker is
   what kills the sovereign-posture feel.

9. **`variant-id` on every render and a `substrate/ux/returns`
   topic are non-optional** from milestone 2 forward. The active
   radar loop is the product, not an analytics feature.

10. **Cursor is a first-class target equivalent to VSCode** as
    long as we use only stable APIs. Cursor-specific chat
    integration is explicitly *not* a goal; we run alongside it.

---

## Open questions

- **Remote / Codespaces posture.** WASM-in-webview works on
  `vscode.dev`, but the daemon is local to the dev's workstation.
  Do we treat `vscode.dev` as "no local daemon available,
  degrade to read-only view of a remote substrate," or do we
  require a reachable daemon (e.g. via WeftOS mesh)? ADR
  material.
- **Windows named-pipe transport.** Our RPC client's non-Unix
  stub returns `None`. Milestone 2+ will have Windows users;
  when?
- **Mic permission resolution path.** If microsoft/vscode#303293
  lands before we ship, the capture sidecar can collapse into
  the webview. Do we want to delay that collapse or adopt it
  immediately? Preference: adopt, but keep the sidecar interface
  stable so third-party capture providers can still plug in.
- **Cursor chat as a substrate signal.** We can read-only observe
  Cursor's chat window via its public extension hooks (limited),
  or via screen-capture OCR of its pane. Is the OCR path
  acceptable under governance, or is it opt-in-per-session? The
  foundations doc leans opt-in; confirm in an ADR.
- **ECC backpressure under token floods.** When three agents
  stream into three columns at 200 tokens/sec each, does the
  return-signal topic keep up? Probably; measure in milestone 2.
- **Protocol for cross-instance co-existence.** The same daemon
  can serve a Cursor webview and a Desktop shell simultaneously.
  Do subscriptions need a `surface-id` so ECC can tell them
  apart? Yes, almost certainly — but confirm the schema in RQ6.
- **Chat Participant API as a second entry point.** VSCode's
  `vscode.chat.createChatParticipant` (finalized mid-2025)
  lets us appear in Copilot Chat as `@weft` without a webview.
  Worth it as a shallow surface that punches the user through
  to the full panel? Maybe; scope in a follow-up.

---

## Sources

- [VSCode Webview API reference][wv]
- [VSCode Extension API reference][api]
- [Chat Participant API][chat]
- [Continue.dev `core ↔ extension ↔ gui` architecture][cont]
- [Microsoft open-sources Copilot Chat][copilot]
- [Mic permission in VSCode webviews — open issue][mic-issue]
- [egui as a web component][egui-wc]
- [Child-process lifecycle gotchas in VSCode extensions][child-proc]
- [Cursor migration notes for VSCode extensions][cursor]

[wv]: https://code.visualstudio.com/api/extension-guides/webview
[api]: https://code.visualstudio.com/api/references/vscode-api
[chat]: https://code.visualstudio.com/api/extension-guides/ai/chat
[cont]: https://deepwiki.com/continuedev/continue
[copilot]: https://github.com/microsoft/vscode-copilot-chat
[mic-issue]: https://github.com/microsoft/vscode/issues/303293
[egui-wc]: https://ramblings.0x53a.com/egui-web-components.html
[child-proc]: https://github.com/microsoft/vscode/issues/46002
[cursor]: https://thinkpeak.ai/migrating-vs-code-extensions-to-cursor-2026/
