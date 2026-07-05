# ADR-068: Full-duplex conversation floor + dumb streaming VoiceEdge

**Date**: 2026-07-04
**Status**: Proposed
**Deciders**: Voice-edge architecture design (system-architect thread, 2026-07-04),
directed by the two theses recorded under "Context"
**Depends-On**: ADR-062 (ECC graph-walk conversation — the decider this actuates),
ADR-061 (conversational voice loop — the pipeline being refactored),
ADR-053 (voice STT canonical path — the streaming-edge precedent this restores),
ADR-047 (self-calibrating cognitive tick — the 50ms cadence the floor runs on),
ADR-063 (signed content-addressed substrate envelope — durable-turn identity),
ADR-025 (Ed25519 node identity — the per-device session key)
**Relates-To**: M2 daemon-hosted ECC loop (`.planning/hermes-loop/m2-daemon-ecc-loop-design.md`;
commits 5791e4ad, 6d457a1e, 85c4add5 — multiplexed `TalkModeLoop`, `ViewResolver`,
modality-neutral `index_turn` impulse seam), ADR-065 (per-source subkey — deferred
multi-speaker relay), ADR-058/060 (context tier + Hermes brain),
`.planning/voice/duplex-edge-agent-plan.md` (the phased implementation plan)

## Context

Two theses set the direction; this ADR engineers them.

1. **The I/O pipeline is a full-duplex object.** Today `TalkModeController`
   (`crates/clawft-channels/src/voice/talkmode.rs`) is a half-duplex loop with
   ad-hoc holds: `speak_with_barge_in` blocks the turn, drains the mic by hand to
   stop the capture channel flooding, prebuffers the slow tier against stutter,
   and gates barge-in behind a fixed grace window plus a `barge_in_enabled` flag
   that is **default-off** because the bot's own reply leaks past AEC and
   self-cancels (commit 55d33c92). Each is a point fix for a missing abstraction:
   one full-duplex object whose transitions are *rules*, not inline branches.

2. **The voice loop is a standalone specialized agent, and the edge is DUMB.**
   Cognition is already daemon-hosted: M2 made the `TalkModeLoop` multiplexed and
   modality-neutral, and `SessionTier::index_turn` emits `register_turn` +
   `EndOfUtterance` at the anchor convergence seam shared by `agent.chat` **and**
   `agent.turn.record`. The edge that handles the ends of the full-duplex pipe —
   text *or* audio — should be **fairly dumb: it streams to the daemon**, exactly
   as the first STT iteration did (ADR-053: sensor nodes capture PCM, push it over
   the substrate to a daemon-hosted whisper service, "the voice channel adapter is
   a thin client of these substrate services"). STT, TTS, endpointing/smart-turn,
   floor logic, and classification are **all daemon-side**. The edge is capture +
   playback + the smallest possible reflex set. The same thin edge runs on a Mac, a
   Snapdragon SBC, or an ESP32-S3; richer local capability is an *optimization
   profile*, never the design target.

The old framing ("voice cutover = move Talk-Mode into the daemon", WEFT-606) is
**superseded**: the cognition is already in the daemon. The edge does not move in;
it streams in. This ADR deliberately inverts today's on-device native session (a
"fat" client) back toward the ADR-053 streaming model as the canonical case —
which ADR-053 itself flagged as a future re-opening for embedded targets.

## Decision

Split the full-duplex pipeline across a **daemon-side decider** and a **dumb edge**:

- **The `DuplexChannel`** — a **daemon-side** full-duplex conversation-floor
  controller: one explicit state machine, driven by ECC outputs (floor verdicts +
  impulses from the `TalkModeLoop`), that translates conversation state into
  concrete stream commands to the edge. It is an **actuator of ECC verdicts, never
  a decider of the floor** — ECC decides; the `DuplexChannel` executes. Every
  current ad-hoc hold becomes a transition rule here, in the daemon.
- **The `VoiceEdge`** — a **dumb streaming transducer** at the device: capture +
  playback + a tiny control channel + a *minimal, enumerated* reflex set. It runs
  the same protocol on every host; STT/TTS/VAD/endpoint/floor/classify all live
  daemon-side. Local STT/TTS is a negotiated capability, off the canonical path.

### D1 — DuplexChannel (daemon-side): the ECC-driven floor state machine

The state machine runs in the daemon, fed by the frames the edge streams up. The
daemon runs VAD, smart-turn endpointing, floor scoring, and backchannel
classification on that stream — the edge derives none of it. The machine's job is
to translate ECC floor verdicts into `EdgeCommand`s (stream, play, mute, silence).

**States** (daemon-side; the edge holds only the reflex shadow of §D5)

| State | Edge mic | Edge speaker | Meaning |
|-------|----------|--------------|---------|
| `Idle` | streaming | silent | no active floor; daemon watches the stream for onset |
| `Listening` | streaming | silent | user holds the floor; daemon accumulates the utterance |
| `Thinking` | streaming | silent | daemon detected EOU; walking the graph, rendering |
| `Speaking` | streaming | playing | agent holds the floor; daemon streams TTS audio down |
| `Overlap` | streaming | playing→ducked | daemon detected onset during `Speaking`; provisional |
| `Backchannel` | streaming | playing | daemon classified a continuer; playback continues |
| `Degraded` | (edge-local) | silent | link lost; edge reflex owns safety, daemon has no view |

The edge **always streams** (subject only to a coarse silence-squelch bandwidth
optimization, §D3) — there is no edge-side "start/stop capturing" turn decision.

**Transition table** (rule source: `ECC` = floor verdict/impulse from `TalkModeLoop`,
one tick + link RTT; `daemon-local` = daemon computes it from the stream without ECC;
`edge-reflex` = §D5, the only edge-side rules)

| From | Event (daemon-observed unless noted) | To | Rule source |
|------|--------------------------------------|----|-------------|
| Idle | VAD onset in stream → `TurnClaim 0x50` | Listening | daemon-local (grant implicit for sole party) |
| Listening | smart-turn EOU → `EndOfUtterance 0x51` | Thinking | daemon-local; ECC owns the commit |
| Thinking | floor grant + `content_readiness` met | Speaking | **ECC** (readiness = the prebuffer hold) |
| Thinking | `TurnShift 0x52` (user resumes) | Listening | ECC |
| Speaking | TTS drained, no pending frontier | Idle | daemon-local (drain reported by edge) |
| Speaking | committed answer supersedes ack | Speaking | ECC (node lifecycle; new content) |
| Speaking | onset above gate, ERL confirms | Overlap | daemon-local (using edge-streamed ERL) |
| Overlap | ECC grants user floor | Listening | **ECC** (cancel TTS = `Contradicts`/prune; `Mute` to edge) |
| Overlap | ECC denies (low ERL confidence) | Speaking | **ECC** (un-duck; the AEC self-cancel fix) |
| Speaking | onset classified `Backchannel 0x60` | Backchannel | ECC (`Continuer` CrossRef, never a turn) |
| Backchannel | continuer ends | Speaking | daemon-local |
| any | edge heartbeat lost | Degraded | edge-reflex + daemon marks stale |
| Degraded | link restored + resync | Idle | edge-reflex + ECC reconcile |

**The self-cancel fix, unchanged in mechanism.** The bot's own reply leaking past
the edge's AEC arrives in the stream as an onset with **low echo-return-loss (ERL)
confidence** — a scalar the edge measures locally (its AEC is edge-side, §D5) and
streams up in the control channel. The daemon floor read (`compute_urgency`,
ADR-062 D4) weights a low-ERL `TurnClaim` down and never grants the floor, so
`barge_in_enabled` stops being a default-off flag and becomes a runtime floor
decision. Every ad-hoc hold in `talkmode.rs` (mic-drain, prebuffer, barge grace,
ack-cache) maps to a rule in this table — but now the rules live in the daemon,
not the device.

### D2 — There are no tiers: one thin edge + two orthogonal axes

The earlier draft's "T-thin / T-mid / T-full profiles" are **rejected as protocol
concepts**. There is exactly one edge — the dumb streaming transducer — and what
looked like tiers decomposes into two axes that the protocol does **not**
special-case:

1. **Deployment topology — *where does the daemon run*.** The wire contract is the
   module boundary, so remote-daemon, localhost-daemon, and same-process-loopback
   are the **same design** (see §D7). An ESP32 talking to a
   daemon on the LAN, a Mac talking to a localhost daemon, and a Snapdragon box
   running edge + daemon in one process all speak the **identical** protocol. Moving
   the daemon is a **connection-target change, not an architecture change**.
2. **Capability negotiation — *what optional offload the edge provides*.** Local
   STT/TTS/codec is not a profile baked into the protocol; it is a **capability the
   edge advertises at connect** (`Config` handshake, §D4). An edge that offers local
   STT sends UTF-8 text on the media lane instead of PCM and the daemon skips its
   STT stage; an edge that offers none streams raw PCM. Same protocol, negotiated
   payload — the old "T-mid" is just "edge advertised the STT+TTS codec capability."

So the canonical thing is the thin streaming edge; everything else is *where you
point it* and *what it happens to offload*, neither of which the protocol branches
on. This is what lets a device **delegate to itself** for free (§D7).

### D3 — Wire: one payload-typed streaming protocol

The connection is a **persistent bidirectional stream**, not a sequence of discrete
messages. Two multiplexed lanes:

- **Media lane (high rate):** opaque, length-prefixed **payload frames**, typed by
  the **negotiated capability** (§D2), not by a baked profile — 16 kHz PCM when the
  edge offers no local codec, UTF-8 text deltas when it advertises local STT/TTS.
  Up = mic/keystrokes; down = TTS PCM / rendered text. No per-frame structure
  beyond a type tag + length; nothing is parsed on the hot path except routing.

The protocol assumes **neither locality nor non-locality**: no field encodes "am I
remote", no timing depends on a round trip being non-zero, and no path assumes it is
zero. Framing, length-prefixing, and the handshake are identical whether the two
ends are separated by a WAN, a loopback socket, or an in-process channel — the only
difference is the transport the bytes ride (§D4).
- **Control lane (low rate):** `EdgeState{playback_active, erl_confidence,
  squelch_active, heartbeat}` up; `EdgeCommand{play|mute|silence|resume|cancel,
  config}` down. This is the "tiny control channel" — floor verdicts reach the edge
  only as concrete `EdgeCommand`s; the edge never sees a `FloorVerdict` or an
  impulse code. The daemon derives impulses (`TurnClaim`/`EOU`/`TurnShift`/
  `Backchannel`) from the media stream itself.

An optional **silence squelch** on the edge suppresses transmitting pure-silence
media frames to save bandwidth. It is a bandwidth optimization with **no semantic
authority** — the daemon re-runs VAD/endpoint on whatever arrives; the squelch is
not the turn detector. A text edge streams keystrokes/lines; the same two lanes,
different payload type — this is the text symmetry (§D6) made literal.

### D4 — Transport + identity: session-authenticated stream, not per-frame signing

Continuous frame streaming favors a **persistent, per-session authenticated
channel**, and the durable/ephemeral split makes per-frame signing wrong:

- **Ephemeral media + control (real-time, never stored):** one authenticated
  session per connection — a Noise/QUIC-style handshake keyed by the edge's Ed25519
  **node identity** (ADR-025). The device proves who it is **once**, at connect;
  frames then flow over the authenticated channel with **no per-frame signature**.
  Audio/keystroke frames are consumed and discarded — they are not substrate state,
  are never re-replicated by a relay, so ADR-063's "survive the hop, re-verifiable
  after storage" reasoning does not apply to them.
- **Durable turns + commits (stored, witnessed):** the committed turn still lands
  via `agent.turn.record` and is content-signed per **ADR-063** — because *that*
  is stored on the kernel-global forest, hash-chained, and audited. Content
  addressing there gives idempotent replay (ADR-064). The impulses the daemon
  derives from the stream are authored by the daemon, so they inherit its identity.

So ADR-063 envelopes gate the **durable** boundary; the **ephemeral** stream is a
session-authenticated socket. This is a change from an earlier draft that wrapped
every impulse in an envelope: per-frame signing on a 16 kHz stream is neither
affordable (ESP32) nor meaningful (nothing is stored).

**Transport is chosen per connection, protocol is not.** The handshake is the same
identity assertion regardless of transport; only its *implementation* varies with
locality — a full cryptographic handshake over a WAN/LAN socket, and a
degenerate-to-trusted assertion for a loopback or in-process channel where the OS
already guarantees the peer. Locality selects the transport; it never selects a
different protocol (Deployment section).

**Identity.** The edge is a native node (ADR-025) with its own key; the session
handshake proves it, and any durable turn it triggers is attributable to it.
ADR-065's per-source **subkey** pattern applies only when one edge fronts *multiple*
un-keyed downstream humans (a shared-room device attributing distinct speakers) —
deferred. **Link drop mid-turn** is the `Degraded` reflex (§D5): the edge stops TX
and silences output; on reconnect it re-handshakes and the daemon reconciles via
ADR-064. Invariant: **a disconnected edge never speaks unprompted and never holds
an open mic into a dead link.**

### D5 — The minimal edge reflex set (the only edge-local logic)

Everything that can tolerate one tick + link RTT is daemon-authoritative. The edge
holds **only** reflexes for things that physically cannot wait for a round trip:

1. **Sample-accurate echo cancellation.** The AEC render reference is the audio the
   edge is *playing*; capture and playback are both local, so only the edge can
   align them sample-accurately. Over the link, jitter would destroy alignment
   (ADR-061 §1: AEC must own device I/O). The edge runs AEC and streams up the
   resulting **ERL confidence** scalar the daemon floor read consumes.
2. **Hard mute / silence-output on command.** When the daemon confirms a barge
   (`EdgeCommand::mute`/`cancel`), the edge silences the speaker **instantly**,
   without draining its playout buffer. Reflex *execution* of a daemon *decision* —
   the edge does not judge the barge.
3. **Link-loss safety (`Degraded`).** On heartbeat loss: stop TX, silence output,
   bound any local capture buffer, wait to re-handshake. The one reflex that fires
   without any daemon command, because its trigger *is* the daemon's absence.

**Explicitly NOT reflexes** (daemon-authoritative): VAD onset, endpointing (EOU),
floor grant/turn-shift, barge **judgment**, backchannel classification, ack timing.

**Latency budget vs the human turn-gap (~200 ms+).** Turn-taking tolerates a round
trip because the gaps it arbitrates are large; the one latency-critical path is the
barge **overtalk window** (from user onset to the mute command landing):

| Deployment / link | Link RTT | Turn decision (onset/EOU → command) | Barge overtalk window |
|---|---|---|---|
| desktop (localhost) | ~0–1 ms | ≤ 1 tick (50 ms) | ~tick |
| LAN | 5–20 ms | RTT + tick | ~RTT + tick (< 100 ms) |
| thin on WiFi/WAN | 20–100 ms | RTT + STT + tick | up to ~200 ms |

A 50 ms tick + LAN RTT is well inside the ~200 ms human turn-gap, so daemon-side
turn-taking reads as natural. The barge overtalk window is the real cost: on a
localhost/LAN link it is tens of ms (imperceptible); only on a poor WAN link does
it approach 200 ms of the bot talking over the user before mute. If a deployment's
link makes that window unacceptable, an **optional** edge `provisional-duck` reflex
may be enabled (duck on loud sustained input during playback; the daemon confirms →
`mute` or reverts → `resume`) — **off by default** to keep the edge dumb, and it
never *decides* the barge, only pre-ducks pending the daemon's verdict.

### D6 — Text is the same dumb streaming edge, payload-typed

A terminal, the GUI chat panel, or an inkpad is a `VoiceEdge` with a **text**
payload type: keystrokes/lines up the media lane, rendered text down. The floor is
always granted (text submit *is* the floor), so `Overlap`/`Backchannel`/barge/ERL
are unreachable and the machine collapses to `Idle → Listening → Thinking →
Speaking → Idle`. This is exactly M2's modality-neutral `index_turn`, which already
emits `register_turn` + `EndOfUtterance` for text at the same seam voice uses. One
protocol, payload-typed; **if text does not collapse this cleanly, the abstraction
is wrong** — a required conformance test (see the plan). Text and audio differ only
in payload type and which transitions are reachable, not in the machine.

### D7 — Deployment topology is not a protocol variant; WASM/browser is a first-class target

**Where the daemon runs is deployment topology, not architecture.** The wire
contract (D3/D4) is THE module boundary; nothing in it may assume network locality
OR non-locality. Four topologies, one protocol, one daemon-side state machine:

1. **Remote daemon** — edge device → daemon over the network.
2. **Localhost daemon** — desktop edge → daemon on the same host.
3. **Same-SoC loopback (self-delegating edge)** — a device (e.g. Snapdragon-class)
   hosting its own daemon and connecting to itself over loopback. Kept open by
   design at zero cost: it is a connection-target choice, never an architecture
   change — the device can later defer up to a bigger daemon or the mesh by
   re-pointing the stream.
4. **Browser → daemon over WebSocket** — the edge compiled to **wasm32**, embedded
   in a website or the agentic-OS web interface: WebAudio-worklet capture,
   event-loop-driven client (no threads by default), wasm-compatible
   `ed25519-dalek` for the D4 session handshake. The website chat box is the
   text-payload edge in wasm — one client serves both text and voice in the web
   UI. The browser also *cannot* host a fat pipeline, independently validating the
   dumb-edge-first decision.

**Transport constraint on D4:** the stream framing must carry over **WebSocket**
as well as Unix socket/TCP — the same two lanes over different carriers (check
whether the GUI's wasm VSCode-panel build already has a daemon WS bridge to
reuse). Profile differences (local STT/TTS offload, the `mid` text uplink) are
**capabilities negotiated at the session handshake**, not protocol variants. A
wasm loopback build is an early protocol-validation target because it
structurally forbids locality assumptions.

## Consequences

**Positive.** The edge is genuinely dumb — buildable on an ESP32-S3 as capture +
playback + AEC + three reflexes, identical protocol to the Mac. Barge-in is a
daemon floor decision (ERL in the read), so the self-cancel bug is fixed by
construction. One daemon-side state machine serves text and every audio edge. The
transport simplifies to a session-authenticated stream for ephemeral media +
ADR-063 envelopes only for durable turns — no per-frame crypto on the hot path.
The design restores the proven ADR-053 streaming model and makes the fat on-device
session the special case it should be.

**Sidecar SDK (packaging).** The edge is an **embeddable sidecar**: package the
edge client (stream lanes, reflex set, session handshake) as a core
`no_std`-capable crate (proposed: `clawft-duplex-edge`) with thin per-host
bindings — a wasm-bindgen bundle (web / agentic-OS UI), a C FFI / UniFFI surface
(iOS/Android/desktop embedding), an embassy/`no_std` integration (ESP32-class
firmware), and the plain Rust API (native apps). One core, many skins — **the
protocol crate is the product** — so WeftOS conversation can be sidecarred into
mobile apps, web apps, desktop apps, and hardware builds.

**Negative / risks.** All conversation latency now crosses the link, so the D5
reflex/authority boundary is load-bearing: the barge overtalk window on a poor WAN
link is the one place daemon-side floor logic is felt, mitigated only by the opt-in
provisional-duck. The ERL-confidence signal must be a real edge AEC measurement or
the barge floor is uninformed (the reason barge is off today — a hardware-AEC
dependency). Streaming raw audio to the daemon costs bandwidth the text-uplink
capability exists to relieve; the canonical thin edge assumes a link that can carry PCM.
ADR-063 is **Proposed, not implemented**, but now blocks only the *durable-turn*
signing, not the stream — so the localhost deployment ships first regardless.

**Deferred.** ADR-065 subkey attribution for multi-human shared-room edges;
on-device Piper-class TTS for a fully-offline thin edge; learned floor weights;
the opt-in provisional-duck reflex (build only if a WAN deployment needs it).

## Alternatives considered

- **Fat edge (on-device STT/TTS/floor) as the canonical profile.** Today's native
  session. Rejected as the *target*: it couples the pipeline to one host's compute,
  cannot run on an ESP32, and re-litigates the ADR-053 decision that streaming to a
  daemon service is the blessed path. Kept as a negotiated local-STT/TTS capability.
- **In-process cutover (move Talk-Mode into the daemon).** The pre-M2 framing.
  Rejected: cognition is already in the daemon; the audio device stays at the edge
  and *streams* in.
- **Per-frame signed envelopes for the media stream.** Wrap every audio/impulse
  frame in an ADR-063 envelope. Rejected: a 16 kHz stream is ephemeral and never
  stored, so content signing buys nothing a session handshake does not, and its
  per-frame cost is unaffordable on an ESP32. Envelopes gate the durable turn only.
- **Half-duplex forever (keep `TalkModeController`).** Rejected: the ad-hoc holds
  cannot express backchannel/floor/repair as first-class and the self-cancel bug
  has no principled fix without the ERL-in-floor rule.
- **Full-duplex S2S (Moshi) on the edge.** Rejected as ADR-062 rejects it: welds
  the LLM to the voice stack, losing the swappable daemon brain.
