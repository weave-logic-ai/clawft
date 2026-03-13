# VS1.2 -- STT + TTS Voice Tool Stubs

**Phase**: Voice Workstream Step 2 (VS1.2)
**Status**: Complete (stubs only)
**Date**: 2026-02-24

## Summary

Added voice tool stubs (`voice_listen`, `voice_speak`) to `clawft-tools` and
CLI voice command stubs (`weft voice setup|test-mic|test-speak|talk`) to
`clawft-cli`. Both are gated behind the `voice` feature flag.

## Changes

### clawft-tools

| File | Description |
|------|-------------|
| `Cargo.toml` | Added `voice` feature (`clawft-plugin/voice`), added `clawft-plugin` as optional dep |
| `src/lib.rs` | Registered `voice_listen` and `voice_speak` modules behind `#[cfg(feature = "voice")]`; registered tools in `register_all()` |
| `src/voice_listen.rs` | `VoiceListenTool` -- implements `Tool` trait; stub returns `status: "stub_not_implemented"` |
| `src/voice_speak.rs` | `VoiceSpeakTool` -- implements `Tool` trait; validates non-empty `text` param; stub returns `status: "stub_not_implemented"` |

### clawft-cli

| File | Description |
|------|-------------|
| `Cargo.toml` | Added `voice` feature (`clawft-tools/voice`, `dep:clawft-plugin`, `clawft-plugin/voice`) |
| `src/commands/mod.rs` | Registered `voice` module behind `#[cfg(feature = "voice")]` |
| `src/commands/voice.rs` | `VoiceArgs` / `VoiceCommand` with subcommands: `setup`, `test-mic`, `test-speak`, `talk` |
| `src/main.rs` | Added `Voice` variant to `Commands` enum and match handler, both gated behind `#[cfg(feature = "voice")]` |

## Tool Schemas

### voice_listen

```json
{
  "type": "object",
  "properties": {
    "timeout_seconds": { "type": "number", "default": 30 },
    "language": { "type": "string", "default": "" }
  },
  "required": []
}
```

Returns: `{ text, confidence, language, duration_ms, status }`

### voice_speak

```json
{
  "type": "object",
  "properties": {
    "text": { "type": "string" },
    "voice": { "type": "string", "default": "" },
    "speed": { "type": "number", "default": 1.0 }
  },
  "required": ["text"]
}
```

Returns: `{ spoken, text_length, duration_ms, status }`

## CLI Commands

```
weft voice setup        # Download STT/TTS/VAD models
weft voice test-mic     # Test microphone (--duration N)
weft voice test-speak   # Test speaker (--text "...")
weft voice talk         # Start Talk Mode loop
```

All commands print stub messages; real implementations depend on VS1.3.

## Testing

- 7 new unit tests added (3 for `voice_listen`, 4 for `voice_speak`)
- All 159 tests pass with `--features voice`
- All 152 tests pass without voice feature (default)
- `cargo check --workspace` passes
- `cargo check -p clawft-tools --features voice` passes
- `cargo check -p clawft-cli --features voice` passes

## Feature Flag Chain

```
clawft-cli/voice
  -> clawft-tools/voice
       -> clawft-plugin/voice
            -> clawft-plugin/voice-vad
  -> clawft-plugin/voice  (also directly)
```

## Next Steps (VS1.3)

- Wire `AudioCapture` + `VoiceActivityDetector` + `SpeechToText` into `VoiceListenTool::execute()`
- Wire `TextToSpeech` + `AudioPlayback` into `VoiceSpeakTool::execute()`
- Implement Talk Mode loop in `weft voice talk`
- Add `weft voice setup` model download logic
