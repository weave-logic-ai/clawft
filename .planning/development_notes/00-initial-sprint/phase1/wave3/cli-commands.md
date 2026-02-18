# CLI Commands Implementation -- Phase 1, Wave 3

## Summary

Implemented the three `weft` CLI subcommands (`agent`, `gateway`, `status`) in `clawft-cli`.

## Files Modified

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-cli/Cargo.toml` | 30 | Added `anyhow`, `async-trait`, `chrono`, `dirs` dependencies |
| `crates/clawft-cli/src/main.rs` | 127 | Clap-derived CLI with `agent`, `gateway`, `status` subcommands |
| `crates/clawft-cli/src/commands/mod.rs` | 154 | Shared helpers: `load_config`, `expand_workspace`, `BusChannelHost` bridge |
| `crates/clawft-cli/src/commands/agent.rs` | 132 | `weft agent` -- interactive REPL + single-message mode |
| `crates/clawft-cli/src/commands/gateway.rs` | 113 | `weft gateway` -- channel plugin lifecycle + Ctrl+C shutdown |
| `crates/clawft-cli/src/commands/status.rs` | 165 | `weft status` -- config discovery, agent/channel/provider diagnostics |

**Total**: ~721 lines across 6 files. All files under 500 lines individually.

## Architecture Decisions

### Config Loading (`commands/mod.rs`)

- `load_config()` accepts an optional path override; falls back to the platform config discovery chain (`CLAWFT_CONFIG` env var -> `~/.clawft/config.json` -> `~/.nanobot/config.json`).
- Uses `clawft_platform::config_loader::load_config_raw()` which handles camelCase-to-snake_case key normalization.
- Returns `Config::default()` when no config file is found (graceful degradation).

### BusChannelHost Bridge (`commands/mod.rs`)

- `PluginHost::new()` requires `Arc<dyn ChannelHost>`, not an `mpsc::UnboundedSender`.
- Created `BusChannelHost` struct that implements `ChannelHost` by forwarding `deliver_inbound()` calls to `MessageBus::publish_inbound()`.
- This bridges the channel plugin system to the core message bus without coupling them directly.

### Agent Command (`commands/agent.rs`)

- Two modes: `--message "..."` for one-shot, or interactive REPL.
- Interactive mode uses `tokio::io::BufReader::lines()` for async line reading.
- Supports `/exit`, `/quit`, `/clear`, `/help`, `/tools` commands.
- Initializes `ToolRegistry` with all built-in tools for the current workspace.
- Agent loop and LLM pipeline are placeholders pending integration phase.

### Gateway Command (`commands/gateway.rs`)

- Validates that at least one channel is enabled before starting.
- Currently registers Telegram only; future channels (Slack, Discord, etc.) follow the same pattern.
- Uses `tokio::signal::ctrl_c()` for graceful shutdown.
- Stops all channels on shutdown via `PluginHost::stop_all()`.

### Status Command (`commands/status.rs`)

- Displays config path discovery result.
- Shows agent defaults (model, workspace, max_tokens, temperature, etc.).
- `--detailed` flag adds channel status, provider key masking, tool config, and MCP server listing.
- API keys are masked (first 4 + last 4 chars) for security in terminal output.

## Test Coverage

24 unit tests covering:
- CLI argument parsing (clap derive validation)
- Subcommand structure
- Global `--verbose` flag
- Workspace path expansion
- API key masking
- Smoke tests for output functions

## Integration Points (Pending)

1. **Agent loop wiring**: `AgentLoop` needs `MessageBus`, `PipelineRegistry`, `ToolRegistry`, `ContextBuilder`, `SessionManager` connected.
2. **LLM transport**: Pipeline transport stage needs a real `clawft-llm` client.
3. **Additional channels**: Slack, Discord, WhatsApp factories need to be registered in gateway.
4. **Outbound message routing**: Gateway needs an outbound consumer task that sends `bus.consume_outbound()` messages back through `PluginHost::send_to_channel()`.

## Build Verification

- `cargo check -p clawft-cli` -- clean
- `cargo test -p clawft-cli` -- 24/24 pass
- `cargo clippy -p clawft-cli -- -D warnings` -- zero warnings
