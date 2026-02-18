# Stream 3F-B: Agent Definitions, Template Renderer, Context Integration

## Summary

Implemented agent definition loading, a template renderer for prompt variable
substitution, and integrated both into the existing ContextBuilder.

## Files Created

- `crates/clawft-core/src/agent/agents.rs` -- AgentDefinition, AgentLoader, AgentRegistry
- `crates/clawft-core/src/agent/helpers.rs` -- render_template function

## Files Modified

- `crates/clawft-core/src/agent/mod.rs` -- Added `pub mod agents;` and `pub mod helpers;`
- `crates/clawft-core/src/agent/context.rs` -- Added `build_system_prompt_for_agent()` and
  `build_messages_for_agent()` methods to ContextBuilder
- `Cargo.toml` (workspace) -- Added `serde_yaml = "0.9"` to workspace dependencies
- `crates/clawft-core/Cargo.toml` -- Added `serde_yaml` dependency

## Design Decisions

### AgentDefinition

- Supports both YAML (`.yaml`, `.yml`) and JSON (`.json`) formats
- `source_path` is `#[serde(skip)]` to avoid serialization of local filesystem paths
- `variables` is a `HashMap<String, String>` for template rendering
- All optional fields default to `None`/empty via `#[serde(default)]`

### AgentLoader

- Synchronous filesystem access (uses `std::fs`) since agent loading is typically
  done at startup, not per-request
- Supports both single-file and directory-per-agent layouts
- Candidate filenames in a directory: `agent.yaml`, `agent.yml`, `agent.json`
- Invalid entries are logged and skipped, not fatal

### AgentRegistry 3-Level Priority

- Built-in agents (lowest) -> user agents -> workspace agents (highest)
- Each level overwrites earlier entries with the same name
- `list()` returns agents sorted by name for deterministic output

### Template Renderer

- Simple, no external template engine dependency
- Three substitution passes: `$ARGUMENTS`, `${N}` positional, `${NAME}` named
- Missing variables silently become empty strings (no errors)
- Malformed `${` without closing `}` is preserved as-is

### ContextBuilder Integration

- `build_system_prompt_for_agent()` prepends the agent's rendered system prompt
  with a `# Agent: <name>` header, followed by the standard system prompt
- `build_messages_for_agent()` assembles the full message list using the agent's
  skills, optional extra skill instructions, memory, and history
- Maintains backward compatibility: existing `build_messages()` is unchanged

## Test Coverage

53 tests total:
- 16 tests in `agents.rs` (loader + registry)
- 16 tests in `helpers.rs` (template renderer)
- 21 tests in `context.rs` (6 new agent integration tests + 15 existing)

## Dependencies Added

- `serde_yaml = "0.9"` -- YAML parsing for agent definition files

## Coordination Notes

- Agent 3Fa-A concurrently created `skills_v2.rs` with `SkillRegistry` -- that module
  was already in `mod.rs` when this work started
- Fixed a clippy `derivable_impls` warning in `clawft-types/src/skill.rs` (from 3Fa-A)
  that blocked compilation
