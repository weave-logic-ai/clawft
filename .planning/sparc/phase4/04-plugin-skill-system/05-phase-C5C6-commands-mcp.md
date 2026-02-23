# Phase C5+C6: Slash-Command Framework & MCP Skill Exposure

> Element 04 -- Plugin & Skill System
> Priority: C5 = P2 (Week 7), C6 = P1 (Week 7-8)
> Dependencies: C3 (SkillLoader), C4 (Hot-Reload watcher)

---

## Overview

C5 wires the existing `SlashCommandRegistry` (currently dead code) into the agent REPL, replacing inline `match` blocks in `agent.rs`. C6 exposes loaded skills as MCP tools through the existing `McpServerShell`, enabling external clients to discover and invoke skill-provided tools via `tools/list` and `tools/call`.

---

## C5: Slash-Command Framework

### Current Code

**`clawft-cli/src/interactive/registry.rs`** (338 lines, fully tested):
- `SlashCommandRegistry` with `register()`, `dispatch()`, `has()`, `names()`, `get()`
- `SlashCommand` trait: `name()`, `description()`, `execute(args, ctx)`
- `InteractiveContext` struct: `active_agent`, `active_skill`, `model`, `tool_names`, `skill_names`, `agent_names`
- 14 unit tests covering dispatch, error propagation, trimming, default creation

**`clawft-cli/src/interactive/builtins.rs`** (564 lines, 20 tests):
- `register_builtins()` registers 8 commands: `/help`, `/skills`, `/use`, `/agent`, `/clear`, `/status`, `/quit`, `/tools`
- `QUIT_SENTINEL` constant for exit detection
- Each command is a struct implementing `SlashCommand`

**`clawft-cli/src/commands/agent.rs`** (444 lines, `run_interactive` at line 218):
- The REPL loop at lines 238-306 handles commands via inline `match`:
  ```rust
  match input {
      "/exit" | "/quit" => break,
      "/clear" => { println!("[session cleared]"); continue; }
      "/help" => { print_help(); continue; }
      "/tools" => { /* inline tool listing */ continue; }
      _ => {}
  }
  ```
- `print_help()` at line 374 duplicates what `HelpCommand` already provides
- The interactive module's `SlashCommandRegistry` is never instantiated in `run_interactive`

### Implementation Tasks

#### C5-T1: Wire `SlashCommandRegistry` into `run_interactive`

**File**: `clawft-cli/src/commands/agent.rs`

1. Import `SlashCommandRegistry`, `InteractiveContext`, `register_builtins`, `QUIT_SENTINEL` from `crate::interactive`
2. Before the REPL loop (after line 237), construct and populate:
   ```rust
   let mut registry = SlashCommandRegistry::new();
   register_builtins(&mut registry);
   let mut ctx = InteractiveContext::new(model.to_string());
   ctx.tool_names = tool_names.to_vec();
   // Populate ctx.skill_names and ctx.agent_names from AppContext
   ```
3. Replace the inline `match` block (lines 254-277) with:
   ```rust
   if input.starts_with('/') {
       match registry.dispatch(input, &mut ctx) {
           Some(Ok(output)) => {
               if output == QUIT_SENTINEL { break; }
               println!("{output}");
               println!();
               continue;
           }
           Some(Err(e)) => {
               eprintln!("error: {e}");
               continue;
           }
           None => {
               eprintln!("Unknown command: {input}");
               continue;
           }
       }
   }
   ```
4. Remove the `print_help()` function (line 374-381) -- now handled by `HelpCommand`

**Risk**: The `/exit` command is currently handled but not registered as a builtin. Either add `ExitCommand` to builtins or make `/quit` the canonical exit (since `QuitCommand` already exists). Recommendation: Add `/exit` as an alias -- register a second `QuitCommand`-like struct named `exit`.

#### C5-T2: Add `register_skill_commands()`

**File**: `clawft-cli/src/interactive/mod.rs` or new `clawft-cli/src/interactive/skill_commands.rs`

1. Accept `&SkillRegistry` (or equivalent skill list) and `&mut SlashCommandRegistry`
2. For each skill with `user_invocable: true`:
   - Create a `SkillSlashCommand` struct implementing `SlashCommand`
   - `name()` returns the skill name (e.g., `"research"`)
   - `description()` returns the skill description
   - `execute()` activates the skill in `InteractiveContext` and returns confirmation
3. Register each into the `SlashCommandRegistry`

#### C5-T3: Command name collision detection

**File**: `clawft-cli/src/interactive/registry.rs`

Currently `register()` silently replaces existing commands. Add collision detection:

1. Add a `register_checked()` method:
   ```rust
   pub fn register_checked(&mut self, cmd: Box<dyn SlashCommand>) -> Result<(), String> {
       let name = cmd.name().to_string();
       if self.commands.contains_key(&name) {
           return Err(format!("command collision: /{name} already registered"));
       }
       self.commands.insert(name, cmd);
       Ok(())
   }
   ```
2. `register_skill_commands()` uses `register_checked()` and logs errors for collisions
3. `register_builtins()` continues to use `register()` (builtins always win)

#### C5-T4: Update `/help` to include skill-contributed commands

**File**: `clawft-cli/src/interactive/builtins.rs`

The current `format_general_help()` is hardcoded. Update `HelpCommand::execute()`:

1. Pass the full registry to help (either via `InteractiveContext` or by making `/help` aware of the registry)
2. Option A: Add `pub command_descriptions: Vec<(String, String)>` to `InteractiveContext`
3. Option B: Have `HelpCommand` accept a reference to the registry at construction
4. List all registered commands with their descriptions, grouped as "Built-in" and "Skill"

#### C5-T5: Remove I6c TODO(C5) markers

Search for `TODO(C5)` and `I6c` markers across the codebase and remove them, as this phase completes that work.

### C5 Acceptance Criteria

- [ ] Agent REPL commands routed through `SlashCommandRegistry::dispatch()`, not inline `match`
- [ ] `print_help()` function removed from `agent.rs`
- [ ] Skills with `user_invocable: true` register as `/skill_name` commands
- [ ] Command name collisions between skills and builtins produce a logged error
- [ ] `/help` output includes skill-contributed commands
- [ ] `/exit` works as an alias for `/quit`
- [ ] All existing interactive tests pass (14 registry + 20 builtins)
- [ ] No regressions in single-message mode (`--message` flag)

---

## C6: MCP Skill Exposure

### Current Code

**`clawft-services/src/mcp/server.rs`** (729 lines, 12 tests):
- `McpServerShell` wraps a `CompositeToolProvider` + middleware chain
- `PROTOCOL_VERSION = "2025-06-18"`, `SERVER_NAME = "clawft"`
- `tools/list` calls `self.provider.list_tools_all()`, applies middleware `filter_tools`, serializes
- `tools/call` resolves `namespace__tool` via `self.provider.call_tool()`, applies middleware `before_call`/`after_call`
- `capabilities` response already includes `"tools": { "listChanged": true }` -- notification infrastructure exists

**`clawft-services/src/mcp/composite.rs`** (297 lines, 11 tests):
- `CompositeToolProvider` aggregates `Vec<Box<dyn ToolProvider>>`
- Tools are listed with `{namespace}__{tool_name}` prefix
- Call routing splits on first `__` separator to find provider by namespace

**`clawft-services/src/mcp/provider.rs`** (400 lines, 14 tests):
- `ToolProvider` trait: `namespace()`, `list_tools()`, `call_tool(name, args)`
- `BuiltinToolProvider`: wraps tool definitions + dispatcher closure
- `ToolDefinition`: `name`, `description`, `input_schema` (JSON Schema)
- `CallToolResult`: `content: Vec<ContentBlock>`, `is_error: bool`
- `ToolError`: `NotFound`, `ExecutionFailed`, `PermissionDenied`

**`clawft-services/src/mcp/middleware.rs`** (808 lines, 22 tests):
- `Middleware` trait: `filter_tools()`, `before_call()`, `after_call()`
- `PermissionFilter`: restricts tools by allowlist -- this will enforce `allowed_tools` from skills

### Implementation Tasks

#### C6-T1: Implement `SkillToolProvider`

**File**: New `clawft-services/src/mcp/skill_provider.rs`

Create a `ToolProvider` implementation that bridges skills to MCP tools:

```rust
pub struct SkillToolProvider {
    /// Reference to the skill registry (or a snapshot of loaded skills)
    skills: Arc<RwLock<Vec<SkillToolDefinition>>>,
}

struct SkillToolDefinition {
    skill_name: String,
    tool_name: String,
    description: String,
    input_schema: Value,
    // Reference to execute through
}

impl ToolProvider for SkillToolProvider {
    fn namespace(&self) -> &str { "skill" }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        // Convert each skill's tools into ToolDefinitions
        // Use skill's parameters_schema() for input_schema
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        // Route to skill.execute_tool(name, args)
    }
}
```

Key consideration: The `ToolProvider` trait's `list_tools()` is synchronous (`fn`, not `async fn`). The provider must hold a pre-computed snapshot of skill tools, updated when skills reload.

#### C6-T2: Register `SkillToolProvider` with `CompositeToolProvider`

**File**: Wherever `McpServerShell` is constructed (likely in the gateway or agent bootstrap)

1. Create `SkillToolProvider` from the `SkillRegistry`
2. Register it with the `CompositeToolProvider` before constructing `McpServerShell`
3. Skill tools will appear in `tools/list` as `skill__{tool_name}`

#### C6-T3: Hot-reload updates MCP tool listing

**File**: `clawft-services/src/mcp/skill_provider.rs` + `server.rs`

When skills are reloaded (C4 watcher fires):

1. `SkillToolProvider` subscribes to skill registry change events (via `tokio::sync::watch` or the existing bus)
2. On change, update the internal `Vec<SkillToolDefinition>` snapshot
3. Emit `notifications/tools/list_changed` notification to connected clients:
   ```rust
   // The capability "listChanged": true is already advertised
   // Send: {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
   ```
4. Implementation detail: `McpServerShell::run()` currently only reads from the client. To push notifications, refactor to use `tokio::select!` between client input and an internal notification channel.

**Concurrency concern**: `list_tools()` is called from the server loop which holds no locks. The `RwLock<Vec<SkillToolDefinition>>` in `SkillToolProvider` must be read-locked only briefly during `list_tools()`.

#### C6-T4: Enforce `allowed_tools` restrictions

**File**: `clawft-services/src/mcp/middleware.rs`

Skills may declare `allowed_tools` to restrict which sub-tools they can call. Implement this as a middleware or within `SkillToolProvider`:

1. When a skill-provided tool calls sub-tools, check against the skill's `allowed_tools` list
2. Use the existing `PermissionFilter` middleware, dynamically configured per-skill
3. Alternative: Check in `SkillToolProvider::call_tool()` before delegating

### Concurrency Plan

- `SkillToolProvider` holds an `Arc<RwLock<Vec<SkillToolDefinition>>>` for thread-safe reads
- Skill registry changes trigger a write-lock to update the snapshot
- `list_tools()` takes a read-lock, clones the Vec, drops the lock
- `call_tool()` takes a read-lock to find the skill reference, drops the lock, then executes
- MCP notification push requires a `tokio::mpsc::Sender` injected into the server shell

### C6 Acceptance Criteria

- [ ] MCP `tools/list` includes loaded skill tools with JSON Schema `input_schema`
- [ ] MCP `tools/call` for skill tools routes through `skill.execute_tool()`
- [ ] Skill tools appear with `skill__` namespace prefix
- [ ] Hot-reload updates MCP tool listing without server restart
- [ ] `notifications/tools/list_changed` sent to clients on skill reload
- [ ] Skills with `allowed_tools` correctly restrict sub-tool access
- [ ] All existing MCP server tests pass (12 tests)
- [ ] All existing middleware tests pass (22 tests)
- [ ] All existing composite provider tests pass (11 tests)

---

## Dependencies

| Phase | Depends On | Reason |
|-------|-----------|--------|
| C5 | C3 (SkillLoader) | Need loaded skills to register as commands |
| C5 | C1 (Skill trait) | Need `user_invocable` flag on skills |
| C6 | C3 (SkillLoader) | Need skill definitions for tool schemas |
| C6 | C4 (Hot-Reload) | Need change notifications for tool list updates |
| C6 | C1 (Skill trait) | Need `parameters_schema()` and `execute_tool()` methods |

---

## Tests Required

### C5 Tests

| Test | File | Description |
|------|------|-------------|
| `repl_dispatches_via_registry` | `agent.rs` (integration) | Verify slash commands route through registry |
| `skill_commands_registered` | `skill_commands.rs` | Skills with `user_invocable` register correctly |
| `skill_command_collision_logged` | `skill_commands.rs` | Duplicate name logs error, doesn't overwrite builtin |
| `exit_alias_works` | `builtins.rs` | `/exit` triggers `QUIT_SENTINEL` |
| `help_includes_skill_commands` | `builtins.rs` | `/help` lists skill-contributed commands |
| `non_invocable_skills_skipped` | `skill_commands.rs` | Skills without `user_invocable` don't become commands |

### C6 Tests

| Test | File | Description |
|------|------|-------------|
| `skill_provider_lists_tools` | `skill_provider.rs` | `list_tools()` returns skill tool definitions |
| `skill_provider_call_routes` | `skill_provider.rs` | `call_tool()` executes via skill |
| `skill_tools_in_mcp_list` | `server.rs` (integration) | MCP `tools/list` includes `skill__*` tools |
| `skill_tools_call_via_mcp` | `server.rs` (integration) | MCP `tools/call` for `skill__*` works |
| `hot_reload_updates_tools` | `skill_provider.rs` | Tool list changes after skill reload |
| `allowed_tools_enforced` | `skill_provider.rs` or `middleware.rs` | Restricted skills can't call unauthorized tools |
| `notification_sent_on_reload` | `server.rs` | `notifications/tools/list_changed` emitted |

---

## Risk Notes

1. **Synchronous `list_tools()`**: The `ToolProvider::list_tools()` method is synchronous. If skill loading becomes async-only, the snapshot pattern (pre-computed Vec behind RwLock) is essential.

2. **MCP push notifications**: The current `McpServerShell::run()` loop only reads client input. Adding server-initiated notifications requires refactoring to a bidirectional event loop with `tokio::select!`. This is a moderate refactor of `server.rs`.

3. **`InteractiveContext` growth**: C5 adds skill command info to `InteractiveContext`. Keep this struct lean -- consider a reference or handle rather than copying all skill metadata.

4. **Namespace collision**: Skill tools use `skill__` prefix. If a builtin tool happens to use the same namespace, there will be a routing conflict. The `CompositeToolProvider` routes to the first matching namespace, so registration order matters.

5. **Thread safety of REPL**: The REPL is single-threaded (reads stdin line-by-line). `SlashCommandRegistry` doesn't need `Arc`/`Mutex` since it lives on the REPL task. But `InteractiveContext` must be kept in sync if skills are hot-reloaded from another task.
