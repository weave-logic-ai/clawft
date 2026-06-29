# Voice ↔ ECC Synthesis: clawstage's shape on the WeftOS ECC kernel

> **Status:** Analysis + planning (no code changes). Author: WeftOS ECC Weaver/Analyst.
> **Ratified by:** ADR-058/059/060/061 (Accepted 2026-06-28). The **ECC mapping (§A, §B)
> is canonical** — it's how the agent loop uses the kernel's causal model. Where the
> *audio/TTS/AEC picks* in §C/Addendum differ from the ADRs, **the ADRs win**: AEC =
> `clawft-voice-aec` (cpal + WebRTC AEC3), NOT Apple VPIO (ADR-061); live TTS =
> Chatterbox (fast) + Orpheus (slow), NOT Supertonic/IndexTTS (ADR-061); embedder =
> Qwen3-Embedding-0.6B (ADR-059); serving = Hermes 4.3-36B (ADR-060). Treat §C as the
> research trail, the ADRs as the decisions.
> **Date:** 2026-06-28
> **Scope:** Maps the clawstage conversation-engine design onto the real WeftOS ECC kernel,
> reframes a voice conversation as an ECC Loom, modernizes the stale `voice_development.md`
> + clawstage audio choices against current Apple-Silicon research (from the `llm` lab), and
> lays out a phased landing plan.
>
> **Three source bodies, three roles:**
> - **`/Users/mathewbeane/weftos/crates/clawft-kernel/src/`** — the *real* ECC kernel we build on (production Rust types).
> - **`/Users/mathewbeane/dev/mentra/clawstage/docs/`** — the *shape* (the conversation-engine design we take after, now being modernized).
> - **`/Users/mathewbeane/llm/`** (`docs/adr/0017`, `0018`, `bin/voice-*`) — the *lab*: a working local-Mac cascade + research that updates the stale choices.

---

## 0. The punchline up front

clawstage was **already an ECC** in everything but name. Its `ConversationNode` is
`BLAKE3(0x01 ‖ actor_id ‖ hlc_timestamp ‖ content_hash ‖ parent_id)` — which is *byte-for-byte the
construction* of `crossref::UniversalNodeId::new()` in the kernel
(`crates/clawft-kernel/src/crossref.rs:71`). clawstage's "impulse events", "witness chain with score
deltas", "cross-ref segment", and "ruvector HNSW" are the four ECC structures the kernel already
ships as `CausalGraph`, `ChainManager`, `CrossRefStore`, and `HnswService`. The kernel even absorbed
clawstage's *engine vocabulary directly* into `CrossRefType` (`crossref.rs:125`): `EmotionCause`
(emotion engine), `GoalMotivation` (DSTE), `SceneBoundary` (script engine), `TomInference` (theory of
mind), `MemoryEncoded` (episodic memory).

So this is **not a port — it's a homecoming.** clawstage's six WASM engines collapse into ECC
operations on the native Rust kernel, and "voice conversation" becomes just another **Loom** (the
kernel's word for "the combined ECC structures for one domain" — `.claude/skills/weftos-ecc/WEAVER.md:49`).

---

## A. The lineage — clawstage design → WeftOS ECC types

clawstage ran six engines (DCTE/DSTE/RSTE + Emotion/Script/Scoring) inside a WASM worker, persisted to
a hand-rolled `.rvf` container. WeftOS already has the production substrate. The mapping:

| clawstage concept (file) | WeftOS ECC type (file:line) | Notes |
|---|---|---|
| `ConversationNode` / DCTE "Bidirectional Temporal Merkle DAG" (`docs/engines/02-DCTE.md`) | `causal::CausalNode` + `CausalGraph` (`causal.rs:102`, `:123`) | DashMap forward/reverse adjacency; same DAG role |
| `UniversalNodeId = BLAKE3(0x01‖actor‖hlc‖content‖parent)` | `crossref::UniversalNodeId::new(tag, ctx, hlc, content_hash, parent)` (`crossref.rs:71`) | **Identical hash construction.** clawstage's `engine_tag` byte = kernel's `StructureTag::as_u8()` |
| DAG `EdgeKind` (Follows / Contradicts / Elaboration / Cause / Evidence) | `causal::CausalEdgeType` (`causal.rs:36`): `Causes, Inhibits, Correlates, Enables, Follows, Contradicts, TriggeredBy, EvidenceFor` | clawstage's RST relations land as edge types + cross-refs |
| **Impulse stage** ("impulse generation ~1ms", `IMPULSE_SEG 0x8D`) | `impulse::ImpulseQueue` + `ImpulseType` (`impulse.rs:24`, `:114`) | `emit()` / `drain_ready()` sorted by HLC; the inter-structure event bus |
| **`CROSS_REF_SEG 0x8E`** (cross-engine links: EmotionCause, GoalMotivation, SceneBoundary, ToM) | `crossref::CrossRefStore` + `CrossRefType` (`crossref.rs:125`, `:190`) | **Kernel already has these exact variants.** Fwd/reverse DashMap index |
| **ruvector HNSW** (VEC_SEG + INDEX_SEG, "clawstage-vector") | `hnsw_service::HnswService` (`hnsw_service.rs:115`) — `insert()`, `search()`, `search_dedup()` | Semantic recall over node content; topic clustering |
| **Witness chain** (`WITNESS_SEG`, append-only hash-chained score deltas) | `chain::ChainManager` (`chain.rs:930`) — `append()`, `append_idempotent()`, `sequence()`, `witness_count()`, `append_loggable(&dyn ChainLoggable)` | Provenance / `chain_seq` already threads through every `CausalEdge` (`causal.rs:89`) |
| **Resource / actor tree** (`ActorTree`, `GlobalState`) | `tree_manager::TreeManager` (`tree_manager.rs:71`) + `StructureTag::ResourceTree` | Per-actor / per-channel structure tree |
| **50ms tick orchestration** (`symposium/02-dynamics-orchestration.md` — "sequential within 50ms tick") | `cognitive_tick::CognitiveTick` (`cognitive_tick.rs:92`), default `tick_interval_ms: 50` (`:41`) | **Same 50ms heartbeat.** Adaptive interval + drift detection built in |
| **Scoring system** (`ScoredWitnessEntry`, per-engine deltas, `docs/engines/05-scoring-system.md`) | `cognitive_tick::run_democritus_loop` two-tier coherence (`cognitive_tick.rs` header) + `causal::spectral_analysis` (`causal.rs:795`) + `eml_coherence.rs` | DEMOCRITUS: O(1) EML predict every tick, Lanczos spectral fallback on drift — the native form of clawstage's per-tick scoring/coherence |
| **Floor Manager** (`FloorState`, `compute_urgency`, `docs/engines/02-DCTE.md §8`) | *Pattern over the kernel* — coherence/impulse scoring on the `CognitiveTick`; no dedicated type yet (build target, see §C) | Urgency = a scored function read off `ImpulseQueue` + `CoherenceAlert` impulses |
| **Emotion engine VAD state** (`symposium/07`) | `CrossRefType::EmotionCause` + node `metadata` JSON (`CausalNode.metadata`, `causal.rs:110`) | VAD triple stored as node metadata; emotion→action as cross-ref |
| **Script engine** (plot hooks, beats, `symposium/08`) | `CrossRefType::SceneBoundary` + `ImpulseType::Custom(u8)` injected impulses | "inject_impulse" stage direction = an emitted `Impulse` |
| **RVF `.rvf` container** (`docs/engines/06`) | kernel persistence: `CausalGraph::save_to_file` (`causal.rs:1675`) + `ChainManager` + `eml_persistence.rs` | Per-structure persistence replaces the monolithic segmented file |
| **The Weaver / "who improves the model"** (implicit in clawstage) | `weaver::WeaverEngine` (`weaver.rs:1142`) + `ModelingSession` (`weaver.rs:165`) | HYPOTHESIZE→OBSERVE→EVALUATE→ADJUST loop, meta-Loom (`weaver.rs:309` `MetaLoomEvent`) |

**Key insight on the Weaver:** clawstage had no equivalent of `WeaverEngine`. In WeftOS, the Weaver is
what makes the floor/emotion/transcript-gate *learnable* rather than hand-tuned: a `ModelingSession`
(`weaver.rs:165`) over the conversation Loom can learn, e.g., that `Interruption` impulses with high
arousal metadata reliably `Causes` floor loss — turning clawstage's hardcoded `compute_urgency`
weights (semantic 0.30 / arousal 0.15 / wait 0.20 / crowd 0.10 / readiness 0.25) into measured,
confidence-scored edges. The floor manager's weights become **emergent edge weights the Weaver tunes.**

---

## B. A voice conversation **as an ECC Loom**

A "Loom" is the combined `{CausalGraph, HnswService, CrossRefStore, ChainManager}` for one domain
(here: one conversation/`VoiceChannel`). The conversation grows as a causal DAG; the `CognitiveTick`
(50ms) drives floor/coherence decisions; impulses carry the real-time turn-taking signals.

### B.1 The object model

| Conversation thing | ECC representation |
|---|---|
| **A turn (completed utterance)** | a `CausalNode` (`add_node(label=transcript, metadata={actor, vad, audio_profile, embedding_id})`), `link()`ed to the previous turn with `CausalEdgeType::Follows` (weight = coherence). Committed to the witness chain via `chain_seq`. |
| **A reply that answers a turn** | edge `EvidenceFor` / `Causes` (answer) or `Elaborates` cross-ref to the question node |
| **An interruption (barge-in turn-claim)** | an `Impulse { impulse_type: Custom(0x50 "TurnClaim"), payload: {urgency, overlap_ms} }` → on grant, the new node links to the cut-off node with `CausalEdgeType::Contradicts` (it competes with / overrides the in-progress turn) |
| **A backchannel ("mm-hmm", "yeah")** | **NOT a turn / NOT a new frontier node.** A `CrossRef { ref_type: Custom(0x60 "Continuer") }` (or `EvidenceFor` with low weight) pointing at the *current speaker's* node. It encourages, it does not take the floor. This is the single most important modeling decision (see VAP, §C). |
| **Floor state** | derived, not stored: a scored read over the `ImpulseQueue` on each tick (Open / Held / Contended / Locked) |
| **Emotion (VAD)** | node `metadata.vad`; emotion→action as `CrossRefType::EmotionCause` |
| **A goal / intention** | `CrossRefType::GoalMotivation` linking an intention node to the utterance that serves it |
| **Theory-of-mind belief** | `CrossRefType::TomInference` |
| **A scripted stage direction** | injected `Impulse` + `CrossRefType::SceneBoundary` |

### B.2 How one turn flows through the kernel (the tick loop)

```
mic frames ─▶ [AEC: Apple VPIO] ─▶ [Silero VAD] ─▶ 2-ch (bot+user) ─▶ [VAP] ─┐
                                                                              │  turn-shift? backchannel?
  [smart-turn v3] end-of-utterance ◀──────────────── stable ASR prefix ◀── [Parakeet-MLX stream]
        │ EOU                                                                   │ partials
        ▼                                                                       ▼
  ImpulseQueue.emit(Custom(0x51 "EndOfUtterance"))            (partials update a Speculative node's metadata,
        │                                                       NOT a committed CausalNode — clawstage's
        ▼                                                       "partial updates pending node on private branch")
  ── CognitiveTick (every 50ms): drain_ready() ──────────────────────────────────────────────
        1. EOU impulse + VAP turn-shift  → floor decision (scored urgency over the queue)
        2. CausalGraph.add_node(transcript, {actor, vad, audio_profile})   ← commit the turn
        3. CausalGraph.link(prev_turn → new_node, Follows, weight=coherence)
        4. HnswService.insert(node_id, embedding, meta)                    ← semantic index
        5. run_democritus_loop: O(1) EML coherence predict; Lanczos spectral on drift
           → low coherence ⇒ emit ImpulseType::CoherenceAlert  (the "transcript gate" verdict)
        6. ChainManager.append_loggable(turn_event)                       ← witness + chain_seq
        7. CrossRefStore.insert(EmotionCause / GoalMotivation / ToM as inferred)
        8. WeaverEngine (if a ModelingSession is active): observe → adjust edge weights
        9. Talk-Mode controller reads floor verdict ⇒ start/stop/duck TTS
   ──────────────────────────────────────────────────────────────────────────────────────────
        ▼
  [Kokoro / mlx-audio streaming TTS] ─▶ speaker   (cancellable; barge-in stops it < 50ms)
```

- **Backchannel vs interruption** is decided at step 1 from the **VAP** signal: a *continuer*
  prediction → write a `Continuer` cross-ref (step 7), **skip steps 2–3** (no new turn, TTS keeps
  going); a *turn-shift/overlap-claim* prediction → `TurnClaim` impulse, floor contends, and on grant
  the new node links `Contradicts` to the interrupted node and TTS is cancelled.
- **The "DTE Transcript Gate"** (clawstage's scored merge decision) = step 5's coherence verdict:
  a `CoherenceAlert` impulse means "this utterance doesn't cohere — hold/clarify rather than commit to
  the main line," exactly like clawstage routing a low-score transcript to a private branch instead of
  merging to `main_line`.
- **Floor management** lives entirely in the impulse-scoring of step 1 — no separate engine. The
  urgency function (clawstage's `compute_urgency`) becomes a scored read over `ImpulseQueue` entries,
  with weights the Weaver can learn.

### B.3 Why this is the right shape

The 50ms `CognitiveTick` already matches clawstage's 50ms orchestration tick, and `run_democritus_loop`
already implements the two-tier "cheap-predict-every-tick, expensive-verify-on-drift" pattern that
clawstage approximated with per-engine scoring. We get clawstage's whole dynamics loop **for free** by
treating audio events as impulses into the existing tick.

---

## C. Modernization — replace the stale choices

`.planning/voice_development.md` (sherpa-onnx for everything, rustpotter wake, **software-AEC-TBD**,
ElevenLabs/OpenAI cloud fallback, 9-week sprint) predates the current model landscape. clawstage's
`symposium/04-audio-realtime.md` is staler still (Web Speech API → whisper.cpp WASM → webrtc-vad →
d-vector profiler → ElevenLabs TTS, all browser-bound). The `llm` lab already built and benchmarked a
*warm local cascade* on the M5 Max (`bin/voice-live`: STT 0.18s / TTS 0.06s warm, LLM-bound) and ran
deep research → **ADR-0017** and **ADR-0018**. Adopt those findings. The kernel stays the same; only
the *audio front-end model choices* and the *orchestration ownership* change.

### C.1 STT — add MLX-native streaming alongside sherpa-rs

| Role | Adopt | License | Why over the stale pick |
|---|---|---|---|
| **Low-latency streaming (default on Mac)** | **Parakeet-MLX** (`transcribe_stream`) | CC-BY-4.0 (attrib) | MLX-native, true incremental partials; warm ~0.18s in the lab. Feeds the Speculative-node updates in §B.2 |
| Alt streaming | **nvidia nemotron-3.5-asr-streaming-0.6b** | OpenMDW-1.1 (commercial-OK) | 80ms–1.12s chunks, 40 locales; runs **CPU-side** (off the GPU/unified-mem budget — ADR-0017 resource partitioning) |
| Tiny/fast | **Moonshine** | — | very low latency option |
| High-accuracy batch | **mlx-whisper** (`bin/whisper`) | Apache-2.0 | offline / re-transcribe / round-trip WER |
| **Cross-platform fallback** | **sherpa-rs** (sherpa-onnx) | — | keep ONLY for non-Mac targets; it is no longer the Mac default |

> sherpa-rs stops being the universal answer. On Apple Silicon, MLX-native Parakeet wins on latency;
> sherpa-rs becomes the Linux/Windows portability leg.

### C.2 TTS — MLX-native sub-300ms; ElevenLabs demoted to baseline

| Role | Adopt | License |
|---|---|---|
| **Default streaming (Mac)** | **Kokoro via mlx-audio** | (have it; warm ~0.06s in lab) |
| Voice variety / word-level | **Kyutai-TTS-MLX**, **Sesame CSM** (`csm-mlx`) | CC-BY-4.0 |
| Max language coverage | **OmniVoice** (600+ langs) | Apache-2.0 |
| Commercial-safe expressive | **Chatterbox** | MIT |
| **Cloud baseline ONLY** | **ElevenLabs** (`eleven_flash_v2_5` low-latency) + **Scribe** STT | (the reference to beat, not the default) |

> clawstage hardwired ElevenLabs (its emotion engine even maps VAD → ElevenLabs `stability/style/speed`).
> Keep that VAD→voice-param mapping, but point it at **mlx-audio** locally; ElevenLabs becomes the
> latency/quality yardstick (`bin/voice-loop --tts-provider {local,elevenlabs}`), never the runtime default.

### C.3 Turn-taking — add smart-turn v3 + VAP, mapped to ECC impulses

This is the biggest upgrade over `voice_development.md` (which had only a "silence timeout").
**Three separate jobs, three components** (ADR-0018):

- **smart-turn v3** (BSD-2, **8MB ONNX** — upgrade from the v2 in ADR-0017): *semantic* end-of-utterance
  ("the user is actually done", beyond silence). → emits `ImpulseType::Custom(0x51 "EndOfUtterance")`.
- **VAP (Voice Activity Projection) via MaAI** (MIT code; pretrained weights academic-only → **retrain
  for commercial**): predicts the joint 2s future of *both* speakers, so it separates **backchannel vs
  interruption** *and* gives **early turn prediction**. It needs 2 channels (bot + user) — which a voice
  agent inherently has.
- **Silero VAD** (MIT): cheap front gate, unchanged.

**VAP/turn signals → ECC `ImpulseType` mapping** (proposed `Custom` discriminants, consistent with
WEAVER.md's reservation of `Custom(0x40)` for meta-loom):

| Turn signal | Impulse | Downstream ECC action |
|---|---|---|
| VAP continuer / backchannel | `Custom(0x60 "Backchannel")` | write `Continuer` cross-ref to current speaker node; **do not** add a turn node; TTS continues |
| VAP turn-shift (clean handover) | `Custom(0x52 "TurnShift")` | floor → Open; next utterance commits as `Follows` |
| VAP overlap / turn-claim during TTS | `Custom(0x50 "TurnClaim")` | floor → Contended; on grant cancel TTS, new node links `Contradicts` |
| smart-turn EOU | `Custom(0x51 "EndOfUtterance")` | trigger the commit path (steps 2–6) |
| Coherence drop (DEMOCRITUS) | `ImpulseType::CoherenceAlert` (native) | transcript-gate hold / clarify |

### C.4 AEC — replace "software-AEC-TBD"

clawstage said "Reference-Signal AEC"; `voice_development.md` left it **TBD** (loopback subtraction vs
WebRTC vs OS). Decide it (ADR-0018):

- **Primary (Mac): Apple Voice Processing I/O (VPIO)** — native macOS, **handles the far-end reference
  automatically** (the thing you cannot do by denoising; you can't subtract the bot's own clean speech
  without the reference). Thin Swift audio layer. This is the modern realization of clawstage's
  "Reference-Signal AEC."
- **Cross-platform fallback: WebRTC AEC3** via `livekit-rtc` (APM).
- **Complement, not replacement: ECAPA speaker-gate (SpeechBrain, Apache-2.0)** — the modern form of
  clawstage's **Speaker Profiler d-vector**. Enroll the user; gate turns on the user's embedding so the
  agent ignores its own/other voices even when AEC leaks. Speaker embedding lands as node `metadata` and
  feeds HNSW-based speaker recall.

> This kills the `voice_development.md` mic-mute stopgap (which forbids barge-in). VPIO + ECAPA = true
> barge-in: the mic stays hot during TTS, AEC removes the echo, the speaker-gate + VAP decide whether
> the residual is the user claiming the floor.

### C.5 Full-duplex alternative — Moshi (evaluate, don't bet)

**Moshi** (`moshi-mlx`, Rust core; CC-BY-4.0) is the only true *talking-while-listening* model with an
MLX port and a commercial license. It is the architectural opposite of the cascade above. **Catch: it
locks the LLM** — Moshi is welded to its own Helium-7B brain and a fixed voice; you cannot swap in a
smarter weftos-served model. **Recommendation:** stand it up as a **parallel PoC** to feel native
overlap and judge whether full-duplex is worth losing model/voice swappability. Keep the cascade as the
product line; Moshi informs whether we ever migrate.

### C.6 Orchestration — Pipecat as reference, native Rust on ECC as product

clawstage **hand-rolled** its Floor Manager + dynamics loop. The Python reference for that job is
**Pipecat** (BSD-2): it already ships interruption strategies, backchannel suppression (`MinWords`),
streaming/revisable output, and an official fully-local macOS reference app
(`kwindla/macos-local-voice-agents`, <800ms voice-to-voice). **But in weftos we do NOT adopt Pipecat as
a runtime** — the orchestration is **native Rust on the ECC kernel**: the **Talk-Mode controller** +
**`VoiceChannel`** (the `ChannelAdapter` from `voice_development.md`) + the **`CognitiveTick`** loop +
the **Weaver**. Use Pipecat only as a **design reference** for which interruption strategies and
backchannel rules to implement, and as a latency baseline (and as the *fast path* in the `llm` lab,
ADR-0018 Phase 1).

### C.7 Revisable responses + fillers (the felt-latency win)

LLM generation dominates the budget (~0.8–2.5s); STT/TTS are nearly free warm. To feel instant
(ADR-0018 Phase 4): **Incremental-Unit restart-and-revise** over *stable* ASR prefixes + speculative
LLM start on a completeness/entropy gate, committed on smart-turn EOU; a **templated filler bank**
("let me think…") spoken on EOU to mask first-token latency; use `bin/serve-llamacpp --draft`
(speculative decoding) so wasted speculative drafts are cheap. In ECC terms: speculative generation =
a **Speculative `CausalNode`** that is committed or pruned at the next tick — exactly clawstage's
`NodeState::{Speculative, Frontier, Committed, Stale, Pruned}` lifecycle, which the kernel models as
node `metadata.state` + `Contradicts`/prune on revision.

---

## D. Phased plan — landing it in weftos (and what stays in the lab)

**Division of labor:** the `llm` repo **stays the lab** — fast Python iteration, model bench/eval,
round-trip-WER + TTFA measurement, ADRs. weftos gets the **native Rust product** once a choice is
proven in the lab.

| Stays in `llm` lab (Python, throwaway-friendly) | Moves to weftos (Rust, native ECC) |
|---|---|
| `bin/voice-live`, `bin/voice-loop`, `_voiceloop.py`, `_voicelive.py` | `VoiceChannel` (`ChannelAdapter`), Talk-Mode controller |
| Per-stage venvs (`.venv-asr/.venv-vad`), model bench/eval, WER/TTFA harness | Audio events → `ImpulseQueue`; turns → `CausalGraph`; coherence on `CognitiveTick` |
| Pipecat fast-path PoC; Moshi PoC; cloud-baseline scoring | Floor controller as scored impulse read; Weaver `ModelingSession` over the conversation Loom |
| Model selection + license vetting (registry/speech.yaml) | Persisted Loom via `CausalGraph::save_to_file` + `ChainManager` |

### Phase 0 — Lab validation (in `llm`, mostly done)
Confirm Parakeet-MLX + Kokoro + smart-turn v3 warm latencies; stand up Pipecat fast-path and Moshi PoC;
measure VPIO AEC + ECAPA gate. Output: locked component choices (ADR-0017/0018 → "Accepted").

### Phase 1 — `VoiceChannel` + audio→impulse bridge (weftos)
Implement `VoiceChannel` as a `ChannelAdapter` (per `voice_development.md` VS1.3.1) but with a **new
mandate**: it does not own logic, it **emits impulses**. cpal/CoreAudio capture → VPIO AEC → Silero VAD
→ Parakeet-MLX partials. EOU/VAD/VAP events → `ImpulseQueue.emit(...)`. Completed turns → `CausalGraph`
nodes with `Follows` edges. Feature-flag behind `voice` (keep the VP3 flag scheme). *Smallest correct
foundation.*

### Phase 2 — Talk-Mode controller on the `CognitiveTick`
Wire the tick loop (§B.2): drain impulses, run the floor decision, commit turns, run `run_democritus_loop`
for the coherence/transcript-gate verdict, drive streaming Kokoro TTS with cancellation. Implement
**barge-in** via VPIO + ECAPA speaker-gate (kills the mic-mute stopgap). Backchannel-vs-interruption
from VAP signals → the `Custom(0x50/0x52/0x60)` impulse mapping (§C.3).

### Phase 3 — Cross-engine richness (emotion / script / ToM as cross-refs)
Port clawstage's emotion (VAD in node metadata, `EmotionCause` cross-refs, VAD→mlx-audio voice params),
script (injected impulses + `SceneBoundary`), and ToM (`TomInference`) — all as **cross-refs + impulses
on the existing Loom**, no new engines. This is where clawstage's `symposium/07`/`08` designs return,
now native.

### Phase 4 — Weaver-learned floor/coherence
Start a `WeaverEngine::ModelingSession` over the conversation domain so the floor-urgency weights and
coherence thresholds become **learned, confidence-scored edges** instead of clawstage's hardcoded
constants. Meta-Loom (`weaver.rs:309`) tracks the tuning. Export a `weave-model.json` for edge/offline
deployment.

### Phase 5 — Felt-latency + full-duplex decision
Incremental restart-and-revise (Speculative nodes) + templated fillers + speculative LLM via
`serve-llamacpp --draft`. Decide Moshi vs cascade from the Phase-0 PoC. Cross-platform leg: swap
Parakeet/VPIO for sherpa-rs + WebRTC AEC3 on Linux/Windows behind the same `VoiceChannel`.

---

## Appendix — exact kernel anchors (for implementers)

- `causal.rs:36` `CausalEdgeType` (Follows/Contradicts/EvidenceFor/Causes…) · `:102` `CausalNode` ·
  `:123` `CausalGraph` · `:168` `add_node` · `:249` `link` · `:795` `spectral_analysis` ·
  `:1675` `save_to_file`
- `impulse.rs:24` `ImpulseType` (BeliefUpdate/CoherenceAlert/NoveltyDetected/EdgeConfirmed/EmbeddingRefined/**Custom(u8)**) ·
  `:114` `ImpulseQueue` (`emit`, `drain_ready`)
- `crossref.rs:21` `StructureTag` · `:71` `UniversalNodeId::new` (the BLAKE3 construction) ·
  `:125` `CrossRefType` (EmotionCause/GoalMotivation/SceneBoundary/MemoryEncoded/TomInference/…) ·
  `:190` `CrossRefStore`
- `hnsw_service.rs:115` `HnswService` (`insert`, `search`, `search_dedup`)
- `chain.rs:930` `ChainManager` (`append`, `append_idempotent`, `sequence`, `witness_count`, `append_loggable`)
- `cognitive_tick.rs:92` `CognitiveTick` (default 50ms, adaptive, drift) · `run_democritus_loop` (two-tier coherence)
- `weaver.rs:1142` `WeaverEngine` · `:165` `ModelingSession` · `:309` `MetaLoomEvent`
- `service.rs:108` `SystemService` trait · `:22` `ServiceType`

**clawstage source-of-shape:** `docs/engines/02-DCTE.md` (tree + floor §8), `03-DSTE.md` (BDI/ToM),
`04-RSTE.md` (coherence formula), `05-scoring-system.md` (`ScoredWitnessEntry`), `06-RVF-implementation.md`
(segments), `symposium/02-dynamics-orchestration.md` (50ms tick), `04-audio-realtime.md` (the stale
audio pipeline), `07-emotion-engine.md` (VAD), `08-script-engine.md` (plot hooks).

**Lab source-of-modernization:** `/Users/mathewbeane/llm/docs/adr/0017-voice-pipeline.md`,
`0018-streaming-conversational-voice.md`; runners `bin/voice-live`, `bin/voice-loop`, `bin/turn`,
`bin/asr`, `bin/tts`, `bin/whisper`; registry `docs/models/registry/speech.yaml`.

---

## Addendum — Tiered streaming + multiplayer (the weftos voice-agent shape)

Two refinements the ECC graph makes natural (from lab discussion, 2026-06-28):

### Tiered pipeline: a SPEED tier + a POWER tier at every stage
Don't pick one model per stage — run two, and let the fast one buy time for the powerful one.
- **STT:** fast streaming (parakeet-mlx / nemotron) reacts off *partials* → immediate "hold on, looking
  at X, Y, Z…"; powerful (whisper-large-v3) produces the *accurate final* the considered answer uses.
- **LLM:** small/fast model emits the filler/acknowledgment; powerful model streams the substantive reply
  piece by piece (incremental/revisable = IU add/revoke/commit over stable STT prefixes).
- **TTS:** **Piper** speaks the filler at <100ms TTFA; CosyVoice/Higgs speaks the considered reply, chunked.

ECC encoding: the filler is a **low-confidence Speculative CausalNode spoken immediately**; the power
path emits **committed nodes** that supersede/extend it (`Contradicts`/`Enables` edges). Fast-tier
**TTFA = felt latency**; power-tier quality = the real answer. The lab harness records `ttfa_ms` so the
speed tier (Piper) is ranked on exactly that.

### Multiplayer: N participants, not 1:1
The conversation is a shared `CausalGraph` with **N speakers — humans AND agents** — i.e. clawstage's
multi-actor DTE, already generalized in the ECC kernel. Each participant = a channel; turns/backchannels/
interruptions = per-speaker nodes + impulses; the **Floor Manager = coherence/impulse scoring across all
speakers** (VAP per relevant pair). Because it's a graph, "who said what, to whom, on which branch" is
directly renderable in weftos surfaces — the visibility that makes this tractable.

Division of labor: the **`llm` lab harness ranks the COMPONENTS** (speed vs power tier per stage, multi-
party turn models, TTFA/WER/A-B); **weftos ECC is the tiered + multiplayer ORCHESTRATION** (Talk-Mode +
Weaver + Floor over the CausalGraph).

### Full participant matrix: {0..N humans} × {0..M agents}
The ECC CausalGraph is participant-agnostic — a speaker is a channel/node-source, human or agent. Four
quadrants, two of them distinct engines:
- **1 human + 1 agent** — classic assistant (Jarvis).
- **N humans + M agents** — meeting copilot / multi-party.
- **0 humans (all agents)** — **autonomous swarm dialogue**: agents need the full turn-taking apparatus
  (don't talk over each other, backchannel, yield, interrupt). This is ruflo/hive reframed as
  conversation (the Weaver's "all activity is conversation"). Fully testable in the lab with NO humans
  (2+ agent voices conversing) — a clean way to stress floor management.
- **0 agents (all humans)** — the **observation / training corpus**, the self-improvement flywheel:
  1. **Training data:** VAP is self-supervised on dyadic human audio, and our VAP weights are
     academic-only → we must retrain on licensable/own data anyway. Real human multi-party conversation
     IS that data (also tunes floor/backchannel models).
  2. **Eval ground truth:** real human turn-taking/backchannels are the labeled set the eval plan would
     otherwise hand-curate. Recording gives training + yardstick at once.
  3. **Weaver fuel:** feeding the ECC Weaver real dialogue improves how the agent models conversation.
  Loop: observe humans → learn better turn-taking + conversation models → agents converse more naturally
  → record those too. Capture substrate already exists (clawstage RVF/witness → ECC `ChainManager`).

Implication for the lab: add an **agent↔agent conversation mode** (N LLM voices, no mic) to stress
turn-taking, and a **conversation-ingest path** (record/replay real human audio → train/score turn models).

### Annotator layer — affect / prosody / vision / ToM on conversation nodes
Beyond words + turn-taking, utterance nodes carry richer signals — already first-class in ECC and
designed in clawstage:
- **ECC destinations (crossref.rs):** `EmotionCause`, `TomInference` (theory-of-mind / speaker state),
  `GoalMotivation`, `SceneBoundary`.
- **clawstage precedent:** Emotion Engine (valence/arousal → voice_settings), Speaker-Profiler
  (pitch/F0/speed/volume/clarity/pauses → SpeakerProfile on every node), RSTE (rhetorical/discourse
  structure, cross-speaker + floor-crossing relations).
- **Annotator stages to add (lab + weftos):** speech-emotion-recognition (SER) + paralinguistics from
  voice; prosody/inflection; face/vision-emotion + gesture (multimodal/VLM); multimodal affect fusion;
  optional bio-signals. Each runs alongside STT, emitting metadata → ECC `EmotionCause`/`TomInference`
  crossrefs on the utterance node. Affect-aware turn-taking (VAP + emotion) and emotional TTS
  (voice_settings from valence/arousal) fall out of this.
- **Two gaps to fill:** (1) the model catalog has no SER/prosody/affect/vision-emotion entries — research
  + register them; (2) the vector store is unbuilt — ingest registry + clawstage/weftos voice docs + ADRs
  so this analysis is actually searchable (install mlx-embeddings, `bin/vsearch ingest`).

> **SCOPE (per user, 2026-06-28):** the annotator/affect/prosody/vision/ToM/bio-signal layer is
> **weftos-NATIVE, not built into the `llm` test harness.** The harness's job is ranking the core
> swappable voice *components* (STT/TTS/turn/floor/AEC/speaker/LLM) by latency/WER/A-B. The rich
> multimodal+affect+ToM analysis needs the ECC graph, the multi-stream/multi-participant substrate,
> cross-modal fusion on the CognitiveTick, and sensor/bio ingestion — which weftos has and a single-shot
> bench does not. The harness MAY optionally bench an individual analyzer model (e.g. an SER model's
> accuracy/latency) to hand weftos a vetted pick, but it does not compose the layer.

### Two-layer voice (fast clone + slow expressive clone) — same identity
Decision (lab, 2026-06-28): the voice tier is TWO engines cloning ONE shared reference voice, so they're
seamlessly the same person:
- **FAST layer (primary, real-time):** lowest-latency engine that can CLONE. Pick = **Supertonic-3 +
  cloned voice_style** (supertonic.embed: 3-10s ref -> WavLM-embedding voice_style JSON; ~0.34s TTFA,
  clean, deep). Handles immediacy: fillers, acknowledgements, first words/sentences.
- **SLOW layer (buffered, ~10-30s OK):** best-in-class EXPRESSIVE cloner. Lead candidate **IndexTTS-2**
  (SOTA WER + speaker-similarity + emotional fidelity, Qwen3 soft emotion-instruction); alt **Higgs
  Audio 2**. Crafts the rich, considered, emotionally-performed response.
- Both clone the SAME reference -> identical timbre. ECC mapping: fast layer = the Speculative spoken
  node (immediate); slow layer = the Committed node that supersedes/extends it. Fits the agent
  lifecycle — the agent buffers the expressive response ahead of time / while the user is still talking,
  fast layer covers the gap. The fast layer is the non-negotiable pick (speed > all); the slow layer is
  the quality pick. Reference voice is swappable (any 3-10s sample; clone synthetic/own voices, not
  copyrighted celebrity recordings).
