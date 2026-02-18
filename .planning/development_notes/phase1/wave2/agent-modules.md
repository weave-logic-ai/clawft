# Agent Modules -- Development Notes

**Date**: 2026-02-16
**Wave**: Phase 1, Wave 2
**Agent**: core-agent
**Files**: `crates/clawft-core/src/agent/{memory,skills,context,loop_core}.rs`

## Modules Implemented

### 1. `memory.rs` -- MemoryStore (452 lines, 15 tests)

Long-term and session memory management. Manages two markdown files:
- `MEMORY.md` -- long-term facts (append-only, double-newline separated paragraphs)
- `HISTORY.md` -- session summaries (grep-searchable log)

**Key design decisions**:
- Directory resolution uses sync `Path::exists()` in constructor since `home_dir()` is sync. Falls back from `.clawft` to `.nanobot` only if the legacy dir already exists.
- `read_*` methods return empty string (not error) when files do not exist, matching Python behavior.
- `append_*` methods trim trailing whitespace then add exactly one `\n\n` separator.
- `search()` splits by double-newline into paragraphs, does case-insensitive substring match.
- All I/O through `Platform::fs()` trait object -- no direct tokio::fs usage in production code.
- `#[cfg(test)] with_paths()` constructor for test isolation.

### 2. `skills.rs` -- SkillsLoader (519 lines, 13 tests)

Skill discovery and lazy loading from directory structure.

**Key design decisions**:
- Same `.clawft`/`.nanobot` fallback chain as memory.
- Cache uses `Arc<RwLock<HashMap<String, Skill>>>` for concurrent access.
- `list_skills()` scans for subdirectories containing `skill.json` (does not recurse).
- `load_skill()` reads `skill.json` metadata + optional `prompt.md`, caches result.
- `load_all()` is fault-tolerant: logs warnings for invalid skills, does not abort.
- `Skill.prompt` is `#[serde(skip)]` since it is loaded from a separate file.
- `Skill.version` defaults to `"1.0.0"` via serde default.

### 3. `context.rs` -- ContextBuilder (616 lines, 12 tests)

LLM context assembly from config, memory, skills, and session history.

**Key design decisions**:
- Defines a local `LlmMessage` struct (compatible with `pipeline::traits::LlmMessage`). This should be replaced with a re-export once the pipeline module is written.
- Message assembly order: system prompt -> skill prompts -> memory -> history.
- Skills are loaded on-demand if not cached (falls through from `get_skill` to `load_skill`).
- History truncation uses `config.defaults.memory_window` via `session.get_history()`.
- Current user message is NOT added -- caller responsibility (as specified).
- Bootstrap files (AGENTS.md, SOUL.md, etc.) loaded from expanded workspace path.
- `expand_workspace()` helper handles `~/` prefix expansion.

### 4. `loop_core.rs` -- AgentLoop (195 lines, 4 tests)

Scaffold for the core agent loop.

**Key design decisions**:
- Minimal struct: only `config` and `platform` fields. All other dependencies (bus, pipeline, tools, context, sessions) are commented out as TODOs.
- Documented the full processing flow (consume -> session -> context -> pipeline -> tools -> respond) in doc comments.
- Future method signatures (`run()`, `process_message()`, `tool_loop()`) are documented in comments for the integration phase.
- Tests verify construction, config access, and `Send` bound.

## Integration Notes

1. **LlmMessage duplication**: `context.rs` defines its own `LlmMessage` to avoid depending on the unfinished `pipeline::traits` module. When the pipeline agent completes `traits.rs`, replace the local definition with `use crate::pipeline::traits::LlmMessage`.

2. **AgentLoop wiring**: The `loop_core.rs` scaffold needs these dependencies added:
   - `bus: Arc<MessageBus>` (from `crate::bus`)
   - `tools: ToolRegistry` (from `crate::tools::registry`)
   - `context: ContextBuilder<P>` (from `crate::agent::context`)
   - `sessions: SessionManager<P>` (from `crate::session`)
   - Pipeline registry (from `crate::pipeline`)

3. **Test coverage**: All modules have comprehensive tests using `NativePlatform` + temp directories. Tests are isolated via atomic counters for unique temp paths and cleanup in each test.

4. **No external deps beyond Cargo.toml**: Uses only `clawft-types`, `clawft-platform`, `serde`, `serde_json`, `tokio`, `tracing` -- all declared in `clawft-core/Cargo.toml`.

## Line Counts

| Module | Lines | Tests |
|--------|-------|-------|
| memory.rs | 452 | 15 |
| skills.rs | 519 | 13 |
| context.rs | 616 | 12 |
| loop_core.rs | 195 | 4 |
| **Total** | **1782** | **44** |
