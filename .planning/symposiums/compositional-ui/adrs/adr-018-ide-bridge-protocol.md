# ADR-018: IDE Bridge Protocol — editor-in topics, editor-out affordances, follow-editor, chrome-match

**Date**: 2026-04-20
**Status**: Proposed — symposium round 3
**Deciders**: Compositional UI Symposium (Session 10, §4 / §7 / §8 recs 3, 8)

## Context

ADR-011 shipped the *where*: egui-wasm inside a VSCode `WebviewView`, a
zero-logic extension proxying daemon RPC, `weft-capture` sidecar for
the microphone. M0 (webview + RPC proxy) and M1 (egui-wasm boot, four
raw RPC verbs — `kernel.status` / `kernel.ps` / `kernel.services` /
`kernel.logs`) landed on `development-0.7.0` (`6e2df4d`).

Session 10 §4 then named four linkage components the current extension
does not implement: an **editor-in adapter** feeding IDE signals onto
ontology topics, an **editor-out affordance catalog** letting surfaces
reach back, **follow-editor** routing that swaps the active surface on
focus change, and **chrome-match** pulling host palette into the panel.
§7 schedules them as M1.6 (editor-in), M1.7 (editor-out), M1.9
(chrome-match); follow-editor mechanics are app-manifest work (ADR-015)
but depend on the focus topic defined here. §8 recs 3 and 8 fix the
scope: the bridge is a module keyed off session config, `desktop` and
`ide` share the compositor code path, each component gets its own
sub-decision with its own wire format.

This ADR specifies the **wire-format layer only** — topic shapes
published by the editor-in adapter, affordance verbs callable on the
editor-out side, the follow-editor manifest DSL, the chrome-match
topic, and per-host transport envelopes. It does **not** specify how
the workspace adapter internally derives those topics from VSCode APIs
(ADR-017), nor the `invocations` field on `Affordance` (ADR-019).

## Decision

Ship the IDE bridge as a **host-agnostic protocol over a host-specific
transport**. The protocol layer: six `substrate/editor/*` topics, seven
`ide.*` affordance verbs, a follow-editor manifest rule DSL, one
`substrate/editor/theme` topic, a versioned handshake. The transport
layer: `postMessage` envelopes for VSCode/Cursor (M1.6 first class),
JSON-RPC over loopback for JetBrains (future), Lua RPC for Neovim
(sketch only).

### Sub-decision 1 — Editor-in: `substrate/editor/*` topic shapes

Six topics, each a standard WSP ontology topic consumable by any
subscribed surface. Semantics are **last-write-wins snapshot per topic
key** unless noted; the adapter may coalesce deltas.

- **`substrate/editor/buffer`** — active file under focus.
  ```json
  { "active_path": "crates/clawft-gui-egui/src/canon/pressable.rs",
    "dirty": false, "language": "rust", "last_saved_at": "2026-04-20T14:03:11Z" }
  ```
  `active_path` workspace-relative if inside a known root, else
  absolute. `language` = host's language id (not normalised).
  `last_saved_at` ISO-8601 UTC; null if never saved.

- **`substrate/editor/cursor`** — cursor + selection of active buffer.
  ```json
  { "line": 142, "column": 7,
    "selection": { "start": {"line":142,"column":7}, "end": {"line":145,"column":12} } }
  ```
  `line` / `column` 0-indexed. `selection` is `null` when not ranging.
  Adapter MUST rate-limit to ≤ 30 Hz.

- **`substrate/editor/diagnostics`** — LSP diagnostics scoped to the
  active buffer. Workspace-wide consumers use the wildcard variant
  (per-path subtopic tree; ADR-017).
  ```json
  [{ "path": "crates/clawft-gui-egui/src/canon/pressable.rs",
     "range": { "start": {"line":142,"column":7}, "end": {"line":142,"column":15} },
     "severity": "error", "message": "cannot find value `foo` in this scope",
     "source": "rust-analyzer" }]
  ```
  Severity ∈ `"error" | "warn" | "info" | "hint"`.

- **`substrate/editor/focus`** — drives follow-editor (§3).
  ```json
  { "pane_id": "workbench.editors.main#2", "kind": "editor",
    "buffer_path": "crates/clawft-gui-egui/src/canon/pressable.rs" }
  ```
  `kind` is `"editor" | "terminal" | "panel"`. `buffer_path` set only
  for `kind == "editor"`. `pane_id` is host-opaque, stable per-pane.

- **`substrate/editor/terminal`** — **stream topic** (WSP
  `open / chunk / commit / cancel`, protocol-spec §stream). Per-chunk
  shape:
  ```json
  { "terminal_id": "t-3", "stream": "stdout", "chunk": "  Compiling clawft-core v0.7.0\n", "ts": "2026-04-20T14:05:02.319Z" }
  ```
  `stream` is `"stdout" | "stderr"`. Capture-grade data (§7).

- **`substrate/editor/tasks`** — running jobs / debug sessions /
  build tasks.
  ```json
  [{ "task_id": "cargo-check-42", "name": "cargo check --workspace",
     "status": "running", "started_at": "2026-04-20T14:04:55Z" },
   { "task_id": "cargo-test-41", "name": "cargo test -p clawft-core",
     "status": "success", "started_at": "2026-04-20T14:02:10Z", "duration_ms": 43120 }]
  ```
  Status ∈ `"running" | "failed" | "success"`; `duration_ms` set only
  on terminal states.

All six are **read-only from the surface side**. Writes happen through
editor-out affordances (§2), never by publishing a mutated copy.

### Sub-decision 2 — Editor-out: `ide.*` affordance catalog

Seven named affordance verbs, each with a typed args schema and a
typed return. Surfaces invoke them through the standard Affordance
machinery (ADR-006); the bridge relays the invocation to the host and
returns the result. Each affordance carries an `invocations` list per
ADR-019 (not specified here — cross-reference).

| Verb | Args | Returns | Safety |
|---|---|---|---|
| `ide.open-file` | `{ path, line?, column?, reveal? }` | `{ ok: bool }` | safe-by-default |
| `ide.show-quick-pick` | `{ title, items: [{label, description?, detail?, value}] }` | `{ value } \| null` | safe-by-default |
| `ide.insert-at-cursor` | `{ text, snippet? }` | `{ inserted_range }` | governance-gated |
| `ide.reveal-in-explorer` | `{ path }` | `{ ok: bool }` | safe-by-default |
| `ide.run-command` | `{ command_id, args? }` | `{ ok, result? }` | governance-gated |
| `ide.write-terminal` | `{ terminal_id?, text, bring_to_front? }` | `{ ok }` | governance-gated |
| `ide.show-notification` | `{ level, message, actions? }` | `{ action_id? } \| null` | safe-by-default |

Schema notes:

- **`ide.open-file`**: `path` workspace-relative or absolute; `line` /
  `column` 0-indexed; `reveal` (default `true`) focuses the editor.
- **`ide.show-quick-pick`**: `items[].value` is opaque JSON returned
  unchanged as the selection; `null` means dismissed.
- **`ide.insert-at-cursor`**: `snippet: true` expands VSCode-style
  placeholders (`${1:name}`). Destructive → gated.
- **`ide.run-command`**: VSCode command id (anything registered via
  `vscode.commands.registerCommand`). Gated per-invocation because
  `command_id` is arbitrary.
- **`ide.write-terminal`**: falls back to the active terminal when
  `terminal_id` omitted. Equivalent to shell execution → gated.
- **`ide.show-notification`**: `actions` is optional labels + ids;
  returned `action_id` is which was clicked (`null` if dismissed).

**Safety classification is normative**: governance-gated affordances
MUST be wrapped in the per-invocation consent prompt (§7) even when
the app was installed with `ide.*` permission granted. The gate is
per-call, not per-install.

### Sub-decision 3 — Follow-editor manifest declaration

An app manifest (ADR-015) may declare zero or more `on_focus_change`
rules. When `substrate/editor/focus` emits a delta where `kind ==
"editor"`, the compositor evaluates each installed app's rules; the
first matching rule in install order fires.

Minimal DSL:

```toml
[[on_focus_change]]
when_path_matches = "crates/*/src/**"
swap_active_surface_to = "project-surface"

[[on_focus_change]]
when_path_matches = "docs/**/*.md"
when_workspace_root = "/home/aepod/dev/clawft"
swap_active_surface_to = "docs-surface"
```

Fields:

- **`when_path_matches`**: glob against `focus.buffer_path`. Standard
  glob: `*` one segment, `**` zero-or-more segments, `?` one char.
  Workspace-relative unless anchored with `/`.
- **`when_workspace_root`** (optional): absolute path — only match
  when the host's workspace root equals it. Lets one app ship per-repo
  rules.
- **`swap_active_surface_to`**: surface id declared elsewhere in the
  manifest (ADR-015 §surface registry).

Evaluated only in `ide` mode (ADR-015). Evaluation is at most once per
focus delta and MUST NOT itself publish onto any editor-in topic
(prevents loops).

### Sub-decision 4 — Chrome matching

Bridge publishes one additional topic — `substrate/editor/theme` —
with the host's palette + typography snapshot.

```json
{ "kind": "dark",
  "palette": { "editor.background": "#1e1e1e", "editor.foreground": "#d4d4d4",
               "activityBar.background": "#333333", "textLink.foreground": "#3794ff",
               "errorForeground": "#f48771" },
  "typography": { "font_family": "Menlo, Monaco, 'Courier New', monospace",
                  "font_size_px": 13, "line_height_px": 20 } }
```

`kind ∈ {"dark","light","high-contrast"}`. `palette` keys are
host-native token names — VSCode uses `--vscode-*` CSS custom
properties, mapped to this stable dictionary by the adapter (full
mapping in ADR-017). Unknown keys allowed; optional per surface.

**M1.9 deliverable** — signature locked here:

```rust
pub fn apply_ide_palette(ctx: &egui::Context, theme: &EditorTheme);
```

Reads `substrate/editor/theme`, translates to `egui::Visuals`. Opt-in
per surface; non-opting surfaces keep the default canon theme.
Chrome-match is optional by design.

### Sub-decision 5 — Host-specific transports

The protocol layer above is host-agnostic. Transports are per-host.

#### VSCode / Cursor — M1.6 first class

Transport: `window.postMessage` bidirectional between extension host
and webview. The extension is the **relay**: VSCode APIs
(`onDidChangeTextDocument`, `activeTextEditor`, `getDiagnostics`,
`createTerminal`, `executeCommand`, …) → WSP verbs outbound; WSP verbs
from the webview → VSCode API calls inbound.

**Inbound** (webview → extension):
```json
{ "type": "wsp", "verb": "subscribe.editor.buffer", "args": {}, "id": "req-17" }
{ "type": "wsp", "verb": "invoke.ide.open-file", "args": { "path": "Cargo.toml", "line": 0 }, "id": "req-18" }
```
`verb` canonical forms: `subscribe.editor.<topic>` /
`unsubscribe.editor.<topic>` / `invoke.ide.<affordance>` /
`handshake.hello` (§6).

**Reply** (extension → webview):
```json
{ "type": "wsp-reply", "id": "req-18", "ok": true, "result": { "ok": true } }
{ "type": "wsp-reply", "id": "req-18", "ok": false, "error": { "code": "governance.denied", "message": "user declined" } }
```

**Stream** (extension → webview, for `subscribe.*`): `wsp-stream`
envelopes with `sub_id` + `frame ∈ {open, chunk, commit, cancel}`.
`open` carries `schema` (e.g. `"substrate/editor/buffer/v1"`); `chunk`
carries `value`; `cancel` carries `reason`.

The extension's existing `ALLOWED_METHODS` set
(`extensions/vscode-weft-panel/src/extension.ts` lines 25–30) extends
to allow the eleven new verb prefixes (`subscribe.editor.*` ×6,
`invoke.ide.*` ×7) as prefixes, not strings. M1's `rpc-request` /
`rpc-response` envelope remains for raw daemon RPC; `wsp` / `wsp-reply`
/ `wsp-stream` is the new layer. Both coexist.

#### JetBrains — future (sketch)

JSON-RPC 2.0 over loopback (named pipe on Windows, Unix socket
elsewhere). Same WSP envelope inside `{jsonrpc, id, method, params}`.
Plugin runs an equivalent relay against IntelliJ Platform APIs
(`FileEditorManagerListener`, `EditorFactoryListener`, …). Deferred.

#### Neovim — future (sketch only)

msgpack-RPC or Lua plugin (`vim.rpcnotify` / `vim.rpcrequest`). Relay
maps autocmds to editor-in topics and verbs to `vim.api.*` calls.
Not scheduled.

**Priority is VSCode/Cursor.** The other two establish that the
protocol layer is host-agnostic, not that we've committed time.

### Sub-decision 6 — Handshake and versioning

On webview mount, the host sends one `ide-hello` message before any
other traffic:

```json
{ "type": "ide-hello", "version": "0.1",
  "capabilities": [
    "editor.buffer", "editor.cursor", "editor.diagnostics",
    "editor.focus",  "editor.theme",
    "ide.open-file", "ide.reveal-in-explorer", "ide.show-notification"
  ] }
```

`version` is the bridge protocol semver (`0.1` for M1.6).
`capabilities` is the set of topic ids + affordance verbs the host
implements; absent entries mean unimplemented. Subscribing to an
unadvertised topic returns an empty stream (`open` → `commit` with no
`chunk`) plus a warning on the observation log:

```json
{ "level": "warn", "code": "bridge.capability.missing",
  "topic": "substrate/editor/terminal",
  "message": "host does not advertise editor.terminal; empty stream" }
```

Invoking an unadvertised affordance returns `ok: false,
error.code: "bridge.capability.missing"`. Apps are expected to
degrade gracefully — not a hard error.

Version negotiation is forward-only: a `0.1` surface works against a
`0.2` host if `0.2` is a superset. Breaking changes bump the major;
the host refuses mount from incompatible surfaces.

### Sub-decision 7 — Governance

Reuse the existing gate (ADR-008 related). Two levels:

1. **Install-time**: manifest (ADR-015) declares which affordances the
   app uses; install prompts the user once.
2. **Per-invocation**: destructive affordances
   (`ide.insert-at-cursor`, `ide.run-command`, `ide.write-terminal`)
   prompt every invocation: `{allow-once, allow-session, deny}`.
   `allow-session` scoped to the session; does not persist.

The prompt is rendered by the compositor (not the app), so the app
cannot style or spoof it. Text includes app identity, verb name, and
a concise arg summary — "`project-OS` wants to run
`workbench.action.closeFolder`. Allow once / this session / deny?".

`ide.show-quick-pick` is **not** gated — the host owns the UI and the
return is user input, not a mutation.

### Sub-decision 8 — Security

Webview origin is `vscode-webview://...`; CSP is already restrictive
(see `renderHtml`, `extension.ts` lines 264–272). Bridge rules:

- Extension MUST validate every inbound `wsp.verb` against the
  allowlist: `subscribe.editor.{buffer,cursor,diagnostics,focus,
  terminal,tasks,theme}` and `invoke.ide.{open-file,show-quick-pick,
  insert-at-cursor,reveal-in-explorer,run-command,write-terminal,
  show-notification}`. Unknowns return `error.code: "bridge.verb.unknown"`
  and are logged.
- `ide.run-command` MUST NOT accept command ids starting with `_`
  (VSCode internal-only convention) without an explicit override flag
  the user is prompted about.
- Path args (`ide.open-file`, `ide.reveal-in-explorer`) MUST resolve
  via `vscode.Uri.file` and check against current workspace roots +
  user-granted-out-of-workspace list. `../` normalised before check.
- No eval. The bridge is a typed verb set; strings are data, not code.

### Sub-decision 9 — Mock bridge for tests

Integration tests need panel apps to run without real VSCode. The
`clawft-gui-egui` test harness ships a **MockIdeBridge** — in-memory
publishers for each editor-in topic + a sink logging every `ide.*`
affordance invocation. Minimum surface:

```rust
pub struct MockIdeBridge { /* publishers + affordance log */ }

impl MockIdeBridge {
    pub fn new() -> Self;
    pub fn publish_buffer(&mut self, buf: EditorBuffer);
    pub fn publish_focus(&mut self, focus: EditorFocus);
    pub fn take_affordance_invocations(&mut self) -> Vec<(String, serde_json::Value)>;
    pub fn reply_to(&mut self, verb: &str, result: serde_json::Value);
}
```

Tests drive the mock: publish focus change → observe surface swap;
invoke affordance → observe log, inject reply. No external process.
Ships with M1.6.

### Sub-decision 10 — Worked example: project-OS follow-editor

End-to-end:

1. User switches from `crates/clawft-core/src/pipeline.rs` to
   `crates/clawft-gui-egui/src/canon/pressable.rs`.
2. VSCode fires `onDidChangeActiveTextEditor`. Editor-in adapter
   publishes `substrate/editor/focus` with `kind: "editor"` and
   `buffer_path: "crates/clawft-gui-egui/src/canon/pressable.rs"`;
   relayed to the webview as a `wsp-stream` chunk.
3. Compositor evaluates installed apps. `project-OS` has
   `when_path_matches = "crates/*/src/**"` → match. Active surface
   swaps to `project-surface`.
4. New surface subscribes to `substrate/gh/prs` (via `gh` adapter,
   ADR-017) scoped to the crate. `Table` shows open PRs.
5. User clicks a PR row → cell carries
   `Affordance { name: "ide.open-file", args: { path: ".github/pull_request_template.md" } }`.
6. Surface emits `invoke.ide.open-file`. Relay calls
   `vscode.workspace.openTextDocument` + `showTextDocument`.
   `ok: true`.
7. User sees the PR template in the editor **and** the panel side.

Steps 1–3 are M1.6; 5–6 are M1.7; step 4 + `gh` adapter are ADR-017;
step 3 manifest mechanics are ADR-015.

## Consequences

### Positive

- `ide` mode becomes a useful place to **be**, not just host a wasm
  app. Panels read the editor; panels act on the editor.
- All four §4 linkage components have stable wire formats before
  implementation begins — M1.6 implementers don't relitigate shapes.
- Protocol layer is host-agnostic. JetBrains / Neovim will reuse the
  topic shapes + affordance catalog unchanged; only the relay differs.
- Reusing M1's `postMessage` channel means the extension gains one
  new envelope family (`wsp` / `wsp-reply` / `wsp-stream`) with no
  refactor of M1 code.
- Degradation is by design: unadvertised capabilities → empty streams
  + warnings, not exceptions. `0.1` surfaces run on `0.2` hosts
  unchanged.
- MockIdeBridge gives every reference panel a CI-runnable integration
  test path before VSCode is launched.

### Negative

- Editor-in adapter is substantial: six topics × VSCode API surface
  × rate-limiting × workspace-root awareness is ≥ two weeks of
  implementation + diagnostics work.
- Capture-grade `substrate/editor/terminal` expands governance over
  M1's read-only RPC. Capture-privacy invariants (ADR-012) now cover
  terminal streams; tray-chip rule applies.
- Per-invocation prompts on destructive affordances add friction.
  `allow-session` partly mitigates, but sovereign sessions are long-
  lived. Open question: is there a safe `run-command` subset
  (e.g. `workbench.action.*`) that can bypass the prompt?
- Chrome-match ships last (M1.9). Early-M1.6 panels look like egui,
  not Cursor. Acceptable — readable either way.
- Two envelope families (`rpc-request` + `wsp`) on the same channel
  is a mild smell. Accepted for continuity; a future M2 could unify.

### Neutral

- `ide.run-command` opens every VSCode command. Governance is the
  gate; no safe-command allowlist curated here — deferred to usage.
- Follow-editor rules live in manifests (ADR-015); this ADR only
  specifies the consumed topic and the DSL skeleton.
- JetBrains / Neovim sketches are non-committing. Protocol is ready
  when the relay is.

## Alternatives considered

1. **One omnibus `ide.*` RPC method with nested verb argument** —
   rejected. Loses the WSP topic/affordance distinction; subscribes
   would have to be polyfilled as long-poll RPC; streams don't fit.

2. **Editor-in topics as `workspace/*` not `substrate/editor/*`** —
   rejected. `workspace` is reserved for the filesystem-root adapter
   (session-10 §3.3). Editor state is not the workspace root; the
   `/editor/` namespace keeps them distinct.

3. **Put `editor/theme` under a separate `substrate/ui/*` tree** —
   rejected. Theme is editor-hosted data (Cursor's theme, not
   WeftOS's). Grouping with other editor-in topics keeps one
   handshake, one capability list.

4. **Make follow-editor a first-class compositor verb** — rejected.
   The compositor renders; the app declares what it wants rendered
   when. A manifest rule is the right layer (ADR-015).

5. **Skip the capability handshake; fail per-topic** — rejected.
   Surfaces need to know at mount time what to build against (e.g.
   hide a terminal widget on a host that doesn't expose terminals).
   A handshake is cheaper than probing.

6. **Expose VSCode's entire extension API as a thin wrapper** —
   rejected. Seven affordances cover session-10 §6's reference panels
   and are curatable. The full API is thousands of entry points; no
   useful governance model exists at that scope.

7. **Governance gate at the extension, not the compositor** —
   rejected. The compositor owns attention; it owns the prompt. An
   extension-level gate bypasses ADR-008.

## Related

- **Sessions**: `session-10-app-layer.md` (§4 names the four
  components; §7 schedules M1.6–M1.9; §8 recs 3 and 8 commit scope),
  `session-7-dev-panel-embedding.md` (predecessor webview-constraint
  research).
- **ADRs**: `adr-011` (embedding hybrid — M0/M1 predecessor this
  builds on), `adr-006` (primitive head — extended by ADR-019 with
  `invocations`), `adr-008` (goals / consent — governance reuse),
  `adr-012` (capture privacy — now covers `editor/terminal`),
  `adr-015` (app manifest — consumes follow-editor DSL), `adr-017`
  (ontology adapters — specifies how the `workspace` adapter
  *produces* the topic shapes this ADR *specifies*), `adr-019` (input
  adaptation — `invocations` on affordances).
- **Protocol spec**: §stream verbs (`open / chunk / commit / cancel`)
  — used for `substrate/editor/terminal` and the `wsp-stream`
  envelope.
- **Code**: `extensions/vscode-weft-panel/src/extension.ts` (gains
  `wsp` envelope family in M1.6), `crates/clawft-gui-egui/src/shell/*`
  (gains follow-editor evaluator M1.6, `theme::apply_ide_palette`
  M1.9), `crates/clawft-core/src/*` (ADR-017's workspace adapter
  implementation).
