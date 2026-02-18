# Stream 1A: clawft-types Implementation Notes

## Date: 2026-02-16

## Summary

Implemented the `clawft-types` crate with all core type definitions, ported faithfully from the Python nanobot source.

## Files Created

| File | Lines | Description |
|------|-------|-------------|
| `crates/clawft-types/src/lib.rs` | 22 | Module declarations and re-exports |
| `crates/clawft-types/src/error.rs` | ~130 | `ClawftError` (9 variants) and `ChannelError` (7 variants) |
| `crates/clawft-types/src/config.rs` | ~680 | Full config schema (20+ structs) with serde aliases |
| `crates/clawft-types/src/event.rs` | ~95 | `InboundMessage` and `OutboundMessage` |
| `crates/clawft-types/src/provider.rs` | ~340 | LLM response types + 14-provider static registry |
| `crates/clawft-types/src/session.rs` | ~120 | `Session` with append-only message history |
| `crates/clawft-types/src/cron.rs` | ~220 | `CronJob`, `CronSchedule`, `CronPayload`, `CronStore` |

## Design Decisions

### 1. Error types are `#[non_exhaustive]`
Both `ClawftError` and `ChannelError` use `#[non_exhaustive]` so new variants can be added in future versions without breaking downstream match arms. A linter added the `ChannelError::NotFound` variant during implementation.

### 2. Config serde strategy
- All structs use `#[serde(default)]` on every optional field for backwards compatibility
- camelCase aliases via `#[serde(alias = "...")]` so both `max_tokens` and `maxTokens` work
- No `#[serde(deny_unknown_fields)]` anywhere -- unknown fields are silently ignored
- `ChannelsConfig` uses `#[serde(flatten)] pub extra: HashMap<String, serde_json::Value>` for unknown channel plugins

### 3. Provider registry uses `&'static str`
The `PROVIDERS` static array uses `&'static [ProviderSpec]` with `&'static str` fields instead of `String`, since all values are compile-time constants. This avoids heap allocation and is idiomatic for static data.

### 4. Session uses `serde_json::Value` for messages
Messages are stored as `Vec<serde_json::Value>` rather than a typed struct, matching the Python implementation where messages are arbitrary dicts with at minimum `role` and `content` fields. This preserves flexibility for tool_calls, metadata, etc.

### 5. ToolsConfig `exec` field rename
The `exec` field in `ToolsConfig` is renamed to `exec_tool` in Rust (since `exec` could collide with future reserved words or be confusing) but serializes as `"exec"` via `#[serde(rename = "exec")]`.

### 6. Added `dirs` dependency
The `Config::workspace_path()` method needs home directory expansion (`~/`), which requires the `dirs` crate (already in workspace deps).

## Python Source Files Ported

| Python Source | Rust Target |
|---------------|-------------|
| `nanobot/config/schema.py` (305 lines) | `config.rs` |
| `nanobot/bus/events.py` (38 lines) | `event.rs` |
| `nanobot/providers/registry.py` (396 lines) | `provider.rs` |
| `nanobot/session/manager.py` (180 lines) | `session.rs` |
| `nanobot/cron/types.py` (60 lines) | `cron.rs` |

## Quality Gates

| Check | Result |
|-------|--------|
| `cargo build -p clawft-types` | PASS |
| `cargo test -p clawft-types` | PASS (55 tests, 0 failures) |
| `cargo clippy -p clawft-types -- -D warnings` | PASS (0 warnings) |
| All public items have doc comments | YES |
| No unsafe blocks | YES |

## Test Coverage (55 tests)

- **error.rs** (6 tests): Display formatting, From impls, Result alias
- **config.rs** (12 tests): Fixture deserialization, camelCase aliases, defaults, roundtrip, unknown fields, extra channels, workspace path expansion, email/mochat/slack defaults, MCP server, provider headers
- **event.rs** (5 tests): Session key, serde roundtrip, optional field defaults
- **provider.rs** (14 tests): Registry count, find_by_model, find_gateway (3 strategies), find_by_name, labels, OAuth flag, LlmResponse/ContentBlock/StopReason/ToolCallRequest serde
- **session.rs** (7 tests): Constructor, add_message, extras, history, truncation, clear, roundtrip, default
- **cron.rs** (11 tests): Defaults, roundtrip, enum serde, missing field defaults, job state with error
