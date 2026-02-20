# ClawFT Voice Development Plan

> Extracted from `improvements.md` Workstream G. Voice is a complex, multi-layer feature
> with hardware dependencies, platform-specific audio APIs, model selection trade-offs,
> and real-time latency constraints that require careful design before implementation.

---

## Architecture Overview

```text
Audio Input (mic / file / stream)
  |
  v
Voice Activity Detection (VAD)  -- silence filtering, speech boundary detection
  |
  v
Speech-to-Text (STT)            -- local or cloud, streaming or batch
  |
  v
Agent Pipeline                   -- existing clawft-core message processing
  |
  v
Text-to-Speech (TTS)            -- local or cloud, streaming output
  |
  v
Audio Output (speaker / file / stream)
```

Optional always-on layer:
```text
Wake Word Detection ("Hey Weft")  --> activates VAD+STT pipeline on trigger
```

---

## G1. Speech-to-Text (STT)

**Type:** Feature -- New Plugin (`clawft-plugin` ChannelAdapter or Tool)
**Deps:** C1 (plugin trait crate)
**Feature flag:** `voice`

### Crate Evaluation (as of Feb 2026)

| Crate | Backend | Streaming | Platforms | Notes |
|-------|---------|-----------|-----------|-------|
| [whisper-rs](https://crates.io/crates/whisper-rs) (v0.15.1) | whisper.cpp (GGML) | No (batch) | All | Most mature, widest adoption. GPU via CUDA/Metal. |
| [transcribe-rs](https://lib.rs/crates/transcribe-rs) | Multi-engine (Whisper, Moonshine, Parakeet, SenseVoice) | Partial | macOS/Win/Linux | Feature-flag engine selection. ONNX-based. Hardware accel via Metal/Vulkan. |
| [sherpa-rs](https://crates.io/crates/sherpa-rs) | sherpa-onnx (next-gen Kaldi) | Yes | All + embedded | Streaming STT, speaker diarization, VAD built-in. Actively maintained (v1.12.25 Feb 2026). |
| [rusty-whisper](https://crates.io/crates/rusty-whisper) | tract (pure Rust) | No | All | Pure Rust, no C deps. Older (Dec 2023). |

### Recommendation

**Primary: `sherpa-rs`** (sherpa-onnx bindings)
- Streaming support is critical for Talk Mode (G5)
- Built-in VAD means we can potentially skip a separate VAD crate
- Supports 12+ languages, embedded targets, RISC-V
- Active development (Feb 2026 releases)
- ONNX-based = consistent cross-platform behavior

**Fallback: `whisper-rs`**
- Better accuracy on long-form transcription
- Larger community, more battle-tested
- No streaming (batch only) -- acceptable for file/message input

**Cloud fallback:** OpenAI Whisper API, Google Cloud STT, or Deepgram for hosted high-accuracy option when local models are insufficient (noisy audio, rare languages).

### Design Decisions Needed

1. **Audio format:** What sample rate and encoding do we standardize on internally? (16kHz 16-bit PCM is the common denominator)
2. **Model management:** How are STT models downloaded, cached, and updated? (~50MB-1.5GB per model)
3. **GPU detection:** Auto-detect CUDA/Metal availability and select appropriate backend?
4. **Streaming granularity:** How often do we emit partial transcriptions? (every N ms? on VAD silence?)

---

## G2. Text-to-Speech (TTS)

**Type:** Feature -- New Plugin
**Deps:** C1
**Feature flag:** `voice`

### Crate Evaluation (as of Feb 2026)

| Crate | Backend | Streaming | Quality | Notes |
|-------|---------|-----------|---------|-------|
| [piper-rs](https://github.com/thewh1teagle/piper-rs) | Piper ONNX | Yes | Good | HuggingFace voice models. ~217 downloads/month. |
| [piper-rs (mush42/sonata)](https://github.com/mush42/piper-rs) | Sonata engine | Yes | Good | Post-processing, parallelism modes. |
| [piper-tts-rs-sys](https://lib.rs/crates/piper-tts-rs-sys) | Piper C bindings | Yes | Good | Updated Jan 2026. Raw bindings. |
| [sherpa-rs](https://crates.io/crates/sherpa-rs) | sherpa-onnx TTS | Yes | Good | Same crate as STT -- single dependency for both. |

### Recommendation

**Primary: `sherpa-rs`** (sherpa-onnx TTS)
- Same dependency as STT -- reduces dependency surface significantly
- Streaming output for natural conversation feel
- ONNX-based, cross-platform

**Alternative: `piper-rs` (thewh1teagle)**
- Larger voice model selection via HuggingFace/rhasspy
- If we need more voice variety or specific language coverage

**Cloud fallback:** ElevenLabs (highest quality), OpenAI TTS, Google Cloud TTS.

### Design Decisions Needed

1. **Voice selection:** How does the user pick a voice? Config file? Per-agent? Runtime switchable?
2. **Output device:** Direct speaker output vs audio file generation vs streaming over WebSocket?
3. **Interruption handling:** When user starts speaking mid-TTS, how fast can we stop output? (needs audio output cancellation)
4. **Latency budget:** Target time from text-ready to first audio byte? (<200ms for natural feel)

---

## G3. VoiceChannel

**Type:** Feature -- `ChannelAdapter` plugin
**Deps:** G1, G2, C1

The VoiceChannel ties STT and TTS together as a full `ChannelAdapter` in the clawft plugin system, meaning voice becomes just another channel like Telegram or Discord.

### Flow

```text
VoiceChannel.start()
  |
  +-- Spawn audio capture task (microphone or audio stream input)
  +-- Spawn VAD filtering task
  +-- On speech detected:
  |     STT transcribe -> InboundMessage { channel: "voice", content: transcription }
  |     -> MessageBus -> AgentLoop -> OutboundMessage
  +-- On outbound received:
        TTS synthesize -> audio output (speaker / stream)
```

### Design Decisions Needed

1. **Audio capture abstraction:** `cpal` (cross-platform audio I/O) vs platform-specific APIs?
2. **Channel identity:** What is `sender_id` / `chat_id` for voice? Device ID? User voice fingerprint?
3. **Multi-turn:** How do we handle silence between turns? Configurable silence timeout?
4. **Concurrent speakers:** Single-user only, or speaker diarization for multi-party?

---

## G4. Voice Activity Detection (VAD)

**Type:** Feature
**Deps:** G1

### Crate Evaluation (as of Feb 2026)

| Crate | Engine | Accuracy (TPR @ 5% FPR) | Notes |
|-------|--------|------------------------|-------|
| [voice_activity_detector](https://crates.io/crates/voice_activity_detector) | Silero VAD V5 | ~88% | Self-contained, simple API. 8/16kHz. |
| [silero-vad-rust](https://crates.io/crates/silero-vad-rust) | Silero ONNX | ~88% | Streaming-ready. Bundles ONNX weights. |
| [voice-stream](https://crates.io/crates/voice-stream) | WebRTC + Silero | ~88% (Silero) / ~50% (WebRTC) | High-level audio capture + VAD pipeline. |
| sherpa-rs (built-in) | Silero via sherpa-onnx | ~88% | No extra dep if using sherpa-rs for STT. |

### Recommendation

**If using sherpa-rs for STT:** Use its built-in VAD -- zero additional dependencies.

**If using whisper-rs for STT:** Add `voice_activity_detector` or `silero-vad-rust` for standalone VAD.

**Skip WebRTC VAD** -- Silero has 4x fewer errors at the same false-positive rate.

### Key Consideration

VAD determines the "speech boundary" -- when the user has finished speaking and the agent should start processing. Getting this wrong means either:
- Cutting off mid-sentence (timeout too short)
- Awkward pauses before response (timeout too long)

This is the single most important UX parameter in voice interaction. It needs to be tunable per-user and possibly adaptive (learn the user's speech patterns over time).

---

## G5. Talk Mode & Voice Wake

**Type:** Feature
**Deps:** G3, G4

### Talk Mode

Continuous conversation loop:
```text
listen >> [VAD detects end-of-speech] >> transcribe >> think >> speak >> listen >> ...
```

**Interruption detection:** If the user starts speaking while the agent is outputting TTS, the agent must:
1. Immediately stop TTS playback
2. Capture the new input
3. Process it (possibly as a correction or new request)

This requires:
- Cancellable TTS output (non-blocking audio playback with abort handle)
- Echo cancellation (don't feed the agent's own TTS output back into STT)
- Fast VAD response (<50ms to detect user started speaking)

### Voice Wake

Always-on wake word detection.

| Crate | Approach | Notes |
|-------|----------|-------|
| [rustpotter](https://github.com/GiviMAD/rustpotter) (v1.0+) | MFCC + DTW / neural | Open source, privacy-first (all local). Train custom wake words from 3-8 WAV samples. Semver-stable since 1.0. CLI for recording/training. |

**Recommendation: `rustpotter`**
- Pure Rust, no cloud dependency
- Custom wake words (not just "Alexa" / "Hey Google")
- Default wake word: "Hey Weft" (train from samples)
- Low CPU footprint suitable for background daemon

### Platform Integration

| Platform | Audio API | Background Mode | Notes |
|----------|-----------|-----------------|-------|
| Linux | PulseAudio / PipeWire via `cpal` | systemd user service | Most straightforward |
| macOS | CoreAudio via `cpal` | launchd agent | Needs microphone permission |
| Windows | WASAPI via `cpal` | Startup task or tray app | Needs microphone permission |
| iOS/Android | Platform-specific | Limited background audio | Significant constraints on always-on listening |

### Design Decisions Needed

1. **Echo cancellation:** Use `cpal`'s loopback capture to subtract TTS output from mic input? Or hardware AEC?
2. **Wake word training UX:** CLI-only (`weft voice train-wake "hey weft"`)? Or guided setup wizard?
3. **Battery/CPU impact:** What's acceptable CPU usage for always-on wake word detection? (<2% target)
4. **Privacy indicator:** Show a visual indicator when mic is active? (expected on modern OSes)
5. **Fallback chain order:** Local STT first (latency) or cloud first (accuracy)?

---

## Dependency Summary

If we go with `sherpa-rs` as the primary engine (STT + TTS + VAD in one):

| Purpose | Primary Crate | Fallback |
|---------|--------------|----------|
| STT | `sherpa-rs` | `whisper-rs` |
| TTS | `sherpa-rs` | `piper-rs` |
| VAD | `sherpa-rs` (built-in) | `voice_activity_detector` |
| Wake Word | `rustpotter` | -- |
| Audio I/O | `cpal` | Platform-specific |

This means **3 new dependencies** for the full voice stack (sherpa-rs, rustpotter, cpal), rather than the 5-6 originally planned.

---

## Pre-Implementation Requirements

1. **Audio pipeline prototype** -- Build a standalone Rust binary that does mic capture -> VAD -> STT -> print text, to validate latency and accuracy before integrating into clawft
2. **Model hosting / download strategy** -- Where do we host models? How does first-run download work? Size budget per model?
3. **Latency benchmarking** -- Measure end-to-end latency: speech-end to first response character, on target hardware (laptop, Raspberry Pi, server)
4. **Platform audio testing** -- Verify `cpal` works on Linux (PipeWire + PulseAudio), macOS (CoreAudio), Windows (WASAPI) for both capture and playback
5. **Echo cancellation feasibility** -- Test whether software AEC is sufficient or if we need to require hardware echo cancellation
6. **Feature flag design** -- `voice` feature enables the full stack; sub-features for `voice-stt`, `voice-tts`, `voice-wake` for partial builds

---

## Open Questions

- Should voice be a channel (`VoiceChannel`) or a tool (`voice_listen`, `voice_speak`) or both?
- How does voice interact with the existing permission system? Level 0 users shouldn't be able to voice-activate shell commands.
- Should the agent's "personality" affect voice selection? (different voice per agent in multi-agent setups)
- Integration with Discord/Slack voice channels? (receive voice in a Discord VC -> STT -> respond via TTS in the same VC)

---

## Sources

- [whisper-rs](https://crates.io/crates/whisper-rs) -- Rust bindings to whisper.cpp
- [transcribe-rs](https://lib.rs/crates/transcribe-rs) -- Multi-engine STT (Whisper, Moonshine, Parakeet, SenseVoice)
- [sherpa-rs](https://crates.io/crates/sherpa-rs) -- Rust bindings to sherpa-onnx (STT + TTS + VAD)
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) -- Upstream multi-platform speech toolkit
- [piper-rs](https://github.com/thewh1teagle/piper-rs) -- Piper TTS models in Rust
- [voice_activity_detector](https://crates.io/crates/voice_activity_detector) -- Silero VAD V5 for Rust
- [silero-vad-rust](https://crates.io/crates/silero-vad-rust) -- Silero VAD with ONNX/ort
- [voice-stream](https://crates.io/crates/voice-stream) -- High-level audio capture + VAD
- [rustpotter](https://github.com/GiviMAD/rustpotter) -- Open source wake word spotter
- [Cobra vs Silero vs WebRTC VAD comparison](https://picovoice.ai/blog/best-voice-activity-detection-vad-2025/) -- Accuracy benchmarks
