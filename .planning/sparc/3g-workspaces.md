# SPARC Implementation Plan: Phase 3G - Projects/Workspaces System

**Stream**: 3G - Workspaces & Projects
**Status**: Ready for execution
**Timeline**: 2-3 sessions (~6-8 hours estimated LOE)
**Requirements Source**: `.planning/07-workspaces.md`
**Deliverables**: Workspace management CLI, config hierarchy with deep merge, CLAWFT.md loading, hook system, scoped sessions/memory/skills

---

## 1. Specification

### 1.1 Requirements Traceability Matrix

Every functional requirement from `07-workspaces.md` is mapped to an implementation task with acceptance criteria.

| Req ID | Requirement | Implementation Module | Section |
|--------|-------------|----------------------|---------|
| FR-W01 | Workspace Creation | `clawft-core::workspace::WorkspaceManager::create` | 1.2.1 |
| FR-W02 | Workspace Listing | `clawft-core::workspace::WorkspaceManager::list` | 1.2.2 |
| FR-W03 | Workspace Loading/Switching | `clawft-core::workspace::WorkspaceManager::load` | 1.2.3 |
| FR-W04 | Workspace Configuration | `clawft-core::workspace::WorkspaceManager::config_*` | 1.2.4 |
| FR-W05 | Workspace Status | `clawft-core::workspace::WorkspaceManager::status` | 1.2.5 |
| FR-W06 | Workspace Deletion | `clawft-core::workspace::WorkspaceManager::delete` | 1.2.6 |
| FR-W07 | CLAWFT.md Loading | `clawft-core::workspace::load_clawft_md` | 1.2.7 |
| FR-W08 | Workspace-Scoped Sessions | `clawft-core::session::SessionManager::with_dir` | 1.2.8 |
| FR-W09 | Workspace-Scoped Memory | `clawft-core::agent::memory::MemoryStore::with_paths` | 1.2.9 |
| FR-W10 | Workspace-Scoped Skills | `clawft-core::agent::skills::SkillsLoader` extension | 1.2.10 |
| FR-W11 | Workspace Registry | `clawft-types::workspace::WorkspaceRegistry` | 1.2.11 |

### 1.2 Functional Requirements (Detailed)

#### 1.2.1 FR-W01: Workspace Creation

**Command**: `weft workspace create <name>`

**Behavior**:
- Scaffolds `<cwd>/<name>/` directory structure:
  ```
  <name>/
    .clawft/
      config.json          (empty JSON object: {})
      sessions/
      memory/
        MEMORY.md           (empty)
        HISTORY.md          (empty)
      skills/
      agents/
      hooks/
    CLAWFT.md               (starter template with project name and date)
  ```
- Registers the new workspace in `~/.clawft/workspaces.json`
- Fires `workspace.created` hook after scaffolding
- Optional `--git` flag runs `git init` in the workspace root and creates `.gitignore` excluding `.clawft/config.json`, `.clawft/sessions/`, `.clawft/memory/`
- Optional `--template <name>` selects a workspace template (only "blank" in this phase)

**Acceptance Criteria**:
- [ ] `weft workspace create my-project` creates directory at `./my-project/`
- [ ] All subdirectories under `.clawft/` are created (sessions, memory, skills, agents, hooks)
- [ ] `config.json` contains `{}`
- [ ] `CLAWFT.md` contains project name and build section template
- [ ] Workspace appears in `~/.clawft/workspaces.json` after creation
- [ ] `--git` flag produces a `.git/` directory and `.gitignore`
- [ ] `workspace.created` hook fires with `{ name, path, template }` payload
- [ ] Creating a workspace where the directory already exists returns a clear error

#### 1.2.2 FR-W02: Workspace Listing

**Command**: `weft workspace list`

**Behavior**:
- Reads the global workspace registry at `~/.clawft/workspaces.json`
- Also scans child directories of cwd for directories containing `.clawft/` (auto-discovery)
- Displays table: name, path, last accessed timestamp, session count, config overrides summary
- `--all` flag includes workspaces whose path no longer exists (marked as "missing")

**Acceptance Criteria**:
- [ ] Shows registered workspaces from `~/.clawft/workspaces.json`
- [ ] Discovers unregistered workspaces in cwd children
- [ ] Without `--all`, missing workspaces are hidden
- [ ] With `--all`, missing workspaces shown with "missing" status
- [ ] Empty registry shows helpful "no workspaces found" message

#### 1.2.3 FR-W03: Workspace Loading/Switching

**Command**: `weft workspace load <name-or-path>`

**Behavior**:
- Resolves the workspace by name (from registry) or by path
- Sets `$CLAWFT_WORKSPACE` environment variable for child processes
- Updates `last_accessed` in the workspace registry
- Fires `workspace.loaded` hook with `{ name, path, previous_workspace }` payload
- If a previous workspace was active, fires `workspace.unloaded` on it first

**Note**: Since `weft` is a CLI tool (not a persistent daemon), `load` primarily prints an `export` command or updates a dotfile. The primary workspace discovery mechanism for most commands is the directory walk (section 1.3).

**Acceptance Criteria**:
- [ ] `weft workspace load ./my-project` succeeds when `.clawft/` exists
- [ ] `weft workspace load my-project` resolves name via registry
- [ ] Registry `last_accessed` is updated
- [ ] `workspace.loaded` hook fires with correct payload
- [ ] Loading a nonexistent workspace returns a clear error

#### 1.2.4 FR-W04: Workspace Configuration

**Commands**:
- `weft workspace config` -- shows resolved merged config (default action)
- `weft workspace config set <key> <value>` -- sets key in workspace `.clawft/config.json`
- `weft workspace config get <key>` -- shows resolved value with source annotation (default/global/workspace)
- `weft workspace config reset <key>` -- removes workspace override (falls back to global)
- `weft workspace config edit` -- opens workspace config in `$EDITOR`

**Key semantics**:
- Dot-path key syntax for nested values: `agents.defaults.model`
- Config changes are validated against the schema before writing
- Fires `workspace.config_changed` hook with `{ name, path, key, old_value, new_value }`

**Acceptance Criteria**:
- [ ] `weft workspace config` displays merged JSON
- [ ] `weft workspace config set agents.defaults.model openai/gpt-4` writes to workspace config
- [ ] `weft workspace config get agents.defaults.model` shows value and source
- [ ] `weft workspace config reset agents.defaults.model` removes from workspace config
- [ ] `weft workspace config edit` opens `$EDITOR`
- [ ] Invalid key paths produce a clear error
- [ ] `workspace.config_changed` hook fires on set/reset

#### 1.2.5 FR-W05: Workspace Status

**Command**: `weft workspace status`

**Behavior**: Displays current workspace information:
- Workspace path and name
- Config source (global/workspace)
- Session count (`.jsonl` files in sessions/)
- Memory size (bytes of `MEMORY.md`)
- Active skills count
- CLAWFT.md presence
- If no workspace is active (global mode), indicates this clearly

**Acceptance Criteria**:
- [ ] Shows workspace name and path
- [ ] Shows correct session count
- [ ] Shows memory size in bytes
- [ ] Shows skills count
- [ ] Shows CLAWFT.md presence (present/absent)
- [ ] In global mode, shows "(global)" with `~/.clawft/` path

#### 1.2.6 FR-W06: Workspace Deletion

**Command**: `weft workspace delete <name-or-path>`

**Behavior**:
- Requires `--confirm` flag or interactive `y/N` confirmation
- Default: removes `.clawft/` directory and `CLAWFT.md`, leaves all other user files
- `--keep-data` flag: removes from registry only, leaves all files on disk
- Fires `workspace.unloaded` hook if the workspace was active
- De-registers from `~/.clawft/workspaces.json`

**Acceptance Criteria**:
- [ ] Without `--confirm`, prompts for confirmation
- [ ] With `--confirm`, deletes immediately
- [ ] Default behavior removes `.clawft/` and `CLAWFT.md` only
- [ ] `--keep-data` only de-registers (files remain)
- [ ] Workspace removed from registry
- [ ] `workspace.unloaded` hook fires with `reason: "delete"`

#### 1.2.7 FR-W07: CLAWFT.md Loading

**Behavior**:
- When a workspace is active, reads `CLAWFT.md` from the workspace root
- Contents inserted into the system prompt: after base system prompt, before skills
- Supports `@path/to/file` import syntax (relative to workspace root)
- Supports hierarchical loading: parent directory CLAWFT.md files loaded first, concatenated
- File is re-read at each agent session start (not cached across sessions)
- Warn if > 10 KB; truncate at 50 KB with warning

**Security**: Import paths are canonicalized and validated to not escape the workspace root. Path traversal attempts (e.g., `@../../../etc/passwd`) are blocked.

**Acceptance Criteria**:
- [ ] CLAWFT.md content appears in agent system prompt
- [ ] `@path/to/file` imports are resolved (file content replaces reference)
- [ ] Missing import files produce a warning, reference left as-is
- [ ] Import paths cannot escape workspace root
- [ ] Parent directory CLAWFT.md files loaded hierarchically (parent first)
- [ ] Content > 50 KB truncated with warning
- [ ] Content > 10 KB produces a warning
- [ ] No CLAWFT.md = no extra content in system prompt

#### 1.2.8 FR-W08: Workspace-Scoped Sessions

**Behavior**:
- When workspace is active, sessions stored in `<workspace>/.clawft/sessions/`
- `weft sessions list` scoped to active workspace by default
- `weft sessions list --global` shows sessions from `~/.clawft/workspace/sessions/`
- `weft sessions list --all` shows both workspace and global sessions
- Session keys prefixed with workspace name when listed globally

**Implementation**: CLI passes `WorkspaceContext.sessions_dir` to `SessionManager::with_dir()`.

**Acceptance Criteria**:
- [ ] Sessions stored in workspace sessions/ directory
- [ ] `weft sessions list` only shows workspace sessions
- [ ] `--global` flag shows global sessions
- [ ] `--all` flag shows both

#### 1.2.9 FR-W09: Workspace-Scoped Memory

**Behavior**:
- When workspace is active, `MEMORY.md` and `HISTORY.md` read/written from `<workspace>/.clawft/memory/`
- Memory consolidation operates on workspace-scoped memory
- `weft memory show` shows workspace memory (with `--global` flag for global)

**Implementation**: CLI passes workspace memory paths to `MemoryStore::with_paths()`.

**Acceptance Criteria**:
- [ ] Memory files stored in workspace memory/ directory
- [ ] `weft memory show` shows workspace memory
- [ ] `--global` flag shows global memory
- [ ] Memory consolidation operates on workspace memory

#### 1.2.10 FR-W10: Workspace-Scoped Skills

**Behavior**:
- Skills are resolved using the canonical 3-level discovery chain (agreed across 3F and 3G):
  ```
  Priority | Location                        | Scope
  1 (highest) | <workspace>/.clawft/skills/   | Workspace-scoped (project)
  2           | ~/.clawft/skills/             | User-level personal
  3 (lowest)  | ~/.clawft/workspace/skills/   | Global workspace (backward compat)
  ```
- When a 3G workspace is active, level 1 uses `WorkspaceContext.skills_dir` (which is `<workspace>/.clawft/skills/`)
- If a skill name exists at multiple levels, the highest-priority level wins
- `weft skills list` shows skills from all sources with provenance indicator

**Implementation**: `SkillsLoader` gains a `with_dirs` constructor accepting ordered skill directories. When a workspace is active, all three levels are passed; in global mode, only levels 2 and 3.

**Acceptance Criteria**:
- [ ] Workspace skills (level 1) loaded before user-level skills (level 2)
- [ ] User-level skills (level 2) loaded before global workspace skills (level 3)
- [ ] Higher-priority skill overrides lower-priority skill of same name
- [ ] Skills list shows provenance (workspace/user/global)
- [ ] Debug log when a skill shadows another at a lower priority level

#### 1.2.11 FR-W11: Workspace Registry

**Data model** (`~/.clawft/workspaces.json`):
```json
{
  "version": 1,
  "workspaces": [
    {
      "name": "my-backend",
      "path": "/home/user/projects/my-backend",
      "created_at": "2026-02-17T10:00:00Z",
      "last_accessed": "2026-02-17T15:30:00Z"
    }
  ]
}
```

**Behavior**:
- Auto-updated on create, load, and discovery
- Registration idempotent: same path updates existing entry
- Deletion removes entry by path
- Uses atomic writes (write to `.tmp`, then rename) to prevent corruption
- Missing file returns default empty registry

**Acceptance Criteria**:
- [ ] Registry file created on first workspace creation
- [ ] Registration updates existing entry if same path
- [ ] Deregistration removes entry by path
- [ ] Atomic write prevents corruption
- [ ] Missing registry file returns empty registry
- [ ] Roundtrip serialization matches the JSON schema in 07-workspaces.md section 7.1

### 1.3 Workspace Discovery Algorithm

Resolution order (from `07-workspaces.md` section 3.4):

```
1. --workspace <path>        (explicit CLI flag)
2. $CLAWFT_WORKSPACE         (environment variable)
3. Directory walk             (walk up from cwd, looking for .clawft/)
4. Global fallback            (~/.clawft/)
```

**Constraints**:
- Directory walk limited to 20 levels depth
- Discovered path is canonicalized (symlink resolution)
- Walk stops at filesystem root
- Nested workspaces not supported (first `.clawft/` found wins)

### 1.4 Config Hierarchy & Deep Merge

Config fallback chain:
```
1. $CLAWFT_CONFIG          (explicit override -- highest priority)
2. ~/.clawft/config.json   (global)
3. Built-in defaults       (Config::default())
```

When a workspace is active, workspace config is deep-merged over the resolved global config:
```
Layer 1 (lowest):   Built-in defaults     (Config::default())
Layer 2:            Global config          ($CLAWFT_CONFIG or ~/.clawft/config.json)
Layer 3 (highest):  Workspace config       (<workspace>/.clawft/config.json)
```

**Deep merge semantics** (from `07-workspaces.md` section 3.3):
- Scalar values: workspace replaces global
- Objects/maps: recursively merged (workspace keys override; global keys not in workspace preserved)
- Arrays: workspace replaces global entirely (no array merging)
- Explicit `null` in workspace removes the key from the merged result

#### 1.4.1 MCP Server Config Merge Behavior

The `tools.mcp_servers` config field is a JSON object (HashMap of server name to server config). Because it is an object, recursive deep merge applies:

- **Add new servers**: A workspace can define MCP servers not present in global config. After merge, both global and workspace servers are available.
- **Override server config**: A workspace can override `args`, `env`, or other fields for a globally-configured MCP server. The workspace values replace the global values for that specific server entry (recursive object merge).
- **Disable a global server**: Setting a server to `null` in workspace config removes it from the merged result (standard null removal rule). This allows a workspace to opt out of a globally-configured MCP server.
- **Global servers preserved**: Servers defined in global config but not mentioned in workspace config remain unchanged in the merged result.

**Example**: Workspace adds a project-specific MCP server while keeping global servers:

```json
// ~/.clawft/config.json (global)
{
  "tools": {
    "mcp_servers": {
      "github": { "command": "npx", "args": ["-y", "@anthropic/mcp-github"] },
      "slack": { "command": "npx", "args": ["-y", "@anthropic/mcp-slack"] }
    }
  }
}

// <workspace>/.clawft/config.json (workspace)
{
  "tools": {
    "mcp_servers": {
      "rvf": { "command": "npx", "args": ["-y", "@rvf/mcp-server"] },
      "slack": null
    }
  }
}

// Merged result:
// - "github" preserved from global (not mentioned in workspace)
// - "rvf" added from workspace
// - "slack" removed (workspace set it to null)
```

This means workspace-scoped MCP servers are fully supported without any special-case logic -- the standard deep merge handles all cases. Skills and agents within the workspace can then reference these MCP tools using the `{server}__{tool}` naming convention (see 3F SEC-SKILL-08 for namespace isolation).

### 1.5 Non-Functional Requirements

| ID | Requirement | Target | Verification |
|----|-------------|--------|--------------|
| NFR-W01a | Workspace discovery (directory walk) | < 50ms for 10 levels | Benchmark test |
| NFR-W01b | Config deep merge | < 5ms | Benchmark test |
| NFR-W01c | CLAWFT.md loading | < 10ms for 50 KB file | Benchmark test |
| NFR-W01d | Workspace creation | < 200ms | Manual timing |
| NFR-W02 | Backward compatibility | All existing behavior unchanged when no workspace detected | Integration test suite |
| NFR-W03 | Platform compatibility | Linux, macOS, Windows; symlinks; spaces in paths | Cross-platform CI |
| NFR-W04 | WASM compatibility | Discovery/merge via explicit path only (no fs walk) | Conditional compilation |

### 1.6 Security Requirements

From `07-workspaces.md` section 8:
- **Path traversal**: CLAWFT.md imports must not allow `../` to escape workspace root
- **Hook execution**: Hooks run with user permissions, no elevation
- **Symlink resolution**: Workspace paths canonicalized to prevent symlink attacks
- **Config validation**: Workspace config undergoes same validation as global config
- **No secrets in CLAWFT.md**: Document that CLAWFT.md is intended for git; secrets go in `.clawft/config.json` which should be `.gitignore`-d

### 1.7 Non-Goals (Explicit Exclusions)

From `07-workspaces.md` section 2:
- Workspace synchronization across machines (use git)
- Multi-user workspace sharing (no access control)
- Workspace templates marketplace (future phase)
- Nested workspaces

### 1.8 Hook Events

| Event | Trigger | Payload |
|-------|---------|---------|
| `workspace.created` | After `weft workspace create` | `{ name, path, template }` |
| `workspace.loaded` | After `weft workspace load` | `{ name, path, previous_workspace }` |
| `workspace.unloaded` | Before switching away or deleting | `{ name, path, reason }` |
| `workspace.config_changed` | After `weft workspace config set/reset` | `{ name, path, key, old_value, new_value }` |

Hook scripts discovered at two levels (both executed, global first):
1. Global: `~/.clawft/hooks/<event>.sh`
2. Workspace: `<workspace>/.clawft/hooks/<event>.sh`

Payload passed as JSON on stdin. Failures logged as warnings, never blocking.
Timeout: 30 seconds per script. `--no-hooks` global flag suppresses all execution.

---

## 2. Pseudocode

### 2.1 Config Deep Merge

```
fn deep_merge(base: Value, overlay: Value) -> Value:
    match (base, overlay):
        // Both objects: recursive merge
        (Object(base_map), Object(overlay_map)):
            for (key, overlay_val) in overlay_map:
                if overlay_val is Null:
                    base_map.remove(key)       // explicit null = removal
                else if key in base_map AND both are Object:
                    base_map[key] = deep_merge(base_map[key], overlay_val)
                else:
                    base_map[key] = overlay_val // scalar/array replacement or new key
            return Object(base_map)

        // Overlay is null at top level: signal removal
        (_, Null):
            return Null

        // All other cases (scalar, array, type mismatch): overlay wins
        (_, overlay):
            return overlay
```

### 2.2 Workspace Discovery

```
const MAX_WALK_DEPTH = 20
const MARKER = ".clawft"

fn discover_workspace(explicit: Option<Path>, env: &Environment, cwd: &Path) -> Option<Path>:
    // 1. Explicit --workspace flag
    if let Some(path) = explicit:
        if path.join(MARKER).is_dir():
            return Some(canonicalize(path))
        else:
            warn("explicit workspace path has no .clawft/ directory")
            return None

    // 2. $CLAWFT_WORKSPACE env var
    if let Some(env_path) = env.get_var("CLAWFT_WORKSPACE"):
        let path = Path::from(env_path)
        if path.join(MARKER).is_dir():
            return Some(canonicalize(path))
        else:
            warn("$CLAWFT_WORKSPACE has no .clawft/ directory")

    // 3. Walk up from cwd
    let mut current = cwd
    for depth in 0..MAX_WALK_DEPTH:
        if current.join(MARKER).is_dir():
            return Some(canonicalize(current))
        if not current.pop():
            break   // reached filesystem root

    // 4. No workspace found -> global mode
    return None
```

### 2.3 CLAWFT.md Loading with Imports and Hierarchy

```
const MAX_BYTES = 50 * 1024
const WARN_BYTES = 10 * 1024
const IMPORT_RE = r"@([\w./-]+)"

fn load_clawft_md(workspace_root: &Path, platform: &Platform) -> Option<String>:
    let mut chain: Vec<String> = Vec::new()

    // Hierarchical: walk up from workspace root, collect parent CLAWFT.md files
    let mut current = workspace_root.parent()
    let mut depth = 0
    while let Some(dir) = current AND depth < 10:
        let clawft_path = dir.join("CLAWFT.md")
        if platform.fs.exists(clawft_path):
            if let Ok(content) = platform.fs.read(clawft_path):
                chain.push(content)
        current = dir.parent()
        depth += 1

    // Reverse so parents come first
    chain.reverse()

    // Add workspace's own CLAWFT.md
    let ws_clawft = workspace_root.join("CLAWFT.md")
    if platform.fs.exists(ws_clawft):
        if let Ok(content) = platform.fs.read(ws_clawft):
            let resolved = resolve_imports(content, workspace_root, platform)
            chain.push(resolved)

    if chain.is_empty():
        return None

    let combined = chain.join("\n\n---\n\n")
    return Some(truncate(combined))


fn resolve_imports(content: &str, base_dir: &Path, platform: &Platform) -> String:
    let mut resolved = content.clone()
    for match in IMPORT_RE.find_all(content):
        let import_path = base_dir.join(match.captured_path)
        // Security: canonicalize both and verify import is under workspace root
        if let (Ok(canonical_import), Ok(canonical_base)) =
            (import_path.canonicalize(), base_dir.canonicalize()):
            if not canonical_import.starts_with(canonical_base):
                warn("CLAWFT.md import escapes workspace root, skipping")
                continue
        if platform.fs.exists(import_path):
            match platform.fs.read(import_path):
                Ok(import_content) => resolved.replace(full_match, import_content)
                Err(e) => warn("failed to read import: {}", e)
        else:
            debug("import file not found, leaving @reference as-is")
    return resolved


fn truncate(content: String) -> String:
    if content.len() > WARN_BYTES:
        warn("CLAWFT.md is large (> 10 KB)")
    if content.len() > MAX_BYTES:
        warn("CLAWFT.md exceeds 50 KB, truncating")
        return content[..MAX_BYTES]
    return content
```

### 2.4 Workspace Manager Operations

```
struct WorkspaceManager<P: Platform>:
    platform: Arc<P>
    global_dir: PathBuf     // ~/.clawft/

fn create(name, parent_dir, git_init) -> Result<WorkspaceContext>:
    let root = parent_dir.join(name)
    let dot_clawft = root.join(".clawft")

    // Scaffold
    for subdir in [sessions, memory, skills, agents, hooks]:
        platform.fs.create_dir_all(dot_clawft.join(subdir))
    platform.fs.write(dot_clawft.join("config.json"), "{}")
    platform.fs.write(root.join("CLAWFT.md"), template(name))

    // Optional git init
    if git_init:
        platform.process.exec("git", ["init"], root, 10s)
        platform.fs.write(root.join(".gitignore"), gitignore_content)

    // Register
    let mut registry = load_registry()
    registry.register(WorkspaceInfo { name, path: root, created_at: now(), last_accessed: now() })
    save_registry(registry)

    // Hook
    fire_hook("workspace.created", root, { name, path, template: "blank" })

    // Return context
    return load(root, load_global_config())


fn load(workspace_root, global_config) -> Result<WorkspaceContext>:
    let dot_clawft = workspace_root.join(".clawft")

    // Deep merge config
    let config = if dot_clawft.join("config.json").exists():
        let global_json = to_json(global_config)
        let ws_json = parse(read(dot_clawft.join("config.json")))
        let merged = deep_merge(global_json, ws_json)
        from_json(merged)
    else:
        global_config.clone()

    // Load CLAWFT.md with hierarchy and imports
    let clawft_md = load_clawft_md(workspace_root, platform)

    return WorkspaceContext {
        root: Some(workspace_root),
        config,
        clawft_md,
        is_global: false,
        dot_clawft_dir: dot_clawft,
        sessions_dir: dot_clawft.join("sessions"),
        memory_dir: dot_clawft.join("memory"),
        skills_dir: dot_clawft.join("skills"),
    }


fn list(scan_cwd: bool) -> Result<Vec<WorkspaceInfo>>:
    let mut registry = load_registry()
    if scan_cwd:
        // Auto-discover child directories of cwd containing .clawft/
        for entry in platform.fs.list_dir(cwd):
            if entry.join(".clawft").is_dir() AND not already_registered(entry):
                registry.register(auto_discovered_workspace(entry))
    return registry.workspaces


fn delete(workspace_root, keep_data) -> Result<()>:
    let name = workspace_root.file_name()

    // Fire unloaded hook
    fire_hook("workspace.unloaded", workspace_root, { name, path, reason: "delete" })

    if not keep_data:
        // Remove .clawft/ recursively
        platform.fs.remove_dir_all(workspace_root.join(".clawft"))
        // Remove CLAWFT.md
        platform.fs.remove_file(workspace_root.join("CLAWFT.md"))

    // De-register
    let mut registry = load_registry()
    registry.deregister(workspace_root)
    save_registry(registry)


fn status(ctx: &WorkspaceContext) -> WorkspaceStatus:
    let session_count = count .jsonl files in ctx.sessions_dir
    let memory_bytes = size of ctx.memory_dir.join("MEMORY.md")
    let skills_count = count subdirs in ctx.skills_dir with skill.json
    return WorkspaceStatus { name, path, is_global, session_count, memory_bytes, skills_count, has_clawft_md }


fn config_set(ctx, key, value) -> Result<()>:
    let mut ws_config = read_ws_config(ctx)
    set_dot_path(ws_config, key, parse_value(value))
    validate_config(ws_config)?
    write_ws_config(ctx, ws_config)
    fire_hook("workspace.config_changed", ctx.root, { name, path, key, old_value, new_value })


fn config_get(ctx, key) -> Result<ConfigValueWithSource>:
    let ws_config = read_ws_config(ctx)
    let global_config = load_global_config()
    let default_config = Config::default()
    let ws_val = get_dot_path(ws_config, key)
    let global_val = get_dot_path(to_json(global_config), key)
    let default_val = get_dot_path(to_json(default_config), key)
    if ws_val.is_some(): return (ws_val, source: "workspace")
    if global_val.is_some(): return (global_val, source: "global")
    return (default_val, source: "default")


fn config_reset(ctx, key) -> Result<()>:
    let mut ws_config = read_ws_config(ctx)
    let old_value = remove_dot_path(ws_config, key)
    write_ws_config(ctx, ws_config)
    fire_hook("workspace.config_changed", ctx.root, { name, path, key, old_value, new_value: null })
```

### 2.5 Hook Execution

```
fn fire_hook(event: &str, workspace_root: &Path, payload: &Value):
    let payload_json = to_json_string(payload)

    // Global hook first
    let global_hook = global_dir.join("hooks").join(format!("{event}.sh"))
    if global_hook.exists() AND global_hook.is_executable():
        run_hook_script(global_hook, payload_json)

    // Workspace hook second
    let ws_hook = workspace_root.join(".clawft/hooks").join(format!("{event}.sh"))
    if ws_hook.exists() AND ws_hook.is_executable():
        run_hook_script(ws_hook, payload_json)


fn run_hook_script(script: &Path, stdin_json: &str):
    match timeout(30s, platform.process.exec_with_stdin(script, [], stdin_json)):
        Ok(Ok(output)) if output.status.success() => debug("hook completed")
        Ok(Ok(output)) => warn("hook exited with non-zero status")
        Ok(Err(e)) => warn("hook execution failed: {}", e)
        Err(_timeout) => warn("hook timed out after 30s, killing")
```

### 2.6 ContextBuilder Integration

```
// Modified build_messages order in clawft-core/src/agent/context.rs:

fn build_messages(session, active_skills, clawft_md: Option<&str>) -> Vec<LlmMessage>:
    let mut messages = Vec::new()

    // 1. Base system prompt
    messages.push(system(build_system_prompt()))

    // 2. CLAWFT.md content (NEW insertion point)
    if let Some(content) = clawft_md:
        messages.push(system(format!("# Project Instructions\n\n{content}")))

    // 3. Active skill prompts
    for skill in active_skills:
        messages.push(system(format!("# Skill: {}\n\n{}", skill.name, skill.prompt)))

    // 4. Memory context
    if let Ok(memory) = memory.read_long_term() AND not memory.is_empty():
        messages.push(system(format!("# Relevant Memory:\n\n{memory}")))

    // 5. Conversation history
    for msg in session.get_history(memory_window):
        messages.push(LlmMessage { role: msg.role, content: msg.content })

    return messages
```

### 2.7 CLI Resolution Flow

```
// In main.rs, before dispatching any command:

fn main():
    let cli = Cli::parse()   // includes --workspace flag, --no-hooks flag

    // Resolve workspace context once, pass to all commands
    let platform = NativePlatform::new()
    let workspace_ctx = resolve_workspace_context(
        &platform,
        cli.workspace.as_deref(),  // explicit --workspace flag
    )

    match cli.command:
        Commands::Agent(args) => run_agent(args, workspace_ctx)
        Commands::Gateway(args) => run_gateway(args, workspace_ctx)
        Commands::Sessions { action } => run_sessions(action, workspace_ctx)
        Commands::Memory { action } => run_memory(action, workspace_ctx)
        Commands::Workspace { action } => run_workspace(action, workspace_ctx)
        // ... other commands use workspace_ctx.config instead of load_config()


fn resolve_workspace_context(platform, explicit_ws) -> WorkspaceContext:
    let env_var = platform.env.get_var("CLAWFT_WORKSPACE")
    let cwd = std::env::current_dir()

    let workspace_root = discover_workspace(explicit_ws, env_var, cwd)
    // Config fallback: $CLAWFT_CONFIG -> ~/.clawft/config.json -> built-in defaults
    let config_path = platform.env.get_var("CLAWFT_CONFIG")
        .unwrap_or(home.join(".clawft/config.json"))
    let global_config = load_global_config(platform, config_path)

    if let Some(root) = workspace_root:
        let mgr = WorkspaceManager::new(platform)
        mgr.load(root, global_config)
    else:
        // Global mode: construct WorkspaceContext pointing to ~/.clawft/
        WorkspaceContext {
            root: None,
            config: global_config,
            clawft_md: None,
            is_global: true,
            dot_clawft_dir: home.join(".clawft"),
            sessions_dir: home.join(".clawft/workspace/sessions"),
            memory_dir: home.join(".clawft/workspace/memory"),
            skills_dir: home.join(".clawft/workspace/skills"),
        }
```

---

## 3. Architecture

### 3.1 Module Layout

```
clawft-types/src/
  lib.rs                     # MODIFY: add `pub mod workspace;`
  config.rs                  # MODIFY: add `pub fn deep_merge(...)`
  workspace.rs               # NEW: WorkspaceInfo, WorkspaceRegistry, WorkspaceContext

clawft-core/src/
  lib.rs                     # MODIFY: add `pub mod workspace;`
  workspace.rs               # NEW: WorkspaceManager, discover_workspace, CLAWFT.md loading, hooks
  agent/
    context.rs               # MODIFY: add clawft_md parameter to ContextBuilder
    memory.rs                # (already has with_paths -- used as-is)
    skills.rs                # MODIFY: add workspace-scoped skill resolution (or no change if CLI handles precedence)

clawft-cli/src/
  main.rs                    # MODIFY: add --workspace flag, --no-hooks flag, Workspace subcommand
  commands/
    mod.rs                   # MODIFY: add workspace module, add resolve_workspace_context helper
    workspace.rs             # NEW: workspace CLI subcommand handlers
    agent.rs                 # MODIFY: accept WorkspaceContext, pass to agent loop
    gateway.rs               # MODIFY: accept WorkspaceContext
    sessions.rs              # MODIFY: accept WorkspaceContext for scoped sessions dir
    memory_cmd.rs            # MODIFY: accept WorkspaceContext for scoped memory dir
```

### 3.2 New Types (`clawft-types/src/workspace.rs`)

```rust
/// Metadata for a single known workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

/// Persistent registry of known workspaces.
/// Stored at ~/.clawft/workspaces.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceRegistry {
    #[serde(default = "default_registry_version")]
    pub version: u32,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceInfo>,
}

/// Resolved workspace context for the current execution.
/// Passed from CLI to all subsystems.
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    /// Workspace root directory (None = global mode).
    pub root: Option<PathBuf>,
    /// Resolved (merged) configuration.
    pub config: Config,
    /// Content of CLAWFT.md (if present).
    pub clawft_md: Option<String>,
    /// Whether this is the global fallback workspace.
    pub is_global: bool,
    /// Path to the .clawft/ directory.
    pub dot_clawft_dir: PathBuf,
    /// Path to sessions directory.
    pub sessions_dir: PathBuf,
    /// Path to memory directory.
    pub memory_dir: PathBuf,
    /// Path to skills directory.
    pub skills_dir: PathBuf,
}
```

### 3.3 Dependency Flow

```
clawft-types (workspace.rs, config.rs additions)
    |
    v
clawft-core (workspace.rs)  -- depends on: types, platform
    |
    v
clawft-cli (commands/workspace.rs) -- depends on: core, types, platform
```

No new external crate dependencies. Uses existing workspace deps:
- `serde_json` (deep merge, config loading)
- `chrono` (timestamps)
- `regex` (CLAWFT.md import resolution)
- `tracing` (logging)
- `dirs` (home directory)
- `comfy-table` (CLI output formatting -- already a dependency)

### 3.4 Config Resolution Flow

```
1. CLI parses --workspace flag
2. resolve_workspace_context() called:
   a. discover_workspace(explicit, $CLAWFT_WORKSPACE, cwd)
   b. load_global_config: $CLAWFT_CONFIG or ~/.clawft/config.json -> serde_json::Value
   c. If workspace found:
      - Load workspace config: <ws>/.clawft/config.json -> serde_json::Value
      - deep_merge(global_json, workspace_json)
      - Deserialize merged JSON -> Config
      - Load CLAWFT.md (hierarchy + imports + truncation)
   d. Build WorkspaceContext with resolved paths
3. WorkspaceContext passed to subsystems:
   - SessionManager::with_dir(ctx.sessions_dir)
   - MemoryStore::with_paths(ctx.memory_dir/MEMORY.md, ctx.memory_dir/HISTORY.md)
   - SkillsLoader::with_dirs([ctx.skills_dir, ~/.clawft/skills/, ~/.clawft/workspace/skills/])
   - ContextBuilder gains clawft_md: ctx.clawft_md
```

### 3.5 CLI Command Hierarchy (After Changes)

```
weft
+-- agent                  (MODIFY: workspace-aware)
+-- gateway                (MODIFY: workspace-aware)
+-- status                 (existing)
+-- channels
|   +-- status
+-- cron
|   +-- list / add / remove / enable / disable / run
+-- sessions               (MODIFY: workspace scoping)
|   +-- list [--global | --all]
|   +-- inspect <id>
|   +-- delete <id>
+-- memory                 (MODIFY: workspace scoping)
|   +-- show [--global]
|   +-- history [--global]
|   +-- search <query>
+-- config                 (existing, now uses merged config)
|   +-- show
|   +-- section <name>
+-- workspace              (NEW)
|   +-- create <name> [--git] [--template] [--path]
|   +-- list [--all]
|   +-- load <name-or-path>
|   +-- config [show | set <key> <value> | get <key> | reset <key> | edit]
|   +-- status
|   +-- delete <name-or-path> [--confirm] [--keep-data]
+-- completions
+-- (global flags)
    +-- --verbose
    +-- --workspace <path>  (NEW)
    +-- --no-hooks          (NEW)
```

### 3.6 Workspace Directory Layout

```
~/.clawft/                            # Global workspace (existing)
  config.json                         # Global config
  workspaces.json                     # NEW: Workspace registry
  workspace/                          # Global sessions/memory/skills (existing)
    sessions/
    memory/
      MEMORY.md
      HISTORY.md
    skills/
  hooks/                              # NEW: Global hooks directory
    workspace.created.sh
    workspace.loaded.sh
    workspace.unloaded.sh
    workspace.config_changed.sh

~/projects/my-backend/                # A project workspace
  .clawft/                            # Workspace marker
    config.json                       # Workspace-level config overrides
                                      #   (includes tools.mcp_servers overrides --
                                      #    can add, override, or null-remove MCP servers;
                                      #    see section 1.4.1 for merge semantics)
    sessions/                         # Workspace-scoped sessions
    memory/
      MEMORY.md
      HISTORY.md
    skills/                           # Workspace-scoped skills
    agents/                           # Workspace-scoped agent definitions (future)
    hooks/
      workspace.loaded.sh             # Per-workspace hooks
  CLAWFT.md                           # Project instructions (loaded into system prompt)
  src/                                # User's project files
  Cargo.toml
```

### 3.7 ContextBuilder Integration Point

Current message assembly order in `context.rs`:
```
1. Base system prompt (identity, config info)
2. Bootstrap files (AGENTS.md, SOUL.md, USER.md, TOOLS.md, IDENTITY.md)
3. Active skill prompts
4. Long-term memory context
5. Conversation history
```

After Phase 3G, the order becomes:
```
1. Base system prompt (identity, config info)
2. Bootstrap files
3. CLAWFT.md content (NEW -- workspace project instructions)
4. Active skill prompts
5. Long-term memory context
6. Conversation history
```

The `ContextBuilder` struct gains a `clawft_md: Option<String>` field set at construction time. The `build_messages` method inserts it as a system message between bootstrap files and skills.

### 3.8 Error Handling Strategy

| Error Case | Behavior |
|------------|----------|
| Workspace config invalid JSON | Error with parse details; CLI exits 1 |
| CLAWFT.md import file not found | Warning log, leave `@reference` as-is |
| CLAWFT.md exceeds 50 KB | Warning log, truncate to 50 KB |
| CLAWFT.md exceeds 10 KB | Warning log (no truncation) |
| Hook script fails (non-zero exit) | Warning log, continue operation |
| Hook script times out (30s) | Warning log, kill process, continue |
| Hook script not executable | Warning log, skip |
| Workspace registry corrupt | Warning log, start with empty registry |
| `--workspace` path has no `.clawft/` | Error, CLI exits 1 |
| `weft workspace delete` without `--confirm` | Interactive y/N prompt |
| Duplicate workspace path in registry | Update existing entry (idempotent) |
| No home directory detectable | Error, CLI exits 1 |

### 3.9 WASM Considerations (NFR-W04)

- `discover_workspace` directory walk uses `std::fs` directly -- in WASM builds, the walk is skipped and only explicit path or env var work
- `deep_merge` is pure computation (no I/O) -- fully WASM-compatible
- `CLAWFT.md` loading via WASI filesystem -- works if platform provides filesystem
- Hook execution requires `ProcessSpawner` -- unavailable in WASM; hooks are no-ops

Implementation: `#[cfg(not(target_arch = "wasm32"))]` guards on directory walk and hook execution.

---

## 4. Refinement (TDD Implementation Plan)

### 4.1 Implementation Order

The implementation proceeds in 5 phases, each following Red-Green-Refactor:

```
Phase A: Types + Deep Merge       (clawft-types)      ~1.5 hours
Phase B: Workspace Discovery      (clawft-core)       ~1 hour
Phase C: Workspace Manager        (clawft-core)       ~2 hours
Phase D: CLI Commands             (clawft-cli)         ~2 hours
Phase E: Integration              (cross-crate)        ~1.5 hours
```

### 4.2 Phase A: Types + Deep Merge

**Crate**: `clawft-types`
**Files**: `src/workspace.rs` (new), `src/config.rs` (modify), `src/lib.rs` (modify)

#### A.1: Write Tests (Red)

**File: `clawft-types/src/workspace.rs` -- tests**

```rust
// Serde roundtrip
#[test] fn workspace_info_serializes_roundtrip()
#[test] fn workspace_registry_empty_default()
#[test] fn workspace_registry_register_and_find()
#[test] fn workspace_registry_register_updates_existing()
#[test] fn workspace_registry_deregister()
#[test] fn workspace_registry_deserializes_from_07_workspaces_json_schema()
#[test] fn workspace_registry_touch_updates_last_accessed()
#[test] fn workspace_registry_find_by_path()
#[test] fn workspace_registry_find_returns_none_for_unknown()
```

**File: `clawft-types/src/config.rs` -- deep_merge tests**

```rust
#[test] fn deep_merge_scalar_override()
#[test] fn deep_merge_nested_objects()
#[test] fn deep_merge_array_replacement()
#[test] fn deep_merge_null_removal()
#[test] fn deep_merge_base_only_keys_preserved()
#[test] fn deep_merge_empty_overlay_noop()
#[test] fn deep_merge_deep_nesting_3_levels()
#[test] fn deep_merge_add_new_keys()
#[test] fn deep_merge_real_config_fixture()
#[test] fn merge_workspace_config_produces_valid_config()
#[test] fn deep_merge_null_at_nested_level()
#[test] fn deep_merge_mixed_types_overlay_wins()
```

#### A.2: Implement (Green)

1. Create `clawft-types/src/workspace.rs` with `WorkspaceInfo`, `WorkspaceRegistry`, `WorkspaceContext`
2. Add `pub fn deep_merge(base: Value, overlay: Value) -> Value` to `config.rs`
3. Add `pub fn merge_workspace_config(global: Value, workspace: Value) -> Result<Config>` to `config.rs`
4. Add `pub mod workspace;` to `lib.rs`

#### A.3: Refactor

- Ensure `WorkspaceRegistry` methods (register, deregister, find, touch) use idiomatic Rust
- Add doc comments matching existing code style

**Acceptance Criteria for Phase A**:
- [ ] All workspace type tests pass
- [ ] All deep_merge tests pass (12+ test cases)
- [ ] `cargo test -p clawft-types` passes
- [ ] `cargo clippy -p clawft-types` clean

### 4.3 Phase B: Workspace Discovery

**Crate**: `clawft-core`
**Files**: `src/workspace.rs` (new), `src/lib.rs` (modify)

#### B.1: Write Tests (Red)

```rust
#[test] fn discover_workspace_explicit_path()
#[test] fn discover_workspace_explicit_path_without_marker_returns_none()
#[test] fn discover_workspace_env_var()
#[test] fn discover_workspace_env_var_without_marker_ignored()
#[test] fn discover_workspace_directory_walk_finds_parent()
#[test] fn discover_workspace_directory_walk_finds_self()
#[test] fn discover_workspace_no_workspace_returns_none()
#[test] fn discover_workspace_walk_stops_at_root()
#[test] fn discover_workspace_walk_limited_to_20_levels()
#[test] fn discover_workspace_explicit_takes_precedence_over_env()
#[test] fn discover_workspace_env_takes_precedence_over_walk()
```

#### B.2: Implement (Green)

1. Create `clawft-core/src/workspace.rs` with `discover_workspace()` function
2. Add constants: `MAX_WALK_DEPTH = 20`, `WORKSPACE_MARKER = ".clawft"`
3. Add `pub mod workspace;` to `src/lib.rs`

#### B.3: Refactor

- Extract `canonicalize_opt` helper
- Add `tracing::debug` for discovery steps

**Acceptance Criteria for Phase B**:
- [ ] All 11 discovery tests pass
- [ ] `cargo test -p clawft-core -- workspace` passes

### 4.4 Phase C: Workspace Manager

**Crate**: `clawft-core`
**Files**: `src/workspace.rs` (extend)

#### C.1: Write Tests (Red)

```rust
// WorkspaceManager::create
#[tokio::test] async fn create_scaffolds_directory_structure()
#[tokio::test] async fn create_writes_empty_config_json()
#[tokio::test] async fn create_writes_clawft_md_template()
#[tokio::test] async fn create_registers_in_registry()
#[tokio::test] async fn create_returns_valid_context()
#[tokio::test] async fn create_with_git_runs_git_init()

// WorkspaceManager::load
#[tokio::test] async fn load_reads_workspace_config()
#[tokio::test] async fn load_merges_workspace_and_global_config()
#[tokio::test] async fn load_reads_clawft_md()
#[tokio::test] async fn load_without_clawft_md_returns_none()
#[tokio::test] async fn load_without_workspace_config_uses_global()

// CLAWFT.md
#[test] fn resolve_imports_replaces_at_references()
#[test] fn resolve_imports_missing_file_leaves_reference()
#[test] fn resolve_imports_blocks_path_traversal()
#[test] fn truncate_clawft_md_under_limit_unchanged()
#[test] fn truncate_clawft_md_over_limit_truncated()
#[test] fn load_clawft_md_hierarchical_parent_first()

// WorkspaceManager::list
#[tokio::test] async fn list_returns_registered_workspaces()
#[tokio::test] async fn list_empty_registry()

// WorkspaceManager::delete
#[tokio::test] async fn delete_removes_dot_clawft_and_clawft_md()
#[tokio::test] async fn delete_keep_data_only_deregisters()
#[tokio::test] async fn delete_deregisters_from_registry()

// WorkspaceManager::status
#[tokio::test] async fn status_counts_sessions_and_memory()
#[tokio::test] async fn status_global_mode()

// WorkspaceManager::config operations
#[tokio::test] async fn config_set_writes_to_workspace_config()
#[tokio::test] async fn config_get_returns_value_with_source()
#[tokio::test] async fn config_get_falls_through_global_to_default()
#[tokio::test] async fn config_reset_removes_workspace_key()

// Hook system
#[tokio::test] async fn fire_hook_executes_global_then_workspace()
#[tokio::test] async fn fire_hook_non_executable_skipped()
#[tokio::test] async fn fire_hook_failure_does_not_block()
```

#### C.2: Implement (Green)

1. `WorkspaceManager` struct with `new()`, `create()`, `load()`, `list()`, `delete()`, `status()`, `config_set()`, `config_get()`, `config_reset()`
2. `fire_hook()` and `run_hook_script()` internals
3. `resolve_imports()`, `truncate_clawft_md()`, `load_clawft_md_hierarchical()`
4. `WorkspaceStatus` struct
5. Registry I/O helpers with atomic writes

#### C.3: Refactor

- Move dot-path get/set/remove into utility functions
- Ensure all filesystem access goes through `Platform` trait (no direct `std::fs`)

**Acceptance Criteria for Phase C**:
- [ ] All 30+ workspace manager tests pass
- [ ] `cargo test -p clawft-core -- workspace` passes
- [ ] No direct `std::fs` usage (only platform abstraction)

### 4.5 Phase D: CLI Commands

**Crate**: `clawft-cli`
**Files**: `src/main.rs` (modify), `src/commands/mod.rs` (modify), `src/commands/workspace.rs` (new)

#### D.1: Write Tests (Red)

```rust
// CLI parsing tests (in main.rs)
#[test] fn cli_workspace_create_parses()
#[test] fn cli_workspace_create_with_git_parses()
#[test] fn cli_workspace_create_with_path_parses()
#[test] fn cli_workspace_list_parses()
#[test] fn cli_workspace_list_all_parses()
#[test] fn cli_workspace_load_parses()
#[test] fn cli_workspace_status_parses()
#[test] fn cli_workspace_delete_parses()
#[test] fn cli_workspace_delete_keep_data_parses()
#[test] fn cli_workspace_config_show_parses()
#[test] fn cli_workspace_config_set_parses()
#[test] fn cli_workspace_config_get_parses()
#[test] fn cli_workspace_config_reset_parses()
#[test] fn cli_workspace_config_edit_parses()
#[test] fn cli_global_workspace_flag_parses()
#[test] fn cli_no_hooks_flag_parses()
#[test] fn cli_has_workspace_subcommand()
#[test] fn cli_workspace_flag_with_agent_subcommand()
```

#### D.2: Implement (Green)

1. Add `workspace: Option<String>` and `no_hooks: bool` to `Cli` struct (global flags)
2. Add `Workspace { action: WorkspaceAction }` to `Commands` enum
3. Create `src/commands/workspace.rs` with:
   - `WorkspaceAction` enum (Create, List, Load, Config, Status, Delete)
   - `WorkspaceConfigAction` enum (Show, Set, Get, Reset, Edit)
   - Handler functions: `workspace_create`, `workspace_list`, `workspace_load`, `workspace_config`, `workspace_status`, `workspace_delete`
4. Add `pub mod workspace;` to `commands/mod.rs`
5. Add `resolve_workspace_context()` helper to `commands/mod.rs`
6. Add match arm in `main()` for `Commands::Workspace`
7. Update `commands/sessions.rs` to accept `--global` and `--all` flags
8. Update `commands/memory_cmd.rs` to accept `--global` flag

#### D.3: Refactor

- Extract workspace context resolution to be shared across all commands
- Ensure all existing CLI tests still pass

**Acceptance Criteria for Phase D**:
- [ ] All 18 CLI parsing tests pass
- [ ] All existing CLI tests still pass (backward compatibility)
- [ ] `cargo test -p clawft-cli` passes

### 4.6 Phase E: Integration

**Crates**: cross-crate integration

#### E.1: ContextBuilder Integration

**File**: `clawft-core/src/agent/context.rs`

Changes:
- Add `clawft_md: Option<String>` field to `ContextBuilder`
- Add `with_clawft_md(mut self, content: Option<String>) -> Self` builder method
- In `build_messages()`, insert CLAWFT.md content as system message after bootstrap files, before skills

Tests:
```rust
#[tokio::test] async fn build_messages_includes_clawft_md()
#[tokio::test] async fn build_messages_clawft_md_before_skills()
#[tokio::test] async fn build_messages_no_clawft_md_unchanged()
```

#### E.2: MemoryStore Integration

**File**: `clawft-core/src/agent/memory.rs`

Changes:
- Make `with_paths()` public (currently `#[cfg(test)]`)
- CLI passes workspace memory paths

Tests:
```rust
#[tokio::test] async fn memory_store_with_workspace_paths()
```

#### E.3: SkillsLoader Integration

**File**: `clawft-core/src/agent/skills.rs`

Changes:
- Add `with_dirs(dirs: Vec<PathBuf>, platform: Arc<P>) -> Self` constructor accepting ordered skill directories (highest priority first)
- When workspace is active, pass 3 directories: `[<workspace>/.clawft/skills/, ~/.clawft/skills/, ~/.clawft/workspace/skills/]`
- In global mode, pass 2 directories: `[~/.clawft/skills/, ~/.clawft/workspace/skills/]`
- `list_skills()` scans all directories in priority order; first occurrence of a skill name wins
- `load_skill()` checks directories in priority order; first match wins
- When a skill shadows another at a lower priority level, log at debug level

Tests:
```rust
#[tokio::test] async fn skills_loader_workspace_dir_takes_precedence()
#[tokio::test] async fn skills_loader_user_dir_takes_precedence_over_global()
#[tokio::test] async fn skills_loader_falls_back_through_chain()
#[tokio::test] async fn skills_loader_lists_all_sources_with_provenance()
```

#### E.4: Agent Command Integration

**File**: `clawft-cli/src/commands/agent.rs`

Changes:
- Accept `WorkspaceContext` parameter
- Use `ctx.sessions_dir` for SessionManager
- Use `ctx.memory_dir` for MemoryStore
- Use `ctx.skills_dir` for SkillsLoader
- Pass `ctx.clawft_md` to ContextBuilder

#### E.5: Gateway Command Integration

**File**: `clawft-cli/src/commands/gateway.rs`

Changes:
- Accept `WorkspaceContext` parameter
- Use workspace-scoped directories for all subsystems

#### E.6: Full Integration Tests

```rust
// clawft-core/tests/workspace_integration.rs

#[tokio::test] async fn workspace_create_load_agent_session_uses_workspace_dirs()
#[tokio::test] async fn no_workspace_uses_global_dirs()
#[tokio::test] async fn workspace_config_merge_affects_agent_behavior()
#[tokio::test] async fn clawft_md_appears_in_system_prompt()
```

**Acceptance Criteria for Phase E**:
- [ ] ContextBuilder includes CLAWFT.md in correct position
- [ ] MemoryStore works with workspace paths
- [ ] SkillsLoader checks workspace dir before global
- [ ] Agent command uses workspace context
- [ ] All existing tests still pass
- [ ] `cargo test --workspace` passes

### 4.7 Test Coverage Requirements

| Module | Minimum Coverage | Critical Paths (100%) |
|--------|-----------------|----------------------|
| `clawft-types::workspace` | >= 80% | WorkspaceRegistry CRUD |
| `clawft-types::config::deep_merge` | 100% | All merge semantics |
| `clawft-core::workspace::discover_workspace` | 100% | All resolution paths |
| `clawft-core::workspace::resolve_imports` | >= 90% | Path traversal blocking |
| `clawft-core::workspace::truncate_clawft_md` | 100% | Size limits |
| `clawft-core::workspace::WorkspaceManager` | >= 80% | create, load, delete |
| `clawft-cli` workspace parsing | >= 90% | All subcommand variants |

---

## 5. Completion

### 5.1 Files to Create

| File | Crate | Purpose |
|------|-------|---------|
| `clawft-types/src/workspace.rs` | clawft-types | WorkspaceInfo, WorkspaceRegistry, WorkspaceContext types |
| `clawft-core/src/workspace.rs` | clawft-core | WorkspaceManager, discovery, CLAWFT.md loading, hooks |
| `clawft-cli/src/commands/workspace.rs` | clawft-cli | CLI workspace subcommand handlers |

### 5.2 Files to Modify

| File | Crate | Changes |
|------|-------|---------|
| `clawft-types/src/lib.rs` | clawft-types | Add `pub mod workspace;` |
| `clawft-types/src/config.rs` | clawft-types | Add `deep_merge()` and `merge_workspace_config()` |
| `clawft-core/src/lib.rs` | clawft-core | Add `pub mod workspace;` |
| `clawft-core/src/agent/context.rs` | clawft-core | Add `clawft_md` field, insert into prompt |
| `clawft-core/src/agent/memory.rs` | clawft-core | Make `with_paths()` public |
| `clawft-core/src/agent/skills.rs` | clawft-core | Add `with_dirs()` for 3-level skill discovery chain |
| `clawft-cli/src/main.rs` | clawft-cli | Add `--workspace`, `--no-hooks` flags, Workspace command |
| `clawft-cli/src/commands/mod.rs` | clawft-cli | Add workspace module, `resolve_workspace_context()` |
| `clawft-cli/src/commands/agent.rs` | clawft-cli | Accept WorkspaceContext |
| `clawft-cli/src/commands/gateway.rs` | clawft-cli | Accept WorkspaceContext |
| `clawft-cli/src/commands/sessions.rs` | clawft-cli | Add `--global`, `--all` flags |
| `clawft-cli/src/commands/memory_cmd.rs` | clawft-cli | Add `--global` flag |

### 5.3 Pre-Merge Validation Checklist

#### Build & Quality
- [ ] `cargo build --release` succeeds
- [ ] `cargo test --workspace` passes (all crates)
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` passes
- [ ] No new compiler warnings

#### Workspace Lifecycle (maps to 07-workspaces.md section 10)
- [ ] `weft workspace create my-project` scaffolds complete workspace
- [ ] `weft workspace list` shows all known workspaces
- [ ] `weft workspace load my-project` activates the workspace
- [ ] `weft workspace status` shows active workspace details
- [ ] Running `weft agent -m "hello"` inside a workspace uses workspace config
- [ ] CLAWFT.md content appears in agent system prompt
- [ ] Sessions created inside workspace stored in workspace sessions/
- [ ] Memory consolidation operates on workspace-scoped memory
- [ ] Skills from workspace take precedence over global skills
- [ ] Workspace hooks fire on create/load/unload events
- [ ] Config deep merge produces correct results (comprehensive test suite)
- [ ] All existing behavior works unchanged when no workspace is detected
- [ ] `weft workspace delete my-project --confirm` removes workspace cleanly

#### Config Hierarchy
- [ ] Global config loaded from `$CLAWFT_CONFIG` (if set) or `~/.clawft/config.json`
- [ ] Workspace config deep-merged over global
- [ ] Scalar override works
- [ ] Nested object merge preserves base keys
- [ ] Array replacement (no merge)
- [ ] Null removal works
- [ ] `weft workspace config set/get/reset` operations work correctly

#### CLAWFT.md
- [ ] Read from workspace root
- [ ] `@path/to/file` imports resolved
- [ ] Path traversal blocked
- [ ] Hierarchical loading (parent + workspace)
- [ ] Size warnings and truncation

#### Workspace Discovery
- [ ] `--workspace ./path` flag works
- [ ] `$CLAWFT_WORKSPACE` env var works
- [ ] Directory walk from nested dir finds workspace
- [ ] No workspace = global mode
- [ ] Symlinks resolved

#### Scoped Subsystems
- [ ] Sessions: workspace-scoped storage
- [ ] Memory: workspace-scoped storage
- [ ] Skills: 3-level discovery chain (workspace > user > global workspace)

#### Hooks
- [ ] `workspace.created` fires
- [ ] `workspace.loaded` fires
- [ ] `workspace.unloaded` fires
- [ ] `workspace.config_changed` fires
- [ ] Global hooks execute before workspace hooks
- [ ] JSON payload on stdin
- [ ] Hook failure logs warning, does not block
- [ ] `--no-hooks` suppresses all hooks
- [ ] 30-second timeout

#### Backward Compatibility (NFR-W02)
- [ ] All existing commands work without workspace
- [ ] `~/.clawft/` serves as global workspace
- [ ] No migration step required
- [ ] Existing config.json, sessions, memory all work
- [ ] `weft agent -m "hello"` works without any workspace

#### Performance (NFR-W01)
- [ ] Discovery < 50ms (10 levels)
- [ ] Deep merge < 5ms
- [ ] CLAWFT.md loading < 10ms (50 KB)
- [ ] Workspace creation < 200ms

#### Security
- [ ] CLAWFT.md import path traversal blocked
- [ ] Config validated before merge
- [ ] Hooks run as user only
- [ ] Registry uses atomic writes

### 5.4 Branch Strategy

- **Feature Branch**: `feature/phase-3g-workspaces`
- **PR Target**: `main`
- **Agent Coordination**:
  - Coder Agent 1: Types + workspace manager (`clawft-types`, `clawft-core`)
  - Coder Agent 2: CLI commands (`clawft-cli`)
  - Agents can work in parallel after types are defined in Phase A

### 5.5 Risk Mitigations

| Risk | Mitigation |
|------|------------|
| Config merge ambiguity | 12+ test cases covering all merge semantics; `weft workspace config show --merged` for debugging |
| Directory walk performance | Limit to 20 levels; no caching needed (runs once per invocation) |
| Hook script security | Hooks run as user; document risks; `--no-hooks` escape hatch |
| Workspace state corruption | Atomic writes for registry; config validated before write |
| Platform path issues | Use `PathBuf` everywhere; canonicalize on load |
| CLAWFT.md bloats prompt | Warn at 10 KB; truncate at 50 KB |

### 5.6 Notes for Implementation Agent

1. **Read existing config loading flow** (`commands/mod.rs` `load_config`) before modifying it. The workspace-aware version must preserve all existing behavior.
2. **`deep_merge` is the most critical function**. Get the tests right first, then implement. It must handle all edge cases in the test suite.
3. **Workspace discovery must be fast**. Cache nothing -- the directory walk is short and runs once per invocation.
4. **CLAWFT.md import resolution** must prevent path traversal. Canonicalize both the import path and the workspace root, then verify the import is under the root.
5. **Hook execution** is fire-and-forget. Never let a hook failure block the user's operation.
6. **The registry file** (`workspaces.json`) must use atomic writes (write to `.tmp`, rename) to prevent corruption.
7. **Global `--workspace` flag** must be parsed before any subcommand dispatch. Clap handles this with `global = true`.
8. **Backward compatibility is paramount**. Run all existing tests after changes. If no workspace is detected, behavior must be identical to pre-workspace code.
9. **Do not create workspace templates** beyond "blank" in this phase. Template support is a future enhancement.
10. **Use `tracing`** for all debug/warning logs, matching existing crate patterns.
11. **`MemoryStore::with_paths`** is currently `#[cfg(test)]` -- it must be made public for workspace integration.
12. **`SkillsLoader::with_dir`** is currently `#[cfg(test)]` -- it must be made public, and a new `with_dirs(dirs: Vec<PathBuf>)` variant added for the canonical 3-level skill discovery chain: `[<workspace>/.clawft/skills/, ~/.clawft/skills/, ~/.clawft/workspace/skills/]`.
13. **Existing `load_config` in `commands/mod.rs`** must still work for commands that do not yet use WorkspaceContext. The migration path is: (a) add `resolve_workspace_context`, (b) update commands one by one to use it, (c) eventually deprecate the per-command `--config` flag in favor of `--workspace`.

### 5.7 Cross-Stream Dependencies

| Dependency | Status | Impact on Phase 3G |
|------------|--------|-------------------|
| Phase 3F (Skills/Agents) | In progress | `SkillsLoader` may be refactored; workspace skill integration should use the same extension points |
| Phase 2F (Security) | Complete | Path traversal protection patterns reusable from `clawft-tools` |
| Phase 2H (CLI Gaps) | Complete | CLI subcommand patterns established; follow the same structure |
| Phase 2E (Integration Wiring) | Complete | `commands/mod.rs` load_config pattern established; workspace-aware version extends it |
