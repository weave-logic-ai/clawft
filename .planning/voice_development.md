# ClawFT Voice Development Sprint Plan

> Comprehensive sprint plan for voice features (STT, TTS, VAD, VoiceChannel, Talk Mode, Voice Wake).
> Runs **in parallel** with the UI sprint. Shares WebSocket transport from the UI backend.
> Primary engine: **sherpa-rs** (sherpa-onnx). Wake word: **rustpotter**.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Technology Stack](#technology-stack)
3. [Pre-Implementation Requirements](#pre-implementation-requirements)
4. [Sprint Plan](#sprint-plan)
5. [Dependency Matrix](#dependency-matrix)
6. [Integration with UI Sprint](#integration-with-ui-sprint)
7. [Platform Testing Matrix](#platform-testing-matrix)

---

## Architecture Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│                        Voice Pipeline                                │
│                                                                      │
│  ┌──────────┐   ┌─────┐   ┌─────┐   ┌──────────┐   ┌─────┐        │
│  │ Audio In │──>│ VAD │──>│ STT │──>│Agent Loop│──>│ TTS │──┐     │
│  │  (cpal)  │   │     │   │     │   │(existing)│   │     │  │     │
│  └──────────┘   └─────┘   └─────┘   └──────────┘   └─────┘  │     │
│       ^                                                       │     │
│       │              ┌────────────┐                           v     │
│       │              │ Wake Word  │                    ┌──────────┐ │
│       │              │(rustpotter)│                    │ Audio Out│ │
│       │              └──────┬─────┘                    │  (cpal)  │ │
│       │                     │ triggers                 └──────────┘ │
│       └─────────────────────┘                                       │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  VoiceChannel (ChannelAdapter)                                │  │
│  │  - Bridges audio pipeline <-> MessageBus                      │  │
│  │  - InboundMessage: transcribed text from STT                  │  │
│  │  - OutboundMessage: text -> TTS -> audio output               │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  Talk Mode Controller                                         │  │
│  │  - Continuous listen -> transcribe -> think -> speak loop      │  │
│  │  - Interruption detection (user speaks during TTS)            │  │
│  │  - Echo cancellation (software AEC)                           │  │
│  │  - Configurable silence timeout (speech boundary detection)   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  Voice Wake Daemon                                            │  │
│  │  - Always-on wake word detection ("Hey Weft")                 │  │
│  │  - Low CPU (~1-2%) background process                         │  │
│  │  - Activates VAD+STT pipeline on trigger                      │  │
│  │  - Platform integration: systemd / launchd / startup task     │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### Integration Points with UI

```text
Voice Pipeline ──WebSocket──> UI Dashboard
                               │
                               ├── Voice status indicator (idle/listening/processing/speaking)
                               ├── Talk Mode overlay (conversation transcript + controls)
                               ├── Audio visualizer (waveform/spectrum)
                               └── Wake word toggle + mic permissions
```

---

## Technology Stack

### Confirmed Dependencies

| Purpose | Crate | Version | Size Impact | Notes |
|---------|-------|---------|-------------|-------|
| **STT** | `sherpa-rs` | 1.12+ | ~15 MB (models) | Streaming STT via sherpa-onnx, 12+ languages |
| **TTS** | `sherpa-rs` | 1.12+ | (shared) | Same crate as STT, streaming output |
| **VAD** | `sherpa-rs` (built-in) | -- | (shared) | Silero VAD V5, no extra dependency |
| **Wake Word** | `rustpotter` | 1.0+ | ~2 MB | Custom wake words, MFCC+DTW/neural |
| **Audio I/O** | `cpal` | 0.15+ | ~200 KB | Cross-platform audio capture/playback |

**Total new dependencies: 3 crates** (sherpa-rs, rustpotter, cpal)

### Model Management

| Model | Type | Size | Download |
|-------|------|------|----------|
| sherpa-onnx STT (int8) | Streaming Zipformer | ~50 MB | First-run auto-download |
| sherpa-onnx TTS (VITS) | Streaming | ~30 MB | First-run auto-download |
| Silero VAD V5 | Bundled in sherpa-onnx | ~2 MB | Included |
| Wake word ("Hey Weft") | Custom trained | ~500 KB | Bundled in binary |

**Model storage:** `~/.clawft/models/voice/`

### Audio Format Standards

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Sample rate | 16 kHz | Common denominator for STT models |
| Bit depth | 16-bit PCM | Standard for speech processing |
| Channels | Mono | Single speaker expected |
| Chunk size | 480 samples (30ms) | Good balance of latency vs efficiency |
| Buffer depth | 10 chunks (300ms) | Covers VAD window |

---

## Pre-Implementation Requirements

### VP1. Audio Pipeline Prototype (Week 0)

Standalone Rust binary (not integrated into clawft) that validates:

```text
mic capture (cpal) -> VAD (sherpa-rs) -> STT (sherpa-rs) -> print text
```

**Success criteria:**
- Works on Linux (PipeWire + PulseAudio), macOS (CoreAudio), Windows (WASAPI)
- End-to-end latency: speech-end to text-available < 500ms
- VAD correctly detects speech boundaries (< 3% false triggers on silence)
- STT accuracy > 90% on common English phrases

**Deliverable:** `voice-proto/` directory with standalone binary + benchmark results.

### VP2. Model Hosting & Download

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Host location | GitHub Releases + HuggingFace mirror | Free, reliable, cacheable |
| Download trigger | First `weft voice` command or `weft voice setup` | No surprise downloads |
| Cache location | `~/.clawft/models/voice/` | Standard user data dir |
| Integrity check | SHA-256 manifest | Detect corrupted downloads |
| Offline mode | Pre-downloaded models work without internet | Local-first principle |

### VP3. Feature Flag Design

```toml
# clawft-plugin/Cargo.toml
[features]
voice = ["voice-stt", "voice-tts", "voice-wake"]
voice-stt = ["dep:sherpa-rs"]
voice-tts = ["dep:sherpa-rs"]
voice-wake = ["dep:rustpotter", "dep:cpal"]

# Sub-features allow partial builds:
# - voice-stt only: transcription tool without TTS or wake word
# - voice-tts only: TTS tool without live mic capture
# - voice-wake: always-on listening (requires cpal)
# - voice (full): complete pipeline
```

### VP4. Platform Audio Testing

Before sprint begins, validate `cpal` on all targets:

| Platform | Audio Backend | Capture | Playback | Status |
|----------|--------------|---------|----------|--------|
| Linux (PipeWire) | PipeWire via `cpal` | ? | ? | Needs testing |
| Linux (PulseAudio) | PulseAudio via `cpal` | ? | ? | Needs testing |
| macOS (CoreAudio) | CoreAudio via `cpal` | ? | ? | Needs testing |
| Windows (WASAPI) | WASAPI via `cpal` | ? | ? | Needs testing |
| WSL2 | PulseAudio bridge | ? | ? | Needs testing |

### VP5. Echo Cancellation Feasibility

Software AEC (Acoustic Echo Cancellation) options:
1. **Loopback subtraction:** Capture TTS output via `cpal` loopback, subtract from mic input
2. **WebRTC AEC:** Use `webrtc-audio-processing` crate
3. **Hardware AEC:** Rely on OS-level echo cancellation (macOS does this well)

**Decision needed after VP1 prototype testing.**

---

## Sprint Plan

### Voice Sprint 1: Core Pipeline (Weeks 1-3)

**Goal:** STT tool, TTS tool, VoiceChannel adapter, basic Talk Mode.
**Runs parallel with UI Sprint 1.**

#### VS1.1 Audio Foundation (Week 1)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS1.1.1 | Add `sherpa-rs`, `cpal` to `clawft-plugin/Cargo.toml` behind `voice` feature | 2h | clawft-plugin |
| VS1.1.2 | Create `clawft-plugin/src/voice/mod.rs` -- voice module structure | 2h | clawft-plugin |
| VS1.1.3 | Implement `AudioCapture` -- `cpal` microphone input stream | 6h | clawft-plugin |
| VS1.1.4 | Implement `AudioPlayback` -- `cpal` speaker output stream | 4h | clawft-plugin |
| VS1.1.5 | Implement `VoiceActivityDetector` -- sherpa-rs Silero VAD wrapper | 4h | clawft-plugin |
| VS1.1.6 | Model download manager -- fetch + cache + integrity check | 6h | clawft-plugin |
| VS1.1.7 | Voice config types: `VoiceConfig` in `clawft-types` | 3h | clawft-types |
| VS1.1.8 | Feature flag wiring: `voice`, `voice-stt`, `voice-tts`, `voice-wake` | 2h | Multiple |

**Week 1 Deliverable:** Audio capture/playback working, VAD detecting speech boundaries, model download functional.

#### VS1.2 STT + TTS (Week 2)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS1.2.1 | Implement `SpeechToText` -- sherpa-rs streaming recognizer | 8h | clawft-plugin |
| VS1.2.2 | Implement `TextToSpeech` -- sherpa-rs streaming synthesizer | 6h | clawft-plugin |
| VS1.2.3 | STT partial result callback -- emit intermediate transcriptions | 4h | clawft-plugin |
| VS1.2.4 | TTS streaming playback -- start audio before full synthesis completes | 4h | clawft-plugin |
| VS1.2.5 | TTS cancellation -- abort handle for interruption support | 3h | clawft-plugin |
| VS1.2.6 | `voice_listen` tool -- on-demand transcription (non-streaming) | 4h | clawft-tools |
| VS1.2.7 | `voice_speak` tool -- on-demand TTS (agent can speak text) | 4h | clawft-tools |
| VS1.2.8 | CLI commands: `weft voice setup`, `weft voice test-mic`, `weft voice test-speak` | 4h | clawft-cli |

**Week 2 Deliverable:** STT transcribes mic input, TTS speaks text, both available as agent tools.

#### VS1.3 VoiceChannel + Talk Mode (Week 3)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS1.3.1 | Implement `VoiceChannel` as `ChannelAdapter` plugin | 8h | clawft-plugin |
| VS1.3.2 | VoiceChannel -> MessageBus integration (transcriptions as InboundMessage) | 4h | clawft-plugin |
| VS1.3.3 | MessageBus -> VoiceChannel TTS (outbound text spoken aloud) | 4h | clawft-plugin |
| VS1.3.4 | Basic Talk Mode controller (listen -> transcribe -> think -> speak -> listen) | 8h | clawft-plugin |
| VS1.3.5 | Silence timeout configuration (configurable speech boundary detection) | 3h | clawft-types |
| VS1.3.6 | Interruption detection: stop TTS when user starts speaking | 4h | clawft-plugin |
| VS1.3.7 | CLI: `weft voice talk` -- start Talk Mode session | 3h | clawft-cli |
| VS1.3.8 | WebSocket voice events: `voice:status` (idle/listening/processing/speaking) | 3h | clawft-services |
| VS1.3.9 | Unit + integration tests for pipeline | 4h | tests/ |

**Sprint VS1 Deliverable:** Full voice pipeline: mic -> VAD -> STT -> agent -> TTS -> speaker. Talk Mode works via CLI. Voice status events on WebSocket.

---

### Voice Sprint 2: Wake Word + Platform Integration (Weeks 4-6)

**Goal:** Voice Wake daemon, platform integration, echo cancellation, Discord voice.
**Runs parallel with UI Sprint 2.**

#### VS2.1 Voice Wake (Week 4)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS2.1.1 | Add `rustpotter` to `clawft-plugin/Cargo.toml` behind `voice-wake` | 2h | clawft-plugin |
| VS2.1.2 | Train "Hey Weft" wake word model (record 5-8 samples, train via rustpotter CLI) | 4h | models/ |
| VS2.1.3 | Implement `WakeWordDetector` -- rustpotter integration | 6h | clawft-plugin |
| VS2.1.4 | Wake word -> VAD pipeline activation (trigger listening on wake word) | 4h | clawft-plugin |
| VS2.1.5 | CLI: `weft voice wake` -- start always-on wake word listener | 3h | clawft-cli |
| VS2.1.6 | CLI: `weft voice train-wake "hey weft"` -- guided wake word training | 4h | clawft-cli |
| VS2.1.7 | Custom wake word support (user-trained models stored in `~/.clawft/models/wake/`) | 3h | clawft-plugin |
| VS2.1.8 | CPU budget monitoring -- ensure wake word uses < 2% CPU | 2h | clawft-plugin |

**Week 4 Deliverable:** "Hey Weft" activates voice pipeline. Custom wake words trainable.

#### VS2.2 Echo Cancellation + Quality (Week 5)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS2.2.1 | Software AEC implementation (loopback subtraction or WebRTC AEC crate) | 8h | clawft-plugin |
| VS2.2.2 | Noise suppression pre-filter (sherpa-rs built-in or separate) | 4h | clawft-plugin |
| VS2.2.3 | Adaptive silence timeout (learn user speech patterns over time) | 6h | clawft-plugin |
| VS2.2.4 | Multi-language STT support (language auto-detection or config) | 4h | clawft-plugin |
| VS2.2.5 | Voice selection (multiple TTS voices, per-agent voice config) | 4h | clawft-types |
| VS2.2.6 | Audio quality metrics (SNR, latency percentiles, WER estimation) | 3h | clawft-plugin |

**Week 5 Deliverable:** Echo cancellation working, noise suppression active, multi-language support.

#### VS2.3 Platform Integration (Week 6)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS2.3.1 | Linux systemd user service for Voice Wake daemon | 3h | scripts/ |
| VS2.3.2 | macOS launchd agent for Voice Wake daemon | 3h | scripts/ |
| VS2.3.3 | Windows startup task for Voice Wake daemon | 3h | scripts/ |
| VS2.3.4 | Microphone permission request handling (macOS/Windows) | 4h | clawft-plugin |
| VS2.3.5 | Privacy indicator: visual notification when mic is active | 3h | clawft-plugin |
| VS2.3.6 | PipeWire audio integration (Linux native) | 3h | clawft-plugin |
| VS2.3.7 | Discord voice channel bridge: receive voice in VC -> STT -> respond via TTS | 6h | clawft-channels |
| VS2.3.8 | Platform audio test suite (automated tests on CI) | 4h | tests/ |

**Sprint VS2 Deliverable:** Voice Wake daemon with platform service files, echo cancellation, Discord voice bridge, multi-language support.

---

### Voice Sprint 3: Advanced Features + UI Integration (Weeks 7-9)

**Goal:** UI voice controls, cloud fallback, speaker diarization, voice agent personality.
**Runs parallel with UI Sprint 3.**

#### VS3.1 UI Voice Integration (Week 7)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS3.1.1 | Voice status bar component (UI): idle/listening/processing/speaking indicator | 4h | ui/ |
| VS3.1.2 | Talk Mode overlay (UI): floating transcript + stop/mute buttons | 6h | ui/ |
| VS3.1.3 | Audio waveform visualizer (UI): real-time mic input display | 4h | ui/ |
| VS3.1.4 | Voice settings panel (UI): mic select, voice select, language, wake word toggle | 4h | ui/ |
| VS3.1.5 | Push-to-talk button (UI): hold to speak, release to process | 3h | ui/ |
| VS3.1.6 | WebSocket voice events: partial transcription streaming to UI | 3h | clawft-services |
| VS3.1.7 | WebSocket voice events: TTS progress (word highlighting) | 3h | clawft-services |
| VS3.1.8 | Tauri voice integration: native mic access from desktop shell | 4h | ui/ |

**Week 7 Deliverable:** Dashboard shows voice status, Talk Mode has visual overlay, settings panel for voice configuration.

#### VS3.2 Cloud Fallback + Quality (Week 8)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS3.2.1 | Cloud STT fallback: OpenAI Whisper API integration | 4h | clawft-plugin |
| VS3.2.2 | Cloud TTS fallback: ElevenLabs / OpenAI TTS API integration | 4h | clawft-plugin |
| VS3.2.3 | Fallback chain: local first -> cloud on failure/low confidence | 4h | clawft-plugin |
| VS3.2.4 | Speaker diarization: multi-speaker identification (sherpa-rs) | 6h | clawft-plugin |
| VS3.2.5 | Conversation mode: distinguish speakers in multi-party voice | 4h | clawft-plugin |
| VS3.2.6 | Voice transcription logging (persist voice conversations to session) | 3h | clawft-core |

**Week 8 Deliverable:** Cloud fallback for STT/TTS, speaker identification, voice sessions persisted.

#### VS3.3 Advanced Voice Features (Week 9)

| # | Task | Est | Crate |
|---|------|-----|-------|
| VS3.3.1 | Per-agent voice personality (different voice per agent in multi-agent setup) | 4h | clawft-types |
| VS3.3.2 | Voice command shortcuts ("Hey Weft, check my email" -> direct tool invocation) | 4h | clawft-plugin |
| VS3.3.3 | Audio file input (process .wav/.mp3 files through STT) | 3h | clawft-tools |
| VS3.3.4 | Audio file output (save TTS to .wav file) | 3h | clawft-tools |
| VS3.3.5 | Latency benchmarking suite: speech-end to first-response-byte | 4h | tests/ |
| VS3.3.6 | WER (Word Error Rate) benchmarking against test corpus | 4h | tests/ |
| VS3.3.7 | Battery/CPU profiling: ensure wake word < 2%, full pipeline < 10% CPU | 3h | tests/ |
| VS3.3.8 | Voice permission integration: restrict voice-triggered tool execution by level | 3h | clawft-plugin |
| VS3.3.9 | End-to-end voice tests (Playwright + audio simulation) | 6h | tests/ |

**Sprint VS3 Deliverable:** Production voice system with cloud fallback, multi-speaker support, per-agent voices, comprehensive benchmarks.

---

## Dependency Matrix

### Voice Dependencies on Main Sprint

| Voice Task | Depends On | Status | Critical? |
|-----------|-----------|--------|-----------|
| VS1.1.1 (voice feature flag) | C1 (Plugin trait crate) | Not Started | Yes -- VoiceHandler trait must exist |
| VS1.3.1 (VoiceChannel) | C1, C7 (ChannelAdapter trait) | Not Started | Yes -- need plugin ChannelAdapter |
| VS1.3.2 (MessageBus integration) | D8 (Bounded bus channels) | Not Started | No -- works with unbounded (current) |
| VS1.3.8 (WS events) | UI S1.1.7 (WebSocket handler) | Not Started | No -- voice works without UI |
| VS2.3.7 (Discord voice) | E1 (Discord Resume) | Not Started | No -- basic Discord works |
| VS3.1.* (UI integration) | UI S1, S2 | Not Started | No -- voice works headless |

### Voice Dependencies That Block UI

| UI Task | Depends On (Voice) | Notes |
|---------|-------------------|-------|
| UI S3 Voice indicator | VS1.3.8 (WS voice events) | Need voice:status events |
| Tauri Voice Wake | VS2.1 (Wake Word) | Native mic in desktop shell |
| Talk Mode overlay | VS1.3.4 (Talk Mode) | Need running talk mode |

### Parallel Execution Plan

```text
Week  UI Sprint              Voice Sprint           Dependency Sprint
─────────────────────────────────────────────────────────────────────
 1    S1.1 Backend API        VS1.1 Audio Foundation  C1 Plugin traits
      S1.2 Frontend scaffold                          B5 Tool registry
 2    S1.3 Core views         VS1.2 STT + TTS         A1-A9 Critical fixes
 3    S1.3 (continued)        VS1.3 VoiceChannel      B1-B9 Cleanup
 4    S2.1 Live Canvas        VS2.1 Voice Wake         C3-C4 Skill loader
 5    S2.2 Skill browser      VS2.2 Echo cancellation  H1-H2 Memory
      S2.3 Memory explorer
 6    S2.4 Config editor      VS2.3 Platform integration L1 Agent routing
      S2.5 Cron + Channels
 7    S3.1 Delegation monitor VS3.1 UI Voice integration M1-M2 Flow delegator
 8    S3.3 Mobile + PWA       VS3.2 Cloud fallback       D5-D6 Latency/cost
      S3.4 Tauri desktop
 9    S3.5 Production harden  VS3.3 Advanced features    K4 ClawHub
```

---

## Integration with UI Sprint

### Shared Infrastructure

| Component | Owner | Used By |
|-----------|-------|---------|
| WebSocket event bus | UI Sprint (S1.1.7) | Voice events broadcast to UI |
| Axum API layer | UI Sprint (S1.1.2) | Voice status/config endpoints |
| Frontend component library | UI Sprint (S1.2) | Voice UI components use shadcn |
| Tauri shell | UI Sprint (S3.4) | Voice Wake runs in Tauri process |

### Voice-Specific UI Components

| Component | Sprint | Location |
|-----------|--------|----------|
| `VoiceStatusBar` | VS3.1.1 | `ui/src/components/voice/status-bar.tsx` |
| `TalkModeOverlay` | VS3.1.2 | `ui/src/components/voice/talk-overlay.tsx` |
| `AudioWaveform` | VS3.1.3 | `ui/src/components/voice/waveform.tsx` |
| `VoiceSettings` | VS3.1.4 | `ui/src/components/voice/settings.tsx` |
| `PushToTalkButton` | VS3.1.5 | `ui/src/components/voice/ptt-button.tsx` |

### Voice API Endpoints (Added to UI API)

```text
GET    /api/voice/status           -- Current voice pipeline state
POST   /api/voice/start            -- Start voice pipeline
POST   /api/voice/stop             -- Stop voice pipeline
GET    /api/voice/config           -- Voice configuration
PATCH  /api/voice/config           -- Update voice config
GET    /api/voice/models           -- List available/downloaded models
POST   /api/voice/models/download  -- Download a model
GET    /api/voice/devices          -- List audio devices (mic + speaker)
POST   /api/voice/test-mic         -- Test microphone (returns audio level)
POST   /api/voice/test-speak       -- Test TTS (speaks sample text)
```

---

## Platform Testing Matrix

### Minimum Hardware Requirements

| Tier | CPU | RAM | Storage | Suitable For |
|------|-----|-----|---------|-------------|
| Low | 2-core ARM (RPi 4) | 2 GB | 200 MB models | STT-only (no TTS, no wake word) |
| Medium | 4-core x86/ARM | 4 GB | 500 MB models | Full pipeline (no wake word daemon) |
| Full | 4+ core x86 | 8 GB | 1 GB models | Full pipeline + always-on wake word |

### Latency Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| VAD detection (speech end) | < 300ms | Time from last speech frame to VAD silence signal |
| STT completion | < 500ms after VAD | Time from silence signal to final transcription |
| Agent response start | < 2s | Time from transcription to first outbound text |
| TTS first audio byte | < 200ms | Time from text-ready to first audio sample |
| End-to-end (speech to response audio) | < 3s | Total pipeline latency |
| Wake word detection | < 500ms | Time from wake phrase to pipeline activation |
| Interruption response | < 50ms | Time from user-speech-detected to TTS-stopped |

### CI/CD Voice Testing

Voice tests cannot use real audio hardware in CI. Strategy:

1. **Unit tests:** Mock `cpal` with synthetic audio buffers
2. **Integration tests:** Pre-recorded WAV files fed through pipeline
3. **VAD tests:** Known speech/silence WAV files, assert correct boundaries
4. **STT accuracy tests:** Standard test corpus, compare against reference transcripts
5. **Platform tests:** Run on self-hosted runners with audio hardware (nightly, not PR-blocking)

---

## Timeline Summary

| Week | Voice Sprint | Key Deliverables |
|------|-------------|-----------------|
| 0 | VP1-VP5 | Audio prototype validated, models hosted, platform audio tested |
| 1 | VS1.1 | Audio capture/playback, VAD, model download |
| 2 | VS1.2 | STT + TTS working, voice_listen + voice_speak tools |
| 3 | VS1.3 | VoiceChannel adapter, basic Talk Mode, WS voice events |
| 4 | VS2.1 | Voice Wake ("Hey Weft"), custom wake word training |
| 5 | VS2.2 | Echo cancellation, noise suppression, multi-language |
| 6 | VS2.3 | Platform daemons (systemd/launchd), Discord voice, privacy indicator |
| 7 | VS3.1 | UI voice components (status bar, Talk Mode overlay, settings) |
| 8 | VS3.2 | Cloud STT/TTS fallback, speaker diarization |
| 9 | VS3.3 | Per-agent voices, benchmarks, voice permissions, E2E tests |

**Total: 9 weeks** (parallel with UI sprint, sharing weeks 1-9)

---

## Open Questions (Track in Sprint)

1. **Voice as channel vs tool vs both?**
   - Current plan: VoiceChannel (ChannelAdapter) for continuous mode + voice_listen/voice_speak tools for on-demand
   - Both coexist -- channel for Talk Mode, tools for agent-initiated voice

2. **Permission escalation via voice?**
   - Voice-triggered commands should respect the same permission levels as text
   - Level 0 users cannot voice-activate shell commands
   - Consider: require voice confirmation for destructive actions ("Are you sure you want to delete?")

3. **Multi-agent voice routing?**
   - In multi-agent setup with L1 routing, which agent handles voice?
   - Options: dedicated voice agent, or route to default agent, or voice menu ("Hey Weft, talk to work agent")

4. **Slack/Teams voice channels?**
   - Receive voice in a Slack/Teams call -> STT -> respond via TTS in same call
   - Requires platform-specific audio bridge (WebRTC for Slack, PSTN for Teams)
   - Deferred to post-sprint unless demand is high

5. **Voice cloning / custom voices?**
   - ElevenLabs offers voice cloning
   - Privacy/ethical considerations
   - Deferred to post-sprint

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| sherpa-rs build fails on platform | Blocks all voice work | VP1 prototype validates all platforms before sprint |
| Model download size (50+ MB) | Bad first-run experience | Progressive download, show progress, allow offline pre-download |
| Echo cancellation insufficient | Poor Talk Mode UX | Test hardware AEC fallback, document mic recommendations |
| cpal doesn't work on WSL2 | No voice for WSL users | Document PulseAudio bridge setup, fallback to cloud STT |
| Wake word false positives | Annoying activations | Adjustable sensitivity, visual confirmation before processing |
| CPU usage too high | Battery drain on laptops | Benchmark in VP1, offer quality tiers (low/medium/high) |
| VAD speech boundary too aggressive | Cuts off mid-sentence | Adaptive timeout, user-configurable, learn from corrections |

---

## Sources

- [sherpa-rs](https://crates.io/crates/sherpa-rs) -- Rust bindings to sherpa-onnx (STT + TTS + VAD)
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) -- Upstream multi-platform speech toolkit
- [rustpotter](https://github.com/GiviMAD/rustpotter) -- Open source wake word spotter
- [cpal](https://crates.io/crates/cpal) -- Cross-platform audio I/O
- [whisper-rs](https://crates.io/crates/whisper-rs) -- Fallback STT (batch mode)
- [piper-rs](https://github.com/thewh1teagle/piper-rs) -- Alternative TTS
- [voice_activity_detector](https://crates.io/crates/voice_activity_detector) -- Standalone Silero VAD
