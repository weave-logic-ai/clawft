# Workspaces & Projects: clawft

## 1. Problem Statement

Developers using clawft work across multiple contexts simultaneously: different projects, research tracks, client engagements, and experiments. Currently, all configuration, sessions, memory, and skills live in a single global directory (`~/.clawft/`). This creates several problems:

- **Context pollution**: Memory from a backend project leaks into frontend conversations. Skills for one client are visible to another.
- **Config rigidity**: A global `config.json` means every project uses the same model, provider, temperature, and system prompt. Overriding per-project requires manual editing or environment variables.
- **No isolation**: Session history from project A is searchable during project B. Agent personas cannot be scoped.
- **No reproducibility**: There is no standard way to check workspace configuration into a repository so that team members share the same agent setup.
- **No CLAUDE.md equivalent**: Claude Code popularized project-level instruction files (`CLAUDE.md`) that give the agent persistent, project-specific context. clawft has no equivalent.

## 2. Goals

| ID | Goal | Success Metric |
|----|------|----------------|
| W1 | Self-contained workspace folders | Each workspace has its own `.clawft/` with config, memory, sessions, skills |
| W2 | Config hierarchy with deep merge | Workspace config overrides global; unset fields inherit from global |
| W3 | Project instruction file (`CLAWFT.md`) | Agent reads workspace-level instructions analogous to `CLAUDE.md` |
| W4 | Auto-discovery from cwd | Running `weft` inside a workspace auto-detects the workspace root |
| W5 | CLI commands for workspace lifecycle | `weft workspace create/list/load/config/status/delete` |
| W6 | Hook system for workspace events | `workspace.created`, `workspace.loaded`, `workspace.unloaded` hooks |
| W7 | Skills/agents scoped to workspace | Workspace skills directory takes precedence over global skills |
| W8 | Isolated session and memory stores | Each workspace has its own sessions/ and memory/ directories |

### Non-Goals

- Workspace synchronization across machines (out of scope; use git)
- Multi-user workspace sharing (no access control layer)
- Workspace templates marketplace (future phase)
- Nested workspaces (a workspace cannot contain another workspace)

## 3. Concepts

### 3.1 Workspace

A **workspace** is a directory containing a `.clawft/` subdirectory. The `.clawft/` directory is the marker that identifies a directory tree as a clawft workspace. A workspace is self-contained: it has its own configuration, memory, sessions, and skills.

```
my-project/                         # Workspace root (any directory)
  .clawft/                          # Workspace marker directory
    config.json                     # Workspace-level config (overrides global)
    sessions/                       # Workspace-scoped session files
    memory/                         # Workspace-scoped memory
      MEMORY.md                     # Long-term memory for this workspace
      HISTORY.md                    # Session history for this workspace
    skills/                         # Workspace-scoped skills
    agents/                         # Workspace-scoped agent definitions
    hooks/                          # Workspace-scoped hook scripts
  CLAWFT.md                         # Project instructions (loaded into system prompt)
  src/                              # Project source code (user's)
  ...
```

### 3.2 CLAWFT.md

Analogous to Claude Code's `CLAUDE.md`. A markdown file at the workspace root that provides persistent, project-specific context to the agent. Its contents are prepended to the system prompt when the agent operates within this workspace.

**Conventions** (following CLAUDE.md best practices):
- Keep it concise (Claude Code guidance: "If Claude keeps ignoring a rule, the file is probably too long")
- Focus on what cannot be inferred from code: build commands, coding standards, environment details
- Can import other files using `@path/to/file` syntax
- Supports hierarchical loading: parent directory `CLAWFT.md` + workspace `CLAWFT.md`

### 3.3 Config Hierarchy

Configuration is resolved by deep-merging three layers, with later layers overriding earlier ones:

```
Layer 1 (lowest priority): Built-in defaults
Layer 2: Global config (~/.clawft/config.json)
Layer 3 (highest priority): Workspace config (<workspace>/.clawft/config.json)
```

**Deep merge semantics**:
- Scalar values: workspace overrides global
- Objects/maps: recursively merged (workspace keys override; global keys not in workspace are preserved)
- Arrays: workspace replaces global (no array merging -- too ambiguous)
- Explicit null in workspace removes the key from the merged result

### 3.4 Workspace Discovery

When `weft` is invoked, the workspace is resolved in this order:

1. **Explicit flag**: `--workspace <path>` argument
2. **Environment variable**: `$CLAWFT_WORKSPACE`
3. **Directory walk**: Walk up from `cwd` looking for a `.clawft/` directory (like `.git/` discovery)
4. **Global fallback**: `~/.clawft/` (the global workspace)

If discovery finds a workspace, all subsequent operations (config loading, session storage, memory, skills) are scoped to that workspace.

## 4. Requirements

### 4.1 Functional Requirements

#### FR-W01: Workspace Creation

- `weft workspace create <name>` scaffolds a new workspace at `<cwd>/<name>/`
- Creates `.clawft/` directory with: `config.json` (empty overrides), `sessions/`, `memory/`, `skills/`, `agents/`, `hooks/`
- Creates `CLAWFT.md` with a starter template (project name, date, empty sections)
- Optionally initializes a git repository (`--git` flag)
- Fires `workspace.created` hook after scaffolding
- Accepts `--template <name>` to use a workspace template (future: predefined templates for common project types)

#### FR-W02: Workspace Listing

- `weft workspace list` discovers and lists all known workspaces
- Searches: (a) the global workspace registry at `~/.clawft/workspaces.json`, (b) child directories of cwd containing `.clawft/`
- Displays: name, path, last accessed, session count, config overrides summary
- Accepts `--all` to show workspaces from registry even if directory is missing (marks as "missing")

#### FR-W03: Workspace Loading/Switching

- `weft workspace load <name-or-path>` switches the active workspace for the current shell session
- Sets `$CLAWFT_WORKSPACE` environment variable (for child processes)
- Updates the global workspace registry with last-accessed timestamp
- Fires `workspace.loaded` hook (can run setup scripts, activate venvs, etc.)
- When leaving a workspace, fires `workspace.unloaded` hook on the previous workspace

#### FR-W04: Workspace Configuration

- `weft workspace config` (no args) shows the resolved merged config for the current workspace
- `weft workspace config set <key> <value>` sets a key in the workspace's `.clawft/config.json`
- `weft workspace config get <key>` shows a specific resolved value (indicates source: global/workspace/default)
- `weft workspace config reset <key>` removes a workspace override (falls back to global)
- `weft workspace config edit` opens the workspace config in `$EDITOR`
- Config changes are validated before writing (schema check)

#### FR-W05: Workspace Status

- `weft workspace status` shows the currently active workspace
- Displays: workspace path, config source, session count, memory size, active skills, CLAWFT.md presence
- If no workspace is active (global mode), indicates this clearly

#### FR-W06: Workspace Deletion

- `weft workspace delete <name>` removes a workspace
- Requires `--confirm` flag or interactive confirmation
- Option `--keep-data` removes the workspace from registry but leaves files on disk
- Default behavior: removes `.clawft/` directory and `CLAWFT.md`, leaves all other files
- Fires `workspace.unloaded` hook if the workspace was active

#### FR-W07: CLAWFT.md Loading

- When a workspace is active, the agent reads `CLAWFT.md` from the workspace root
- Contents are inserted into the system prompt (before skills, after the base system prompt)
- Supports `@path/to/file` imports (relative to workspace root)
- Supports hierarchical loading: if parent directories contain `CLAWFT.md`, those are loaded first (like Claude Code's parent directory convention)
- File is re-read at the start of each agent session (not cached across sessions)

#### FR-W08: Workspace-Scoped Sessions

- When a workspace is active, sessions are stored in `<workspace>/.clawft/sessions/`
- `weft sessions list` only shows sessions from the active workspace
- `weft sessions list --global` shows sessions from `~/.clawft/sessions/`
- `weft sessions list --all` shows sessions from both
- Session keys are prefixed with workspace name for disambiguation when listed globally

#### FR-W09: Workspace-Scoped Memory

- When a workspace is active, `MEMORY.md` and `HISTORY.md` are read/written from `<workspace>/.clawft/memory/`
- Memory consolidation operates on workspace-scoped memory
- `weft memory show` shows workspace memory (with `--global` flag for global)

#### FR-W10: Workspace-Scoped Skills

- When a workspace is active, skills are loaded from `<workspace>/.clawft/skills/` first
- If a skill name exists in both workspace and global, workspace version takes precedence
- `weft skills list` shows skills from both sources, indicating provenance

#### FR-W11: Workspace Registry

- `~/.clawft/workspaces.json` maintains a registry of known workspaces
- Schema: `{ "workspaces": [{ "name": "...", "path": "...", "created_at": "...", "last_accessed": "..." }] }`
- Auto-updated when workspaces are created, loaded, or discovered
- Workspace creation registers the workspace; deletion de-registers it

### 4.2 Non-Functional Requirements

#### NFR-W01: Performance

- Workspace discovery (directory walk) completes in < 50ms for directory trees up to 10 levels deep
- Config deep merge completes in < 5ms
- CLAWFT.md loading completes in < 10ms for files up to 50 KB
- Workspace creation completes in < 200ms

#### NFR-W02: Backward Compatibility

- All existing behavior MUST work unchanged when no workspace is detected
- `~/.clawft/` continues to serve as the default global workspace
- Existing `config.json` at `~/.clawft/config.json` remains the global config
- No migration step required for existing users

#### NFR-W03: Platform Compatibility

- Workspace paths work on Linux, macOS, and Windows
- Symlinked workspace directories are supported (resolved to canonical path)
- Workspace paths with spaces are handled correctly

#### NFR-W04: WASM Compatibility

- Workspace discovery and config merge are WASM-compatible (no filesystem walk; explicit path only)
- CLAWFT.md loading works via WASI filesystem

## 5. Hooks Integration

### 5.1 Hook Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `workspace.created` | After `weft workspace create` completes | `{ name, path, template }` |
| `workspace.loaded` | After `weft workspace load` activates a workspace | `{ name, path, previous_workspace }` |
| `workspace.unloaded` | Before switching away from or deleting a workspace | `{ name, path, reason }` |
| `workspace.config_changed` | After `weft workspace config set/reset` | `{ name, path, key, old_value, new_value }` |

### 5.2 Hook Scripts

Hooks can be defined at two levels:

1. **Global hooks**: `~/.clawft/hooks/workspace.created.sh` (or any executable)
2. **Workspace hooks**: `<workspace>/.clawft/hooks/workspace.loaded.sh`

Both are executed (global first, then workspace). Hook scripts receive the event payload as JSON on stdin.

### 5.3 Use Cases for Hooks

- **workspace.created**: Run `git init`, copy template files, install dependencies
- **workspace.loaded**: Activate a Python virtualenv, set environment variables, start local services
- **workspace.unloaded**: Deactivate virtualenv, stop local services, save state
- **workspace.config_changed**: Validate config, notify team (via Slack), log audit trail

## 6. Integration with Existing Systems

### 6.1 Config System (`clawft-types/config.rs`)

The existing `Config` struct and config loading in `clawft-platform` needs to be extended:

- Add workspace-aware config resolution to `load_config()`
- The config loader gains a `workspace_root: Option<PathBuf>` parameter
- Deep merge is implemented as a utility in `clawft-types` (JSON value deep merge)

### 6.2 Skills System (Phase 3F Integration)

- `SkillsLoader` gains workspace awareness: check `<workspace>/.clawft/skills/` before `~/.clawft/skills/`
- Skill resolution order: workspace skills > global skills > built-in skills
- Skill metadata (`skill.json` / `SKILL.md`) is loaded from the workspace-scoped directory

### 6.3 Agent System (Context Builder)

- `ContextBuilder` gains a `clawft_md_content: Option<String>` field
- When building the system prompt, CLAWFT.md content is inserted after the base system prompt and before skill docs
- The agent loop re-reads CLAWFT.md at session start

### 6.4 Session Manager

- `SessionManager` constructor gains `sessions_dir: PathBuf` parameter (already exists as `SessionManager::with_dir`)
- When a workspace is active, the CLI passes the workspace sessions directory

### 6.5 Memory Store

- `MemoryStore` constructor gains `memory_dir: PathBuf` parameter
- When a workspace is active, the CLI passes the workspace memory directory

### 6.6 CLI (`clawft-cli/main.rs`)

- All existing subcommands gain awareness of the active workspace
- A new `Workspace` subcommand group is added to the `Commands` enum
- The global `--workspace <path>` flag is added to `Cli` struct
- Config loading flow: discover workspace -> load global config -> load workspace config -> deep merge

## 7. Data Model

### 7.1 Workspace Registry

```json
{
  "version": 1,
  "workspaces": [
    {
      "name": "my-backend",
      "path": "/home/user/projects/my-backend",
      "created_at": "2026-02-17T10:00:00Z",
      "last_accessed": "2026-02-17T15:30:00Z"
    },
    {
      "name": "research-llm",
      "path": "/home/user/research/llm-eval",
      "created_at": "2026-02-15T09:00:00Z",
      "last_accessed": "2026-02-16T18:00:00Z"
    }
  ]
}
```

### 7.2 Workspace Config (`.clawft/config.json`)

Workspace config uses the exact same schema as the global `config.json`. Only the fields that differ from global need to be specified. Deep merge fills in the rest.

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-20250514",
      "temperature": 0.3,
      "workspace": "."
    }
  },
  "tools": {
    "restrictToWorkspace": true
  }
}
```

### 7.3 CLAWFT.md Template

```markdown
# Project: {{name}}

## Build & Test

- `cargo build` -- build the project
- `cargo test` -- run tests

## Coding Standards

- Follow existing code style
- Write tests for all new code

## Notes

(Add project-specific context here)
```

## 8. Security Considerations

- **Path traversal**: Workspace config loading must not allow `../` to escape the workspace root
- **Hook execution**: Hook scripts are executed with the user's permissions. No elevation.
- **Symlink resolution**: Workspace paths are canonicalized to prevent symlink attacks
- **Config validation**: Workspace config undergoes the same validation as global config before merge
- **No secrets in CLAWFT.md**: CLAWFT.md is intended to be committed to git. Document this clearly. Secrets should go in `.clawft/config.json` (which should be `.gitignore`-d)

## 9. Risk Register

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Config merge ambiguity (arrays, nulls) | Medium | Medium | 4 | Document merge semantics clearly; provide `weft workspace config show --merged` for debugging |
| Directory walk performance in deep trees | Low | Low | 2 | Limit walk depth to 20 levels; cache discovery result per process |
| Hook script security (arbitrary execution) | Medium | Medium | 4 | Document that hooks run as user; add `--no-hooks` flag |
| Workspace state corruption (partial write) | Low | High | 4 | Use atomic file writes for config and registry |
| Platform path issues (Windows backslashes) | Low | Medium | 2 | Use `PathBuf` everywhere; canonicalize on load |
| CLAWFT.md too large (bloats prompt) | Medium | Medium | 4 | Warn if > 10 KB; truncate at 50 KB with warning |

## 10. Success Criteria

- [ ] `weft workspace create my-project` scaffolds a complete workspace
- [ ] `weft workspace list` shows all known workspaces
- [ ] `weft workspace load my-project` activates the workspace
- [ ] `weft workspace status` shows the active workspace details
- [ ] Running `weft agent -m "hello"` inside a workspace uses workspace config
- [ ] CLAWFT.md content appears in agent system prompt
- [ ] Sessions created inside a workspace are stored in workspace sessions/
- [ ] Memory consolidation operates on workspace-scoped memory
- [ ] Skills from workspace take precedence over global skills
- [ ] Workspace hooks fire on create/load/unload events
- [ ] Config deep merge produces correct results (comprehensive test suite)
- [ ] All existing behavior works unchanged when no workspace is detected
- [ ] `weft workspace delete my-project --confirm` removes the workspace cleanly
