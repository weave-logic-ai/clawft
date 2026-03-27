# Phase K2: Agent-to-Agent IPC

**Phase ID**: K2
**Workstream**: W-KERNEL
**Duration**: Week 5-6
**Goal**: Implement agent-to-agent messaging with typed message envelopes, pub/sub topic routing, and IPC scope enforcement

---

## S -- Specification

### What Changes

This phase extends the kernel's IPC subsystem (built on the existing `MessageBus`) with direct agent-to-agent messaging using PIDs, pub/sub topic routing with wildcard support, and IPC scope enforcement via the capability system. Messages use a JSON-RPC-inspired wire format for structured tool call delegation between agents.

### Files to Create

| File | Purpose |
|---|---|
| `crates/clawft-kernel/src/a2a.rs` | `A2AProtocol` -- agent-to-agent message routing and delivery |
| `crates/clawft-kernel/src/topic.rs` | `TopicRouter` -- pub/sub topic management with subscriptions |

### Files to Modify

| File | Change |
|---|---|
| `crates/clawft-kernel/src/ipc.rs` | Extend `KernelIpc` with A2A routing, topic dispatch, scope enforcement |
| `crates/clawft-kernel/src/lib.rs` | Re-export A2A and topic types |
| `crates/clawft-services/src/delegation/mod.rs` | Wire `DelegationEngine` to use A2A protocol for inter-agent tool delegation |
| `crates/clawft-services/src/mcp/server.rs` | Expose A2A messaging as MCP tool (`ipc_send`, `ipc_subscribe`) |

### Key Types

**A2AProtocol** (`a2a.rs`):
```rust
pub struct A2AProtocol {
    ipc: Arc<KernelIpc>,
    process_table: Arc<ProcessTable>,
    capability_checker: Arc<CapabilityChecker>,
    pending_requests: DashMap<String, PendingRequest>,
}

pub struct PendingRequest {
    pub request_id: String,
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub sent_at: DateTime<Utc>,
    pub timeout: Duration,
    pub response_tx: oneshot::Sender<KernelMessage>,
}

impl A2AProtocol {
    pub async fn send(&self, from: Pid, msg: KernelMessage) -> Result<()>;
    pub async fn request(&self, from: Pid, msg: KernelMessage, timeout: Duration) -> Result<KernelMessage>;
    pub async fn reply(&self, to_request_id: &str, response: MessagePayload) -> Result<()>;
    pub fn inbox(&self, pid: Pid) -> mpsc::Receiver<KernelMessage>;
}
```

**TopicRouter** (`topic.rs`):
```rust
pub struct TopicRouter {
    subscriptions: DashMap<String, Vec<Pid>>,
    capability_checker: Arc<CapabilityChecker>,
}

pub struct Subscription {
    pub topic: String,
    pub subscriber_pid: Pid,
    pub filter: Option<String>,
}

impl TopicRouter {
    pub fn subscribe(&self, pid: Pid, topic: &str) -> Result<()>;
    pub fn unsubscribe(&self, pid: Pid, topic: &str) -> Result<()>;
    pub async fn publish(&self, from: Pid, topic: &str, payload: MessagePayload) -> Result<u32>;
    pub fn list_topics(&self) -> Vec<(String, usize)>;
    pub fn subscribers(&self, topic: &str) -> Vec<Pid>;
    pub fn topics_for_pid(&self, pid: Pid) -> Vec<String>;
}
```

**KernelMessage** (extended in `ipc.rs`):
```rust
pub struct KernelMessage {
    pub id: String,
    pub from: Pid,
    pub to: MessageTarget,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
}

pub enum MessageTarget {
    Pid(Pid),
    Topic(String),
    Broadcast,
    Service(String),
}

pub enum MessagePayload {
    Text(String),
    Json(serde_json::Value),
    ToolCall { name: String, args: serde_json::Value },
    ToolResult { call_id: String, result: serde_json::Value },
    Signal(ProcessSignal),
}

pub enum ProcessSignal {
    Ping,
    Pong,
    Shutdown,
    Suspend,
    Resume,
}
```

### Wire Format (JSON-RPC inspired)

```json
{
  "id": "msg-uuid",
  "from": 42,
  "to": { "pid": 7 },
  "payload": {
    "type": "tool_call",
    "name": "read_file",
    "args": { "path": "/src/main.rs" }
  },
  "timestamp": "2026-02-28T12:00:00Z",
  "correlation_id": "req-uuid"
}
```

### CLI Commands

```
weft ipc send <pid> <message>              -- Send text message to agent by PID
weft ipc topics                             -- List active topics with subscriber counts
weft ipc subscribe <pid> <topic>            -- Subscribe agent to topic
```

---

## P -- Pseudocode

### Direct Message Send

```
fn A2AProtocol::send(from, msg):
    // 1. Validate sender exists
    sender = process_table.get(from)?
    if sender.state != Running:
        return Err(SenderNotRunning)

    // 2. Check IPC scope
    match msg.to:
        Pid(target_pid):
            capability_checker.check_ipc_target(from, target_pid)?
        Topic(topic):
            capability_checker.check_topic_access(from, topic)?
        Broadcast:
            capability_checker.check_broadcast(from)?

    // 3. Route message
    match msg.to:
        Pid(target_pid):
            deliver_to_pid(target_pid, msg).await
        Topic(topic):
            topic_router.publish(from, topic, msg.payload).await
        Broadcast:
            for pid in process_table.list_running():
                if pid != from:
                    deliver_to_pid(pid, msg.clone()).await
        Service(name):
            deliver_to_service(name, msg).await
```

### Request-Response Pattern

```
fn A2AProtocol::request(from, msg, timeout):
    // 1. Create response channel
    (tx, rx) = oneshot::channel()
    request_id = msg.id.clone()

    // 2. Register pending request
    pending_requests.insert(request_id, PendingRequest {
        from, to: msg.to.as_pid()?, sent_at: now(), timeout, response_tx: tx
    })

    // 3. Send message
    self.send(from, msg).await?

    // 4. Wait for response with timeout
    match tokio::time::timeout(timeout, rx).await:
        Ok(Ok(response)) -> Ok(response)
        Ok(Err(_)) -> Err(ResponseChannelClosed)
        Err(_) ->
            pending_requests.remove(request_id)
            Err(RequestTimeout)
```

### Topic Pub/Sub

```
fn TopicRouter::publish(from, topic, payload):
    // 1. Check publisher has topic access
    capability_checker.check_topic_access(from, topic)?

    // 2. Get subscribers
    subscribers = subscriptions.get(topic)?

    // 3. Deliver to each subscriber
    delivered = 0
    for pid in subscribers:
        // Check subscriber is still running
        if process_table.get(pid)?.state == Running:
            msg = KernelMessage {
                id: uuid(),
                from,
                to: Pid(pid),
                payload: payload.clone(),
                timestamp: now(),
                correlation_id: None,
            }
            deliver_to_pid(pid, msg).await
            delivered += 1

    return Ok(delivered)
```

### IPC Scope Enforcement

```
fn CapabilityChecker::check_ipc_target(from_pid, to_pid):
    caps = process_table.get(from_pid)?.capabilities

    match caps.ipc_scope:
        IpcScope::None:
            return Err(IpcDenied("agent has no IPC access"))
        IpcScope::Explicit(allowed_pids):
            if to_pid not in allowed_pids:
                return Err(IpcDenied("target PID not in allowed list"))
        IpcScope::Topic(topics):
            return Err(IpcDenied("agent can only use topic-based IPC"))
        IpcScope::All:
            // Allowed

    Ok(())
```

---

## A -- Architecture

### Component Relationships

```
KernelIpc (extended)
  |
  +-- A2AProtocol
  |     +-- CapabilityChecker (IPC scope enforcement)
  |     +-- ProcessTable (PID lookup, state validation)
  |     +-- PendingRequests (request-response tracking)
  |     +-- Per-agent inboxes (mpsc channels)
  |
  +-- TopicRouter
  |     +-- Subscriptions (DashMap<topic, Vec<Pid>>)
  |     +-- CapabilityChecker (topic access check)
  |
  +-- MessageBus (existing, wrapped)
        +-- Kernel adds typed envelope on top
```

### Integration Points

1. **MessageBus wrapping**: `KernelIpc` wraps the existing `MessageBus`. Messages between agents go through `A2AProtocol` which adds the typed envelope (`KernelMessage`). The underlying `MessageBus` handles the actual channel delivery.

2. **DelegationEngine**: The existing `DelegationEngine` in `clawft-services` can delegate tool calls between agents. In K2, this is wired to use `A2AProtocol::request()` for cross-agent tool delegation, replacing the current in-process delegation with PID-addressed IPC.

3. **MCP exposure**: `ipc_send` and `ipc_subscribe` are exposed as MCP tools so external clients (IDE plugins, web UI) can send messages to agents and subscribe to topics.

4. **Per-agent inbox**: Each agent gets an `mpsc::Receiver<KernelMessage>` created at spawn time. The `AgentLoop` polls this inbox alongside its normal message processing.

### Message Flow

```
Agent A (PID 1)                   KernelIpc                    Agent B (PID 7)
     |                                |                              |
     |-- send(KernelMessage) -------->|                              |
     |                                |-- check_ipc_scope(1, 7) --->|
     |                                |<-- Ok ----------------------|
     |                                |                              |
     |                                |-- deliver_to_inbox(7, msg) ->|
     |                                |                              |-- process msg
     |                                |                              |
     |                                |<-- reply(correlation_id) ----|
     |<-- response -------------------|                              |
```

### Ruvector Integration (Doc 07)

When the `ruvector-ipc` and `ruvector-wire` feature gates are enabled, ruvector crates
replace the custom IPC transport and topic routing. The custom JSON-RPC implementations
remain as fallbacks when the feature gates are disabled. See `07-ruvector-deep-integration.md`
for full adapter code.

| Custom Component | Ruvector Replacement | Feature Gate | Benefit |
|---|---|---|---|
| JSON-RPC wire format | `rvf-wire` segments (64-byte SIMD-aligned, type/size/checksum) | `ruvector-wire` | Crash-safe, zero-copy parsing, per-segment CRC |
| `A2AProtocol` request-response tracking | `ruvector-delta-consensus::DeltaConsensus` + `VectorClock` | `ruvector-ipc` | CRDT-based sync with causal ordering; no message reordering bugs |
| `TopicRouter` (DashMap subscriptions) | `ruvector-nervous-system::ShardedEventBus` | `ruvector-ipc` | High-throughput sharded delivery with budget-aware routing |
| Manual per-agent rate limiting | `ruvector-nervous-system::BudgetGuardrail` | `ruvector-ipc` | Resource-aware routing that avoids overloaded agents |
| (none -- new capability) | Gossip protocol from `ruvector-delta-consensus` | `ruvector-ipc` | Gossip-based pub/sub dissemination for topic messages |

**Wire format note**: When `ruvector-wire` is enabled, `KernelMessage` payloads are
serialized into `rvf-wire::WireSegment` with a fixed 64-byte header instead of JSON-RPC.
The `IpcTransport` trait abstracts over both; `WireIpcTransport` (rvf-wire) and
`JsonIpcTransport` (original) are selected at compile time via feature gate.

**ExoChain references**: `exo-core::HybridLogicalClock` (HLC) provides an alternative
to ruvector's `VectorClock` for causal ordering, offering wall-clock-correlated timestamps
while still preserving causality guarantees.

Cross-reference: `07-ruvector-deep-integration.md`, Section 3 "Phase K2: Agent-to-Agent IPC"
and Section 5 "Wire Format Migration".

---

## R -- Refinement

### Edge Cases

1. **Message to non-existent PID**: Return `ProcessNotFound` error; don't silently drop
2. **Message to exited process**: Return `ProcessNotRunning` error with last known state
3. **Topic with no subscribers**: `publish()` returns `Ok(0)` -- no error, just zero deliveries
4. **Subscriber exits while subscribed**: Lazy cleanup -- detect on next publish, remove dead PID from subscriber list
5. **Inbox buffer overflow**: Bounded mpsc channel (default 1024 messages). If full, oldest messages are dropped with warning log. Configurable per-agent.
6. **Self-messaging**: Agent sending to own PID is allowed (useful for deferred execution patterns)
7. **Concurrent subscribe/unsubscribe**: `DashMap` handles concurrent access. Subscribe during publish may miss the current publish.

### Backward Compatibility

- Existing `MessageBus` usage unchanged. Kernel IPC adds a layer on top.
- `DelegationEngine` changes are additive: fallback to current behavior when kernel is disabled
- MCP tools are additive; existing tool list unchanged

### Error Handling

- `IpcError` enum: `ProcessNotFound`, `ProcessNotRunning`, `IpcDenied`, `TopicNotFound`, `InboxFull`, `RequestTimeout`, `ResponseChannelClosed`
- All errors include `from_pid`, `to` target, and human-readable reason
- Rate limiting: per-agent, configurable in `ResourceLimits.max_messages_per_second` (default: 100)

---

## C -- Completion

### Exit Criteria

- [x] Direct PID-to-PID messaging works between two agents
- [x] Request-response pattern works with timeout
- [x] Topic subscription and publish deliver to all subscribers
- [x] IPC scope `None` blocks all messaging
- [x] IPC scope `Explicit([7])` allows messaging to PID 7 only
- [x] IPC scope `Topic(["build"])` allows topic-only communication
- [x] IPC scope `All` allows unrestricted messaging
- [x] Dead subscriber cleanup works on publish
- [x] `weft ipc send <pid> <msg>` CLI command works
- [x] `weft ipc topics` lists active topics
- [x] `weft ipc subscribe <pid> <topic>` creates subscription
- [x] DelegationEngine uses A2A protocol when kernel is active
- [x] MCP tools `ipc_send` and `ipc_subscribe` work
- [x] All workspace tests pass (`scripts/build.sh test`)
- [x] Clippy clean (`scripts/build.sh clippy`)

### Testing Verification

```bash
# A2A unit tests
cargo test -p clawft-kernel -- a2a

# Topic router tests
cargo test -p clawft-kernel -- topic

# IPC scope enforcement tests
cargo test -p clawft-kernel -- ipc_scope

# Integration: two agents exchange messages
cargo test -p clawft-kernel -- test_agent_messaging

# Integration: topic pub/sub
cargo test -p clawft-kernel -- test_topic_pubsub

# Regression check
scripts/build.sh test

# CLI smoke test
cargo run --bin weft -- ipc topics
```
