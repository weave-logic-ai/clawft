# ADR-061: Conversational voice agent loop — full-duplex, dual-layer TTS, grounded

**Date**: 2026-06-28
**Status**: Accepted (2026-06-28)
**Deciders**: Main-thread design + validation (voice-pipeline prototyping thread, 2026-06-26..28; built and measured end-to-end in the `~/llm` voicelab harness)
**Depends-On**: ADR-053 (voice STT canonical path — `SttBackend`), ADR-018 + ADR-060 (Hermes/local LLM provider + serving — the *brain* this speaks for), ADR-058 (per-conversation context memory tier)
**Relates-To**: ADR-056 (ECC indexes — speaker/utterance nodes), `~/weftos/.planning/voice-ecc-synthesis.md` (the full mapping), `crates/clawft-voice-aec` (the native AEC bridge introduced here), `crates/clawft-channels` (`voice` / `voice-real-audio` features — the Talk-Mode home), the `~/llm` voicelab harness (validated reference implementation)

## Context

ADR-053 settled STT (substrate-side whisper, swappable `SttBackend`) and ADR-018/060
settled the local LLM brain, but **nothing records how those compose into a real-time
*conversation*** — the loop a person actually talks to. We prototyped that loop
end-to-end in the `~/llm` voicelab harness (a modular, swappable, measured bench;
each stage emits ECC-style events) and validated every component on the target
Apple-silicon host. This ADR records the resulting architecture and the canonical
engine/component decisions so weftos can port it (the harness stays the lab; the
product lands in `clawft-channels` Talk-Mode + `clawft-voice-aec`).

Measured facts that drove the design (mid-2026, this host):
- **STT is cheap warm**: parakeet-tdt-0.6b ~0.05s / 0% WER (English), whisper-large-v3-turbo
  for multilingual. The "slow first turn" was cold-load → **pre-warm at startup**.
- **No single local TTS is simultaneously fast + voice-cloning + literal-tag-performing**
  (the *voice trilemma*). Measured: Chatterbox (clone, ~0.7s TTFA, emotion *intensity* only);
  Supertonic (99M ONNX, ~0.34s, weak clone 0.30); Orpheus (performs `<laugh>` literally, preset
  voice, RTF 0.84 → streams gap-free); IndexTTS-2 (best clone 0.83 + emotion, RTF ~1.9 → must
  prebuffer). Grounded LLM answers add a web-search round (seconds).
- **Apple VPIO is a dead end on this hardware**: AVAudioEngine reports the built-in mic as
  9-channel and VPIO `-10875`s. Echo cancellation must own device I/O itself.

## Decision

**The conversational voice loop is: STT → grounded agent → dual-layer TTS, over a
full-duplex echo-cancelled audio channel, with per-speaker identity.** Canonical pieces:

### 1. Audio channel — native AEC, full-duplex (`clawft-voice-aec`)
A weftos-native Rust crate (`cpal` device I/O + WebRTC **AEC3** via `webrtc-audio-processing`,
feature-gated) owns capture+playback in one tightly-aligned loop. It **downmixes any mic to
mono** (sidesteps the AVAudioEngine 9-ch/VPIO failure), resamples to 16 kHz, and runs AEC3
with the playback as the render reference (measured **−41 dB** echo). Exposes a stdio bridge
(`aec-bridge`: stdin = play/reference PCM, stdout = cleaned mic) + a `flush()` to silence
playback instantly. **Barge-in** = VAD on the cleaned mic during playback → `flush()` → capture
the user's onset. This supersedes the Apple-VPIO approach for AEC. (Pinned to built-in hardware,
never Bluetooth — VPIO/SCO unsupported and a drifting default kills barge-in.)

### 2. STT (per ADR-053)
A `SttBackend` (parakeet for English-fast, whisper-turbo for multilingual) — **pre-warmed** so
the first turn isn't a cold load. Streaming-partial STT is a future fast-reaction enhancement.

### 3. The agent (per ADR-018/060) — voice is a front end, not a fork
The voice front speaks for the **same agent loop** ADR-058/060 serve and is agnostic to *how* the
agent produces its reply. **Tool-calling / web grounding is OUT OF SCOPE for this ADR** — it
appeared in the voicelab harness only for testing; the agent's tools are the agent loop's concern
(ADR-060). The one **voice-specific** constraint: answers are **short** (hard spoken-length token
cap — a voice agent, not a document writer). The agent may surface a long-running operation as a
tool-intent impulse distinct from its committed reply so the fast TTS layer can acknowledge while
the slow answer renders (the speculative→committed pattern below).

### 4. Dual-layer TTS (the core latency decision)
Two engines — a fast ack covering a streamed expressive answer:
- **FAST layer — Chatterbox (cloned voice):** the immediate, *contextual* acknowledgment
  ("Puyo Puyo — one sec.") and snappy chit-chat answers. ~0.7s.
- **SLOW layer — Orpheus (default):** expressive, **streamed chunk-by-chunk** (producer renders
  sentence-by-sentence, consumer plays each chunk as ready with a small lead buffer → gap-free for
  RTF<1). Orpheus performs paralinguistics literally and streams gap-free; first audio ~1–2s
  instead of a single ~30s blocking render.
  - **Voice-identity trade (accepted default):** Orpheus uses a **preset voice**, so by default the
    slow grounded answer is NOT in the fast layer's cloned voice — a deliberate choice of literal
    `<laugh>` + gap-free streaming over single-voice identity. **`IndexTTS-2` is the same-cloned-
    voice swap** (emotion + the fast clone's identity, RTF~1.9 → prebuffered) when one consistent
    voice matters more than literal paralinguistics. Swappable per deployment.
The fast ack **covers** the slow render's latency — the loop never goes silent. The slow stream
is interruptible (barge-in flushes producer+consumer). A blocking "render-all-then-play-once"
slow call is explicitly rejected (it produced ~31s of silence / no audio).

### 5. Turn-taking — semantic endpointing
smart-turn (semantic end-of-utterance) gates capture: on a short pause, keep listening while the
turn reads "open", finalize on complete or a hard max-silence. Replaces fixed-silence cutoff so
the user isn't clipped mid-thought.

### 6. Speaker identity → ECC per-speaker node
An ECAPA d-vector per utterance, matched against a **persistent named registry** (cosine ≥ ~0.45),
fed to the LLM as private context (never spoken). Each identity **is** an ECC per-speaker
CrossRef/node (ADR-056): enrollment = node creation, `identify()` = CrossRef resolve, the
running-mean centroid = the node's evolving embedding. This is the **multiplayer foundation** — N
humans (and agents), each a stable node with per-speaker context/access, turns attributed by who spoke.

### 7. Expressive tags
The LLM emits ONE canonical vocab (`<laugh> <sigh> <gasp> … <emotion>…</emotion> <strong>`); a
tag layer maps it per engine (Orpheus performs paralinguistics literally; IndexTTS-2 performs
`<emotion>` as affect; fast/speed engines strip cleanly, never reading a tag aloud) with a hard
scrub guarantee (no `<`/`>`/`[` fragment ever reaches audio).

### ECC mapping (voice loop → kernel)
Utterance → `CausalNode(Follows)`; EOU → `Impulse::EOU`; backchannel/interruption → floor impulses;
the **fast ack = a Speculative spoken node**, the **slow grounded answer = the Committed node** that
supersedes/extends it (the generic speculative→committed pattern, reusable for any high-latency tool).

## Consequences

- **Enables** a real conversation: fast, cloned-voice (fast layer), expressive (literal-
  paralinguistic slow layer), interruptible, speaker-aware, natural-turn-taking — on the hardware
  mic. (Grounding/tools are the agent loop's concern, ADR-060 — not this ADR.)
- **`clawft-voice-aec`** is a new product crate (feature-gated; default workspace build unaffected).
  It is the device/AEC substrate `clawft-channels::voice` (ADR-053's thin client) consumes for
  full-duplex; the substrate whisper path (ADR-053) remains the canonical STT.
- **Voice trilemma is explicit**: pick two of {fast, voice-cloned, literal-laughs}. **Accepted
  default = fast cloned (Chatterbox) ack + Orpheus (preset, literal-laughs, gap-free) slow layer**
  — trading single-voice identity for literal paralinguistics. `IndexTTS-2` is the same-voice
  expressive swap (prebuffered). Swappable per deployment.
- **Latency budget**: warm STT ~0.05s + LLM TTFT ~0.2–0.3s + fast TTS ~0.3–0.7s ≈ ~0.6s felt for
  fast answers; grounded/expressive answers ride behind the contextual ack.
- **Deferred**: a fast engine that clones *and* performs literal paralinguistics (frontier); VAP
  backchannels ("mm-hmm"); streaming-partial STT for sub-turn reaction; buffered-ahead speculative
  response generation (compose while the user is still speaking).

## Alternatives considered

- **Apple VPIO for AEC** — rejected: AVAudioEngine 9-ch built-in mic → `-10875` on this hardware;
  not device-robust. `clawft-voice-aec` (cpal + AEC3) owns the device instead.
- **Single-layer TTS** — rejected: no one engine is fast + cloned + expressive; the dual layer with
  a fast ack covering a streamed expressive answer is the way to get all three *perceptually*.
- **IndexTTS-2 as the live engine** — rejected for live (RTF ~1.9, ~31s blocking, produced no audio
  render-all-then-play); kept as the prebuffered same-voice expressive option.
- **Python-only audio (sounddevice + livekit APM)** — viable but higher jitter degrades AEC; the
  native single-callback bridge gives tight render↔capture alignment. (Harness keeps both.)
- **Generic fillers** — replaced by *contextual* acks (echo the subject) so latency reads as
  "thinking", not lag.
