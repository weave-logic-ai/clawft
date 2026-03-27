# Step 1 / VS1.1: Voice Module Foundation

**Date:** 2026-02-24
**Phase:** VS1.1 (Audio Foundation)
**Branch:** feature/three-workstream-implementation

## Summary

Created the voice pipeline module skeleton in `clawft-plugin` with stub
implementations for all pipeline components. No real audio library
dependencies are introduced yet -- sherpa-rs and cpal integration is
deferred to VP (pre-implementation validation).

## Module Structure Created

```
crates/clawft-plugin/src/voice/
  mod.rs       -- Module root with feature-gated submodule declarations
  config.rs    -- VoicePipelineConfig runtime configuration
  models.rs    -- ModelDownloadManager for STT/TTS/VAD model caching
  capture.rs   -- AudioCapture stub (cpal microphone input)
  playback.rs  -- AudioPlayback stub (cpal speaker output)
  vad.rs       -- VoiceActivityDetector stub (Silero VAD)
  stt.rs       -- SpeechToText stub (sherpa-rs streaming recognizer)
  tts.rs       -- TextToSpeech stub with TtsAbortHandle (sherpa-rs)
```

## Feature Flag Layout

```toml
[features]
voice      = ["voice-vad"]    # Top-level voice flag, implies VAD
voice-stt  = []               # Speech-to-text (empty until VP)
voice-tts  = []               # Text-to-speech (empty until VP)
voice-vad  = []               # Voice activity detection (empty until VP)
voice-wake = []               # Wake-word detection (reserved)
```

### Conditional Compilation Map

| Module       | Required Feature |
|-------------|-----------------|
| `config`     | `voice` (always compiled when voice enabled) |
| `models`     | `voice` (always compiled when voice enabled) |
| `capture`    | `voice-vad` |
| `playback`   | `voice-vad` |
| `vad`        | `voice-vad` |
| `stt`        | `voice-stt` |
| `tts`        | `voice-tts` |

## What Is Real vs. Placeholder

### Real (functional now)
- Feature flag gating and conditional compilation
- `VoicePipelineConfig` with sensible defaults (~/.clawft/models/voice/)
- `ModelDownloadManager` structure, `is_cached()` / `model_path()` logic
- `ModelInfo` catalog entries (URLs are real, SHA-256 hashes are placeholders)
- `TtsAbortHandle` atomic cancellation mechanism
- All public API surface (method signatures, types, enums)

### Stub (placeholder for VP)
- `AudioCapture::start()` -- just sets `active = true`, no cpal stream
- `AudioPlayback::start()` -- just sets `active = true`, no cpal stream
- `VoiceActivityDetector::process()` -- always returns `VadEvent::Silence`
- `SpeechToText::process()` / `finalize()` -- returns empty results
- `TextToSpeech::synthesize()` -- returns empty audio samples
- Model download/verification (no HTTP client wired yet)
- SHA-256 hashes in `ModelInfo` (marked PLACEHOLDER)

## Verification

- `cargo check -p clawft-plugin --features voice` -- PASS
- `cargo check -p clawft-plugin --features voice,voice-stt,voice-tts,voice-wake` -- PASS
- `cargo check -p clawft-plugin` (default, no voice) -- PASS
- `cargo check --workspace` -- PASS
- `cargo test -p clawft-plugin` -- 73/73 PASS

## Pre-existing Fix

Fixed `clawft-platform/Cargo.toml` workspace inheritance issue:
removed redundant `default-features = false` on `clawft-types` dep
(workspace root already specifies it; Cargo 2024 edition rejects the
duplicate).

## Next Steps (VP Validation Required)

1. **VP: sherpa-rs** -- Validate sherpa-rs builds on Linux/macOS/Windows,
   check binary size impact, confirm ONNX runtime compatibility
2. **VP: cpal** -- Validate cpal audio capture/playback on target
   platforms, measure latency at 16 kHz mono
3. **VS1.2: Wire pipeline** -- Connect capture -> VAD -> STT -> agent
   pipeline using tokio channels
4. **VS1.3: TTS output** -- Connect agent response -> TTS -> playback
   with abort-on-new-input support
5. **Model SHA-256** -- Download actual models, compute real hashes,
   update `ModelInfo` entries
