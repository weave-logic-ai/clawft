# Step 4: VS2.1 - Wake Word Detection

## Summary

Added wake word detection infrastructure with stub implementations, following the same pattern as VS1.x (stubs first, real audio later).

## Files Created

### `crates/clawft-plugin/src/voice/wake.rs`
- `WakeWordConfig` - Serializable configuration with model path, threshold, min gap frames, sample rate, log toggle
- `WakeWordEvent` - Tagged enum with `Detected`, `Started`, `Stopped`, `Error` variants (serde-serializable)
- `WakeWordDetector` - Stub detector with `new()`, `process_frame()`, `start()`, `stop()`, `is_running()`, `config()` API
- 11 unit tests covering config defaults, serde roundtrip, custom values, detector lifecycle, process_frame stub, event serialization

### `crates/clawft-plugin/src/voice/wake_daemon.rs`
- `WakeDaemon` - Background daemon wrapping `WakeWordDetector`
- `new()` creates daemon, `run()` starts detection loop (stub waits for cancellation), `is_active()` and `detector()` accessors
- `run()` gated behind `#[cfg(not(target_arch = "wasm32"))]` for tokio dependency
- 3 unit tests covering creation, detector access, run-and-cancel lifecycle

## Files Modified

### `crates/clawft-plugin/src/voice/mod.rs`
- Added `wake` and `wake_daemon` module declarations gated behind `#[cfg(feature = "voice-wake")]`
- Added re-exports: `WakeWordConfig`, `WakeWordDetector`, `WakeWordEvent`, `WakeDaemon`

### `crates/clawft-plugin/Cargo.toml`
- Added `voice-wake` to the `voice` feature (so it compiles with `--features voice`)

### `crates/clawft-cli/src/commands/voice.rs`
- Added `Wake` variant to `VoiceCommand` enum
- Added `handle_wake()` async function that creates a `WakeDaemon`, prints stub status, and waits for Ctrl+C
- Updated module doc comment

## Design Decisions

1. **Stub-only**: No real rustpotter dependency added. `process_frame()` always returns `false`.
2. **Feature gating**: All wake word code gated behind `voice-wake` feature, included in the `voice` umbrella feature.
3. **WASM safety**: `WakeDaemon::run()` gated behind `#[cfg(not(target_arch = "wasm32"))]` to avoid tokio on WASM targets.
4. **Config already existed**: `WakeConfig` in `clawft-types/src/config/voice.rs` already had all needed fields from Step 0 setup.
5. **CancellationToken pattern**: Reused the same `tokio_util::CancellationToken` (native) / `AtomicBool` (WASM) pattern as `VoiceChannel` and `TalkModeController`.

## Verification

- `cargo test -p clawft-plugin --features voice` -- 105 tests passed (14 new wake word tests)
- `cargo test --workspace` -- all tests passed, zero regressions
- `cargo build --release --bin weft` -- native build OK
