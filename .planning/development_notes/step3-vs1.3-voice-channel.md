# VS1.3: VoiceChannel + Talk Mode

## Summary

Implemented VoiceChannel (ChannelAdapter trait), TalkModeController, WebSocket event types, and wired up the CLI `weft voice talk` command.

## Changes

### New Files

1. **`crates/clawft-plugin/src/voice/channel.rs`** -- VoiceChannel implementing ChannelAdapter
   - `VoiceStatus` enum (Idle, Listening, Transcribing, Processing, Speaking) with serde
   - `VoiceChannel` struct with mpsc status reporting
   - `start()` waits for cancellation (stub), transitions Listening -> Idle
   - `send()` logs TTS output (stub), transitions Speaking -> Listening
   - 11 unit tests covering trait methods, status lifecycle, serde roundtrips

2. **`crates/clawft-plugin/src/voice/talk_mode.rs`** -- TalkModeController
   - Wraps `Arc<VoiceChannel>` + CancellationToken
   - `run()` delegates to channel.start() -- blocks until cancel
   - `status()` returns current VoiceStatus
   - 2 unit tests (initial status, run-and-cancel lifecycle)

3. **`crates/clawft-plugin/src/voice/events.rs`** -- WebSocket event types
   - `VoiceWsEvent` struct with event type, status, timestamp
   - `VoiceWsEvent::new()` convenience constructor
   - 4 unit tests covering construction, serde roundtrip, JSON structure

### Modified Files

4. **`crates/clawft-plugin/Cargo.toml`** -- Added tokio as optional dep behind `voice` feature
5. **`crates/clawft-plugin/src/voice/mod.rs`** -- Registered channel, events, talk_mode modules with re-exports
6. **`crates/clawft-cli/src/commands/voice.rs`** -- Wired up Talk subcommand
   - Creates VoiceChannel + TalkModeController
   - Prints status changes in real time
   - Handles Ctrl+C for graceful shutdown
   - Uses StubAdapterHost (real agent integration deferred)

## Feature Gating

- All new modules are under `#[cfg(feature = "voice")]` (inherited from voice/mod.rs)
- tokio dependency is optional, only pulled in when `voice` feature is enabled
- CLI voice feature chain: `clawft-cli/voice -> clawft-plugin/voice`

## Test Results

- `cargo test -p clawft-plugin --features voice` -- 90 passed, 0 failed (17 new voice tests)
- `cargo test --workspace` -- all tests pass, 0 regressions
- `cargo build --release --bin weft` -- OK
- `cargo build --release --bin weft --features voice` -- OK

## Architecture Decisions

- Used `tokio::sync::mpsc` for status reporting (enables async WebSocket broadcast)
- VoiceChannel is `Send + Sync` via `Arc<Mutex<VoiceStatus>>` for interior mutability
- Status channel uses `try_send` (non-blocking) to avoid backpressure blocking the voice loop
- TalkModeController is a thin wrapper -- real orchestration logic will be added when audio processing is implemented
- StubAdapterHost in CLI keeps the talk command functional without full agent pipeline wiring

## What's Still Stub

- `start()` just waits for cancellation -- no real audio capture/VAD/STT
- `send()` logs text but doesn't actually synthesize/play audio
- CLI talk command shows status transitions but no real voice interaction
- Agent pipeline integration (deliver_inbound with real transcriptions) deferred
