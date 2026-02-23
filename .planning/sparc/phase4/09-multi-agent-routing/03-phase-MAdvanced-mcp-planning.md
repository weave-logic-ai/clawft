# Phase M-Advanced: Dynamic MCP + Bidirectional Bridge + Planning Strategies

> **Element:** 09 -- Multi-Agent Routing & Claude Flow Integration
> **Phase:** M-Advanced (M4, M5, L4, M6)
> **Timeline:** Week 6-9
> **Priority:** M4/M5: P1 (Week 6-8), L4: P2 (Week 8-9), M6: P1 (Week 8)
> **Crates:** `clawft-services` (extends `src/mcp/`), `clawft-cli`, `clawft-core`
> **Dependencies IN:** F9a (Core MCP client -- `McpClient`/`McpSession` in `mod.rs`), C6 (MCP server for skills), D6 (`sender_id` for cost tracking), D9 (MCP transport)
> **Blocks:** None directly. L4 is post-MVP.
> **Status:** Planning

---

## 1. Overview

Phase M-Advanced delivers four workstream items that extend clawft's MCP integration from static tool serving to dynamic, runtime-managed multi-server orchestration:

1. **M4: Dynamic MCP Server Discovery** -- Runtime management of external MCP servers via CLI (`weft mcp add/list/remove`) and config-driven hot-reload with drain-and-swap protocol.

2. **M5: Bidirectional MCP Bridge** -- Enables clawft to both expose tools to Claude Code (outbound, already implemented via `McpServerShell`) and consume tools from Claude Code (inbound, via `claude mcp serve`).

3. **L4: Planning Strategies** -- ReAct (Reason+Act) and Plan-and-Execute reasoning strategies for the agent router, with configurable guard rails to prevent infinite loops and cost runaway. Post-MVP.

4. **M6: Delegation Config Documentation** -- User-facing guides for delegation configuration and MCP bridge setup.

---

## 2. Specification

### 2.1 M4: Dynamic MCP Server Discovery

#### 2.1.1 McpServerManager

The central runtime manager for external MCP servers. Lives in a new file `crates/clawft-services/src/mcp/discovery.rs`.

```rust
// crates/clawft-services/src/mcp/discovery.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use super::{McpSession, ToolDefinition};
use super::transport::{StdioTransport, HttpTransport, McpTransport};

/// Status of a managed MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    /// Connected and accepting tool calls.
    Active,
    /// Draining: completing in-flight calls, rejecting new ones.
    Draining,
    /// Disconnected or failed to connect.
    Disconnected,
}

/// Configuration for a single MCP server connection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    /// Display name (also used as the namespace key).
    pub name: String,
    /// Connection target:
    /// - Starts with "/" or contains no "://" => stdio command
    /// - Starts with "http://" or "https://" => HTTP transport
    pub target: String,
    /// Arguments for stdio command (ignored for HTTP).
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables for stdio child process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Request timeout in milliseconds (default: 30000).
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 { 30_000 }

/// A managed MCP server with session and status tracking.
pub struct ManagedMcpServer {
    pub name: String,
    pub config: McpServerConfig,
    pub session: McpSession,
    pub status: ServerStatus,
    pub tools: Vec<ToolDefinition>,
    /// Number of in-flight tool calls (for drain tracking).
    pub inflight: Arc<std::sync::atomic::AtomicU32>,
}

/// Runtime manager for external MCP server connections.
///
/// Provides add/remove/list operations and hot-reload support.
/// Each server's tools are namespaced as `mcp:<server-name>:<tool-name>`
/// in the tool registry.
pub struct McpServerManager {
    servers: Arc<RwLock<HashMap<String, ManagedMcpServer>>>,
}
```

Key behaviors:
- `add_server()` connects via the appropriate transport (stdio or HTTP), performs the MCP initialize handshake via `McpSession::connect()`, fetches `tools/list`, and registers the server.
- `remove_server()` initiates the drain-and-swap protocol (Section 2.1.3).
- `list_servers()` returns status and tool count for each managed server.

#### 2.1.2 Server Connection Factory

Determines transport type from the `target` field in `McpServerConfig`:

```rust
// crates/clawft-services/src/mcp/discovery.rs

impl McpServerManager {
    /// Connect to an MCP server based on its configuration.
    ///
    /// Transport selection:
    /// - target starts with "http://" or "https://" => HttpTransport
    /// - otherwise => StdioTransport (target is the command)
    async fn connect(config: &McpServerConfig) -> Result<McpSession, DiscoveryError> {
        let transport: Box<dyn McpTransport> = if config.target.starts_with("http://")
            || config.target.starts_with("https://")
        {
            // Validate URL (SSRF protection)
            validate_mcp_url(&config.target)?;
            Box::new(HttpTransport::new(config.target.clone()))
        } else {
            // Validate command path exists
            validate_command_path(&config.target)?;
            Box::new(
                StdioTransport::new(&config.target, &config.args, &config.env)
                    .await
                    .map_err(|e| DiscoveryError::Connection(e.to_string()))?,
            )
        };

        McpSession::connect(transport)
            .await
            .map_err(|e| DiscoveryError::Handshake(e.to_string()))
    }
}
```

#### 2.1.3 Drain-and-Swap Protocol

When a server is removed or its config changes, the manager uses drain-and-swap to avoid dropping in-flight calls:

```
1. Mark server status = Draining
2. Reject new tool calls to this server (return ToolError::ServerDraining)
3. Wait for inflight.load() == 0, with 30-second timeout
4. If timeout: log warning, force-disconnect
5. Disconnect session (drop transport)
6. Remove from servers map
7. Remove tools from ToolRegistry
```

```rust
// crates/clawft-services/src/mcp/discovery.rs

/// Drain timeout for in-flight calls when removing a server.
const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl McpServerManager {
    /// Remove a server with drain-and-swap protocol.
    ///
    /// In-flight calls are given up to 30 seconds to complete.
    /// New calls are rejected immediately once draining starts.
    pub async fn remove_server(&self, name: &str) -> Result<(), DiscoveryError> {
        // Step 1: Mark as draining
        {
            let mut servers = self.servers.write().await;
            let server = servers
                .get_mut(name)
                .ok_or_else(|| DiscoveryError::NotFound(name.to_string()))?;
            server.status = ServerStatus::Draining;
            info!(server = name, "MCP server draining started");
        }

        // Step 2: Wait for in-flight calls to complete
        let inflight = {
            let servers = self.servers.read().await;
            servers.get(name).map(|s| s.inflight.clone())
        };

        if let Some(inflight) = inflight {
            let deadline = tokio::time::Instant::now() + DRAIN_TIMEOUT;
            loop {
                if inflight.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    warn!(
                        server = name,
                        remaining = inflight.load(std::sync::atomic::Ordering::Relaxed),
                        "drain timeout, force-disconnecting"
                    );
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        // Step 3: Remove from map (session is dropped, transport closed)
        {
            let mut servers = self.servers.write().await;
            servers.remove(name);
        }

        info!(server = name, "MCP server removed");
        Ok(())
    }
}
```

#### 2.1.4 Tool Call Routing with Inflight Tracking

When routing a tool call to an external MCP server, the inflight counter is incremented before the call and decremented after (in all code paths):

```rust
// crates/clawft-services/src/mcp/discovery.rs

impl McpServerManager {
    /// Call a tool on a managed MCP server.
    ///
    /// Returns an error if the server is draining or disconnected.
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, DiscoveryError> {
        let (session_ref, inflight) = {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| DiscoveryError::NotFound(server_name.to_string()))?;

            if server.status == ServerStatus::Draining {
                return Err(DiscoveryError::ServerDraining(server_name.to_string()));
            }
            if server.status == ServerStatus::Disconnected {
                return Err(DiscoveryError::ServerDisconnected(server_name.to_string()));
            }

            // Increment inflight BEFORE the call
            server.inflight.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // We need the session -- but McpSession is not Clone.
            // We use a shared reference pattern (see Architecture section).
            (server_name.to_string(), server.inflight.clone())
        };

        // Decrement inflight on all exit paths
        let _guard = InFlightGuard(inflight);

        let servers = self.servers.read().await;
        let server = servers
            .get(&session_ref)
            .ok_or_else(|| DiscoveryError::NotFound(session_ref.clone()))?;

        server
            .session
            .call_tool(tool_name, params)
            .await
            .map_err(|e| DiscoveryError::ToolCall(e.to_string()))
    }
}

/// RAII guard that decrements the inflight counter on drop.
struct InFlightGuard(Arc<std::sync::atomic::AtomicU32>);

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
```

#### 2.1.5 CLI Commands

Three new subcommands under `weft mcp`:

**File:** `crates/clawft-cli/src/commands/mcp_cmd.rs` (NEW)

```rust
// crates/clawft-cli/src/commands/mcp_cmd.rs

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Subcommand)]
pub enum McpCommand {
    /// Add an MCP server connection.
    Add(McpAddArgs),
    /// List connected MCP servers.
    List,
    /// Remove an MCP server connection.
    Remove(McpRemoveArgs),
}

#[derive(Args)]
pub struct McpAddArgs {
    /// Server name (used as namespace key).
    pub name: String,
    /// Command or URL to connect to.
    /// - Command path => stdio transport
    /// - http(s) URL => HTTP transport
    pub target: String,
    /// Additional arguments for stdio command.
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

#[derive(Args)]
pub struct McpRemoveArgs {
    /// Name of the server to remove.
    pub name: String,
}
```

CLI usage:
```bash
weft mcp add github /usr/local/bin/gh-mcp-server
weft mcp add my-api https://api.example.com/mcp
weft mcp list
weft mcp remove github
```

#### 2.1.6 Config-Driven Hot-Reload

File watcher on `clawft.toml` triggers hot-reload when the `[tools.mcp_servers]` section changes:

```toml
# clawft.toml
[tools.mcp_servers.github]
target = "/usr/local/bin/gh-mcp-server"
args = ["--token-env", "GITHUB_TOKEN"]

[tools.mcp_servers.filesystem]
target = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
```

Hot-reload algorithm:
1. File watcher detects `clawft.toml` change; debounce 500ms.
2. Parse new `[tools.mcp_servers]` section.
3. Diff against current `McpServerManager` state.
4. **New servers** (in new config, not in manager): call `add_server()`.
5. **Removed servers** (in manager, not in new config): call `remove_server()` (drain-and-swap).
6. **Changed servers** (same name, different config): `remove_server()` then `add_server()`.
7. **Unchanged servers**: no action.

```rust
// crates/clawft-services/src/mcp/discovery.rs

impl McpServerManager {
    /// Apply a config diff from hot-reload.
    ///
    /// New servers are connected, removed servers are drained,
    /// changed servers are replaced (remove + add).
    pub async fn apply_config_diff(
        &self,
        new_configs: HashMap<String, McpServerConfig>,
    ) -> Vec<DiscoveryError> {
        let mut errors = Vec::new();
        let current_names: Vec<String> = {
            let servers = self.servers.read().await;
            servers.keys().cloned().collect()
        };
        let new_names: std::collections::HashSet<String> =
            new_configs.keys().cloned().collect();

        // Remove servers no longer in config
        for name in &current_names {
            if !new_names.contains(name) {
                if let Err(e) = self.remove_server(name).await {
                    errors.push(e);
                }
            }
        }

        // Add or update servers
        for (name, config) in new_configs {
            let needs_update = {
                let servers = self.servers.read().await;
                match servers.get(&name) {
                    None => true, // New server
                    Some(existing) => {
                        // Changed if target or args differ
                        existing.config.target != config.target
                            || existing.config.args != config.args
                    }
                }
            };

            if needs_update {
                // Remove existing if present (changed server)
                if current_names.contains(&name) {
                    if let Err(e) = self.remove_server(&name).await {
                        errors.push(e);
                        continue;
                    }
                }
                // Add new/updated server
                if let Err(e) = self.add_server(&name, config).await {
                    errors.push(e);
                }
            }
        }

        errors
    }
}
```

---

### 2.2 M5: Bidirectional MCP Bridge

#### 2.2.1 Bridge Topology

```
                    +-----------+
  Claude Code  <----|  clawft   |----> External MCP Servers
  (MCP Client)     | MCP Bridge|      (github, filesystem, etc.)
  Claude Code  ---->|           |
  (MCP Server)     +-----------+

  Outbound (clawft -> Claude Code):
    clawft runs McpServerShell on stdio.
    Claude Code connects as MCP client via `claude mcp add`.
    Already implemented in server.rs + mcp_server.rs.

  Inbound (Claude Code -> clawft):
    Claude Code runs `claude mcp serve` as an MCP server.
    clawft connects as MCP client via McpSession.
    New: bridge.rs orchestrates the inbound connection.
```

#### 2.2.2 Outbound Bridge (clawft -> Claude Code)

Already functional. The existing `weft mcp-server` command (`crates/clawft-cli/src/commands/mcp_server.rs`) starts an `McpServerShell` on stdin/stdout. Claude Code connects with:

```bash
claude mcp add clawft -- weft mcp-server
```

No new code required for outbound. M5 documents this workflow and verifies it works end-to-end.

#### 2.2.3 Inbound Bridge (Claude Code -> clawft)

New file: `crates/clawft-services/src/mcp/bridge.rs`

clawft connects to Claude Code's MCP server to access Claude Code's tools (file editing, bash execution, etc.):

```rust
// crates/clawft-services/src/mcp/bridge.rs

use std::collections::HashMap;
use tracing::{info, warn};

use super::{McpSession, ToolDefinition};
use super::transport::StdioTransport;

/// Bidirectional MCP bridge between clawft and Claude Code.
///
/// Manages the inbound connection (clawft as MCP client connecting
/// to Claude Code's MCP server) and coordinates tool namespacing.
pub struct McpBridge {
    /// Inbound session: clawft -> Claude Code MCP server.
    inbound_session: Option<McpSession>,
    /// Tools available from Claude Code.
    claude_tools: Vec<ToolDefinition>,
}

impl McpBridge {
    pub fn new() -> Self {
        Self {
            inbound_session: None,
            claude_tools: Vec::new(),
        }
    }

    /// Connect to Claude Code's MCP server.
    ///
    /// Spawns `claude mcp serve` as a child process and performs
    /// the MCP initialize handshake.
    pub async fn connect_to_claude_code(
        &mut self,
    ) -> Result<(), BridgeError> {
        let env = HashMap::new();
        let transport = StdioTransport::new(
            "claude",
            &["mcp".to_string(), "serve".to_string()],
            &env,
        )
        .await
        .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        let session = McpSession::connect(Box::new(transport))
            .await
            .map_err(|e| BridgeError::HandshakeFailed(e.to_string()))?;

        info!(
            server = session.server_info.name,
            version = session.server_info.version,
            "connected to Claude Code MCP server"
        );

        // Fetch available tools
        let tools = session
            .list_tools()
            .await
            .map_err(|e| BridgeError::ToolListFailed(e.to_string()))?;

        info!(tool_count = tools.len(), "discovered Claude Code tools");
        self.claude_tools = tools;
        self.inbound_session = Some(session);

        Ok(())
    }

    /// Call a tool on Claude Code's MCP server.
    pub async fn call_claude_tool(
        &self,
        name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, BridgeError> {
        let session = self
            .inbound_session
            .as_ref()
            .ok_or(BridgeError::NotConnected)?;

        session
            .call_tool(name, params)
            .await
            .map_err(|e| BridgeError::ToolCallFailed(e.to_string()))
    }

    /// List tools available from Claude Code.
    pub fn claude_tools(&self) -> &[ToolDefinition] {
        &self.claude_tools
    }

    /// Check if the bridge is connected.
    pub fn is_connected(&self) -> bool {
        self.inbound_session.is_some()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("connection to Claude Code failed: {0}")]
    ConnectionFailed(String),

    #[error("MCP handshake with Claude Code failed: {0}")]
    HandshakeFailed(String),

    #[error("failed to list Claude Code tools: {0}")]
    ToolListFailed(String),

    #[error("Claude Code MCP bridge not connected")]
    NotConnected,

    #[error("tool call failed: {0}")]
    ToolCallFailed(String),
}
```

#### 2.2.4 Bridge Configuration in clawft.toml

```toml
# clawft.toml -- Bridge configuration
[tools.mcp_servers.claude-code]
target = "claude"
args = ["mcp", "serve"]
# No env vars needed -- claude CLI handles its own auth
```

This leverages the same `McpServerManager` from M4. The bridge is just a specific MCP server entry with `claude mcp serve` as the stdio command. The `McpBridge` struct in `bridge.rs` provides a higher-level abstraction for the specific Claude Code integration patterns (tool filtering, delegation routing).

#### 2.2.5 Recursive Delegation Guard

When clawft connects to Claude Code as both client and server, there is a risk of recursive delegation loops (clawft -> Claude Code -> clawft -> ...). Protection:

```rust
// crates/clawft-services/src/mcp/bridge.rs

/// Maximum delegation depth to prevent recursive loops.
const MAX_DELEGATION_DEPTH: u32 = 3;

/// Header key for tracking delegation depth in MCP calls.
const DEPTH_HEADER: &str = "x-clawft-delegation-depth";

impl McpBridge {
    /// Call a tool with delegation depth tracking.
    pub async fn call_claude_tool_with_depth(
        &self,
        name: &str,
        mut params: serde_json::Value,
        current_depth: u32,
    ) -> Result<serde_json::Value, BridgeError> {
        if current_depth >= MAX_DELEGATION_DEPTH {
            return Err(BridgeError::ToolCallFailed(format!(
                "delegation depth limit reached ({MAX_DELEGATION_DEPTH})"
            )));
        }

        // Inject depth counter into params metadata
        if let Some(obj) = params.as_object_mut() {
            obj.insert(
                "_clawft_depth".to_string(),
                serde_json::Value::Number((current_depth + 1).into()),
            );
        }

        self.call_claude_tool(name, params).await
    }
}
```

---

### 2.3 L4: Planning Guard Rails

#### 2.3.1 Planning Strategies

Two reasoning strategies for the agent router, both post-MVP:

```rust
// crates/clawft-core/src/agent/planning.rs (NEW)

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Available planning strategies for the agent router.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    /// ReAct: interleaved Reasoning and Acting.
    /// Each step: Thought -> Action -> Observation -> repeat.
    ReAct,
    /// Plan-and-Execute: generate full plan first, then execute steps.
    /// Plan -> [Step1, Step2, ...] -> Execute each -> Replan if needed.
    PlanAndExecute,
}

impl Default for PlanningStrategy {
    fn default() -> Self {
        Self::ReAct
    }
}
```

#### 2.3.2 Planning Configuration

```rust
// crates/clawft-core/src/agent/planning.rs

/// Configuration for planning guard rails.
///
/// All limits are configurable in `clawft.toml` under `[router.planning]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConfig {
    /// Which planning strategy to use.
    #[serde(default)]
    pub strategy: PlanningStrategy,

    /// Maximum number of planning steps before forced termination.
    #[serde(default = "default_max_depth")]
    pub max_planning_depth: u32,

    /// Hard budget cap for LLM calls during planning (USD).
    #[serde(default = "default_max_cost")]
    pub max_planning_cost_usd: f64,

    /// Maximum duration for a single planning step.
    #[serde(default = "default_step_timeout")]
    pub planning_step_timeout: Duration,

    /// Consecutive no-op steps before circuit breaker triggers.
    #[serde(default = "default_circuit_breaker")]
    pub circuit_breaker_threshold: u32,
}

fn default_max_depth() -> u32 { 10 }
fn default_max_cost() -> f64 { 1.0 }
fn default_step_timeout() -> Duration { Duration::from_secs(60) }
fn default_circuit_breaker() -> u32 { 3 }

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            strategy: PlanningStrategy::default(),
            max_planning_depth: default_max_depth(),
            max_planning_cost_usd: default_max_cost(),
            planning_step_timeout: default_step_timeout(),
            circuit_breaker_threshold: default_circuit_breaker(),
        }
    }
}
```

#### 2.3.3 Planning Router

```rust
// crates/clawft-core/src/agent/planning.rs

/// Result of a planning execution, including partial results.
pub struct PlanningResult {
    /// Steps that were executed successfully.
    pub completed_steps: Vec<PlanningStep>,
    /// Whether the plan completed fully or was terminated early.
    pub termination_reason: Option<TerminationReason>,
    /// Total estimated cost of LLM calls during planning.
    pub total_cost_usd: f64,
    /// Final output (may be partial if terminated early).
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct PlanningStep {
    pub step_number: u32,
    pub description: String,
    pub action_taken: String,
    pub observation: String,
    pub cost_usd: f64,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReason {
    /// Completed naturally (end_turn or plan exhausted).
    Complete,
    /// Hit max_planning_depth.
    DepthLimit,
    /// Hit max_planning_cost_usd.
    BudgetExhausted,
    /// A single step exceeded planning_step_timeout.
    StepTimeout,
    /// Circuit breaker: N consecutive no-op steps.
    CircuitBreaker { consecutive_noops: u32 },
}

/// The planning router executes a task using the configured strategy
/// with guard rail enforcement.
pub struct PlanningRouter {
    config: PlanningConfig,
}

impl PlanningRouter {
    pub fn new(config: PlanningConfig) -> Self {
        Self { config }
    }

    /// Execute a task using the configured planning strategy.
    ///
    /// Enforces all guard rails (depth, cost, timeout, circuit breaker).
    /// Returns partial results with explanation if any limit is hit.
    pub async fn execute(
        &self,
        task: &str,
        // Tool executor callback -- provided by the agent loop
        tool_executor: &dyn ToolExecutor,
    ) -> PlanningResult {
        match self.config.strategy {
            PlanningStrategy::ReAct => self.execute_react(task, tool_executor).await,
            PlanningStrategy::PlanAndExecute => {
                self.execute_plan_and_execute(task, tool_executor).await
            }
        }
    }
}
```

#### 2.3.4 Guard Rail Enforcement

Each planning step is wrapped in guard rail checks:

```rust
// crates/clawft-core/src/agent/planning.rs

impl PlanningRouter {
    /// Check all guard rails before executing the next step.
    ///
    /// Returns Some(TerminationReason) if any limit is hit.
    fn check_guard_rails(
        &self,
        step_number: u32,
        total_cost: f64,
        consecutive_noops: u32,
    ) -> Option<TerminationReason> {
        if step_number >= self.config.max_planning_depth {
            return Some(TerminationReason::DepthLimit);
        }
        if total_cost >= self.config.max_planning_cost_usd {
            return Some(TerminationReason::BudgetExhausted);
        }
        if consecutive_noops >= self.config.circuit_breaker_threshold {
            return Some(TerminationReason::CircuitBreaker {
                consecutive_noops,
            });
        }
        None
    }

    /// Execute one planning step with timeout enforcement.
    async fn execute_step_with_timeout(
        &self,
        step_fn: impl std::future::Future<Output = PlanningStep>,
    ) -> Result<PlanningStep, TerminationReason> {
        match tokio::time::timeout(self.config.planning_step_timeout, step_fn).await {
            Ok(step) => Ok(step),
            Err(_) => Err(TerminationReason::StepTimeout),
        }
    }
}
```

#### 2.3.5 ReAct Loop Implementation

```rust
// crates/clawft-core/src/agent/planning.rs

impl PlanningRouter {
    async fn execute_react(
        &self,
        task: &str,
        tool_executor: &dyn ToolExecutor,
    ) -> PlanningResult {
        let mut steps = Vec::new();
        let mut total_cost = 0.0;
        let mut consecutive_noops = 0u32;
        let mut output = String::new();

        for step_num in 0..self.config.max_planning_depth {
            // Check guard rails
            if let Some(reason) = self.check_guard_rails(
                step_num, total_cost, consecutive_noops
            ) {
                return PlanningResult {
                    completed_steps: steps,
                    termination_reason: Some(reason),
                    total_cost_usd: total_cost,
                    output,
                };
            }

            // Execute step: Thought -> Action -> Observation
            let step_result = self.execute_step_with_timeout(async {
                // 1. Generate thought (LLM call)
                // 2. Determine action (tool call or finish)
                // 3. Execute action via tool_executor
                // 4. Observe result
                // Returns PlanningStep with cost tracking
                todo!("implementation requires LLM integration")
            }).await;

            match step_result {
                Ok(step) => {
                    if step.action_taken == "noop" || step.observation.is_empty() {
                        consecutive_noops += 1;
                    } else {
                        consecutive_noops = 0;
                    }
                    total_cost += step.cost_usd;
                    output = step.observation.clone();
                    steps.push(step);
                }
                Err(reason) => {
                    return PlanningResult {
                        completed_steps: steps,
                        termination_reason: Some(reason),
                        total_cost_usd: total_cost,
                        output,
                    };
                }
            }
        }

        PlanningResult {
            completed_steps: steps,
            termination_reason: Some(TerminationReason::Complete),
            total_cost_usd: total_cost,
            output,
        }
    }
}
```

#### 2.3.6 Configuration in clawft.toml

```toml
# clawft.toml -- Planning configuration (post-MVP)
[router.planning]
strategy = "react"            # "react" or "plan_and_execute"
max_planning_depth = 10       # Maximum steps
max_planning_cost_usd = 1.0   # Hard budget cap
planning_step_timeout = "60s" # Per-step timeout
circuit_breaker_threshold = 3 # Consecutive no-ops before abort
```

#### 2.3.7 Cost Tracking Integration

L4 planning cost tracking uses D6's `sender_id` to attribute costs:

```rust
// crates/clawft-core/src/agent/planning.rs

/// Track cost for a planning step.
///
/// Integrates with D6 sender_id for per-agent cost attribution.
fn track_step_cost(
    sender_id: &str,
    step: &PlanningStep,
    model: &str,
) {
    // Emit cost event for the pipeline's cost tracking system
    tracing::info!(
        sender_id = sender_id,
        model = model,
        cost_usd = step.cost_usd,
        step = step.step_number,
        "planning step cost"
    );
}
```

---

### 2.4 M6: Delegation Config Documentation

Two documentation deliverables:

#### 2.4.1 docs/guides/configuration.md -- Delegation Section

Add a "Delegation & Multi-Agent" section covering:
- `[delegation]` TOML section (model, max_turns, max_tokens, excluded_tools)
- `[delegation.flow]` for Claude Flow integration (M1)
- `[tools.mcp_servers.*]` for dynamic MCP servers (M4)
- `[router.planning]` for planning strategies (L4)
- Per-agent delegation overrides

#### 2.4.2 docs/guides/tool-calls.md -- MCP Bridge Section

Add an "MCP Bridge Setup" section covering:
- Outbound: `weft mcp-server` + `claude mcp add clawft -- weft mcp-server`
- Inbound: Configure `[tools.mcp_servers.claude-code]` in clawft.toml
- Bidirectional: both simultaneously
- Recursive delegation protection

---

## 3. Pseudocode

### 3.1 M4: `weft mcp add` Flow

```
FUNCTION weft_mcp_add(name, target, args):
    config = McpServerConfig { name, target, args, env: {}, timeout_ms: 30000 }

    IF target starts with "http://" or "https://":
        validate_mcp_url(target)
        transport = HttpTransport::new(target)
    ELSE:
        validate_command_path(target)
        transport = StdioTransport::new(target, args, env)

    session = McpSession::connect(transport)
    tools = session.list_tools()

    server = ManagedMcpServer {
        name, config, session,
        status: Active,
        tools,
        inflight: AtomicU32::new(0),
    }

    manager.servers.write().insert(name, server)
    PRINT "Added MCP server '{name}' with {tools.len()} tools"
```

### 3.2 M4: Hot-Reload Config Diff

```
FUNCTION on_config_file_changed(old_config, new_config):
    # Debounce 500ms (caller responsibility)

    old_servers = old_config.tools.mcp_servers
    new_servers = new_config.tools.mcp_servers

    removed = old_servers.keys() - new_servers.keys()
    added = new_servers.keys() - old_servers.keys()
    maybe_changed = old_servers.keys() & new_servers.keys()

    FOR name IN removed:
        manager.remove_server(name)  # drain-and-swap

    FOR name IN added:
        manager.add_server(name, new_servers[name])

    FOR name IN maybe_changed:
        IF old_servers[name] != new_servers[name]:
            manager.remove_server(name)  # drain-and-swap
            manager.add_server(name, new_servers[name])
```

### 3.3 M5: Bridge Connection

```
FUNCTION connect_bridge():
    # Outbound (already working):
    #   User runs: claude mcp add clawft -- weft mcp-server
    #   Claude Code connects to clawft's McpServerShell

    # Inbound:
    bridge = McpBridge::new()
    bridge.connect_to_claude_code()
    # This spawns `claude mcp serve` and performs handshake

    # Register Claude Code tools in the manager:
    FOR tool IN bridge.claude_tools():
        # Available as mcp:claude-code:<tool-name>
        register_tool("claude-code", tool)

    RETURN bridge
```

### 3.4 L4: ReAct Planning Loop

```
FUNCTION react_loop(task, config, tool_executor):
    steps = []
    total_cost = 0.0
    noops = 0

    FOR step_num IN 0..config.max_planning_depth:
        # Guard rails
        IF total_cost >= config.max_planning_cost_usd:
            RETURN partial_result(steps, BudgetExhausted)
        IF noops >= config.circuit_breaker_threshold:
            RETURN partial_result(steps, CircuitBreaker)

        # Execute step with timeout
        step = TIMEOUT(config.planning_step_timeout):
            thought = llm_call("Given context, what should I do next?")
            action = parse_action(thought)
            IF action == "finish":
                RETURN complete(steps)
            observation = tool_executor.execute(action)
            RETURN PlanningStep { thought, action, observation }

        IF step timed out:
            RETURN partial_result(steps, StepTimeout)

        IF step is no-op:
            noops += 1
        ELSE:
            noops = 0

        total_cost += step.cost_usd
        steps.push(step)

    RETURN partial_result(steps, DepthLimit)
```

### 3.5 M4: Drain-and-Swap Sequence

```
FUNCTION drain_and_swap(server_name):
    server = manager.servers[server_name]

    # Phase 1: Mark draining
    server.status = Draining
    LOG "Server '{server_name}' entering drain phase"

    # Phase 2: Wait for in-flight calls
    deadline = now() + 30 seconds
    WHILE server.inflight > 0 AND now() < deadline:
        sleep(100ms)

    IF server.inflight > 0:
        LOG WARN "Drain timeout: {server.inflight} calls still in-flight"

    # Phase 3: Disconnect and remove
    manager.servers.remove(server_name)
    LOG "Server '{server_name}' removed"
    # McpSession dropped -> StdioTransport dropped -> child process killed
    # or HttpTransport dropped (no cleanup needed)
```

---

## 4. Architecture

### 4.1 File Map

| File | Owner | Status | Purpose |
|------|-------|--------|---------|
| `crates/clawft-services/src/mcp/discovery.rs` | M4 | NEW | `McpServerManager`, `McpServerConfig`, hot-reload, drain-and-swap |
| `crates/clawft-services/src/mcp/bridge.rs` | M5 | NEW | `McpBridge`, Claude Code bidirectional connection |
| `crates/clawft-core/src/agent/planning.rs` | L4 | NEW | `PlanningRouter`, `PlanningStrategy`, guard rails |
| `crates/clawft-cli/src/commands/mcp_cmd.rs` | M4 | NEW | `weft mcp add/list/remove` CLI commands |
| `crates/clawft-services/src/mcp/mod.rs` | Shared | MODIFY | Add `pub mod discovery;` and `pub mod bridge;` |
| `crates/clawft-cli/src/commands/mod.rs` | Shared | MODIFY | Register `mcp` subcommand |
| `crates/clawft-services/src/mcp/transport.rs` | Shared | READ-ONLY | Uses existing `StdioTransport`, `HttpTransport` |
| `crates/clawft-services/src/mcp/server.rs` | C6 | READ-ONLY | Existing server shell (outbound bridge) |
| `docs/guides/configuration.md` | M6 | NEW or MODIFY | Delegation config documentation |
| `docs/guides/tool-calls.md` | M6 | NEW or MODIFY | MCP bridge setup guide |

### 4.2 MCP Topology Diagram

```
+------------------------------------------------------------------+
|                         clawft Runtime                            |
|                                                                   |
|  +------------------+    +--------------------+                   |
|  | McpServerShell   |    | McpServerManager   |                   |
|  | (server.rs)      |    | (discovery.rs)     |                   |
|  |                  |    |                    |                   |
|  | Exposes tools    |    | Manages external   |                   |
|  | to MCP clients   |    | MCP server conns   |                   |
|  +--------+---------+    +----+------+--------+                   |
|           |                   |      |                            |
|           | stdio             |      |                            |
|           v                   |      |                            |
|   +--------------+            |      |                            |
|   | Claude Code  |<-----------+      |                            |
|   | (MCP Client) |  inbound   |      |                            |
|   |              |  (bridge)  |      |                            |
|   | Claude Code  |            |      |                            |
|   | (MCP Server) |--via------>|      |                            |
|   +--------------+ bridge.rs  |      |                            |
|                               |      |                            |
|                     +---------+      +---------+                  |
|                     v                          v                  |
|              +------------+            +---------------+          |
|              | github     |            | filesystem    |          |
|              | MCP Server |            | MCP Server    |          |
|              | (stdio)    |            | (stdio)       |          |
|              +------------+            +---------------+          |
|                                                                   |
|  +--------------------+                                           |
|  | PlanningRouter     |  L4 (post-MVP)                           |
|  | (planning.rs)      |                                           |
|  | ReAct / PlanExec   |                                           |
|  | Guard Rails        |                                           |
|  +--------------------+                                           |
+------------------------------------------------------------------+
```

### 4.3 Bridge Data Flow

```
Outbound Flow (clawft tools -> Claude Code):
  Claude Code --> JSON-RPC stdin --> McpServerShell
                                        |
                                        v
                                   CompositeToolProvider
                                        |
                                        v
                                   BuiltinToolProvider
                                        |
                                        v
                                   ToolRegistry.execute()
                                        |
                                        v
                                   JSON-RPC stdout --> Claude Code

Inbound Flow (Claude Code tools -> clawft agents):
  Agent Loop --> McpServerManager.call_tool("claude-code", tool, params)
                        |
                        v
                   ManagedMcpServer (claude-code)
                        |
                        v
                   McpSession.call_tool()
                        |
                        v
                   StdioTransport --> `claude mcp serve` --> Claude Code
                        |
                        v
                   JSON-RPC response --> Agent Loop
```

### 4.4 Hot-Reload Data Flow

```
clawft.toml change detected
         |
         | notify crate (debounce 500ms)
         v
  Parse new [tools.mcp_servers]
         |
         v
  Diff against McpServerManager state
         |
         +---> New servers: McpSession::connect() + add_server()
         |
         +---> Removed servers: drain-and-swap (30s timeout)
         |
         +---> Changed servers: drain-and-swap + reconnect
```

---

## 5. Refinement

### 5.1 Hot-Reload Edge Cases

#### 5.1.1 Config Parse Failure During Hot-Reload

If the new `clawft.toml` is malformed or the `[tools.mcp_servers]` section fails to parse:
- Log `warn!` with the parse error
- Keep existing server state unchanged (no servers added or removed)
- Do NOT partially apply the diff

```rust
// In the config watcher callback:
match parse_mcp_servers_config(&new_toml) {
    Ok(new_configs) => manager.apply_config_diff(new_configs).await,
    Err(e) => {
        warn!(error = %e, "failed to parse MCP server config, keeping existing state");
        vec![] // No errors to report since nothing changed
    }
}
```

#### 5.1.2 Server Connection Failure During Hot-Reload

If a new server fails to connect during hot-reload:
- Log `warn!` with the connection error
- Skip that server; continue processing remaining servers
- The failed server is NOT added to the manager
- Other servers in the diff are still processed normally

#### 5.1.3 Rapid Config Changes

Multiple config changes within the debounce window (500ms) are collapsed into a single diff. If a config change arrives while a previous diff is still being applied:
- The new change is queued and applied after the current diff completes
- Only one diff is applied at a time (serialized via the `RwLock`)

#### 5.1.4 Config Change During Drain

If a server is currently draining (from a previous config change) and a new config change re-adds the same server:
- Wait for the current drain to complete
- Then add the new server
- This is naturally handled by `apply_config_diff()` which calls `remove_server()` (blocking until drain) before `add_server()`

### 5.2 Drain Race Conditions

#### 5.2.1 Tool Call Arriving During Drain Start

A tool call may be in the `call_tool()` path (past the status check) when the drain starts:
- The `InFlightGuard` increments the inflight counter before the status is set to `Draining`
- The drain loop waits for `inflight == 0`, so this call completes normally
- **Ordering guarantee**: `inflight.fetch_add(1)` happens before `status = Draining` check in the read path; drain observes the incremented counter

#### 5.2.2 Multiple Concurrent Drain Requests

If `remove_server()` is called concurrently for the same server:
- The first call acquires the write lock and sets `Draining`
- The second call reads the server as `Draining` (or finds it already removed)
- Error handling: second call returns `DiscoveryError::NotFound` if the server was already removed, or `DiscoveryError::AlreadyDraining` if still draining

#### 5.2.3 Drain Timeout with Stuck Call

If a tool call hangs and the 30-second drain timeout expires:
- The server is force-removed from the manager
- The `McpSession` is dropped, which drops the `StdioTransport`, which kills the child process
- The hanging `call_tool()` future receives a transport error (pipe broken / child exited)
- The `InFlightGuard` decrements on drop regardless

### 5.3 Planning Infinite Loops (L4)

#### 5.3.1 No-Op Detection

A planning step is classified as a "no-op" if:
- The LLM returns a thought but no action
- The action is the same as the previous step's action with the same parameters
- The observation is empty or identical to the previous observation

Three consecutive no-ops trigger the circuit breaker.

#### 5.3.2 Cost Estimation Accuracy

Cost estimation uses token counts from the LLM response headers (if available) or falls back to character-based estimation. The budget check is conservative: if the estimated cost of the next step would exceed the remaining budget, the planner stops before executing the step.

#### 5.3.3 Partial Result Quality

When any guard rail terminates planning early:
- The planner includes all successfully completed steps
- The output includes a human-readable explanation: "Planning terminated: [reason]. Completed [N] of [M] planned steps. Partial results below."
- Completed tool call results are preserved (not rolled back)

### 5.4 Security Considerations

#### 5.4.1 MCP URL Validation (SSRF Protection)

```rust
// crates/clawft-services/src/mcp/discovery.rs

fn validate_mcp_url(url: &str) -> Result<(), DiscoveryError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| DiscoveryError::InvalidUrl(e.to_string()))?;

    // Reject non-HTTP schemes
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(DiscoveryError::InvalidUrl(
            format!("unsupported scheme: {}", parsed.scheme())
        ));
    }

    // Reject localhost/private IPs for untrusted servers
    // (configurable -- trusted servers may use localhost)
    let host = parsed.host_str().unwrap_or("");
    if host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("172.") // simplified; full check for 172.16-31
    {
        tracing::warn!(url = url, "MCP server URL points to private address");
        // Allow but warn (user explicitly configured this)
    }

    Ok(())
}
```

#### 5.4.2 Stdio Command Validation

```rust
// crates/clawft-services/src/mcp/discovery.rs

fn validate_command_path(command: &str) -> Result<(), DiscoveryError> {
    // Reject empty commands
    if command.is_empty() {
        return Err(DiscoveryError::InvalidCommand("empty command".into()));
    }

    // Reject commands with shell metacharacters
    if command.contains(';')
        || command.contains('|')
        || command.contains('&')
        || command.contains('`')
        || command.contains('$')
    {
        return Err(DiscoveryError::InvalidCommand(
            "command contains shell metacharacters".into(),
        ));
    }

    // Verify command exists on PATH (non-blocking)
    // This is best-effort; the actual spawn will fail if not found
    Ok(())
}
```

#### 5.4.3 MCP Temp File Permissions

All temporary files created during MCP operations (e.g., for large payloads) use the `tempfile` crate with restricted permissions:

```rust
use tempfile::NamedTempFile;
use std::os::unix::fs::PermissionsExt;

fn create_mcp_temp_file() -> std::io::Result<NamedTempFile> {
    let file = NamedTempFile::new()?;
    // Set 0600 permissions (owner read+write only)
    std::fs::set_permissions(
        file.path(),
        std::fs::Permissions::from_mode(0o600),
    )?;
    Ok(file)
}
```

---

## 6. Completion

### 6.1 Acceptance Criteria

#### M4: Dynamic MCP Server Discovery

- [ ] `McpServerManager` struct created in `discovery.rs`
- [ ] `add_server()` connects via stdio or HTTP transport based on `target` format
- [ ] `add_server()` performs MCP initialize handshake via `McpSession::connect()`
- [ ] `add_server()` fetches and stores tool list from connected server
- [ ] `remove_server()` implements drain-and-swap protocol (30s timeout)
- [ ] `remove_server()` rejects new tool calls during drain (returns `ServerDraining`)
- [ ] `remove_server()` waits for in-flight calls to complete before disconnecting
- [ ] `list_servers()` returns name, status, and tool count for each server
- [ ] `call_tool()` tracks in-flight calls with `AtomicU32` + RAII guard
- [ ] `call_tool()` rejects calls to draining or disconnected servers
- [ ] `apply_config_diff()` handles add, remove, and change operations
- [ ] `weft mcp add <name> <target>` CLI command works (stdio and HTTP)
- [ ] `weft mcp list` CLI command shows connected servers and their tools
- [ ] `weft mcp remove <name>` CLI command triggers drain-and-swap
- [ ] Hot-reload: file watcher on `clawft.toml` with 500ms debounce
- [ ] Hot-reload: parse failure does not disrupt existing server state
- [ ] Tools namespaced as `mcp:<server-name>:<tool-name>` in the registry
- [ ] URL validation rejects non-HTTP schemes
- [ ] Command validation rejects shell metacharacters

#### M5: Bidirectional MCP Bridge

- [ ] `McpBridge` struct created in `bridge.rs`
- [ ] `connect_to_claude_code()` spawns `claude mcp serve` and handshakes
- [ ] `call_claude_tool()` delegates tool calls to Claude Code's MCP server
- [ ] `claude_tools()` returns tool definitions from Claude Code
- [ ] Delegation depth tracking prevents recursive loops (max depth: 3)
- [ ] Outbound bridge documented: `claude mcp add clawft -- weft mcp-server`
- [ ] Inbound bridge works via `[tools.mcp_servers.claude-code]` config
- [ ] Bridge connection errors are logged and surfaced to the agent loop

#### L4: Planning Guard Rails (Post-MVP)

- [ ] `PlanningStrategy` enum: `ReAct`, `PlanAndExecute`
- [ ] `PlanningConfig` with all five parameters (depth, cost, timeout, circuit breaker, strategy)
- [ ] `PlanningConfig` defaults match orchestrator spec (Section 8)
- [ ] `PlanningRouter::execute()` dispatches to correct strategy
- [ ] `max_planning_depth` enforced: planning stops at limit
- [ ] `max_planning_cost_usd` enforced: planning stops before exceeding budget
- [ ] `planning_step_timeout` enforced: step cancelled after timeout
- [ ] Circuit breaker: planning aborts after N consecutive no-op steps
- [ ] Partial results returned with human-readable termination explanation
- [ ] `[router.planning]` section parseable from `clawft.toml`
- [ ] Cost tracking integrates with D6 `sender_id`

#### M6: Documentation

- [ ] `docs/guides/configuration.md` updated with delegation config section
- [ ] `docs/guides/tool-calls.md` updated with MCP bridge setup guide
- [ ] Both outbound and inbound bridge workflows documented
- [ ] Per-agent MCP server override syntax documented
- [ ] Planning guard rail configuration documented

### 6.2 Test Plan

#### M4 Tests

| Test ID | Description | Type | File |
|---------|-------------|------|------|
| M4-T01 | `McpServerManager::new()` creates empty manager | Unit | `discovery.rs` |
| M4-T02 | `add_server()` with stdio target connects and fetches tools | Unit | `discovery.rs` |
| M4-T03 | `add_server()` with HTTP target connects and fetches tools | Unit | `discovery.rs` |
| M4-T04 | `add_server()` with invalid target returns error | Unit | `discovery.rs` |
| M4-T05 | `remove_server()` transitions status to Draining | Unit | `discovery.rs` |
| M4-T06 | `remove_server()` waits for in-flight calls | Unit | `discovery.rs` |
| M4-T07 | `remove_server()` force-disconnects after 30s timeout | Unit | `discovery.rs` |
| M4-T08 | `remove_server()` returns NotFound for unknown server | Unit | `discovery.rs` |
| M4-T09 | `list_servers()` returns correct status and tool count | Unit | `discovery.rs` |
| M4-T10 | `call_tool()` rejects calls to Draining server | Unit | `discovery.rs` |
| M4-T11 | `call_tool()` rejects calls to Disconnected server | Unit | `discovery.rs` |
| M4-T12 | `call_tool()` increments and decrements inflight counter | Unit | `discovery.rs` |
| M4-T13 | `InFlightGuard` decrements on panic (drop safety) | Unit | `discovery.rs` |
| M4-T14 | `apply_config_diff()` adds new servers | Unit | `discovery.rs` |
| M4-T15 | `apply_config_diff()` removes deleted servers | Unit | `discovery.rs` |
| M4-T16 | `apply_config_diff()` replaces changed servers | Unit | `discovery.rs` |
| M4-T17 | `apply_config_diff()` ignores unchanged servers | Unit | `discovery.rs` |
| M4-T18 | `validate_mcp_url()` rejects non-HTTP schemes | Unit | `discovery.rs` |
| M4-T19 | `validate_mcp_url()` warns on private IPs | Unit | `discovery.rs` |
| M4-T20 | `validate_command_path()` rejects shell metacharacters | Unit | `discovery.rs` |
| M4-T21 | `validate_command_path()` rejects empty command | Unit | `discovery.rs` |
| M4-T22 | `McpServerConfig` serde roundtrip (TOML) | Unit | `discovery.rs` |
| M4-T23 | Hot-reload: config parse failure preserves existing state | Integration | `discovery.rs` |
| M4-T24 | Full add -> call -> remove lifecycle | Integration | `discovery.rs` |

#### M5 Tests

| Test ID | Description | Type | File |
|---------|-------------|------|------|
| M5-T01 | `McpBridge::new()` starts disconnected | Unit | `bridge.rs` |
| M5-T02 | `is_connected()` returns false initially | Unit | `bridge.rs` |
| M5-T03 | `connect_to_claude_code()` performs handshake (mock) | Unit | `bridge.rs` |
| M5-T04 | `call_claude_tool()` returns NotConnected when disconnected | Unit | `bridge.rs` |
| M5-T05 | `call_claude_tool()` delegates to session (mock) | Unit | `bridge.rs` |
| M5-T06 | `claude_tools()` returns discovered tools | Unit | `bridge.rs` |
| M5-T07 | `call_claude_tool_with_depth()` rejects at max depth | Unit | `bridge.rs` |
| M5-T08 | `call_claude_tool_with_depth()` injects depth counter | Unit | `bridge.rs` |
| M5-T09 | Bridge registers as MCP server in McpServerManager | Integration | `bridge.rs` |
| M5-T10 | `BridgeError` variants have descriptive messages | Unit | `bridge.rs` |

#### L4 Tests

| Test ID | Description | Type | File |
|---------|-------------|------|------|
| L4-T01 | `PlanningConfig::default()` matches orchestrator defaults | Unit | `planning.rs` |
| L4-T02 | `PlanningConfig` serde roundtrip (TOML) | Unit | `planning.rs` |
| L4-T03 | `check_guard_rails()` triggers on depth limit | Unit | `planning.rs` |
| L4-T04 | `check_guard_rails()` triggers on budget exhaustion | Unit | `planning.rs` |
| L4-T05 | `check_guard_rails()` triggers on circuit breaker | Unit | `planning.rs` |
| L4-T06 | `check_guard_rails()` returns None when within limits | Unit | `planning.rs` |
| L4-T07 | `execute_step_with_timeout()` returns StepTimeout | Unit | `planning.rs` |
| L4-T08 | ReAct loop terminates on depth limit | Integration | `planning.rs` |
| L4-T09 | ReAct loop terminates on budget exhaustion | Integration | `planning.rs` |
| L4-T10 | ReAct loop terminates on circuit breaker | Integration | `planning.rs` |
| L4-T11 | Partial results include all completed steps | Integration | `planning.rs` |
| L4-T12 | Partial results include termination explanation | Integration | `planning.rs` |
| L4-T13 | `PlanningStrategy` deserialization from string | Unit | `planning.rs` |
| L4-T14 | No-op detection classifies empty observations | Unit | `planning.rs` |
| L4-T15 | No-op counter resets on productive step | Unit | `planning.rs` |

### 6.3 Exit Criteria

All of the following must be true before M-Advanced is marked complete:

1. **M4 functional**: `weft mcp add/list/remove` commands work end-to-end with both stdio and HTTP MCP servers. Hot-reload updates servers without downtime.
2. **M5 functional**: Bidirectional bridge tested -- clawft tools callable from Claude Code AND Claude Code tools callable from clawft agents.
3. **L4 types defined**: `PlanningStrategy`, `PlanningConfig`, `PlanningRouter` structs compile and all guard rail unit tests pass. Full LLM integration is deferred to the L4 implementation sprint.
4. **M6 published**: Both documentation pages exist and accurately describe the configuration.
5. **All 49 tests pass** (24 M4 + 10 M5 + 15 L4).
6. **No new `unwrap()` calls** outside test code.
7. **Files under 500 lines** each.
8. **Existing tests unbroken**: `cargo test` passes for all crates.
9. **Security**: URL validation, command validation, temp file permissions, delegation depth limit all implemented and tested.

---

## 7. Cross-Element Dependencies

### 7.1 Inbound Dependencies

| Dependency | Element/Phase | Required For | Status |
|-----------|---------------|--------------|--------|
| F9a: Core MCP client (`McpClient`, `McpSession`) | 07/F-Core | M4 (server connections), M5 (bridge) | Implemented in `mod.rs` |
| C6: MCP server for skills | 04/C6 | M5 outbound (tool exposure) | Planning |
| D6: `sender_id` | 05/D6 | L4 cost tracking | Planning |
| D9: MCP transport | 05/D9 | M4 (transport reuse) | Implemented in `transport.rs` |
| Contract 3.2: MCP Client <-> Agent Routing | 02/Cross-Element | M4 per-agent MCP config | Planning |

### 7.2 Outbound Dependencies

| Beneficiary | Description |
|------------|-------------|
| F9b (Element 07) | Shares `McpServerConfig` types; M4's `McpServerManager` is the runtime complement to F9b's `McpDiscovery` |
| F8 (Element 07) | IDE MCP tools registered alongside M4-managed servers |
| L1-L3 (Element 09) | Agent routing uses M4 manager for per-agent tool availability |

### 7.3 Shared File Coordination

| File | M-Advanced Touches | Other Owners | Coordination |
|------|-------------------|--------------|--------------|
| `transport.rs` | Read-only (uses `StdioTransport`, `HttpTransport`) | F9b adds `new_sandboxed` | No conflict |
| `types.rs` | Read-only (uses `JsonRpcRequest`, `JsonRpcResponse`) | Shared | No conflict |
| `mod.rs` | Adds `pub mod discovery;` and `pub mod bridge;` | F8 adds `pub mod ide;`, F9b adds re-exports | Trivial merge |
| `composite.rs` | Read-only (uses `CompositeToolProvider::register()`) | Shared | No conflict |

---

## 8. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| MCP hot-reload race: tool call during drain start | Low | Medium | **3** | `InFlightGuard` RAII pattern; `AtomicU32` ordering ensures correct inflight count |
| Drain timeout with hanging MCP server | Medium | Medium | **4** | 30s hard timeout; force-disconnect drops transport, killing child process |
| Recursive delegation loop (clawft <-> Claude Code) | Medium | High | **6** | `MAX_DELEGATION_DEPTH = 3`; depth counter injected into call params |
| L4 planning cost runaway | Medium | High | **6** | `max_planning_cost_usd` hard cap; budget check before each step (conservative) |
| L4 infinite loop (ReAct) | Medium | Medium | **4** | Circuit breaker (3 no-ops); `max_planning_depth` (10 steps) |
| SSRF via MCP HTTP URLs | Low | High | **4** | `validate_mcp_url()` warns on private IPs; URL scheme restricted to http/https |
| Shell injection via MCP stdio command | Low | Critical | **5** | `validate_command_path()` rejects shell metacharacters; direct spawn (no `sh -c`) |
| Config parse failure disrupts running servers | Low | Medium | **3** | Parse failure preserves existing state; no partial diff application |
| `claude mcp serve` not available on user system | Medium | Low | **2** | Bridge connection failure logged; feature degrades gracefully (inbound tools unavailable) |
| F9a API changes break M4/M5 | Low | Medium | **3** | F9a is already implemented and tested; M4/M5 use stable public API (`McpSession::connect`, `list_tools`, `call_tool`) |

---

## 9. Implementation Order

```
Week 6-7: M4 Core
  M4.1: McpServerConfig + McpServerManager types
  M4.2: connect() factory (stdio + HTTP)
  M4.3: add_server() + list_servers()
  M4.4: InFlightGuard + call_tool()
  M4.5: remove_server() drain-and-swap
  M4.6: weft mcp add/list/remove CLI
  M4.7: validate_mcp_url() + validate_command_path()

Week 7-8: M4 Hot-Reload + M5 Bridge
  M4.8: apply_config_diff()
  M4.9: Config file watcher integration
  M5.1: McpBridge struct + connect_to_claude_code()
  M5.2: call_claude_tool() + delegation depth guard
  M5.3: Bridge as McpServerManager entry

Week 8: M6 Documentation
  M6.1: docs/guides/configuration.md delegation section
  M6.2: docs/guides/tool-calls.md MCP bridge guide

Week 8-9: L4 Planning (Post-MVP)
  L4.1: PlanningStrategy + PlanningConfig types
  L4.2: PlanningRouter + check_guard_rails()
  L4.3: execute_step_with_timeout()
  L4.4: ReAct loop skeleton
  L4.5: Plan-and-Execute skeleton
  L4.6: Cost tracking integration

mod.rs updates: Add module declarations after M4.1 and M5.1 complete.
```

M4 and M5 share no new code and can be partially parallelized (M5.1-M5.2 can start as soon as M4.2 proves the transport factory pattern). L4 is fully independent and can be developed in parallel with M5 and M6.
