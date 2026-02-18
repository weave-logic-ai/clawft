# Telegram Channel Plugin - Development Notes

## Summary

Implemented the Telegram channel plugin for clawft-channels. The plugin
connects to the Telegram Bot API using long polling and implements the
`Channel` trait for bidirectional message exchange.

## Files Created/Modified

### New files (5)

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-channels/src/telegram/mod.rs` | 21 | Module declarations and re-exports |
| `crates/clawft-channels/src/telegram/types.rs` | 303 | Telegram Bot API request/response types |
| `crates/clawft-channels/src/telegram/client.rs` | 194 | HTTP client wrapper (`TelegramClient`) |
| `crates/clawft-channels/src/telegram/channel.rs` | 347 | `TelegramChannel` impl + `TelegramChannelFactory` |
| `crates/clawft-channels/src/telegram/tests.rs` | 386 | Comprehensive test suite (27 tests) |

### Modified files (2)

| File | Change |
|------|--------|
| `crates/clawft-channels/src/lib.rs` | Added `pub mod telegram;` |
| `crates/clawft-channels/Cargo.toml` | Added `reqwest` and `chrono` to `[dependencies]`, removed `chrono` from `[dev-dependencies]` |

## Architecture Decisions

1. **No external Telegram SDK** -- The client is a thin wrapper around
   `reqwest` making direct HTTP calls to the Bot API. This keeps
   dependencies minimal and control maximal.

2. **Long polling** -- Uses `getUpdates` with a 30-second timeout. The
   polling loop respects the `CancellationToken` at every stage (poll,
   error backoff, inter-poll sleep).

3. **Error recovery** -- On `getUpdates` failure the channel sets its
   status to `Error`, waits 5 seconds, then retries. The offset is
   advanced past each update immediately so that a processing failure
   on one update does not block subsequent updates.

4. **`process_update` is `pub(crate)`** -- Elevated from private to
   `pub(crate)` so that the test module (in a sibling file) can call it
   directly without going through the full `start()` loop.

5. **Tests are in a separate file** -- Extracted to
   `telegram/tests.rs` to keep `channel.rs` at 347 lines (under the
   500-line limit). The test file contains a `MockHost` that captures
   delivered `InboundMessage` values.

## Test Coverage

29 new telegram-specific tests across 3 categories:

- **types.rs** (12 tests): Serde round-trips for all API types
- **client.rs** (2 tests): URL construction and custom base URL
- **tests.rs** (15 tests): `is_allowed`, metadata, status, send
  validation, factory build/error cases, `process_update` with various
  update shapes (text, no-text, no-from, disallowed user)

Total crate test count: 50 (21 pre-existing + 29 new).

## Dependencies Added

- `reqwest` (workspace) -- HTTP client for Telegram Bot API calls
- `chrono` (workspace, moved from dev-dependencies to dependencies) --
  timestamp on `InboundMessage`

## Limitations / Future Work

- No media attachment support yet (photos, documents). The `InboundMessage`
  `media` field is always empty; Telegram photo/document handling would
  need `getFile` API support.
- No webhook mode. Only long polling is implemented. Webhook support
  would require an HTTP server (likely via `axum` or `warp`).
- No rate limiting or message splitting for large outbound messages.
- `send_message` does not set `parse_mode`; HTML/Markdown formatting
  could be added as a config option.
