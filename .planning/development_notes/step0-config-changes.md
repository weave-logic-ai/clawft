# Step 0 (A1+A3): Config Schema Changes for Three Workstreams

**Date**: 2026-02-24
**Branch**: `feature/three-workstream-implementation`
**Scope**: Single-pass config additions to prevent merge conflicts across workstreams

## Files Changed

### New File: `crates/clawft-types/src/config/voice.rs` (266 lines)

Created a dedicated module for voice pipeline configuration types:
- `VoiceConfig` -- top-level voice toggle + sub-configs
- `AudioConfig` -- sample rate, chunk size, channels, device selection
- `SttConfig` -- speech-to-text model, language, enable flag
- `TtsConfig` -- text-to-speech model, voice ID, speed
- `VadConfig` -- voice activity detection threshold and timeouts
- `WakeConfig` -- wake word phrase, sensitivity, custom model path
- `CloudFallbackConfig` -- cloud STT/TTS provider fallback settings

All types derive `Debug, Clone, Serialize, Deserialize` and implement `Default`.
Sub-configs reference `super::default_true` from the parent module.

### Modified: `crates/clawft-types/src/config/mod.rs`

**W-VOICE additions:**
- Added `pub mod voice;` and `pub use voice::*;`
- Added `voice: VoiceConfig` field to root `Config` struct with `#[serde(default)]`

**W-UI additions (GatewayConfig):**
- `api_port: u16` -- defaults to 18789, alias `apiPort`
- `cors_origins: Vec<String>` -- defaults to `["http://localhost:5173"]`, alias `corsOrigins`
- `api_enabled: bool` -- defaults to false, alias `apiEnabled`
- Updated `Default` impl for `GatewayConfig`

**W-BROWSER additions (ProviderConfig):**
- `browser_direct: bool` -- defaults to false, alias `browserDirect`
- `cors_proxy: Option<String>` -- defaults to None, alias `corsProxy`
- Added `baseUrl` alias to existing `api_base` field (A3 fix)

**Tests added (6 new):**
- `voice_config_defaults` -- full default verification for all VoiceConfig sub-structs
- `gateway_api_fields_defaults` -- verifies api_port, cors_origins, api_enabled
- `provider_browser_fields_defaults` -- verifies browser_direct, cors_proxy
- `provider_base_url_alias` -- verifies `baseUrl` JSON key maps to `api_base`
- `config_with_voice_section` -- round-trip with voice enabled
- `config_with_all_new_fields` -- comprehensive JSON with all three workstreams

## Design Decisions

1. **All fields use `#[serde(default)]`**: Existing config files parse without changes.
2. **Voice types in separate file**: Keeps production code in mod.rs under 500 lines (432 lines excluding tests). The voice module is 266 lines.
3. **camelCase aliases on all new fields**: Maintains consistency with existing codebase pattern (e.g., `apiPort`, `sampleRate`).
4. **No changes to existing fields or tests**: Only additive changes to prevent regressions.

## Verification

- `cargo test -p clawft-types` -- 175 tests pass (6 new)
- `cargo check --workspace` -- full workspace compiles cleanly
- Existing fixture `tests/fixtures/config.json` parses without modification
