# DuplexChannel + dumb VoiceEdge — Implementation Plan

Companion to **ADR-068** (daemon-side full-duplex floor + dumb streaming VoiceEdge).
DESIGN/PLANNING doc — no code lands from this file. Line anchors are current as of
2026-07-04 recon and will drift; treat as landmarks. All paths absolute-from-repo-root.

**The refined thesis (2026-07-04 directive):** the edge is DUMB — it streams to the
daemon, like the first STT iteration (ADR-053: capture PCM → daemon whisper service,
channel adapter is a thin client). STT/TTS/VAD/endpoint/floor/classify are all
daemon-side. The **thin streaming edge is canonical**; on-device STT/TTS is a `mid`
optimization; the desktop native session becomes a thin client of a localhost daemon.

**Dependency status**
- **M2 landed** (5791e4ad, 6d457a1e, 85c4add5): multiplexed `TalkModeLoop`,
  `ViewResolver`, modality-neutral `index_turn` seam. The daemon decider exists.
- **ADR-053 precedent exists**: `clawft-service-whisper` is daemon-side streaming STT
  (`crates/clawft-service-whisper`); the channel adapter was already "a thin client."
  This plan restores that shape and adds full-duplex + TTS-down.
- **ADR-063 (signed envelope) is Proposed, NOT implemented** — but now blocks only the
  *durable-turn* signing on the REMOTE path (Phase 2+), **not** the media stream and
  **not** the localhost desktop path (Phase 1).
- ADR-062 `compute_urgency` is designed; the ERL-confidence term (ADR-068 D1/D5) is a
  **new input** streamed up from the edge that must be threaded into the floor read.

---

## 1. Phasing (desktop-over-localhost first — zero hardware, zero remote-transport risk)

### Phase 0 — Daemon-side DuplexChannel + EdgeReflex core + loopback sim (no hardware)
Define both halves as pure objects and prove them against a software loopback. This
is the abstraction-correctness gate before any wiring.

- **NEW `crates/clawft-kernel/src/duplex.rs` (~300 lines) — the daemon-side machine.**
  - `enum DuplexState { Idle, Listening, Thinking, Speaking, Overlap, Backchannel, Degraded }`.
  - Inputs: `enum StreamObservation` (daemon-derived from the media stream:
    `VadOnset`, `SmartTurnEou`, `PlaybackDrained`, `Erl(f32)`, `HeartbeatLost`) and
    `enum FloorVerdict` (from `TalkModeLoop`: `GrantUser`, `GrantAgent`, `DenyBarge`,
    `Backchannel`, `TurnShift`). Output: `enum EdgeCommand { Play(bytes|text), Mute,
    Silence, Resume, Cancel, Config }`.
  - `fn observe(&mut self, StreamObservation) -> (Option<Impulse>, Vec<EdgeCommand>)`
    (daemon-local transitions + impulse emission) and
    `fn apply(&mut self, FloorVerdict) -> Vec<EdgeCommand>` (ECC-authoritative floor
    transitions). **Only `apply` may change floor-bearing state** — enforced by types
    so a reviewer verifies the actuator/decider boundary by compilation (ADR-068 D1).
  - Encode the ADR-068 D1 transition table literally; table-driven `#[cfg(test)]`.
- **NEW `crates/clawft-channels/src/voice/edge_reflex.rs` (~120 lines) — the dumb edge.**
  The `no_std`-friendly minimal reflex set (ADR-068 D5), dependency-free so it later
  compiles for ESP32:
  - `enum EdgeReflexState { Streaming, Muted, Degraded }`.
  - `on_command(EdgeCommand)` → mute/silence/resume/cancel executed instantly.
  - `on_link_loss()` → `Degraded` (stop TX, silence output). The only self-firing rule.
  - ERL pass-through + optional silence-squelch (bandwidth only; carries no authority).
  - AEC stays in `clawft-voice-aec` (already owns device I/O); this module just relays
    its ERL scalar up and executes mute against it.
- **Loopback duplex simulation (`duplex.rs` + a sim harness, ~150 lines).** `MockStream`
  feeds scripted `StreamObservation`s; `MockFloor` returns scripted verdicts; assert the
  daemon state trajectory + the `EdgeCommand`s a `MockEdge` receives. No cpal, no daemon,
  no network. This is the self-cancel / barge / backchannel bench (§3).

Estimate: ~2 days. Pure logic; no I/O.

### Phase 1 — Desktop thin edge over localhost: stream to a daemon that hosts everything
Wire the real streaming pipeline with the desktop as a **thin client of a localhost
daemon**. Move STT / endpoint / TTS daemon-side; the edge keeps only capture + AEC +
playback + reflexes. Delivers the state machine, the self-cancel fix, and
voice-as-thin-client. **No ADR-063, no remote transport, no hardware.**

- **NEW streaming duplex channel (localhost) — `crates/clawft-rpc` + `daemon.rs`, ~+160.**
  A persistent bidi stream per conversation (ADR-068 D3): media lane (PCM up / TTS PCM
  down) + control lane (`EdgeState{erl_confidence, playback_active, heartbeat}` up /
  `EdgeCommand` down). Localhost = in-process/UDS; the session-auth handshake (D4) is a
  no-op stub here, real in Phase 2.
- **Daemon hosts the capture-side pipeline (MOVE, ~+120 net).** The VAD + smart-turn
  endpoint that live in `crates/clawft-channels/src/voice/{capture.rs,turn.rs,vad.rs}`
  and the STT call move to run **daemon-side on the streamed frames**. `capture.rs`
  already emits `VoiceImpulse{TurnClaim 0x50, EOU 0x51}` from a frame stream via
  `CaptureProcessor` — relocate that behind the daemon stream endpoint so it feeds the
  `TalkModeLoop` `ImpulseQueue` (the M2 `ecc_impulses` consumer). STT = `clawft-service-
  whisper` (ADR-053) or the native parakeet path hosted daemon-side.
- **Daemon hosts TTS-down (~+80).** `DualLayerTts` (`clawft-voice-talk/src/tts.rs`)
  moves daemon-side; rendered PCM streams down the media lane. The ack-cache /
  speculative→committed lifecycle stays (it is ECC node state, ADR-062), now emitting
  `EdgeCommand::Play` chunks instead of driving a local `TtsSink`.
- **`DuplexChannel` wired to `TalkModeLoop` (`talk_loop.rs`, ~+50).** `compute_urgency`
  (ADR-062 D4) gains the ERL-confidence weighting (ADR-068 D1): a low-ERL `TurnClaim`
  cannot win the floor — the principled self-cancel fix. Verdicts drive `duplex.apply`.
- **Desktop VoiceEdge (`clawft-voice-talk/src/native.rs` + `edge_reflex.rs`, ~−250/+120).**
  Collapse `TalkModeController`: cpal capture + `clawft-voice-aec` + playback + the
  `EdgeReflex` core, streaming to the localhost daemon. Delete the inline mic-drain loop,
  `BARGE_IN_*` constants, and `barge_in_enabled` — all become daemon D1 rules. `talkmode.rs`
  shrinks to the thin edge driver; well under the 500-line ceiling.
- **`AgentAnchorConfig` (~+3):** `duplex_edge: bool` (default `false`) — opt-in, same
  discipline as M2's `talk_loop` flag; no global-default flip.

Estimate: ~5–7 days. Depends only on M2 (landed) + the localhost stream. **No ADR-063.**

### Phase 2 — Remote thin edge (LAN/WAN): session-auth transport + durable-turn signing
Same protocol over the network; the edge is a separate process/host.

- **Session-authenticated stream transport (NEW, ~+180).** Noise/QUIC-style handshake
  keyed by the edge's Ed25519 node identity (ADR-025); frames flow unsigned over the
  authenticated channel (ADR-068 D4). Heartbeat + `Degraded`/reconnect (edge re-handshake;
  daemon reconcile). **No per-frame signing.**
- **Durable-turn signing (ADR-063, ~+60).** The committed turn via `agent.turn.record`
  becomes an ADR-063 signed envelope (it is stored/witnessed). Media frames stay unsigned.
  Blocks on ADR-063 `apply_signed` existing — but only THIS path, not the stream.
- **`clawft-voice-edge` standalone binary (extract, ~+150).** The desktop edge from
  Phase 1, repackaged as a remote-capable process (cpal + AEC + playback + `EdgeReflex`
  + transport client). Same code, remote transport.

Estimate: ~5–7 days **after** the session transport + ADR-063 durable path. Remote only.

### Phase 2b (optional) — `mid` optimization profile
For links where streaming PCM is prohibitive: run STT/TTS on an SBC edge, exchange
**text** over the media lane (ADR-068 D2). Purely additive — a different payload type on
the same protocol; do NOT let it complicate the thin path. Build only when a real
deployment needs it.

### Phase 3 — ESP32-S3 thin edge firmware (the payoff)
The dumb edge on constrained hardware: capture + playback + AEC + `EdgeReflex` + stream.

- **`crates/clawft-edge-pad/` extension (or sibling `clawft-edge-voice`, firmware).**
  Already `#![no_std]`, holds `ed25519-dalek`, publishes over `clawft-substrate`. Add I²S
  mic capture, PCM playback, the `no_std` `EdgeReflex` core (Phase 0 built it
  dependency-free for this), and the session-auth stream client. Optional silence-squelch
  to fit the WiFi budget; optional `provisional-duck` reflex if WAN overtalk is felt
  (ADR-068 D5).

Estimate: firmware-scale, multi-week; scope a spike first. Blocks on Phase 2 transport.

---

## 2. The actuator/decider boundary as a type-level invariant

The riskiest thing (ADR-068 D1/D5) is enforced structurally, not by discipline:
- **Daemon `DuplexChannel`:** only `apply(FloorVerdict)` mutates floor-bearing state;
  `observe(StreamObservation)` may emit impulses + daemon-local transitions but **cannot
  construct a `GrantUser`/`GrantAgent`**. ECC decides the floor; the machine executes.
- **Edge `EdgeReflex`:** its input type is `EdgeCommand` (from the daemon) + link events;
  it has **no** `FloorVerdict` and **no** impulse constructor — it is structurally
  incapable of deciding anything. A reviewer verifies "the edge is dumb" by types.

---

## 3. Test plan (all runnable WITHOUT hardware)

1. **Transition-table conformance (Phase 0).** Table-driven over every ADR-068 D1 row:
   `(state, observation|verdict) → state + emitted impulse + EdgeCommands`. Fails if a
   transition is added in code without a table row.
2. **Text-degenerate conformance (ADR-068 D6) — REQUIRED.** Drive the daemon
   `DuplexChannel` in text-payload mode (floor auto-granted; overlap/backchannel/barge/ERL
   unreachable); assert the trajectory collapses to `Idle→Listening→Thinking→Speaking→Idle`
   with those states NEVER entered. Cross-check against M2's `index_turn` emitting the same
   `register_turn`+`EOU` for text. If text does not collapse cleanly, the abstraction is wrong.
3. **Loopback duplex simulation (Phase 0).** `MockStream`+`MockFloor`+`MockEdge`:
   - **self-cancel regression:** inject an onset with **low ERL** during `Speaking`
     (the bot's own AEC leakage); assert `Overlap` entered but ECC `DenyBarge` reverts to
     `Speaking` — the answer is NOT cancelled and NO `Cancel`/`Mute` reaches the edge. The
     no-hardware proof of the 55d33c92 fix.
   - **genuine barge:** high-ERL onset; assert `Overlap→Listening`, TTS cancelled,
     `Contradicts`/prune emitted, `Mute` delivered to the edge.
   - **backchannel:** brief onset ECC-classified `Backchannel 0x60`; `Speaking` never
     yields, no `EdgeCommand` interrupts playback, a `Continuer` CrossRef (not a turn) logged.
   - **link drop:** `HeartbeatLost` mid-`Speaking`; edge `EdgeReflex` → `Degraded`
     (stop TX, silence output) with no daemon command; daemon marks the conv stale.
4. **Localhost stream round-trip (Phase 1).** Extend `tests/assembly.rs`
   `live_native_talk_session`: WAV → desktop edge → localhost stream → daemon VAD/endpoint/
   STT → `DuplexChannel` → `TalkModeLoop` → TTS PCM down → assert one committed ECC turn on
   the shared forest (not a private `TalkForest`).
5. **Latency-budget assertion (Phase 1/2).** Instrument onset→`Mute`-delivered; assert the
   localhost overtalk window is within a tick and the LAN window < 100 ms (ADR-068 D5 table).
6. **Session-auth + durable signing (Phase 2).** Assert an edge that fails the Ed25519
   handshake is refused; assert a committed turn is a valid ADR-063 envelope; assert media
   frames carry NO per-frame signature (transport-authenticated only).
7. **Existing gates stay green.** `crates/clawft-voice-talk/tests/speak_wav.rs` (real
   DualLayerTts intelligibility, `--ignored`) must still pass after TTS moves daemon-side —
   the daemon-hosted render is behavior-preserving for the happy path.

---

## 4. Plane-item candidates (per cycle)

Cycle taxonomy: `0.7.x` must-ship, `0.8.x`+ later. Create via `plane-workflow` on claim;
cite ADR-068 + this plan.

| Candidate | Cycle | Scope | Blocks on |
|---|---|---|---|
| Daemon `DuplexChannel` + `EdgeReflex` core + loopback sim (Phase 0) | 0.8.x | `duplex.rs`, `edge_reflex.rs`, sim, tests 1–3 | — |
| Text-degenerate conformance test | 0.8.x | test 2 (abstraction gate) | Phase 0 |
| Desktop thin edge over localhost (Phase 1) | 0.8.x | stream; move VAD/endpoint/STT/TTS daemon-side; collapse `TalkModeController` | Phase 0; M2 (done) |
| ERL-confidence in floor + re-enable barge (Phase 1) | 0.8.x | `compute_urgency` ERL term; self-cancel fix; supersedes the "Re-enable barge-in" handoff item | Phase 1 |
| `duplex_edge` opt-in flag | 0.8.x | `AgentAnchorConfig`; no default flip | Phase 1 |
| Session-auth stream transport + `Degraded`/reconnect (Phase 2) | 0.9.x | Noise/QUIC handshake keyed by ADR-025; heartbeat | Phase 1 |
| Durable-turn ADR-063 signing on the remote path | 0.9.x | `agent.turn.record` → envelope; media stays unsigned | **ADR-063 impl** |
| `clawft-voice-edge` standalone remote binary | 0.9.x | extract Phase-1 edge as a remote process | Phase 2 transport |
| `mid` text-payload optimization profile (Phase 2b) | 0.9.x/1.0.x | optional; SBC local STT/TTS, text wire | Phase 2 |
| ESP32-S3 thin edge firmware (Phase 3) | 1.0.x | capture+playback+AEC+`EdgeReflex`+stream on no_std | Phase 2 transport |
| Optional `provisional-duck` reflex for WAN overtalk | 1.0.x | build only if a WAN deployment needs it (ADR-068 D5) | Phase 3 |
| ADR-065 subkey for multi-human shared-room edge | 1.0.x | deferred multi-speaker attribution | Phase 2 |

**Existing items to reconcile (do not duplicate):**
- **WEFT-606** (0.8.x, "daemon-hosted Talk-Mode tick"): M2 landed the loop; ADR-068
  reframes the remainder as "the edge streams in as a thin client," and additionally
  moves STT/endpoint/TTS daemon-side. Update WEFT-606's scope to point at Phase 1, or
  close it and open the Phase 1 item.
- **WEFT-613** (0.9.x, voice-matched fast tier): orthogonal (TTS voice identity), still
  relevant but now the TTS runs daemon-side.
- **WEFT-614** (0.9.x, grounded agent brain): rides Phase 1 — once the edge streams into
  the daemon, the brain is the daemon agent loop by construction. Note the dependency.
- **"Re-enable barge-in"** handoff item (Tailscale-blocked): **superseded** by the
  ERL-floor item — barge-in becomes a floor decision, not a boolean.

## 5. Packaging + topology notes (ADR-068 D7 / Consequences)

- **Sidecar SDK**: `edge_reflex.rs` + the stream-lane client graduate into a core
  `no_std`-capable crate (proposed `clawft-duplex-edge`) with thin per-host
  bindings: wasm-bindgen bundle (web / agentic-OS UI), C FFI / UniFFI
  (iOS/Android/desktop embedding), embassy/`no_std` (ESP32 — the Phase 3 target
  consumes the same core, not a fork), plain Rust API. One core, many skins — the
  protocol crate is the product. Extract the crate no later than Phase 2 (when the
  edge splits out of `clawft-channels`).
- **WASM/browser edge**: a wasm32 build of the edge client (WebAudio-worklet
  capture, event-loop state machine, wasm-compatible ed25519 session handshake)
  targeting the agentic-OS web UI; the website chat box is the text-payload edge.
  Transport requirement lands in Phase 2: the D3 two-lane framing must carry over
  **WebSocket** as well as Unix socket/TCP (check the GUI wasm VSCode-panel build
  for an existing daemon WS bridge to reuse). A **wasm loopback validation build**
  should accompany Phase 0's sim — it structurally forbids locality assumptions in
  the protocol code.
- **Topologies are connection targets, not variants** (ADR-068 D7): remote daemon /
  localhost / same-SoC loopback (self-delegating device) / browser-WS all speak the
  identical protocol; profile differences are session-handshake capabilities.
