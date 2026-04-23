# Session handoff — 2026-04-23

Pick-up doc for the next session. Reflects `development-0.7.0` at commit `2b33b10` (up-to-date with `origin/master` after v0.6.19 merge).

---

## Right this second — the open loop

**User just rebooted to test a fix. They're expecting the GUI audio meter to start tracking ESP32-bridge-published RMS data on their return.**

### What's fixed (uncommitted, in working tree)
- `crates/clawft-substrate/src/mic.rs` — `poll_level` now skips publishing `{available: false, reason: "source-missing"}` when the backing file `/tmp/weftos/mic/stream.raw` is absent. Before the fix, the local `MicrophoneAdapter` was publishing this every 500 ms into the same `substrate/sensor/mic` path that the relay poller injects bridge data into — last-writer-wins race, GUI gauge couldn't bind to `rms_db` because the local adapter periodically overwrote the relayed value. After the fix the local adapter is silent when there's no source file.
- 9/9 mic tests still pass; `scripts/build.sh check` green.

### What the user will do on return
1. `scripts/build.sh native` — rebuild with the fix
2. Restart whichever GUI they're running (VSCode extension reload, or kill + relaunch `weft-demo-lab`, or however the weaver panel is hosted)
3. Tap the piezo. **Expected:** detail-window `ui://gauge` bound to `$substrate/sensor/mic.rms_db` tracks bridge values in the `-43 … -16 dBFS` range.
4. If that works → commit the mic.rs fix, then start the TODO below.
5. If it doesn't → the sequence of downstream debug steps below (temporary `eprintln!` at the relay-inject site in `native_live.rs:333`, confirm relay is landing).

### Confirmed healthy before the restart
Daemon side is 100% fine — user ran `substrate.read` twice via TCP relay (`[kernel.ipc_tcp]` enabled on 127.0.0.1:9471):

```
tick 109 → 114   rms_db -36.1 → -41.6   peak_db -17.4 → -18.3
available: true  sample_rate: 16000  samples_in_window: 15360
```

`tick` climbs, values move. Bridge → daemon pipeline is solid. Everything left is GUI-side rendering.

### ESP32 bridge status (from the user's own debug narration earlier)
The user fixed the RMS math on the bridge side. Root cause there: missing variance formula (`rms² = E[s²] - E[s]²`) — their DIY piezo front-end has no DC bias resistor so the mean drifts between -351 and +4904, and squaring-without-mean-subtraction drowned out the audio modulation. After the variance fix the bridge reports honest `-43 … -27 dBFS` with real dynamic range. They also sped up the IIR filter (was `4095/4096` ≈ 0.25 s τ — too slow to track a piezo-with-no-bias-resistor's drift). That code lives in `mic_capture.c` in their separate `~/dev/vector-synth/sources/musica` repo — not in this repo.

---

## Running state (as of the reboot)

### Daemon

- **Last PID** `2406238` — dead after reboot. Check `.weftos/runtime/kernel.pid` + `pgrep -af clawft-weave` on return to see if it auto-started.
- **Unix socket** `/home/aepod/dev/clawft/.weftos/runtime/kernel.sock`
- **TCP relay** `127.0.0.1:9471` — user confirmed working (it's how they ran the `substrate.read` queries). Config: `[kernel.ipc_tcp] enabled = true` in `weave.toml`. Shipped in v0.6.19.
- **Kernel log** `.weftos/runtime/kernel.log` (6.6 MB at last check)

### Repo

- Branch: `development-0.7.0` (tracking origin, up-to-date with master post-0.6.19)
- Last commit on branch: `2b33b10 merge: origin/master into development-0.7.0 for v0.6.19 release`
- Master HEAD: `b9b439f Merge pull request #31` (publish-crates workflow fix)
- Tag: `v0.6.19` landed, GitHub release live at https://github.com/weave-logic-ai/weftos/releases/tag/v0.6.19, all 82 binary assets uploaded.

### Uncommitted

```
M crates/clawft-substrate/src/mic.rs      ← the fix above, not yet committed
?? .planning/clients/                      ← ESP32 bridge RPC doc + QSR client
?? .planning/eml/                          ← EML core plan + training findings
?? .planning/sensors/                      ← 12-category sensor taxonomy
?? .planning/symposiums/exochain-logging/  ← 4-specialist symposium + synthesis + PROJECT-PLAN
?? .planning/weftos_sensors.md
?? docs/runtime-scratch/                   ← `chain.json` scratch, gitignore candidate
```

---

## TODOs queued (in priority order)

1. **Commit the `mic.rs` race fix** once the user confirms the meter moves. Message sketch: `fix(mic): silence local adapter when source file missing to avoid overwriting bridge-published values`.

2. **Add `weaver substrate` subcommands.** User approved this earlier. Three methods needed (the daemon already exposes the JSON-RPC methods — this is pure CLI):
   - `weaver substrate read <path>` — one-shot read with `[--watch]` and `[--interval 1s]` flags
   - `weaver substrate publish <path> <json>` — one-shot publish for testing
   - `weaver substrate tail <path>` — subscribe and print deltas until Ctrl+C
   Pattern: copy `crates/clawft-weave/src/commands/ipc_cmd.rs` as a template, wire a new `SubstrateArgs` in `commands/mod.rs`, register the subcommand in `weaver`'s top-level `clap` tree. Scope: ~half a day.

3. **ExoChain logging redesign (ADRs Phase 0)** — the 4-specialist symposium landed a 270-line synthesis at `.planning/symposiums/exochain-logging/00-synthesis.md` and a 360-line project plan at `.planning/symposiums/exochain-logging/PROJECT-PLAN.md`. User answered all 5 open questions inline in the synthesis. Decisions locked:
   - 4 channels: `Governance / Fabric / Stream / Error` (user renamed `Diag` → `Error`)
   - `k_level` observe-only v1
   - Arrow IPC in Phase 4 (WELF deferred to 4b)
   - Retention as proposed (Gov forever / Fabric 30d+1y / Stream 7d+90d / Error 14d+180d)
   - Fatal → `RestartStrategyModel` with exponential backoff
   - Next step: write the 5 ADRs listed in §12 of synthesis (`docs/adrs/0001`…`0005`). That unblocks Phases 1-7 of the plan. Full dependency graph + ticket breakdown in `PROJECT-PLAN.md`.

4. **Pre-existing hung test** in `clawft-kernel --lib` aggregate suite. Targeted (`--lib cluster::tests::`, `--test stream_anchor_test`, etc.) all pass. One test spins forever when the whole `--lib` suite runs in aggregate. I killed a 65-min-old orphan mid-session. Not on the critical path but worth tracking.

5. **Workspace clippy debt** — ~150 pre-existing errors across `clawft-types/src/goal.rs`, `clawft-rpc`, `eml-core`, some older kernel/weave. `scripts/build.sh check` green; `scripts/build.sh clippy` red on pre-existing code. Candidate for a `chore(clippy-debt)` PR that does nothing but apply the suggestions.

6. **53 dependabot vulnerabilities** on the default branch per GitHub (5 critical / 5 high / 33 moderate / 10 low). Completely untouched. Candidate for its own `chore(deps)` PR.

---

## What shipped this session (2026-04-22 → 23)

All landed on master via PR #30 and PR #31 and tagged `v0.6.19`:

1. **`feat(ipc): optional TCP relay for daemon JSON-RPC socket`** (`f3c0e18`) — `[kernel.ipc_tcp] enabled = true` binds an additional TCP listener that transparently byte-copies to the unix socket. Windows/WSL bridges no longer need to open `AF_UNIX`. I didn't write this (it was uncommitted work on the branch); I committed it as part of the clean-up.
2. **`feat(cluster): persist peer membership across daemon restarts`** (`34442f1`) — Option C. `ClusterMembership` now persists to `.weftos/runtime/cluster_peers.json` via atomic tmp+rename on add/remove/state-change, rehydrates on boot. Fixes the "cluster: degraded - no healthy cluster nodes" regression that started this whole session. 3 new tests (`persisted_peers_rehydrate_on_restart`, `persist_reflects_remove_and_state_change`, `with_persist_path_missing_file_is_ok`).
3. **`fix(graphify): writer emits edges with source_file (closes #26)`** (`496271b`) — cherry-picked forward from v0.6.18 which was cut on an orphan commit that never reached master. Back-filled the 0.6.18 CHANGELOG entry too.
4. **`release: v0.6.19 — 0.7-line rollup onto 0.6 release train`** (`8fc14cb`) — 57 commits rolled up: M1.5 app-layer trilogy, canon primitives, sensor framework, ExoChain stream-anchor, 8 EML-swap wirings, `clawft-casestudy-gen-qsr` crate (renamed from `qfs-gen` by a parallel agent during the session).
5. **`fix(ci): drop mutually-exclusive flags from publish-crates workflow`** (PR #31, merged `b9b439f`) — `cargo-workspaces 0.3+` rejects `--no-git-commit` with `--no-git-push`/`--no-git-tag`. Every release since the upgrade silently failed the `Publish Crates` job. One-line fix, commented to prevent re-adding.
6. **`feat(casestudy-gen-qsr)`** (part of the release commit) — 42-file crate renamed from `clawft-qfs-gen` to `clawft-casestudy-gen-qsr`. QSR (Quick-Service-Restaurant) synthetic corpus generator. Scenarios: `geo_miss`, `labor_shock`, `promo_pull`, `store_closure`, `supply_disruption`, `weather_season`.

### Release trivia (for next time's sanity)

- **Don't manually `gh release create` before cargo-dist finishes** — cargo-dist's `host` job tries to create the release itself and dies with `a release with the same tag name already exists`. Either let cargo-dist own release creation (populate notes from the CHANGELOG match) or create it `--draft` and undraft after the workflow uploads.
- When cargo-dist fails at the host-upload step but all platform builds succeeded, the artifacts are still in the workflow's artifact storage. Download with `gh run download <run-id>` and upload manually with `gh release upload <tag> --clobber <files>`. We did exactly that for 0.6.19 — 82 assets up.
- GitHub repo moved `clawft` → `weftos` but the SSH remote still works via redirect. PR URLs are on `weave-logic-ai/weftos`. Local remote string can stay as-is.

---

## Architecture notes I learned this session (for continuity)

### How mic data flows
1. ESP32 INMP441 I²S → bridge code in `~/dev/vector-synth/sources/musica/.../mic_capture.c`
2. Bridge computes RMS using `rms² = E[s²] - E[s]²` (variance form, DC-subtracting)
3. Bridge calls `substrate.publish` at `substrate/sensor/mic` via TCP relay on `127.0.0.1:9471`
4. Daemon's `SubstrateService` stores the value; `tick` increments
5. GUI process (`clawft-gui-egui`) runs a **relay poller** (`native_live.rs:301 relay_external_paths`) that reads the daemon's `substrate/sensor/mic` every 250 ms and injects into the GUI's local `Substrate` as `StateDelta::Replace`
6. `refresh_snapshot` pulls the path into `Snapshot.audio_mic`
7. Tray chip reads `snap.audio_mic` and maps through `audio_state_to_ok` (3-state lamp, NOT a meter)
8. Detail-window gauge is rendered by `surface_host::compose::render_gauge` from `weftos-chip-audio.toml` fixture, binding `$substrate/sensor/mic.rms_db` to a `ui://gauge` with range `-120..0`

### Chip vs meter (important distinction for future debugging)
- `ChipId::Audio` in `shell/tray.rs:50` is the tiny tray lamp. 3 states only: grey (`available: false` OR `rms ≤ -90`), amber (`rms > -3`, near-clip), green (otherwise).
- Live **meter** is only visible when the user clicks the chip to open the detail window. That's where the fixture's `ui://gauge` actually renders a numeric dBFS value.

### Stream anchor + observability (the big TODO)
The 0.6.19 `StreamWindowAnchor` at `crates/clawft-kernel/src/stream_anchor.rs` is the scaffolding for the ExoChain logging redesign. The symposium's Phase 2 generalizes it into a `RollingWindowAnchor` that accepts a pluggable `WindowAggregator`. Three immediate callers to migrate (`peer.envelope`, `reconciler.tick`, `hnsw.eml.observe`) — each one currently produces one SHAKE-256 per event, which is why the chain log is noisy enough to motivate the whole redesign.

---

## Test commands that work right now

```bash
# Workspace compile check
scripts/build.sh check

# The mic tests (9, all green after the fix)
cargo test -p clawft-substrate --lib mic::

# Cluster persistence tests (37, all green)
cargo test -p clawft-kernel --lib cluster::tests:: -- --test-threads=1

# The 3 previously-broken kernel integration tests (fixed by adding ipc_tcp: None fixtures)
cargo test -p clawft-kernel --test stream_anchor_test --test e2e_integration --test feature_composition

# Daemon-side sanity (confirms bridge is publishing)
echo '{"method":"substrate.read","params":{"path":"substrate/sensor/mic"}}' | \
  nc 127.0.0.1 9471 | jq

# Don't run (hangs):
#   scripts/build.sh test          — pre-existing hung test in the aggregate --lib suite
#   scripts/build.sh clippy        — 150 pre-existing clippy errors
```

---

## If the meter still doesn't move after rebuild+restart

Quick debug path, in order:

1. **Confirm relay is landing.** Temporary `eprintln!("[relay] {path} → {value:?}")` at `native_live.rs:333` inside the `substrate.apply(…)` call. Restart GUI. You should see `[relay] substrate/sensor/mic → …rms_db: -41.6…` every 250 ms. If you don't, `DaemonClient::connect()` inside the relay is failing — check the unix socket path vs TCP relay configuration.
2. **If relay lands but snapshot doesn't have it**, something in `refresh_snapshot` is filtering it out. Add an `eprintln!` at `native_live.rs:251` to dump `audio_mic`.
3. **If snapshot has it but detail window is blank**, the composer's `render_gauge` isn't finding `rms_db` in the bound value. Check `$substrate/sensor/mic.rms_db` resolves via the fixture's binding evaluator — `crates/clawft-surface/src/binding.rs` has the resolver. The binding syntax is `$<path>.<field>` where `<path>` is an exact substrate path key and `<field>` walks into the JSON value.
4. **If the detail window never opens**, chip click handling is broken — check `shell/tray.rs` chip click → open detail surface wiring. Unlikely, but possible.

---

## Files to be aware of (so you don't accidentally clobber untracked work)

- `.planning/symposiums/exochain-logging/` — 5 MD files totaling ~2500 lines of design work. Don't touch without asking.
- `.planning/sensors/` — 12-category sensor taxonomy, user-authored.
- `.planning/clients/esp32-bridge-rpc.md` — the RPC contract for the ESP32 bridge. Source of truth for what the bridge expects.
- `docs/runtime-scratch/chain.json` — scratch data; gitignore candidate.

---

## Who's in charge of what

- **User (Mathew):** driving design decisions, running the physical ESP32 hardware, hand-authoring the DC-bias-less piezo mic capture code in the separate `musica` repo.
- **This Claude session:** kernel-side plumbing (cluster persistence, ipc_tcp fixture fills), release engineering (v0.6.19 rollup + tag + GitHub release + asset upload), CI fix (publish-crates workflow flags), ExoChain observability symposium coordination + synthesis, CHANGELOG back-fill for 0.6.18, mic.rs GUI race fix.
- **Background agents this session:**
  - `clawft-casestudy-gen-qsr` rename (42 files, clean)
  - 4 specialists for the ExoChain logging symposium (all delivered)
  - None are still running.

## Pick up here

**Highest priority:** wait for the user's report on whether the meter moves after the rebuild. If yes → commit `mic.rs`, start the `weaver substrate` CLI TODO. If no → walk the debug path in the section above.
