# Development Assignment: Element 05 - Pipeline & LLM Reliability

## Quick Reference
- **Branch**: `sprint/phase-5-pipeline-reliability`
- **Crates touched**: `clawft-core`, `clawft-llm`, `clawft-types`, `clawft-services`
- **Estimated items**: 11 (D1-D11)
- **Priority order**: D-Perf (D1, D10, D11) first, then D-Reliability (D3, D4, D7, D2, D8), then D-Observability (D5, D6), then D-Transport (D9)

## Prerequisites
- None -- Element 05 can run in parallel with Element 03
- **Cross-element dependency**: D9 blocks Element 09/M1 (FlowDelegator requires MCP transport concurrency for delegation)
- D6 `sender_id` is needed for Element 09 multi-agent cost tracking

## Internal Dependency Graph

```
D3 (structured errors) -------> D4 (configurable retry)
  Retry policy should use structured error variants, not strings.

D7 (FnMut callbacks) -------> D2 (streaming failover)
  Streaming failover benefits from FnMut callbacks for stateful
  reset tracking. Recommended but not strictly blocking.

D8 (bounded bus) is independent of all other D items.

D1, D10, D11 are fully independent of each other and all other items.
D5, D6 are independent of each other.
```

---

## Concurrent Work Units

### Unit 1: Performance [D-Perf] (can run in parallel with all other units)

All three items are independent and can be implemented in parallel.

#### Task D1: Parallel tool execution
- **File**: `crates/clawft-core/src/agent/loop_core.rs`
- **Current Code** (the bottleneck):
  ```rust
  // loop_core.rs:376-403
  for (id, name, input) in tool_calls {
      let permissions = request
          .auth_context
          .as_ref()
          .map(|ctx| &ctx.permissions);
      let result = self
          .tools
          .execute(&name, input.clone(), permissions)
          .await;
      let result_json = match result {
          Ok(val) => {
              let truncated =
                  crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
              serde_json::to_string(&truncated).unwrap_or_default()
          }
          Err(e) => {
              error!(tool = %name, error = %e, "tool execution failed");
              format!("{{\"error\": \"{}\"}}", e)
          }
      };

      request.messages.push(LlmMessage {
          role: "tool".into(),
          content: result_json,
          tool_call_id: Some(id),
          tool_calls: None,
      });
  }
  ```
  When the LLM returns multiple tool calls, they execute sequentially in a `for` loop.

- **Required Fix**: Use `futures::future::join_all` for concurrent execution. Collect results, then push them to messages in order.
  ```rust
  use futures::future::join_all;

  let permissions = request.auth_context.as_ref().map(|ctx| &ctx.permissions);

  let futures: Vec<_> = tool_calls
      .iter()
      .map(|(id, name, input)| {
          let tools = &self.tools;
          let name = name.clone();
          let input = input.clone();
          let perms = permissions;
          async move {
              let result = tools.execute(&name, input, perms).await;
              let result_json = match result {
                  Ok(val) => {
                      let truncated =
                          crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
                      serde_json::to_string(&truncated).unwrap_or_default()
                  }
                  Err(e) => {
                      error!(tool = %name, error = %e, "tool execution failed");
                      serde_json::json!({"error": e.to_string()}).to_string()
                  }
              };
              (id.clone(), result_json)
          }
      })
      .collect();

  let results = join_all(futures).await;

  for (id, result_json) in results {
      request.messages.push(LlmMessage {
          role: "tool".into(),
          content: result_json,
          tool_call_id: Some(id),
          tool_calls: None,
      });
  }
  ```

- **Risk**: Race conditions when parallel tools write to the same file. Mitigate with per-path advisory locks.

- **Tests Required**:
  - Timing test: 3 tools with 100ms simulated latency complete in <200ms
  - Correctness test: tool results maintain correct `tool_call_id` ordering
  - Race condition test: concurrent writes to same path handled correctly

- **Acceptance Criteria**:
  - [ ] Multiple tool calls execute concurrently via `join_all`
  - [ ] 3 tools with 100ms simulated latency complete in <200ms (timing test)
  - [ ] Tool results maintain correct `tool_call_id` association
  - [ ] `futures` crate added to dependencies if not already present

#### Task D10: Cache skill/agent bootstrap files
- **File**: `crates/clawft-core/src/agent/context.rs`
- **Current Code** (the bottleneck):
  ```rust
  // context.rs:105-139
  pub async fn build_system_prompt(&self) -> String {
      // ...
      for file_path in &candidates {
          debug!(file = %file_path.display(), "checking for bootstrap file");
          if self.platform.fs().exists(file_path).await {
              match self.platform.fs().read_to_string(file_path).await {
                  Ok(content) if !content.trim().is_empty() => {
                      debug!(file = %filename, bytes = content.len(), "loaded bootstrap file");
                      loaded_files.insert(*filename, content);
                  }
                  // ...
              }
              break;
          }
      }
  ```
  `build_system_prompt` reads `SOUL.md`, `AGENTS.md`, and skills files from disk on every single LLM call.

- **Required Fix**: Cache content with mtime-based invalidation. Store cached content + last mtime in the `ContextBuilder`. On subsequent calls, check `fs::metadata().modified()` and only re-read if changed.
  ```rust
  struct CachedFile {
      content: String,
      mtime: std::time::SystemTime,
  }

  // In ContextBuilder:
  bootstrap_cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
  ```

- **Tests Required**:
  - Second LLM call in same session skips disk read (verify via mock fs call count)
  - File modification triggers re-read
  - Cache invalidation on mtime change

- **Acceptance Criteria**:
  - [ ] Bootstrap files cached with mtime invalidation
  - [ ] Second LLM call in same session skips disk I/O for unchanged files
  - [ ] Modified files are detected and re-read

#### Task D11: Async file I/O in skills loader
- **File**: `crates/clawft-core/src/agent/skills_v2.rs`
- **Current Code** (the bottleneck):
  ```rust
  // skills_v2.rs:439
  let entries = std::fs::read_dir(dir).map_err(ClawftError::Io)?;

  // skills_v2.rs:473
  match std::fs::metadata(&skill_md_path) {

  // skills_v2.rs:496
  match std::fs::read_to_string(&skill_md_path) {

  // skills_v2.rs:568
  let json_content = std::fs::read_to_string(json_path).map_err(ClawftError::Io)?;

  // skills_v2.rs:581
  match std::fs::read_to_string(&prompt_path) {
  ```
  All these calls use blocking `std::fs` operations which block the Tokio executor thread.

- **Required Fix**: Replace all `std::fs` calls with `tokio::fs` equivalents. The function signatures may need to become `async`.
  ```rust
  let mut entries = tokio::fs::read_dir(dir).await.map_err(ClawftError::Io)?;
  let meta = tokio::fs::metadata(&skill_md_path).await;
  let content = tokio::fs::read_to_string(&skill_md_path).await;
  ```

- **Tests Required**:
  - Skills load correctly with async I/O
  - No blocking `std::fs` calls remain in the async execution path

- **Acceptance Criteria**:
  - [ ] Skills loader uses `tokio::fs` throughout
  - [ ] No blocking `std::fs` calls on the async executor path
  - [ ] All skill loading tests pass

---

### Unit 2: Reliability [D-Reliability] (D3 before D4; D7 before D2 recommended; D8 independent)

#### Task D3: Structured error variants for retry
- **Files**: `crates/clawft-llm/src/retry.rs`, `crates/clawft-types/src/error.rs`
- **Current Code** (the bug):
  ```rust
  // retry.rs:43-54
  pub fn is_retryable(err: &ProviderError) -> bool {
      match err {
          ProviderError::RateLimited { .. } => true,
          ProviderError::Timeout => true,
          ProviderError::Http(_) => true,
          ProviderError::RequestFailed(msg) => {
              // Retry on 5xx server errors
              msg.starts_with("HTTP 500")
                  || msg.starts_with("HTTP 502")
                  || msg.starts_with("HTTP 503")
                  || msg.starts_with("HTTP 504")
          }
          // ...
      }
  }
  ```
  `is_retryable()` uses fragile string prefix matching. If error message format changes, retry logic silently breaks.

- **Required Fix**: Add `ServerError { status: u16 }` variant to `ProviderError`. Replace string matching with variant matching.
  ```rust
  // In ProviderError enum:
  ServerError { status: u16, body: String },

  // In is_retryable:
  ProviderError::ServerError { status, .. } => (500..=599).contains(status),
  ```

- **Tests Required**:
  - `ServerError { status: 503 }` is retryable
  - `ServerError { status: 400 }` is NOT retryable
  - `ServerError { status: 599 }` is retryable (edge case)
  - No string prefix matching remains in `is_retryable()`

- **Acceptance Criteria**:
  - [ ] `ProviderError` has `ServerError { status: u16 }` variant
  - [ ] Retry logic uses structured variant matching, not string prefixes
  - [ ] No `msg.starts_with("HTTP 5")` patterns remain

#### Task D4: Configurable retry policy (depends on D3)
- **File**: `crates/clawft-core/src/pipeline/llm_adapter.rs`
- **Current Code** (the hardcoded values):
  ```rust
  // llm_adapter.rs:103-123
  const MAX_RETRIES: u32 = 3;
  let mut last_err = String::new();

  for attempt in 0..=MAX_RETRIES {
      match self.provider.complete(&request).await {
          Ok(response) => return Ok(convert_response_to_value(&response)),
          Err(ProviderError::RateLimited { retry_after_ms }) => {
              if attempt == MAX_RETRIES {
                  last_err = format!("rate limited after {} retries", MAX_RETRIES);
                  break;
              }
              let backoff_floor = 1000u64 * 2u64.pow(attempt);
              let wait = retry_after_ms.max(backoff_floor);
              // ...
          }
          Err(e) => return Err(e.to_string()),
      }
  }
  ```
  Retry count (3), backoff delay (1s base), and eligible error types are all hardcoded.

- **Required Fix**: Make configurable via `ClawftLlmConfig` / `config.json`.
  ```rust
  // In config:
  pub struct RetryPolicyConfig {
      pub max_retries: u32,        // default: 3
      pub base_delay_ms: u64,      // default: 1000
      pub max_delay_ms: u64,       // default: 30000
      pub retryable_status_codes: Vec<u16>,  // default: [500, 502, 503, 504]
  }
  ```

- **Tests Required**:
  - Custom retry count is honored
  - Custom backoff delay is used
  - Custom status codes determine retryability

- **Acceptance Criteria**:
  - [ ] Retry policy configurable via config.json (count, backoff delay, eligible status codes)
  - [ ] Default values match current hardcoded behavior
  - [ ] Config deserialization works with missing fields (defaults applied)

#### Task D7: Change `StreamCallback` to `FnMut`
- **File**: `crates/clawft-core/src/pipeline/traits.rs`
- **Current Code** (the limitation):
  ```rust
  // traits.rs:264
  pub type StreamCallback = Box<dyn Fn(&str) -> bool + Send + Sync>;
  ```
  `Fn` prevents token accumulators or progress trackers from working as callbacks since they need mutable state.

- **Required Fix**: Change to `FnMut`.
  ```rust
  pub type StreamCallback = Box<dyn FnMut(&str) -> bool + Send>;
  ```
  Note: `Sync` can be dropped since the callback is invoked from a single task. All call sites that pass `StreamCallback` must be updated.

- **Tests Required**:
  - A stateful token-counting callback compiles and runs
  - Existing streaming tests still pass

- **Acceptance Criteria**:
  - [ ] `StreamCallback` accepts `FnMut` closures
  - [ ] A stateful token-counting callback compiles and runs
  - [ ] All `complete_stream` call sites updated

#### Task D2: Streaming failover correctness (depends on D7 recommended)
- **File**: `crates/clawft-llm/src/failover.rs`
- **Current Code** (the bug):
  ```rust
  // failover.rs:34-36
  pub struct FailoverChain {
      providers: Vec<Box<dyn Provider>>,
  }
  ```
  Mid-stream provider failure sends partial data from the first provider followed by full data from the next, concatenated on the same channel. No stream reset mechanism.

- **Required Fix**: Implement "reset stream" that discards partial output before failover. Add a `StreamFailoverController` that tracks bytes emitted and can signal "discard and restart" to the callback consumer.

- **Tests Required**:
  - First provider fails mid-stream, second provider's complete output is delivered cleanly
  - No partial concatenation in output
  - Verify partial output is discarded

- **Acceptance Criteria**:
  - [ ] Streaming failover produces clean output (no partial concatenation)
  - [ ] First provider fails mid-stream, second provider's complete output is delivered cleanly
  - [ ] Partial output discarded before failover

#### Task D8: Bounded message bus channels (independent)
- **File**: `crates/clawft-core/src/bus.rs`
- **Current Code** (the risk):
  ```rust
  // bus.rs:3
  // Provides a thread-safe MessageBus using tokio unbounded MPSC channels

  // bus.rs:38-42
  pub struct MessageBus {
      inbound_tx: mpsc::UnboundedSender<InboundMessage>,
      inbound_rx: Mutex<mpsc::UnboundedReceiver<InboundMessage>>,
      outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
      outbound_rx: Mutex<mpsc::UnboundedReceiver<OutboundMessage>>,
  }

  // bus.rs:47-49
  pub fn new() -> Self {
      let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
      let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
  ```
  Unbounded channels provide no backpressure. A fast producer with a slow consumer grows memory without limit, leading to OOM.

- **Required Fix**: Switch to `mpsc::channel(buffer_size)` with configurable buffer size.
  ```rust
  pub struct MessageBus {
      inbound_tx: mpsc::Sender<InboundMessage>,
      inbound_rx: Mutex<mpsc::Receiver<InboundMessage>>,
      outbound_tx: mpsc::Sender<OutboundMessage>,
      outbound_rx: Mutex<mpsc::Receiver<OutboundMessage>>,
  }

  impl MessageBus {
      pub fn new() -> Self {
          Self::with_capacity(1024) // sensible default
      }

      pub fn with_capacity(buffer_size: usize) -> Self {
          let (inbound_tx, inbound_rx) = mpsc::channel(buffer_size);
          let (outbound_tx, outbound_rx) = mpsc::channel(buffer_size);
          // ...
      }
  }
  ```

- **Note**: `send()` on a bounded channel is async and will await when the buffer is full (backpressure). All call sites using `.send()` or `.try_send()` must be audited.

- **Tests Required**:
  - Backpressure test: full buffer causes sender to await
  - Default capacity test: bus works with default 1024 buffer
  - Custom capacity test: `with_capacity(10)` works correctly

- **Acceptance Criteria**:
  - [ ] Message bus uses bounded channels with configurable buffer size
  - [ ] Default buffer size is reasonable (1024)
  - [ ] Backpressure behavior is tested
  - [ ] No OOM risk from unbounded queue growth

---

### Unit 3: Observability [D-Observability] (can run after Unit 1 starts)

#### Task D5: Record actual latency in `ResponseOutcome`
- **Files**: `crates/clawft-core/src/pipeline/traits.rs`, `crates/clawft-core/src/agent/loop_core.rs`
- **Current Code** (the stub):
  ```rust
  // traits.rs:447-452
  let outcome = ResponseOutcome {
      success: true,
      quality: trajectory.quality,
      latency_ms: 0, // Caller should measure actual latency
  };

  // Also traits.rs:498-502
  let outcome = ResponseOutcome {
      success: true,
      quality: trajectory.quality,
      latency_ms: 0,
  };
  ```
  `latency_ms` is hardcoded to `0` everywhere. No actual latency measurement.

- **Required Fix**: Record wall-clock latency around provider calls using `std::time::Instant`.
  ```rust
  let start = std::time::Instant::now();
  let response = pipeline.transport.complete(&transport_request).await?;
  let latency_ms = start.elapsed().as_millis() as u64;

  let outcome = ResponseOutcome {
      success: true,
      quality: trajectory.quality,
      latency_ms,
  };
  ```

- **Tests Required**:
  - `latency_ms` is non-zero after a real (or mocked) provider call
  - Latency measurement wraps the correct code section

- **Acceptance Criteria**:
  - [ ] `latency_ms` populated in all `ResponseOutcome` records (no hardcoded zeros)
  - [ ] Latency measured around the actual transport call
  - [ ] Both `complete()` and `complete_stream()` paths measure latency

#### Task D6: Thread `sender_id` for cost recording
- **File**: `crates/clawft-core/src/pipeline/tiered_router.rs`
- **Current Code** (the gap):
  ```rust
  // tiered_router.rs:316
  let budget_check = cost_tracker.check_budget(
      &auth.sender_id,
      estimated_cost,
      permissions.cost_budget_daily_usd,
      permissions.cost_budget_monthly_usd,
  );
  ```
  `sender_id` is available in the routing phase for budget checks, but is NOT propagated through `RoutingDecision` to `CostTracker.record_actual()`. The cost tracker infrastructure is built but the integration is a no-op -- actual costs are never recorded per-user.

- **Required Fix**: Thread `sender_id` through the pipeline:
  1. Add `sender_id: Option<String>` to `RoutingDecision`
  2. Populate it from `ChatRequest.auth_context.sender_id` during routing
  3. Pass it to `CostTracker.record_actual()` after the LLM call completes

- **Tests Required**:
  - Integration test: `sender_id` propagated from `InboundMessage` through `ChatRequest` and `RoutingDecision` to `CostTracker.update()`
  - Unit test: `RoutingDecision` carries `sender_id`

- **Acceptance Criteria**:
  - [ ] `sender_id` propagated from `InboundMessage` through `ChatRequest` and `RoutingDecision` to `CostTracker.update()`
  - [ ] Integration test verifies end-to-end flow
  - [ ] Per-user cost recording is functional (not a no-op)

---

### Unit 4: Transport [D-Transport] (can run in parallel with Unit 2, Unit 3)

#### Task D9: MCP transport concurrency
- **File**: `crates/clawft-services/src/mcp/transport.rs`
- **Current Code** (the bottleneck):
  ```rust
  // transport.rs:33-38
  pub struct StdioTransport {
      #[allow(dead_code)]
      child: Arc<Mutex<Child>>,
      stdin: Arc<Mutex<tokio::process::ChildStdin>>,
      stdout: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
  }

  // transport.rs:74-79
  impl McpTransport for StdioTransport {
      async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
          let mut line = serde_json::to_string(&request)?;
          line.push('\n');
          debug!(method = %request.method, id = request.id, "sending stdio request");
          // Write to stdin (holding lock for entire request-response cycle)
  ```
  `StdioTransport` serializes concurrent calls completely -- the `Mutex<ChildStdin>` and `Mutex<ChildStdout>` locks are held for the entire request-response cycle. `HttpTransport` creates a new `reqwest::Client` per instance.

- **Required Fix**:
  1. **Stdio**: Implement request-ID multiplexer. Send requests without waiting for responses (release stdin lock after write). Use a response router (`HashMap<u64, oneshot::Sender>`) to match responses to pending requests by ID.
  2. **HTTP**: Accept `Arc<reqwest::Client>` for connection pooling instead of creating new clients.
  3. Redirect child stderr to log stream.

  ```rust
  pub struct StdioTransport {
      child: Arc<Mutex<Child>>,
      stdin: Arc<Mutex<tokio::process::ChildStdin>>,
      pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
      // Response reader task runs in background
  }
  ```

- **Cross-element dependency**: D9 blocks Element 09/M1 (FlowDelegator requires MCP transport concurrency for delegation).

- **Tests Required**:
  - Concurrent call test: 10+ simultaneous requests complete correctly
  - Request-ID correlation: responses match their requests
  - Timeout test: orphaned requests eventually error
  - HTTP client sharing test: `Arc<reqwest::Client>` reused across instances

- **Acceptance Criteria**:
  - [ ] MCP stdio transport supports concurrent requests via request-ID multiplexing
  - [ ] Concurrent call test with 10+ simultaneous requests passes
  - [ ] HTTP transport accepts shared `Arc<reqwest::Client>`
  - [ ] Child stderr redirected to log stream
  - [ ] Orphaned request timeout mechanism in place

---

## Exit Criteria Checklist

### D-Perf
- [ ] D1: Multiple tool calls execute concurrently; 3 tools with 100ms simulated latency complete in <200ms (timing test)
- [ ] D10: Bootstrap files cached with mtime invalidation; second LLM call in same session skips disk
- [ ] D11: Skills loader uses `tokio::fs`; no blocking `std::fs` calls on the async executor path

### D-Reliability
- [ ] D2: Streaming failover produces clean output (no partial concatenation); first provider fails mid-stream, second provider's complete output is delivered cleanly
- [ ] D3: Retry logic uses `ProviderError` enum variants; no string-prefix matching in `is_retryable()`
- [ ] D4: Retry policy configurable via config.json (count, backoff delay, eligible status codes)
- [ ] D7: `StreamCallback` accepts `FnMut` closures; a stateful token-counting callback compiles and runs
- [ ] D8: Message bus has configurable buffer size with backpressure

### D-Observability
- [ ] D5: `latency_ms` populated in all `ResponseOutcome` records (no hardcoded zeros)
- [ ] D6: `sender_id` propagated from `InboundMessage` through `ChatRequest` and `RoutingDecision` to `CostTracker.update()`; integration test verifies end-to-end flow

### D-Transport
- [ ] D9: MCP stdio transport supports concurrent requests via request-ID multiplexing (verified by concurrent call test)

### Regression
- [ ] All existing tests pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Development Notes Location

Record decisions, blockers, and difficult tasks in:
- `.planning/development_notes/02-improvements-overview/element-05/`

---

## Review Requirements
- All items must pass code review
- `cargo test --workspace` must pass after each unit
- `cargo clippy --workspace -- -D warnings` clean
- Performance items (D1, D10, D11) should include benchmark results in review comments
- D9 requires careful review of the multiplexer design -- concurrency bugs are subtle
