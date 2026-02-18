# Stream 2A: Slack + Discord Channel Plugins -- Development Notes

**Agent**: channel-engineer (coder)
**Date**: 2026-02-17
**Phase**: 2 / Stream 2A
**Crate**: `clawft-channels`
**Modules**: `slack/`, `discord/`

## Summary

Implemented Slack Socket Mode and Discord Gateway v10 WebSocket channel plugins in `clawft-channels`. Both follow the existing `Channel`/`ChannelHost`/`ChannelFactory` trait system from Phase 1's Telegram plugin. Each plugin has full event deserialization, API client, WebSocket lifecycle, and factory wiring.

## Files Written

| File | Lines | Purpose |
|------|-------|---------|
| `slack/mod.rs` | 25 | Module declarations and re-exports |
| `slack/events.rs` | 392 | Slack Socket Mode event types, envelope deserialization |
| `slack/signature.rs` | 230 | HMAC-SHA256 signature verification with 5-min anti-replay |
| `slack/api.rs` | 203 | `SlackApiClient` -- `apps.connections.open`, `chat.postMessage` |
| `slack/channel.rs` | 376 | `SlackChannel` -- Socket Mode WebSocket lifecycle, message routing |
| `slack/factory.rs` | 165 | `SlackChannelFactory` -- config validation, channel construction |
| `slack/tests.rs` | 432 | 49 tests for Slack module |
| `discord/mod.rs` | 23 | Module declarations and re-exports |
| `discord/events.rs` | 492 | Gateway v10 event types, opcodes, `RateLimitInfo` |
| `discord/api.rs` | 197 | `DiscordApiClient` -- REST endpoints, rate limit header parsing |
| `discord/channel.rs` | 455 | `DiscordChannel` -- Gateway WebSocket, heartbeat, resume |
| `discord/factory.rs` | 182 | `DiscordChannelFactory` -- config validation, intent filtering |
| `discord/tests.rs` | 273 | 47 tests for Discord module |

**Total new lines**: ~3,445

## Architecture Decisions

### Slack Socket Mode over Events API
The plan specified Socket Mode (WebSocket) rather than the HTTP Events API.
Socket Mode is preferable for self-hosted bots because it requires no public
HTTP endpoint. The `apps.connections.open` API returns a WebSocket URL, and
the channel maintains the connection with automatic reconnection.

### HMAC-SHA256 with anti-replay
Slack HTTP signature verification uses `v0:{timestamp}:{body}` as the HMAC
base string. The implementation includes a 5-minute anti-replay window
(`MAX_TIMESTAMP_AGE_SECS = 300`) to reject stale requests, matching Slack's
official recommendation. This is used for any HTTP webhook fallback paths.

### DM/group policy filtering
`SlackChannel` respects the `SlackConfig.dm_policy` field from
`clawft-types::config`. Messages are filtered at the channel level before
being forwarded to the message bus, so the agent loop never sees messages
that violate the configured policy.

### Discord Gateway v10
Uses the latest stable Discord Gateway version (v10). The implementation
handles the full lifecycle: Hello (opcode 10) -> Identify (opcode 2) ->
Heartbeat loop -> Dispatch events -> Resume on disconnect. Session resume
uses the `resume_gateway_url` from the Ready event when available.

### Rate limit header parsing
`DiscordApiClient` parses `X-RateLimit-Remaining`, `X-RateLimit-Reset`,
`X-RateLimit-Bucket`, and `Retry-After` headers from Discord REST responses.
`RateLimitInfo::is_limited()` returns true when remaining = 0, allowing
callers to back off proactively.

### No real WebSocket connections in tests
All tests use mock data and event deserialization. The WebSocket connection
logic is tested through structure verification (correct URLs, headers,
payloads) rather than live connections, keeping tests fast and offline.

## Dependencies Added

- `tokio-tungstenite` (workspace): WebSocket client for Socket Mode / Gateway
- `futures-util` (workspace): `StreamExt`/`SinkExt` for WebSocket message handling
- `hmac` (workspace): HMAC computation for Slack signature verification
- `sha2` (workspace): SHA-256 digest for Slack signature verification

## Test Coverage (96 tests)

- **slack/events.rs**: Envelope deserialization, event type variants, acknowledgment
- **slack/signature.rs**: Valid/invalid signatures, anti-replay, edge cases
- **slack/api.rs**: URL construction, default/custom base URLs
- **slack/channel.rs**: Config validation, DM policy filtering, message routing
- **slack/factory.rs**: Factory name, build success/failure, camelCase aliases
- **discord/events.rs**: All opcodes, Ready, MessageCreate, Hello, rate limit parsing
- **discord/api.rs**: URL construction, custom base URL
- **discord/channel.rs**: Config validation, intent computation, message routing
- **discord/factory.rs**: Factory name, build success/failure, empty config errors

## Quality Gates

| Check | Result |
|-------|--------|
| `cargo build -p clawft-channels` | PASS |
| `cargo test -p clawft-channels` | PASS (152 tests, 0 failures) |
| `cargo clippy -p clawft-channels -- -D warnings` | PASS (0 warnings) |

## Integration Points

- **clawft-types::config**: `SlackConfig` (bot_token, app_token, signing_secret, dm_policy) and `DiscordConfig` (bot_token, guild_id, allowed_channels)
- **clawft-channels::traits**: Both implement `Channel`, `ChannelHost`, and `ChannelFactory`
- **clawft-core::bus::MessageBus**: Channels send `InboundMessage` via the bus inbound sender
- **clawft-cli**: `weft channels status` discovers both via `ChannelFactory::name()`

## Next Steps

1. Wire Slack/Discord factories into the CLI's `PluginHost`
2. Add integration tests with mock WebSocket servers
3. Implement Slack interactive components (buttons, modals)
4. Add Discord slash command registration
