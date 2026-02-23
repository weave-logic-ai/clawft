# Testing MCP and Delegation Integration

This guide walks through verifying the two primary MCP server integrations
(Claude Code and claude-flow v3) and the delegation engine that routes tasks
between local execution and Claude AI.

---

## 1. Prerequisites

| Dependency | How to verify | Notes |
|---|---|---|
| Rust toolchain (stable nightly) | `rustc --version` | The project uses nightly features (`let_chains`) |
| Cargo | `cargo --version` | |
| Claude Code CLI | `claude --version` | Provides `claude mcp-server` for stdio transport |
| Node.js + npx | `node --version && npx --version` | Required for claude-flow v3 |
| `ANTHROPIC_API_KEY` env var | `echo $ANTHROPIC_API_KEY` | Required for Claude delegation. Without it delegation gracefully degrades. |
| clawft built with features | `cargo build -p clawft-cli --features delegate,services` | Both features must be compiled in (they are in the default feature set). |

Build the project:

```bash
cd /path/to/clawft
cargo build -p clawft-cli
```

Verify the binary includes delegation:

```bash
./target/debug/weft status
```

The status output should list `delegate_task` among the registered tools when
`ANTHROPIC_API_KEY` is set.

---

## 2. Configuration Setup

Create (or update) your config file at `~/.clawft/config.json`. The example
below shows both MCP servers configured as `internal_only: true`, plus
delegation enabled.

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-opus-4-5",
      "maxTokens": 8192,
      "temperature": 0.7,
      "maxToolIterations": 20
    }
  },
  "providers": {
    "anthropic": {
      "apiKey": ""
    }
  },
  "tools": {
    "mcpServers": {
      "claude-code": {
        "command": "claude",
        "args": ["mcp", "serve"],
        "env": { "CLAUDECODE": "" },
        "internalOnly": true
      },
      "claude-flow": {
        "command": "npx",
        "args": ["-y", "@claude-flow/cli@latest", "mcp", "start"],
        "env": {},
        "internalOnly": true
      }
    }
  },
  "delegation": {
    "claude_enabled": true,
    "claudeModel": "claude-sonnet-4-20250514",
    "maxTurns": 10,
    "maxTokens": 4096,
    "rules": [
      {
        "pattern": "(?i)deploy|orchestrate|swarm",
        "target": "flow"
      },
      {
        "pattern": "(?i)research|analyze|architect|refactor",
        "target": "claude"
      },
      {
        "pattern": "(?i)^list\\b|^echo\\b|^ping\\b",
        "target": "local"
      }
    ],
    "excludedTools": ["exec_shell"]
  }
}
```

### Key configuration points

- **`internalOnly: true`** (the default for `MCPServerConfig`). MCP sessions
  are created and the initialize handshake runs, but tools are NOT registered
  in the `ToolRegistry`. This means the LLM cannot directly invoke these tools
  unless a skill explicitly exposes them via `allowed-tools` patterns.

- **`providers.anthropic.apiKey`** can be left empty if you set the
  `ANTHROPIC_API_KEY` environment variable. The delegation registration code
  checks the env var first, then falls back to the config value.

- **`delegation.claude_enabled: true`** enables the delegation engine. Setting
  it to `false` causes all tasks to fall back to `Local` regardless of rules.

---

## 3. Testing Claude Code Delegation

Claude Code provides an MCP server via `claude mcp-server` over stdio
transport. When configured as `internal_only`, a session is established but
its tools are hidden from the main LLM loop until a skill exposes them.

### Step-by-step

#### a. Start weft with config

```bash
ANTHROPIC_API_KEY="sk-ant-..." \
  RUST_LOG=info,clawft_cli=debug \
  ./target/debug/weft agent --config ~/.clawft/config.json
```

#### b. Verify MCP session connects

Look for the following log line in the startup output:

```
INFO clawft_cli::mcp_tools: MCP server connected as internal-only (tools not registered) server="claude-code"
```

This confirms:
1. The `claude mcp-server` process was spawned successfully.
2. The MCP initialize handshake completed (protocol version, capabilities
   exchanged).
3. Because `internalOnly: true`, tools were NOT registered in the
   `ToolRegistry`.

If you see this instead, the spawn failed:

```
WARN clawft_cli::mcp_tools: failed to spawn MCP stdio transport server="claude-code" error="..."
```

Common causes: `claude` binary not on `PATH`, or the binary exited before
completing the handshake.

#### c. Activate a skill that exposes Claude Code tools

Create a skill definition at `~/.clawft/workspace/skills/claude-assist/SKILL.md`:

```markdown
---
name: claude-assist
description: Delegate coding tasks to Claude Code via MCP
version: 1.0.0
allowed-tools:
  - claude-code__*
  - delegate_task
user-invocable: true
argument-hint: "<task description>"
---

# Claude Code Assistance

You have access to Claude Code tools for file operations, code analysis,
and project management. Use the available tools to accomplish the user's
request.
```

The `allowed-tools` glob pattern `claude-code__*` matches all tools from the
`claude-code` MCP server (tools are namespaced as `{server}__{tool_name}`).

Activate the skill:

```bash
# In the weft agent interactive prompt:
/skill claude-assist
```

#### d. Send a task that triggers delegation

Type a complex task in the agent session:

```
Analyze the authentication module and suggest security improvements.
```

#### e. Verify the delegation engine routes to Claude

With `RUST_LOG=debug`, look for:

```
DEBUG clawft_services::delegation: delegation rule matched task="Analyze the authentication..." matched_target=Claude resolved_target=Claude
```

Or if no rule matched but complexity is high enough:

```
DEBUG clawft_services::delegation: auto delegation decision task="..." complexity=0.55 target=Claude
```

#### f. Check tool execution via MCP

When the Claude sub-agent invokes tools, you will see:

```
DEBUG clawft_services::delegation::claude: executing delegated tool call tool="read_file" id="call_..."
DEBUG clawft_core::tools::registry: executing tool tool="read_file"
```

The response flows back through the delegation loop until `stop_reason`
is `end_turn`, at which point the final text is returned.

---

## 4. Testing claude-flow v3 Delegation

claude-flow v3 provides swarm coordination, memory management, and task
orchestration via MCP.

### Step-by-step

#### a. Start the claude-flow daemon (optional)

Some claude-flow tools require a running daemon for background coordination:

```bash
npx @claude-flow/cli@latest daemon start
```

Verify it is running:

```bash
npx @claude-flow/cli@latest doctor --fix
```

#### b. Configure as MCP server

The config from section 2 already includes the claude-flow entry. Verify:

```json
"claude-flow": {
  "command": "npx",
  "args": ["-y", "@claude-flow/cli@latest", "mcp", "start"],
  "env": {},
  "internalOnly": true
}
```

#### c. Verify session connects

Start weft and look for:

```
INFO clawft_cli::mcp_tools: MCP server connected as internal-only (tools not registered) server="claude-flow"
```

If the handshake fails, you will see:

```
WARN clawft_cli::mcp_tools: MCP initialize handshake failed server="claude-flow" error="..."
```

This typically means `npx` could not resolve the package or the MCP server
did not respond with a valid `initialize` result within the timeout.

#### d. Activate a claude-flow skill

Create `~/.clawft/workspace/skills/flow-swarm/SKILL.md`:

```markdown
---
name: flow-swarm
description: Swarm coordination and memory via claude-flow v3
version: 1.0.0
allowed-tools:
  - claude-flow__*
  - delegate_task
user-invocable: true
argument-hint: "<coordination task>"
---

# Claude Flow Swarm Coordination

You have access to claude-flow v3 tools for:
- Swarm initialization and coordination
- Memory storage and search
- Task creation and lifecycle
- Agent spawning and management

Use these tools to orchestrate multi-agent workflows.
```

Activate:

```bash
/skill flow-swarm
```

#### e. Test swarm coordination, memory, and task management

Try these operations in the agent session:

**Memory store and search:**
```
Store a memory about our authentication pattern: "JWT with refresh tokens, 15-minute access token TTL"
```

Expected tool calls: `claude-flow__memory_store`, followed potentially by
`claude-flow__memory_search` to confirm.

**Task management:**
```
Create a task to review the security audit findings, then list all open tasks.
```

Expected tool calls: `claude-flow__task_create`, `claude-flow__task_list`.

**Swarm coordination:**
```
Initialize a hierarchical swarm with 4 agents for a code review.
```

Expected tool calls: `claude-flow__swarm_init`, `claude-flow__agent_spawn`.

#### f. Verify tool discovery and execution

With debug logging, confirm tool calls flow through the MCP session:

```
DEBUG clawft_cli::mcp_tools: executing tool via MCP session tool="memory_store" server="claude-flow"
```

The `McpToolWrapper` sends a `tools/call` JSON-RPC request to the MCP server
and extracts text content from the response's `content` array.

---

## 5. Testing Tool Discovery and internal_only Behavior

This section verifies the core `internal_only` contract: sessions are created
for all MCP servers, but only non-internal servers have their tools registered.

### 5.1 Verify internal_only servers have sessions but no registered tools

Start weft with the config from section 2. Then check tool listings:

```bash
# In a separate terminal, or via the agent prompt:
weft status
```

The tools list should show built-in tools (`read_file`, `write_file`,
`edit_file`, `list_dir`, `web_search`, `exec_shell`, `delegate_task`, etc.)
but NOT any `claude-code__*` or `claude-flow__*` tools.

To verify sessions exist, check the log output at startup. Both servers
should report `connected as internal-only`.

### 5.2 Verify skills properly scope tool access

The `ToolRegistry::schemas_for_tools()` method filters schemas per-turn using
glob matching. When a skill with `allowed-tools: ["claude-flow__memory_*"]`
is active, only matching tools are sent to the LLM.

Test with a restrictive skill:

```markdown
---
name: memory-only
description: Only memory tools
allowed-tools:
  - claude-flow__memory_*
---

You can only use memory tools. Do not attempt other operations.
```

Activate the skill and send a request. In debug logs, confirm:

```
DEBUG clawft_core::agent::loop_core: filtering tools for skill allowed_tools=["claude-flow__memory_*"]
```

The LLM should only receive schemas matching `claude-flow__memory_*`.

### 5.3 Test that the LLM only sees scoped tools when a skill is active

Without any active skill, the LLM sees all registered tools (the full
`ToolRegistry::schemas()` output). When a skill is activated, the agent loop
reads the `allowed_tools` metadata from the inbound message and calls
`schemas_for_tools()` instead.

You can verify this by comparing tool counts in the debug output:

```
# Without skill:
DEBUG: sending 12 tool schemas to LLM

# With restrictive skill:
DEBUG: filtering tools for skill allowed_tools=["claude-flow__memory_*"]
# Only matching schemas are sent
```

### 5.4 Testing with a non-internal MCP server

To contrast, add a non-internal MCP server to the config:

```json
"test-external": {
  "command": "npx",
  "args": ["-y", "some-mcp-server"],
  "internalOnly": false
}
```

On startup, you should see:

```
INFO clawft_cli::mcp_tools: registered MCP tools server="test-external" tools=5
```

These tools appear in the registry as `test-external__<tool_name>` and are
visible to the LLM at all times (no skill activation needed).

---

## 6. Testing Delegation Routing

The `DelegationEngine` routes tasks based on regex rules and complexity
heuristics. The engine is exercised when the LLM invokes the `delegate_task`
tool.

### 6.1 Low-complexity tasks route to Local

Send a simple task:

```
List all files in the workspace.
```

Expected behavior with the rules from section 2:

```
DEBUG clawft_services::delegation: delegation rule matched task="List all files..." matched_target=Local resolved_target=Local
```

The `delegate_task` tool returns immediately with:

```json
{
  "status": "local",
  "message": "Task does not require delegation; handle locally.",
  "task": "List all files in the workspace."
}
```

### 6.2 High-complexity tasks route to Claude

Send a complex task:

```
Architect a comprehensive distributed authentication system with security audit and code review.
```

The complexity heuristic should score this above 0.7 (many keywords:
"architect", "comprehensive", "distributed", "security", "audit", "review").
Even without a matching rule, the auto-decision routes to Claude:

```
DEBUG clawft_services::delegation: auto delegation decision task="Architect a comprehensive..." complexity=0.85 target=Claude
```

### 6.3 Flow targets fall back to Claude

The config rule `"(?i)deploy|orchestrate|swarm"` targets `Flow`. Since Flow
delegation was removed in the MCP-first architecture, Flow targets resolve
to Claude when Claude is available:

```
Deploy the application to staging.
```

Expected:

```
DEBUG clawft_services::delegation: delegation rule matched task="Deploy the application..." matched_target=Flow resolved_target=Claude
```

If Claude is also unavailable (no API key), it falls back further to Local:

```
resolved_target=Local
```

### 6.4 Testing with `claude_enabled: false`

Update the config:

```json
"delegation": {
  "claude_enabled": false
}
```

Restart weft. The `register_delegation` function will log:

```
INFO clawft_cli::mcp_tools: delegation disabled in config, skipping
```

The `delegate_task` tool will not be registered. Any task that would have
been delegated will instead be handled by the main agent loop locally.

### 6.5 Testing with no API key

Remove or unset `ANTHROPIC_API_KEY` and leave `providers.anthropic.apiKey`
empty:

```bash
unset ANTHROPIC_API_KEY
```

On startup:

```
INFO clawft_cli::mcp_tools: ANTHROPIC_API_KEY not set and no key in providers config; delegation disabled
```

The `delegate_task` tool is not registered. The engine gracefully degrades.

### 6.6 Complexity scoring reference

The `DelegationEngine::complexity_estimate()` function scores tasks on a
0.0 to 1.0 scale using three factors:

| Factor | Weight | Saturation | Description |
|---|---|---|---|
| Text length | 30% | 500 chars | Longer text = more complex |
| Question marks | 20% | 3 `?` marks | Questions suggest research |
| Keyword hits | 50% | 4 keywords | Complexity keywords present |

**Complexity keywords**: `deploy`, `refactor`, `architect`, `design`,
`optimize`, `migrate`, `security`, `audit`, `review`, `analyze`,
`orchestrate`, `coordinate`, `integrate`, `implement`, `debug`,
`investigate`, `comprehensive`, `distributed`, `concurrent`, `parallel`.

**Routing thresholds**:
- Score < 0.3: routes to `Local`
- Score >= 0.3 with Claude available: routes to `Claude`
- Score >= 0.3 without Claude: falls back to `Local`

---

## 7. Troubleshooting

### MCP handshake failures

**Symptom**: `MCP initialize handshake failed` in logs.

**Causes**:
- The MCP server binary is not installed or not on PATH.
- The server crashed during startup (check stderr output).
- The server sent a malformed `initialize` response.

**Fix**:
```bash
# Test claude-code server manually:
CLAUDECODE= echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | CLAUDECODE= claude mcp serve

# Test claude-flow server manually:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | npx -y @claude-flow/cli@latest mcp start
```

You should receive a JSON-RPC response with `result.protocolVersion` and
`result.capabilities`.

### Missing API keys

**Symptom**: `delegation disabled` in logs, `delegate_task` tool not
appearing in status.

**Fix**: Set the `ANTHROPIC_API_KEY` environment variable:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Or add it to the config:
```json
"providers": {
  "anthropic": {
    "apiKey": "sk-ant-..."
  }
}
```

The env var takes priority over the config value.

### Tool not found during delegation

**Symptom**: `tool execution failed: tool not found: <name>` in logs during
a delegated task.

**Causes**:
- The tool was excluded via `delegation.excludedTools`.
- The tool is from an `internal_only` MCP server and no skill exposed it.
- The Claude sub-agent hallucinated a tool name.

**Fix**:
1. Check `excludedTools` in the delegation config.
2. Verify the tool exists: `weft status` lists all registered tools.
3. If the tool is from an internal MCP server, create a skill with the
   appropriate `allowed-tools` pattern.

### Permission denied on tool execution

**Symptom**: `permission denied for tool '<name>'` errors.

**Causes**:
- The user's permission level is insufficient.
- The tool is not in the user's `tool_access` allowlist.
- The tool is in the user's `tool_denylist`.

**Fix**:
- CLI channel (`weft agent`) always gets admin (level 2) permissions.
- For other channels (Telegram, Slack, etc.), configure per-user or
  per-channel permission overrides in the routing config.
- Check the tiered permission system: level 0 (zero-trust) has no tool
  access by default.

### MCP server not spawning (stdio transport)

**Symptom**: `failed to spawn MCP stdio transport` error.

**Fix**:
```bash
# Verify the command exists:
which claude
which npx

# Verify it can be executed:
CLAUDECODE= claude mcp serve --help
npx -y @claude-flow/cli@latest mcp start --help
```

If using `npx`, ensure the package can be resolved. On first run, `npx -y`
will download the package, which may take a few seconds. The MCP handshake
timeout may need to account for this.

### Delegation loop exceeds max turns

**Symptom**: `max turns (10) exceeded` error.

**Causes**:
- The delegated task is too complex for the configured turn limit.
- The Claude sub-agent is stuck in a tool-use loop.

**Fix**: Increase `delegation.maxTurns` in the config, or simplify the task.
Consider adding specific tools to `excludedTools` if a particular tool is
causing loops.

### Internal-only server tools appearing in LLM context

**Symptom**: Tools from an `internal_only` server show up without an active
skill.

**Fix**: Verify `internalOnly: true` is set in the MCP server config. Note
that `internalOnly` defaults to `true`, so this should only happen if it was
explicitly set to `false`. Check for typos in the config key name (both
`internal_only` and `internalOnly` are accepted).

---

## Appendix: Architecture Quick Reference

### Tool registration flow

```text
Config.tools.mcp_servers
  |
  v
register_mcp_tools()
  |
  +-- For each server:
  |     |
  |     +-- create_mcp_client() -> McpSession
  |     |     |
  |     |     +-- StdioTransport::new() or HttpTransport::new()
  |     |     +-- McpSession::connect() (initialize handshake)
  |     |
  |     +-- if internal_only: true
  |     |     +-- Log "connected as internal-only"
  |     |     +-- Session stored, tools NOT registered
  |     |
  |     +-- if internal_only: false
  |           +-- session.list_tools()
  |           +-- For each tool: McpToolWrapper -> registry.register()
  |           +-- Log "registered MCP tools"
  |
  v
register_delegation()
  |
  +-- Check claude_enabled
  +-- Resolve API key (env var > config)
  +-- Create ClaudeDelegator
  +-- Create DelegationEngine
  +-- Snapshot registry (to prevent recursive delegation)
  +-- Register DelegateTaskTool
```

### Delegation decision flow

```text
delegate_task invoked with task text
  |
  v
DelegationEngine::decide(task, claude_available=true)
  |
  +-- Walk compiled rules (first regex match wins)
  |     |
  |     +-- Rule matched? -> resolve_availability(target)
  |           +-- Claude target + Claude unavailable -> Local
  |           +-- Flow target -> Claude (if available) else Local
  |           +-- Local target -> Local
  |
  +-- No rule matched? -> auto_decide(task)
        |
        +-- complexity_estimate(task) -> 0.0..1.0
        +-- score < 0.3 -> Local
        +-- score >= 0.3 + Claude available + claude_enabled -> Claude
        +-- Otherwise -> Local
```

### Skill-based tool scoping flow

```text
InboundMessage arrives
  |
  v
process_message()
  |
  +-- Check msg.metadata["allowed_tools"]
  |     |
  |     +-- Present (skill active):
  |     |     tools.schemas_for_tools(allowed_patterns)
  |     |     -> Only matching tool schemas sent to LLM
  |     |
  |     +-- Absent (no skill):
  |           tools.schemas()
  |           -> All registered tool schemas sent to LLM
  |
  v
ChatRequest.tools = filtered schemas
  |
  v
Pipeline execution + tool loop
```
