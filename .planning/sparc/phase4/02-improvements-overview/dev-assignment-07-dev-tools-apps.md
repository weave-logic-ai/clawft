# Development Assignment: Element 07 -- Dev Tools & Applications

**Workstream**: F (Software Dev & App Tooling)
**Timeline**: Weeks 5-10
**Element Orchestrator**: `.planning/sparc/07-dev-tools-apps/00-orchestrator.md`
**Dev Stream Branch**: `sprint/phase-5-F`

---

## Prerequisites

Before starting any work in this element, the following must be merged to the integration branch:

| Prerequisite | Element | Description | Blocks |
|-------------|---------|-------------|--------|
| C1 Plugin Traits | 04 | `Tool` trait in `clawft-plugin` | F1-F7 (all tool plugins) |
| D9 MCP Transport | 05 | MCP transport concurrency | F9a, F9b (MCP client) |

---

## Crate Structure

Dev tool plugins are implemented as **separate workspace crates**, one per tool, behind CLI feature flags. This keeps compilation parallel and avoids a monolithic plugin crate.

| Crate | Tool | Feature Flag |
|-------|------|-------------|
| `crates/clawft-plugin-git` | F1: Git operations | `plugin-git` |
| `crates/clawft-plugin-cargo` | F2: Cargo/build | `plugin-cargo` |
| `crates/clawft-plugin-treesitter` | F3: Code analysis | `plugin-treesitter` |
| `crates/clawft-plugin-browser` | F4: Browser CDP | `plugin-browser` |
| `crates/clawft-plugin-calendar` | F5: Calendar | `plugin-calendar` |
| `crates/clawft-plugin-oauth2` | F6: OAuth2 helper | `plugin-oauth2` |
| `crates/clawft-plugin-containers` | F7: Docker/Podman | `plugin-containers` |

F8 and F9 are NOT separate crates. They extend `crates/clawft-services/src/mcp/` (MCP infrastructure layer).

---

## Tool Permission Model

Each dev tool plugin declares its permission requirements in the plugin manifest:
- `permissions.filesystem` -- read/write paths
- `permissions.network` -- allowed domains/ports
- `permissions.shell` -- whether subprocess execution is allowed

The K3 sandbox enforces these permissions at runtime. Permissions must be specified in the manifest before implementation begins.

---

## Tool Plugin <-> Memory Contract

From cross-element integration spec (Section 3.1):

Tool plugins use `ToolContext::key_value_store` for state persistence. The `KeyValueStore` trait lives in `clawft-plugin`:

```rust
pub trait KeyValueStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
}
```

Plugin crates depend on `clawft-plugin` (for the trait) only, NEVER on `clawft-core` memory modules. The `AgentLoop` backs `KeyValueStore` with `~/.clawft/agents/<agentId>/tool_state/<plugin_name>/`.

---

## Unit 1: F1 Git + F2 Cargo + F6 OAuth2 + F9a MCP Client (Week 5-7)

### F1: Git Tool Plugin

**New Crate**: `crates/clawft-plugin-git/`
**Feature Flag**: `plugin-git`
**Key Dependency**: `git2` crate

**Crate Structure**:
```
crates/clawft-plugin-git/
  Cargo.toml
  src/
    lib.rs          -- Tool trait implementation, tool registration
    operations.rs   -- clone, commit, branch, diff, blame, log, PR
    types.rs        -- GitConfig, operation result types
```

**Operations to Implement**:
| Tool Name | Description | git2 API |
|-----------|-------------|----------|
| `git_clone` | Clone a repository | `Repository::clone()` |
| `git_commit` | Stage files and commit | `Index::add_path()`, `Signature::now()`, `Repository::commit()` |
| `git_branch` | Create/switch branches | `Repository::branch()`, `Repository::set_head()` |
| `git_diff` | Show changes | `Repository::diff_index_to_workdir()` |
| `git_blame` | File blame | `Repository::blame_file()` |
| `git_log` | Commit history | `Repository::revwalk()` |
| `git_status` | Working tree status | `Repository::statuses()` |

**Trait Implementation**: Each operation implements the `Tool` trait from `clawft-plugin` (C1). Tool definitions include MCP-compatible JSON Schema for `input_schema`.

**Permission Manifest**:
```json
{
  "permissions": {
    "filesystem": { "read": true, "write": true, "paths": ["$WORKSPACE"] },
    "network": { "allowed": ["github.com", "gitlab.com"] },
    "shell": false
  }
}
```

**Acceptance Criteria**:
- [ ] `weft` can create git branches and commits via tool call
- [ ] Clone, commit, branch, diff, blame, log, status all functional
- [ ] Implements `Tool` plugin trait
- [ ] MCP-exposed: tools appear in `tools/list` response
- [ ] Feature-gated behind `plugin-git`

### F2: Cargo/Build Integration

**New Crate**: `crates/clawft-plugin-cargo/`
**Feature Flag**: `plugin-cargo`
**Transport**: subprocess (`tokio::process::Command`)

**Operations**:
| Tool Name | Command | Description |
|-----------|---------|-------------|
| `cargo_build` | `cargo build` | Build the project |
| `cargo_test` | `cargo test` | Run tests |
| `cargo_clippy` | `cargo clippy` | Lint check |
| `cargo_check` | `cargo check` | Type check |
| `cargo_publish` | `cargo publish` | Publish to crates.io |

**Implementation Notes**:
- Use `tokio::process::Command` for subprocess execution
- Capture stdout/stderr and return as tool result
- Support `--release`, `--workspace`, `-p <crate>` flags via tool arguments
- Parse JSON output (`--message-format=json`) for structured results where available

**Security**: Command arguments must be validated -- no shell injection through user-provided package names or flags. Build the command programmatically, never via string interpolation.

**Acceptance Criteria**:
- [ ] Build, test, clippy, check, publish all functional
- [ ] Implements `Tool` plugin trait
- [ ] Structured output from `--message-format=json` where available
- [ ] Feature-gated behind `plugin-cargo`
- [ ] Command injection prevention on all arguments

### F6: Generic REST + OAuth2 Helper

**New Crate**: `crates/clawft-plugin-oauth2/`
**Feature Flag**: `plugin-oauth2`
**Key Dependency**: `oauth2` crate

**CRITICAL**: F6 is a dependency for E5a (Google Chat channel) in Element 06. This makes F6 one of the highest-priority items in Element 07.

**Functionality**:
1. **OAuth2 Authorization Code Flow**: Redirect URI callback, token exchange, refresh
2. **OAuth2 Client Credentials Flow**: Service account authentication
3. **Token Persistence**: Store tokens in `~/.clawft/tokens/` with 0600 permissions
4. **Token Refresh**: Automatic refresh before expiration, persist rotated tokens
5. **Generic REST Client**: Authenticated HTTP client using stored tokens

**Tool Interface**:
| Tool Name | Description |
|-----------|-------------|
| `oauth2_authorize` | Start authorization flow, return redirect URL |
| `oauth2_callback` | Handle callback with auth code, exchange for tokens |
| `oauth2_refresh` | Manually refresh tokens |
| `rest_request` | Make authenticated REST request using stored tokens |

**Provider Presets**:
- Google (Gmail, Calendar, Chat, Drive)
- Microsoft (Azure AD, Graph API)
- Generic (custom authorization/token endpoints)

**Security Requirements**:
- OAuth2 `state` parameter for CSRF protection (MUST)
- Tokens stored with 0600 permissions
- Rotated refresh tokens persisted immediately (prevent loss on crash)
- `client_secret` via `SecretRef` (env var name, not plaintext)

**Cross-Element Dependencies**:
- **E2 (Email)**: Gmail OAuth2 depends on F6
- **E5a (Google Chat)**: Google Workspace API auth depends on F6
- **F5 (Calendar)**: Google Calendar auth depends on F6

**Cross-Element Integration Test** (from `01-cross-element-integration.md`):
> "Email Channel -> OAuth2 Helper" (Week 7, P0): Configure Gmail via E2, verify F6 OAuth2 flow completes, token stored in agent workspace.

**Acceptance Criteria**:
- [ ] OAuth2 authorization code flow works for at least one provider (Google)
- [ ] Token refresh and persistence to `~/.clawft/tokens/` (0600 permissions)
- [ ] `state` parameter included in all authorization requests
- [ ] Generic REST client makes authenticated requests
- [ ] Implements `Tool` plugin trait
- [ ] Feature-gated behind `plugin-oauth2`

### F9a: Core MCP Client Library (MVP Scope)

**Location**: `crates/clawft-services/src/mcp/client.rs` (new file)
**NOT a separate crate** -- extends the MCP infrastructure in `clawft-services`

**Scope** (minimal viable client):
- Connect to a single configured MCP server
- List available tools
- Invoke tools with JSON-RPC
- Return results

**Interface**:
```rust
pub struct MpcClient {
    transport: Box<dyn McpTransport>,
    server_info: Option<ServerInfo>,
}

impl MpcClient {
    pub async fn connect(config: McpServerConfig) -> Result<Self>;
    pub async fn list_tools(&self) -> Result<Vec<ToolSchema>>;
    pub async fn invoke(&self, tool_name: &str, params: Value) -> Result<Value>;
}
```

**Existing MCP Infrastructure** (`crates/clawft-services/src/mcp/`):

The MCP module already has:
- `transport.rs`: `McpTransport` trait with `StdioTransport` and `HttpTransport` implementations
- `types.rs`: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification` types
- `server.rs`: `McpServerShell` (server-side, F9a needs the client-side counterpart)

F9a builds on these existing types. The `MpcClient` uses `McpTransport` to send JSON-RPC requests.

**Current Transport Code** (`crates/clawft-services/src/mcp/transport.rs:20-27`):

```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()>;
}
```

**MCP Tool Namespace Convention**: External MCP tools use the `mcp:<server-name>:<tool-name>` namespace to avoid name collisions with local tools. When F9a connects to an external server, all tools are registered in the `ToolRegistry` with this prefix.

**Protocol Flow**:
1. Establish transport (stdio or HTTP)
2. Send `initialize` request with client capabilities
3. Send `notifications/initialized`
4. Send `tools/list` to discover available tools
5. Invoke tools via `tools/call`

**Dependencies**:
- D9 (MCP transport concurrency) must land first -- F9a needs non-serialized concurrent calls

**Security Requirements**:
- MCP stdio child processes MUST NOT inherit secret environment variables (construct minimal env explicitly)
- External MCP server tools MUST be tagged as "untrusted" in the tool registry
- Validate stdio command paths

**Acceptance Criteria**:
- [ ] MPC client connects to a single external MCP server
- [ ] `list_tools()` returns tool schemas from the remote server
- [ ] `invoke()` calls a remote tool and returns the result
- [ ] Tools registered with `mcp:<server-name>:<tool-name>` namespace
- [ ] Stdio child processes inherit minimal env (no secrets)
- [ ] External tools tagged as "untrusted"

**Test Requirements**:
- Unit test: initialize handshake using `MockTransport`
- Unit test: `list_tools` parses tool schemas
- Unit test: `invoke` sends correct JSON-RPC and parses response
- Unit test: error handling for failed connections
- Unit test: namespace prefixing on registered tools

---

## Unit 2: F3 Tree-sitter + F4 Browser CDP (Week 6-8)

### F3: Code Analysis via Tree-sitter

**New Crate**: `crates/clawft-plugin-treesitter/`
**Feature Flag**: `plugin-treesitter`
**Key Dependency**: `tree-sitter` crate + language grammars

**Implementation Notes**:
- Native-only, NO WASM variant
- Grammars as optional sub-features: `tree-sitter-rust`, `tree-sitter-typescript`, `tree-sitter-python`, etc.
- Each grammar is an optional dependency to keep binary size small

**Cargo.toml pattern**:
```toml
[features]
default = []
rust = ["tree-sitter-rust"]
typescript = ["tree-sitter-typescript"]
python = ["tree-sitter-python"]
javascript = ["tree-sitter-javascript"]
```

**Operations**:
| Tool Name | Description |
|-----------|-------------|
| `ts_parse` | Parse file and return AST |
| `ts_query` | Run tree-sitter query on parsed AST |
| `ts_complexity` | Calculate cyclomatic complexity |
| `ts_symbols` | List functions, structs, classes |
| `ts_references` | Find references to a symbol |

**Acceptance Criteria**:
- [ ] Parses Rust, TypeScript, Python source files
- [ ] AST query returns matching nodes
- [ ] Complexity metric calculation
- [ ] Implements `Tool` plugin trait
- [ ] Feature-gated with per-language sub-features

**Test Requirements**:
- Unit test: parse a Rust file, verify AST structure
- Unit test: query for function definitions
- Unit test: complexity calculation on known code snippet
- Fuzz test: C grammar FFI bindings (risk mitigation for memory safety)

### F4: Browser CDP Automation

**New Crate**: `crates/clawft-plugin-browser/`
**Feature Flag**: `plugin-browser`
**Key Dependency**: `chromiumoxide` crate

**Operations**:
| Tool Name | Description |
|-----------|-------------|
| `browser_navigate` | Navigate to URL |
| `browser_screenshot` | Capture page screenshot |
| `browser_fill` | Fill a form field |
| `browser_click` | Click an element |
| `browser_get_text` | Extract text from element |
| `browser_evaluate` | Run JavaScript (sandboxed) |

**BrowserSandboxConfig** (from orchestrator Section 6):

```rust
pub struct BrowserSandboxConfig {
    /// Domains the browser is allowed to navigate to. Empty = block all.
    pub allowed_domains: Vec<String>,
    /// Maximum number of concurrent browser pages per agent.
    pub max_concurrent_pages: u32,   // default: 2
    /// Maximum browser session lifetime before forced termination.
    pub session_lifetime: Duration,  // default: 300s
    /// Maximum memory for the browser process.
    pub max_memory_mb: u64,          // default: 512
    /// Cookie/storage policy: clear between sessions.
    pub clear_state_between_sessions: bool, // default: true
}
```

This config is part of the C1 plugin manifest extension. The K3 sandbox enforces it at runtime.

**Security Requirements (CRITICAL)**:
- [ ] Block `file://`, `data://`, and `javascript://` URL schemes
- [ ] Clear state (cookies, storage, sessions) between sessions when `clear_state_between_sessions` is true
- [ ] Enforce `allowed_domains` -- reject navigation to unlisted domains
- [ ] Enforce `max_concurrent_pages` per agent
- [ ] Force-kill browser process after `session_lifetime` expires
- [ ] Memory limit enforcement via `max_memory_mb`

**Resource Limits** (risk mitigation):
- Default: 1 browser session per agent
- Maximum 2 concurrent pages per session
- 5-minute session timeout

**Acceptance Criteria**:
- [ ] Browser CDP takes screenshots and fills forms
- [ ] `BrowserSandboxConfig` enforced at runtime
- [ ] URL scheme blocking (file://, data://, javascript://)
- [ ] State clearing between sessions
- [ ] Implements `Tool` plugin trait
- [ ] Feature-gated behind `plugin-browser`

---

## Unit 3: F5 Calendar + F7 Docker (Week 7-9)

### F5: Calendar Integration

**New Crate**: `crates/clawft-plugin-calendar/`
**Feature Flag**: `plugin-calendar`
**Transport**: REST + OAuth2 (depends on F6)

**Operations**:
| Tool Name | Description |
|-----------|-------------|
| `cal_list_events` | List upcoming events |
| `cal_create_event` | Create a new calendar event |
| `cal_update_event` | Modify an existing event |
| `cal_delete_event` | Remove a calendar event |
| `cal_check_availability` | Check free/busy status |

**Provider Support**:
- Google Calendar (via Google Calendar API v3, uses F6 OAuth2)
- Microsoft Outlook (via Microsoft Graph API, uses F6 OAuth2)
- iCal (via `.ics` file parsing, local only)

**Acceptance Criteria**:
- [ ] List, create, update, delete calendar events
- [ ] At least Google Calendar operational
- [ ] Uses F6 OAuth2 for authentication
- [ ] Implements `Tool` plugin trait
- [ ] Feature-gated behind `plugin-calendar`

### F7: Docker/Podman Orchestration

**New Crate**: `crates/clawft-plugin-containers/`
**Feature Flag**: `plugin-containers`
**Transport**: subprocess (`docker` / `podman` CLI)

**Operations**:
| Tool Name | Description |
|-----------|-------------|
| `container_build` | Build an image |
| `container_run` | Run a container |
| `container_stop` | Stop a running container |
| `container_logs` | Get container logs |
| `container_list` | List running containers |
| `container_exec` | Execute command in container |

**Resource Limits** (from orchestrator):
- Default: 3 Docker operations globally (concurrent limit)
- Per-tool concurrency limits in plugin manifest

**Security**: Command arguments must be validated. Container names, image names, and environment variables must be sanitized. Use builder pattern for command construction.

**Acceptance Criteria**:
- [ ] Build, run, stop, logs, list, exec all functional
- [ ] Works with both `docker` and `podman` runtimes
- [ ] Global concurrency limit (default 3)
- [ ] Command injection prevention
- [ ] Implements `Tool` plugin trait
- [ ] Feature-gated behind `plugin-containers`

---

## Unit 4: F8 MCP IDE + F9b Full MCP Client (Week 7-10)

### F8: MCP Deep IDE Integration

**Location**: `crates/clawft-services/src/mcp/ide.rs` (new file)
**NOT a separate crate** -- extends MCP infrastructure in `clawft-services`

**Purpose**: VS Code extension backend via MCP. The agent edits code live in the IDE through MCP tool calls.

**Current MCP Server** (`crates/clawft-services/src/mcp/server.rs`):

The existing `McpServerShell` handles `initialize`, `tools/list`, `tools/call`. F8 extends this with IDE-specific tools:

| Tool Name | Description |
|-----------|-------------|
| `ide_open_file` | Open a file in the editor |
| `ide_edit` | Apply a text edit to an open document |
| `ide_diagnostics` | Get current diagnostics (errors, warnings) |
| `ide_symbols` | Search workspace symbols |
| `ide_hover` | Get hover information for a position |

**Architecture**: F8 creates a `ToolProvider` implementation (same pattern as `EchoProvider` in server tests) that provides IDE tools. These tools are registered with `CompositeToolProvider` and automatically exposed via the MCP server.

**Dependencies**: C6 (MCP server for loaded skills) should be merged first.

**Acceptance Criteria**:
- [ ] VS Code extension can connect to clawft MCP server
- [ ] IDE tools appear in `tools/list` response
- [ ] File open, edit, diagnostics functional
- [ ] Implements `ToolProvider` for compositing into `McpServerShell`

### F9b: Full MCP Client Features

**Location**: Extends `crates/clawft-services/src/mcp/client.rs` (from F9a) and adds `crates/clawft-services/src/mcp/discovery.rs`

**Scope** (post-MVP, builds on F9a):
- **Auto-discovery**: Find MCP servers from config/environment/`.clawft/mcp_servers/`
- **Connection pooling**: Reuse connections across tool calls
- **Schema caching**: Avoid re-listing tools on every call; cache with TTL
- **Health checks**: Detect and reconnect failed servers
- **Namespace management**: Handle 1000+ external tools without collision

**Dependencies**: F9a must be stable before F9b work begins.

**Per-Agent MCP Configuration** (from cross-element integration spec Section 3.2):

```toml
# In ~/.clawft/agents/coding-agent/config.toml
[mcp_servers.github]
url = "stdio://gh-mcp-server"
enabled = true

[mcp_servers.slack]
enabled = false  # Explicitly exclude global slack server for this agent
```

Agent-level entries with the same server name replace global entries. New names are appended. `enabled = false` explicitly excludes a global server.

**Acceptance Criteria**:
- [ ] Auto-discovery of MCP servers from config and environment
- [ ] Connection pooling (reuse connections across calls)
- [ ] Schema caching with configurable TTL
- [ ] Health checks detect and reconnect failed servers
- [ ] Per-agent MCP server configuration works
- [ ] Handles 1000+ tools via namespace management

**Test Requirements**:
- Unit test: auto-discovery from config file
- Unit test: connection pool reuse
- Unit test: schema cache hit/miss behavior
- Unit test: health check reconnection
- Integration test: "Plugin -> Hot-reload -> MCP" (Week 8, P0 from cross-element spec)

---

## Security Checklist (All Units)

Mandatory exit gates from the orchestrator:

- [x] Browser tool blocks `file://`, `data://`, and `javascript://` URL schemes
- [x] Browser tool clears state (cookies, storage, sessions) between sessions
- [x] MCP stdio child processes do not inherit secret environment variables (minimal env constructed explicitly)
- [x] External MCP server tools are tagged as "untrusted" in the tool registry
- [x] All tool plugins validate and sanitize command arguments (no command injection)
- [x] OAuth2 `state` parameter for CSRF protection
- [x] All existing tests pass (regression gate)

---

## Merge Coordination

**File Ownership** (from cross-element integration spec Section 5.5):

Within `crates/clawft-services/src/mcp/`:
- F8 owns `ide.rs` (new)
- F9 owns `client.rs` (new) and `discovery.rs` (new)
- Shared files (`transport.rs`, `types.rs`, `middleware.rs`) require cross-stream PR review

**Conflict Zones**:
- Plugin crates are independent -- no cross-stream conflicts
- `clawft-services/src/mcp/` is shared with streams 5C (C6 server skills) and 5I (M4/M5 delegation)
- Changes to `transport.rs` or `types.rs` require cross-stream PR review

**Merge Order**: F6 first (unblocks E5a), F1+F2+F9a next (core tools + MVP MCP client), then F3+F4 (advanced), then F5+F7 (calendar/docker), then F8+F9b (MCP ecosystem).

---

## Timeline Summary

| Week | Items | Priority |
|------|-------|----------|
| 5-6 | F6 (OAuth2), F9a (MCP client core) | P0 (unblocks E5a and M4) |
| 5-7 | F1 (Git), F2 (Cargo) | P1 (core dev tools) |
| 6-8 | F3 (Tree-sitter), F4 (Browser) | P2 |
| 7-9 | F5 (Calendar), F7 (Docker) | P2 |
| 8-10 | F8 (MCP IDE), F9b (Full MCP client) | P2 (post-MVP) |
