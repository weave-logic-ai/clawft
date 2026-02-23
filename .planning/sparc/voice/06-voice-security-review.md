# Voice Security Review -- ClawFT Voice Development

> **Reviewer**: Security Architect Agent (SPARC)
> **Date**: 2026-02-23
> **Scope**: Voice pipeline (STT, TTS, VAD, Wake Word, Talk Mode, VoiceChannel), sprints VS1-VS3
> **Classification**: Internal -- Engineering Review
> **Status**: Initial Review (pre-implementation)

---

## Executive Summary

The voice development sprint introduces a significant new attack surface: always-on microphone access, audio data processing, speech-to-text transcription, and text-to-speech output. Unlike text-based channels, voice inherently captures ambient audio, operates in real-time, and can be weaponized for social engineering, surveillance, or command injection through adversarial audio.

This review enumerates 10 threat categories, defines security controls for each, establishes a platform security matrix, maps voice permissions to the existing permission level system, and provides security exit criteria for the implementation phase.

**Key findings:**
- Audio data privacy is the highest-risk area (T1, T6, T8): raw audio must never persist to disk by default and cloud fallback must be opt-in only
- Adversarial audio (T3) and permission escalation (T7) require defense-in-depth: text-level command validation after STT, not just audio-level filtering
- The wake word daemon (T2, T8) requires visible privacy indicators on all platforms to maintain user trust
- Plugin voice access (T10) must be gated through the existing WASM permission system with a dedicated `voice` capability

**Overall risk assessment: MANAGEABLE with controls specified below.**

---

## 1. Threat Model

### Threat Enumeration

| ID | Threat | Category | Likelihood | Impact | Score | Description |
|----|--------|----------|------------|--------|-------|-------------|
| T1 | Audio Data Privacy | Privacy | High | Critical | **10** | Microphone captures sensitive conversations, voice data retained in memory beyond processing window |
| T2 | Wake Word False Activation | Availability | Medium | Medium | **6** | Unintended command execution from ambient speech, TV audio, music, or background conversations |
| T3 | STT Adversarial Audio | Integrity | Low | Critical | **8** | Crafted audio that transcribes to malicious commands (e.g., "delete all files", "send credentials to attacker.com") |
| T4 | TTS Social Engineering | Integrity | Low | High | **6** | Agent voice used to impersonate user in calls or to manipulate other humans in earshot |
| T5 | Model Tampering | Integrity | Low | Critical | **8** | Corrupted or malicious STT/TTS models producing incorrect transcriptions or injecting hidden commands |
| T6 | Network Exfiltration | Privacy | Medium | Critical | **9** | Cloud fallback (Whisper API, ElevenLabs) sending raw audio to external services without explicit user consent |
| T7 | Permission Escalation | Authorization | Medium | Critical | **9** | Voice-triggered commands bypassing the Level 0/1/2 permission system, executing privileged operations without confirmation |
| T8 | Eavesdropping via Always-On Wake Word | Privacy | Medium | Critical | **9** | Background microphone access used for surveillance; wake word daemon processes all ambient audio |
| T9 | Audio Replay Attack | Authentication | Low | High | **6** | Pre-recorded audio replayed to trigger voice commands, bypassing liveness detection |
| T10 | Plugin Voice Access | Authorization | Medium | High | **8** | Untrusted WASM plugins accessing the voice pipeline to capture audio, inject TTS output, or register wake words |

### Risk Matrix

```
Impact ->     Low        Medium      High       Critical
Likelihood
  High                                          T1(10)
  Medium                 T2(6)                  T6(9) T7(9) T8(9)
  Low                               T4(6) T9(6) T3(8) T5(8) T10(8)
```

### Threat Severity Classification

- **Critical (Score 8-10)**: T1, T3, T5, T6, T7, T8, T10 -- Must be addressed before GA release
- **High (Score 6-7)**: T2, T4, T9 -- Must be addressed before public beta
- **Medium (Score 4-5)**: None identified
- **Low (Score 1-3)**: None identified

---

## 2. Security Controls

### SC-1: Mic Privacy (mitigates T1, T8)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Visual indicator when mic is active | Platform-native notification: tray icon (Linux/Windows), menu bar icon (macOS), status bar (iOS/Android). Must be visible at all times when any audio capture is active, including wake word detection. | P0 |
| OS-level permission prompts | Request microphone permission via OS APIs on first use. Do not silently access mic. macOS: `AVCaptureDevice.requestAccess`. Windows: Settings > Privacy > Microphone. Linux: PipeWire/PulseAudio permission prompt via desktop portal. | P0 |
| Mic mute hardware respect | Honor hardware mic mute switches. Check `cpal` device state before capture. If hardware mute is active, report status to user rather than silently failing. | P1 |
| Session-scoped mic access | Microphone stream is opened only for the duration of a voice session (Talk Mode or explicit `voice_listen` call). No persistent background mic access outside of wake word mode. | P0 |

### SC-2: Audio Data Retention (mitigates T1, T6)

| Control | Implementation | Priority |
|---------|---------------|----------|
| No raw audio stored by default | Audio buffers are processed in-memory by the VAD/STT pipeline and discarded immediately after transcription. No `.wav`, `.pcm`, or other audio files written to disk unless explicitly configured. | P0 |
| Configurable retention policy | `voice.audio_retention` config option: `none` (default), `session` (retain for current session, delete on exit), `persist` (retain in `~/.clawft/audio/`, user responsibility). | P1 |
| Memory zeroization | Audio buffers (`Vec<i16>`) are zeroized after STT processing using `zeroize` crate or explicit `ptr::write_bytes`. Prevents audio data from lingering in freed heap memory. | P1 |
| Transcription-only logging | Voice session logs contain only the text transcription and timestamp, never raw audio data. Log format: `[timestamp] [voice] user: "transcribed text"`. | P0 |

### SC-3: Cloud Fallback (mitigates T6)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Opt-in only | Cloud STT/TTS fallback is disabled by default. Must be explicitly enabled via `voice.cloud_fallback = true` in config. | P0 |
| Local-first default | All voice processing uses local sherpa-onnx models by default. Cloud APIs are only contacted when local processing fails AND `voice.cloud_fallback = true`. | P0 |
| Config flag with consent prompt | First-time enabling `voice.cloud_fallback` triggers a CLI prompt: "Audio will be sent to [provider]. Continue? (yes/no)". Config file changes require explicit `--i-understand-audio-is-sent-to-cloud` flag. | P0 |
| Provider transparency | When cloud fallback activates, log and display which provider receives audio: `[voice] Cloud fallback active: sending audio to OpenAI Whisper API`. | P1 |
| No audio caching by proxy | ClawFT does not cache or log audio sent to cloud providers. Audio is streamed directly from the pipeline to the API endpoint and discarded. | P0 |

### SC-4: Permission Enforcement (mitigates T7)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Voice commands respect Level 0/1/2 | Voice-transcribed text is processed through the same permission system as typed text. A Level 0 user cannot voice-activate shell commands. | P0 |
| Text-level validation | After STT transcription, the resulting text is validated against the permission system BEFORE execution. The agent loop does not distinguish between voice-originated and text-originated commands for permission checks. | P0 |
| Destructive action gate | Commands classified as destructive (file deletion, git operations, process termination) require explicit voice confirmation: "Are you sure? Say 'yes' to confirm." TTS speaks the pending action before confirmation. | P0 |
| Voice-specific permission flags | Dedicated permission flags: `voice_listen`, `voice_speak`, `wake_word`, `talk_mode`, `voice_exec_shell`, `voice_delegate`. Each independently controllable per permission level. | P0 |

### SC-5: Wake Word Sensitivity (mitigates T2)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Configurable threshold | `voice.wake_word.sensitivity` config option: `low` (fewer false activations, may miss quiet triggers), `medium` (default), `high` (catches quiet triggers, more false positives). Maps to rustpotter confidence threshold. | P1 |
| Visual confirmation before processing | After wake word detection, display a brief visual indicator (1-2 seconds) before activating the full pipeline. User can dismiss to cancel. Configurable via `voice.wake_word.confirm_before_listen = true` (default). | P1 |
| Cooldown period | After a wake word activation, ignore subsequent wake word triggers for `voice.wake_word.cooldown_seconds` (default: 3). Prevents rapid re-activation loops. | P2 |
| Environment awareness | Log wake word activation context (ambient noise level at trigger time) to help users diagnose false positive patterns. | P2 |

### SC-6: Voice Confirmation (mitigates T3, T7, T9)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Destructive action confirmation | Commands that modify files, execute shell commands, or delegate to external services require spoken "yes" confirmation after the agent reads back the pending action via TTS. | P0 |
| Confirmation timeout | If no confirmation is received within `voice.confirmation_timeout_seconds` (default: 10), the action is cancelled. Prevents stale confirmations from being accepted. | P1 |
| Anti-replay nonce | Confirmation prompts include a random word: "Say 'confirm delta' to proceed." The nonce word changes each time, defeating simple replay attacks. | P1 |
| Transcription echo | Before executing any voice command, echo the transcription to the user via TTS or display: "I heard: 'delete the config file'. Is that correct?" | P0 |

### SC-7: Model Integrity (mitigates T5)

| Control | Implementation | Priority |
|---------|---------------|----------|
| SHA-256 verification on download | Every model download is verified against a SHA-256 hash from the signed manifest file. Manifest is fetched over HTTPS from a pinned URL. | P0 |
| Signed manifests | Model manifest files are Ed25519-signed. Signature verification before any model file is loaded. Key is bundled in the binary. | P0 |
| Model provenance tracking | `~/.clawft/models/voice/manifest.json` records: model name, version, SHA-256 hash, download URL, download timestamp, signature status. | P1 |
| No arbitrary model loading | Users cannot load arbitrary `.onnx` files without explicitly disabling model verification via `--allow-unverified-models` flag. This flag emits a warning: "Loading unverified model. This may be unsafe." | P1 |

### SC-8: Rate Limiting (mitigates T2, T3, T9)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Max voice commands per minute | `voice.rate_limit.commands_per_minute` (default: 10). After exceeding the limit, additional voice commands are queued with a warning: "Rate limit reached. Please wait." | P1 |
| Max wake word activations per minute | `voice.rate_limit.wake_activations_per_minute` (default: 5). Prevents rapid-fire wake word triggering from noisy environments. | P1 |
| Cooldown after failed commands | After 3 consecutive failed voice commands (STT confidence below threshold), enter a 30-second cooldown to prevent ambient noise from flooding the pipeline. | P2 |

### SC-9: Audit Logging (mitigates T1, T3, T7, T9)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Voice command transcription logging | All voice command transcriptions logged to the session log with timestamp and confidence score. Format: `[2026-02-23T10:00:00Z] [voice] command="list files" confidence=0.92 source=wake_word`. | P0 |
| Text only, not audio | Audit logs contain only text transcriptions. Raw audio is NEVER written to audit logs. | P0 |
| Permission check logging | Log all permission checks triggered by voice commands, including denied commands: `[voice] DENIED: voice_exec_shell not permitted at Level 0`. | P0 |
| Cloud fallback logging | When cloud fallback is used, log the event: `[voice] cloud_fallback provider=openai-whisper audio_duration_ms=3200`. No audio content in logs. | P1 |
| Confirmation logging | Log all confirmation prompts and responses: `[voice] confirm_prompt action="delete config.json" nonce="delta" response="confirmed" latency_ms=2100`. | P1 |

### SC-10: Plugin Voice Access (mitigates T10)

| Control | Implementation | Priority |
|---------|---------------|----------|
| Voice capability in plugin manifest | Plugins must declare `capabilities: ["voice"]` in their manifest to access any voice API. Without this capability, all voice host functions return `Err("voice capability not granted")`. | P0 |
| Granular voice permissions | Voice access is split into sub-permissions: `voice.listen` (receive transcriptions), `voice.speak` (produce TTS output), `voice.raw_audio` (access raw audio buffers -- requires explicit user approval). | P0 |
| No raw audio access by default | WASM plugins NEVER receive raw audio buffers unless the plugin declares `voice.raw_audio` AND the user explicitly approves at install time. Default: plugins receive only text transcriptions. | P0 |
| Voice pipeline isolation | Each plugin that accesses voice receives its own `VoiceHandle` with scoped permissions. Plugins cannot intercept or modify another plugin's voice interactions. | P1 |

---

## 3. Platform Security Matrix

| Platform | Mic Permission Model | Privacy Indicator | Background Audio Policy | Audio Backend | Notes |
|----------|---------------------|-------------------|------------------------|---------------|-------|
| **Linux** | No OS-level prompt; app has implicit access via PulseAudio/PipeWire | Application-managed tray icon (XDG system tray) | Unrestricted; app manages own lifecycle | PipeWire / PulseAudio via `cpal` | Flatpak/Snap sandboxing can restrict mic access via portals. Document portal configuration for sandboxed installs. |
| **macOS** | OS prompt via `AVCaptureDevice.requestAccess(for: .audio)` | OS-native orange dot (mic indicator, macOS 12+) + app menu bar icon | Background audio allowed for signed apps; mic access persists until revoked in System Settings | CoreAudio via `cpal` | Notarization required for distribution. TCC (Transparency, Consent, Control) database tracks mic permission. App must handle permission revocation gracefully. |
| **Windows** | OS prompt via Settings > Privacy > Microphone; per-app toggle | OS-native mic icon in taskbar (Windows 11) + app system tray icon | Background audio allowed; mic access persists until revoked in Settings | WASAPI via `cpal` | Windows Defender SmartScreen may flag unsigned binaries. UWP/MSIX packaging provides additional sandboxing. |
| **iOS** | OS prompt via `AVAudioSession.requestRecordPermission()` | OS-native orange dot (iOS 14+) | Background audio requires `UIBackgroundModes: audio` in Info.plist; wake word daemon requires explicit background audio entitlement | CoreAudio via platform bridge | App Store review will scrutinize always-on mic access. Must justify background audio in review notes. Wake word may be rejected without clear user benefit disclosure. |
| **Android** | OS prompt via `RECORD_AUDIO` runtime permission (Android 6+) | OS-native green dot (Android 12+) | Foreground service required for continuous mic access; notification must be visible | OpenSL ES / AAudio via `cpal` (NDK) | Android 12+ restricts background mic access. Foreground service with persistent notification required for wake word daemon. |

### Platform-Specific Security Notes

1. **Linux**: No standardized mic permission model. ClawFT must provide its own first-run consent dialog before opening any audio stream. Log initial mic access to audit trail.

2. **macOS**: TCC revocation is silent -- the app receives empty audio buffers, not an error. Detect zero-energy audio frames and alert the user that mic permission may have been revoked.

3. **Windows**: WASAPI exclusive mode can block other apps from using the mic. ClawFT must use shared mode only. Document this requirement.

4. **Mobile (iOS/Android)**: Wake word daemon is the highest-risk feature for app store rejection. Consider making wake word a separate optional binary/service rather than embedding in the main app.

---

## 4. Data Flow Diagram

```text
                           AUDIO DATA FLOW -- SECURITY BOUNDARIES
                           ========================================

  ┌─────────────────────────────────────────────────────────────────────────┐
  │  ZONE 1: RAW AUDIO (in-memory only, never persisted)                   │
  │                                                                         │
  │  [Microphone]                                                           │
  │       │                                                                 │
  │       v                                                                 │
  │  ┌──────────┐   16kHz mono PCM                                          │
  │  │  cpal    │──────────────────┐                                        │
  │  │ capture  │                  │                                        │
  │  └──────────┘                  v                                        │
  │       │               ┌──────────────┐                                  │
  │       │               │  Ring Buffer │  480 samples/chunk (30ms)        │
  │       │               │  (10 chunks) │  300ms max audio in memory       │
  │       │               └──────┬───────┘                                  │
  │       │                      │                                          │
  │       │          ┌───────────┼───────────┐                              │
  │       │          │           │           │                              │
  │       │          v           v           v                              │
  │       │   ┌──────────┐ ┌─────────┐ ┌──────────────┐                    │
  │       │   │Wake Word │ │   VAD   │ │ Cloud STT    │ OPT-IN ONLY       │
  │       │   │(rustpotter)│ │(silero) │ │ (Whisper API)│──────────────┐    │
  │       │   └─────┬────┘ └────┬────┘ └──────┬───────┘              │    │
  │       │         │           │              │                      │    │
  │  ─────┼─────────┼───────────┼──────────────┼──────────────────────┼────│
  │  SECURITY BOUNDARY: RAW AUDIO DISCARDED AFTER THIS POINT          │    │
  │       │         │           │              │                      │    │
  └───────┼─────────┼───────────┼──────────────┼──────────────────────┼────┘
          │         │           │              │                      │
          │         │           v              │                      │
          │         │    ┌─────────────┐       │                      │
          │         │    │  Local STT  │       │                      │
          │         │    │ (sherpa-rs) │       │                      │
          │         │    └──────┬──────┘       │                      │
          │         │           │              │                      │
  ┌───────┼─────────┼───────────┼──────────────┼──────────────────────┼────┐
  │  ZONE 2: TEXT ONLY (audio fully discarded)  │                      │    │
  │       │         │           │              │                      │    │
  │       │         │           v              v                      │    │
  │       │         │    ┌──────────────────────────┐                 │    │
  │       │         │    │   Transcription Text     │                 │    │
  │       │         │    │   + confidence score     │                 │    │
  │       │         │    └────────────┬─────────────┘                 │    │
  │       │         │                 │                               │    │
  │       │    wake │                 v                               │    │
  │       │  trigger│    ┌──────────────────────────┐                 │    │
  │       │         │    │  Permission Check        │                 │    │
  │       │         │    │  (Level 0/1/2 gate)      │                 │    │
  │       │         │    └────────────┬─────────────┘                 │    │
  │       │         │                 │                               │    │
  │       │         │                 v                               │    │
  │       │         └───>┌──────────────────────────┐                 │    │
  │       │              │  Agent Loop (existing)   │                 │    │
  │       │              │  - processes text command │                 │    │
  │       │              │  - generates response    │                 │    │
  │       │              └────────────┬─────────────┘                 │    │
  │       │                           │                               │    │
  │       │                           v                               │    │
  │       │              ┌──────────────────────────┐                 │    │
  │       │              │  Response Text           │                 │    │
  │       │              └────────────┬─────────────┘                 │    │
  │       │                           │                               │    │
  │       │                    ┌──────┴──────┐                        │    │
  │       │                    v             v                        │    │
  │       │           ┌─────────────┐ ┌──────────┐                   │    │
  │       │           │  Local TTS  │ │Cloud TTS │ OPT-IN            │    │
  │       │           │ (sherpa-rs) │ │(ElevenLabs)│ ONLY             │    │
  │       │           └──────┬──────┘ └─────┬────┘                   │    │
  │       │                  │              │                         │    │
  └───────┼──────────────────┼──────────────┼─────────────────────────┼────┘
          │                  │              │                         │
  ┌───────┼──────────────────┼──────────────┼─────────────────────────┼────┐
  │  ZONE 3: AUDIO OUTPUT (generated, not captured)                   │    │
  │       │                  v              v                         │    │
  │       │           ┌──────────────────────────┐                   │    │
  │       │           │  Audio Output Buffer     │                   │    │
  │       │           │  (synthesized speech)    │                   │    │
  │       │           └────────────┬─────────────┘                   │    │
  │       │                        │                                 │    │
  │       │                        v                                 │    │
  │       │                 ┌─────────────┐                          │    │
  │       │                 │   Speaker   │                          │    │
  │       │                 │ (cpal out)  │                          │    │
  │       │                 └─────────────┘                          │    │
  └───────┼──────────────────────────────────────────────────────────┼────┘
          │                                                          │
  ┌───────┼──────────────────────────────────────────────────────────┼────┐
  │  ZONE 4: PERSISTENCE (text transcription only)                   │    │
  │       │                                                          │    │
  │       │  ┌──────────────────────────────────┐                    │    │
  │       │  │  Session Log (text only)         │                    │    │
  │       │  │  - transcription + timestamp     │  PERSISTED         │    │
  │       │  │  - confidence score              │  TO DISK           │    │
  │       │  │  - command result                │                    │    │
  │       │  │  - NO raw audio                  │                    │    │
  │       │  └──────────────────────────────────┘                    │    │
  │       │                                                          │    │
  │       │  ┌──────────────────────────────────┐                    │    │
  │       │  │  Audit Log                       │                    │    │
  │       │  │  - permission checks             │  PERSISTED         │    │
  │       │  │  - cloud fallback events         │  TO DISK           │    │
  │       │  │  - confirmation prompts          │                    │    │
  │       │  │  - NO raw audio                  │                    │    │
  │       │  └──────────────────────────────────┘                    │    │
  │       │                                                          │    │
  └───────┼──────────────────────────────────────────────────────────┼────┘
          │                                                          │
          │  ┌──────────────────────────────────┐                    │
          │  │  EXTERNAL (opt-in only)          │                    │
          │  │                                  │                    │
          │  │  Cloud STT: raw audio sent  ─────┼──> OpenAI Whisper  │
          │  │  Cloud TTS: text sent       ─────┼──> ElevenLabs     │
          │  │                                  │                    │
          │  │  ONLY when:                      │                    │
          │  │  - voice.cloud_fallback = true   │                    │
          │  │  - User explicitly consented     │                    │
          │  │  - Local processing failed       │                    │
          │  └──────────────────────────────────┘                    │
          │                                                          │
          └──────────────────────────────────────────────────────────┘

  LEGEND:
  ───── = data flow
  ┌───┐ = processing stage
  ZONE 1 = raw audio exists (in-memory only)
  ZONE 2 = text only (audio discarded)
  ZONE 3 = synthesized audio output
  ZONE 4 = persistent storage (text only)
```

### Critical Security Boundaries

1. **Zone 1 -> Zone 2 boundary**: Raw audio is discarded immediately after STT transcription. This is the most critical security boundary. Implementation must ensure no audio data leaks past this point.

2. **Cloud fallback**: Raw audio crosses the network boundary ONLY when `voice.cloud_fallback = true` AND local STT fails. This is the only path where raw audio leaves the local machine.

3. **Zone 4 persistence**: Only text transcriptions are persisted. Audio buffers are never serialized to disk unless the user explicitly enables `voice.audio_retention = persist`.

---

## 5. Voice Permission Matrix

| Permission Level | `voice_listen` | `voice_speak` | `wake_word` | `talk_mode` | `voice_exec_shell` | `voice_delegate` |
|-----------------|---------------|---------------|-------------|-------------|-------------------|-----------------|
| **Level 0** (Restricted) | DENIED | ALLOWED | DENIED | DENIED | DENIED | DENIED |
| **Level 1** (Standard) | ALLOWED | ALLOWED | ALLOWED | ALLOWED | DENIED | DENIED |
| **Level 2** (Elevated) | ALLOWED | ALLOWED | ALLOWED | ALLOWED | ALLOWED (with confirmation) | ALLOWED (with confirmation) |

### Permission Definitions

| Permission | Description | Risk Level |
|-----------|-------------|------------|
| `voice_listen` | Activate microphone for STT transcription. Includes VAD. | Medium -- mic access |
| `voice_speak` | Produce TTS audio output through speakers. | Low -- output only |
| `wake_word` | Enable always-on wake word detection daemon. | High -- continuous mic |
| `talk_mode` | Enable continuous listen-respond loop (Talk Mode). | High -- continuous mic + auto-execute |
| `voice_exec_shell` | Allow voice-transcribed commands to execute shell operations. | Critical -- voice to shell |
| `voice_delegate` | Allow voice-transcribed commands to delegate to external agents/services. | Critical -- voice to external |

### Permission Escalation Rules

1. `voice_exec_shell` requires `voice_listen` (cannot execute shell via voice without mic access)
2. `talk_mode` requires `voice_listen` AND `voice_speak` (bidirectional voice)
3. `wake_word` requires `voice_listen` (wake word triggers listening)
4. `voice_delegate` requires `voice_listen` (cannot delegate via voice without transcription)
5. All Level 2 permissions with `voice_exec_shell` or `voice_delegate` require per-command voice confirmation (SC-6)

### Plugin Voice Permissions

| Plugin Permission | Grants | Requires User Approval |
|------------------|--------|----------------------|
| `voice.listen` | Receive text transcriptions from the voice pipeline | Yes (install-time) |
| `voice.speak` | Inject text into the TTS pipeline for audio output | Yes (install-time) |
| `voice.raw_audio` | Receive raw audio buffers from the microphone | Yes (install-time + runtime confirmation) |
| `voice.wake_word` | Register custom wake words | Yes (install-time) |

---

## 6. Security Exit Criteria

### P0 -- Must pass before any voice feature ships

- [ ] Raw audio buffers are zeroized after STT processing; no audio data persists in heap after transcription
- [ ] No raw audio is written to disk by default (`voice.audio_retention = none`)
- [ ] Cloud STT/TTS fallback is disabled by default (`voice.cloud_fallback = false`)
- [ ] First-time cloud fallback enablement requires explicit user consent prompt
- [ ] Voice-transcribed commands are subject to the same Level 0/1/2 permission checks as text commands
- [ ] Destructive voice commands require spoken confirmation with anti-replay nonce
- [ ] Voice confirmation has a timeout (default: 10 seconds)
- [ ] STT model integrity verified via SHA-256 hash check on download
- [ ] Model manifest files are Ed25519-signed and signature is verified before loading
- [ ] Visual privacy indicator is displayed whenever microphone is active (all platforms)
- [ ] OS-level microphone permission is requested before first audio capture (macOS, Windows, iOS, Android)
- [ ] WASM plugins cannot access voice pipeline without declaring `voice` capability in manifest
- [ ] Plugins never receive raw audio unless explicitly granted `voice.raw_audio` AND user approves
- [ ] Audit log records all voice commands (text only), permission checks, and cloud fallback events
- [ ] Session logs contain only text transcriptions, never raw audio data

### P1 -- Must pass before public beta

- [ ] Wake word detection has configurable sensitivity threshold
- [ ] Wake word activation shows visual confirmation before processing (configurable)
- [ ] Rate limiting enforced: max 10 voice commands/minute, max 5 wake activations/minute
- [ ] Cloud fallback events are logged with provider name and audio duration (no audio content)
- [ ] Confirmation prompt responses are logged to audit trail
- [ ] Model provenance is tracked in manifest.json (name, version, hash, source, timestamp)
- [ ] Unverified model loading requires explicit `--allow-unverified-models` flag with warning
- [ ] Audio memory buffers use fixed-size ring buffer (300ms max) to bound memory exposure
- [ ] TTS output cannot be redirected to network streams without explicit configuration
- [ ] Plugin voice access is scoped per-plugin (no cross-plugin voice interception)

### P2 -- Should pass before GA release

- [ ] Wake word cooldown period prevents rapid re-activation (default: 3 seconds)
- [ ] Failed voice command cooldown prevents ambient noise flooding (3 failures -> 30s cooldown)
- [ ] Audio energy level monitoring detects potential TCC/permission revocation (macOS)
- [ ] Voice pipeline gracefully degrades when mic permission is revoked mid-session
- [ ] Speaker diarization does not leak identified speaker data outside the session
- [ ] Per-agent voice personality configuration does not enable voice cloning capabilities

---

## 7. Recommendations

Ranked by priority (highest first):

### Priority 1: Critical (implement in VS1)

**R1. Audio data lifecycle enforcement**
Implement a `SecureAudioBuffer` wrapper type that enforces zeroization on drop and prevents accidental serialization. This type should be the only way to hold raw audio data in the voice pipeline.

```rust
struct SecureAudioBuffer {
    data: Vec<i16>,
}

impl Drop for SecureAudioBuffer {
    fn drop(&mut self) {
        // Zeroize audio data on drop
        self.data.zeroize();
    }
}
```

**R2. Permission system integration before any voice tool ships**
The `voice_listen` and `voice_speak` tools must check permission levels before execution. Do not ship voice tools without permission integration, even for internal testing. A voice tool that bypasses permissions during development may ship to production.

**R3. Cloud fallback consent is a hard gate, not a soft default**
The cloud fallback consent prompt must block execution until the user responds. Do not fall through to cloud on timeout. If the user does not respond, the command fails with a clear error.

### Priority 2: High (implement in VS2)

**R4. Wake word daemon as a separate process**
Run the wake word daemon as a separate process (not a thread in the main clawft process). This allows:
- Independent process-level sandboxing
- OS-level resource monitoring
- Clean shutdown without affecting the main agent
- App store compliance (mobile): separate audio background service

**R5. Adversarial audio defense**
STT output should be validated against a command allowlist or intent classifier before execution. Raw transcription should not be passed directly to the agent loop as a command without sanitization. Consider:
- Command prefix requirement: voice commands must begin with an intent keyword
- Confidence threshold: reject transcriptions below configurable confidence (default: 0.7)
- Ambiguity resolution: if STT produces multiple hypotheses, present all to the user for selection

**R6. Replay attack mitigation**
Beyond the anti-replay nonce in confirmation prompts, consider:
- Voice activity timestamp correlation (command must follow wake word within 30 seconds)
- Acoustic environment fingerprinting (detect if the audio source is a speaker vs. live voice)
- Optional: voice enrollment for speaker verification (deferred, high complexity)

### Priority 3: Medium (implement in VS3)

**R7. Network isolation for voice processing**
The voice pipeline should not have direct network access. Cloud fallback should be implemented as a separate service/channel that the pipeline communicates with via IPC, not direct HTTP calls from the audio processing thread. This prevents a compromised STT model from exfiltrating data.

**R8. Formal threat model review after VS1**
After the initial voice pipeline is functional, conduct a focused security review with real audio input/output testing. The threat model in this document is based on specification review; a live system may reveal additional attack vectors.

**R9. User education**
Provide clear documentation on:
- What data is captured and where it goes
- How to verify that cloud fallback is disabled
- How to revoke microphone permissions per platform
- How to audit voice command history
- What the privacy indicator looks like on each platform

**R10. Mobile platform compliance**
iOS App Store and Google Play have specific policies for apps with background audio access. Prepare review justification documents before submission. Consider making wake word a separate app extension (iOS) or foreground service (Android) to comply with platform policies.

---

## Appendix A: Threat-to-Control Mapping

| Threat | Primary Controls | Secondary Controls |
|--------|-----------------|-------------------|
| T1 (Audio Privacy) | SC-1 (Mic Privacy), SC-2 (Retention) | SC-9 (Audit) |
| T2 (False Activation) | SC-5 (Wake Sensitivity) | SC-8 (Rate Limit) |
| T3 (Adversarial Audio) | SC-6 (Voice Confirm), SC-4 (Permissions) | SC-8 (Rate Limit), SC-9 (Audit) |
| T4 (Social Engineering) | SC-10 (Plugin Access) | SC-4 (Permissions) |
| T5 (Model Tampering) | SC-7 (Model Integrity) | -- |
| T6 (Network Exfiltration) | SC-3 (Cloud Fallback), SC-2 (Retention) | SC-9 (Audit) |
| T7 (Permission Escalation) | SC-4 (Permissions), SC-6 (Voice Confirm) | SC-9 (Audit) |
| T8 (Eavesdropping) | SC-1 (Mic Privacy), SC-2 (Retention) | SC-5 (Wake Sensitivity), SC-9 (Audit) |
| T9 (Replay Attack) | SC-6 (Voice Confirm), SC-8 (Rate Limit) | SC-9 (Audit) |
| T10 (Plugin Voice Access) | SC-10 (Plugin Access) | SC-4 (Permissions) |

## Appendix B: Comparison with WASM Security Model

The voice security model extends the WASM plugin security model (Element 04, Section 4) with voice-specific capabilities:

| WASM Security Concept | Voice Equivalent |
|----------------------|-----------------|
| `permissions.network` allowlist | `voice.cloud_fallback` opt-in flag |
| `permissions.filesystem` paths | `voice.audio_retention` config (no disk access by default) |
| `permissions.env_vars` allowlist | `voice.listen/speak/raw_audio` capability flags |
| Fuel metering (CPU limit) | Rate limiting (commands/minute) |
| Memory limit (16 MB default) | Ring buffer size (300ms max audio) |
| Plugin manifest capabilities | `capabilities: ["voice"]` declaration |
| First-run permission approval | Mic permission OS prompt + voice capability approval |
| Audit logging per host function | Audit logging per voice command |

---

> Security review complete. All voice features must satisfy P0 exit criteria before shipping. P1 criteria before public beta. P2 criteria before GA release.
