# Session handoff — 2026-04-21

Picking up where this session left off. Everything below reflects
`development-0.7.0` at commit `613b58a`, clean working tree except
for user-owned untracked files (see bottom).

---

## Running state

### Daemon

- **PID** `2563788`, CPU idle (<1%), uptime when this doc was written
- **Unix socket** `/home/aepod/dev/clawft/.weftos/runtime/kernel.sock`
- **Mesh listener** `0.0.0.0:9470` TCP (plaintext; `noise = false` in
  `weave.toml` for bring-up)
- **Node ID** issued fresh on each boot — check `weaver kernel status`
  or the `Cluster membership ready` line in
  `.weftos/runtime/kernel.log`
- **DEMOCRITUS CPU-spin is fixed** — the stuck-detector is now edge-
  triggered with exponential backoff. Roughly 1 warn per phase
  transition, not 4.5 per second

### Binary

- `~/.cargo/bin/weaver` is a stale build. The daemon is running
  `target/release/weaver` built today (Apr 21 late evening). If you
  restart the daemon, use `target/release/weaver kernel start`
  not `weaver` from PATH — the PATH version doesn't have any of
  today's work
- `weave.toml` has `[kernel.mesh] enabled = true` — uncommitted
  working-tree change

### Substrate / chips / panels

- Daemon's `substrate.publish` has a fresh synthetic ToF frame at
  tick 8 (radial gradient, two corner 0xFFFF sentinels)
- GUI relay polls `substrate.read` every 250 ms for
  `substrate/sensor/{tof,mic}` and injects into the local substrate
- `/tmp/weftos/mic/` exists but is empty — file-backed
  `MicrophoneAdapter` polls `stream.raw` which doesn't exist yet;
  chip reads grey until something writes bytes

### VS Code / Cursor extension

- `extensions/vscode-weft-panel/webview/wasm/` rebuilt at 20:20. The
  Cursor panel should hot-reload on next open; if it doesn't,
  `Developer: Reload Window` from the command palette
- Seven tray chips after reload: Kernel, Mesh, ExoChain, Wi-Fi,
  Bluetooth, Audio, **ToF**

---

## Shipped today (commit chain)

| Hash | What |
|---|---|
| `acfe881`..`a2ae646` | Findings #5 (6 kernel EML models), #7 (tick-interval recommender), #9 (causal-edge decay treecalc) — sub-agent batch |
| `f0f77eb` | Lift `Form` + `triage` into new `clawft-treecalc` crate (Finding #10) |
| `eff2d7c` | DEMOCRITUS P0: `Option<f64>` sentinel, `VecDeque` history, edge-triggered warn, swap to `spectral_analysis_rff` (Findings #1, #2, #3, #4, #8) |
| `97b5857` | Wire `RetryModel` into `RetryPolicy::with_model()` (Finding #6) |
| `0e4cc51` | Tray chips open ontology-backed detail panels + 6 fixtures + scope_builder tray fix |
| `8fe5be5` | Register `MicrophoneAdapter` + extend extension RPC allowlist |
| `75e0688` | External-socket streaming subscribers (`ipc.subscribe_stream`) |
| `cb05b00` | `agent.register` + signed IPC envelopes |
| `1b946dd` | `substrate.{read,subscribe,publish,notify}` RPCs |
| `d96ed64` | `StreamWindowCommit` chain anchor service |
| `d81582d` | ToF tray chip with native 8×8 heatmap (superseded by 613b58a) |
| `623b6ae` | GUI relay: poll daemon's substrate.read every 250 ms for externally-published paths |
| `613b58a` | `ui://heatmap` + `ui://waveform` composer primitives; ToF fixture goes declarative; retire native escape hatch |

Branch is **31 commits ahead of origin**. Not pushed.

### Tests added

- `clawft-treecalc`: 9 unit + 1 doctest
- `clawft-kernel`: 18 new unit tests across governance/supervisor/health/reliable_queue/cluster/complexity/weaver/eml_kernel/causal; + stream_anchor integration test
- `clawft-weave`: ipc_subscribe_stream / agent_register_and_sign / substrate_rpc integration tests (7)
- `clawft-llm`: 2 retry-model round-trip tests
- `clawft-gui-egui`: `chip_surfaces` (3 tests: parse / populated / empty)

All green at `613b58a`. `scripts/build.sh check` clean.

---

## Open loops

### Hardware → substrate bridge (Windows side)

- ESP32 is streaming **audio + ToF over WTP** to a Python bridge on
  Windows (618 audio / 80 depth in 20 s reported earlier today).
  Bridge currently receives frames but does not forward them
- **Missing 15-line glue**: bridge opens unix socket at
  `\\wsl$\<distro>\home\aepod\dev\clawft\.weftos\runtime\kernel.sock`
  and calls `substrate.publish {path:"substrate/sensor/{mic|tof}",
  value:<frame>}` for each received frame. See example at the bottom
  of the previous session transcript
- If `AF_UNIX` over the WSL UNC path doesn't work from Windows
  Python, we add a TCP RPC port (separate from mesh 9470). No one
  has tried it yet — that's the fast experiment to do first

### ToF register map (VL53L7CX behind RP2040)

- Sensor is reachable over I2C but reading `0x0400` returns `0xFFFF`
  on every pixel. `0x0400` is ST's native VL53L7CX register — wrong
  layer. The Waveshare RP2040 bridge has its own firmware with its
  own simpler register map
- Product: **Waveshare 8x8 Matrix ToF (ASIN B0FS77NZRC)** — chip
  VL53L7CX, RP2040 bridge, I2C/UART/USB-C
- Diagnostic: sequential-read 256 bytes starting at `0x00`, look for
  a `~128-byte chunk` (64 × 16-bit) whose values change when a hand
  approaches. That's the real buffer
- Or: Waveshare wiki + github.com/waveshare has example code for
  this module

### Audio waveform / spectrogram

- `ui://waveform` primitive exists and works — just needs a samples
  buffer in the substrate topic to plot
- `MicrophoneAdapter` today emits only scalar RMS/peak per window. To
  plot a waveform we need the adapter (or the bridge) to also emit
  a short rolling buffer (e.g. last 2048 samples) at a sibling path
  `substrate/sensor/mic/waveform` or embed it in the mic object
- Spectrogram needs FFT + 2D rolling — use the existing `ui://heatmap`
  primitive once we have a `[freq_bin][time_bin]` matrix topic

### Policy layer (security boundary)

- `egress_check(caller, path)` in `substrate_service.rs` is the seam;
  currently allow-all-but-warn for unsigned callers on Capture-tier
  topics. Hook it to `CapabilityGrant` and a new `EgressGrant`
  governance rule type
- Flip unsigned `ipc.publish` + `substrate.publish` from warn-accept
  to hard-reject once all callers migrate to `agent.register`-signed
  envelopes
- `noise = true` on the mesh transport once ESP32 and Python bridge
  both have their Ed25519 keys minted and exchanged

### StreamWindowCommit anchor

- Service is built and wires on boot but the `[kernel.anchor]`
  config section isn't enabled in `weave.toml`. Once sensor
  topics are being published regularly, add:
  ```toml
  [kernel.anchor]
  enabled = true
  topics = ["sensor.*"]
  window_secs = 2
  ```
  Then every 2 s rolling window of each sensor topic chain-appends
  a signed `StreamWindowCommit` with a BLAKE3 hash. That's the
  audit-without-bloat path we scoped

### Composer gap for sensors

- Audio fixture still uses scalar chip + gauges. Once the mic adapter
  emits a samples buffer, swap RMS/peak gauges for a `ui://waveform`
  node bound to the buffer
- `ui://heatmap` works for ToF, also works for any future spectrogram,
  depth-cam image, thermal grid, 2D flow field. That's a
  general-purpose primitive now

---

## Next-session priorities

In order of highest impact:

1. **Land the Windows bridge glue.** Python script opens the WSL
   unix socket (or TCP RPC if that doesn't work), registers via
   `agent.register`, then `substrate.publish`es each ESP32 frame on
   the right topic. Once that flows, BOTH chips light up without any
   more node-side work
2. **Decode the VL53L7CX register layout** through the RP2040 bridge.
   Either from the Waveshare docs or empirically via the 256-byte
   dump diagnostic. Then ToF frames become real instead of the
   synthetic test gradient
3. **Mic waveform path** — extend `MicrophoneAdapter` to emit a
   short rolling samples buffer, swap audio fixture to use
   `ui://waveform`
4. **`[kernel.anchor]` config** — flip on, let the chain build up
   stream-audit history
5. **Hard-reject unsigned publishes** + wire `CapabilityGrant` into
   `egress_check`. This closes the "mesh is the trust boundary"
   story the user repeatedly emphasized

---

## Architecture invariants from this session's conversation

These came out of the user's pushback and are load-bearing for
future decisions:

- **DeFi/chain is the control plane, not the data plane.** Per-frame
  signing of sensor streams is the wrong layer. Use chain for
  stream-descriptor events (register / window-commit / close) and
  substrate/mesh for bulk with Noise envelope + caller-identity
  auth on every message
- **Mesh is the trust boundary.** Three concentric gates: intra-
  process (`CapabilityGrant`), intra-mesh (peer auth + Noise),
  off-mesh egress (`EgressGrant`, chain-logged, human-confirmed)
- **Broadcast is the natural shape.** Producers `publish`; anyone
  with permission can `subscribe` (stream), `read` (latest-value
  pull), or `notify` (cheap signal). Not file polling. Not mesh
  frames per sample. Not shared markdown files for cross-agent
  coordination
- **Real cross-agent coordination must go over the real bus** —
  signed envelopes, chain-auditable. The fallback to file-based
  handoff is a workaround we removed. Don't reintroduce it

---

## Working-tree status (not mine, leave alone)

- `.planning/sensors/` — user's
- `.planning/weftos_sensors.md` — user's
- `crates/clawft-qfs-gen/` — user's new crate (added to workspace in
  `Cargo.toml` during this session). Do not commit, do not touch
- `scripts/build_vp_deck.py` — user's
- `weave.toml` — uncommitted `[kernel.mesh] enabled = true` + field
  rename. Commit if you want this to persist across daemon restarts

---

## Reference

- Swap-sites catalog: `docs/eml-treecalc-swap-sites.md` (now 12/12
  findings either shipped or research-deferred)
- VS Code extension smoke test: `extensions/vscode-weft-panel/SMOKE.md`
- Build script: `scripts/build.sh`
- Rebuild wasm: `extensions/vscode-weft-panel/scripts/build-wasm.sh`
- Rebuild native weaver:
  `cargo build --bin weaver --profile release`
