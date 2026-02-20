# Phase F-MCP: MCP IDE Integration & Full MCP Client

> **Element:** 07 -- Dev Tools & Applications
> **Phase:** F-MCP
> **Timeline:** Week 8-10
> **Priority:** P2 (Post-MVP)
> **Crates:** `clawft-services` (extends `src/mcp/`)
> **Dependencies IN:** C6 (MCP server for loaded skills), F9a (Core MCP client), D9 (MCP transport)
> **Blocks:** None (post-MVP features)
> **Status:** Planning

---

## 1. Overview

Phase F-MCP adds two post-MVP capabilities to the MCP infrastructure layer:

1. **F8 -- MCP IDE Integration**: A `ToolProvider` implementation that exposes IDE operations (open file, edit, diagnostics, symbols, hover) as MCP tools. A VS Code extension connects as an MCP client and the agent manipulates code live through standard MCP tool calls.

2. **F9b -- Full MCP Client Features**: Extends the core MCP client (F9a) with auto-discovery of MCP servers, connection pooling, schema caching, health checks, and namespace management for 1000+ external tools.

Both items live entirely within `crates/clawft-services/src/mcp/` -- they are NOT separate crates. F8 creates `ide.rs` (new file). F9b extends `client.rs` (from F9a) and creates `discovery.rs` (new file).

---

## 2. Current MCP Infrastructure Code

### `crates/clawft-services/src/mcp/mod.rs` (12 lines)

Module root. Re-exports key types:

```rust
pub mod composite;
pub mod middleware;
pub mod provider;
pub mod server;
pub mod transport;
pub mod types;

pub use provider::{BuiltinToolProvider, CallToolResult, ContentBlock, ToolError, ToolProvider};
```

After F-MCP, this file adds `pub mod ide;` and `pub mod discovery;`.

### `crates/clawft-services/src/mcp/provider.rs` (400 lines, 14 tests)

Defines the `ToolProvider` trait -- the central abstraction F8 implements:

```rust
#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn namespace(&self) -> &str;
    fn list_tools(&self) -> Vec<ToolDefinition>;
    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError>;
}
```

Supporting types:
- `CallToolResult` -- content blocks + `is_error` flag
- `ContentBlock::Text { text: String }` -- single content variant
- `ToolError` -- `NotFound`, `ExecutionFailed`, `PermissionDenied`
- `BuiltinToolProvider` -- wraps local tools with a dispatcher closure

### `crates/clawft-services/src/mcp/composite.rs` (297 lines, 11 tests)

`CompositeToolProvider` aggregates multiple `Box<dyn ToolProvider>`:
- `register()` adds a provider
- `list_tools_all()` returns all tools prefixed as `"{namespace}__{tool_name}"`
- `call_tool()` splits on first `"__"` to route to the correct provider

F8's `IdeToolProvider` registers here and becomes immediately visible via the MCP server.

### `crates/clawft-services/src/mcp/server.rs` (729 lines, 12 tests)

`McpServerShell` -- JSON-RPC server over `AsyncBufRead + AsyncWrite`:
- Protocol version: `"2025-06-18"`
- Capabilities: `{ "tools": { "listChanged": true } }`
- Methods: `initialize`, `tools/list`, `tools/call`, `notifications/initialized`
- Middleware pipeline: `filter_tools`, `before_call`, `after_call`

### `crates/clawft-services/src/mcp/transport.rs` (351 lines, 6 tests)

- `McpTransport` trait: `send_request()`, `send_notification()`
- `StdioTransport`: spawns child process, communicates via stdin/stdout JSON lines
- `HttpTransport`: REST-based JSON-RPC POST requests
- `MockTransport` (test-only): pre-programmed responses, records requests/notifications

### `crates/clawft-services/src/mcp/middleware.rs` (808 lines, 25 tests)

Middleware pipeline with 4 built-in implementations:
- `SecurityGuard`: command policy + URL safety validation
- `PermissionFilter`: tool allowlist filtering
- `ResultGuard`: truncates oversized output (default 64 KB)
- `AuditLog`: traces tool invocations

### `crates/clawft-services/src/mcp/types.rs` (208 lines, 11 tests)

JSON-RPC 2.0 wire types:
- `JsonRpcRequest` -- `jsonrpc`, `id`, `method`, `params`
- `JsonRpcResponse` -- `jsonrpc`, `id`, `result?`, `error?`
- `JsonRpcNotification` -- `jsonrpc`, `method`, `params` (no `id`)
- `JsonRpcError` -- `code`, `message`, `data?`

### `crates/clawft-services/src/mcp/mod.rs` -- `McpClient` & `McpSession`

The module root also contains `McpClient` (basic JSON-RPC client with `list_tools()`, `call_tool()`, `send_raw()`) and `McpSession` (handshake wrapper: `connect()` performs initialize + notifications/initialized). F9b extends these.

---

## 3. F8: MCP IDE Integration

### 3.1 Purpose

Agent edits code live in an IDE through MCP tool calls. A VS Code extension acts as an MCP client connecting to the clawft MCP server. The `IdeToolProvider` translates MCP tool invocations into IDE operations via an `IdeConnection` abstraction.

### 3.2 Architecture

```
VS Code Extension (MCP Client)
         |
         | JSON-RPC (stdio or HTTP)
         v
McpServerShell
         |
         | tools/call "ide__open_file" { "path": "src/main.rs" }
         v
CompositeToolProvider
         |
         | namespace="ide", tool="open_file"
         v
IdeToolProvider
         |
         | IdeConnection trait
         v
Actual IDE backend (VS Code, Neovim, etc.)
```

### 3.3 File: `crates/clawft-services/src/mcp/ide.rs` (NEW)

### Task F8.1: Define `IdeConnection` trait

The connection abstraction between `IdeToolProvider` and the actual IDE. This trait is what a VS Code extension (or any editor) implements.

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Position in a text document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based character offset (UTF-16 code units, per LSP spec).
    pub character: u32,
}

/// A range within a text document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// A text edit to apply to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace. Empty range = insertion.
    pub range: Range,
    /// Replacement text.
    pub new_text: String,
}

/// A diagnostic (error/warning) from the IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    /// Source (e.g. "rustc", "clippy", "eslint").
    #[serde(default)]
    pub source: Option<String>,
}

/// Diagnostic severity levels (matches LSP).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// A workspace symbol result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub range: Range,
}

/// Hover information for a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    #[serde(default)]
    pub range: Option<Range>,
}

/// Abstraction over the IDE connection.
///
/// Implementors bridge MCP tool calls to actual IDE operations.
/// A VS Code extension provides a concrete implementation that
/// communicates back to the extension host.
#[async_trait]
pub trait IdeConnection: Send + Sync {
    /// Open a file in the editor at an optional position.
    async fn open_file(&self, path: &str, position: Option<Position>)
        -> Result<(), IdeError>;

    /// Apply a text edit to an open document.
    async fn apply_edit(&self, path: &str, edit: TextEdit)
        -> Result<(), IdeError>;

    /// Get current diagnostics for a file (or all files if path is None).
    async fn get_diagnostics(&self, path: Option<&str>)
        -> Result<Vec<Diagnostic>, IdeError>;

    /// Search workspace symbols by query string.
    async fn search_symbols(&self, query: &str, limit: usize)
        -> Result<Vec<SymbolInfo>, IdeError>;

    /// Get hover information at a position.
    async fn hover(&self, path: &str, position: Position)
        -> Result<Option<HoverInfo>, IdeError>;
}

/// Errors from IDE operations.
#[derive(Debug, thiserror::Error)]
pub enum IdeError {
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("editor not connected")]
    NotConnected,

    #[error("operation failed: {0}")]
    OperationFailed(String),

    #[error("timeout waiting for IDE response")]
    Timeout,
}
```

### Task F8.2: Implement `IdeToolProvider`

Implements the existing `ToolProvider` trait for IDE operations. Follows the same pattern as `BuiltinToolProvider` in `provider.rs`.

```rust
use std::sync::Arc;
use super::ToolDefinition;
use super::provider::{CallToolResult, ToolError, ToolProvider};

/// MCP tool provider for IDE operations.
///
/// Exposes five tools through the MCP server:
/// - `ide__open_file`: Open a file in the editor
/// - `ide__edit`: Apply a text edit to an open document
/// - `ide__diagnostics`: Get current diagnostics
/// - `ide__symbols`: Search workspace symbols
/// - `ide__hover`: Get hover information
pub struct IdeToolProvider {
    connection: Arc<dyn IdeConnection>,
}

impl IdeToolProvider {
    pub fn new(connection: Arc<dyn IdeConnection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl ToolProvider for IdeToolProvider {
    fn namespace(&self) -> &str {
        "ide"
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "open_file".into(),
                description: "Open a file in the IDE editor, optionally at a specific position".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or workspace-relative file path"
                        },
                        "line": {
                            "type": "integer",
                            "description": "Optional zero-based line number to navigate to"
                        },
                        "character": {
                            "type": "integer",
                            "description": "Optional zero-based character offset"
                        }
                    },
                    "required": ["path"]
                }),
            },
            ToolDefinition {
                name: "edit".into(),
                description: "Apply a text edit to an open document in the IDE".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to edit"
                        },
                        "start_line": { "type": "integer" },
                        "start_character": { "type": "integer" },
                        "end_line": { "type": "integer" },
                        "end_character": { "type": "integer" },
                        "new_text": {
                            "type": "string",
                            "description": "Replacement text"
                        }
                    },
                    "required": ["path", "start_line", "start_character",
                                 "end_line", "end_character", "new_text"]
                }),
            },
            ToolDefinition {
                name: "diagnostics".into(),
                description: "Get current diagnostics (errors, warnings) from the IDE".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Optional file path. Omit for all files."
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "symbols".into(),
                description: "Search workspace symbols by name query".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Symbol name search query"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results (default: 20)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "hover".into(),
                description: "Get hover/type information at a specific position in a file".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "line": { "type": "integer" },
                        "character": { "type": "integer" }
                    },
                    "required": ["path", "line", "character"]
                }),
            },
        ]
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        match name {
            "open_file" => {
                let path = args.get("path").and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::ExecutionFailed("missing 'path'".into()))?;
                let position = match (
                    args.get("line").and_then(|v| v.as_u64()),
                    args.get("character").and_then(|v| v.as_u64()),
                ) {
                    (Some(line), char_opt) => Some(Position {
                        line: line as u32,
                        character: char_opt.unwrap_or(0) as u32,
                    }),
                    _ => None,
                };
                self.connection.open_file(path, position).await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                Ok(CallToolResult::text(format!("Opened {path}")))
            }
            "edit" => { /* extract args, build TextEdit, call apply_edit */ }
            "diagnostics" => { /* call get_diagnostics, serialize to JSON */ }
            "symbols" => { /* call search_symbols, serialize results */ }
            "hover" => { /* call hover, serialize result */ }
            _ => Err(ToolError::NotFound(name.to_string())),
        }
    }
}
```

### Task F8.3: Path sanitization

All file paths received through MCP tool calls MUST be sanitized before passing to `IdeConnection`:

```rust
/// Sanitize a file path from MCP input.
///
/// Rejects paths containing `..` traversal, null bytes, or absolute paths
/// outside the workspace root.
fn sanitize_path(workspace_root: &Path, raw: &str) -> Result<PathBuf, ToolError> {
    if raw.contains('\0') {
        return Err(ToolError::PermissionDenied {
            tool: "ide".into(),
            reason: "null byte in path".into(),
        });
    }

    let candidate = Path::new(raw);
    let resolved = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        workspace_root.join(candidate)
    };

    // Canonicalize and verify it's within workspace.
    let canonical = resolved.canonicalize()
        .map_err(|e| ToolError::ExecutionFailed(format!("path resolution: {e}")))?;
    if !canonical.starts_with(workspace_root) {
        return Err(ToolError::PermissionDenied {
            tool: "ide".into(),
            reason: format!("path escapes workspace: {}", raw),
        });
    }

    Ok(canonical)
}
```

### Task F8.4: Register `IdeToolProvider` with `CompositeToolProvider`

When an IDE connection is established, register the provider:

```rust
// In the server setup code (e.g., gateway or session init):
if let Some(ide_conn) = ide_connection {
    let ide_provider = IdeToolProvider::new(ide_conn);
    composite.register(Box::new(ide_provider));
    // Optionally notify via "notifications/tools/listChanged"
}
```

The `CompositeToolProvider` will automatically prefix tools as `ide__open_file`, `ide__edit`, etc., matching the `{namespace}__{tool_name}` convention.

### Task F8.5: VS Code extension communication protocol

The VS Code extension connects to the MCP server and ALSO serves as the `IdeConnection` backend. Communication is bidirectional:

```
Agent --> McpServerShell --> IdeToolProvider --> IdeConnection --> VS Code
             (JSON-RPC)                          (reverse channel)
```

The reverse channel uses a separate JSON-RPC stream where `IdeToolProvider` sends requests to the extension and the extension responds. This avoids multiplexing on the main MCP channel.

Implementation options (pick one during development):
1. **Separate stdio stream**: Extension spawns a helper process for the reverse channel
2. **HTTP callback**: Extension starts a local HTTP server; `IdeConnection` POSTs to it
3. **Shared channel with message tagging**: Use a single stdio stream with request IDs that distinguish MCP requests from IDE requests

Recommended approach: **HTTP callback** -- simplest to implement, uses existing `HttpTransport`.

### Task F8.6: Update `mod.rs`

Add `pub mod ide;` to `crates/clawft-services/src/mcp/mod.rs`.

---

## 4. F9b: Full MCP Client Features

### 4.1 Purpose

Extend F9a's basic `McpClient`/`McpSession` to support the "1000+ community servers" story: auto-discovery, connection pooling, schema caching, health monitoring, and per-agent configuration.

### 4.2 Architecture

```
Agent Config (TOML)
       |
       v
McpDiscovery -----> finds MCP servers from:
       |             1. Global config (~/.clawft/mcp_servers/)
       |             2. Agent-specific config
       |             3. Workspace config (.clawft/mcp.toml)
       v
McpConnectionPool
       |
       | Connection per server, reused across calls
       v
McpSession (from F9a)
       |
       v
CompositeToolProvider <-- registers ExternalMcpProvider per server
       |
       v
McpServerShell (serves tools to local agents)
```

### 4.3 File: `crates/clawft-services/src/mcp/discovery.rs` (NEW)

### Task F9b.1: MCP server discovery

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Configuration for a single MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Display name for this server.
    pub name: String,

    /// Connection URL. Formats:
    /// - `stdio://<command>` -- spawn child process
    /// - `http://<host>:<port>` / `https://...` -- HTTP transport
    pub url: String,

    /// Whether this server is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Environment variables to pass to stdio child process.
    /// MUST NOT contain secrets -- see security notes.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Optional timeout for requests to this server (milliseconds).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Trust level for tools from this server.
    #[serde(default)]
    pub trust: TrustLevel,
}

fn default_true() -> bool { true }
fn default_timeout() -> u64 { 30_000 }

/// Trust level for external MCP server tools.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustLevel {
    /// Require user approval for first invocation of each tool.
    #[default]
    Untrusted,
    /// Tools can be invoked without user approval.
    Trusted,
}

/// Per-agent MCP server overrides.
///
/// Agent-level entries replace global entries with the same name.
/// New names are appended. `enabled = false` explicitly excludes
/// a global server for this agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMcpConfig {
    /// Server configs keyed by server name.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Discovers MCP server configurations from multiple sources.
pub struct McpDiscovery {
    /// Global MCP server directory.
    global_config_dir: PathBuf,
}

impl McpDiscovery {
    pub fn new(global_config_dir: PathBuf) -> Self {
        Self { global_config_dir }
    }

    /// Discover all enabled MCP servers, merging global + agent configs.
    ///
    /// Resolution order (later overrides earlier):
    /// 1. Global configs from `~/.clawft/mcp_servers/*.toml`
    /// 2. Workspace configs from `.clawft/mcp.toml`
    /// 3. Agent-specific configs from `~/.clawft/agents/<name>/config.toml`
    pub async fn discover(
        &self,
        workspace_root: Option<&Path>,
        agent_config: Option<&AgentMcpConfig>,
    ) -> Result<Vec<McpServerConfig>, DiscoveryError> {
        let mut servers: HashMap<String, McpServerConfig> = HashMap::new();

        // 1. Global configs
        self.load_global_configs(&mut servers).await?;

        // 2. Workspace configs
        if let Some(root) = workspace_root {
            self.load_workspace_configs(root, &mut servers).await?;
        }

        // 3. Agent overrides
        if let Some(agent) = agent_config {
            for (name, config) in &agent.mcp_servers {
                if !config.enabled {
                    servers.remove(name);
                } else {
                    servers.insert(name.clone(), config.clone());
                }
            }
        }

        // Return only enabled servers
        Ok(servers.into_values().filter(|s| s.enabled).collect())
    }

    async fn load_global_configs(
        &self,
        servers: &mut HashMap<String, McpServerConfig>,
    ) -> Result<(), DiscoveryError> {
        // Read *.toml files from global_config_dir
        // Parse each as McpServerConfig
        // Insert into servers map keyed by name
        todo!("implementation")
    }

    async fn load_workspace_configs(
        &self,
        workspace_root: &Path,
        servers: &mut HashMap<String, McpServerConfig>,
    ) -> Result<(), DiscoveryError> {
        // Read .clawft/mcp.toml from workspace root
        // Parse [mcp_servers] section
        // Insert/override into servers map
        todo!("implementation")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config parse error: {0}")]
    Parse(String),
}
```

### Task F9b.2: Connection pooling

Extend `crates/clawft-services/src/mcp/client.rs` (from F9a) with a connection pool:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Pool of MCP sessions keyed by server name.
///
/// Reuses connections across tool calls. Automatically reconnects
/// when a session is detected as unhealthy.
pub struct McpConnectionPool {
    sessions: Arc<RwLock<HashMap<String, PooledSession>>>,
    max_connections: usize,
}

struct PooledSession {
    session: McpSession,
    config: McpServerConfig,
    /// Last successful communication timestamp.
    last_healthy: std::time::Instant,
    /// Consecutive failed health checks.
    failed_checks: u32,
}

impl McpConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
        }
    }

    /// Get or create a session for the named server.
    pub async fn get_session(
        &self,
        server_name: &str,
        config: &McpServerConfig,
    ) -> Result<&McpSession> {
        // Check existing session
        // If healthy, return it
        // If unhealthy or missing, create new connection
        // Enforce max_connections limit
        todo!("implementation")
    }

    /// Remove a session from the pool (e.g., on server shutdown).
    pub async fn remove(&self, server_name: &str) {
        self.sessions.write().await.remove(server_name);
    }

    /// Close all connections.
    pub async fn shutdown(&self) {
        self.sessions.write().await.clear();
    }
}
```

### Task F9b.3: Schema caching

Cache tool definitions from external servers to avoid re-listing on every call:

```rust
use std::time::{Duration, Instant};

/// Cached tool schema for an external MCP server.
struct SchemaCache {
    tools: Vec<ToolDefinition>,
    fetched_at: Instant,
    ttl: Duration,
}

impl SchemaCache {
    fn is_expired(&self) -> bool {
        self.fetched_at.elapsed() > self.ttl
    }
}

/// Schema cache manager for all connected MCP servers.
pub struct McpSchemaCache {
    cache: RwLock<HashMap<String, SchemaCache>>,
    /// Default TTL for cached schemas.
    default_ttl: Duration,
}

impl McpSchemaCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            default_ttl,
        }
    }

    /// Get cached tools or fetch fresh from session.
    pub async fn get_tools(
        &self,
        server_name: &str,
        session: &McpSession,
    ) -> Result<Vec<ToolDefinition>> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(server_name) {
                if !entry.is_expired() {
                    return Ok(entry.tools.clone());
                }
            }
        }

        // Fetch fresh
        let tools = session.list_tools().await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(server_name.to_string(), SchemaCache {
                tools: tools.clone(),
                fetched_at: Instant::now(),
                ttl: self.default_ttl,
            });
        }

        Ok(tools)
    }

    /// Invalidate cache for a server (e.g., on listChanged notification).
    pub async fn invalidate(&self, server_name: &str) {
        self.cache.write().await.remove(server_name);
    }

    /// Invalidate all cached schemas.
    pub async fn invalidate_all(&self) {
        self.cache.write().await.clear();
    }
}
```

### Task F9b.4: Health checks

Background task that periodically pings connected MCP servers:

```rust
use tokio::time::interval;

/// Health monitor for MCP server connections.
pub struct McpHealthMonitor {
    pool: Arc<McpConnectionPool>,
    check_interval: Duration,
    /// Maximum consecutive failures before removing a connection.
    max_failures: u32,
}

impl McpHealthMonitor {
    /// Start the health check loop. Returns a handle that can be
    /// used to stop monitoring.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(self.check_interval);
            loop {
                ticker.tick().await;
                self.check_all().await;
            }
        })
    }

    async fn check_all(&self) {
        // For each session in the pool:
        // 1. Send a lightweight request (e.g., tools/list with timeout)
        // 2. If it succeeds, reset failed_checks and update last_healthy
        // 3. If it fails, increment failed_checks
        // 4. If failed_checks > max_failures, remove from pool and log warning
    }
}
```

### Task F9b.5: Namespace management for external tools

When registering external MCP server tools in the `CompositeToolProvider`, ensure namespace collision avoidance:

```rust
/// A ToolProvider backed by an external MCP server connection.
///
/// Tools are served through the existing McpSession, with namespace
/// prefixing handled by CompositeToolProvider.
pub struct ExternalMcpProvider {
    server_name: String,
    session: Arc<McpSession>,
    schema_cache: Arc<McpSchemaCache>,
    trust_level: TrustLevel,
}

#[async_trait]
impl ToolProvider for ExternalMcpProvider {
    fn namespace(&self) -> &str {
        // Returns "mcp:<server-name>" to produce tool names like
        // "mcp:github__create_issue" via CompositeToolProvider.
        &self.namespace_str
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        // Return cached tools (blocking cache read for sync interface).
        // In practice, tools are pre-fetched during connection setup.
        self.cached_tools.clone()
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        // Delegate to session.call_tool()
        // Convert Result<Value> to CallToolResult
        let result = self.session.call_tool(name, args).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Parse the MCP result format
        let content = result.get("content")
            .and_then(|v| v.as_array())
            .map(|blocks| {
                blocks.iter().filter_map(|b| {
                    b.get("text").and_then(|t| t.as_str())
                        .map(|text| ContentBlock::Text { text: text.to_string() })
                }).collect()
            })
            .unwrap_or_default();

        let is_error = result.get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(CallToolResult { content, is_error })
    }
}
```

### Task F9b.6: Per-agent MCP configuration

Support per-agent overrides of global MCP server configuration:

```toml
# ~/.clawft/agents/coding-agent/config.toml
[mcp_servers.github]
url = "stdio://gh-mcp-server"
enabled = true

[mcp_servers.slack]
enabled = false  # Explicitly exclude global server for this agent

[mcp_servers.custom-api]
name = "custom-api"
url = "http://localhost:3000/mcp"
enabled = true
trust = "Trusted"
```

Agent-level entries replace global entries with the same name. New names are appended. `enabled = false` explicitly excludes a global server for the agent.

### Task F9b.7: Security -- stdio child process isolation

MCP stdio child processes MUST NOT inherit secret environment variables:

```rust
impl StdioTransport {
    /// Spawn a child process with a MINIMAL environment.
    ///
    /// Only explicitly listed env vars from McpServerConfig are passed.
    /// The parent process's environment is NOT inherited.
    pub async fn new_sandboxed(
        command: &str,
        args: &[String],
        explicit_env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            // Clear inherited environment
            .env_clear()
            // Add only explicitly allowed vars
            .envs(explicit_env)
            // Add minimal required vars
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut child = cmd.spawn()?;
        // ... same as existing new()
    }
}
```

Security constraints for stdio command paths:
- **Untrusted servers**: Require full absolute paths for the command
- **Validate command exists**: Check that the command binary exists before spawning
- **No shell expansion**: Spawn directly, never through `sh -c`

### Task F9b.8: Update `mod.rs`

Add `pub mod discovery;` to `crates/clawft-services/src/mcp/mod.rs`.

---

## 5. Tests Required

### F8 Tests

| Test ID | Description | Type | File |
|---------|-------------|------|------|
| F8-T01 | `IdeToolProvider::namespace()` returns `"ide"` | Unit | `ide.rs` |
| F8-T02 | `IdeToolProvider::list_tools()` returns 5 tools | Unit | `ide.rs` |
| F8-T03 | `call_tool("open_file")` delegates to `IdeConnection::open_file` | Unit | `ide.rs` |
| F8-T04 | `call_tool("edit")` delegates to `IdeConnection::apply_edit` | Unit | `ide.rs` |
| F8-T05 | `call_tool("diagnostics")` returns serialized diagnostics | Unit | `ide.rs` |
| F8-T06 | `call_tool("symbols")` delegates search and serializes results | Unit | `ide.rs` |
| F8-T07 | `call_tool("hover")` returns hover info or null | Unit | `ide.rs` |
| F8-T08 | `call_tool("unknown")` returns `ToolError::NotFound` | Unit | `ide.rs` |
| F8-T09 | Path traversal attack rejected (`../../../etc/passwd`) | Unit | `ide.rs` |
| F8-T10 | Null byte in path rejected | Unit | `ide.rs` |
| F8-T11 | Absolute path outside workspace rejected | Unit | `ide.rs` |
| F8-T12 | `IdeToolProvider` registers with `CompositeToolProvider` and tools are listed as `ide__*` | Integration | `ide.rs` |
| F8-T13 | Full MCP session: `tools/list` shows IDE tools, `tools/call` routes to mock IDE | Integration | `ide.rs` |
| F8-T14 | `IdeConnection::NotConnected` error maps to `CallToolResult` with `is_error=true` | Unit | `ide.rs` |
| F8-T15 | All IDE types (`Position`, `Range`, `TextEdit`, `Diagnostic`, `SymbolInfo`, `HoverInfo`) serde roundtrip | Unit | `ide.rs` |
| F8-T16 | `DiagnosticSeverity` enum values match LSP specification (1-4) | Unit | `ide.rs` |

### F9b Tests

| Test ID | Description | Type | File |
|---------|-------------|------|------|
| F9b-T01 | `McpDiscovery` loads global configs from `~/.clawft/mcp_servers/` | Unit | `discovery.rs` |
| F9b-T02 | `McpDiscovery` loads workspace configs from `.clawft/mcp.toml` | Unit | `discovery.rs` |
| F9b-T03 | Agent config overrides global config for same server name | Unit | `discovery.rs` |
| F9b-T04 | Agent config `enabled = false` excludes global server | Unit | `discovery.rs` |
| F9b-T05 | Agent config adds new servers not in global | Unit | `discovery.rs` |
| F9b-T06 | Discovery returns only enabled servers | Unit | `discovery.rs` |
| F9b-T07 | `McpServerConfig` serde roundtrip (TOML and JSON) | Unit | `discovery.rs` |
| F9b-T08 | `TrustLevel` defaults to `Untrusted` | Unit | `discovery.rs` |
| F9b-T09 | `McpConnectionPool::get_session` creates new session on first call | Unit | `client.rs` |
| F9b-T10 | `McpConnectionPool::get_session` reuses existing healthy session | Unit | `client.rs` |
| F9b-T11 | `McpConnectionPool` enforces `max_connections` limit | Unit | `client.rs` |
| F9b-T12 | `McpConnectionPool::remove` drops session | Unit | `client.rs` |
| F9b-T13 | `McpSchemaCache` returns cached tools within TTL | Unit | `client.rs` |
| F9b-T14 | `McpSchemaCache` fetches fresh tools after TTL expires | Unit | `client.rs` |
| F9b-T15 | `McpSchemaCache::invalidate` clears single server cache | Unit | `client.rs` |
| F9b-T16 | `McpSchemaCache::invalidate_all` clears all caches | Unit | `client.rs` |
| F9b-T17 | `McpHealthMonitor` detects failed server and removes from pool | Integration | `client.rs` |
| F9b-T18 | `McpHealthMonitor` resets failure count on successful check | Integration | `client.rs` |
| F9b-T19 | `ExternalMcpProvider::namespace()` returns `"mcp:<name>"` | Unit | `client.rs` |
| F9b-T20 | `ExternalMcpProvider::call_tool` delegates to `McpSession` | Unit | `client.rs` |
| F9b-T21 | `ExternalMcpProvider::call_tool` converts MCP result to `CallToolResult` | Unit | `client.rs` |
| F9b-T22 | External tools registered in `CompositeToolProvider` with `mcp:<name>__` prefix | Integration | `client.rs` |
| F9b-T23 | `StdioTransport::new_sandboxed` does NOT inherit parent env | Unit | `transport.rs` |
| F9b-T24 | `StdioTransport::new_sandboxed` passes only explicit env vars + PATH/HOME | Unit | `transport.rs` |
| F9b-T25 | Untrusted server with relative command path is rejected | Unit | `discovery.rs` |
| F9b-T26 | 1000+ tools from multiple servers are correctly namespaced without collision | Stress | `client.rs` |

---

## 6. Acceptance Criteria

### F8: MCP IDE Integration

- [ ] `IdeToolProvider` implements `ToolProvider` trait with namespace `"ide"`
- [ ] 5 IDE tools exposed: `open_file`, `edit`, `diagnostics`, `symbols`, `hover`
- [ ] All tool input schemas are valid JSON Schema
- [ ] Path sanitization rejects directory traversal, null bytes, and out-of-workspace paths
- [ ] `IdeToolProvider` registers with `CompositeToolProvider` and tools appear in `tools/list`
- [ ] `IdeConnection` trait is `Send + Sync + 'static` for multi-agent use
- [ ] Mock `IdeConnection` implementation exists for testing
- [ ] All 16 F8 tests pass
- [ ] File stays under 500 lines (types may split to `ide_types.rs` if needed)

### F9b: Full MCP Client Features

- [ ] `McpDiscovery` loads configs from global, workspace, and agent-specific sources
- [ ] Agent config correctly overrides/excludes global server entries
- [ ] `McpConnectionPool` manages sessions with reuse and `max_connections` limit
- [ ] `McpSchemaCache` caches tool definitions with configurable TTL
- [ ] `McpSchemaCache` invalidates on `tools/listChanged` notification
- [ ] `McpHealthMonitor` detects and removes unhealthy connections
- [ ] `ExternalMcpProvider` implements `ToolProvider` for remote MCP servers
- [ ] External tools are namespaced as `"mcp:<server-name>"` to avoid collision
- [ ] `StdioTransport::new_sandboxed` clears inherited environment
- [ ] Untrusted servers require absolute command paths
- [ ] Per-agent MCP configuration works with TOML `[mcp_servers.*]` sections
- [ ] All 26 F9b tests pass
- [ ] No new `unwrap()` calls outside test code

### Security Criteria

- [ ] MCP stdio child processes do NOT inherit secret environment variables
- [ ] External tools are tagged with `TrustLevel::Untrusted` by default
- [ ] Untrusted tool invocation requires user approval on first use
- [ ] `StdioTransport::new_sandboxed` validates command paths (no shell expansion)
- [ ] IDE file paths are sanitized against directory traversal
- [ ] `SecurityGuard` middleware applies to external MCP tool calls

---

## 7. File Ownership

| File | Owner | Notes |
|------|-------|-------|
| `mcp/ide.rs` | F8 | New file |
| `mcp/discovery.rs` | F9b | New file |
| `mcp/client.rs` | F9b extends F9a | Connection pool, schema cache, health monitor |
| `mcp/transport.rs` | Shared (F9b adds `new_sandboxed`) | Cross-stream PR review required |
| `mcp/types.rs` | Shared | Cross-stream PR review required |
| `mcp/middleware.rs` | Shared | Cross-stream PR review required |
| `mcp/mod.rs` | Both (add module declarations) | Trivial change, no conflict expected |
| `mcp/server.rs` | Neither (no changes needed) | Provider registration happens outside server |
| `mcp/composite.rs` | Neither (no changes needed) | Existing `register()` API is sufficient |
| `mcp/provider.rs` | Neither (no changes needed) | Existing `ToolProvider` trait is sufficient |

---

## 8. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Malicious MCP server tool injection via F9b | Medium | High | **6** | Tag external tools as `TrustLevel::Untrusted`; require user approval for first invocation per tool; sandbox stdio children with `env_clear()` |
| MCP stdio transport spawns arbitrary commands | Medium | High | **6** | Validate stdio command paths; require absolute paths for untrusted servers; spawn directly without shell; inherit minimal env vars (PATH, HOME only) |
| IDE path traversal via F8 | Medium | High | **6** | `sanitize_path()` rejects `..`, null bytes, and paths outside workspace root; canonicalize before access |
| Connection pool resource exhaustion (many MCP servers) | Medium | Medium | **4** | `max_connections` limit (default: 20); health monitor removes dead connections; connection timeout enforcement |
| Schema cache staleness (server updates tools, cache not invalidated) | Low | Medium | **3** | TTL-based expiry (default: 5 minutes); immediate invalidation on `tools/listChanged` notification; manual `invalidate()` API |
| IDE connection dropped mid-operation | Medium | Low | **2** | `IdeError::NotConnected` propagates cleanly as `CallToolResult::error()`; `IdeToolProvider` can be unregistered from `CompositeToolProvider` on disconnect |
| VS Code extension communication latency | Low | Low | **2** | HTTP callback approach avoids stdio multiplexing; timeout on IDE operations (default: 5s) |
| F9a instability delays F9b | Low | Medium | **3** | F9a is already implemented with 37+ tests; F9b builds incrementally on stable foundation |
| Namespace collision with 1000+ tools | Low | Low | **2** | `"mcp:<server-name>__<tool>"` format; server names are unique by config key; `CompositeToolProvider` handles prefix correctly |

---

## 9. Dependencies

### Inbound Dependencies

| Dependency | Element/Phase | Required For | Status |
|-----------|---------------|--------------|--------|
| C6: MCP server for loaded skills | 04/C6 | F8 (server infrastructure) | Planning |
| F9a: Core MCP client | 07/F-Core | F9b (base client code) | Planning |
| D9: MCP transport | 05/D9 | F9b (transport layer) | Planning |

### Outbound Dependencies

F-MCP is **post-MVP** and does not block any other element. However, the following items would benefit from F-MCP completion:

| Beneficiary | Description |
|------------|-------------|
| M4: Tool Ecosystem | Can use F9b to connect to 1000+ community MCP servers |
| Voice pipeline | Could use F8 IDE tools for hands-free coding |
| Multi-agent routing | F9b's namespace management enables agents to share external tool pools |

---

## 10. Implementation Order

```
F9b.1 (Discovery types + config)     F8.1 (IdeConnection trait + types)
         |                                      |
F9b.2 (Connection pooling)           F8.2 (IdeToolProvider implementation)
         |                                      |
F9b.3 (Schema caching)               F8.3 (Path sanitization)
         |                                      |
F9b.4 (Health checks)                F8.4 (CompositeToolProvider registration)
         |                                      |
F9b.5 (ExternalMcpProvider)          F8.5 (VS Code extension protocol)
         |                                      |
F9b.6 (Per-agent config)             F8.6 (mod.rs update)
         |
F9b.7 (Stdio sandboxing)
         |
F9b.8 (mod.rs update)
```

F8 and F9b can be developed in parallel -- they share no new code. F8 only depends on the existing `ToolProvider` trait (already stable). F9b depends on F9a's `McpClient`/`McpSession` (already implemented in `mod.rs`).
