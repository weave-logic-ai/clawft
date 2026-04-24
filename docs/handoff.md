# Session handoff — 2026-04-23 (evening)

Pick-up doc for the next session. Reflects `development-0.7.0` at commit `2156157`.

---

## Right this second — the open loop

**User was about to reload the WeftOS VSCode webview to test whether the just-applied Explorer Phase 0 allowlist fix brings the mic gauge + tray chip icons back to life. The result is unconfirmed as of session end.**

### What's in the working tree (uncommitted)

One file: `extensions/vscode-weft-panel/src/extension.ts` — added `substrate.read` and `substrate.subscribe` to the `ALLOWED_METHODS` Set around line 39. `npm run compile` ran and emitted fresh `out/extension.js` (tsc prints pre-existing `node:*` module errors in `rpc.ts` but emits anyway — unrelated, longstanding). The allowlist change is verified present in the compiled output (`grep -c 'substrate.read' out/extension.js` → 2).

### What the user should do on return

1. `Ctrl+Shift+P` → "Developer: Reload Webviews" (or close+reopen the WeftOS panel tab) — this picks up the fresh extension.js.
2. Watch the tray chips:
   - **Kernel** — green before, still green
   - **Mesh** — icon should flip from grey to green, panel body should show `total_nodes`/`healthy_nodes`/shards
   - **ExoChain** — icon green, panel body shows chain head + recent events
   - **Explorer** — green (tracks daemon connection); detail window shows a placeholder until Phase 1 lands
3. Click the Mesh or ExoChain chip → the outside icon-vs-inside-green-dot disagreement from earlier should be gone
4. Tap the piezo — the mic gauge is **GONE from the tray** now (Audio chip was retired), but substrate data still flows. To see the gauge you'd need to drive it through the Explorer MVP — **which isn't built yet**, only planned. So there's no visual mic feedback in the GUI this session. The RMS stream is live on `substrate.read` though (`substrate/sensor/mic` ticks climb, rms_db moves in the `-43…-16 dBFS` range).

### If the allowlist fix works

1. Commit the 2-line change with message like `fix(vscode): allow substrate.read/subscribe through webview proxy`
2. Go answer Q3 + Q6 inline in `.planning/symposiums/RLM - arxiv-2512.24601/00-synthesis.md` — those unblock ADR-0006 (substrate as prompt handle) + ADR-0007 (metadata-only tool history), both of which are prerequisites for the Explorer MVP and for RLM Option D.

### If the allowlist fix doesn't work

The diagnostic path is at the bottom of `.planning/explorer/PROJECT-PLAN.md` §2 ("If Phase 0 does NOT fix Mesh/Chain/Mic icons"). Short version: add a temporary `tracing::debug!` at the first substrate.read call site in the WASM `Live` loop (`crates/clawft-gui-egui/src/live/native_live.rs` around line 280+ — exact line may have drifted after the chip refactor). Check the webview devtools console to see whether the call even fires.

---

## What shipped this session (2026-04-23 midday → evening)

### Commit `2156157` — `refactor(gui,mic): explorer chip + mic race fix + clippy sweep`

Landed on `development-0.7.0`. 120 files, +1607/−994. Not yet pushed.

1. **Mic race fix** (`crates/clawft-substrate/src/mic.rs`) — the actual change that resolved the open loop from the prior handoff. Local `MicrophoneAdapter.poll_level` now skips publishing `{available: false, reason: "source-missing"}` when the backing file is absent, so ESP32-bridge-published values at `substrate/sensor/mic` don't get stomped every 500 ms.
2. **Tray refactor** (`crates/clawft-gui-egui/src/shell/{tray,desktop}.rs`) — retired Wi-Fi / Bluetooth / Audio / ToF chips. Kept Kernel / Mesh / ExoChain and added **Explorer** chip at the far right (user's call). Adapter wiring + `Snapshot` fields stay intact; the chip TOML fixtures on disk stay too — they'll come back as viewer patterns in the Explorer's pattern registry.
3. **Workspace-wide clippy sweep** — ~100 files touched, mix of `cargo clippy --fix` autos (inlined format args, needless lifetimes, `&PathBuf` → `&Path`, derivable_impls, strip_prefix, div_ceil, collapsible let-chains) and targeted manual fixes (math-heavy `needless_range_loop` in `clawft-kernel/src/causal.rs` got a file-level allow; `too_many_arguments` got targeted allows; scaffold dead-code got `#[allow(dead_code)]` with rationale). Prior handoff listed this as open TODO #5 — **closed**.
4. **Planning artifacts** — `.planning/explorer/PROJECT-PLAN.md` (Explorer MVP plan, phase 0 + phase 1 + viewer pattern growth) and `.planning/symposiums/RLM - arxiv-2512.24601/` (5-doc panel review of arXiv:2512.24601 "Recursive Language Models" — see next section).

### RLM Option D selected (2026-04-23)

Full panel synthesis at `.planning/symposiums/RLM - arxiv-2512.24601/00-synthesis.md`. Decision:

- **Option D** — full Rust-native RLM scaffold. `llm_query` wrapped as a gated tool at `clawft-kernel/src/chain.rs`. Per-trajectory budget `{max_depth:1, max_subcalls:32, token_cap:200K}`. Sub-call events on ExoChain Stream via new `RollingWindowAnchor` aggregator. No Python REPL. Rust-native loop driver in `crates/clawft-core/src/agent/rlm.rs`. ~4-5 engineer-weeks.
- **Cross-cutting constraint (§4.5): channel-generality.** Every primitive — `substrate.describe`, `substrate.project`, `tree.peek`/`search`/`slice`, `llm_query`, viewer registry, trajectory event schema — must work uniformly across all substrate paths. No mic-specialized code, no tof-specialized code. Schema-match in the viewer, not the loop. Explorer MVP shares these primitives; the Explorer is effectively Option D's first visible artifact.
- **4 ADRs to write** before implementation begins:
  - **ADR-0006** Substrate as Prompt Handle — blocked on Q6 answer
  - **ADR-0007** Metadata-Only Tool History — blocked on Q3 answer
  - **ADR-0008** Bounded Recursive Sub-Calls — blocked on Q2, Q4 answers
  - **ADR-0009** RLM Trajectory Event Schema — blocked on Q2, Q5, Q7 answers
- **Q1-Q8** are in §5 of the synthesis. The user needs to answer them inline. Recommended start order: Q3 + Q6 first (low-contention), then Q2/Q4/Q5/Q7.

### Clippy-adjacent workflow gotchas the session surfaced

Now persisted in memory (`~/.claude/projects/-home-aepod-dev-clawft/memory/`):

- `build_script_test_exit_code.md` — `scripts/build.sh test` masks cargo's exit code. Use `cargo test --workspace` directly for commit gating.
- `clawft_rpc_no_daemon_tests.md` — `client::tests::connect_returns_none_when_no_daemon` and `is_daemon_running_false_when_no_daemon` fail when a daemon is running. Run with `WEFTOS_RUNTIME_DIR=/tmp/nonexistent-$$` to isolate.
- `wsl_xdg_open_test_hang.md` — some test fires `xdg-open http://localhost:99999`, hangs in WSL. Set `BROWSER=/bin/true` before the test run.
- `weaver_binary_install_path.md` — `~/.cargo/bin/weaver` (what the daemon-launch resolves to via $PATH) drifts from `target/release/weaver` (what `scripts/build.sh native` writes). `cargo install --path crates/clawft-weave --force` to refresh.

---

## Running state (end of session)

### Daemon

- **PID 43247** — `weaver kernel start --foreground` from the refreshed `~/.cargo/bin/weaver` (installed mid-session after we diagnosed the stale-binary drift). Alive and publishing.
- **Unix socket** `.weftos/runtime/kernel.sock`
- **TCP relay** `127.0.0.1:9471` (bound on `0.0.0.0:9471` in fact — ESP32 bridge at `192.168.1.178` has two ESTAB connections open as of last check)
- **Kernel log** `.weftos/runtime/kernel.log` — only DEMOCRITUS stuck-phase warnings in normal operation, no real errors

### ESP32 bridge

Publishing live `substrate/sensor/mic` values. `tick` climbs, `rms_db` in `-43…-16 dBFS` range. Values confirmed via direct `substrate.read` on `127.0.0.1:9471`.

### Repo

- Branch: `development-0.7.0` (tracking origin)
- Last commit: `2156157 refactor(gui,mic): explorer chip + mic race fix + clippy sweep`
- Not pushed — user's call when to `git push origin development-0.7.0`

### Uncommitted / untracked

```
M extensions/vscode-weft-panel/src/extension.ts    ← Phase 0 allowlist fix
(plus out/extension.js + out/rpc.js rebuilt but those are build artifacts)

?? .planning/clients/                              ← pre-existing
?? .planning/eml/                                  ← pre-existing
?? .planning/sensors/                              ← pre-existing
?? .planning/symposiums/exochain-logging/          ← pre-existing (prior session's symposium)
?? .planning/weftos_sensors.md                     ← pre-existing
?? bun.lock
?? docs/runtime-scratch/                           ← gitignore candidate
?? node_modules/
```

---

## TODOs queued (priority order)

1. **Verify Phase 0** — user reloads webview; confirm chip icons go green, Mesh/Chain panel bodies populate. Commit the 2-line extension.ts change after confirmation.

2. **Answer Q3 + Q6 in the RLM synthesis** — low-contention, unblocks ADR-0006 + ADR-0007 in parallel. Those two ADRs are the Explorer MVP's backbone AND Option D's foundation primitives. Best ROI per unit of user-decision time.

3. **Explorer Phase 1 MVP** (after ADR-0006 lands). Plan at `.planning/explorer/PROJECT-PLAN.md` §3. Needs a new `substrate.list` daemon RPC (~20 lines), a tree-left-viewer-right egui panel, and 4 MVP viewers (AudioMeter, ConnectionBadge, DepthMap, JsonFallback). ~1-2 engineering days.

4. **Answer Q2 / Q4 / Q5 / Q7** — critical path for ADR-0008 + ADR-0009. These are harder than Q3/Q6 (touch governance, cost-tail cancellation, replayability, mesh replication).

5. **RLM implementation** (after all 4 ADRs). ~4-5 engineer-weeks of code in `crates/clawft-core/src/agent/rlm.rs` + `clawft-kernel/src/chain.rs`. Do not start before ADR-0008 lands — ungated `llm_query` is a non-starter per gaps-and-risks §3.1.

6. **ExoChain logging redesign Phase 0 ADRs** (inherited from prior handoff). 5 ADRs listed in `.planning/symposiums/exochain-logging/00-synthesis.md` §12. Interacts with RLM Option D's channel-routing question (Q2); worth settling Q2 first so ADR-0005 (ExoChain) and ADR-0009 (RLM trajectories) agree on how sub-call events land on Stream.

7. **`weaver substrate` CLI subcommands** — inherited from prior handoff. ~half day. Lower priority now that Explorer absorbs the interactive case, but still useful for scripting. Template is `crates/clawft-weave/src/commands/ipc_cmd.rs`.

8. **Quality-of-life fixes** (each ~5-30 min, each removes a recurring footgun):
   - `scripts/build.sh test` should propagate cargo's exit code
   - `clawft-rpc` "no daemon" tests should be hermetic
   - WSL xdg-open test hang — find + fix the offending test
   - `scripts/build.sh install` command so `cargo install --path crates/clawft-weave --force` has a memorable name

9. **53 dependabot vulns on origin/master** (inherited). Untouched. Separate `chore(deps)` PR candidate.

---

## Test commands that work right now

```bash
# Workspace compile check — green
scripts/build.sh check

# Clippy — now workspace-clean as of 2156157 (was the open-debt TODO from prior handoff)
scripts/build.sh clippy

# Full tests (needs env overrides — see gotchas above)
WEFTOS_RUNTIME_DIR=/tmp/nonexistent-weftos-$$ BROWSER=/bin/true cargo test --workspace

# Targeted — the two daemon-flaky tests pass when WEFTOS_RUNTIME_DIR points elsewhere
WEFTOS_RUNTIME_DIR=/tmp/nonexistent cargo test -p clawft-rpc --lib

# Mic tests (9 green)
cargo test -p clawft-substrate --lib mic::

# Daemon-side sanity — confirms bridge is publishing
echo '{"jsonrpc":"2.0","id":"t","method":"substrate.read","params":{"path":"substrate/sensor/mic"}}' | \
  nc -q1 127.0.0.1 9471

# Check the running daemon matches what you just built
diff -q target/release/weaver ~/.cargo/bin/weaver && echo "in sync" || echo "RESTART NEEDED"
```

---

## Architecture notes — what changed since the prior handoff

### Tray chip model

- **Before:** 7 chips (Kernel, Mesh, ExoChain, Wi-Fi, Bluetooth, Audio, ToF). Each with a bespoke TOML fixture and per-sensor status logic in `shell/tray.rs`.
- **After:** 4 chips (Kernel, Mesh, ExoChain, Explorer). Explorer at the far right, status = daemon connection. Wi-Fi/BT/Audio/ToF adapter wiring and `Snapshot` fields kept; only their tray surface is gone. TOML fixtures on disk for the retired chips are unused-but-kept (will be resurrected as viewer patterns per `.planning/explorer/PROJECT-PLAN.md` Phase 2).

### The VSCode proxy allowlist was the real gate

The original mic-gauge problem wasn't the mic.rs race (that was real and got fixed too), but the fact that the VSCode extension's RPC proxy allowlist didn't include `substrate.read` / `substrate.subscribe`. The WASM `Live` loop running inside the webview uses substrate subscriptions to populate `Snapshot` — same code the native GUI runs. Blocked at the proxy = all substrate-driven chip icons and gauges stayed dark regardless of daemon health. Phase 0 of the Explorer plan fixes this in 2 lines.

### Mic path (unchanged from prior handoff but worth re-linking)

See prior handoff's "How mic data flows" section for the 8-step chain from ESP32 INMP441 → bridge RMS math → daemon substrate → GUI snapshot → detail window gauge. Still accurate. The only thing that changed is the tray Audio chip was retired, so you no longer see a 3-state lamp in the tray — you'd see the full meter in the Explorer once Phase 1 ships, OR in a direct `substrate.read` loop.

---

## Files to be aware of (so you don't accidentally clobber untracked work)

- `.planning/symposiums/exochain-logging/` — 5 MD files from prior session. Still uncommitted.
- `.planning/symposiums/RLM - arxiv-2512.24601/` — 5 MD files from THIS session, now committed in `2156157`. Safe to edit, but note that Q1-Q8 in `00-synthesis.md` §5 are meant for the user to answer inline.
- `.planning/explorer/PROJECT-PLAN.md` — committed in `2156157`. Update it as Phase 1 progresses.
- `.planning/sensors/` — 12-category sensor taxonomy, user-authored, still untracked.
- `.planning/clients/esp32-bridge-rpc.md` — ESP32 bridge RPC contract, source of truth, still untracked.
- `.planning/weftos_sensors.md` — still untracked.
- `docs/runtime-scratch/chain.json` — scratch data; gitignore candidate.

---

## Who's in charge of what

- **User (Mathew):** driving design decisions, running the physical ESP32 hardware, operating the VSCode webview as the primary GUI surface. Picks commit shapes (usually one bundled commit; confirmed twice this session). Renames symposium dirs to include topic shorthand (`RLM - arxiv-NNN`, not bare `arxiv-NNN`).
- **This Claude session:** mic race fix, tray refactor, workspace clippy sweep (~100 files), RLM symposium coordination + synthesis + decision-recording, Explorer PROJECT-PLAN draft, Phase 0 extension.ts fix. Wrote 7 memory files covering the dev-env gotchas + project direction + collaboration prefs.
- **Background agents this session:**
  - 4 specialists for the RLM arxiv symposium (all delivered + committed)
  - 1 synthesis agent for the RLM panel (delivered + committed, edited by this session to record Option D + channel-generality constraint)
  - Test-suite background runs — all completed before end of session.
  - None are still running.

---

## Pick up here

**Highest priority:** confirm whether the Phase 0 allowlist fix works — user should reload the webview as first act of the next session. If yes → commit the 2-line change and move on to answering Q3/Q6 so ADR work can start. If no → walk the diagnostic path at the bottom of `.planning/explorer/PROJECT-PLAN.md` §2.

If the user is done for the day, the commit in `2156157` is stable and the workspace is in a known-green state (clippy clean, daemon healthy, all tests pass with the env overrides noted). `git push origin development-0.7.0` is safe whenever they decide to land it on the remote.
