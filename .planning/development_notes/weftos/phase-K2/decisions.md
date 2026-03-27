# Phase K2: A2A IPC + Topic Pub/Sub -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. A2ARouter uses per-agent mpsc inboxes
**Problem**: The spec describes using the existing MessageBus for delivery, but MessageBus is a broadcast channel with no per-agent filtering. Direct PID-to-PID delivery requires a different mechanism.

**Decision**: Created `A2ARouter` that manages per-agent inboxes as `mpsc::Sender<KernelMessage>` channels (bounded, 1024 capacity). The router creates inboxes at spawn time and removes them at cleanup.

**Rationale**: `mpsc` channels provide bounded, ordered, per-agent message delivery with backpressure. The 1024-message capacity is configurable per-agent (future work). This is a clean separation from the broadcast-oriented MessageBus.

### 2. TopicRouter is separate from A2ARouter
**Decision**: `TopicRouter` (topic.rs) manages subscriptions and subscriber lists. `A2ARouter` (a2a.rs) handles routing and delivery. The A2ARouter holds an `Arc<TopicRouter>` and delegates topic operations to it.

**Rationale**: Separation of concerns. TopicRouter handles subscription state; A2ARouter handles message delivery. They share the process table for state validation.

### 3. Lazy dead subscriber cleanup
**Decision**: `TopicRouter::live_subscribers()` checks each subscriber's state in the process table and removes dead ones (Exited state). `subscribers()` returns the raw list without cleanup.

**Rationale**: Eager cleanup (on process exit) would require the process table to notify the topic router, creating coupling. Lazy cleanup during publish is simpler and self-healing.

### 4. IPC message extensions (ToolCall, ToolResult, correlation_id)
**Decision**: Extended `MessagePayload` with `ToolCall { name, args }` and `ToolResult { call_id, result }` variants. Added `correlation_id: Option<String>` to `KernelMessage` (skip_serializing_if None).

**Rationale**: Tool delegation between agents requires structured call/result messages. The correlation_id enables request-response tracking without a separate tracking structure. JSON wire format stays backward-compatible (correlation_id is optional).

### 5. Topic variant added to MessageTarget
**Decision**: Added `MessageTarget::Topic(String)` variant. The A2ARouter handles topic routing by looking up subscribers via the TopicRouter.

**Rationale**: Topics are a first-class routing target alongside direct PID addressing. The router dispatches based on target type.

### 6. Cross-crate wiring deferred
**Decision**: Did NOT modify `clawft-services::delegation` or `clawft-services::mcp::server` as the spec suggests. These integrations require changes to files with many other pending modifications.

**Rationale**: The A2A protocol is fully functional within the kernel crate. Integration with delegation and MCP exposure can be done when those subsystems are stable. The IPC layer is designed to be consumable from outside the kernel crate via its public API.

## What Was Skipped

1. **Request-response with oneshot channels** -- The `PendingRequest` tracking was deferred. The A2ARouter provides the routing; callers can implement request-response patterns using the correlation_id field.
2. **DelegationEngine integration** -- Deferred to avoid modifying unstable files.
3. **MCP tool exposure** (ipc_send, ipc_subscribe) -- Deferred to avoid modifying unstable files.
4. **CLI `weft ipc` subcommand** -- Can be added later using the A2ARouter API.
5. **Ruvector integration** -- Feature-gated, not implemented.
6. **Per-agent rate limiting** -- Configurable in future; capacity is bounded by inbox channel size.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/a2a.rs` | ~330 | A2ARouter with inbox management and message routing |
| `crates/clawft-kernel/src/topic.rs` | ~280 | TopicRouter with pub/sub subscriptions and lazy cleanup |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/ipc.rs` | Added ToolCall/ToolResult payloads, correlation_id, Topic target, new constructors |
| `crates/clawft-kernel/src/lib.rs` | Added a2a and topic modules, new re-exports |

## Test Summary

- 12 new tests in a2a.rs (direct delivery, broadcast, topic pub/sub, IPC scope enforcement, inbox management)
- 11 new tests in topic.rs (subscribe, unsubscribe, list, dead subscriber cleanup, unsubscribe_all)
- 8 new tests in ipc.rs (correlation_id, ToolCall/ToolResult, Topic target, serde roundtrips)
- All 139 kernel tests pass
- All workspace tests pass
