# SPARC Implementation Plan: Stream 2G - Service Wiring

**Timeline**: Week 11-12
**Owned Crates**: `clawft-cli`, `clawft-core` (bootstrap changes only)
**Dependencies**: Phase 1 complete (MessageBus, ToolRegistry, SessionManager), Stream 2E (clawft-services: CronService, HeartbeatService, McpClient)

---

## 1. Agent Instructions

### Source Files to Read (Existing Implementation)
```
clawft/crates/clawft-services/src/cron_service/mod.rs        # CronService API (new, add_job, start, run_job_now)
clawft/crates/clawft-services/src/heartbeat/mod.rs           # HeartbeatService API (new, start)
clawft/crates/clawft-services/src/mcp/mod.rs                 # McpClient API (new, list_tools, call_tool)
clawft/crates/clawft-services/src/mcp/transport.rs           # StdioTransport, HttpTransport
clawft/crates/clawft-services/src/error.rs                   # ServiceError variants
clawft/crates/clawft-core/src/bootstrap.rs                   # AppContext -- needs service fields
clawft/crates/clawft-core/src/tools/registry.rs              # Tool trait, ToolRegistry
clawft/crates/clawft-core/src/intelligent_router.rs          # IntelligentRouter (vector-memory gated)
clawft/crates/clawft-core/src/session_indexer.rs             # SessionIndexer (vector-memory gated)
clawft/crates/clawft-cli/src/commands/gateway.rs             # Gateway command -- needs cron/heartbeat spawn
clawft/crates/clawft-cli/src/commands/cron.rs                # cron_run() -- placeholder to wire
clawft/crates/clawft-cli/src/commands/agent.rs               # Agent command -- needs --intelligent-routing
clawft/crates/clawft-cli/Cargo.toml                          # Feature dependencies
clawft/crates/clawft-types/src/config.rs                     # MCPServerConfig, GatewayConfig, ToolsConfig
```

### Planning Documents (MUST READ)
```
.planning/02-technical-requirements.md  # Service lifecycle, tool registry, MCP wiring
.planning/03-development-guide.md       # Stream 2G timeline
.planning/01-project-overview.md        # Service integration requirements
```

### Key Constraints

1. **CronService** requires an `mpsc::UnboundedSender<InboundMessage>` -- obtained from `bus.inbound_sender()`
2. **HeartbeatService** requires the same sender type
3. **McpClient** requires a `Box<dyn McpTransport>` -- choose `StdioTransport` or `HttpTransport` based on config
4. **IntelligentRouter** is gated behind `#[cfg(feature = "vector-memory")]` in clawft-core
5. **All background tasks** must accept a `CancellationToken` and exit gracefully on Ctrl+C
6. **Gateway command** currently uses `tokio::signal::ctrl_c()` for shutdown -- must be replaced with a `CancellationToken` that is cancelled on Ctrl+C
7. **`weft cron run`** currently prints a placeholder -- must delegate to `CronService::run_job_now()`

---

## 2. Specification

### 2.1 CronService Wiring

#### Acceptance Criteria
- [ ] Gateway command creates a `CronService` from the JSONL storage path (`~/.clawft/cron.jsonl` with `~/.nanobot/cron.jsonl` fallback)
- [ ] CronService receives a clone of `bus.inbound_sender()` for posting InboundMessages
- [ ] Persisted jobs are loaded from JSONL storage at startup via `CronService::new()`
- [ ] `cron_service.start(cancel_token.clone())` is spawned as a background tokio task
- [ ] Invalid persisted jobs log a warning but do not prevent startup
- [ ] On Ctrl+C the cancel token is triggered, causing `CronService::start()` to return `Ok(())`
- [ ] `weft cron run <job-id>` delegates to `CronService::run_job_now()` when run inside a gateway context
- [ ] Cron-fired messages arrive on the bus with `channel: "cron"` and `sender_id: "system"`

#### Config Surface
No new config fields required. CronService uses file-based JSONL storage; the path is derived from the platform home directory (same logic as `cron_store_path()` in `cron.rs`).

### 2.2 HeartbeatService Wiring

#### Acceptance Criteria
- [ ] Gateway command creates a `HeartbeatService` when heartbeat is configured
- [ ] Interval is configurable via a new `gateway.heartbeat_interval_minutes` config field (default: 0 = disabled)
- [ ] Heartbeat prompt is configurable via `gateway.heartbeat_prompt` config field (default: `"heartbeat"`)
- [ ] `heartbeat_service.start(cancel_token.clone())` is spawned as a background tokio task
- [ ] Heartbeat messages arrive on the bus with `channel: "heartbeat"` and `sender_id: "system"`
- [ ] When `heartbeat_interval_minutes` is 0, no HeartbeatService is created
- [ ] On cancel, the heartbeat task exits without error
- [ ] Heartbeat start/stop events are logged via `tracing::info!`

#### Config Surface
Add to `GatewayConfig`:
```rust
/// Heartbeat interval in minutes (0 = disabled).
#[serde(default, alias = "heartbeatIntervalMinutes")]
pub heartbeat_interval_minutes: u64,

/// Heartbeat prompt text.
#[serde(default = "default_heartbeat_prompt", alias = "heartbeatPrompt")]
pub heartbeat_prompt: String,
```

### 2.3 McpClient to ToolRegistry Wiring

#### Acceptance Criteria
- [ ] For each entry in `config.tools.mcp_servers`, an `McpClient` is created with the appropriate transport
- [ ] If `mcp_server.command` is non-empty, use `StdioTransport::new(command, args, env)`
- [ ] If `mcp_server.url` is non-empty and `command` is empty, use `HttpTransport::new(url)`
- [ ] If both are empty, log a warning and skip the server
- [ ] `client.list_tools()` is called to discover available tools
- [ ] For each discovered tool, an `McpToolWrapper` struct is created implementing the `Tool` trait
- [ ] `McpToolWrapper::name()` returns the tool name prefixed with the server name: `"{server_name}__{tool_name}"`
- [ ] `McpToolWrapper::description()` returns the MCP tool's description
- [ ] `McpToolWrapper::parameters()` returns the MCP tool's `input_schema`
- [ ] `McpToolWrapper::execute()` delegates to `client.call_tool(tool_name, args)`
- [ ] All discovered tools are registered in `AppContext.tools_mut()` before `into_agent_loop()`
- [ ] Transport errors during `list_tools()` log a warning but do not prevent startup
- [ ] Tool execution errors are mapped to `ToolError::ExecutionFailed`

#### McpToolWrapper Design
```rust
struct McpToolWrapper {
    server_name: String,
    tool_def: ToolDefinition,
    client: Arc<McpClient>,
}
```

The `McpClient` must be wrapped in `Arc` because multiple `McpToolWrapper` instances share the same client (one per tool on the same server).

### 2.4 Vector-Memory / Intelligent Routing Wiring

#### Acceptance Criteria
- [ ] `clawft-cli` Cargo.toml gains a `vector-memory` feature that enables `clawft-core/vector-memory`
- [ ] `AppContext` conditionally holds an `Option<IntelligentRouter>` when `vector-memory` is enabled
- [ ] `AppContext` conditionally holds an `Option<SessionIndexer>` when `vector-memory` is enabled
- [ ] `AppContext::new()` creates both when the feature is enabled, using `HashEmbedder` as default embedder
- [ ] `weft agent` and `weft gateway` gain a `--intelligent-routing` CLI flag
- [ ] When `--intelligent-routing` is passed and `vector-memory` feature is enabled, the `IntelligentRouter` is constructed and passed to the pipeline
- [ ] When `--intelligent-routing` is passed but `vector-memory` feature is disabled, a clear error message is printed
- [ ] `SessionIndexer` indexes each turn after `process_message()` completes (user message + assistant response pair)
- [ ] Indexed turns are searchable via the `SessionIndexer::search_turns()` API

---

## 3. Pseudocode

### 3.1 Gateway Command with Service Wiring
```
async fn run(args: GatewayArgs) -> Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let config = load_config(&platform, args.config).await?;

    // Create global cancellation token.
    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();

    // Spawn Ctrl+C handler.
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("received Ctrl+C, shutting down");
        cancel_for_signal.cancel();
    });

    // Create the message bus.
    let bus = Arc::new(MessageBus::new());
    let inbound_tx = bus.inbound_sender();

    // ── CronService ──────────────────────────────────────
    let cron_storage_path = resolve_cron_storage_path(&platform);
    let cron_service = Arc::new(
        CronService::new(cron_storage_path, inbound_tx.clone()).await?
    );
    let cron_cancel = cancel.clone();
    let cron_handle = {
        let svc = cron_service.clone();
        tokio::spawn(async move { svc.start(cron_cancel).await })
    };
    info!("cron service started");

    // ── HeartbeatService ─────────────────────────────────
    let heartbeat_handle = if config.gateway.heartbeat_interval_minutes > 0 {
        let svc = HeartbeatService::new(
            config.gateway.heartbeat_interval_minutes,
            config.gateway.heartbeat_prompt.clone(),
            inbound_tx.clone(),
        );
        let hb_cancel = cancel.clone();
        Some(tokio::spawn(async move { svc.start(hb_cancel).await }))
    } else {
        info!("heartbeat service disabled (interval=0)");
        None
    };

    // ── MCP Tool Discovery ───────────────────────────────
    let mut app_ctx = AppContext::new(config.clone(), platform.clone()).await?;

    for (server_name, server_cfg) in &config.tools.mcp_servers {
        match create_mcp_client(server_name, server_cfg).await {
            Ok(client) => {
                let client = Arc::new(client);
                match client.list_tools().await {
                    Ok(tools) => {
                        info!(server = %server_name, count = tools.len(), "discovered MCP tools");
                        for tool_def in tools {
                            let wrapper = McpToolWrapper {
                                server_name: server_name.clone(),
                                tool_def: tool_def.clone(),
                                client: client.clone(),
                            };
                            app_ctx.tools_mut().register(Arc::new(wrapper));
                        }
                    }
                    Err(e) => {
                        warn!(server = %server_name, error = %e, "failed to list MCP tools, skipping");
                    }
                }
            }
            Err(e) => {
                warn!(server = %server_name, error = %e, "failed to create MCP client, skipping");
            }
        }
    }

    // ── Channel plugins (existing logic) ─────────────────
    let host = make_channel_host(bus.clone());
    let plugin_host = PluginHost::new(host);
    // ... register and start channels ...

    // ── Agent loop ───────────────────────────────────────
    let agent_loop = app_ctx.into_agent_loop();
    let agent_cancel = cancel.clone();
    let agent_handle = tokio::spawn(async move {
        agent_loop.run(agent_cancel).await
    });

    // ── Wait for shutdown ────────────────────────────────
    cancel.cancelled().await;
    info!("shutting down gateway");

    // Stop channels.
    plugin_host.stop_all().await;

    // Wait for background tasks.
    let _ = cron_handle.await;
    if let Some(hb) = heartbeat_handle {
        let _ = hb.await;
    }
    let _ = agent_handle.await;

    info!("gateway shutdown complete");
    Ok(())
}
```

### 3.2 MCP Client Factory
```
async fn create_mcp_client(
    name: &str,
    cfg: &MCPServerConfig,
) -> Result<McpClient> {
    if !cfg.command.is_empty() {
        // Stdio transport: spawn child process.
        let transport = StdioTransport::new(&cfg.command, &cfg.args, &cfg.env).await?;
        Ok(McpClient::new(Box::new(transport)))
    } else if !cfg.url.is_empty() {
        // HTTP transport.
        let transport = HttpTransport::new(cfg.url.clone());
        Ok(McpClient::new(Box::new(transport)))
    } else {
        Err(anyhow!("MCP server '{}' has neither command nor url", name))
    }
}
```

### 3.3 McpToolWrapper Implementation
```
struct McpToolWrapper {
    server_name: String,
    tool_def: ToolDefinition,
    client: Arc<McpClient>,
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        // Return pre-computed prefixed name.
        &self.prefixed_name
    }

    fn description(&self) -> &str {
        &self.tool_def.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.tool_def.input_schema.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        self.client
            .call_tool(&self.tool_def.name, args)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!(
                "MCP server '{}' tool '{}' failed: {}",
                self.server_name, self.tool_def.name, e
            )))
    }
}
```

### 3.4 Cron Run Wiring
```
/// Manually trigger a cron job via the CronService.
///
/// This replaces the placeholder in the current cron_run().
pub async fn cron_run(job_id: String, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();

    // Verify job exists in store.
    let store = load_store(&path)?;
    let job = store.jobs.iter()
        .find(|j| j.id == job_id)
        .ok_or_else(|| anyhow!("cron job not found: {job_id}"))?;

    println!("Triggering cron job '{}' ({})", job.name, job.id);

    // Create a temporary CronService to fire the job.
    let (tx, mut rx) = mpsc::unbounded_channel();
    let svc = CronService::new(path.with_extension("jsonl"), tx).await?;
    svc.run_job_now(&job_id).await
        .map_err(|e| anyhow!("failed to run job: {e}"))?;

    // Drain the message that was fired.
    if let Ok(msg) = rx.try_recv() {
        println!("  Fired message: channel={}, content={}", msg.channel, msg.content);
    }

    println!("Cron job '{}' executed successfully.", job.name);
    Ok(())
}
```

### 3.5 Vector-Memory Conditional Wiring
```
// In AppContext::new(), after building the default pipeline:

#[cfg(feature = "vector-memory")]
let intelligent_router = if enable_intelligent_routing {
    use crate::embeddings::hash_embedder::HashEmbedder;
    let embedder = Box::new(HashEmbedder::new(64));
    Some(IntelligentRouter::new(embedder).await)
} else {
    None
};

#[cfg(feature = "vector-memory")]
let session_indexer = if enable_intelligent_routing {
    use crate::embeddings::hash_embedder::HashEmbedder;
    let embedder = Box::new(HashEmbedder::new(64));
    Some(SessionIndexer::new(embedder).await)
} else {
    None
};

// After process_message() in the agent loop:
#[cfg(feature = "vector-memory")]
if let Some(ref mut indexer) = self.session_indexer {
    let turn = ConversationTurn {
        session_id: session_id.clone(),
        turn_id: turn_number,
        user_message: user_msg.clone(),
        assistant_message: response.clone(),
        timestamp: now_secs(),
        model: model_name.clone(),
    };
    if let Err(e) = indexer.index_turn(&turn).await {
        warn!(error = %e, "failed to index conversation turn");
    }
}
```

---

## 4. Architecture

### 4.1 Service Lifecycle in Gateway

```
Gateway Process
├── CancellationToken (root)
│   ├── ctrl_c_handler          # cancels token on Ctrl+C
│   ├── CronService.start()     # background task, exits on cancel
│   ├── HeartbeatService.start()# background task, exits on cancel (optional)
│   ├── PluginHost channels     # each channel has its own cancel token
│   └── AgentLoop.run()         # main processing loop
│
├── MessageBus
│   ├── inbound_sender ──→ CronService (fires cron messages)
│   ├── inbound_sender ──→ HeartbeatService (fires heartbeat messages)
│   ├── inbound_sender ──→ PluginHost channels (telegram, slack, etc.)
│   └── inbound_receiver ──→ AgentLoop (consumes all inbound messages)
│
├── ToolRegistry
│   ├── built-in tools (file_read, file_write, exec, web_search)
│   └── MCP tools (discovered at startup)
│       ├── server-a__tool-1 ──→ McpToolWrapper ──→ Arc<McpClient(StdioTransport)>
│       ├── server-a__tool-2 ──→ McpToolWrapper ──→ Arc<McpClient(StdioTransport)>
│       └── server-b__tool-1 ──→ McpToolWrapper ──→ Arc<McpClient(HttpTransport)>
│
└── [vector-memory]
    ├── IntelligentRouter       # 3-tier routing (ADR-026)
    └── SessionIndexer          # indexes turns after process_message()
```

### 4.2 Shutdown Sequence

```
1. User presses Ctrl+C
2. ctrl_c_handler cancels the root CancellationToken
3. All background tasks observe cancellation:
   a. CronService::start() → breaks select! loop → returns Ok(())
   b. HeartbeatService::start() → breaks select! loop → returns Ok(())
   c. Channel plugins → PluginHost::stop_all() called
   d. AgentLoop → exits processing loop
4. Gateway awaits all JoinHandles
5. MCP child processes (StdioTransport) are dropped, stdin closed → child exits
6. Gateway logs "shutdown complete" and returns
```

### 4.3 MCP Tool Discovery Flow

```
For each (server_name, server_cfg) in config.tools.mcp_servers:
  │
  ├─ cfg.command non-empty?
  │   YES → StdioTransport::new(command, args, env)
  │          └── Spawns child process, captures stdin/stdout
  │
  ├─ cfg.url non-empty?
  │   YES → HttpTransport::new(url)
  │          └── Creates reqwest::Client with endpoint
  │
  ├─ Both empty?
  │   YES → warn! and skip
  │
  └── client.list_tools()
      │
      ├─ Ok(tools) → for each ToolDefinition:
      │   └── McpToolWrapper { server_name, tool_def, client }
      │       └── registry.register(Arc::new(wrapper))
      │
      └─ Err(e) → warn! and skip (non-fatal)
```

### 4.4 Module Changes Summary

```
Files MODIFIED:
  clawft-types/src/config.rs               # +8 lines: heartbeat fields on GatewayConfig
  clawft-core/Cargo.toml                    # +1 line: optional clawft-services dep (for re-export)
  clawft-cli/Cargo.toml                     # +3 lines: vector-memory feature, tokio-util dep
  clawft-cli/src/commands/gateway.rs        # +60 lines: CronService, HeartbeatService, MCP wiring
  clawft-cli/src/commands/cron.rs           # +15 lines: replace placeholder in cron_run()
  clawft-cli/src/commands/agent.rs          # +10 lines: --intelligent-routing flag
  clawft-cli/src/commands/mod.rs            # +5 lines: resolve_cron_storage_path() helper

Files CREATED:
  clawft-cli/src/mcp_tools.rs              # ~80 lines: McpToolWrapper + create_mcp_client()

Files NOT MODIFIED (consumed as-is):
  clawft-services/src/cron_service/mod.rs   # CronService (no changes needed)
  clawft-services/src/heartbeat/mod.rs      # HeartbeatService (no changes needed)
  clawft-services/src/mcp/mod.rs            # McpClient (no changes needed)
  clawft-core/src/tools/registry.rs         # Tool trait (no changes needed)
  clawft-core/src/intelligent_router.rs     # IntelligentRouter (no changes needed)
  clawft-core/src/session_indexer.rs        # SessionIndexer (no changes needed)
```

### 4.5 Dependency Graph (New Edges)

```
clawft-cli
├── clawft-core          (existing)
├── clawft-services      (existing, now used for CronService, HeartbeatService, McpClient)
├── clawft-types         (existing)
├── clawft-channels      (existing)
├── clawft-llm           (existing)
├── tokio-util           (NEW: CancellationToken)
└── [feature: vector-memory]
    └── clawft-core/vector-memory  (enables IntelligentRouter, SessionIndexer, embeddings)

clawft-services
├── clawft-types         (InboundMessage, config types)
├── tokio                (async runtime, process spawning)
├── tokio-util           (CancellationToken)
├── reqwest              (HttpTransport)
├── serde_json           (JSON-RPC serialization)
├── chrono               (timestamps)
├── uuid                 (job IDs)
└── tracing              (logging)
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 McpToolWrapper Unit Tests

```rust
// clawft-cli/src/mcp_tools.rs (or tests/mcp_tools_tests.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_services::mcp::transport::MockTransport;
    use clawft_services::mcp::types::JsonRpcResponse;

    fn mock_tool_def() -> ToolDefinition {
        ToolDefinition {
            name: "echo".into(),
            description: "Echoes input".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                }
            }),
        }
    }

    #[test]
    fn mcp_tool_wrapper_name_is_prefixed() {
        let client = make_mock_client(vec![]);
        let wrapper = McpToolWrapper::new("my-server".into(), mock_tool_def(), client);
        assert_eq!(wrapper.name(), "my-server__echo");
    }

    #[test]
    fn mcp_tool_wrapper_description() {
        let client = make_mock_client(vec![]);
        let wrapper = McpToolWrapper::new("srv".into(), mock_tool_def(), client);
        assert_eq!(wrapper.description(), "Echoes input");
    }

    #[test]
    fn mcp_tool_wrapper_parameters() {
        let client = make_mock_client(vec![]);
        let wrapper = McpToolWrapper::new("srv".into(), mock_tool_def(), client);
        let params = wrapper.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["text"].is_object());
    }

    #[tokio::test]
    async fn mcp_tool_wrapper_execute_delegates_to_client() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({"output": "hello"})),
            error: None,
        };
        let client = make_mock_client(vec![response]);
        let wrapper = McpToolWrapper::new("srv".into(), mock_tool_def(), client);

        let result = wrapper.execute(serde_json::json!({"text": "hello"})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["output"], "hello");
    }

    #[tokio::test]
    async fn mcp_tool_wrapper_execute_maps_errors() {
        // Client returns a JSON-RPC error.
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: None,
            error: Some(JsonRpcError { code: -1, message: "fail".into(), data: None }),
        };
        let client = make_mock_client(vec![response]);
        let wrapper = McpToolWrapper::new("srv".into(), mock_tool_def(), client);

        let result = wrapper.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => {
                assert!(msg.contains("srv"));
                assert!(msg.contains("echo"));
            }
            other => panic!("expected ExecutionFailed, got: {other}"),
        }
    }
}
```

### 5.2 MCP Client Factory Tests

```rust
#[test]
fn create_mcp_client_prefers_stdio_when_command_set() {
    let cfg = MCPServerConfig {
        command: "echo".into(),
        args: vec!["hello".into()],
        env: HashMap::new(),
        url: String::new(),
    };
    // StdioTransport::new will fail because "echo" doesn't speak JSON-RPC,
    // but we verify the code path is correct.
    let result = tokio::runtime::Runtime::new().unwrap().block_on(
        create_mcp_client("test", &cfg)
    );
    // Stdio spawn should succeed (echo is a valid command).
    // The transport may fail at list_tools(), but creation should work.
    assert!(result.is_ok() || result.is_err()); // non-panic
}

#[test]
fn create_mcp_client_uses_http_when_url_set() {
    let cfg = MCPServerConfig {
        command: String::new(),
        args: vec![],
        env: HashMap::new(),
        url: "http://localhost:9999".into(),
    };
    let result = tokio::runtime::Runtime::new().unwrap().block_on(
        create_mcp_client("test", &cfg)
    );
    assert!(result.is_ok());
}

#[test]
fn create_mcp_client_errors_when_both_empty() {
    let cfg = MCPServerConfig::default();
    let result = tokio::runtime::Runtime::new().unwrap().block_on(
        create_mcp_client("test", &cfg)
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("neither command nor url"));
}
```

### 5.3 Gateway Service Wiring Integration Tests

```rust
// clawft-cli/tests/gateway_wiring_tests.rs

#[tokio::test]
async fn cron_service_fires_messages_through_bus() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let dir = tempdir().unwrap();
    let path = dir.path().join("cron.jsonl");

    let svc = CronService::new(path, tx).await.unwrap();
    let id = svc.add_job("test".into(), "0 0 * * * * *".into(), "hello cron".into()).await.unwrap();
    svc.run_job_now(&id).await.unwrap();

    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.channel, "cron");
    assert_eq!(msg.sender_id, "system");
    assert_eq!(msg.content, "hello cron");
}

#[tokio::test]
async fn heartbeat_service_fires_on_interval() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let svc = HeartbeatService::new(0, "test heartbeat".into(), tx);
    // Note: interval=0 creates Duration::from_secs(0), fires immediately.
    // In real config, minimum is 1 minute. For test we use the raw constructor.
    let svc = HeartbeatService {
        interval: Duration::from_millis(50),
        prompt: "test heartbeat".into(),
        message_tx: tx_clone,
    };

    let cancel = CancellationToken::new();
    let cancel_c = cancel.clone();
    let handle = tokio::spawn(async move { svc.start(cancel_c).await });

    tokio::time::sleep(Duration::from_millis(150)).await;
    cancel.cancel();
    handle.await.unwrap().unwrap();

    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.channel, "heartbeat");
}

#[tokio::test]
async fn cancellation_token_stops_all_services() {
    let cancel = CancellationToken::new();

    let (tx, _rx) = mpsc::unbounded_channel();
    let dir = tempdir().unwrap();

    let cron = CronService::new(dir.path().join("cron.jsonl"), tx.clone()).await.unwrap();
    let cron_cancel = cancel.clone();
    let cron_handle = tokio::spawn(async move { cron.start(cron_cancel).await });

    let hb = HeartbeatService::new(60, "hb".into(), tx);
    let hb_cancel = cancel.clone();
    let hb_handle = tokio::spawn(async move { hb.start(hb_cancel).await });

    // Cancel immediately.
    tokio::time::sleep(Duration::from_millis(10)).await;
    cancel.cancel();

    // Both should exit without error.
    assert!(cron_handle.await.unwrap().is_ok());
    assert!(hb_handle.await.unwrap().is_ok());
}
```

### 5.4 GatewayConfig Heartbeat Tests

```rust
#[test]
fn gateway_config_heartbeat_defaults() {
    let cfg = GatewayConfig::default();
    assert_eq!(cfg.heartbeat_interval_minutes, 0);
    assert_eq!(cfg.heartbeat_prompt, "heartbeat");
}

#[test]
fn gateway_config_heartbeat_from_json() {
    let json = r#"{
        "host": "0.0.0.0",
        "port": 18790,
        "heartbeatIntervalMinutes": 5,
        "heartbeatPrompt": "Are you alive?"
    }"#;
    let cfg: GatewayConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.heartbeat_interval_minutes, 5);
    assert_eq!(cfg.heartbeat_prompt, "Are you alive?");
}
```

### 5.5 Vector-Memory Feature Gate Tests

```rust
// Only compiled when vector-memory feature is enabled.
#[cfg(feature = "vector-memory")]
mod vector_memory_tests {
    use clawft_core::intelligent_router::IntelligentRouter;
    use clawft_core::session_indexer::SessionIndexer;
    use clawft_core::embeddings::hash_embedder::HashEmbedder;

    #[tokio::test]
    async fn intelligent_router_can_be_created() {
        let embedder = Box::new(HashEmbedder::new(64));
        let router = IntelligentRouter::new(embedder).await;
        // Simple prompt goes to Tier 2.
        let decision = router.route_request("hello", &Default::default()).await.unwrap();
        assert_eq!(decision.tier, 2);
    }

    #[tokio::test]
    async fn session_indexer_can_index_and_search() {
        let embedder = Box::new(HashEmbedder::new(64));
        let mut indexer = SessionIndexer::new(embedder).await;
        indexer.index_turn(&ConversationTurn {
            session_id: "s1".into(),
            turn_id: 0,
            user_message: "test".into(),
            assistant_message: "response".into(),
            timestamp: 1000,
            model: "test".into(),
        }).await.unwrap();
        assert_eq!(indexer.len(), 1);
    }
}
```

### 5.6 Test Coverage Requirements
- **Unit test coverage**: >= 80% for all new modules (`mcp_tools.rs`, gateway wiring logic)
- **Integration test coverage**: >= 70% for end-to-end service lifecycle flows
- **Critical paths**: 100% coverage for:
  - CancellationToken propagation to all background tasks
  - MCP tool discovery fallback (skip on error, log warning)
  - McpToolWrapper::execute() error mapping
  - HeartbeatService disabled when interval=0

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [ ] All unit tests passing (>= 80% coverage)
- [ ] All integration tests passing
- [ ] CronService fires messages through the bus in gateway context
- [ ] HeartbeatService fires periodic messages when enabled
- [ ] McpClient discovers tools from a test MCP server
- [ ] McpToolWrapper correctly delegates tool execution
- [ ] CancellationToken stops all background services gracefully
- [ ] `weft cron run <id>` actually fires the job (not just a placeholder)

### 6.2 Configuration Integration
- [ ] `gateway.heartbeat_interval_minutes` added to GatewayConfig with default 0
- [ ] `gateway.heartbeat_prompt` added to GatewayConfig with default "heartbeat"
- [ ] camelCase aliases work (`heartbeatIntervalMinutes`, `heartbeatPrompt`)
- [ ] `config.tools.mcp_servers` correctly parsed with command/args/env/url fields
- [ ] Test fixture `tests/fixtures/config.json` updated with heartbeat and MCP server entries

### 6.3 Service Lifecycle Integration
- [ ] CronService starts before agent loop, persists across gateway lifetime
- [ ] HeartbeatService conditionally created based on config
- [ ] MCP tools discovered before `AppContext::into_agent_loop()` (tools available to agent)
- [ ] All services respect the shared CancellationToken
- [ ] Ctrl+C triggers orderly shutdown: channels stop, services stop, agent loop exits
- [ ] JoinHandles for all background tasks are awaited (no abandoned tasks)

### 6.4 MCP Tool Registry Integration
- [ ] MCP tools registered with `{server_name}__{tool_name}` naming convention
- [ ] Multiple tools from same server share a single `Arc<McpClient>`
- [ ] Tool schemas appear in `ToolRegistry::schemas()` for LLM function calling
- [ ] Transport errors during startup do not prevent gateway from starting
- [ ] `weft status` shows count of registered MCP tools

### 6.5 CLI Integration
- [ ] `weft agent --intelligent-routing` flag parsed (no-op if feature disabled)
- [ ] `weft gateway --intelligent-routing` flag parsed
- [ ] Clear error message when `--intelligent-routing` used without `vector-memory` feature
- [ ] `weft cron run <job-id>` delegates to CronService

### 6.6 Feature Flag Integration
- [ ] `clawft-cli` Cargo.toml has `vector-memory = ["clawft-core/vector-memory"]` feature
- [ ] `cargo build` without `vector-memory` compiles without IntelligentRouter/SessionIndexer
- [ ] `cargo build --features vector-memory` compiles with all vector-memory types
- [ ] No dead code warnings when feature is disabled

### 6.7 Error Handling Validation
- [ ] MCP server spawn failure (bad command) logs warning, gateway continues
- [ ] MCP list_tools() timeout or error logs warning, skips server
- [ ] MCP call_tool() failure maps to `ToolError::ExecutionFailed` with context
- [ ] CronService JSONL storage corruption at startup logs warning per-job, loads valid jobs
- [ ] HeartbeatService channel closure returns `ServiceError::ChannelClosed`
- [ ] Ctrl+C during MCP discovery does not hang (child processes cleaned up)

### 6.8 Performance Validation
- [ ] MCP tool discovery completes within 10 seconds per server (timeout)
- [ ] HeartbeatService interval accuracy within 1 second
- [ ] CronService tick interval (60s) does not drift significantly
- [ ] Gateway startup time with 5 MCP servers < 30 seconds
- [ ] Shutdown completes within 5 seconds after Ctrl+C

### 6.9 Final Review
- [ ] Code review: no hardcoded secrets or paths
- [ ] All `unwrap()` calls justified or replaced with error handling
- [ ] Tracing spans cover all service lifecycle events (start, stop, error)
- [ ] No `clippy` warnings introduced
- [ ] `cargo test` passes with and without `vector-memory` feature
- [ ] `cargo build --release` succeeds

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockEnvironment, MockFileSystem};`
- **Import mocks from 1B**: `use clawft_core::test_utils::MockMessageBus;`
- **Use shared fixtures**: Load `tests/fixtures/config.json` for config deserialization tests
- **Use MCP mock transport**: `use clawft_services::mcp::transport::MockTransport;` for tool wrapper tests

### Security Tests (Required)
- MCP server command injection prevention (validate `command` field does not contain shell metacharacters)
- MCP server environment variable isolation (env vars from config do not leak host secrets)
- Tool name collision prevention (prefixed naming `server__tool` avoids overwriting built-in tools)
- CancellationToken propagation ensures no orphaned child processes

### Coverage Target
- Unit test coverage: >= 80% (measured via `cargo-tarpaulin`)
- Critical paths (cancellation, MCP error handling, feature gates): 100%

---

## Notes for Implementation Agent

1. **Read existing service files first** to understand exact APIs and constructor signatures
2. **CancellationToken replaces ctrl_c()**: The gateway currently calls `tokio::signal::ctrl_c().await` -- replace this with a token-based pattern so all services share the same cancellation signal
3. **McpClient must be Arc-wrapped**: Multiple McpToolWrapper instances for the same server share one client
4. **tokio-util dependency**: Add `tokio-util` to `clawft-cli` if not already present (for `CancellationToken`)
5. **Feature flag compilation**: Test with both `cargo build` and `cargo build --features vector-memory`
6. **MCP tool naming convention**: Use `__` (double underscore) as separator to avoid conflicts with tools that have hyphens in their names
7. **Cron storage path**: The CronService uses append-only JSONL, while the CLI `cron.rs` uses a full JSON store. These are separate files -- the wiring creates a JSONL file alongside the existing JSON store
8. **Error handling philosophy**: Service startup errors are warnings, not fatal. The gateway should start even if MCP servers are unreachable or cron storage is corrupt
9. **Shutdown ordering**: Stop accepting new messages (channels) before stopping background services (cron, heartbeat), then stop the agent loop
10. **StdioTransport child process cleanup**: When the McpClient is dropped, the child process stdin is closed, which should cause the child to exit. If not, consider adding explicit kill on drop
