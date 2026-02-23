# SPARC Voice Element: Voice Pipeline & Integration

**Workstream**: G (Voice)
**Timeline**: Weeks 1-9 (parallel with UI sprint)
**Status**: Not Started
**Dependencies**: C1 (Plugin trait crate -- VoiceHandler trait), UI Sprint S1.1.7 (WebSocket handler for voice events)
**Blocks**: UI Sprint S3 (voice UI components), K6 (native shells Voice Wake)

---

## 1. Summary

Implement a complete voice pipeline for ClawFT: audio capture/playback (cpal), voice activity detection (Silero VAD via sherpa-rs), speech-to-text (sherpa-rs streaming), text-to-speech (sherpa-rs streaming), wake word detection (rustpotter), and a VoiceChannel adapter that bridges audio I/O to the existing MessageBus. Includes Talk Mode (continuous listen-think-speak loop), echo cancellation, platform daemon integration, cloud STT/TTS fallback, and UI voice components. All voice features are behind granular feature flags (`voice`, `voice-stt`, `voice-tts`, `voice-wake`).

---

## 2. Phases

### Phase VP: Pre-Implementation Validation (Week 0) -- P0

| Deliverable | Description | Crate / Location |
|-------------|-------------|------------------|
| Audio pipeline prototype | Standalone binary: mic (cpal) -> VAD (sherpa-rs) -> STT (sherpa-rs) -> print text | voice-proto/ |
| Model hosting + download | GitHub Releases + HuggingFace mirror, SHA-256 integrity, `~/.clawft/models/voice/` | voice-proto/ |
| Feature flag design | `voice`, `voice-stt`, `voice-tts`, `voice-wake` feature gates in Cargo.toml | clawft-plugin |
| Platform audio validation | cpal capture + playback on Linux (PipeWire, PulseAudio), macOS (CoreAudio), Windows (WASAPI), WSL2 | voice-proto/ |
| Echo cancellation feasibility | Evaluate loopback subtraction, WebRTC AEC crate, and OS-level AEC; decide approach | voice-proto/ |

### Phase VS1.1: Audio Foundation (Week 1) -- P0

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Voice dependencies | Add `sherpa-rs`, `cpal` to `clawft-plugin/Cargo.toml` behind `voice` feature | clawft-plugin |
| Voice module structure | Create `clawft-plugin/src/voice/mod.rs` with submodule layout | clawft-plugin |
| AudioCapture | `cpal` microphone input stream wrapper (16 kHz, 16-bit PCM, mono) | clawft-plugin |
| AudioPlayback | `cpal` speaker output stream wrapper | clawft-plugin |
| VoiceActivityDetector | Silero VAD V5 wrapper via sherpa-rs | clawft-plugin |
| Model download manager | Fetch, cache, SHA-256 integrity check for STT/TTS/VAD models | clawft-plugin |
| VoiceConfig types | `VoiceConfig` struct in `clawft-types` (sample rate, chunk size, silence timeout, etc.) | clawft-types |
| Feature flag wiring | `voice`, `voice-stt`, `voice-tts`, `voice-wake` across workspace crates | Multiple |

### Phase VS1.2: STT + TTS (Week 2) -- P0

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| SpeechToText | sherpa-rs streaming recognizer with partial result callbacks | clawft-plugin |
| TextToSpeech | sherpa-rs streaming synthesizer with playback-before-complete | clawft-plugin |
| TTS cancellation | Abort handle for interruption support | clawft-plugin |
| `voice_listen` tool | On-demand transcription tool for agent use | clawft-tools |
| `voice_speak` tool | On-demand TTS tool for agent use | clawft-tools |
| CLI voice commands | `weft voice setup`, `weft voice test-mic`, `weft voice test-speak` | clawft-cli |

### Phase VS1.3: VoiceChannel + Talk Mode (Week 3) -- P0

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| VoiceChannel adapter | Implement `VoiceChannel` as `ChannelAdapter` plugin | clawft-plugin |
| Inbound integration | VoiceChannel -> MessageBus (transcriptions as InboundMessage) | clawft-plugin |
| Outbound integration | MessageBus -> VoiceChannel TTS (outbound text spoken aloud) | clawft-plugin |
| Talk Mode controller | Continuous listen -> transcribe -> think -> speak -> listen loop | clawft-plugin |
| Silence timeout config | Configurable speech boundary detection | clawft-types |
| Interruption detection | Stop TTS playback when user starts speaking | clawft-plugin |
| CLI talk command | `weft voice talk` -- start Talk Mode session | clawft-cli |
| WebSocket voice events | `voice:status` (idle/listening/processing/speaking) over WS | clawft-services |
| Pipeline tests | Unit + integration tests for full audio pipeline | tests/ |

### Phase VS2.1: Voice Wake (Week 4) -- P1

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| rustpotter dependency | Add `rustpotter` behind `voice-wake` feature flag | clawft-plugin |
| Wake word model | Train "Hey Weft" model (5-8 samples via rustpotter CLI) | models/ |
| WakeWordDetector | rustpotter integration with MFCC+DTW/neural matching | clawft-plugin |
| Wake -> VAD activation | Trigger full listening pipeline on wake word detection | clawft-plugin |
| CLI wake command | `weft voice wake` -- start always-on listener | clawft-cli |
| CLI train command | `weft voice train-wake "hey weft"` -- guided wake word training | clawft-cli |
| Custom wake words | User-trained models stored in `~/.clawft/models/wake/` | clawft-plugin |
| CPU budget enforcement | Ensure wake word detection uses < 2% CPU | clawft-plugin |

### Phase VS2.2: Echo Cancellation + Quality (Week 5) -- P1

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Software AEC | Loopback subtraction or WebRTC AEC crate integration | clawft-plugin |
| Noise suppression | Pre-filter via sherpa-rs built-in or separate crate | clawft-plugin |
| Adaptive silence timeout | Learn user speech patterns, adjust boundary detection over time | clawft-plugin |
| Multi-language STT | Language auto-detection or config-based selection (12+ languages) | clawft-plugin |
| Voice selection | Multiple TTS voices, per-agent voice config in VoiceConfig | clawft-types |
| Audio quality metrics | SNR, latency percentiles, WER estimation logging | clawft-plugin |

### Phase VS2.3: Platform Integration (Week 6) -- P1

| Deliverable | Description | Crate / Location |
|-------------|-------------|------------------|
| Linux systemd service | User service unit for Voice Wake daemon | scripts/ |
| macOS launchd agent | Plist for Voice Wake daemon | scripts/ |
| Windows startup task | Scheduled task for Voice Wake daemon | scripts/ |
| Mic permission handling | OS-native permission request (macOS TCC, Windows privacy settings) | clawft-plugin |
| Privacy indicator | Visual notification when mic is active (tray icon or terminal badge) | clawft-plugin |
| PipeWire integration | Native PipeWire audio support for Linux | clawft-plugin |
| Discord voice bridge | VC audio -> STT -> agent -> TTS -> VC response | clawft-channels |
| Platform audio test suite | Automated tests on CI with synthetic audio | tests/ |

### Phase VS3.1: UI Voice Integration (Week 7) -- P2

| Deliverable | Description | Crate / Location |
|-------------|-------------|------------------|
| VoiceStatusBar | UI component: idle/listening/processing/speaking indicator | ui/ |
| TalkModeOverlay | Floating transcript + stop/mute buttons | ui/ |
| AudioWaveform | Real-time mic input waveform visualizer | ui/ |
| VoiceSettings | Settings panel: mic select, voice select, language, wake word toggle | ui/ |
| PushToTalkButton | Hold to speak, release to process | ui/ |
| WS partial transcription | Streaming partial transcription to UI over WebSocket | clawft-services |
| WS TTS progress | Word highlighting during TTS playback | clawft-services |
| Tauri voice integration | Native mic access from Tauri desktop shell | ui/ |

### Phase VS3.2: Cloud Fallback (Week 8) -- P2

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Cloud STT fallback | OpenAI Whisper API integration | clawft-plugin |
| Cloud TTS fallback | ElevenLabs / OpenAI TTS API integration | clawft-plugin |
| Fallback chain | Local-first -> cloud on failure or low confidence | clawft-plugin |
| Speaker diarization | Multi-speaker identification via sherpa-rs | clawft-plugin |
| Conversation mode | Distinguish speakers in multi-party voice sessions | clawft-plugin |
| Voice transcript logging | Persist voice conversations to session history | clawft-core |

### Phase VS3.3: Advanced Features (Week 9) -- P2

| Deliverable | Description | Crate / Location |
|-------------|-------------|------------------|
| Per-agent voice personality | Different TTS voice per agent in multi-agent setup | clawft-types |
| Voice command shortcuts | "Hey Weft, check email" -> direct tool invocation | clawft-plugin |
| Audio file input | Process .wav/.mp3 files through STT pipeline | clawft-tools |
| Audio file output | Save TTS output to .wav file | clawft-tools |
| Latency benchmarks | Speech-end to first-response-byte benchmarking suite | tests/ |
| WER benchmarks | Word Error Rate testing against standard corpus | tests/ |
| CPU/battery profiling | Wake word < 2%, full pipeline < 10% CPU validation | tests/ |
| Voice permissions | Restrict voice-triggered tool execution by permission level | clawft-plugin |
| End-to-end voice tests | Playwright + audio simulation E2E test suite | tests/ |

---

## 2.5 Internal Dependency Graph

```
VP (pre-validation)
  |
  +---> VS1.1 (audio foundation) -- requires VP platform validation
  |       |     also depends on: C1 (VoiceHandler trait from Plugin trait crate)
  |       |
  |       +---> VS1.2 (STT + TTS) -- requires AudioCapture, AudioPlayback, VAD
  |       |       |
  |       |       +---> VS1.3 (VoiceChannel + Talk Mode) -- requires STT + TTS
  |       |               |     also depends on: C7 (ChannelAdapter trait)
  |       |               |     also depends on: UI S1.1.7 (WebSocket handler for voice events)
  |       |               |
  |       |               +---> VS2.1 (Voice Wake) -- requires working audio pipeline
  |       |               |       |
  |       |               |       +---> VS2.2 (Echo Cancellation) -- requires full pipeline + wake
  |       |               |       |       |
  |       |               |       |       +---> VS2.3 (Platform Integration) -- requires stable pipeline
  |       |               |       |               |     also depends on: E1 (Discord Resume for voice bridge)
  |       |               |       |               |
  |       |               |       |               +---> VS3.1 (UI Voice) -- requires WS events + stable backend
  |       |               |       |               |       |     also depends on: UI S1, S2 (frontend scaffold)
  |       |               |       |               |       |
  |       |               |       |               |       +---> VS3.2 (Cloud Fallback) -- requires local pipeline stable
  |       |               |       |               |       |       |
  |       |               |       |               |       |       +---> VS3.3 (Advanced Features) -- final polish
  |       |               |       |               |       |
  |       |               |       |               +---> K6 (native shells Voice Wake) -- blocked by VS2.1, VS2.3
```

### Cross-Workstream Dependencies

- **C1 -> VS1.1**: VoiceHandler trait must exist before voice module can implement it
- **C7 -> VS1.3**: ChannelAdapter trait unification before VoiceChannel adapter
- **UI S1.1.7 -> VS1.3**: WebSocket handler needed for voice status events
- **VS1.3 -> UI S3**: Voice status events must be emitting before UI can display them
- **VS2.1 + VS2.3 -> K6**: Wake word and platform daemons before native shell integration
- **E1 -> VS2.3**: Discord resume/reconnect before Discord voice bridge

---

## 3. Exit Criteria

### VP Exit Criteria (Pre-Implementation Validation)

- [ ] Standalone prototype (`voice-proto/`) passes: mic -> VAD -> STT -> text on Linux, macOS, Windows
- [ ] End-to-end latency: speech-end to text-available < 500ms
- [ ] VAD false trigger rate < 3% on silence
- [ ] STT accuracy > 90% on common English phrases
- [ ] Model download + SHA-256 integrity check works from GitHub Releases
- [ ] Echo cancellation approach decided and documented

### VS1.1 Exit Criteria (Audio Foundation)

- [ ] `cpal` AudioCapture streams 16 kHz mono PCM from default mic
- [ ] `cpal` AudioPlayback plays audio to default speaker
- [ ] Silero VAD correctly detects speech start/end boundaries
- [ ] Model download manager fetches, caches, and validates models in `~/.clawft/models/voice/`
- [ ] `VoiceConfig` type defined in `clawft-types` with all audio parameters
- [ ] Feature flags compile correctly: `--features voice`, `--features voice-stt`, `--no-default-features`

### VS1.2 Exit Criteria (STT + TTS)

- [ ] Streaming STT produces partial transcriptions during speech
- [ ] Final STT transcription available within 500ms of speech end
- [ ] Streaming TTS begins audio playback before full synthesis completes
- [ ] TTS abort handle stops playback within 50ms
- [ ] `voice_listen` tool returns transcription text to agent
- [ ] `voice_speak` tool speaks text through default speaker
- [ ] `weft voice test-mic` and `weft voice test-speak` CLI commands functional

### VS1.3 Exit Criteria (VoiceChannel + Talk Mode)

- [ ] VoiceChannel implements ChannelAdapter and registers with PluginHost
- [ ] Transcribed speech arrives as InboundMessage on MessageBus
- [ ] Agent outbound text is spoken via TTS through VoiceChannel
- [ ] Talk Mode completes full loop: listen -> transcribe -> agent -> speak -> listen
- [ ] Interruption detection stops TTS within 50ms of user speech
- [ ] `weft voice talk` starts and maintains a Talk Mode session
- [ ] WebSocket emits `voice:status` events (idle/listening/processing/speaking)
- [ ] All pipeline unit and integration tests pass

### VS2.1 Exit Criteria (Voice Wake)

- [ ] "Hey Weft" wake word detected with < 500ms latency
- [ ] Wake word detection uses < 2% CPU in idle monitoring
- [ ] Wake word triggers VAD+STT pipeline activation
- [ ] `weft voice wake` starts background wake word listener
- [ ] `weft voice train-wake` creates custom wake word model
- [ ] Custom wake word models stored and loaded from `~/.clawft/models/wake/`

### VS2.2 Exit Criteria (Echo Cancellation + Quality)

- [ ] Echo cancellation prevents TTS output from being re-transcribed
- [ ] Noise suppression reduces background noise without clipping speech
- [ ] Multi-language STT works for at least 5 languages beyond English
- [ ] Multiple TTS voices selectable per-agent
- [ ] Audio quality metrics (SNR, latency) logged and queryable

### VS2.3 Exit Criteria (Platform Integration)

- [ ] Linux systemd user service starts/stops Voice Wake daemon
- [ ] macOS launchd agent starts/stops Voice Wake daemon
- [ ] Windows scheduled task starts/stops Voice Wake daemon
- [ ] Microphone permission requests handled on macOS and Windows
- [ ] Privacy indicator shows when mic is active
- [ ] Discord voice bridge: receive VC audio -> STT -> agent -> TTS -> VC audio
- [ ] Platform audio test suite passes on CI with synthetic audio

### VS3.1 Exit Criteria (UI Voice Integration)

- [ ] VoiceStatusBar reflects real-time pipeline state
- [ ] TalkModeOverlay shows live transcript and stop/mute controls
- [ ] AudioWaveform displays real-time mic input
- [ ] VoiceSettings panel allows mic/voice/language/wake word configuration
- [ ] Push-to-talk button works in browser and Tauri
- [ ] Partial transcriptions stream to UI in real time
- [ ] TTS word highlighting works during playback

### VS3.2 Exit Criteria (Cloud Fallback)

- [ ] Cloud STT (Whisper API) produces transcription when local STT fails
- [ ] Cloud TTS (ElevenLabs/OpenAI) produces audio when local TTS fails
- [ ] Fallback chain: local attempted first, cloud only on failure/low confidence
- [ ] Speaker diarization identifies 2+ speakers in a conversation
- [ ] Voice conversations persisted to session history

### VS3.3 Exit Criteria (Advanced Features)

- [ ] Per-agent voice personality: different agents use different TTS voices
- [ ] Voice command shortcuts route to correct tools
- [ ] Audio file input (.wav/.mp3) transcribed through STT
- [ ] Audio file output (.wav) saved from TTS
- [ ] Latency benchmark: end-to-end < 3s (speech to response audio)
- [ ] WER benchmark: < 10% on standard English test corpus
- [ ] CPU profiling: wake word < 2%, full pipeline < 10%
- [ ] Voice-triggered tool execution respects permission levels
- [ ] E2E voice tests pass with simulated audio

---

## 4. Security Requirements

### 4.1 Microphone Privacy

| Requirement | Implementation |
|-------------|---------------|
| Explicit opt-in | Voice features disabled by default; user must run `weft voice setup` or enable in config |
| Mic permission gating | OS-native permission request before first capture (macOS TCC, Windows privacy) |
| Privacy indicator | Visible indicator (tray icon, terminal badge, UI status bar) whenever mic is active |
| No silent recording | Audio data never persisted to disk unless user explicitly enables transcript logging |
| Mic kill switch | `weft voice stop` and UI mute button immediately close audio stream |

### 4.2 Audio Data Retention

| Requirement | Implementation |
|-------------|---------------|
| Ephemeral audio buffers | Raw audio frames held only in ring buffer (< 1s); discarded after STT processing |
| No audio recording by default | Audio is not saved to disk; only transcribed text is retained (if logging enabled) |
| Transcript retention policy | Voice transcripts follow same session retention policy as text messages |
| Cloud fallback data | Cloud STT/TTS requests use ephemeral API calls; audio not stored server-side (per provider TOS) |
| Model data isolation | Downloaded models stored in `~/.clawft/models/voice/` with user-only permissions (0700) |

### 4.3 Voice-Triggered Permissions

| Requirement | Implementation |
|-------------|---------------|
| Same permission model as text | Voice-triggered commands respect identical permission levels as typed commands |
| No privilege escalation via voice | Level 0 users cannot voice-activate shell commands or destructive tools |
| Destructive action confirmation | Voice-triggered destructive actions require verbal confirmation ("Are you sure?") |
| Wake word scope | Wake word only activates the voice pipeline; does not bypass authentication |
| Voice command allow-list | Configurable allow-list for voice-executable tools (default: non-destructive only) |

### 4.4 Network Security (Cloud Fallback)

| Requirement | Implementation |
|-------------|---------------|
| API key handling | Cloud STT/TTS API keys stored via SecretRef (A6 pattern), never in config plaintext |
| TLS enforcement | All cloud API calls use TLS 1.2+ |
| Data minimization | Send only the minimum audio segment needed for transcription |
| Fallback disable option | Users can disable cloud fallback entirely (`voice.cloud_fallback = false`) |

---

## 5. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| sherpa-rs build failure on target platform | Medium | Critical | **8** | VP prototype validates all platforms before sprint starts. Maintain fallback to whisper-rs (batch STT) and piper-rs (TTS). |
| Model download size (80+ MB total) degrades first-run UX | High | Medium | **6** | Progressive download with progress bar. `weft voice setup` command for explicit pre-download. Offline mode with pre-downloaded models. |
| Echo cancellation insufficient for Talk Mode | Medium | High | **6** | VP5 evaluates three AEC approaches. Hardware AEC fallback documented. Recommend headset for best experience. Push-to-talk as alternative to continuous mode. |
| cpal does not work on WSL2 | High | Low | **4** | Document PulseAudio bridge setup for WSL2. Cloud STT fallback covers no-mic scenarios. WSL2 is not a primary target. |
| Wake word false positives annoy users | Medium | Medium | **4** | Adjustable sensitivity threshold in config. Visual confirmation before processing. Easy disable via `voice.wake_enabled = false`. |
| CPU usage exceeds budget on low-end hardware | Medium | Medium | **4** | VP1 benchmarks CPU on target tiers. Quality tiers (low/medium/high) reduce CPU. Wake word daemon has hard 2% CPU limit with auto-throttle. |
| VAD cuts off speech mid-sentence | Medium | Medium | **4** | Adaptive silence timeout learns user speech patterns. User-configurable timeout (default 1.5s). Partial transcriptions allow correction. |
| Week 7-9 overload (voice + UI integration simultaneous) | Medium | Medium | **6** | VS3.1 UI work can be done by UI sprint team. VS3.2 and VS3.3 are P2; defer to follow-up if behind schedule. Voice works headless without UI integration. |
| Cloud API cost accumulation from fallback | Low | Medium | **3** | Fallback disabled by default. Local-first always attempted. Usage metrics warn on high cloud call rate. |
| Privacy backlash from always-on mic (wake word) | Low | High | **4** | Wake word disabled by default, requires explicit opt-in. Clear privacy indicator. No audio ever persisted. Mic kill switch always available. |
