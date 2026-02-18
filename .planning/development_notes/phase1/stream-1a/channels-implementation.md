# clawft-channels Implementation Notes

**Date**: 2026-02-16
**Phase**: 1 (Warp) - Foundation
**Stream**: 1A - Types/Platform/Plugin API
**Agent**: channel-architect
**Status**: COMPLETE

## Files Created

| File | LOC | Purpose |
|------|-----|---------|
| `crates/clawft-channels/src/lib.rs` | 36 | Module declarations, re-exports, crate-level docs |
| `crates/clawft-channels/src/traits.rs` | 197 | `Channel`, `ChannelHost`, `ChannelFactory` traits + data types |
| `crates/clawft-channels/src/host.rs` | 430 | `PluginHost` lifecycle manager + 10 async tests |

## Design Decisions

### 1. Re-use types from clawft-types
Rather than duplicating error and message types, the crate imports and re-exports:
- `clawft_types::error::ChannelError` -- re-exported as `clawft_channels::ChannelError`
- `clawft_types::event::{InboundMessage, OutboundMessage}` -- used in trait signatures

A `NotFound(String)` variant was added to `ChannelError` to support the "channel not found" case during routing.

### 2. SPARC-aligned Channel trait
The `Channel` trait follows the SPARC plan (1a-types-platform-plugin-api.md):
- `start()` receives `Arc<dyn ChannelHost>` + `CancellationToken` (not stored in constructor)
- `start()` is long-lived (runs until token is cancelled)
- `send()` takes `&OutboundMessage` by reference

This differs from the task description's simpler design (which stored host in the struct and defined a separate `ChannelMessage` type) but aligns with the canonical SPARC plan.

### 3. ChannelHost has three methods
- `deliver_inbound()` -- core: pushes an `InboundMessage` to the pipeline
- `register_command()` -- for slash-command registration
- `publish_inbound()` -- convenience wrapper that builds an `InboundMessage` from raw parts

### 4. PluginHost lifecycle
`PluginHost` manages:
- Factory registration (`register_factory`)
- Channel instantiation from config (`init_channel`)
- Per-channel tokio tasks with individual `CancellationToken`s (`start_channel`)
- Graceful shutdown (`stop_channel`, `stop_all`)
- Outbound message routing (`send_to_channel`)

### 5. tokio-util dependency
Added `tokio-util` to workspace and crate dependencies for `CancellationToken`, which provides cooperative cancellation without `select!` boilerplate.

## Changes to Other Crates

### clawft-types
- **`error.rs`**: Added `ChannelError::NotFound(String)` variant
- **`lib.rs`**: Added `pub mod error;` and `pub mod event;` declarations (was placeholder-only)
- **`Cargo.toml`**: Added `dirs` dependency (needed by config module added by concurrent agent)

### Workspace Cargo.toml
- Added `tokio-util = "0.7"` to `[workspace.dependencies]`

## Tests (17 total, all passing)

### traits.rs (6 tests)
- `channel_metadata_creation` -- field values
- `channel_metadata_clone` -- Clone impl
- `channel_status_equality` -- PartialEq for all variants
- `message_id_equality_and_hash` -- Eq + Hash in HashSet
- `command_creation` -- Command with parameters
- `command_no_parameters` -- empty params vec

### host.rs (11 tests)
- `register_factory` -- single factory
- `register_multiple_factories` -- two factories, sorted check
- `init_channel_with_registered_factory` -- happy path
- `init_channel_unknown_factory_errors` -- NotFound error
- `start_and_stop_channel` -- status transitions Running -> Stopped
- `start_all_stop_all` -- multi-channel lifecycle
- `send_to_unknown_channel_errors` -- NotFound on route
- `send_to_active_channel` -- successful message delivery
- `get_status_empty_host` -- empty map
- `start_nonexistent_channel_errors` -- NotFound
- `stop_nonexistent_channel_errors` -- NotFound

## Quality Gates

- [x] `cargo build -p clawft-channels` passes
- [x] `cargo test -p clawft-channels` -- 17/17 pass
- [x] `cargo clippy -p clawft-channels --all-targets -- -D warnings` -- zero warnings
- [x] All public items have doc comments
- [x] No unsafe blocks
- [x] No hardcoded secrets

## Integration Points

### For Stream 1C (Telegram plugin)
Import and implement:
```rust
use clawft_channels::{Channel, ChannelFactory, ChannelHost, ChannelMetadata, ChannelStatus, MessageId};
```

### For Stream 2A (Slack/Discord plugins)
Same trait surface. Plugins only need to implement `Channel` + `ChannelFactory`.

### For clawft-core (MessageBus)
The `ChannelHost` trait's `deliver_inbound()` will be backed by a `MessageBus` implementation. For now, the trait is abstract; the concrete `HostImpl` (that wraps a `MessageBus`) lives in the binary crate or clawft-core.
