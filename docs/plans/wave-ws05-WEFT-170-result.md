# WEFT-170 — PluginHost C7: migrate Telegram/Discord/Slack to ChannelAdapter

**Status:** Done (incremental dual-impl; residual noted)  
**Branch / worktree:** `weft-170-channel-adapter`  
**Path:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb673-a49c-7ef0-b9a7-496eceab1c5f`  
**Base:** `release/0.8-staging` @ `4d8fa5d2`

## Acceptance

| Criterion | Result |
|-----------|--------|
| Telegram, Discord, Slack on `ChannelAdapter` | **Yes** — dual-impl with shared loops |
| Decision: keep or retire `ChannelAdapterShim` | **Keep (deprecated for first-party)** |
| Tests for adapters | **Yes** — capabilities, payload, cancel contract, inbound via `ChannelAdapterHost` |
| `scripts/build.sh test clawft-channels` | **PASS** — 227 tests, 0 failed |

## What changed

### Dual-impl pattern (incremental)

Each of Telegram / Discord / Slack now implements:

1. **`ChannelAdapter`** (primary C7 surface)  
   - `start(host: Arc<dyn ChannelAdapterHost>, cancel)`  
   - `send(target, &MessagePayload)` — text only  
   - Inbound via `MessagePayload::text` + metadata map  

2. **`Channel`** (legacy, kept for `PluginHost` / gateway)  
   - `start` wraps host with `ChannelAdapterHostBridge` and reuses the same loop  
   - `send(&OutboundMessage)` retains reply_to / thread_ts / chunking where applicable  

Shared internal loops:

| Channel | Loop method | Inbound entry |
|---------|-------------|----------------|
| Telegram | `run_poll_loop` | `process_update` → adapter host |
| Discord | `run_gateway_loop` | `process_message_create` → adapter host |
| Slack | `run_socket_loop` | `process_envelope` → adapter host |

### Cancellation contract

- On **native** (default for `clawft-channels`), `clawft_plugin::CancellationToken` **is** `tokio_util::sync::CancellationToken`.
- Migrated channels use `cancel.cancelled().await` in `tokio::select!` (same as before).
- No poll-based bridge on the first-party path.
- `ChannelAdapterShim` still contains a poll-bridge for non-native / legacy `Channel`-only plugins.

### `ChannelAdapterShim` decision

**Keep with `#[deprecated]` for first-party use.**

Rationale:

- Residual first-party: `web` (and any other `Channel`-only) still need the shim if exposed as `ChannelAdapter`.
- Third-party plugins may still ship `Channel` only.
- Retiring the shim now would force a hard cut on `web` + external adapters.

Deprecation note on the type; tests remain with `#[allow(deprecated)]`.

## Residual / follow-ups

1. **`PluginHost` still owns `Arc<dyn Channel>`** — full C7 simplification (host speaks `ChannelAdapter` only) is a later ticket.
2. **`register_command` migration** — still separate (acceptance called out as separate item).
3. **`web` channel** — still legacy `Channel` only; natural next consumer of the shim or a future dual-impl.
4. **Gateway** (`clawft-cli`) continues to register factories as `Channel`; no call-site change required for this ticket.

## Files touched

- `crates/clawft-channels/src/telegram/channel.rs`, `tests.rs`, `mod.rs`
- `crates/clawft-channels/src/discord/channel.rs`, `tests.rs`, `mod.rs`
- `crates/clawft-channels/src/slack/channel.rs`, `tests.rs`, `mod.rs`
- `crates/clawft-channels/src/plugin_host.rs` (shim deprecation + docs)

## How to test

```bash
cd /Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb673-a49c-7ef0-b9a7-496eceab1c5f
./scripts/build.sh test clawft-channels
# or:
cargo test -p clawft-channels --lib
```

## Merge note for lead

Worktree branch `weft-170-channel-adapter` is isolated from `release/0.8-staging`. Merge via PR or cherry-pick; do not commit to `master`.
