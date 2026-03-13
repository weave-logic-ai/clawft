# Step 5: VS2.2 Echo Cancellation + Quality / VS2.3 Platform Integration

**Date**: 2026-02-24
**Branch**: feature/three-workstream-implementation
**Phase**: VS2.2 + VS2.3

## Summary

Delivered three pure-computation audio processing modules (echo cancellation, noise
suppression, audio quality metrics) and platform integration support (systemd/launchd
service files, CLI install-service command, multi-language STT config verification).

## VS2.2: Echo Cancellation + Audio Quality

### New Files

| File | Purpose |
|------|---------|
| `crates/clawft-plugin/src/voice/echo.rs` | Echo canceller with circular reference buffer (stub passthrough) |
| `crates/clawft-plugin/src/voice/noise.rs` | Noise suppressor with EMA noise floor estimation (stub passthrough) |
| `crates/clawft-plugin/src/voice/quality.rs` | Audio quality metrics: RMS, peak, clipping detection, SNR estimation |

### Design Decisions

- **No feature gates** on echo/noise/quality modules. They are pure computation
  (no platform dependencies, no I/O) and compile on all targets including WASM.
- **Stub implementations** for AEC and noise suppression. Real implementations
  will use webrtc-audio-processing (AEC) and RNNoise (noise suppression) in a
  future phase when native audio capture is integrated.
- **EchoCanceller** uses a circular buffer sized to `tail_length_ms * 16kHz / 1000`
  samples. Feed reference audio from TTS output, then process mic input.
- **NoiseSuppressor** tracks noise floor via exponential moving average (alpha=0.05).
  Noise floor converges over ~200 frames.
- **AudioMetrics** provides frame-level analysis: RMS level, peak level, clipping
  detection (threshold 0.99), and SNR estimation clamped to [-20, 80] dB.

### Tests Added (18 new tests)

- `echo::tests` (6): new, process passthrough, feed_reference wrap-around, reset, disabled, config defaults
- `noise::tests` (6): new, process passthrough, noise floor convergence, disabled no-update, reset, config defaults
- `quality::tests` (6): silence, sine wave, clipping, SNR estimation, empty frame, SNR clamping

## VS2.3: Platform Integration

### New Files

| File | Purpose |
|------|---------|
| `scripts/clawft-wake.service` | systemd user unit for wake word daemon |
| `scripts/com.clawft.wake.plist` | launchd plist for macOS wake word daemon |

### Modified Files

| File | Change |
|------|--------|
| `crates/clawft-plugin/src/voice/mod.rs` | Register echo, noise, quality modules + re-exports |
| `crates/clawft-cli/src/commands/voice.rs` | Add `InstallService` subcommand with platform detection |

### CLI install-service Command

- **Auto-detection**: Uses `cfg!(target_os)` to detect Linux (systemd) or macOS (launchd)
- **Linux**: Copies service file to `~/.config/systemd/user/`, runs `systemctl --user enable`
- **macOS**: Copies plist to `~/Library/LaunchAgents/`, runs `launchctl load`
- **Windows**: Prints manual setup instructions (Task Scheduler)
- **WASM guard**: Entire install-service handler is `#[cfg(not(target_arch = "wasm32"))]`
- Uses `include_str!` to embed service files at compile time

### Multi-language STT

The `language` field already exists in `SttConfig` at
`crates/clawft-types/src/config/voice.rs:113` with empty string default
(auto-detect). No changes needed.

## Verification

- `cargo test --workspace`: 2525 tests passed, 0 failed
- `cargo test -p clawft-plugin --features voice`: 123 tests passed (18 new voice tests)
- `cargo build -p clawft-cli --features voice`: Compiles successfully
- `cargo build --release --bin weft`: Release binary builds successfully

## Module Registration

Updated `voice/mod.rs` re-exports:
```rust
pub use echo::{EchoCanceller, EchoCancellerConfig};
pub use noise::{NoiseSuppressor, NoiseSuppressorConfig};
pub use quality::{AudioMetrics, analyze_frame};
```
