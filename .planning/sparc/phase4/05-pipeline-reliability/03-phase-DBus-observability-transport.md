# Phase 3: D-Bus / D-Observability / D-Transport

**Element**: 05 -- Pipeline & LLM Reliability
**Phase**: 3 (Bus + Observability + Transport)
**Timeline**: Weeks 3-5
**Crates**: `clawft-core`, `clawft-services`
**Dependencies**: None (all items are independent of each other and of Phase 1/Phase 2)
**Blocks**: Element 09/M1 (D9 MCP concurrency required for FlowDelegator)
**Status**: Planning

---

## Overview

This phase addresses four independent reliability and observability improvements:

| Item | Title | File(s) | Category |
|------|-------|---------|----------|
| D8 | Bounded message bus channels | `clawft-core/src/bus.rs` | Bus / Backpressure |
| D5 | Record actual latency in ResponseOutcome | `clawft-core/src/pipeline/traits.rs` | Observability |
| D6 | Thread sender_id for cost recording | `clawft-core/src/pipeline/tiered_router.rs`, `traits.rs` | Observability |
| D9 | MCP transport concurrency (request-ID multiplexing) | `clawft-services/src/mcp/transport.rs` | Transport |

All four items are independent and can be worked in parallel by separate developers. D9 is a cross-element blocker for Element 09/M1 (FlowDelegator).

---

## D8: Bounded Message Bus Channels

### Current Code

**File**: `crates/clawft-core/src/bus.rs` (333 lines)

The message bus uses unbounded channels for both inbound and outbound message routing. Under sustained load, unbounded channels provide no backpressure -- memory grows without limit.

```rust
// bus.rs:38-43
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Mutex<mpsc::UnboundedReceiver<InboundMessage>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::UnboundedReceiver<OutboundMessage>>,
}

// bus.rs:47-59
impl MessageBus {
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        debug!("MessageBus created");

        Self {
            inbound_tx,
            inbound_rx: Mutex::new(inbound_rx),
            outbound_tx,
            outbound_rx: Mutex::new(outbound_rx),
        }
    }
```

Publish methods use synchronous `send()` (unbounded channels never block):

```rust
// bus.rs:64-73
pub fn publish_inbound(&self, msg: InboundMessage) -> Result<(), ClawftError> {
    debug!(
        channel = %msg.channel,
        chat_id = %msg.chat_id,
        "publishing inbound message"
    );
    self.inbound_tx
        .send(msg)
        .map_err(|e| ClawftError::Channel(format!("inbound channel closed: {e}")))
}
```

Sender accessors return `UnboundedSender`:

```rust
// bus.rs:111-113
pub fn inbound_sender(&self) -> mpsc::UnboundedSender<InboundMessage> {
    self.inbound_tx.clone()
}

// bus.rs:119-121
pub fn outbound_sender(&self) -> mpsc::UnboundedSender<OutboundMessage> {
    self.outbound_tx.clone()
}
```

### Implementation Tasks

#### D8-1: Switch struct fields to bounded channel types

```rust
pub struct MessageBus {
    inbound_tx: mpsc::Sender<InboundMessage>,
    inbound_rx: Mutex<mpsc::Receiver<InboundMessage>>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::Receiver<OutboundMessage>>,
}
```

#### D8-2: Add `with_capacity` constructor, default = 1024

```rust
const DEFAULT_BUS_CAPACITY: usize = 1024;

impl MessageBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_BUS_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel(capacity);
        let (outbound_tx, outbound_rx) = mpsc::channel(capacity);

        debug!(capacity, "MessageBus created with bounded channels");

        Self {
            inbound_tx,
            inbound_rx: Mutex::new(inbound_rx),
            outbound_tx,
            outbound_rx: Mutex::new(outbound_rx),
        }
    }
}
```

#### D8-3: Make publish methods async (bounded `send()` is async)

**Critical change**: `mpsc::Sender::send()` is `async fn` (it awaits when the buffer is full). This propagates to all call sites.

```rust
pub async fn publish_inbound(&self, msg: InboundMessage) -> Result<(), ClawftError> {
    debug!(
        channel = %msg.channel,
        chat_id = %msg.chat_id,
        "publishing inbound message"
    );
    self.inbound_tx
        .send(msg)
        .await
        .map_err(|e| ClawftError::Channel(format!("inbound channel closed: {e}")))
}

pub async fn dispatch_outbound(&self, msg: OutboundMessage) -> Result<(), ClawftError> {
    debug!(
        channel = %msg.channel,
        chat_id = %msg.chat_id,
        "dispatching outbound message"
    );
    self.outbound_tx
        .send(msg)
        .await
        .map_err(|e| ClawftError::Channel(format!("outbound channel closed: {e}")))
}
```

#### D8-4: Update sender accessors to return bounded `Sender<T>`

```rust
pub fn inbound_sender(&self) -> mpsc::Sender<InboundMessage> {
    self.inbound_tx.clone()
}

pub fn outbound_sender(&self) -> mpsc::Sender<OutboundMessage> {
    self.outbound_tx.clone()
}
```

#### D8-5: Update all call sites

Search for all uses of `publish_inbound`, `dispatch_outbound`, `inbound_sender`, and `outbound_sender` across the workspace. Each call to `publish_inbound()` and `dispatch_outbound()` must add `.await`. Each use of `inbound_sender()` / `outbound_sender()` return types changes from `UnboundedSender<T>` to `Sender<T>`, and `.send(msg)` calls on those handles become `.send(msg).await`.

Known call sites to audit:
- `crates/clawft-core/src/agent/loop_core.rs` (agent loop dispatches outbound)
- Channel adapters (Telegram, Slack, Discord, etc.) that clone `inbound_sender()`
- Any integration tests or benchmarks

#### D8-6: Update 12 existing tests in `bus.rs`

All tests that call `publish_inbound()` or `dispatch_outbound()` must add `.await`. Tests using `inbound_sender().send()` or `outbound_sender().send()` must also become async and await the send. The standalone channel tests (`publish_inbound_error_on_closed_channel`, `dispatch_outbound_error_on_closed_channel`) must switch from `mpsc::unbounded_channel` to `mpsc::channel(N)`.

Key test changes:
- `publish_and_consume_inbound`: `bus.publish_inbound(msg).await.unwrap()`
- `inbound_sender_allows_multi_producer`: `tx1.send(make_inbound("from-tx1")).await.unwrap()`
- `concurrent_publish_and_consume`: `bus_clone.publish_inbound(...).await.unwrap()`
- `default_creates_valid_bus`: must become `#[tokio::test] async` since publish is now async
- `consume_returns_none_when_all_senders_dropped`: switch to `mpsc::channel(1)`

Add a new test for backpressure behavior:

```rust
#[tokio::test]
async fn bounded_channel_applies_backpressure() {
    let bus = MessageBus::with_capacity(2);
    // Fill the buffer
    bus.publish_inbound(make_inbound("msg-1")).await.unwrap();
    bus.publish_inbound(make_inbound("msg-2")).await.unwrap();
    // Third send should block (verify with timeout)
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        bus.publish_inbound(make_inbound("msg-3")),
    ).await;
    assert!(result.is_err(), "send should block when buffer is full");
    // Consume one to unblock
    bus.consume_inbound().await.unwrap();
    // Now the third send should succeed
    bus.publish_inbound(make_inbound("msg-3")).await.unwrap();
}
```

---

## D5: Record Actual Latency in ResponseOutcome

### Current Code

**File**: `crates/clawft-core/src/pipeline/traits.rs`

Both `PipelineRegistry::complete()` and `PipelineRegistry::complete_stream()` hardcode `latency_ms: 0`:

```rust
// traits.rs:447-452 (non-streaming path)
let outcome = ResponseOutcome {
    success: true,
    quality: trajectory.quality,
    latency_ms: 0, // Caller should measure actual latency
};
pipeline.router.update(&routing, &outcome);

// traits.rs:498-502 (streaming path)
let outcome = ResponseOutcome {
    success: true,
    quality: trajectory.quality,
    latency_ms: 0,
};
pipeline.router.update(&routing, &outcome);
```

The `ResponseOutcome` struct already has the field:

```rust
// traits.rs:139-149
pub struct ResponseOutcome {
    pub success: bool,
    pub quality: QualityScore,
    pub latency_ms: u64,
}
```

### Implementation Tasks

#### D5-1: Instrument `PipelineRegistry::complete()` with Instant timing

Wrap the transport call (line 432) in `std::time::Instant` measurement:

```rust
// In PipelineRegistry::complete(), around line 432:
let start = std::time::Instant::now();
let response = pipeline.transport.complete(&transport_request).await?;
let latency_ms = start.elapsed().as_millis() as u64;

// ... score, learn ...

let outcome = ResponseOutcome {
    success: true,
    quality: trajectory.quality,
    latency_ms,
};
pipeline.router.update(&routing, &outcome);
```

#### D5-2: Instrument `PipelineRegistry::complete_stream()` with Instant timing

Same pattern for the streaming path (around line 484):

```rust
// In PipelineRegistry::complete_stream(), around line 484:
let start = std::time::Instant::now();
let response = pipeline
    .transport
    .complete_stream(&transport_request, callback)
    .await?;
let latency_ms = start.elapsed().as_millis() as u64;

// ... score, learn ...

let outcome = ResponseOutcome {
    success: true,
    quality: trajectory.quality,
    latency_ms,
};
pipeline.router.update(&routing, &outcome);
```

#### D5-3: Add `use std::time::Instant` import

At the top of traits.rs (if not already present):

```rust
use std::time::Instant;
```

### Scope note

The timer should wrap ONLY the transport call (`complete()` / `complete_stream()`), not the full pipeline. This measures LLM provider latency specifically, which is what the `ResponseOutcome.latency_ms` field is intended for and what the router uses for adaptive decisions.

---

## D6: Thread sender_id for Cost Recording

### Current Code

**File**: `crates/clawft-core/src/pipeline/tiered_router.rs`

During routing, `sender_id` is available from `AuthContext` and used for budget checks:

```rust
// tiered_router.rs:315-320 (inside apply_budget_constraints)
let budget_check = cost_tracker.check_budget(
    &auth.sender_id,
    estimated_cost,
    permissions.cost_budget_daily_usd,
    permissions.cost_budget_monthly_usd,
);
```

But in the `update()` method (called after the LLM response), `sender_id` is NOT available:

```rust
// tiered_router.rs:701-711
fn update(&self, decision: &RoutingDecision, _outcome: &ResponseOutcome) {
    if let Some(cost) = decision.cost_estimate_usd {
        let _ = cost;
        // NOTE: sender_id is not available on RoutingDecision.
        // Phase F will thread sender_id through the pipeline properly.
    }
}
```

The `RoutingDecision` struct lacks a `sender_id` field:

```rust
// traits.rs:114-136
pub struct RoutingDecision {
    pub provider: String,
    pub model: String,
    pub reason: String,
    pub tier: Option<String>,
    pub cost_estimate_usd: Option<f64>,
    pub escalated: bool,
    pub budget_constrained: bool,
    // NOTE: no sender_id field!
}
```

### Implementation Tasks

#### D6-1: Add `sender_id` to `RoutingDecision`

```rust
// traits.rs -- RoutingDecision struct
pub struct RoutingDecision {
    pub provider: String,
    pub model: String,
    pub reason: String,
    pub tier: Option<String>,
    pub cost_estimate_usd: Option<f64>,
    pub escalated: bool,
    pub budget_constrained: bool,
    /// Identity of the sender, propagated from AuthContext for cost attribution.
    pub sender_id: Option<String>,
}
```

Since `RoutingDecision` derives `Default`, the new field defaults to `None` -- all existing construction sites using `..Default::default()` continue to compile.

#### D6-2: Populate `sender_id` in TieredRouter::route()

In `tiered_router.rs`, set `sender_id` when building the RoutingDecision (around line 687):

```rust
RoutingDecision {
    provider,
    model,
    reason: format!(
        "tiered routing: complexity={:.2}, tier={}, level={}, user={}",
        profile.complexity, final_tier.name, permissions.level, auth.sender_id,
    ),
    tier: Some(final_tier.name.clone()),
    cost_estimate_usd: Some(cost_estimate),
    escalated,
    budget_constrained,
    sender_id: Some(auth.sender_id.clone()),
}
```

Also populate in fallback/rate-limited decision paths where `auth` is available.

#### D6-3: Use `sender_id` in TieredRouter::update()

```rust
fn update(&self, decision: &RoutingDecision, _outcome: &ResponseOutcome) {
    if let (Some(ref tracker), Some(ref sender_id), Some(estimated_cost)) = (
        &self.cost_tracker,
        &decision.sender_id,
        decision.cost_estimate_usd,
    ) {
        // For now, actual_cost = estimated_cost (actual cost calculation
        // requires token-level billing which is a separate task).
        tracker.record_actual(sender_id, estimated_cost, estimated_cost);
    }
}
```

#### D6-4: Update tests

- All test assertions that construct `RoutingDecision` manually will continue to compile (thanks to `..Default::default()`) but should be reviewed to add `sender_id` where relevant.
- The `route_update_does_not_panic` test should verify that `record_actual` is called with the correct sender_id by using a mock tracker with recording.
- Add integration test: route with auth context -> verify `sender_id` appears on returned `RoutingDecision`.

---

## D9: MCP Transport Concurrency (Request-ID Multiplexing)

### Current Code

**File**: `crates/clawft-services/src/mcp/transport.rs` (351 lines)

#### StdioTransport (serialized request-response)

The current implementation holds the stdout lock for the entire request-response cycle, preventing any concurrency:

```rust
// transport.rs:33-38
pub struct StdioTransport {
    #[allow(dead_code)]
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    stdout: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
}
```

```rust
// transport.rs:75-110
async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
    let mut line = serde_json::to_string(&request)?;
    line.push('\n');

    debug!(method = %request.method, id = request.id, "sending stdio request");

    // Write to stdin.
    {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await.map_err(|e| {
            ServiceError::McpTransport(format!("failed to write to stdin: {e}"))
        })?;
        stdin
            .flush()
            .await
            .map_err(|e| ServiceError::McpTransport(format!("failed to flush stdin: {e}")))?;
    }

    // Read from stdout.
    let mut response_line = String::new();
    {
        let mut stdout = self.stdout.lock().await;
        stdout.read_line(&mut response_line).await.map_err(|e| {
            ServiceError::McpTransport(format!("failed to read from stdout: {e}"))
        })?;
    }

    if response_line.is_empty() {
        return Err(ServiceError::McpTransport(
            "child process closed stdout".into(),
        ));
    }

    let response: JsonRpcResponse = serde_json::from_str(response_line.trim())?;
    Ok(response)
}
```

The `stdin` and `stdout` locks are held sequentially but the overall pattern is: write request -> read response. No other request can start until the current one finishes reading.

#### HttpTransport (no connection pooling)

Creates a new `reqwest::Client` per transport instance. This is correct for connection reuse within a single transport (reqwest::Client internally pools connections), but does not allow sharing a client across multiple MCP servers:

```rust
// transport.rs:141-148
impl HttpTransport {
    pub fn new(endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint,
        }
    }
}
```

#### stderr is discarded

```rust
// transport.rs:52 (inside StdioTransport::new)
.stderr(std::process::Stdio::null());
```

Child process stderr is sent to /dev/null -- diagnostic output from MCP servers is lost.

### Implementation Tasks

#### D9-1: Redesign StdioTransport with request-ID multiplexer

Replace the lock-per-call pattern with a background reader task that routes responses by ID:

```rust
use tokio::sync::oneshot;

pub struct StdioTransport {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    reader_handle: tokio::task::JoinHandle<()>,
}
```

#### D9-2: Spawn background reader in constructor

```rust
impl StdioTransport {
    pub async fn new(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()); // D9: capture stderr

        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take()
            .ok_or_else(|| ServiceError::McpTransport("failed to capture stdin".into()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| ServiceError::McpTransport("failed to capture stdout".into()))?;
        let stderr = child.stderr.take();

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Spawn stderr logger
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            tracing::debug!(target: "mcp_stderr", "{}", line.trim());
                        }
                    }
                }
            });
        }

        // Spawn background stdout reader
        let pending_clone = pending.clone();
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => {
                        // stdout closed -- wake all pending with error
                        let mut map = pending_clone.lock().await;
                        for (_, sender) in map.drain() {
                            let _ = sender.send(JsonRpcResponse {
                                jsonrpc: "2.0".into(),
                                id: 0,
                                result: None,
                                error: Some(serde_json::json!({
                                    "code": -32000,
                                    "message": "child process closed stdout"
                                })),
                            });
                        }
                        break;
                    }
                    Ok(_) => {
                        match serde_json::from_str::<JsonRpcResponse>(line.trim()) {
                            Ok(response) => {
                                let mut map = pending_clone.lock().await;
                                if let Some(sender) = map.remove(&(response.id as u64)) {
                                    let _ = sender.send(response);
                                } else {
                                    tracing::warn!(
                                        id = response.id,
                                        "received response with unknown request ID"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "failed to parse JSON-RPC response from stdout"
                                );
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            reader_handle,
        })
    }
}
```

#### D9-3: Implement multiplexed send_request

The `stdin` lock is held only for the write. The response arrives via the background reader:

```rust
#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let request_id = request.id as u64;

        let mut line = serde_json::to_string(&request)?;
        line.push('\n');

        debug!(method = %request.method, id = request.id, "sending stdio request");

        // Register pending response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(request_id, tx);
        }

        // Write to stdin (lock held only for the write)
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(line.as_bytes()).await.map_err(|e| {
                ServiceError::McpTransport(format!("failed to write to stdin: {e}"))
            })?;
            stdin.flush().await.map_err(|e| {
                ServiceError::McpTransport(format!("failed to flush stdin: {e}"))
            })?;
        }

        // Wait for the background reader to deliver our response
        let response = rx.await.map_err(|_| {
            ServiceError::McpTransport("response channel closed (reader task died)".into())
        })?;

        // Check for error response indicating stdout closure
        if response.error.is_some() && response.id == 0 {
            return Err(ServiceError::McpTransport(
                "child process closed stdout".into(),
            ));
        }

        Ok(response)
    }

    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        // Notifications unchanged -- just write to stdin, no response expected
        let notif = JsonRpcNotification::new(method, params);
        let mut line = serde_json::to_string(&notif)?;
        line.push('\n');

        debug!(method = %method, "sending stdio notification");

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await.map_err(|e| {
            ServiceError::McpTransport(format!("failed to write notification to stdin: {e}"))
        })?;
        stdin.flush().await.map_err(|e| {
            ServiceError::McpTransport(format!("failed to flush stdin after notification: {e}"))
        })?;

        Ok(())
    }
}
```

#### D9-4: Add timeout for orphaned requests

Requests that never receive a response should not hang forever. Add a configurable timeout (default 30s):

```rust
// In send_request, replace bare rx.await with:
let response = tokio::time::timeout(
    std::time::Duration::from_secs(30),
    rx,
).await
    .map_err(|_| ServiceError::McpTransport(
        format!("request {} timed out after 30s", request_id)
    ))?
    .map_err(|_| ServiceError::McpTransport(
        "response channel closed (reader task died)".into()
    ))?;
```

Clean up the pending entry on timeout:

```rust
// After timeout error, remove the pending entry
let response = match tokio::time::timeout(Duration::from_secs(30), rx).await {
    Ok(Ok(resp)) => resp,
    Ok(Err(_)) => {
        return Err(ServiceError::McpTransport(
            "response channel closed (reader task died)".into(),
        ));
    }
    Err(_) => {
        // Remove orphaned pending entry
        self.pending.lock().await.remove(&request_id);
        return Err(ServiceError::McpTransport(
            format!("request {} timed out after 30s", request_id),
        ));
    }
};
```

#### D9-5: HttpTransport -- accept shared client

Allow callers to share a `reqwest::Client` across multiple `HttpTransport` instances for connection pooling:

```rust
impl HttpTransport {
    pub fn new(endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint,
        }
    }

    /// Create an HTTP transport with a shared client for connection pooling.
    pub fn with_client(client: reqwest::Client, endpoint: String) -> Self {
        Self { client, endpoint }
    }
}
```

#### D9-6: Implement Drop for StdioTransport to clean up reader task

```rust
impl Drop for StdioTransport {
    fn drop(&mut self) {
        self.reader_handle.abort();
    }
}
```

---

## Concurrency Plan

All four items are independent and can be worked simultaneously:

```
  D8 (bus.rs)  ─────────┐
  D5 (traits.rs latency) ├──> All merge to main independently
  D6 (traits.rs + router) ┤
  D9 (transport.rs)  ────┘
```

**No ordering constraints between items.** D5 and D6 both touch `traits.rs` but different sections:
- D5 modifies `PipelineRegistry::complete()` and `complete_stream()` (lines 410-506)
- D6 modifies `RoutingDecision` struct (lines 114-136) and `TieredRouter::update()` in tiered_router.rs

If worked in parallel, a merge conflict in `traits.rs` will be trivial (different hunks).

---

## Tests Required

### D8 Tests

| Test | Description |
|------|-------------|
| `publish_and_consume_inbound` | Update to async publish (existing, modified) |
| `dispatch_and_consume_outbound` | Update to async dispatch (existing, modified) |
| `multiple_inbound_messages_in_order` | Async publish calls (existing, modified) |
| `multiple_outbound_messages_in_order` | Async dispatch calls (existing, modified) |
| `inbound_sender_allows_multi_producer` | `Sender::send()` is now async (existing, modified) |
| `outbound_sender_allows_multi_producer` | `Sender::send()` is now async (existing, modified) |
| `consume_returns_none_when_all_senders_dropped` | Switch from unbounded to bounded channel (existing, modified) |
| `publish_inbound_error_on_closed_channel` | Switch from unbounded to bounded (existing, modified) |
| `dispatch_outbound_error_on_closed_channel` | Switch from unbounded to bounded (existing, modified) |
| `default_creates_valid_bus` | Must become async test (existing, modified) |
| `concurrent_publish_and_consume` | Async publish (existing, modified) |
| `message_bus_is_send_sync` | Unchanged (existing) |
| `inbound_and_outbound_are_independent` | Async publish/dispatch (existing, modified) |
| **`bounded_channel_applies_backpressure`** | **NEW**: verify send blocks when buffer full |
| **`with_capacity_custom_size`** | **NEW**: verify custom capacity construction |

### D5 Tests

| Test | Description |
|------|-------------|
| **`complete_records_nonzero_latency`** | NEW: Run `PipelineRegistry::complete()` with test transport, verify `latency_ms > 0` in outcome |
| **`complete_stream_records_nonzero_latency`** | NEW: Same for streaming path |
| `response_outcome_construction` | Existing test already covers `latency_ms: 1500` -- no change needed |

To capture the latency in tests, the `TestRouter` mock needs to record the `ResponseOutcome` passed to `update()`:

```rust
struct TestRouterWithCapture {
    provider: String,
    model: String,
    captured_outcome: Mutex<Option<ResponseOutcome>>,
}

impl ModelRouter for TestRouterWithCapture {
    fn update(&self, _decision: &RoutingDecision, outcome: &ResponseOutcome) {
        *self.captured_outcome.lock().unwrap() = Some(outcome.clone());
    }
}
```

### D6 Tests

| Test | Description |
|------|-------------|
| **`routing_decision_includes_sender_id`** | NEW: Route with auth, verify `decision.sender_id == Some("user_123")` |
| **`routing_decision_none_sender_id_without_auth`** | NEW: Route without auth, verify `decision.sender_id == None` or default |
| **`update_calls_record_actual_with_sender_id`** | NEW: Mock cost tracker records `record_actual()` call args |
| `routing_decision_construction` | Existing -- unchanged (uses `..Default::default()`) |
| `route_update_does_not_panic` | Existing -- should be enhanced to verify actual behavior |

### D9 Tests

| Test | Description |
|------|-------------|
| **`multiplexed_concurrent_requests`** | NEW: Send 10 requests concurrently, verify all get correct responses matched by ID |
| **`orphaned_request_times_out`** | NEW: Send request where background reader never delivers, verify timeout error |
| **`reader_wakes_all_on_stdout_close`** | NEW: Close child stdout while requests are pending, verify all get error responses |
| **`http_transport_with_shared_client`** | NEW: Construct with `with_client()`, verify endpoint and client are set |
| **`stderr_is_captured`** | NEW: Verify child stderr output is logged (not discarded) |
| `http_transport_construction` | Existing -- unchanged |
| `mock_transport_returns_responses` | Existing -- unchanged |
| `mock_transport_records_requests` | Existing -- unchanged |
| `mock_transport_empty_responses_errors` | Existing -- unchanged |
| `mock_transport_records_notifications` | Existing -- unchanged |
| `notification_has_no_id_field` | Existing -- unchanged |

---

## Acceptance Criteria

### D8
- [ ] `MessageBus` uses `mpsc::channel(N)` (bounded) instead of `mpsc::unbounded_channel()`
- [ ] `with_capacity(usize)` constructor exists; `new()` defaults to 1024
- [ ] `publish_inbound()` and `dispatch_outbound()` are `async fn`
- [ ] `inbound_sender()` returns `mpsc::Sender<InboundMessage>` (not Unbounded)
- [ ] `outbound_sender()` returns `mpsc::Sender<OutboundMessage>` (not Unbounded)
- [ ] All 12 existing tests updated and pass
- [ ] Backpressure test demonstrates blocking behavior when buffer is full
- [ ] All call sites across the workspace compile with the new async signatures

### D5
- [ ] `latency_ms` is populated with actual `Instant::elapsed()` measurement in both `complete()` and `complete_stream()`
- [ ] No hardcoded `latency_ms: 0` remains in pipeline orchestration code
- [ ] Test verifies `latency_ms > 0` in the outcome passed to `router.update()`

### D6
- [ ] `RoutingDecision` has `sender_id: Option<String>` field
- [ ] `TieredRouter::route()` populates `sender_id` from `AuthContext`
- [ ] `TieredRouter::update()` calls `cost_tracker.record_actual()` with the sender_id
- [ ] Integration test verifies sender_id propagation end-to-end
- [ ] Existing tests compile without changes (field defaults to `None`)

### D9
- [ ] `StdioTransport` uses background reader task for response routing
- [ ] `stdin` lock held only during writes (not during response wait)
- [ ] Request-ID multiplexing allows concurrent `send_request()` calls
- [ ] Orphaned requests time out after configurable duration (default 30s)
- [ ] `HttpTransport::with_client()` constructor accepts shared `reqwest::Client`
- [ ] Child stderr captured and logged via `tracing::debug!` (not `/dev/null`)
- [ ] All existing tests pass
- [ ] Concurrent request test demonstrates correct response routing

### Regression
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` has no new warnings

---

## Risk Notes

| Risk | Item | Likelihood | Impact | Mitigation |
|------|------|-----------|--------|------------|
| Async publish propagation breaks many call sites | D8 | High | Medium | Compile-driven: `cargo check` surfaces all sites. Consider adding `try_publish_inbound()` (using `try_send()`) as a non-async escape hatch for fire-and-forget contexts. |
| Backpressure stalls agent loop | D8 | Medium | High | Default 1024 is large enough for normal operation. Monitor in production. `try_send()` with drop-oldest could be added later if needed. |
| Timer resolution on different platforms | D5 | Low | Low | `Instant::elapsed()` is monotonic and reliable on all supported platforms. |
| Multiplexer complexity introduces subtle ordering bugs | D9 | Medium | Medium | Thorough integration tests with concurrent calls. Background reader task is the only stdout consumer, eliminating races. |
| Orphan cleanup missed on timeout | D9 | Low | Low | `pending.lock().remove()` in the timeout branch prevents leak. Periodic sweep could be added if needed. |
| `JsonRpcResponse.id` type mismatch (u64 vs i64) | D9 | Low | High | Verify `JsonRpcResponse.id` field type matches the HashMap key type. Cast consistently. |
| D9 blocks Element 09/M1 timeline | D9 | Medium | High | Prioritize D9 early in the phase. FlowDelegator design can proceed in parallel using the `McpTransport` trait. |
