# SPARC Implementation Plan: Phase 3H -- Claude & Claude-Flow Tool Delegation

**Stream**: 3H -- Tool Call Support for Task Delegation
**Status**: PLANNED
**Timeline**: ~4 sessions (10-13 hours) -- Session 2 expanded for pluggable McpServerShell architecture
**Depends on**: 2c-services (MCP client), 2e-integration-wiring (tool registry wiring)
**Codebase root**: `/home/aepod/dev/clawft/`

---

## 1. Specification

### 1.1 Problem Statement

clawft has a working MCP client (`clawft-services/src/mcp/`) with `McpClient`, `StdioTransport`, `HttpTransport`, and `MockTransport`. It also has `McpToolWrapper` in `clawft-cli/src/mcp_tools.rs` that bridges MCP tools into the `ToolRegistry`. The config already has `tools.mcp_servers: HashMap<String, MCPServerConfig>`.

**What is already wired**:
- `McpClient` can `list_tools()` and `call_tool()` via JSON-RPC 2.0
- `StdioTransport` spawns child processes and communicates via newline-delimited JSON
- `HttpTransport` sends HTTP POST to an endpoint
- `McpToolWrapper` wraps MCP tool definitions as `dyn Tool` with `{server}__{tool}` namespacing
- `register_mcp_tools()` iterates `config.tools.mcp_servers`, creates clients, lists tools, and registers them
- Both `weft agent` and `weft gateway` call `register_mcp_tools()` at startup

**What is missing (the gaps)**:

1. **MCP `initialize` handshake**: The current `McpClient` sends `tools/list` directly without the required MCP `initialize` -> `notifications/initialized` handshake. Real MCP servers (Claude Code, claude-flow) require this handshake before accepting any other method call. Without it, `tools/list` returns a JSON-RPC error.

2. **JSON-RPC notifications**: The `McpTransport` trait only has `send_request()` which expects a response. MCP notifications (`notifications/initialized`) are fire-and-forget -- no response expected. The trait needs `send_notification()`.

3. **~~Content-Length framing for stdio~~**: _(Resolved -- see Transport Correction below.)_ The current `StdioTransport` already uses newline-delimited JSON, which is correct per the MCP 2025-06-18 spec for stdio transport. Content-Length framing is an LSP convention, not MCP. No change needed here.

4. **Tool result content block mapping**: MCP `tools/call` returns structured content blocks (`{content: [{type: "text", text: "..."}], isError: false}`), not raw JSON values. The `McpToolWrapper.execute()` returns the raw `call_tool` result without extracting text content.

5. **MCP Server Mode**: clawft cannot currently act as an MCP server. A `weft mcp-server` command would let other tools (Claude Code, other LLMs) connect to clawft and use its tools (read_file, write_file, exec_shell, memory, web_search, etc.) via MCP protocol. **Resolution: The new `McpServerShell` + `BuiltinToolProvider` architecture replaces the originally-planned monolithic `mcp_server_run()` with a pluggable, composable server. See Section 3.7.**

6. **Delegation decision engine**: No mechanism to decide whether a tool call should be handled locally, delegated to Claude API, or forwarded to claude-flow. This is a higher-level concern.

7. **Anthropic tool use bridge**: No direct bridge for delegating tasks to Claude's Messages API with tool use. This enables "Claude-as-orchestrator, clawft-as-executor" patterns.

8. **Tool execution duplication (CRIT-02)**: No shared infrastructure for tool dispatch -- the MCP server, the agent loop, and the delegation bridge would each have their own tool execution paths. **Resolution: The `ToolProvider` trait and `CompositeToolProvider` provide a single dispatch layer. See Section 3.7.**

9. **No access control on tool execution (CRIT-01)**: No middleware layer to enforce `CommandPolicy`/`UrlPolicy` on MCP tool calls. **Resolution: The middleware pipeline (`SecurityGuard`, `PermissionFilter`, `ResultGuard`, `AuditLog`) sits between transport and dispatch. See Section 3.7.**

### 1.2 Functional Requirements

```
FR-001  MCP Initialize Handshake
        McpClient shall send `initialize` with protocolVersion, capabilities,
        and clientInfo before any other RPC method. After receiving the
        initialize result, it shall send `notifications/initialized`.
        Acceptance: Mock MCP server test verifies handshake sequence.

FR-002  Newline-Delimited Stdio Framing (CONFIRMED CORRECT)
        StdioTransport already uses newline-delimited JSON, which is correct
        per MCP 2025-06-18 spec for stdio transport. Content-Length framing
        is an LSP convention, not MCP. No rewrite needed.
        Acceptance: Existing newline-delimited tests pass.

FR-003  JSON-RPC Notifications
        McpTransport trait shall support send_notification() for fire-and-forget
        messages (no id, no response). StdioTransport writes the notification.
        HttpTransport sends a POST and ignores the response.
        Acceptance: Unit tests verify notification format.

FR-004  MCP Tool Result Content Extraction
        McpToolWrapper.execute() shall parse MCP tool result content blocks,
        extracting text content. If isError is true, return ToolError.
        If content is an array of blocks, concatenate text blocks.
        Acceptance: Unit tests with various MCP response shapes.

FR-005  MCP Server Mode (`weft mcp-server`)
        clawft shall serve as an MCP server over stdio, exposing its registered
        tools via the MCP protocol. It shall handle `initialize`, `tools/list`,
        and `tools/call` requests from clients.
        Acceptance: Start `weft mcp-server`, connect with a test client,
        list tools, call read_file.

FR-006  Claude API Delegation (Tool Use Bridge)
        clawft shall delegate tasks to the Anthropic Messages API with tool use.
        It converts ToolRegistry schemas (OpenAI format) to Anthropic format
        (input_schema), sends the task, executes tool_use requests locally,
        returns tool_result blocks, and loops until end_turn or max_turns.
        Acceptance: Mock HTTP test with tool use round-trip.

FR-007  Delegation Decision Engine
        Configurable rule engine (regex patterns -> target) decides whether to
        handle locally, delegate to Claude, or forward to claude-flow.
        Falls back to local when target unavailable.
        Acceptance: Unit tests for rule matching and fallback.

FR-008  Graceful Degradation
        Missing API keys or unreachable MCP servers disable delegation
        (not a fatal error). Agent loop continues with reduced capabilities.
        Acceptance: Startup with missing claude-flow binary succeeds.
```

### 1.3 Non-Functional Requirements

```
NFR-001  Performance: MCP handshake completes in <100ms for local stdio servers
NFR-002  Security: API keys never logged; Debug impls mask key values
NFR-003  Security: Delegated tool execution respects CommandPolicy and UrlPolicy
         (enforced by SecurityGuard middleware in McpServerShell pipeline)
NFR-004  Reliability: MCP server crash does not crash agent loop
NFR-005  Modularity: Delegation code behind feature flag `tool-delegate`
NFR-006  Compatibility: WASM build unaffected (delegation is native-only)
NFR-007  Size: Tool result truncation at MAX_TOOL_RESULT_BYTES (64KB)
         (enforced by ResultGuard middleware in McpServerShell pipeline)
```

### 1.4 Configuration Schema

Existing in `clawft-types/src/config.rs`:
```rust
pub struct ToolsConfig {
    pub mcp_servers: HashMap<String, MCPServerConfig>,
    // ...
}

pub struct MCPServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: String,
}
```

New addition to `Config`:
```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub delegation: DelegationConfig,
}
```

New type in `clawft-types/src/delegation.rs`:
```rust
pub struct DelegationConfig {
    pub claude_enabled: bool,
    pub claude_model: String,        // default: "claude-sonnet-4-20250514"
    pub max_turns: u32,              // default: 10
    pub max_tokens: u32,             // default: 4096
    pub claude_flow_enabled: bool,
    pub claude_flow_server: Option<MCPServerConfig>,
    pub rules: Vec<DelegationRule>,
    pub excluded_tools: Vec<String>,
}
```

Example config:
```json
{
  "tools": {
    "mcpServers": {
      "claude-code": {
        "command": "claude",
        "args": ["mcp", "serve"]
      },
      "claude-flow": {
        "command": "npx",
        "args": ["@claude-flow/cli@latest", "mcp", "start"]
      }
    }
  },
  "delegation": {
    "claude_enabled": true,
    "claude_model": "claude-sonnet-4-20250514",
    "max_turns": 10,
    "excluded_tools": ["exec_shell"]
  }
}
```

---

## 2. Pseudocode

### 2.1 MCP Client Initialization Flow

```
FUNCTION mcp_client_connect(transport: McpTransport) -> Result<McpSession>
    client = McpClient::new(transport)

    // Step 1: Send initialize request
    init_result = client.send_raw("initialize", {
        "protocolVersion": "2025-06-18",
        "capabilities": { "tools": {} },
        "clientInfo": { "name": "clawft", "version": CARGO_PKG_VERSION }
    })

    // Step 2: Parse server response
    server_caps = parse_capabilities(init_result["capabilities"])
    server_info = parse_server_info(init_result["serverInfo"])
    protocol_version = init_result["protocolVersion"]

    // Step 3: Send initialized notification (fire-and-forget)
    client.send_notification("notifications/initialized", {})

    RETURN McpSession { client, server_caps, server_info, protocol_version }
END FUNCTION
```

### 2.2 Stdio Framing (Newline-Delimited JSON -- No Change Needed)

> **Transport Correction**: The MCP 2025-06-18 spec uses **newline-delimited JSON** for
> stdio transport, NOT Content-Length framing. Content-Length is the LSP (Language Server
> Protocol) convention. The existing `StdioTransport` already uses `read_line()` / `write_line()`,
> which is correct. The Streamable HTTP transport uses HTTP's own framing (chunked transfer
> encoding or Content-Length on HTTP responses), but that is handled by the HTTP library.
>
> The original plan incorrectly identified this as a bug. No rewrite needed.

```
// EXISTING (CORRECT for MCP stdio):
FUNCTION stdio_write(stdin, message: JsonRpc) -> Result<()>
    json_bytes = serialize(message)
    stdin.write_all(json_bytes + "\n")
    stdin.flush()
END FUNCTION

FUNCTION stdio_read(stdout) -> Result<JsonRpc>
    line = read_line(stdout)
    RETURN deserialize(line)
END FUNCTION
```

### 2.3 MCP Tool Result Content Extraction

```
FUNCTION extract_mcp_tool_result(raw: Value) -> Result<Value, ToolError>
    // MCP tools/call returns:
    // { "content": [{"type": "text", "text": "..."}], "isError": false }
    // OR just a raw value (for simple servers)

    IF raw.has("isError") AND raw["isError"] == true THEN
        error_text = extract_text_blocks(raw["content"])
        RETURN Err(ToolError::ExecutionFailed(error_text))
    END IF

    IF raw.has("content") AND raw["content"].is_array() THEN
        text = extract_text_blocks(raw["content"])
        RETURN Ok(json!({"output": text}))
    END IF

    // Fallback: return raw value as-is
    RETURN Ok(raw)
END FUNCTION

FUNCTION extract_text_blocks(blocks: Vec<Value>) -> String
    result = ""
    FOR block IN blocks
        IF block["type"] == "text" THEN
            result += block["text"]
        ELSE IF block.is_string() THEN
            result += block.as_str()
        END IF
    END FOR
    RETURN result
END FUNCTION
```

### 2.4 MCP Server Mode (Pluggable `McpServerShell` Architecture)

The monolithic `mcp_server_run(tool_registry)` function is replaced by a composable
architecture: `McpServerShell` handles all protocol concerns, `ToolProvider` trait
abstracts tool sources, and a middleware pipeline enforces security/policy.

```
// --- ToolProvider trait (provider.rs) ---
TRAIT ToolProvider: Send + Sync
    fn namespace(&self) -> &str
    async fn list_tools(&self) -> Vec<ToolDefinition>
    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError>
END TRAIT

// --- CompositeToolProvider (composite.rs) ---
// Aggregates multiple providers, handles namespace routing
STRUCT CompositeToolProvider
    providers: Vec<Box<dyn ToolProvider>>

    fn register(provider: Box<dyn ToolProvider>)

    async fn list_tools_all() -> Vec<ToolDefinition>
        // Iterate providers, prefix each tool name with "{namespace}__"
        FOR provider IN self.providers
            FOR tool IN provider.list_tools()
                tool.name = "{provider.namespace()}__{tool.name}"
                tools.push(tool)
            END FOR
        END FOR
        RETURN tools

    async fn call_tool(namespaced_name: &str, args: Value) -> Result<CallToolResult>
        // Split on "__", find provider by namespace, strip prefix, dispatch
        (namespace, tool_name) = split_namespace(namespaced_name)
        provider = find_provider(namespace)
        RETURN provider.call_tool(tool_name, args)
END STRUCT

// --- Middleware pipeline (middleware.rs) ---
// Applied between transport and tool dispatch:
//   1. SecurityGuard:    enforces CommandPolicy + UrlPolicy on tool args
//   2. PermissionFilter: intersects tools with session's allowed_tools config
//   3. ResultGuard:      truncate_result(MAX_TOOL_RESULT_BYTES) + sanitize_content()
//   4. AuditLog:         logs tool invocations for observability
TRAIT Middleware: Send + Sync
    async fn filter_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition>
    async fn before_call(&self, request: &ToolCallRequest) -> Result<ToolCallRequest>
    async fn after_call(&self, request: &ToolCallRequest, result: &CallToolResult) -> Result<CallToolResult>
END TRAIT

// --- McpServerShell (server.rs) ---
// Generic server handling all MCP protocol concerns
FUNCTION McpServerShell::new(provider: CompositeToolProvider, middleware: Vec<Box<dyn Middleware>>)
    RETURN McpServerShell { provider, middleware, initialized: false }
END FUNCTION

FUNCTION McpServerShell::run(reader: AsyncRead, writer: AsyncWrite)
    // Newline-delimited JSON for stdio transport
    LOOP
        line = reader.read_line()
        IF line.is_empty() THEN BREAK  // EOF, parent terminated
        request = deserialize(line)

        IF request.method == "initialize" THEN
            self.initialized = true
            respond(writer, request.id, {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "clawft",
                    "version": CARGO_PKG_VERSION
                }
            })

        ELSE IF request.method == "notifications/initialized" THEN
            // Notification: no response needed

        ELSE IF NOT self.initialized THEN
            respond_error(writer, request.id, -32002, "not initialized")

        ELSE IF request.method == "tools/list" THEN
            tools = self.provider.list_tools_all()
            // Apply PermissionFilter middleware
            FOR mw IN self.middleware
                tools = mw.filter_tools(tools)
            END FOR
            respond(writer, request.id, { "tools": tools })

        ELSE IF request.method == "tools/call" THEN
            name = request.params["name"]
            args = request.params["arguments"]

            // Apply SecurityGuard + PermissionFilter middleware (before_call)
            request = ToolCallRequest { name, args }
            FOR mw IN self.middleware
                request = mw.before_call(request)?  // may reject with ToolError
            END FOR

            result = self.provider.call_tool(request.name, request.args).await

            // Apply ResultGuard middleware (after_call: truncation + sanitization)
            FOR mw IN self.middleware
                result = mw.after_call(request, result)?
            END FOR

            MATCH result
                Ok(val) => respond(writer, request.id, val)
                Err(e)  => respond(writer, request.id, {
                    "content": [{"type": "text", "text": e.to_string()}],
                    "isError": true
                })
            END MATCH

        ELSE
            respond_error(writer, request.id, -32601, "method not found")
        END IF
    END LOOP
END FUNCTION

// --- Wiring in `weft mcp-server` command ---
FUNCTION mcp_server_command(config: Config)
    // Build providers
    builtin = BuiltinToolProvider::new(tool_registry)   // wraps existing ToolRegistry
    composite = CompositeToolProvider::new()
    composite.register(builtin)

    // Future: composite.register(RvfToolProvider::new(rvf_runtime))    // 3F-RVF
    // Future: composite.register(DelegationToolProvider::new(delegator)) // Claude bridge

    // Build middleware
    middleware = [
        SecurityGuard::new(config.security.command_policy, config.security.url_policy),
        PermissionFilter::new(config.tools.allowed_tools),
        ResultGuard::new(MAX_TOOL_RESULT_BYTES),
        AuditLog::new(),
    ]

    shell = McpServerShell::new(composite, middleware)
    shell.run(stdin, stdout)
END FUNCTION
```

### 2.5 Claude API Delegation (Tool Use Bridge)

```
FUNCTION delegate_to_claude(task, registry, config) -> Result<String>
    // Convert OpenAI tool schemas to Anthropic format
    openai_schemas = registry.schemas()
    anthropic_tools = openai_schemas.map(|s| {
        func = s["function"]
        { "name": func["name"], "description": func["description"],
          "input_schema": func["parameters"] }
    }).filter(|t| t["name"] NOT IN config.excluded_tools)

    messages = [{ "role": "user", "content": task }]
    turns = 0

    LOOP
        IF turns >= config.max_turns THEN RETURN Error("max turns")

        response = http_post("https://api.anthropic.com/v1/messages", {
            headers: { "x-api-key": key, "anthropic-version": "2023-06-01" },
            body: {
                "model": config.claude_model,
                "max_tokens": config.max_tokens,
                "tools": anthropic_tools,
                "messages": messages
            }
        })

        text = ""; tool_results = []
        FOR block IN response["content"]
            IF block["type"] == "text" THEN text += block["text"]
            IF block["type"] == "tool_use" THEN
                result = registry.execute(block["name"], block["input"])
                tool_results.push(tool_result_block(block["id"], result))
            END IF
        END FOR

        IF tool_results.is_empty() OR response["stop_reason"] == "end_turn" THEN
            RETURN Ok(text)
        END IF

        messages.push({ "role": "assistant", "content": response["content"] })
        messages.push({ "role": "user", "content": tool_results })
        turns += 1
    END LOOP
END FUNCTION
```

### 2.6 Delegation Decision Engine

```
FUNCTION decide_delegation(task, config, claude_ok, flow_ok) -> Target
    FOR rule IN config.rules
        IF regex_match(rule.pattern, task) THEN
            MATCH rule.target
                Auto    => GOTO auto_decide
                Claude  => IF claude_ok THEN RETURN Claude ELSE RETURN Local
                Flow    => IF flow_ok   THEN RETURN Flow   ELSE RETURN Local
                Local   => RETURN Local
            END MATCH
        END IF
    END FOR
    RETURN Local  // no rule matched

    auto_decide:
    complexity = estimate_complexity(task)
    IF complexity < 0.3           THEN RETURN Local
    ELSE IF flow_ok AND > 0.7     THEN RETURN Flow
    ELSE IF claude_ok             THEN RETURN Claude
    ELSE                          RETURN Local
END FUNCTION
```

---

## 3. Architecture

### 3.1 Crate-Level Changes

| Crate | File | Change Type | Description |
|-------|------|-------------|-------------|
| **clawft-types** | `src/delegation.rs` | NEW | `DelegationConfig`, `DelegationRule`, `DelegationTarget` |
| **clawft-types** | `src/lib.rs` | MODIFY | Add `pub mod delegation;` |
| **clawft-types** | `src/config.rs` | MODIFY | Add `delegation: DelegationConfig` field to `Config` |
| **clawft-services** | `src/mcp/types.rs` | MODIFY | Add `JsonRpcNotification` struct |
| **clawft-services** | `src/mcp/transport.rs` | MODIFY | Add `send_notification()` to `McpTransport` trait; add `send_notification` impls (stdio framing unchanged -- already correct) |
| **clawft-services** | `src/mcp/mod.rs` | MODIFY | Add `send_raw()`, `send_notification()` methods to `McpClient`; add `pub mod session; pub mod provider; pub mod server; pub mod middleware; pub mod composite;` |
| **clawft-services** | `src/mcp/session.rs` | NEW | `McpSession` with initialize handshake, capability tracking |
| **clawft-services** | `src/mcp/provider.rs` | NEW (~80 lines) | `ToolProvider` trait, `ToolDefinition`, `CallToolResult`, `BuiltinToolProvider` (wraps ToolRegistry) |
| **clawft-services** | `src/mcp/server.rs` | NEW (~300 lines) | `McpServerShell`: generic MCP server handling JSON-RPC, newline-delimited stdio framing, initialize handshake, tools/list aggregation, tools/call routing via providers + middleware |
| **clawft-services** | `src/mcp/middleware.rs` | NEW (~150 lines) | `Middleware` trait + `SecurityGuard` (CommandPolicy/UrlPolicy enforcement), `PermissionFilter` (allowed_tools intersection), `ResultGuard` (truncation + sanitization), `AuditLog` |
| **clawft-services** | `src/mcp/composite.rs` | NEW (~120 lines) | `CompositeToolProvider`: aggregates multiple `ToolProvider`s, namespace prefixing, routing, `list_changed` notification support |
| **clawft-services** | `src/delegation/mod.rs` | NEW | `ClaudeDelegator` (Anthropic API bridge) |
| **clawft-services** | `src/delegation/schema.rs` | NEW | `openai_to_anthropic()`, `tool_result()`, `filter_tools()` |
| **clawft-services** | `src/delegation/engine.rs` | NEW | `DelegationEngine` (rule matcher) |
| **clawft-services** | `src/lib.rs` | MODIFY | Add `#[cfg(feature = "delegate")] pub mod delegation;` |
| **clawft-services** | `src/error.rs` | MODIFY | Add `DelegationFailed(String)` variant to `ServiceError` |
| **clawft-services** | `Cargo.toml` | MODIFY | Add `delegate` feature, `regex` dep |
| **clawft-tools** | `src/delegate_tool.rs` | NEW | `DelegateTaskTool` (Tool trait impl) |
| **clawft-tools** | `src/lib.rs` | MODIFY | Add `#[cfg(feature = "delegate")] pub mod delegate_tool;` |
| **clawft-tools** | `Cargo.toml` | MODIFY | Add `delegate` feature |
| **clawft-cli** | `src/mcp_tools.rs` | MODIFY | Use `McpSession` for initialization; add `register_delegation()` |
| **clawft-cli** | `src/commands/mcp_server.rs` | NEW | `weft mcp-server` command |
| **clawft-cli** | `src/main.rs` | MODIFY | Add `McpServer` subcommand |
| **clawft-cli** | `Cargo.toml` | MODIFY | Add `tool-delegate` feature |
| **tests/fixtures** | `config.json` | MODIFY | Add `delegation` section |

### 3.2 Module Dependency Graph

```
clawft-types
  src/delegation.rs          DelegationConfig, DelegationTarget, DelegationRule
  src/config.rs              Config.delegation: DelegationConfig
        |
        v
clawft-services
  src/mcp/
    types.rs                 JsonRpcNotification (new)
    transport.rs             McpTransport::send_notification() (new method)
                             StdioTransport: newline-delimited (unchanged, already correct)
    mod.rs                   McpClient::send_raw(), send_notification() (new)
    session.rs               McpSession::connect() (new file)
    provider.rs              ToolProvider trait, BuiltinToolProvider (new file)
    server.rs                McpServerShell (new file) -- generic MCP server
    middleware.rs             SecurityGuard, PermissionFilter, ResultGuard, AuditLog (new file)
    composite.rs             CompositeToolProvider (new file) -- multi-provider aggregation
  src/delegation/            (feature-gated: "delegate")
    mod.rs                   ClaudeDelegator
    schema.rs                openai_to_anthropic(), tool_result()
    engine.rs                DelegationEngine
        |
        v
clawft-cli
  src/mcp_tools.rs           register_mcp_tools() uses McpSession
                             register_delegation() (new fn)
  src/commands/mcp_server.rs weft mcp-server: builds BuiltinToolProvider +
                             CompositeToolProvider + middleware, runs McpServerShell
        |
        v
clawft-tools                 (feature-gated: "delegate")
  src/delegate_tool.rs       DelegateTaskTool

Cross-crate consumers (future):
  clawft-rvf-mcp (3F-RVF)   RvfToolProvider implements ToolProvider trait
                             Registers into CompositeToolProvider -- NO separate transport.rs needed
                             Gates on: McpServerShell + BuiltinToolProvider from Session 2
```

### 3.3 StdioTransport Framing: Confirmed Correct (Newline-Delimited JSON)

> **CORRECTION from original plan**: The original plan identified the current
> `StdioTransport` as broken and proposed rewriting it to use Content-Length
> framing. This was **incorrect**. The MCP 2025-06-18 specification uses
> **newline-delimited JSON** for the stdio transport. Content-Length framing
> is the Language Server Protocol (LSP) convention, not MCP.
>
> The Streamable HTTP transport uses standard HTTP framing (Content-Length on
> responses, chunked transfer encoding for SSE streams), but that is handled
> by the HTTP library, not by our transport code.
>
> The current `StdioTransport` implementation is correct:

```rust
// CURRENT (CORRECT for MCP 2025-06-18 stdio):
line.push('\n');
stdin.write_all(line.as_bytes()).await?;
// ...
stdout.read_line(&mut response_line).await?;
```

No rewrite needed. The only change to `transport.rs` in this phase is adding
`send_notification()` to the `McpTransport` trait (see section 3.4).

### 3.4 McpTransport Trait Extension

Current trait (line 20-24 of `transport.rs`):
```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
}
```

Extended:
```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    async fn send_notification(&self, notification: JsonRpcNotification) -> Result<()>;
}
```

For `StdioTransport`: write the notification as newline-delimited JSON, do not read a response.
For `HttpTransport`: POST the notification, ignore the response.
For `MockTransport`: store the notification for test assertions.

### 3.5 McpClient Extension

New methods on `McpClient` (in `crates/clawft-services/src/mcp/mod.rs`):

```rust
/// Send a raw request and return just the result Value.
pub async fn send_raw(&self, method: &str, params: Value) -> Result<Value> {
    let id = self.next_id();
    let request = JsonRpcRequest::new(id, method, params);
    let response = self.transport.send_request(request).await?;
    // Error handling same as call_tool()
    ...
}

/// Send a notification (no id, no response expected).
pub async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
    let notification = JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
    };
    self.transport.send_notification(notification).await
}
```

### 3.6 McpToolWrapper Content Extraction

Current execute (line 60-68 of `mcp_tools.rs`):
```rust
async fn execute(&self, args: Value) -> Result<Value, ToolError> {
    self.client.call_tool(&self.tool_def.name, args).await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
}
```

Extended to parse MCP content blocks:
```rust
async fn execute(&self, args: Value) -> Result<Value, ToolError> {
    let raw = self.client.call_tool(&self.tool_def.name, args).await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

    // Check isError flag
    if raw.get("isError").and_then(|v| v.as_bool()).unwrap_or(false) {
        let err_text = extract_text_content(&raw);
        return Err(ToolError::ExecutionFailed(err_text));
    }

    // Extract text from content blocks if present
    if let Some(content) = raw.get("content").and_then(|v| v.as_array()) {
        let text = content.iter()
            .filter_map(|block| {
                if block.get("type")?.as_str()? == "text" {
                    block.get("text")?.as_str().map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");
        return Ok(json!({"output": text}));
    }

    // Fallback: return raw value
    Ok(raw)
}
```

### 3.7 MCP Server Mode Architecture (Pluggable `McpServerShell`)

The MCP server is built on a pluggable provider architecture rather than a
monolithic function. This enables multiple tool sources (builtin, RVF, proxy,
delegation) to be composed into a single MCP endpoint.

#### Core Components

**`ToolProvider` trait** (`provider.rs`, ~80 lines):
```rust
#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn namespace(&self) -> &str;
    async fn list_tools(&self) -> Vec<ToolDefinition>;
    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError>;
}
```
- `BuiltinToolProvider`: wraps clawft's existing `ToolRegistry` (the tools from `weft mcp-server`). Namespace: `"builtin"` (or empty for backward compat).
- `RvfToolProvider` (3F-RVF, future): 11 RVF tools, in-process calls to rvf-runtime. Namespace: `"rvf"`.
- `ProxyToolProvider` (future): wraps external MCP servers via `McpClient`. Namespace: per-server name.
- `DelegationToolProvider` (future): Claude delegation bridge. Namespace: `"delegate"`.

**`CompositeToolProvider`** (`composite.rs`, ~120 lines):
- Holds `Vec<Box<dyn ToolProvider>>`
- `list_tools_all()`: iterates providers, prefixes tool names with `{namespace}__`
- `call_tool(namespaced_name, args)`: splits namespace, routes to correct provider
- Emits `notifications/tools/list_changed` when providers are added/removed

**`McpServerShell`** (`server.rs`, ~300 lines):
Generic server handling all MCP protocol concerns:
- Newline-delimited JSON parsing/serialization for stdio transport
- `initialize` handshake + capability negotiation (rejects pre-init requests)
- `tools/list`: aggregates across all registered providers via `CompositeToolProvider`
- `tools/call`: routes to correct provider (strip namespace, dispatch through middleware)
- Error formatting per JSON-RPC 2.0
- Takes `AsyncBufRead + AsyncWrite` for testability (not hardcoded to stdin/stdout)
- Single-threaded async; handles one request at a time (MCP spec does not require batching)

**Middleware pipeline** (`middleware.rs`, ~150 lines):
Applied between transport and tool dispatch. Resolves CRIT-01 and CRIT-02.

| Middleware | Purpose | Resolves |
|-----------|---------|----------|
| `SecurityGuard` | Enforces `CommandPolicy` + `UrlPolicy` on tool call args | CRIT-01 |
| `PermissionFilter` | Intersects available tools with session's `allowed_tools` | CRIT-01 |
| `ResultGuard` | `truncate_result(MAX_TOOL_RESULT_BYTES)` + `sanitize_content()` | NFR-007 |
| `AuditLog` | Logs tool invocations (tool name, duration, success/fail) | Observability |

Each middleware implements:
```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn filter_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition> { tools }
    async fn before_call(&self, name: &str, args: &Value) -> Result<(), ToolError> { Ok(()) }
    async fn after_call(&self, result: CallToolResult) -> CallToolResult { result }
}
```

#### Wiring (`weft mcp-server` command)

`crates/clawft-cli/src/commands/mcp_server.rs` (~150 lines -- reduced from 250, since protocol logic moved to `McpServerShell`):

1. Loads config and builds `ToolRegistry` (same as `weft agent`)
2. Wraps it in `BuiltinToolProvider`
3. Creates `CompositeToolProvider`, registers builtin provider
4. Builds middleware stack from config
5. Creates `McpServerShell::new(composite, middleware)`
6. Calls `shell.run(stdin, stdout)` -- runs until stdin closes

#### Cross-crate consumption (3F-RVF)

The `clawft-rvf-mcp` crate (Phase 3F-RVF Sprint 4) implements `RvfToolProvider` and
registers it into the `CompositeToolProvider`. It does **NOT** need its own `transport.rs`
or server loop -- it reuses `McpServerShell` from `clawft-services`. This is the key
dependency that Session 2 must deliver.

### 3.8 Feature Flag Layout

```toml
# clawft-services/Cargo.toml
[features]
default = []
delegate = ["dep:regex"]

# clawft-tools/Cargo.toml
[features]
default = ["native-exec"]
delegate = ["clawft-services/delegate"]

# clawft-cli/Cargo.toml
[features]
default = ["channels", "services"]
tool-delegate = ["clawft-tools/delegate", "clawft-services/delegate"]
```

The MCP core improvements (handshake, notifications, session) are NOT feature-gated -- they fix the existing MCP client for all users. The new server infrastructure (`ToolProvider`, `CompositeToolProvider`, `McpServerShell`, middleware pipeline) is also NOT feature-gated -- it is shared infrastructure consumed by 3F-RVF and the delegation system.

Only the delegation engine, Claude delegator, and delegate_task tool are behind `delegate` / `tool-delegate`.

The MCP server mode (`weft mcp-server` CLI command) is also NOT feature-gated (it uses the shared server infrastructure).

---

## 4. Refinement (TDD Test Plan)

### 4.1 Task 1: StdioTransport Framing (Confirmed Correct -- Minimal Changes)

**File**: `crates/clawft-services/src/mcp/transport.rs`

> The original plan called for a Content-Length framing rewrite. This is no
> longer needed -- MCP 2025-06-18 uses newline-delimited JSON for stdio, and
> the existing implementation is already correct. The only change is adding
> `send_notification()` impls (covered in Task 2).

**Tests** (in existing `mod tests` -- confirm existing behavior):

```
T-1.1  newline_delimited_write_format
       Create a mock stdin/stdout pair. Write a request.
       Verify output is JSON followed by "\n".

T-1.2  newline_delimited_read_parse
       Write "{json}\n" to mock stdout.
       Verify StdioTransport reads and parses it correctly.

T-1.3  empty_line_returns_eof_error
       Write empty string to mock stdout.
       Verify transport returns appropriate error.
```

**Implementation approach**: No rewrite needed. Existing tests should already cover T-1.1 and T-1.2. Add T-1.3 if not already present. The `Child`, `stdin`, `stdout` fields stay the same.

### 4.2 Task 2: JsonRpcNotification + McpTransport Extension

**Files**:
- `crates/clawft-services/src/mcp/types.rs` -- add `JsonRpcNotification`
- `crates/clawft-services/src/mcp/transport.rs` -- add `send_notification()` to trait + impls

**Tests**:

```
T-2.1  notification_serialization
       Serialize JsonRpcNotification, verify no "id" field in output.

T-2.2  notification_deserialization
       Deserialize `{"jsonrpc":"2.0","method":"notifications/initialized"}`.

T-2.3  mock_transport_send_notification
       MockTransport.send_notification() stores notification for assertions.

T-2.4  http_transport_send_notification
       (Optional) Verify HttpTransport sends POST for notification.
```

### 4.3 Task 3: McpClient send_raw and send_notification

**File**: `crates/clawft-services/src/mcp/mod.rs`

**Tests** (extend existing `mod tests`):

```
T-3.1  send_raw_returns_result_value
       MockTransport returns {"result": {"key": "val"}}.
       Verify send_raw("method", params) returns {"key": "val"}.

T-3.2  send_raw_propagates_error
       MockTransport returns error response.
       Verify send_raw returns Err(McpProtocol).

T-3.3  send_notification_does_not_expect_response
       MockTransport with 0 responses.
       Verify send_notification succeeds (does not try to read).
```

### 4.4 Task 4: McpSession with Initialize Handshake

**File**: `crates/clawft-services/src/mcp/session.rs` (NEW)

**Tests**:

```
T-4.1  connect_sends_initialize_then_notification
       MockTransport with [initialize_response].
       McpSession::connect() succeeds.
       Verify first request was "initialize" with protocolVersion.
       Verify notification "notifications/initialized" was sent.

T-4.2  connect_parses_server_capabilities
       Server returns capabilities: {tools: {}, resources: {}}.
       Verify supports_tools() == true, supports_resources() == true.

T-4.3  connect_parses_server_info
       Server returns serverInfo: {name: "test-server", version: "1.0"}.
       Verify session.server_info.name == "test-server".

T-4.4  connect_handles_missing_capabilities
       Server returns {} result.
       Verify session has default (empty) capabilities, does not panic.

T-4.5  connect_propagates_transport_error
       MockTransport with 0 responses.
       Verify connect() returns error.

T-4.6  list_tools_delegates_to_client
       After connect, call list_tools().
       Verify it works like McpClient.list_tools().

T-4.7  call_tool_delegates_to_client
       After connect, call call_tool("echo", args).
       Verify result is returned.
```

### 4.5 Task 5: McpToolWrapper Content Extraction

**File**: `crates/clawft-cli/src/mcp_tools.rs`

**Tests** (extend existing `mod tests`):

```
T-5.1  execute_extracts_text_content_blocks
       call_tool returns {"content": [{"type":"text","text":"hello"}], "isError": false}.
       Verify execute() returns {"output": "hello"}.

T-5.2  execute_handles_is_error_true
       call_tool returns {"content": [{"type":"text","text":"not found"}], "isError": true}.
       Verify execute() returns Err(ToolError::ExecutionFailed("not found")).

T-5.3  execute_handles_raw_value_fallback
       call_tool returns {"output": "hello"} (no content/isError).
       Verify execute() returns {"output": "hello"} unchanged.

T-5.4  execute_concatenates_multiple_text_blocks
       call_tool returns {"content": [{"type":"text","text":"a"},{"type":"text","text":"b"}]}.
       Verify execute() returns {"output": "ab"}.

T-5.5  execute_ignores_non_text_blocks
       call_tool returns {"content": [{"type":"image","url":"..."},{"type":"text","text":"ok"}]}.
       Verify execute() returns {"output": "ok"}.
```

### 4.6 Task 6: register_mcp_tools Uses McpSession

**File**: `crates/clawft-cli/src/mcp_tools.rs`

Modify `register_mcp_tools()` to use `McpSession::connect()` instead of raw `McpClient::new()` + `list_tools()`. This adds the initialize handshake.

**Tests**: The existing tests in this file use `TestTransport` with pre-programmed responses. Extend them to include initialize handshake responses:

```
T-6.1  register_mcp_tools_performs_handshake
       TestTransport programmed with [initialize_response, tools_list_response].
       Verify handshake occurs before tools/list.
```

### 4.7 Task 7: McpServerShell + BuiltinToolProvider + Middleware

This task is expanded from the original "MCP Server Mode" to deliver the full
pluggable infrastructure that 3F-RVF Sprint 4 depends on.

**Files**:
- `crates/clawft-services/src/mcp/provider.rs` (NEW) -- `ToolProvider` trait + `BuiltinToolProvider`
- `crates/clawft-services/src/mcp/composite.rs` (NEW) -- `CompositeToolProvider`
- `crates/clawft-services/src/mcp/middleware.rs` (NEW) -- Middleware trait + `SecurityGuard`, `PermissionFilter`, `ResultGuard`, `AuditLog`
- `crates/clawft-services/src/mcp/server.rs` (NEW) -- `McpServerShell`
- `crates/clawft-cli/src/commands/mcp_server.rs` (NEW) -- `weft mcp-server` CLI wiring

**Tests (provider.rs)**:

```
T-7.1   builtin_provider_namespace
        BuiltinToolProvider wrapping a ToolRegistry returns correct namespace.

T-7.2   builtin_provider_list_tools
        Register 3 tools in ToolRegistry, wrap in BuiltinToolProvider.
        Verify list_tools() returns 3 ToolDefinitions.

T-7.3   builtin_provider_call_tool
        Register "echo" tool, call via BuiltinToolProvider.
        Verify result matches direct ToolRegistry execution.

T-7.4   builtin_provider_call_unknown_tool
        Call nonexistent tool. Verify ToolError returned.
```

**Tests (composite.rs)**:

```
T-7.5   composite_list_namespaced
        Register two providers ("a" with [tool1], "b" with [tool2]).
        Verify list returns ["a__tool1", "b__tool2"].

T-7.6   composite_routes_to_correct_provider
        Two providers with same tool name "echo" but different namespaces.
        Verify "a__echo" routes to provider A, "b__echo" routes to provider B.

T-7.7   composite_unknown_namespace_errors
        Call "unknown__tool". Verify ToolError.
```

**Tests (middleware.rs)**:

```
T-7.8   security_guard_blocks_forbidden_command
        SecurityGuard with CommandPolicy blocking "rm -rf /".
        Verify before_call() returns ToolError for exec_shell with "rm -rf /".

T-7.9   security_guard_allows_permitted_command
        SecurityGuard allows "ls -la". Verify before_call() returns Ok.

T-7.10  permission_filter_removes_unlisted_tools
        PermissionFilter with allowed_tools=["read_file", "write_file"].
        Verify filter_tools() removes tools not in the list.

T-7.11  result_guard_truncates_large_output
        ResultGuard with max 100 bytes. Tool returns 500 bytes.
        Verify after_call() truncates to 100 bytes + "[truncated]".

T-7.12  audit_log_records_invocation
        Call a tool through AuditLog middleware.
        Verify log entry created with tool name and duration.
```

**Tests (server.rs)**:

```
T-7.13  shell_handles_initialize
        Send initialize request via mock reader/writer.
        Verify response includes protocolVersion and capabilities.

T-7.14  shell_rejects_pre_init_request
        Send tools/list before initialize.
        Verify JSON-RPC error -32002 "not initialized".

T-7.15  shell_handles_tools_list
        After initialize, send tools/list.
        Verify response contains registered tools with namespace prefixes.

T-7.16  shell_handles_tools_call
        After initialize, send tools/call for "builtin__echo" tool.
        Verify response contains tool result.

T-7.17  shell_handles_unknown_method
        Send request with unknown method.
        Verify JSON-RPC -32601 error response.

T-7.18  shell_handles_tools_call_error
        Call a tool that returns ToolError.
        Verify isError: true in response.

T-7.19  shell_applies_middleware_on_list
        Register PermissionFilter middleware that removes one tool.
        Verify tools/list response does not contain the filtered tool.

T-7.20  shell_applies_middleware_on_call
        Register SecurityGuard that blocks a specific tool.
        Verify tools/call returns error for blocked tool.

T-7.21  shell_applies_result_guard
        Register ResultGuard with small limit. Tool returns large output.
        Verify response content is truncated.
```

**Implementation note**: `McpServerShell::run()` takes `AsyncBufRead + AsyncWrite` trait
objects, not stdin/stdout, allowing tests to use `Cursor<Vec<u8>>`. The CLI wiring in
`mcp_server.rs` passes actual stdin/stdout.

### 4.8 Task 8: Delegation Config Types

**File**: `crates/clawft-types/src/delegation.rs` (NEW)

**Tests**:

```
T-8.1  delegation_config_defaults
       Deserialize "{}". Verify claude_enabled=false, max_turns=10, etc.

T-8.2  delegation_target_serde_roundtrip
       All variants (Local, Claude, ClaudeFlow, Auto) serialize/deserialize.

T-8.3  delegation_rule_serde
       Rule with pattern, target, tools roundtrips.

T-8.4  config_with_delegation_section
       Full Config JSON with delegation section deserializes.

T-8.5  config_without_delegation_uses_default
       Full Config JSON without delegation field deserializes with defaults.
```

### 4.9 Task 9: Schema Translation

**File**: `crates/clawft-services/src/delegation/schema.rs` (NEW, feature-gated)

**Tests**:

```
T-9.1  openai_to_anthropic_single_tool
       Input: [{"type":"function","function":{"name":"echo","description":"Echo","parameters":{...}}}]
       Output: [{"name":"echo","description":"Echo","input_schema":{...}}]

T-9.2  openai_to_anthropic_multiple_tools
       Input: 3 tools. Verify all 3 converted.

T-9.3  openai_to_anthropic_skips_malformed
       Input includes entry without "function" key. Verify it is skipped.

T-9.4  tool_result_success
       Ok(json!({"file":"content"})) -> {"type":"tool_result","tool_use_id":"id","content":"..."}

T-9.5  tool_result_error
       Err("not found") -> {"type":"tool_result","tool_use_id":"id","is_error":true,"content":"not found"}

T-9.6  filter_tools_include
       3 tools, include=["echo"]. Result has 1 tool.

T-9.7  filter_tools_exclude
       3 tools, exclude=["exec_shell"]. Result has 2 tools.

T-9.8  filter_tools_empty_include_means_all
       3 tools, include=[]. Result has 3 tools.
```

### 4.10 Task 10: Delegation Engine

**File**: `crates/clawft-services/src/delegation/engine.rs` (NEW, feature-gated)

**Tests**:

```
T-10.1  rule_matching_simple
        Rule: pattern="orchestrat", target=ClaudeFlow.
        Task: "orchestrate a multi-agent workflow". Match -> ClaudeFlow.

T-10.2  first_match_wins
        Rules: [pattern=".*claude.*"->Claude, pattern=".*"->Local].
        Task: "ask claude about X". Matches first rule -> Claude.

T-10.3  auto_simple_task_returns_local
        Task: "hello" (short, no keywords). Auto -> Local.

T-10.4  auto_complex_task_with_claude
        Task: "analyze and compare these two approaches...". Auto -> Claude.

T-10.5  fallback_when_claude_unavailable
        Rule matches Claude but claude_available=false. Returns Local.

T-10.6  fallback_when_flow_unavailable
        Rule matches ClaudeFlow but flow_available=false. Returns Local.

T-10.7  no_rule_match_returns_local
        Rules: [pattern="^foo$"->Claude]. Task: "bar". Returns Local.

T-10.8  invalid_regex_skipped
        Rule with pattern="[invalid". Verify no panic, rule skipped.

T-10.9  estimate_complexity_keywords
        Task with "orchestrate" + "research" -> high score (>0.5).

T-10.10 estimate_complexity_long_text
        Task with 150 words, no keywords -> moderate score (~0.2).
```

### 4.11 Task 11: ClaudeDelegator

**File**: `crates/clawft-services/src/delegation/mod.rs` (NEW, feature-gated)

Testing requires mocking the Anthropic HTTP endpoint. Use `mockito` or a custom mock server.

**Tests**:

```
T-11.1  single_turn_text_response
        Mock server returns text response (no tool_use). Verify text returned.

T-11.2  tool_use_then_text
        Mock server returns tool_use on call 1, text on call 2.
        Verify tool was executed locally and final text returned.

T-11.3  max_turns_exceeded
        Mock server always returns tool_use.
        Verify error after max_turns.

T-11.4  api_error_returns_error
        Mock server returns 401. Verify DelegationFailed error.

T-11.5  excluded_tools_not_in_schemas
        Config excludes "exec_shell". Verify Anthropic request tools array
        does not contain "exec_shell".
```

### 4.12 Task 12: DelegateTaskTool

**File**: `crates/clawft-tools/src/delegate_tool.rs` (NEW, feature-gated)

**Tests**:

```
T-12.1  name_returns_delegate_task
T-12.2  parameters_is_valid_schema
T-12.3  execute_missing_task_returns_invalid_args
T-12.4  execute_target_claude_delegates
T-12.5  execute_target_claude_flow_delegates
T-12.6  execute_target_auto_uses_engine
T-12.7  execute_no_claude_returns_error
```

### 4.13 Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| AC-1 | `cargo check --workspace` passes without delegate feature | CI |
| AC-2 | `cargo check --workspace --features tool-delegate` passes | CI |
| AC-3 | `cargo clippy --workspace -- -D warnings` clean | CI |
| AC-4 | `cargo test --workspace` passes (no delegate) | CI |
| AC-5 | `cargo test --workspace --features tool-delegate` passes | CI |
| AC-6 | `cargo fmt --all -- --check` clean | CI |
| AC-7 | `cargo check -p clawft-wasm --target wasm32-wasip2` passes | CI |
| AC-8 | MCP client connects to real claude-flow (manual test) | Manual |
| AC-9 | `weft mcp-server` responds to initialize + tools/list (manual test) | Manual |
| AC-10 | Config with MCP servers parses correctly | Unit test |
| AC-11 | Config with delegation section parses correctly | Unit test |
| AC-12 | ~~Content-Length framing round-trips correctly~~ Newline-delimited stdio works (already does) | Unit test |
| AC-13 | `ToolProvider` trait + `BuiltinToolProvider` list and dispatch correctly | Unit test |
| AC-14 | `CompositeToolProvider` routes namespaced tool calls to correct provider | Unit test |
| AC-15 | `SecurityGuard` blocks forbidden commands/URLs in tool args | Unit test |
| AC-16 | `PermissionFilter` filters tools/list to allowed_tools | Unit test |
| AC-17 | `ResultGuard` truncates tool output at MAX_TOOL_RESULT_BYTES | Unit test |
| AC-18 | `McpServerShell` rejects pre-initialize requests | Unit test |

---

## 5. Completion

### 5.1 Implementation Order

The tasks build on each other. Here is the dependency-ordered execution plan:

**Session 1: MCP Protocol Fixes** (FR-001, FR-002, FR-003)
1. Task 2: `JsonRpcNotification` type + `McpTransport::send_notification()` method
2. Task 1: Confirm `StdioTransport` newline-delimited framing is correct (no rewrite needed); add `send_notification` impls
3. Task 3: `McpClient::send_raw()` and `send_notification()`
4. Task 4: `McpSession` with initialize handshake
5. Run: `cargo test -p clawft-services`

**Session 2: MCP Server Infrastructure** (FR-004, FR-005 + pluggable architecture)

> **KEY DELIVERABLE**: Session 2 delivers the shared `McpServerShell` infrastructure
> that 3F-RVF Sprint 4 gates on. The `ToolProvider` trait, `CompositeToolProvider`,
> and middleware pipeline must be complete and tested by end of this session.

6. Task 7a: `ToolProvider` trait + `BuiltinToolProvider` (`provider.rs`)
7. Task 7b: `CompositeToolProvider` (`composite.rs`)
8. Task 7c: Middleware pipeline -- `SecurityGuard`, `PermissionFilter`, `ResultGuard`, `AuditLog` (`middleware.rs`)
9. Task 7d: `McpServerShell` (`server.rs`)
10. Task 5: `McpToolWrapper` content extraction
11. Task 6: `register_mcp_tools()` uses `McpSession`
12. Task 7e: `weft mcp-server` CLI wiring (`commands/mcp_server.rs`)
13. Run: `cargo test -p clawft-services && cargo test -p clawft-cli`

**Session 3: Delegation Engine** (FR-006, FR-007)
14. Task 8: `DelegationConfig` types
15. Task 9: Schema translation
16. Task 10: `DelegationEngine`
17. Task 11: `ClaudeDelegator`
18. Run: `cargo test -p clawft-types && cargo test -p clawft-services --features delegate`

**Session 4: Wiring + Validation** (FR-008)
19. Task 12: `DelegateTaskTool`
20. CLI wiring: `register_delegation()` in `mcp_tools.rs`
21. Update test fixture `config.json`
22. Full validation suite

### 5.2 File Manifest

**New files** (estimated lines):

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-types/src/delegation.rs` | ~100 | Config types |
| `crates/clawft-services/src/mcp/session.rs` | ~180 | MCP session with handshake |
| `crates/clawft-services/src/mcp/provider.rs` | ~80 | `ToolProvider` trait, `ToolDefinition`, `CallToolResult`, `BuiltinToolProvider` |
| `crates/clawft-services/src/mcp/server.rs` | ~300 | `McpServerShell`: generic MCP server with protocol handling |
| `crates/clawft-services/src/mcp/middleware.rs` | ~150 | `Middleware` trait + `SecurityGuard`, `PermissionFilter`, `ResultGuard`, `AuditLog` |
| `crates/clawft-services/src/mcp/composite.rs` | ~120 | `CompositeToolProvider`: multi-provider aggregation + namespace routing |
| `crates/clawft-services/src/delegation/mod.rs` | ~200 | ClaudeDelegator |
| `crates/clawft-services/src/delegation/schema.rs` | ~120 | Schema translation |
| `crates/clawft-services/src/delegation/engine.rs` | ~170 | DelegationEngine |
| `crates/clawft-tools/src/delegate_tool.rs` | ~130 | DelegateTaskTool |
| `crates/clawft-cli/src/commands/mcp_server.rs` | ~150 | CLI wiring for `weft mcp-server` (reduced -- protocol logic in `McpServerShell`) |

**Modified files**:

| File | Change |
|------|--------|
| `crates/clawft-types/src/lib.rs` | Add `pub mod delegation;` |
| `crates/clawft-types/src/config.rs` | Add `delegation: DelegationConfig` to `Config` |
| `crates/clawft-services/src/mcp/types.rs` | Add `JsonRpcNotification` |
| `crates/clawft-services/src/mcp/transport.rs` | Add `send_notification()` to trait + impls; `MockTransport` stores notifications (NO stdio framing rewrite -- already correct) |
| `crates/clawft-services/src/mcp/mod.rs` | Add `pub mod session; pub mod provider; pub mod server; pub mod middleware; pub mod composite;`; add `send_raw()`, `send_notification()` to `McpClient` |
| `crates/clawft-services/src/error.rs` | Add `DelegationFailed(String)` |
| `crates/clawft-services/src/lib.rs` | Add `#[cfg(feature = "delegate")] pub mod delegation;` |
| `crates/clawft-services/Cargo.toml` | Add `delegate` feature + `regex` dep |
| `crates/clawft-tools/src/lib.rs` | Add `#[cfg(feature = "delegate")] pub mod delegate_tool;` |
| `crates/clawft-tools/Cargo.toml` | Add `delegate` feature |
| `crates/clawft-cli/src/mcp_tools.rs` | Use `McpSession`; add `register_delegation()` |
| `crates/clawft-cli/src/main.rs` | Add `McpServer` subcommand |
| `crates/clawft-cli/Cargo.toml` | Add `tool-delegate` feature |
| `tests/fixtures/config.json` | Add `delegation` section |

**Total**: ~1700 lines new code (+550 from pluggable infra), ~200 lines modifications, 11 new files (+4), 14 modified files.

### 5.3 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| ~~Content-Length framing breaks existing mock tests~~ | ~~High~~ | ~~Low~~ | ~~N/A -- no framing rewrite needed (MCP uses newline-delimited JSON for stdio)~~ |
| Real MCP servers reject our initialize params | Medium | Medium | Test with real claude-flow before merging; version negotiation |
| StdioTransport read hangs if server sends partial data | Medium | Medium | Add timeout to read operations; configurable per-server |
| Anthropic API format changes | Low | Medium | Pin anthropic-version header; isolated schema translation |
| claude-flow MCP server not installed | High | Low | Graceful degradation: log warning, skip, continue |
| Feature flag leaks into WASM | Very Low | Medium | All delegation behind cfg(feature); WASM crate has no dependency |
| ToolRegistry snapshot stale in DelegateTaskTool | Medium | Low | Snapshot taken at startup; tools registered after are not visible |
| API key exposure in logs | Low | High | Key masking in Debug impls; never log request headers |

### 5.4 Validation Commands

```bash
cd /home/aepod/dev/clawft

# ---- Phase 1: Core MCP fixes (no feature flag) ----
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace

# ---- Phase 2: With delegation feature ----
cargo check --workspace --features tool-delegate
cargo clippy --workspace --features tool-delegate -- -D warnings
cargo test --workspace --features tool-delegate

# ---- Targeted test runs ----
cargo test -p clawft-services -- mcp::transport
cargo test -p clawft-services -- mcp::session
cargo test -p clawft-services -- mcp::provider
cargo test -p clawft-services -- mcp::composite
cargo test -p clawft-services -- mcp::middleware
cargo test -p clawft-services -- mcp::server
cargo test -p clawft-services --features delegate -- delegation
cargo test -p clawft-types -- delegation
cargo test -p clawft-cli -- mcp_tools
cargo test -p clawft-tools --features delegate -- delegate_tool

# ---- WASM unaffected ----
cargo check -p clawft-wasm --target wasm32-wasip2

# ---- Format ----
cargo fmt --all -- --check

# ---- Manual integration test ----
# Start claude-flow MCP server:
#   npx @claude-flow/cli@latest mcp start
# Then:
#   cargo run -p clawft-cli -- agent -m "list your tools"
# Verify claude-flow tools appear in registry

# MCP server mode test:
#   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}' | \
#     cargo run -p clawft-cli -- mcp-server
```

### 5.5 Edge Cases

- Empty `mcp_servers` config: no MCP clients created, no error
- MCP server returns empty `tools` array: no tools registered, log info
- MCP server crashes mid-handshake: transport error, server skipped
- MCP server returns protocol version we do not support: log warning, proceed with basic capabilities
- `tools/call` returns deeply nested content blocks: extract text, ignore others
- `tools/call` returns `isError: true` with no content: return generic error message
- ~~`Content-Length` header with value larger than available data~~: N/A (stdio uses newline-delimited, not Content-Length)
- CompositeToolProvider with 0 providers: `tools/list` returns empty array, `tools/call` returns "unknown namespace" error
- Middleware rejects tool call: McpServerShell returns isError:true before reaching provider
- Provider added at runtime: CompositeToolProvider emits `notifications/tools/list_changed`
- Multiple MCP servers with same tool name: namespaced as `{server}__{tool}`, no collision
- Claude delegation with 0 tools: sends empty tools array, Claude responds with text only
- Delegation rule with empty pattern: matches everything (regex `""` matches all strings)
- Config `delegation` section missing entirely: `DelegationConfig::default()` used, all disabled

### 5.6 Deliverables Checklist

**Session 1: MCP Protocol Fixes**
- [ ] `JsonRpcNotification` type in `types.rs`
- [ ] `McpTransport::send_notification()` on trait + all impls
- [ ] ~~`StdioTransport` Content-Length framing~~ (NOT NEEDED -- already correct for MCP)
- [ ] `McpClient::send_raw()` and `send_notification()`
- [ ] `McpSession` with initialize handshake (`session.rs`)

**Session 2: MCP Server Infrastructure (3F-RVF gate)**
- [ ] `ToolProvider` trait + `BuiltinToolProvider` (`provider.rs`)
- [ ] `CompositeToolProvider` (`composite.rs`)
- [ ] Middleware pipeline: `SecurityGuard`, `PermissionFilter`, `ResultGuard`, `AuditLog` (`middleware.rs`)
- [ ] `McpServerShell` (`server.rs`)
- [ ] `McpToolWrapper` content block extraction
- [ ] `register_mcp_tools()` uses `McpSession`
- [ ] `weft mcp-server` CLI wiring (`commands/mcp_server.rs`)
- [ ] Update clawft/docs/ with ToolProvider architecture: architecture/overview.md (add Pluggable MCP Architecture section), guides/tool-calls.md (add ToolProvider section), reference/tools.md (add custom provider section), development/contributing.md (add ToolProvider implementation guide)

**Session 3: Delegation Engine**
- [ ] `DelegationConfig` types
- [ ] Schema translation (`openai_to_anthropic`, `tool_result`)
- [ ] `DelegationEngine`
- [ ] `ClaudeDelegator`

**Session 4: Wiring + Validation**
- [ ] `DelegateTaskTool`
- [ ] CLI wiring (`register_delegation()`)
- [ ] Feature flags configured in Cargo.toml files
- [ ] Test fixture updated

**All Sessions**
- [ ] All existing tests pass
- [ ] All new tests pass
- [ ] WASM build unaffected

### 5.7 Branch Strategy

- **Feature Branch**: `feature/phase-3h-tool-delegation`
- **PR Target**: `main`
- **Commit order**: One commit per session (4 total), each passing CI
  - `feat(mcp): add notifications, send_raw, and McpSession with initialize handshake`
  - `feat(mcp): add pluggable McpServerShell with ToolProvider, CompositeToolProvider, middleware pipeline`
  - `feat(delegation): add Claude delegator, engine, and schema translation`
  - `feat(delegation): wire delegate_task tool and CLI integration`

---

## Appendix A: Protocol References

### A.1 MCP Transport Framing

> **CORRECTION**: The original plan incorrectly stated MCP uses Content-Length
> framing for stdio. That is the LSP (Language Server Protocol) convention.
> MCP 2025-06-18 uses **newline-delimited JSON** for stdio transport.

**Stdio transport** (newline-delimited JSON):
```
{"jsonrpc":"2.0","id":1,"method":"initialize",...}\n
{"jsonrpc":"2.0","id":1,"result":{...}}\n
```

Each message is a single line of JSON terminated by `\n`. No headers.

**Streamable HTTP transport**: Uses standard HTTP framing (Content-Length on
responses, chunked transfer encoding for SSE event streams). This is handled
by the HTTP library, not our transport code.

### A.2 MCP Initialize Handshake

**Client sends**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": { "tools": {} },
    "clientInfo": { "name": "clawft", "version": "0.1.0" }
  }
}
```

**Server responds**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "tools": { "listChanged": true },
      "resources": {}
    },
    "serverInfo": { "name": "claude-flow", "version": "3.0.0" }
  }
}
```

**Client sends notification** (no id, no response):
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

### A.3 MCP tools/call Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      { "type": "text", "text": "File contents here..." }
    ],
    "isError": false
  }
}
```

Error case:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      { "type": "text", "text": "File not found: /missing.txt" }
    ],
    "isError": true
  }
}
```

### A.4 Anthropic Tool Use Protocol

**Request** (POST `https://api.anthropic.com/v1/messages`):
```json
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 4096,
  "tools": [
    {
      "name": "read_file",
      "description": "Read a file",
      "input_schema": { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }
    }
  ],
  "messages": [
    { "role": "user", "content": "Read /etc/hostname" }
  ]
}
```

**Response with tool_use**:
```json
{
  "content": [
    { "type": "tool_use", "id": "toolu_abc123", "name": "read_file", "input": { "path": "/etc/hostname" } }
  ],
  "stop_reason": "tool_use"
}
```

**Tool result** (sent as next user message):
```json
{
  "role": "user",
  "content": [
    { "type": "tool_result", "tool_use_id": "toolu_abc123", "content": "myhost.local" }
  ]
}
```

### A.5 Schema Translation (OpenAI -> Anthropic)

OpenAI format (from `ToolRegistry::schemas()`):
```json
{ "type": "function", "function": { "name": "X", "description": "Y", "parameters": {...} } }
```

Anthropic format:
```json
{ "name": "X", "description": "Y", "input_schema": {...} }
```

Translation: unwrap `function` envelope, rename `parameters` to `input_schema`.

### A.6 Reference Sources

- [MCP Specification 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18)
- [Anthropic Tool Use Docs](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- [MCP Transports (stdio, HTTP)](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [Official Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
- [claude-flow MCP Tools](https://github.com/ruvnet/claude-flow)
