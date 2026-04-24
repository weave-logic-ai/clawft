# Session handoff — 2026-04-24

Pick-up doc for the next session. Reflects `development-0.7.0` at commit `eb4cd9d8`.

---

## Right this second — the open loop

**User is about to `Developer: Reload Window` in Cursor to pick up a freshly-compiled extension (`out/extension.js`) that now includes `substrate.list` in `ALLOWED_METHODS`. Without the reload, every tree expansion is blocked by the extension-host allowlist check and the Explorer panel shows "substrate empty."**

### What the user should do on return

1. `Ctrl+Shift+P` → **"Developer: Reload Window"** — NOT "Reload Webviews." The allowlist lives in the extension-host process, not the webview; only a full window reload re-loads `extension.js`.
2. Open the WeftOS panel.
3. Expected tree state (after ~1 s of daemon warmup):
   ```
   ▸ substrate
     ▾ sensor
         • mic  (has_value, live tick)
   ```
4. Click `substrate/sensor/mic` → detail pane renders `AudioMeterViewer` with live MEMS dB bars (silence ~ -57 dBFS, speech ~ -10 dBFS). Above it, Phase 2's `[Audio Stream]` Object Type badge.
5. Tap the mic → bars should move within a frame.

### If the tree is still empty after reload

Three failure modes, in order of likelihood:

1. **Cursor cached the old extension.** Try `Developer: Reload Window` a second time, or `Developer: Restart Extension Host`.
2. **Daemon isn't answering.** `ps -p 13059` → should be `/home/aepod/.cargo/bin/weaver kernel start --foreground`. If dead, restart: `nohup /home/aepod/.cargo/bin/weaver kernel start --foreground > .weftos/runtime/kernel.log 2>&1 &`.
3. **RPC broken.** Direct probe:
   ```bash
   echo '{"jsonrpc":"2.0","method":"substrate.list","params":{"prefix":"substrate","depth":1},"id":"t"}' | timeout 5 nc 127.0.0.1 9471
   ```
   Expect: `{"ok":true,"result":{"children":[{"path":"substrate/sensor",...}],...}}`. If `"unknown method"` comes back, the daemon is the old binary — re-run `cargo install --path crates/clawft-weave --force` and restart.

### If that works — open TODOs

- Re-run `scripts/build.sh gate` end-to-end for the explicit clippy-clean + all-tests-passing verdict. The gate was interrupted mid-run for the daemon rebuild; not yet verified post-fix.
- Push `development-0.7.0` to origin (currently 20+ commits ahead).
- Triage `.planning/clients/`, `.planning/eml/`, `.planning/sensors/` untracked files — some of these were swept into `fix/canon-response-windows-deadlock`'s polluted commit (37k files) and deserve a clean curated import separate from the canon fix.
- Delete stale feature branches (`phase1-*`, `phase2-*`) once confident nothing needs rolling back from them.
- ADR-0006..0009 from the RLM symposium (see prior handoff) still blocked on user-answered Q1–Q8 in `.planning/symposiums/RLM - arxiv-2512.24601/00-synthesis.md`.

---

## What shipped this session (2026-04-23 evening → 2026-04-24 morning)

### Phase 0 completed first — context

Prior handoff's open loop: verify that adding `substrate.read` / `substrate.subscribe` to the VSCode extension allowlist fixes tray chip icons + mic gauge.

Outcome: **confirmed working.** Committed as:
- `00176c53` fix(vscode): allow substrate.read/subscribe through webview proxy
- `d053f22e` docs(handoff): Phase 0 allowlist fix + MEMS mic context

### Phase 1 — Explorer MVP (merged)

`.planning/explorer/PROJECT-PLAN.md` §3. Three-track parallel build in git worktrees:

- **Backend** (`phase1-explorer-backend` → `a96723ed`): `substrate.list` RPC — prefix + depth → `{ children: [{ path, has_value, child_count }], tick }`. Added to `SubstrateService`, wired through `clawft-weave/src/daemon.rs` dispatch, allowlisted in `extension.ts`. Capture-tier egress gates hide capture children from anonymous callers. 14 unit + 1 integration test.
- **Panel** (`phase1-explorer-panel` → `adb8a980`): Explorer panel skeleton (left tree + right detail), `SubstrateViewer` trait + dispatch registry with marker-comment insertion pattern for future tracks, `JsonFallbackViewer` (type badges + expand/collapse, priority 1 catch-all), mounted in both launcher button + the pre-existing Explorer tray chip. Subscription lifecycle drops cleanly on path-switch and panel-close.
- **Viewers** (`phase1-explorer-viewers` → `441a97cf`): AudioMeterViewer (INMP441 MEMS-sized `-65…-5 dBFS`), ConnectionBadgeViewer, DepthMapViewer. 28 unit tests.

### Phase 2 — five parallel tracks (merged)

`.planning/explorer/PHASE-2-PLAN.md`. Wave A then Wave B:

**Wave A:**
- **Object Types** (`phase2-object-types` → `f3ee88d1`): `ObjectType` trait, same `matches(value) -> u32` priority cascade pattern lifted one level above viewers. `PropertyDecl` schema, `ObjectTypeCapabilities` slots (empty today — shape reserved for Actions / events / default-viewer-hint). First concrete types: `Mesh` (root, heuristic top-section match), `AudioStream` (shape-overlaps AudioMeterViewer), `ChainEvent` (array-of-event stream). Explorer detail pane renders a `[TypeName]` badge above the value when `ontology::infer()` returns Some. 39 unit + integration tests.
- **Whisper spike** (`phase2-whisper-spike` → `27ad00e5`): new crate `clawft-service-whisper`. **Pivot mid-session from FFI to HTTP client** — user pointed at `~/llama.cpp/docs/whisper-service-api.md`, local whisper.cpp is running as an HTTP service. New architecture: `reqwest` multipart POST to `http://127.0.0.1:8080/inference` (configurable via `WHISPER_SERVICE_URL`), WAV-wraps b64-encoded PCM chunks from `substrate/sensor/mic/pcm_chunk`, publishes transcripts to `substrate/derived/transcript/mic`. Drop-oldest backpressure (whisper service is 1-in-flight-per-instance, no 429). 29 hermetic tests (wiremock). `.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md` (3518 words) — provisional primitive shape: `SensorStage { id, input_topics, output_topics, placement, ready, buffer_policy, run }`. HTTP-stage framing surfaced **placement** (in-proc / sidecar / remote HTTP) as a first-class axis that FFI framing would have hidden.

**Wave B:**
- **Viewer growth** (`phase2-viewers` → `f5b82889`): all 5 queued viewers shipped. WaveformViewer (priority 15, inline line render), MeshNodesViewer (12), ChainTailViewer (12), TimeSeriesViewer (5, history in `Mutex<HashMap<String, Vec<f64>>>` bounded to 240/path — `SubstrateViewer` stateless tension noted), ProcessTableViewer (12, matches both `kernel.ps` shape and generic `name/cpu/mem`). 54 new viewer tests.
- **`ui://graph` primitive** (`phase2-ui-graph` → `cf8ae3e7`): GraphViewer — Vertex analog. **Library decision: roll-own** (no `egui_graphs`, no `egui_node_graph`). Read-only MVP, ~300 LOC, tolerates multiple node/edge shape variants (typed objects, bare-id arrays, pair-array edges, string OR numeric ids, `[node_id, port_idx]` endpoint form for ⊃μBus cables). Circular layout fallback; honors supplied `pos`. `matches_vector_synth_cable_shape` test confirms ⊃μBus patch graph projects cleanly. 31 unit tests.
- **Workshop hot-reload** (`phase2-workshop` → `dc132499`): the **vector-synth iteration unblocker** per ADOPTION.md §8 Step 3. Workshop schema at `substrate/ui/workshop/<name>` — `{ title?, layout?, panels: [{ substrate_path, title?, viewer_hint?, min_height? }] }`. GUI subscribes; republish causes live reconfigure. Shape-match mount (no path whitelist). TOML file-watcher binary at `crates/clawft-gui-egui/examples/workshop-watcher.rs`. Working fixture at `examples/example-workshop.toml`. 18 unit + integration tests.

### Canon deadlock fix — surgical cherry-pick

The user's `fix/canon-response-windows-deadlock` branch had commit `7d80c0af` that contained the genuine canon fix (15 lines in `crates/clawft-gui-egui/src/canon/response.rs`) **plus 37,314 files / 5.5M insertions of unrelated auto-staged content** (full `node_modules/`, `.planning/clients/*` binaries, sensor catalog markdown, `docs/runtime-scratch/chain.json` at 125k lines, etc.) — the result of a wayward `git add -A`. Cherry-picking the whole branch would drag all that in.

Solution: surgically extract only the canon response.rs diff + the clean repro-test commit.

- `b5ed97f4` fix(canon): prevent ctx.input reentrancy deadlock in CanonResponse::from_egui (authored as Mathew Beane, original date preserved)
- `b4515572` test(canon): add standalone Windows deadlock repro for from_egui (cherry-picked `0fe4eaa0`, clean)

Bug: `Response::drag_delta()` internally calls `ctx.input(...)`. Calling it from inside an outer `ctx.input(|i| ...)` nests two read-locks on egui's `Context` RwLock. Linux futex permits recursion; Windows SRWLock deadlocks. Fix: read `inner.drag_delta()` into a local before entering the closure.

The pollution on `fix/canon-response-windows-deadlock` is NOT brought in. That branch stands on its own; `development-0.7.0` has only the intended code change + test.

### WASM Instant-underflow fix

`eb4cd9d8` fix(explorer): guard Instant-subtraction against WASM time-origin panic.

Symptom: webview loaded, then crashed on first repaint with
```
panicked at web-time-1.1.0/src/time/instant.rs:101:14:
  overflow when subtracting duration from instant
```

Root cause: Phase 1 panel + Phase 2 Workshop code initialised `last_poll` / `last_slow_tick` fields via `web_time::Instant::now() - POLL_INTERVAL * 2` to trick "first poll fires immediately." Works on native (Linux `CLOCK_MONOTONIC` is huge, time-since-boot). Panics on WASM because the browser time-origin is when the tab loaded — early in the page lifecycle `Instant::now()` is less than `POLL_INTERVAL * 2` and the unchecked subtraction underflows.

Fix at three sites: `.checked_sub(dur).unwrap_or_else(Instant::now)`. WASM saturates to "first poll one interval later instead of immediately" — sub-second UX degradation, no crash.

- `crates/clawft-gui-egui/src/explorer/mod.rs:67` (SubscriptionHandle::new)
- `crates/clawft-gui-egui/src/explorer/mod.rs:145` (Explorer::default)
- `crates/clawft-gui-egui/src/explorer/workshop.rs:298` (PanelSub::new)

---

## Runtime state

### Daemon

- **Binary**: `/home/aepod/.cargo/bin/weaver` — rebuilt and `cargo install --path crates/clawft-weave --force`'d this session.
- **Process**: `/home/aepod/.cargo/bin/weaver kernel start --foreground` — PID to check via `pgrep -af 'weaver kernel'`. Last known PID during the session: 13059.
- **Unix socket**: `.weftos/runtime/kernel.sock`
- **TCP relay**: `127.0.0.1:9471` (per `[kernel.ipc_tcp]` in `weave.toml`).
- **Log**: `.weftos/runtime/kernel.log` (log appended via `nohup ... > log 2>&1 &`).
- **Substrate tick**: climbing (~300+ at restart time). INMP441 MEMS mic published — `substrate.read substrate/sensor/mic` returns live `rms_db` in `-65…-5 dBFS` range.

### Extension

- **Source**: `extensions/vscode-weft-panel/src/extension.ts` — `ALLOWED_METHODS` includes `substrate.read`, `substrate.subscribe`, `substrate.list` (line 63).
- **Compiled**: `extensions/vscode-weft-panel/out/extension.js` — **freshly recompiled this session** via `npm run compile`. Gitignored artifact; regenerate if you pull on another machine.
- **WASM bundle**: `extensions/vscode-weft-panel/webview/wasm/clawft_gui_egui_bg.wasm` — rebuilt after the Instant-underflow fix (5.77 MB, ~10:25 UTC). Gitignored; regenerate with `extensions/vscode-weft-panel/scripts/build-wasm.sh`.
- **Cursor install**: extension is registered at `~/.cursor-server/extensions/extensions.json` via the repo path (`source: "resource"`, `fsPath: /home/aepod/dev/clawft/extensions/vscode-weft-panel`). Cursor loads it live from the repo; the extension host needs a window reload to pick up a new `out/extension.js`.

### Branches

- `development-0.7.0` — tip `eb4cd9d8`. **Not pushed.** 20+ commits ahead of origin.
- `fix/canon-response-windows-deadlock` — user's branch, commits `7d80c0af` (polluted) + `0fe4eaa0` (clean). Not merged; the genuine code change was cherry-picked surgically. Branch can be left alone or deleted once confident.
- `phase1-explorer-{backend,panel,viewers}`, `phase2-{object-types,whisper-spike,viewers,ui-graph,workshop}` — feature branches preserved locally, worktrees cleaned up. Safe to delete.

---

## Key architectural decisions this session

Read in order for context:

- **`.planning/ontology/ADOPTION.md`** — adopt Palantir Foundry architecture on top of substrate. Preserve Foundry vocabulary (Object, Object Type, Property, Link Type, Interface, Action, Function, Ontology Manager, Vertex, Machinery, Workshop). One motivated local rename: **Mesh as root Object**. Core inference rule: "data shape defines the interface." Governance as slot-not-fill. Seven-step staircase (Explorer → Object Types → **Config-driven hot-reload** → Manual drag-drop → Event wiring → Usage telemetry → LLM composer).
- **`.planning/ontology/palantir-foundry-research.md`** — 3972-word grounded research. Three load-bearing findings: (1) Foundry ontology is 1:1 with Space (coupled to workspace construct — validates "one mesh = one Object"); (2) Actions are the sole sanctioned mutation path, each a single transaction; (3) writebacks are pipeline-mediated + eventually consistent. Verdict: partially fits — adoptable through semantic + kinetic; diverges at closed-world identity vs open-namespace substrate (resolved by Mesh scoping).
- **`.planning/sensors/PIPELINE-PRIMITIVE-SPIKE.md`** — whisper as a probe for the primitive, not a deliverable. 5 boundaries × 9 axes × 6 concrete questions.
- **`.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md`** — the whisper-build journal answering all of the above. Provisional shape: `SensorStage { id, input_topics, output_topics, placement, ready, buffer_policy, run }`.
- **`.planning/explorer/PHASE-2-PLAN.md`** — the five-track Phase 2 sequencing that produced this session's merges.

---

## Open loops carried forward

1. **Full `scripts/build.sh gate` post-fix** — not yet verified green end-to-end. The gate was interrupted twice (once for mid-session fix, once for daemon rebuild). Workspace-scoped tests passed 136/136 on explorer, check + clippy clean at tip; gate's `wasi` / `browser` / `fmt` / `doc` / `audit` steps still unverified this session. Run as next step if you want clean ritual pre-push verification.
2. **Push to origin** — 20+ commits ahead.
3. **Whisper end-to-end against live service** — `clawft-service-whisper` tests are hermetic (wiremock). Running the real pipeline needs the daemon registering the service + the ESP32 (or the WAV test harness at `crates/clawft-service-whisper/examples/publish_wav.rs`) producing PCM on `substrate/sensor/mic/pcm_chunk`.
4. **ADR-0006 … ADR-0009** from RLM symposium, still blocked on Q1–Q8 answers.
5. **Second sensor** for the pipeline primitive — whisper is data point #1; the primitive proposal is blocked until a second sensor (camera or ToF) validates the axis list.
6. **Untracked `.planning/clients/`, `.planning/eml/`, `.planning/sensors/*.md`** — legitimate content swept into the polluted canon commit. Needs a separate curated import.
7. **Delete stale feature branches** once confident nothing needs rollback from them.
