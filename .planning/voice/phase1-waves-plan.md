# Voice Phase 1 — three-wave plan (observe → converse → duplex)

Companion to **ADR-068** and `.planning/voice/duplex-edge-agent-plan.md`. DESIGN/PLANNING
only — no code lands from this file. Line anchors are current as of the 2026-07-05 recon
(HEAD `e7460f66`, branch `feat/hermes-loop-base`) and will drift; treat as landmarks.
All paths absolute-from-repo-root.

**The restructuring (user directive, 2026-07-05).** ADR-068's Phase 1 goes straight to the
streaming-edge duplex refactor. This plan inserts an **observation-first, half-duplex
validation stage in front of it**, in three user-defined waves:

- **Wave 1 — Complete STT + voice decomposition + the surface.** Get STT ingestion with
  **full labeling, recognition, and a complete per-utterance acoustic/prosodic/paralinguistic
  decomposition** working, and surface the whole process live so the user can run tests, act
  out emotions, and watch every stage. Listen-only. **This wave ships the complete
  `VoiceAnalysis` record**, not a first-pass extractor — every field present with an honest
  confidence flag, structure complete so later upgrades refine values without reshaping
  storage or surface.
- **Wave 2 — Wire into the agent loop.** Route finalized voice turns into the agent; the
  agent replies as **text** on the same surface. No audio out; the surface shows the full loop.
- **Wave 3 — Full duplex, and only that.** TTS out + DuplexChannel floor + ERL + barge.
  Every audio-out / interrupt problem isolated here. **Wave 3 == ADR-068 Phase 1.**

**Why this ordering de-risks the feature.** Waves 1–2 reuse the **already-working in-process
native capture** (`weft voice talk`) and the **already-wired `agent.turn.record` →
daemon `index_turn` commit+classify seam**, touching no transport/floor/TTS. The
finalized-turn contract Wave 1 targets (`role`, `text`, `speaker`, `emotion`,
`voice_analysis`) is the **same seam** a future streamed edge feeds (`index_turn` is
modality-neutral, ADR-068 D6), so nothing built in Waves 1–2 is thrown away by Wave 3 —
and the Wave 1 surface becomes Wave 3's floor/barge debugging instrument.

---

## 0. Ground truth the design stands on (2026-07-05 API audit)

| Fact | Location | Consequence |
|---|---|---|
| `weft voice talk` runs the full in-process native stack (parakeet TDT + smart-turn + ECAPA + TTS + Hermes) and forwards finalized turns to `agent.turn.record` via a `ConversationObserver` | `crates/clawft-cli/src/commands/voice.rs:85,190,220`; `run_live_observed(...extra_observer)` `voice-talk/src/live.rs:62` | Wave 1 reuses this capture; builds no transport |
| **STT returns a bare `String`** — `SttBackend::transcribe -> Result<String>`; substrate HTTP contract is `{"text":"…"}` only | `channels/src/voice/stt.rs:64,135` | Per-token data must be surfaced by a richer return type (§W1.3); substrate path stays text-only |
| **But the native TDT decode already computes per-token encoder-frame index `t` + duration `skip`**, and the joiner emits full token logits | `voice-onnx/src/parakeet_tdt.rs:106-146` (loop tracks `t`; `argmax(&logits[..n_tokens])`) | Per-token **timestamp, duration, and confidence (softmax of the kept logits)** are all recoverable at ~zero cost on the native path |
| **No running partial transcript today** — the endpointer is called with `partial_text=""`; STT runs once on the finalized utterance | `channels/src/voice/talkmode.rs:314` (`observe(&frame, voiced, "")`), finalize at `:360` | Partials are a *medium* item (needs incremental decode); the surface degrades to level-meter + finalized line if deferred |
| Smart-turn `completion_prob ∈ [0,1]` (ONNX sigmoid output, or heuristic fallback) computed then **discarded** after the finalize decision; `is_runtime_available()` says which path | `voice-onnx/src/smart_turn.rs:116,136`; `channels/src/voice/turn.rs:35,168` | Capture the prob + its source as endpoint "related data" |
| ECAPA → 192-d L2-normalized d-vector; `SpeakerMatch{id, score=cosine}`, threshold ~0.45, `identify_or_enroll`; **score dropped before `UserTurn`** | `voice-onnx/src/ecapa.rs:28` (`EMBED_DIM=192`); `channels/src/voice/speaker.rs:54,121,166` | Thread cosine score + enrollment action onto the record |
| VAD gives `SpeechStart{at_sample}` / `SpeechEnd{at_sample}` (utterance = `[start,end]`), a voiced-sample counter, `EnergyVad::rms_dbfs`, and `NoiseFloor::floor_dbfs` | `channels/src/voice/vad.rs:19,25,88,177` | **Duration, voiced/silence split, RMS envelope, floor, and SNR = rms−floor are all free** |
| `ConversationEvent` has 5 coarse, finalized variants — **no partials, no endpoint-fire, no speaker score, no prosody** | `channels/src/voice/talkmode.rs:105` | Wave 1 enriches this enum with process events (§W1.4) |
| Keyword turn-classifier **implemented + wired**, dormant by default; `dual_write_turn` writes `classification`+`text`+`Emotion/Goal` crossrefs when the blob is `Some`; `tier:"voice"` emotion slot reserved, **no producer** | `service-agent/src/turn_classifier.rs`; `session_forest.rs:140`; gated `daemon.rs:1232`; `Tier::Voice` | Wave 1 enables classification (free 4-axis) and builds the first `tier:"voice"` producer |
| `conversation.graph {conv_id,since?,window?}` serves per-node `{id,role,text,state,chain_seq,ts_ms,classification}` verbatim from metadata, `conv_id`-scoped; speaker is a `Speaker` crossref edge | `weave/src/daemon.rs:5937,6009-6022` | Add one verbatim `voice_analysis` read; the ADR-067 GUI graph lights up from the same data |
| Model-dir pattern: `~/.weftos/models/{parakeet,ecapa,smart-turn,kokoro,snac}/…`, env override, **graceful degrade to a no-model fallback** | `smart_turn.rs:31-33,167`; `ecapa.rs` | A **SER ONNX** slots into this exact pattern (`~/.weftos/models/ser/`, env override, DSP fallback) |

---

## Wave 1 — complete STT + voice decomposition + the process surface (detailed design)

**Goal (user).** "Wave 1 has to have complete decomp and analysis of STT and voice." Full
per-utterance recognition + acoustics + prosody + paralinguistics, one structured record,
surfaced live so the user can act and watch every stage. Listen-only; no reply, no audio out.

### W1.1 — Topology: reuse the in-process native session + `agent.turn.record` (RECOMMENDED)

Reuse the current in-process native capture; push finalized turns via `agent.turn.record`
(with the new `voice_analysis` blob, §W1.2). **Not** a new thin streaming edge.

- **Already wired end-to-end**, so Wave 1 turns on classification and enriches the observer;
  it builds no transport. The streaming edge / two-lane wire / floor / TTS-down are *Wave 3*.
- **Zero throwaway vs ADR-068:** the invariant contract both topologies feed is a finalized
  turn at `index_turn`; Wave 1's daemon-side work (commit, classify, store the record, serve
  it) is topology-agnostic and survives Wave 3 unchanged.
- **The in-process session is the *right* place to source the decomposition**, not merely the
  fastest: every signal (per-token logits, endpoint prob, ECAPA score, raw PCM for prosody)
  lives next to the frames. A streaming edge would have to ship all of it up and back — strictly
  more work for the exact thing this wave needs most. **Full decomposition *wants* the fat
  local session**; it is computed edge-side and travels up as the `voice_analysis` blob.

Enable path: daemon with `[kernel.agent.classification] mode="keyword"` + the ECC loop on,
`weft voice` connected as recorder, a **listen-only** entry point (§W1.4) that skips the LLM
brain and only records + classifies + stores the record.

### W1.2 — The `VoiceAnalysis` record: complete per-utterance decomposition

One versioned, `tier:"voice"` record per utterance. **Every field present with an honest
confidence flag** — a field may be present-but-coarse, but the structure is complete so a
later SER model or incremental decoder refines *values* without reshaping storage or surface.

#### Signal enumeration by cost (audit-grounded, honest tiers)

**CHEAP — already computed somewhere; capture it (no new algorithms):**
- **Energy/loudness** — `EnergyVad::rms_dbfs` per frame → utterance RMS mean/peak, envelope.
- **Noise floor + SNR** — `NoiseFloor::floor_dbfs`; `snr_db = rms_mean − floor`.
- **Timing** — `SpeechStart/End.at_sample` → `duration_ms`; voiced-sample counter →
  `voiced_ms` / `silence_ms`; onset→finalize wall clock → `endpoint_latency_ms`; `transcribe`
  wrapped in `Instant` → `stt_latency_ms`.
- **Endpoint** — smart-turn `completion_prob` + `is_runtime_available()` source
  (`smart-turn-v3` | `heuristic`) + silence tail.
- **Speaker** — ECAPA `SpeakerMatch{id,score}` cosine vs enrolled centroid, `threshold`,
  enrollment `action` (identified | enrolled | unknown), `embedding_dim=192`.
- **Capture health** — clipping % and DC offset from the i16 frames (one pass; not done today).

**MEDIUM — compute from data already present (DSP / bookkeeping, no model):**
- **Per-token timing + duration + confidence** — from the TDT decode: capture `t` (encoder
  frame → ms via the subsampling stride) and `skip` at each `out.push(y)`, and
  `softmax(logits[..n_tokens])[y]` as per-token confidence. Yields `token_conf_mean/min`.
  *(Native path only — §W1.3; substrate path emits `tokens: []`, honestly flagged.)*
- **f0 / pitch track** — greenfield autocorrelation/YIN over the utterance (~150 lines, no
  model): `f0_mean/min/max_hz`, `f0_range_semitones`, `f0_slope`.
- **Speaking rate** — token/word count over `voiced_ms` → `rate_tokens_per_s`; with token
  timings, a syllable-ish rate.
- **Pause structure** — intra-utterance silences from the per-frame voiced flag (the
  endpointer already runs a `silence_run`) → `pause_count`, `pause_mean_ms`.
- **Energy dynamics** — envelope range in dB.

**MODEL-NEEDED — honest limits; leave coarse or add a model:**
- **Arousal** — robustly derivable from prosody (f0 variability + rate + energy dynamics);
  DSP output is honest at **medium** confidence.
- **Valence** — DSP can only lean (f0 contour + spectral tilt); **low** confidence without a
  model. A real value needs a SER model.
- **Dominance** — not DSP-inferable; `0.0` unless a model provides it.
- **Categorical emotion label** (angry/happy/sad/neutral/…) — coarse from prosody; a robust
  label needs a SER ONNX.

#### SER ONNX — cost/benefit (evaluated, IN scope as a wired seam)

A speech-emotion model (e.g. a wav2vec2/HuBERT-SER int8 export, or a small prosody→emotion
MLP) drops into the **exact smart-turn pattern**: `~/.weftos/models/ser/<file>.onnx`, env
override `WEFTOS_SER_MODEL`, construction never fails, **graceful degrade to the DSP
prosody extractor** when absent — identical to `SmartTurnEndpoint` falling back to
`HeuristicEndpoint`. Benefit: real `valence` + categorical `label` + possibly `dominance`.
Cost: a front-end (reuse `WhisperMel`/`fbank` already in-crate) + an ~80–400 MB int8 model to
stage. **Recommendation:** Wave 1 **wires the SER seam and ships the DSP producer**; the
record's `emotion.source` distinguishes `prosody-dsp` from `ser-onnx`, and a present SER model
overrides `valence`+`label` (and `dominance` if it emits it) while **DSP arousal stays the
always-present floor**. Whether to *stage a real SER model* in Wave 1 vs fast-follow is the one
cost/benefit call for the user: the seam + DSP ship regardless, so enabling SER later refines
values with no reshape. Recommend: seam + DSP now; stage a SER ONNX as a fast-follow if a clean
int8 export is available (default it off / degraded, like every other model).

#### Paralinguistics groundwork (coarse now, feeds Wave 3's floor)

When STT text is empty/near-empty but there is voiced energy (laughter, "mm-hmm", sigh,
cough), classify a **non-lexical vocalization**. Signals (all already in the record): short
`voiced_ms` + energy-burst envelope + f0 presence + empty transcript → a coarse rule-based
`class`: `speech | backchannel_candidate | laughter_candidate | filler | unknown`. This is the
signal Wave 3's DuplexChannel `Backchannel` classification (ADR-062 D5 / ADR-068 D1) consumes;
Wave 1 designs the field + a coarse classifier now so the storage/surface never reshape when
the real classifier lands.

#### Record schema (versioned; flat-keyed for verbatim graph serving)

```jsonc
"voice_analysis": {
  "v": 1, "tier": "voice",
  "stt": { "model": "parakeet-tdt-0.6b", "latency_ms": 48, "path": "native",
           "tokens": [ { "text": "hello", "t_ms": 120, "dur_ms": 240, "conf": 0.98 } ],
           "token_conf_mean": 0.94, "token_conf_min": 0.71 },   // tokens: [] on substrate path
  "endpoint": { "completion_prob": 0.86, "source": "smart-turn-v3",
                "silence_tail_ms": 320, "latency_ms": 410 },
  "speaker": { "id": "spk_3", "name": "Mathew", "score": 0.72, "threshold": 0.45,
               "action": "identified", "embedding_dim": 192 },
  "audio": { "duration_ms": 1830, "voiced_ms": 1510, "silence_ms": 320,
             "rms_dbfs_mean": -28.4, "rms_dbfs_peak": -12.1,
             "noise_floor_dbfs": -52.0, "snr_db": 23.6, "clip_pct": 0.0, "dc_offset": 3 },
  "prosody": { "f0_mean_hz": 165, "f0_min_hz": 110, "f0_max_hz": 240,
               "f0_range_semitones": 13.4, "f0_slope": -0.8,
               "rate_tokens_per_s": 4.2, "pause_count": 2, "pause_mean_ms": 180,
               "energy_dynamics_db": 18.0, "confidence": "medium" },
  "emotion": { "valence": 0.1, "arousal": 0.63, "dominance": 0.0, "label": "agitated",
               "arousal_conf": "medium", "valence_conf": "low", "source": "prosody-dsp" },
  "paralinguistics": { "non_lexical": false, "class": "speech", "confidence": "low" }
}
```

#### Storage + wire (RECOMMENDED: sibling key + emotion-axis override)

- **Sibling `voice_analysis` metadata key** holds the full decomp; **and** the record's
  `emotion` overrides the compact `classification` blob's emotion axis with `tier:"voice"`
  (the existing merge point). Rationale: `classification` is the small cross-modality 4-axis
  contract many consumers read (graph hue/glyph, floor `arousal_of`); `voice_analysis` is the
  rich voice-only decomp for the surface + inspection. Separate keys mean text turns carry no
  voice fields and the classification schema never reshapes — voice just *adds* a sibling.
- **Wire:** extend `agent.turn.record` to accept an optional per-turn `voice_analysis` object;
  `index_turn` writes it to the sibling metadata key **and** merges its emotion into the
  classification blob before `dual_write_turn`. `conversation.graph` node build gains one
  verbatim `meta.get("voice_analysis")` read (mirrors the `classification` read at
  `daemon.rs:6010`). One RPC field, one metadata write, one graph read line.

### W1.3 — STT decode enrichment: surface per-token timing/duration/confidence

The native TDT decode discards data it already computes. Change:
- `TdtEngine::greedy_tdt` → return `Vec<Token{ id, t_frame, dur_frames, conf }>` instead of
  `Vec<usize>`: capture `t` and `skip` at each emission and `softmax(&logits[..n_tokens])[y]`.
- `TdtEngine::decode` / `ParakeetStt::run` → a `TranscriptResult{ text, tokens }`; frame→ms via
  the encoder subsampling stride.
- `SttBackend` grows a richer return (add a `transcribe_detailed -> TranscriptResult` with a
  default that wraps `transcribe` as `{text, tokens:[]}`), so the **substrate HTTP path stays
  text-only** (ADR-053 server unchanged) and honestly emits `tokens:[]` / `path:"substrate"`.
  Wave 1 uses the native in-process path, so full token data flows.
- **Partials (medium, deferrable):** true incremental partials need periodic decode over the
  growing buffer feeding `endpointer.observe(..partial_text..)` (today `""`). Design the
  `PartialTranscript` event now; if incremental decode is too costly, ship the level-meter +
  finalized-line fallback (§W1.4) and defer partials without reshaping the surface.

### W1.4 — Enrich `ConversationEvent` + the live surface

**Events** (in `talkmode.rs`, emitted by the loop): `PartialTranscript{text}` (or the
level-meter fallback), `EndpointFired{prob,source,silence_ms}`, and enrich `UserTurn` with the
full `VoiceAnalysis` (speaker score, latencies, prosody, emotion, paralinguistics). Add the
`Instant` instrumentation the STT/endpoint path lacks.

**Surface — `weft voice watch` CLI stream FIRST; the ADR-067 GUI graph for free (RECOMMENDED).**
Terminal-native, pushes enriched observer events as they fire + a ~1 Hz `conversation.graph`
poll for committed+classified state. **The decomposition renders LIVE, stage by stage:**

| Stage | Shown live | Source |
|---|---|---|
| Capturing | level meter (dBFS vs floor) + SNR | `rms_dbfs`, `NoiseFloor` |
| Recognizing | partial transcript (grey) *or* "listening…" | `PartialTranscript` / fallback |
| Endpoint | `⏎ p=0.86 (smart-turn) @ 320ms` | `EndpointFired` |
| Speaker | name + cosine score + action | record `speaker` |
| Finalized | transcript + per-token confidence heat + STT latency | record `stt` |
| Prosody | f0 mean/range · rate · pauses (compact summary) | record `prosody` |
| Emotion | **arousal bar (moving)** + valence + label + `source` badge | record `emotion` |
| Paralinguistics | non-lexical `class` when fired | record `paralinguistics` |
| Labeled/Committed | intent glyph · topic · node state · **tier flip keyword→voice** | `conversation.graph` |

The **informative subset** renders inline; the **full record is inspectable on demand**
(`weft voice watch --json`, or a per-turn expand). Refresh: process events immediately;
committed classification/graph at the ~1 Hz aux cadence (ADR-067 D2). Process must be
*visible* — arousal moves with the acting, the `source` badge shows prosody-dsp vs ser-onnx,
the tier badge flips when the voice emotion lands, endpoint prob and speaker score are shown.

**Free second surface:** the ADR-067 Explorer graph view renders the same conversation
automatically once classification is on — its own track, not a Wave 1 dependency; Wave 1 makes
its data real (and the `voice_analysis` inspectable in node-detail).

### W1.5 — Wave 1 exit test

The user speaks, **including acted emotion**, and sees live: level+SNR, partial/"listening…",
the endpoint fire (prob+source), the finalized transcript with **per-token confidence + speaker
name + cosine score + STT latency**, the **prosody summary and a moving arousal bar with the
emotion label + source badge**, any non-lexical tag, intent/topic labels, the **tier flip
keyword→voice**, and every turn committed+classified on the forest with the full
`voice_analysis` served by `conversation.graph`.

Automated proof: extend `crates/clawft-voice-talk/tests/assembly.rs` `live_native_talk_session`
— WAV (incl. an acted/high-arousal clip) → in-process session → enriched observer →
`agent.turn.record{voice_analysis}` → daemon `index_turn` → assert one committed node carries a
**complete `voice_analysis`** (non-empty `stt.tokens`, `audio.snr_db`, `prosody.f0_mean_hz`,
`emotion.arousal` with `tier:"voice"`), a voice-overridden classification emotion axis,
populated `text`, and a `Speaker` crossref; assert `conversation.graph` returns it verbatim.
Unit tests: f0 estimator on a synthetic tone; rate/pause math; per-token softmax confidence;
paralinguistic classifier on a short energy-burst clip; SER-absent degradation to DSP.

### W1.6 — Wave 1 scope summary

**Built:** STT decode enrichment (per-token timing/dur/confidence; richer `SttBackend` return) ·
DSP prosody (f0 + rate + pauses + energy dynamics) · SER-ONNX seam (degraded-off) + DSP emotion
producer · capture-health (clip/DC) · paralinguistic coarse classifier · the `VoiceAnalysis`
record + `voice_analysis` on `agent.turn.record` + `index_turn` store & emotion-merge + the
`conversation.graph` read · enriched `ConversationEvent` + latency instrumentation ·
`weft voice watch` (+ listen-only mode) · assembly + unit tests.
**Free (enable, don't build):** 4-axis keyword classification + text + crossrefs · graph
node/edge serving · the ADR-067 GUI graph view.
**Not touched (Wave 3):** streaming edge, two-lane wire, DuplexChannel, floor/ERL/barge,
TTS-down. **Deferred (medium):** true incremental partials (fallback ships); staging a real SER
model (seam ships).

---

## Wave 2 — wire into the agent loop, text reply only (outline)

Route finalized voice turns into the agent (`agent.chat`, the M2 `TalkModeLoop` decider)
instead of `turn.record`-only; the agent replies as **text** on the same Wave 1 surface. No TTS.

- **Route:** recorder path calls `agent.chat`; the committed reply returns as `CommittedReply`
  and renders as a text line + its `Follows` edge on the graph surface.
- **Tools/spawn by voice:** M4 spawn + the `tool_choice` lever (WEFT-632, landed) work for free;
  spoken requests that spawn show the (already-classified) `spawn_goal`/`spawn_result` nodes.
- **Surface:** the assistant node's Speculative→Committed lifecycle + tool edges; Wave 1's
  `voice_analysis` now annotates the user side of each exchange.
- Still half-duplex — no floor, no barge, no audio out.

Estimate ~4–6 days. Swarm: `coder` (route + surface) + `tester` + `reviewer`.

---

## Wave 3 — full duplex, and only that (outline == ADR-068 Phase 1)

TTS out + the DuplexChannel floor machine + EdgeReflex + barge, on top of a validated
ingestion + surface. This **is** `duplex-edge-agent-plan.md` §1 Phase 1 (five wiring jobs) plus
its Phase 0 (DuplexChannel + EdgeReflex core + loopback sim; `edge_reflex.rs` already scaffolded
at `crates/clawft-channels/src/voice/edge_reflex.rs`):

1. streaming duplex channel (localhost two-lane wire, `clawft-rpc`);
2. daemon hosts the capture pipeline (VAD/endpoint/STT move daemon-side);
3. daemon hosts TTS-down;
4. `DuplexChannel` ↔ `TalkModeLoop` — `compute_urgency` gains the ERL term (the self-cancel
   fix, ADR-068 D1); **Wave 1's `paralinguistics.class` feeds the `Backchannel` verdict**;
5. desktop `VoiceEdge` collapse (`native.rs` → thin edge + `EdgeReflex`).

The Wave 1 `weft voice watch` surface becomes the floor debugger: `Overlap`/`Backchannel`/barge
and the ERL self-cancel are watched through it, and the `voice_analysis` arousal/paralinguistics
are the floor's own inputs made visible.

Estimate ~2–3 weeks (firmware/transport-scale). Swarm: `system-architect` → `coder`×2
(edge/daemon) → `tester` → `reviewer`.

---

## Plane mapping (WEFT-628 → umbrella + wave items)

**WEFT-628 becomes the umbrella epic** "Voice Phase 1: ingestion → converse → duplex," with
three wave items as children, Wave 1 decomposed. Cycle **0.8.x** (matches the duplex plan §4);
Wave 1 is a candidate to pull to **0.7.x** if the acting-test surface is wanted as a must-ship
demo (operator's call at claim via `plane-workflow`).

| Item | Wave | Cycle | Scope | Blocks on |
|---|---|---|---|---|
| **WEFT-628** (umbrella) | — | 0.8.x | epic: the three waves | — |
| Enable + verify keyword classification on the voice path | 1 | 0.8.x | `mode="keyword"`, ECC loop on; assert graph populated | — (config) |
| STT decode enrichment (per-token timing/dur/confidence) | 1 | 0.8.x | TDT decode return type; richer `SttBackend`; §W1.3 | — |
| DSP prosody + capture-health (f0/rate/pauses/energy/clip/SNR) | 1 | 0.8.x | greenfield f0 + reuse energy/floor; §W1.2 | — |
| SER-ONNX seam + DSP emotion producer (`tier:"voice"`) | 1 | 0.8.x | model-dir seam (degraded-off) + DSP; §W1.2 | prosody |
| Paralinguistic coarse classifier (non-lexical/backchannel groundwork) | 1 | 0.8.x | rule-based `class`; §W1.2 | prosody |
| `VoiceAnalysis` record + `voice_analysis` on `turn.record` + `index_turn` store/merge + graph read | 1 | 0.8.x | schema + wire + daemon; §W1.2 | the extractors |
| Enrich `ConversationEvent` + latency instrumentation | 1 | 0.8.x | `talkmode.rs` events; §W1.4 | — |
| `weft voice watch` live surface + listen-only mode | 1 | 0.8.x | CLI stream + graph poll + `--json`; §W1.4 | enriched events + record |
| Wave 1 exit/assembly + unit tests | 1 | 0.8.x | `assembly.rs` + unit; §W1.5 | all Wave 1 |
| Route voice turns into `agent.chat`; text reply on surface | 2 | 0.8.x | recorder→chat; CommittedReply render | Wave 1 |
| Full-duplex (== ADR-068 Phase 0+1) | 3 | 0.8.x/0.9.x | the duplex-plan items | Wave 2 |

**Reconcile (do not duplicate):** WEFT-606 (daemon tick) + the duplex-plan §4 Phase-0/1
candidates **are Wave 3** — point them there. WEFT-614 (grounded brain) rides Wave 2.

---

## Estimates + swarm shape (Wave 1)

Wave 1 is now substantially larger than a first-pass extractor — it spans the STT decode
return-type change (through parakeet + the `SttBackend` trait), greenfield f0 DSP, the record
plumbing + wire + graph read, the SER seam, paralinguistics, and the live surface. Estimate
**~8–12 days** with a small swarm: `system-architect` (this) → `coder`-A (voice-onnx TDT decode
enrichment + f0/prosody DSP) ∥ `coder`-B (record wire: `agent.turn.record` + `index_turn` store/
merge + graph read + `ConversationEvent` + `weft voice watch`) → `tester` (assembly + unit) →
`reviewer`. Coder-A and Coder-B are largely parallel — they meet at the `VoiceAnalysis` schema,
so **freeze the schema (§W1.2) first**, then fan out.

---

## Riskiest calls (Wave 1)

1. **STT decode enrichment touches a load-bearing return type.** Changing
   `SttBackend::transcribe`'s contract ripples to every caller. Mitigate with an *additive*
   `transcribe_detailed -> TranscriptResult` (default wraps `transcribe`, `tokens:[]`), so the
   substrate path and existing callers are untouched and only the native path grows tokens.
2. **The `voice_analysis` wire + emotion merge is the one new durable surface.** Keep it
   minimal: one optional RPC field, one sibling metadata write, an emotion-axis overwrite at
   `index_turn` (voice > keyword), one verbatim graph read. In-scope for the exit test (arousal
   must land on the forest, not just the terminal).
3. **Honesty of the emotion axis.** DSP arousal is medium-confidence; valence is a guess and
   dominance is `0.0` without SER. Encode the confidence flags in the record and show the
   `source` badge on the surface so the user reads arousal as the honest signal and valence as
   provisional. The complete *structure* ships now; the SER model refines *values* later.
4. **Partials may be costly** (no incremental decode today). Don't block the surface on them —
   the level-meter + finalized-line fallback satisfies "watch the process"; the
   `PartialTranscript` event is designed so partials slot in later without reshaping the surface.

## Recommended first implementation step

**Freeze the `VoiceAnalysis` schema (§W1.2), then enable the classifier and prove the free
daemon path before writing extractors:** run the daemon with `mode="keyword"` + the ECC loop on,
run `weft voice talk` (recorder connected), speak a few turns, and confirm via
`conversation.graph` that nodes carry non-null `classification` and populated `text`. That
validates the entire daemon-side commit+classify+serve foundation the record rides on, and lets
Coder-A/Coder-B fan out against a frozen schema — turning the rest of Wave 1 into additive work
(STT decode enrichment → prosody/SER → the record wire → the surface) against a known-good base.
