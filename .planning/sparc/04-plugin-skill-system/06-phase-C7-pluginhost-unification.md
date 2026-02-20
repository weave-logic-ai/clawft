# Phase C7: PluginHost Unification

> Element 04 -- Plugin & Skill System
> Priority: P2 (Week 8)
> Dependencies: C1 (Plugin traits), C3 (SkillLoader)

---

## Overview

Migrate existing channels through the unified PluginHost with concurrent lifecycle management. Three objectives: (1) existing Telegram, Discord, Slack channels work through a `ChannelAdapter` compatibility layer without behavior changes, (2) `start_all()` and `stop_all()` execute concurrently instead of sequentially, and (3) SOUL.md personality content is injected into the Assembler pipeline stage.

---

## Current Code

### `clawft-channels/src/host.rs` (553 lines, 10 tokio tests)

`PluginHost` manages channel plugin lifecycle:

```rust
pub struct PluginHost {
    factories: RwLock<HashMap<String, Arc<dyn ChannelFactory>>>,
    channels: RwLock<HashMap<String, Arc<dyn Channel>>>,
    cancel_tokens: RwLock<HashMap<String, CancellationToken>>,
    task_handles: RwLock<HashMap<String, JoinHandle<()>>>,
    host_impl: Arc<dyn ChannelHost>,
}
```

**Sequential `start_all()`** (lines 89-100):
```rust
pub async fn start_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let channels = self.channels.read().await;
    let names: Vec<String> = channels.keys().cloned().collect();
    drop(channels);
    let mut results = Vec::with_capacity(names.len());
    for name in names {
        let result = self.start_channel(&name).await;  // Sequential!
        results.push((name, result));
    }
    results
}
```

**Sequential `stop_all()`** (lines 106-117):
```rust
pub async fn stop_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let tokens = self.cancel_tokens.read().await;
    let names: Vec<String> = tokens.keys().cloned().collect();
    drop(tokens);
    let mut results = Vec::with_capacity(names.len());
    for name in names {
        let result = self.stop_channel(&name).await;  // Sequential!
        results.push((name, result));
    }
    results
}
```

**`start_channel()`** (lines 123-155):
- Clones the channel `Arc`, creates a `CancellationToken`, spawns a tokio task
- Stores the cancel token and JoinHandle in separate RwLock maps
- The spawned task calls `channel.start(host, cancel)` which blocks until cancellation

**`stop_channel()`** (lines 160-178):
- Removes and cancels the token, then awaits the JoinHandle
- Uses `let-else` chains for clean error handling

### `clawft-channels/src/traits.rs` (247 lines, 7 tests)

Core trait definitions:

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn metadata(&self) -> ChannelMetadata;
    fn status(&self) -> ChannelStatus;
    fn is_allowed(&self, sender_id: &str) -> bool;
    async fn start(&self, host: Arc<dyn ChannelHost>, cancel: CancellationToken) -> Result<(), ChannelError>;
    async fn send(&self, msg: &OutboundMessage) -> Result<MessageId, ChannelError>;
}

#[async_trait]
pub trait ChannelHost: Send + Sync {
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError>;
    async fn register_command(&self, cmd: Command) -> Result<(), ChannelError>;
    async fn publish_inbound(&self, channel: &str, sender_id: &str, chat_id: &str, content: &str, media: Vec<String>, metadata: HashMap<String, Value>) -> Result<(), ChannelError>;
}

pub trait ChannelFactory: Send + Sync {
    fn channel_name(&self) -> &str;
    fn build(&self, config: &Value) -> Result<Arc<dyn Channel>, ChannelError>;
}
```

Supporting types: `ChannelMetadata`, `ChannelStatus` (enum: Stopped, Starting, Running, Error, Stopping), `MessageId`, `Command`, `CommandParameter`.

### `clawft-core/src/agent/context.rs` (965 lines, 18 tests)

`ContextBuilder` assembles LLM prompts. Already handles SOUL.md in `build_system_prompt()` (lines 105-200):

```rust
let bootstrap_files = ["SOUL.md", "IDENTITY.md", "AGENTS.md", "USER.md", "TOOLS.md"];
// ...searches workspace root and .clawft/ subdirectory
// If SOUL.md exists, uses it as identity preamble
if let Some(soul) = loaded_files.get("SOUL.md") {
    parts.push(format!("## SOUL.md\n\n{soul}"));
}
```

The `ContextBuilder` reads SOUL.md from the filesystem at prompt assembly time. The PluginHost needs to ensure SOUL.md content reaches channels that operate independently of the main agent REPL.

---

## Implementation Tasks

### C7-T1: ChannelAdapter Compatibility Layer

**Goal**: Existing `Channel` implementations work through the unified PluginHost without modification.

**Option A -- Wrapper shim** (recommended for zero-change migration):

Create a `ChannelAdapter` struct that wraps an existing `Channel`:

```rust
/// Adapts an existing Channel implementation to the unified plugin interface.
pub struct ChannelAdapter {
    inner: Arc<dyn Channel>,
}

impl ChannelAdapter {
    pub fn new(channel: Arc<dyn Channel>) -> Self {
        Self { inner: channel }
    }
}
```

If the C1 `PluginTrait` (from the plugin trait foundation phase) defines a different lifecycle interface, the adapter translates between them:

```rust
// Hypothetical C1 PluginTrait
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn plugin_type(&self) -> PluginType;
    async fn init(&self, config: &Value) -> Result<(), PluginError>;
    async fn start(&self, ctx: PluginContext) -> Result<(), PluginError>;
    async fn stop(&self) -> Result<(), PluginError>;
}

// ChannelAdapter implements Plugin by delegating to Channel
#[async_trait]
impl Plugin for ChannelAdapter {
    fn name(&self) -> &str { self.inner.name() }
    fn plugin_type(&self) -> PluginType { PluginType::Channel }
    async fn init(&self, _config: &Value) -> Result<(), PluginError> { Ok(()) }
    async fn start(&self, ctx: PluginContext) -> Result<(), PluginError> {
        self.inner.start(ctx.host, ctx.cancel).await
            .map_err(|e| PluginError::from(e))
    }
    async fn stop(&self) -> Result<(), PluginError> { Ok(()) } // Cancellation-based
}
```

**Option B -- Direct implementation**: Implement the unified trait directly on existing Telegram/Discord/Slack channels. This is cleaner long-term but requires modifying each channel.

**Recommendation**: Start with Option A for zero-risk migration, then migrate to Option B channel-by-channel in future phases.

### C7-T2: Concurrent `start_all()`

**File**: `clawft-channels/src/host.rs`, lines 89-100

Replace the sequential `for` loop with `futures::future::join_all`:

```rust
pub async fn start_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let channels = self.channels.read().await;
    let names: Vec<String> = channels.keys().cloned().collect();
    drop(channels);

    let futures: Vec<_> = names.into_iter().map(|name| {
        async move {
            let result = self.start_channel(&name).await;
            (name, result)
        }
    }).collect();

    futures::future::join_all(futures).await
}
```

**Borrow checker issue**: `self` is borrowed across `.await` points in the closure. Since `PluginHost` is not `Clone`, the closure needs an `&self` reference. Solutions:

1. **Use `Arc<Self>` pattern**: Change `start_all` to take `self: &Arc<Self>` -- requires callers to wrap in Arc
2. **Collect names + use indexed approach**: Spawn tasks that capture cloned data
3. **Use `FuturesUnordered` with explicit borrows**: Since all futures borrow `&self`, and `join_all` runs them on the same task, this should work as-is with Rust's borrow checker (the futures are polled sequentially on a single executor, but `start_channel` spawns the actual work into tokio tasks)

**Preferred approach** (minimal change):

```rust
pub async fn start_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let channels = self.channels.read().await;
    let names: Vec<String> = channels.keys().cloned().collect();
    drop(channels);

    // start_channel() spawns a tokio task internally, so calling it
    // concurrently just means all channels begin startup simultaneously.
    // The actual channel.start() runs in its own spawned task.
    let mut futs = Vec::with_capacity(names.len());
    for name in names {
        futs.push(async move {
            let result = self.start_channel(&name).await;
            (name, result)
        });
    }
    futures::future::join_all(futs).await
}
```

Note: `start_channel()` itself is fast -- it just spawns a task, creates a cancel token, and stores handles. The concurrent benefit is most visible when there are many channels and the RwLock acquisition is the bottleneck. The real win is in `stop_all()` where each channel may take time to shut down.

### C7-T3: Concurrent `stop_all()`

**File**: `clawft-channels/src/host.rs`, lines 106-117

This is where concurrency matters most. Each `stop_channel()` cancels a token then **awaits** the JoinHandle, which may block while the channel gracefully shuts down.

```rust
pub async fn stop_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    // Phase 1: Cancel all tokens simultaneously
    let mut tokens_to_cancel = Vec::new();
    {
        let mut tokens = self.cancel_tokens.write().await;
        let names: Vec<String> = tokens.keys().cloned().collect();
        for name in &names {
            if let Some(token) = tokens.remove(name) {
                token.cancel();
                tokens_to_cancel.push(name.clone());
            }
        }
    }

    // Phase 2: Await all handles concurrently
    let mut handles_to_await = Vec::new();
    {
        let mut handles = self.task_handles.write().await;
        for name in &tokens_to_cancel {
            if let Some(handle) = handles.remove(name) {
                handles_to_await.push((name.clone(), handle));
            }
        }
    }

    let futs: Vec<_> = handles_to_await.into_iter().map(|(name, handle)| {
        async move {
            match handle.await {
                Ok(()) => (name, Ok(())),
                Err(e) => {
                    tracing::warn!(channel = %name, error = %e, "channel task panicked");
                    (name, Ok(())) // Task panicked but we still report success for the stop
                }
            }
        }
    }).collect();

    futures::future::join_all(futs).await
}
```

**Key design decision**: Two-phase shutdown. Phase 1 cancels all tokens in one pass (fast, single lock). Phase 2 awaits all handles concurrently (may be slow, no locks held). This avoids holding RwLock across await points.

### C7-T4: SOUL.md Injection into Assembler Pipeline

**Context**: `ContextBuilder` already reads SOUL.md at prompt assembly time (lines 111-145 of `context.rs`). For channels (Telegram, Discord, Slack), the pipeline is invoked when an inbound message arrives. The question is: does SOUL.md already flow through the pipeline for channel-originated messages?

**Analysis**:
- `ContextBuilder::build_system_prompt()` reads SOUL.md from the workspace filesystem
- The `AgentLoop` (in `loop_core.rs`) uses `ContextBuilder` to assemble prompts
- Channel messages arrive via `MessageBus` -> `AgentLoop` -> `ContextBuilder`
- Therefore SOUL.md is **already injected** for any message flowing through the standard pipeline

**What's needed**: Ensure the PluginHost passes the workspace path to channels so that if a channel needs to reference SOUL.md directly (e.g., for a custom greeting or personality display), it can:

1. Add `workspace_path: Option<PathBuf>` to the `ChannelHost` implementation or make it available through a config accessor
2. Channels that want raw SOUL.md content can read it via the host
3. The primary injection path (ContextBuilder) already works -- this is a secondary access path

**Implementation** (if needed):
```rust
// In the ChannelHost implementation
impl ChannelHost for HostImpl {
    // Existing methods...

    // Optional: expose workspace config
    fn workspace_path(&self) -> Option<&Path> {
        self.config.workspace_path.as_deref()
    }
}
```

**Risk**: Adding methods to `ChannelHost` is a breaking change for existing implementations. Consider a separate `ChannelConfig` trait or pass workspace info through `Channel::start()` metadata instead.

---

## Concurrency Plan

### Lock Ordering (to prevent deadlocks)

The `PluginHost` has four RwLock-protected maps. Establish a strict acquisition order:

1. `factories` (read-only after init, rarely written)
2. `channels` (read for lookups, written during init)
3. `cancel_tokens` (written during start/stop)
4. `task_handles` (written during start/stop)

**Rule**: Never hold a write lock on a lower-numbered map while acquiring a higher-numbered one.

### Concurrent Start Sequence

```
start_all()
  |-- read channels (lock 2, read)
  |-- drop lock
  |-- for each channel:
  |     |-- start_channel()
  |     |     |-- read channels (lock 2, read) -> get Arc<dyn Channel>
  |     |     |-- drop lock
  |     |     |-- tokio::spawn() -> JoinHandle
  |     |     |-- write cancel_tokens (lock 3)
  |     |     |-- write task_handles (lock 4)
  |-- join_all futures
```

With `join_all`, multiple `start_channel()` calls may contend on locks 3 and 4. Since each writes a different key, contention is brief.

### Concurrent Stop Sequence

```
stop_all()
  |-- Phase 1: Cancel tokens
  |     |-- write cancel_tokens (lock 3) -> remove all, cancel
  |-- Phase 2: Await handles
  |     |-- write task_handles (lock 4) -> remove all
  |     |-- join_all(handle.await) -> no locks held
```

Phase 2 holds no locks while awaiting JoinHandles, which is critical since channel shutdown may take arbitrary time.

---

## Dependencies

| Dependency | Reason | Status |
|-----------|--------|--------|
| C1 (Plugin traits) | `ChannelAdapter` bridges to unified trait | In progress |
| C3 (SkillLoader) | Needed for skill-aware channel configuration | In progress |
| `futures` crate | `join_all` for concurrent operations | Already in Cargo.toml |
| `tokio-util` | `CancellationToken` (already used) | Already in Cargo.toml |

---

## Tests Required

### Existing Tests (must pass unchanged)

All 10 tokio tests in `host.rs:212-553`:

| Test | Lines | Description |
|------|-------|-------------|
| `register_factory` | 357-366 | Single factory registration |
| `register_multiple_factories` | 369-383 | Two factories, sorted names |
| `init_channel_with_registered_factory` | 386-400 | Init from factory config |
| `init_channel_unknown_factory_errors` | 403-416 | Unknown factory -> NotFound |
| `start_and_stop_channel` | 419-442 | Full lifecycle: start -> Running -> stop -> Stopped |
| `start_all_stop_all` | 445-474 | Two channels, start all, verify Running, stop all, verify Stopped |
| `send_to_unknown_channel_errors` | 477-493 | Send to nonexistent -> NotFound |
| `send_to_active_channel` | 496-525 | Send to running channel -> MessageId |
| `get_status_empty_host` | 528-534 | Empty host returns empty map |
| `start_nonexistent_channel_errors` | 537-543 | Start unknown -> NotFound |
| `stop_nonexistent_channel_errors` | 546-552 | Stop unknown -> NotFound |

### New Tests

| Test | Description |
|------|-------------|
| `start_all_concurrent_timing` | Verify N channels start in roughly O(1) time, not O(N) |
| `stop_all_concurrent_timing` | Verify N channels stop in roughly O(1) time |
| `stop_all_two_phase` | Verify all tokens are cancelled before any handle is awaited |
| `channel_adapter_wraps_existing` | `ChannelAdapter` delegates all methods to inner Channel |
| `channel_adapter_preserves_metadata` | Metadata passthrough is correct |
| `channel_adapter_start_stop_lifecycle` | Full lifecycle through adapter |
| `start_all_partial_failure` | One channel fails to start, others succeed |
| `stop_all_with_panicked_task` | One channel task panics, stop_all still completes for others |
| `concurrent_start_lock_contention` | Many channels (10+) start concurrently without deadlock |

### Timing Test Pattern

```rust
#[tokio::test]
async fn start_all_concurrent_timing() {
    let host = Arc::new(MockChannelHost::new());
    let plugin_host = PluginHost::new(host);

    // Register 5 channels with a slow-starting mock
    for i in 0..5 {
        let name = format!("chan-{i}");
        plugin_host.register_factory(Arc::new(SlowStartFactory::new(&name, Duration::from_millis(100)))).await;
        plugin_host.init_channel(&name, &json!({})).await.unwrap();
    }

    let start = Instant::now();
    let results = plugin_host.start_all().await;
    let elapsed = start.elapsed();

    assert!(results.iter().all(|(_, r)| r.is_ok()));
    // Concurrent: should take ~100ms, not ~500ms
    assert!(elapsed < Duration::from_millis(300), "start_all took {elapsed:?}, expected concurrent execution");
}
```

---

## Acceptance Criteria

- [ ] Existing Telegram, Discord, Slack channels work through unified PluginHost without behavior changes
- [ ] `ChannelAdapter` bridges existing `Channel` implementations to unified interface
- [ ] `start_all()` executes concurrently (verified by timing test)
- [ ] `stop_all()` executes concurrently with two-phase shutdown
- [ ] All tokens cancelled before any JoinHandle is awaited in `stop_all()`
- [ ] SOUL.md content is injected into Assembler pipeline for channel-originated messages (already works via `ContextBuilder`)
- [ ] SOUL.md accessible to channels that need direct personality reference
- [ ] No deadlocks under concurrent start/stop operations
- [ ] All 10 existing `host.rs` tests pass unchanged
- [ ] All 7 existing `traits.rs` tests pass unchanged
- [ ] Partial failure in `start_all()` does not prevent other channels from starting
- [ ] Panicked channel task in `stop_all()` does not prevent other channels from stopping

---

## Risk Notes

1. **Borrow checker with `join_all`**: The `&self` borrow in async closures within `start_all()` may not compile directly with `join_all`. Fallback: use `FuturesUnordered` or restructure to avoid self-borrows (extract all needed data before the concurrent section).

2. **Lock contention during concurrent start**: Multiple concurrent `start_channel()` calls will contend on `cancel_tokens` and `task_handles` write locks. Each write is very fast (HashMap insert), so this is acceptable. If profiling shows issues, batch the writes into a single lock acquisition.

3. **`stop_channel()` refactoring**: The current `stop_channel()` implementation removes from `cancel_tokens` and `task_handles` sequentially. The two-phase `stop_all()` operates differently (batch remove). Ensure `stop_channel()` still works correctly for single-channel stops.

4. **`ChannelAdapter` lifetime**: The adapter holds `Arc<dyn Channel>`. If the unified plugin system expects owned values or different lifetime semantics, the adapter may need adjustment.

5. **SOUL.md injection scope**: SOUL.md already flows through `ContextBuilder` for all pipeline-processed messages. The only risk is if a channel bypasses the pipeline and needs direct SOUL.md access. Document this path clearly.

6. **futures crate dependency**: Verify `futures` is already in `Cargo.toml` for the `clawft-channels` crate. If not, add `futures = "0.3"` as a dependency.
