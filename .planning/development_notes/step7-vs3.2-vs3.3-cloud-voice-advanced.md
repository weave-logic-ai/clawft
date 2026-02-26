# Step 7: VS3.2 Cloud Fallback + VS3.3 Advanced Voice Features

## Summary

Implemented VS3.2 (Cloud Fallback + Quality) and VS3.3 (Advanced Voice Features) for the voice pipeline. These tasks add cloud STT/TTS providers, local-first fallback chains, transcription logging, per-agent voice personalities, voice command shortcuts, and audio file tools.

## Changes by Deliverable

### VS3.2: Cloud Fallback + Quality

#### 1. Cloud STT Provider (`crates/clawft-plugin/src/voice/cloud_stt.rs`)
- `CloudSttProvider` trait: `name()`, `transcribe(audio_data, mime_type, language) -> CloudSttResult`
- `CloudSttResult`: text, confidence, language, duration_ms
- `WhisperSttProvider`: OpenAI Whisper API via reqwest multipart
  - POST to `https://api.openai.com/v1/audio/transcriptions`
  - Bearer auth, model "whisper-1", response_format "verbose_json"
  - MIME-to-extension mapping for wav/webm/mp3/ogg
  - Default confidence 0.95 (Whisper does not return confidence)
- 4 unit tests (name, builder, MIME mapping, result fields, network error)

#### 2. Cloud TTS Provider (`crates/clawft-plugin/src/voice/cloud_tts.rs`)
- `CloudTtsProvider` trait: `name()`, `available_voices()`, `synthesize(text, voice_id) -> CloudTtsResult`
- `CloudTtsResult`: audio_data, mime_type, duration_ms
- `VoiceInfo`: id, name, language (serde::Serialize)
- `OpenAiTtsProvider`: 6 voices (alloy/echo/fable/onyx/nova/shimmer), mp3 output
- `ElevenLabsTtsProvider`: 4 voices (Rachel/Domi/Bella/Antoni), mpeg output
- 9 tests across both providers (names, voices, builders, serialization, network errors)

#### 3. Fallback Chain (`crates/clawft-plugin/src/voice/fallback.rs`)
- `LocalSttEngine` trait: `transcribe(audio_data, language) -> LocalSttResult`
- `LocalTtsEngine` trait: `synthesize(text) -> (audio_data, mime_type)`
- `SttFallbackChain`: local-first with cloud fallback
  - Default confidence threshold: 0.60
  - On high confidence local: return local
  - On low confidence local: try cloud, pick higher confidence
  - On local error: cloud fallback
  - On both fail: propagate error
- `TtsFallbackChain`: local-first, cloud fallback on error
- `SttSource` / `TtsSource` enums for result attribution
- 10 tests covering all decision paths (mock-based)

#### 4. Transcription Logger (`crates/clawft-plugin/src/voice/transcript_log.rs`)
- `TranscriptEntry`: timestamp, speaker, text, source, confidence, language, duration_ms
- `TranscriptLogger`: append-only JSONL writer
  - Path: `{workspace}/.clawft/transcripts/{session_key}.jsonl`
  - `log()`, `read_all()`, `path()` methods
- 4 tests (serde roundtrip, optional fields, write+read, directory creation)

### VS3.3: Advanced Voice Features

#### 5. Voice Personality (`crates/clawft-types/src/config/personality.rs`)
- `VoicePersonality`: voice_id, provider, speed, pitch, greeting_prefix, language
- Default impl: voice_id="default", provider="local", speed=1.0, pitch=0.0, language="en"
- `validate()` method for range checks
- Added `personalities: HashMap<String, VoicePersonality>` to `VoiceConfig`
- 8 tests (default, serde roundtrip, defaults from partial JSON, validation)

#### 6. Voice Command Registry (`crates/clawft-plugin/src/voice/commands.rs`)
- `VoiceCommand`: triggers, tool, params, confirm, description
- `VoiceCommandRegistry`: indexed by lowercase triggers
  - `match_command()`: exact prefix match, then Levenshtein fuzzy (distance <= 2)
  - `with_defaults()`: stop listening, what time is it, list files
- `levenshtein_distance()`: standard DP implementation
- 14 tests (exact/fuzzy/no match, case insensitivity, Levenshtein edge cases)

#### 7. Audio Tools

**`crates/clawft-tools/src/audio_transcribe.rs`**
- `AudioTranscribeTool`: Tool impl for transcribing .wav/.mp3/.ogg/.webm files
- Parameters: file_path (required), language (optional)
- Feature-gated behind `voice`
- 5 tests (metadata, missing params, nonexistent file, wrong ext, stub result)

**`crates/clawft-tools/src/audio_synthesize.rs`**
- `AudioSynthesizeTool`: Tool impl for generating .wav audio from text
- Parameters: text (required), output_path (required), voice (optional), speed (optional)
- Feature-gated behind `voice`
- 7 tests (metadata, missing params, empty text, wrong ext, valid stub, bad dir)

## Module Registration

- `crates/clawft-plugin/src/voice/mod.rs`: Added cloud_stt, cloud_tts, commands, fallback, transcript_log
- `crates/clawft-types/src/config/mod.rs`: Added personality module with re-export
- `crates/clawft-tools/src/lib.rs`: Added audio_transcribe, audio_synthesize (voice-gated), registered in `register_all`

## Dependency Changes

- `crates/clawft-plugin/Cargo.toml`: Added `reqwest` with `json` and `multipart` features for cloud API calls

## Verification

- `cargo test -p clawft-plugin --features voice` -- 166 tests pass
- `cargo test -p clawft-types` -- 224 tests pass
- `cargo test -p clawft-tools --features voice` -- 171 tests pass (+ 33 integration + 1 doctest)
- `cargo check --features voice -p clawft-plugin` -- compiles
- `cargo build --release --bin weft` -- release binary compiles

## Test Summary

| Crate | Tests | Status |
|-------|-------|--------|
| clawft-plugin | 166 | All pass |
| clawft-types | 224 | All pass |
| clawft-tools | 205 | All pass |

New tests added: ~60 across all deliverables.
