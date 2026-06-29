# Build Plan — ECC graph-walk voice conversation (ADR-062)

**Goal:** drive the local-Hermes voice conversation as the **Act mode (ADR-042)** of the cognitive
**forest (ADR-046)** on the **self-calibrating tick (ADR-047)** — a turn is a walk that mutates the
graph; responses are **nodes** (Speculative→Committed); "routing" is which node is current. All
inference **native Rust, no Python**.

**Implements:** ADR-062 (the model) · extends ADR-058 (L2 tier = frontier-walk substrate),
ADR-061 (voice front), ADR-053 (native STT). **Read ADR-062 first.**

**Source of truth for *what exists*:** the kernel-reality analysis — ~60% of the walk skeleton is
built (`SessionView` live; `DemocritusLoop` dormant; `NodeState` enum; `ImpulseQueue`; the
self-calibrating tick; the full edge/crossref vocab). The 40% below is the orchestration.

**Repo rules:** branch off `integration` (NEVER master); `cargo build` + `cargo clippy --workspace
--all-targets -- -D warnings` (currently **0** — keep it) + `cargo test` green before merge; no
`Co-Authored-By`; files <500; `cargo fmt`. Native ONNX models are gitignored under `.weftos/models/`;
live tests `#[ignore]`-gated where weights/mic/Hermes are required — never fabricate measurements.

## Dependency overview
```
P0 routing primitives ─┐
                       ├─► P2 Talk-Mode tick service ─► P6 assembly + live e2e
P1 join the forest ────┘        ▲              ▲
P3 native STT/turn/speaker ─────┘              │  (impulse sources)
P4 native TTS (node-renderers) ────────────────┘
P5 VoiceChannel (cpal→AEC→impulses) ───────────┘
```
Critical path: **P0 → P1 → P2 → P6**. P3/P4/P5 are component tracks that join at P2/P6.

---

## Phase 0 — Kernel routing primitives (`clawft-kernel`)
The vocabulary the walk needs. Both `ImpulseType` and `CrossRefType` are `#[non_exhaustive]` → promote
`Custom(u8)` to named variants.
- **0.1 Turn impulses + Continuer.** `ImpulseType::{EndOfUtterance 0x51, TurnClaim 0x50, TurnShift 0x52,
  Backchannel 0x60}`; `CrossRefType::Continuer`. *Done-when:* variants exist, HLC-ordered drain unit test.
- **0.2 Per-turn NodeState assignment.** Helpers to set `Speculative→Frontier→Committed/Stale` on
  `SessionView`/`ChunkMeta` (`context_graft.rs:393`) AND on `CausalNode` metadata.state. *Done-when:*
  a node walks Speculative→Committed and Speculative→Pruned in tests; today `Speculative` is unassigned.
- **0.3 Floor read.** `compute_urgency` (0.30 semantic / 0.15 arousal / 0.20 wait / 0.10 crowd / 0.25
  readiness; hard-interrupt `f32::MAX`) + `FloorState{Open,Held,Contended,Locked}` as a scored read over
  `ImpulseQueue` × `SessionView.live_seqs()`. Weights are constants (Weaver learns them later).
  *Done-when:* given a queue + candidates, the scored winner/decision matches hand-computed expectations.
- **0.4 Coherence dampener.** Expose the RSTE soft read (×0.8 below threshold, NOT a hard gate);
  `CoherenceAlert` already exists. *Done-when:* dampener applies + clears on recovery in a test.

## Phase 1 — Join the forest (`clawft-kernel` + `clawft-service-agent`) — the central change
Make the live conversation path a **forest traversal (ADR-046)**, not HNSW-in-isolation.
- **1.1 Dual-write turns.** `index_turn` (`session_tier.rs:239` / `substrate_sink.rs:336`) ALSO
  `CausalGraph.add_node` + `link(prev→new, Follows, weight=coherence)` + register speaker/emotion/goal
  `CrossRef`s. *Done-when:* a turn lands as a chain event AND a causal node with a Follows edge + crossrefs.
- **1.2 Lineage-fused graft.** Extend `graft_text` (`context_graft.rs:382`) to fuse HNSW recall with
  `traverse_forward`/`trace_causal_chain` (`causal.rs:429/503`) + `CrossRefStore.by_type`. *Done-when:* a
  recall test surfaces a causally-linked chunk that cosine-alone misses; provenance backrefs intact.
- *Dep:* P0.2 (state), ADR-046 structures (exist).

## Phase 2 — Talk-Mode tick service (`clawft-kernel`/`clawft-service-agent`/`clawft-weave`)
A turn-scoped `SystemService` modeled on the **dormant `DemocritusLoop`** (`democritus.rs:88` already
drain→embed→search→update→commit on a budget), alongside the coherence `CognitiveTick` (LEAVE THAT
ALONE), sharing ADR-047 budget/adaptive primitives.
- **2.1 Loop body.** SENSE = `ImpulseQueue.drain_ready()`; FLOOR = 0.3 over `live_seqs()`
  (EOU→commit, TurnShift→handoff, **Backchannel→emit Continuer crossref, NO node**, TurnClaim→prune +
  `Contradicts`); MUTATE = `set_state` + dual-write node/lineage/crossref. *Done-when:* deterministic
  test (mock impulses) drives the full SENSE→FLOOR→MUTATE path; backchannel-never-a-node invariant holds.
- **2.2 Render-from-node.** The missing path: **Speculative node = cheap graph-local read → fast TTS**;
  **Committed node = graft→LLM-ground → supersedes** (`Contradicts`/`Enables`). *Done-when:* a turn
  emits a Speculative render then a Committed render that supersedes it (mock TTS/LLM, no live deps).
- **2.3 Overt-repair primitive.** When the Committed answer contradicts the spoken Speculative ack, emit
  a repair node ("sorry — I heard X, you meant Y"), not a silent abandon. *Done-when:* contradiction →
  repair node emitted + rendered.
- *Dep:* P0, P1.

## Phase 3 — Native components → impulse sources (`clawft-voice-onnx`)
Behind the existing `clawft_channels::voice` traits; `ort`/sherpa-onnx ONNX; cross-platform.
- **3.1 ECAPA speaker — DONE** (`clawft-voice-onnx`, `3ae13466`/`36821947`): native 192-d, parity vs
  SpeechBrain. → feeds per-speaker `CrossRef`.
- **3.2 parakeet STT.** `SttBackend` over parakeet ONNX (sherpa-onnx/`ort`); partials → Speculative-node
  metadata updates; final → EOU commit. *Done-when:* loads ONNX, sanity + (≈ lab) parity, `#[ignore]` live.
- **3.3 smart-turn endpoint.** `EndpointModel` over smart-turn-v3 ONNX (in HF cache) → `EndOfUtterance`
  impulse. *Done-when:* EOU classification test; heuristic fallback retained.
- *Dep:* P0.1 (impulses to emit into).

## Phase 4 — Native TTS as node-renderers (`clawft-voice-*`) — no Python
- **4.1 SNAC decoder (`ort`).** SNAC-24kHz token→audio in native Rust. *Done-when:* decodes a token
  fixture to PCM.
- **4.2 Orpheus (slow).** Ollama `:11434/api/generate` (`<|audio|>dan: …`, raw+stream) → SNAC decode →
  streamed `TtsChunk`s. *Done-when:* text→gap-free chunks, barge-in flush, `#[ignore]` live-Ollama.
- **4.3 Fast layer.** Native ONNX TTS (Kokoro/Piper via sherpa-onnx) for the Speculative ack; Chatterbox
  staged behind the same `TtsEngine` trait when an ONNX export exists. *Done-when:* <1s TTFA fixture path.
- *Dep:* P2.2 (render-from-node consumes these).

## Phase 5 — VoiceChannel: cpal → AEC → impulse emitter (`clawft-voice-aec` + `clawft-channels`)
The channel **does not own logic — it emits impulses** (ADR-062). cpal capture → `clawft-voice-aec`
AEC3 → 16k mono `i16` frames → VAD/endpoint → `ImpulseQueue.emit`. cpal playback `TtsSink`.
- *Done-when:* mic frames produce VAD/EOU impulses; a `TtsChunk` plays to the speaker; barge-in flush wired.

## Phase 6 — Assembly + live end-to-end
- **6.1** `weft talk` wires P2 (tick service) + P3/P4 (components) + P5 (channel) + `LocalProvider`→Hermes.
- **6.2** Live spoken conversation on local Hermes: interruptible (barge-in prunes), speaker-named,
  grounded by the forest walk, with the Speculative→Committed handoff audible. *Dep:* P0–P5 + live Hermes
  (`:8090`) + mic/speaker + STT/TTS/ECAPA weights (all present on this host).

## Deferred backlog (post-v0.1 — schedule after its dep)
- **Weaver-learned floor/coherence weights** (`WeaverEngine::ModelingSession`) — turn the P0.3/P0.4
  constants into learned, confidence-scored edges. *Dep:* P2.
- **VAP** (backchannel/interruption split + early turn prediction; 2-channel) — the named work that makes
  `TurnClaim` vs `Backchannel` predictive, not heuristic. Weights academic-only → retrain. *Dep:* P3.3.
- **hierarchical-EOT bench** (arxiv 2603.13379, ~36ms vs smart-turn ~800-1300ms; backchannel +
  primary-speaker built in) — bench in the `~/llm` lab as a smart-turn successor. *Dep:* lab.
- **Multiplayer N-speaker floor** + primary-speaker segmentation (ADR-042 Generate mode shares this).
- **Emotion/Script cross-refs** (VAD→voice params, `inject_impulse` in-flight rebase) — the annotator
  layer. *Dep:* P2.
- **Full `select_merge_strategy` set** (Sequential/Interleave/SemanticDedup; v0.1 = Priority only).
- **sheaf-Laplacian coherence** (not WASM-ready) — keep the weighted formula.
