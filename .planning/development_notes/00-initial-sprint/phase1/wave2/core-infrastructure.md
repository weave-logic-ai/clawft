# Core Infrastructure Modules - Development Notes

## What Was Implemented

Three core infrastructure modules in `clawft-core`:

### 1. `crates/clawft-core/src/bus.rs` -- MessageBus (129 impl lines)

Thread-safe message bus using tokio unbounded MPSC channels for routing
inbound messages (from channels) and outbound messages (from the agent
pipeline). Ported from Python `nanobot/bus/queue.py`.

**Public API:**
- `MessageBus::new()` / `Default` -- create bus with fresh channels
- `publish_inbound(msg)` -- publish from channel adapter, returns error if closed
- `consume_inbound()` -- async recv, returns None if closed
- `dispatch_outbound(msg)` -- publish from agent pipeline
- `consume_outbound()` -- async recv for channel adapters
- `inbound_sender()` / `outbound_sender()` -- cloneable sender handles

### 2. `crates/clawft-core/src/session.rs` -- SessionManager (323 impl lines)

Conversation session manager with in-memory caching and JSONL persistence.
All filesystem I/O goes through `Platform::fs()` for WASM portability.
Ported from Python `nanobot/session/manager.py`.

**Public API:**
- `SessionManager::new(platform)` -- discover sessions dir with fallback
- `SessionManager::with_dir(platform, dir)` -- explicit dir (useful for tests)
- `get_or_create(key)` -- cache-first, then disk, then new
- `load_session(key)` -- parse JSONL from disk
- `save_session(session)` -- write full JSONL + update cache
- `append_turn(key, role, content)` -- append single turn to file and cache
- `list_sessions()` -- enumerate `.jsonl` files in sessions dir
- `invalidate(key)` -- evict from cache (does not delete file)

### 3. `crates/clawft-core/src/tools/registry.rs` -- Tool + ToolRegistry (217 impl lines)

Tool trait definition and registry for the agent pipeline. The `Tool` trait
is the contract that tool implementations in `clawft-tools` will satisfy.

**Public API:**
- `ToolError` enum -- 7 variants (NotFound, InvalidArgs, ExecutionFailed, PermissionDenied, FileNotFound, InvalidPath, Timeout)
- `Tool` trait -- `name()`, `description()`, `parameters()`, `execute(args)`
- `ToolRegistry::new()` / `Default` -- empty registry
- `register(tool)` -- add or replace by name
- `get(name)` -- lookup by name
- `list()` -- sorted tool names
- `schemas()` -- OpenAI function calling format
- `execute(name, args)` -- dispatch by name
- `len()` / `is_empty()` -- cardinality

## Key Design Decisions

1. **Unbounded channels for MessageBus**: Chosen to match Python behavior (no drops under burst). Bounded channels would require backpressure handling that the Python codebase does not have.

2. **`tokio::sync::Mutex` for receivers**: Since only one consumer should read from each channel at a time, the mutex guards single-consumer access while allowing the bus struct itself to be shared (`Arc<MessageBus>`).

3. **Platform abstraction in SessionManager**: All I/O goes through `Platform::fs()` so the session manager works in WASM environments with mock filesystems.

4. **Sessions dir fallback**: `~/.clawft/workspace/sessions/` is checked first, then `~/.nanobot/workspace/sessions/`. If neither exists, the clawft path is created.

5. **Session key sanitization**: Colons in session keys (e.g. `telegram:123`) are replaced with underscores for filenames. The reverse mapping (`_` to `:`) is applied when listing sessions.

6. **JSONL format**: First line is metadata (with `_type: "metadata"`), subsequent lines are individual message objects. This matches the Python nanobot format.

7. **ToolError is separate from ClawftError**: Tool errors have their own enum to keep them focused on tool-specific failure modes. They can be converted to ClawftError at the agent loop boundary if needed.

8. **Sorted outputs**: `ToolRegistry::list()` and `schemas()` return sorted results for deterministic behavior in tests and API responses.

9. **`with_dir` constructor for SessionManager**: Added alongside `new()` to enable test isolation without needing to mock the full directory discovery logic.

## Test Count

| Module | Sync Tests | Async Tests | Total |
|--------|-----------|-------------|-------|
| bus.rs | 2 | 10 | 12 |
| session.rs | 0 | 12 | 12 |
| registry.rs | 8 | 5 | 13 |
| **Total** | **10** | **27** | **37** |

## Deviations from Spec

1. **Added `with_dir` constructor to SessionManager**: Not in the original spec but essential for test isolation (avoids needing to mock home directory discovery in every test).

2. **Added `len()` and `is_empty()` to ToolRegistry**: Standard Rust container methods that enable idiomatic usage.

3. **Session mock platform in tests**: Built a complete mock platform (MockFs, MockEnv, MockHttp, MockPlatform) in the test module to verify session I/O without touching the real filesystem.

4. **`list_sessions` returns sorted keys**: Added sorting for deterministic output; not explicitly specified but follows Rust conventions.
