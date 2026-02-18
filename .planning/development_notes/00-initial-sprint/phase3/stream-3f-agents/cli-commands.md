# Stream 3F-C: Slash Commands + Skills/Agents CLI

## Summary

Implemented the interactive slash-command framework, built-in slash commands for
the `weft agent` REPL, and new top-level CLI subcommands for skills and agent
management.

## Files Created

- `crates/clawft-cli/src/interactive/mod.rs` -- Module root for interactive framework
- `crates/clawft-cli/src/interactive/registry.rs` -- `SlashCommandRegistry`, `SlashCommand` trait, `InteractiveContext`
- `crates/clawft-cli/src/interactive/builtins.rs` -- 8 built-in slash commands + `register_builtins()`
- `crates/clawft-cli/src/commands/skills_cmd.rs` -- `weft skills list|show|install` subcommands
- `crates/clawft-cli/src/commands/agents_cmd.rs` -- `weft agents list|show|use` subcommands

## Files Modified

- `crates/clawft-cli/src/main.rs` -- Added `Skills` and `Agents` to `Commands` enum; wired dispatch; added `pub mod interactive`; added CLI parsing tests
- `crates/clawft-cli/src/commands/mod.rs` -- Added `pub mod agents_cmd;` and `pub mod skills_cmd;`

## Design Decisions

### SlashCommandRegistry

- Uses a `HashMap<String, Box<dyn SlashCommand>>` for O(1) dispatch
- The `SlashCommand` trait requires `Send + Sync` for future async support
- Dispatch returns `Option<Result<String>>` -- `None` means "not a slash command,
  pass through to the LLM", `Some(Err(_))` means command error, `Some(Ok(_))`
  is output to display
- The `/quit` command returns a sentinel string (`__QUIT__`) that the REPL loop
  checks to break out

### InteractiveContext

- Lightweight mutable struct passed to every command handler
- Contains: active_agent, active_skill, model, tool_names, skill_names, agent_names
- Commands mutate the context directly (e.g., `/use research` sets `active_skill`)
- Designed for easy extension -- new fields can be added without changing the trait

### Built-in Slash Commands (8 total)

| Command | Description |
|---------|-------------|
| `/help [topic]` | General help or topic-specific (skills, agents, tools) |
| `/skills` | List available skills with active marker |
| `/use <skill>` | Activate/deactivate a skill |
| `/agent <name>` | Switch agent persona |
| `/tools` | List registered tools |
| `/clear` | Clear session context |
| `/status` | Show agent, model, skill, counts |
| `/quit` | Exit interactive session |

### Skills CLI (`weft skills`)

- `list` -- Discovers skills from workspace + user + builtin, displays in a table
  with source annotation (workspace/user/builtin), format (SKILL.md/legacy),
  and truncated description
- `show <name>` -- Full details including variables, allowed tools, metadata,
  and a 500-char instructions preview
- `install <path>` -- Copies a skill directory to `~/.clawft/skills/`

Discovery uses `SkillRegistry::discover()` from `clawft-core::agent::skills_v2`.

### Agents CLI (`weft agents`)

- `list` -- Discovers agents from workspace + user + builtin, displays in a table
  with source annotation, model override, and description
- `show <name>` -- Full details including model, system prompt preview, skills,
  allowed tools, variables
- `use <name>` -- Validates the agent exists and prints guidance on how to
  activate it (via `--model` flag or `/agent` in REPL)

Discovery uses `AgentRegistry::discover()` from `clawft-core::agent::agents`.

### Source Classification

Both skills_cmd and agents_cmd classify items as workspace/user/builtin by
checking whether the item's `source_path` starts with the workspace or user
directory prefix. Items without a `source_path` default to "builtin".

## Test Coverage

### interactive::registry (14 tests)
- Empty registry, register/lookup, dispatch known/unknown/non-slash commands
- Error propagation, duplicate registration, sorted names
- Get by name, default construction, context creation/mutation, input trimming

### interactive::builtins (23 tests)
- Each built-in command tested for normal and edge cases
- `/help` general + 3 topics + unknown topic
- `/skills` with/without skills, active marker
- `/use` activate, deactivate, unknown, no-args
- `/agent` switch, unknown, no-args default/current
- `/tools` with/without tools
- `/clear` resets skill
- `/status` with data, with defaults
- `/quit` sentinel value
- `register_builtins` registers all 8, dispatch through registry works

### commands::skills_cmd (14 tests)
- Truncation (short, long, exact)
- Source classification (workspace, user, builtin)
- Registry with skills, show found/not-found
- Install: source not found, no user dir, success, already exists
- copy_dir_recursive

### commands::agents_cmd (12 tests)
- Truncation, source classification
- Registry with agents, list empty, show found/not-found/all-fields
- Use found/not-found

### main.rs CLI parsing (6 tests)
- skills list, show, install
- agents list, show, use

**Total: 71 new tests, all passing**

## Verification

```
cargo test -p clawft-cli -- interactive skills_cmd agents_cmd cli_skills cli_agents
# 71 passed; 0 failed

cargo clippy -p clawft-cli -- -W warnings
# 0 new warnings (1 pre-existing in mcp_tools.rs)
```
