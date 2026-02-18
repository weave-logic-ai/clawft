# SPARC Implementation Plan: Stream 3F - Agents, Skills, Commands, Helpers

**Phase**: 3F (Part 2 of Phase 3: Finish)
**Owned Crates**: `clawft-core` (skills, agents, helpers), `clawft-cli` (commands, interactive shell, help)
**Dependencies**: Phase 1 complete (core engine, CLI, tools), Phase 2D (CLI completion)
**Estimated LOE**: 3-4 weeks

---

## 1. Agent Instructions

### Existing Rust Files to Read

```
clawft/crates/clawft-core/src/agent/skills.rs      # Existing SkillsLoader (skill.json + prompt.md)
clawft/crates/clawft-core/src/agent/context.rs      # ContextBuilder (skill prompt injection)
clawft/crates/clawft-core/src/agent/loop_core.rs    # AgentLoop (message processing pipeline)
clawft/crates/clawft-core/src/agent/memory.rs       # MemoryStore
clawft/crates/clawft-core/src/agent/mod.rs          # Agent module structure
clawft/crates/clawft-cli/src/main.rs                # CLI entrypoint (clap)
clawft/crates/clawft-cli/src/commands/mod.rs        # Command registry, load_config
clawft/crates/clawft-cli/src/commands/agent.rs      # Interactive REPL (slash commands)
clawft/crates/clawft-tools/src/lib.rs               # Tool registry and registration
clawft/crates/clawft-types/src/config.rs            # Config types (AgentsConfig)
```

### Planning Documents (MUST READ)

```
.planning/01-business-requirements.md    # G5 config compat, workspace compat
.planning/02-technical-requirements.md   # SkillsLoader spec, workspace layout
.planning/03-development-guide.md        # Phase 3 scope, file ownership
```

### External Reference: Claude Code Skills Format

The Claude Code skills format (Agent Skills open standard, https://agentskills.io) uses
`SKILL.md` with YAML frontmatter. OpenClaw/ClawHub uses the same format. clawft should
support both the existing `skill.json` + `prompt.md` format AND the `SKILL.md` format
for maximum compatibility.

### Module Structure

```
clawft-core/src/
  agent/
    mod.rs                          # EDIT: add skills_v2, agents, helpers modules
    skills.rs                       # EXISTING: SkillsLoader (skill.json format)
    skills_v2.rs                    # NEW: Enhanced SkillDefinition, SKILL.md parser, SkillRegistry
    agents.rs                       # NEW: AgentDefinition, AgentLoader, AgentRegistry
    helpers.rs                      # NEW: Helper traits and built-in helpers
    context.rs                      # EDIT: integrate SkillRegistry + AgentDefinition

clawft-cli/src/
  commands/
    mod.rs                          # EDIT: add skills_cmd, agents_cmd modules
    agent.rs                        # EDIT: expand interactive slash commands
    skills_cmd.rs                   # NEW: weft skills {list,show,run,install}
    agents_cmd.rs                   # NEW: weft agents {list,show,use}
    help_cmd.rs                     # NEW: weft help <topic>
  interactive/
    mod.rs                          # NEW: Slash command dispatcher
    registry.rs                     # NEW: SlashCommandRegistry
    builtins.rs                     # NEW: Built-in slash commands

clawft-types/src/
  skill.rs                          # NEW: Shared skill/agent metadata types
```

---

## 2. Specification

### 2.1 Skill System

#### FR-3F-001: Dual-Format Skill Loading

The skill system MUST support two file formats:

**Format A: Legacy (existing)** -- `skill.json` + `prompt.md`
```
skills/research/
  skill.json     # {"name":"research","description":"...","variables":["topic"]}
  prompt.md      # LLM instructions
```

**Format B: SKILL.md (Claude Code / OpenClaw compatible)**
```
skills/research/
  SKILL.md       # YAML frontmatter + markdown instructions
  reference.md   # Optional supporting files
  scripts/
    validate.sh  # Optional scripts
```

SKILL.md format:
```yaml
---
name: research
description: Deep research on a topic. Use when investigating technologies or concepts.
version: 1.0.0
variables:
  - topic
  - depth
argument-hint: "[topic] [depth]"
disable-model-invocation: false
user-invocable: true
allowed-tools:
  - read_file
  - web_search
  - web_fetch
context: inline
metadata:
  openclaw:
    requires:
      bins: []
      env: []
    primaryEnv: ""
---

When researching $ARGUMENTS:

1. Start with a broad overview of the topic
2. Identify key concepts and relationships
3. Provide concrete examples
4. Summarize findings with references
```

**Detection precedence**: When a skill directory contains both `SKILL.md` and `skill.json`,
`SKILL.md` takes precedence. The loader checks for `SKILL.md` first, then falls back to
`skill.json`.

#### FR-3F-002: Skill Discovery Chain

Skills are discovered from multiple locations with explicit precedence (highest wins):

| Priority | Location | Scope |
|----------|----------|-------|
| 1 (highest) | `<project>/.clawft/skills/` | Project-scoped (via cwd walk-up for `.clawft/`) |
| 2 | `~/.clawft/skills/` | User-level personal |
| 3 (lowest) | `~/.clawft/workspace/skills/` | Global workspace (backward compat) |

When skills share the same name across levels, the higher-priority location wins.

The `SkillRegistry` merges all locations into a single namespace. Each skill entry tracks
its source location for diagnostics.

> **Note:** The `~/.clawft/workspace/skills/` path (level 3) is the current global default.
> When 3G workspaces ship, this remains the fallback global workspace. At that point,
> `Project` scope becomes workspace-aware via `WorkspaceContext.skills_dir`.

#### FR-3F-003: Skill Metadata Types

```rust
/// Enhanced skill definition supporting both legacy and SKILL.md formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Skill identifier (matches directory name if not specified).
    pub name: String,

    /// Human-readable description (used by LLM to decide when to apply skill).
    pub description: String,

    /// Semantic version.
    #[serde(default = "default_version")]
    pub version: String,

    /// Template variable names expected by the skill prompt.
    #[serde(default)]
    pub variables: Vec<String>,

    /// Hint shown during autocomplete for expected arguments.
    #[serde(default)]
    pub argument_hint: Option<String>,

    /// If true, only the user can invoke this skill (not the LLM).
    #[serde(default)]
    pub disable_model_invocation: bool,

    /// If false, hide from the /skills menu. Use for background knowledge.
    #[serde(default = "default_true")]
    pub user_invocable: bool,

    /// Tools the agent can use without confirmation when this skill is active.
    ///
    /// **MCP tool naming**: MCP tools are namespaced as `{server}__{tool}` (double
    /// underscore convention, e.g., `github__create_repo`). Entries in `allowed_tools`
    /// support two matching modes:
    /// - **Exact match**: `"github__create_repo"` matches only that tool.
    /// - **Prefix glob**: `"github__*"` matches all tools from the `github` MCP server.
    ///
    /// When `allowed_tools` is empty, all currently registered tools (including MCP
    /// tools) are permitted. When non-empty, only tools matching an entry are allowed.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Execution context: "inline" (default) or "fork" (subagent).
    #[serde(default = "default_context")]
    pub context: SkillContext,

    /// Agent type to use when context is "fork".
    #[serde(default)]
    pub agent: Option<String>,

    /// OpenClaw/ClawHub metadata block (for compatibility).
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,

    /// Source format: Legacy (skill.json) or SkillMd (SKILL.md).
    #[serde(skip)]
    pub source_format: SkillFormat,

    /// Filesystem path to the skill directory.
    #[serde(skip)]
    pub source_path: PathBuf,

    /// LLM instructions loaded from prompt.md or SKILL.md body.
    #[serde(skip)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum SkillContext {
    #[default]
    Inline,
    Fork,
}

#[derive(Debug, Clone, Default)]
pub enum SkillFormat {
    #[default]
    Legacy,
    SkillMd,
}
```

#### FR-3F-004: SKILL.md Parser

The parser extracts YAML frontmatter and markdown body from a `SKILL.md` file:

```rust
/// Parse a SKILL.md file into a SkillDefinition.
///
/// The file format is:
/// ```text
/// ---
/// name: my-skill
/// description: Does things
/// ---
///
/// Markdown instructions here...
/// ```
///
/// If no frontmatter is present, the entire file is treated as the prompt
/// and the skill name is inferred from the directory name.
pub fn parse_skill_md(content: &str, dir_name: &str) -> Result<SkillDefinition>
```

**Frontmatter parsing rules**:
- Frontmatter is delimited by `---` on its own line at the start of the file
- YAML is parsed with `serde_yaml` (or a lightweight YAML front-matter parser)
- Fields use kebab-case in YAML (e.g., `disable-model-invocation`) and are mapped
  to snake_case Rust fields via serde rename
- The markdown body (after the second `---`) becomes the skill's `prompt`
- If `name` is omitted, use the directory name
- If `description` is omitted, use the first paragraph of the markdown body

#### FR-3F-005: Skill Registry

```rust
/// Unified skill registry that merges skills from all discovery locations.
pub struct SkillRegistry<P: Platform> {
    /// Skills indexed by name (higher-priority overwrites lower).
    skills: Arc<RwLock<HashMap<String, SkillDefinition>>>,
    /// Ordered list of search directories (highest priority first).
    search_paths: Vec<SkillSearchPath>,
    platform: Arc<P>,
}

#[derive(Debug, Clone)]
pub struct SkillSearchPath {
    pub path: PathBuf,
    pub scope: SkillScope,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkillScope {
    /// Project-level: <project>/.clawft/skills/
    Project,
    /// User-level: ~/.clawft/skills/
    User,
    /// Global workspace: ~/.clawft/workspace/skills/
    Workspace,
}
```

**Registry API**:
```rust
impl<P: Platform> SkillRegistry<P> {
    /// Create a new registry with standard search paths.
    pub fn new(platform: Arc<P>, project_dir: Option<PathBuf>) -> Result<Self>;

    /// Scan all search paths and populate the registry.
    pub async fn discover(&self) -> Result<()>;

    /// List all discovered skills with their metadata.
    pub async fn list(&self) -> Vec<SkillSummary>;

    /// Get a skill definition by name (loads prompt lazily if needed).
    pub async fn get(&self, name: &str) -> Result<Option<SkillDefinition>>;

    /// Get skills whose descriptions match the given context (for auto-invocation).
    pub async fn match_context(&self, user_input: &str) -> Vec<SkillDefinition>;

    /// Resolve $ARGUMENTS and $N placeholders in a skill prompt.
    pub fn render_prompt(skill: &SkillDefinition, args: &[String]) -> String;
}

#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub scope: SkillScope,
    pub format: SkillFormat,
    pub user_invocable: bool,
}
```

#### FR-3F-006: String Substitution in Skill Prompts

Skill prompts support these substitutions at invocation time:

| Variable | Description |
|----------|-------------|
| `$ARGUMENTS` | All arguments passed when invoking the skill |
| `$ARGUMENTS[N]` | Argument by 0-based index |
| `${N}` | Shorthand for `$ARGUMENTS[N]` (e.g., `${0}`, `${1}`) |
| `$SESSION_ID` | Current session identifier |

Indexed argument shorthand MUST use braced syntax `${N}` (not bare `$N`) to avoid
collision with dollar amounts in prompts (e.g., "$100"). `$ARGUMENTS` and
`$ARGUMENTS[N]` are unchanged.

If `$ARGUMENTS` is not present in the prompt and arguments were provided,
append `ARGUMENTS: <value>` to the end.

---

### 2.2 Agent Definitions

#### FR-3F-010: Agent Definition Format

Agent definitions configure named agent personas with specific tools, models, and
system prompts. Stored as YAML files.

**File format**: `agent.yaml` or `AGENT.md` (with YAML frontmatter)

```yaml
# ~/.clawft/agents/researcher.yaml
name: researcher
description: Deep research agent with web access
model: anthropic/claude-sonnet-4-20250514
system_prompt: |
  You are a research specialist. Focus on accuracy, cite sources,
  and provide comprehensive analysis.
tools:
  - read_file
  - web_search
  - web_fetch
  - memory_read
  - memory_write
skills:
  - research
  - summarize
max_tool_iterations: 20
temperature: 0.3
context_window: 100000
```

**AGENT.md format** (alternative):
```yaml
---
name: researcher
description: Deep research agent
model: anthropic/claude-sonnet-4-20250514
tools: [read_file, web_search, web_fetch]
skills: [research, summarize]
max_tool_iterations: 20
temperature: 0.3
---

You are a research specialist. Focus on accuracy, cite sources,
and provide comprehensive analysis.
```

#### FR-3F-011: Agent Discovery Chain

| Priority | Location | Scope |
|----------|----------|-------|
| 1 (highest) | `.clawft/agents/` (project-level) | Current project only |
| 2 | `~/.clawft/agents/` | All projects (personal) |
| 3 (lowest) | Built-in agents | Always available |

**Built-in agents** (compiled into the binary):

| Name | Description | Key Traits |
|------|-------------|------------|
| `default` | General-purpose agent (uses config defaults) | All tools |
| `explore` | Read-only exploration | read_file, list_directory, web_search |
| `coder` | Code-focused with file write access | All file tools + exec |
| `planner` | Planning mode (no tool execution) | No tools, high temperature |

#### FR-3F-012: Agent Definition Types

```rust
/// A named agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent identifier.
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// LLM model to use (overrides config default).
    #[serde(default)]
    pub model: Option<String>,

    /// System prompt (if using agent.yaml; body if using AGENT.md).
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Allowed tool names. If specified, only these tools are registered for the
    /// agent session.
    ///
    /// **Empty list (`tools: []`) means "all registered tools"** -- this includes
    /// every MCP tool from every configured MCP server. This is a security
    /// consideration: an agent with `tools: []` has access to all MCP server
    /// capabilities. For tighter control, list tools explicitly.
    ///
    /// **MCP tool naming**: MCP tools use `{server}__{tool}` format (double
    /// underscore). Entries support exact match (`"github__create_repo"`) and
    /// prefix glob (`"github__*"`) to include all tools from an MCP server.
    /// When `tools` is non-empty, MCP tools MUST be explicitly listed by full
    /// namespaced name or prefix glob to be available.
    ///
    /// Example: `tools: ["read_file", "write_file", "rvf__*"]` grants file
    /// tools plus all RVF MCP tools, but no other MCP server tools.
    #[serde(default)]
    pub tools: Vec<String>,

    /// Skills to preload into context.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Maximum tool call iterations per message.
    #[serde(default)]
    pub max_tool_iterations: Option<u32>,

    /// LLM temperature override.
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Context window size override.
    #[serde(default)]
    pub context_window: Option<usize>,

    /// Source path (for diagnostics).
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}
```

#### FR-3F-013: Agent Registry

```rust
pub struct AgentRegistry<P: Platform> {
    agents: Arc<RwLock<HashMap<String, AgentDefinition>>>,
    search_paths: Vec<PathBuf>,
    platform: Arc<P>,
}

impl<P: Platform> AgentRegistry<P> {
    pub fn new(platform: Arc<P>, project_dir: Option<PathBuf>) -> Self;

    /// Scan search paths and register all agents.
    pub async fn discover(&self) -> Result<()>;

    /// List all known agents.
    pub async fn list(&self) -> Vec<AgentSummary>;

    /// Get an agent definition by name.
    pub async fn get(&self, name: &str) -> Option<AgentDefinition>;

    /// Register built-in agents.
    fn register_builtins(&self);
}
```

#### FR-3F-014: Agent Selection from CLI

```bash
# Use a named agent
weft agent --agent researcher -m "What is WASM?"

# List available agents
weft agents list

# Show agent details
weft agents show researcher

# Interactive: use slash command
> /agent researcher
Switched to agent: researcher (anthropic/claude-sonnet-4-20250514)
```

When `--agent <name>` is passed:
1. Look up `AgentDefinition` in the registry
2. Override model, system_prompt, temperature, max_tool_iterations from the definition
3. Filter the tool registry to only include the agent's allowed tools
4. Preload the agent's listed skills into context

---

### 2.3 Interactive Slash Commands

#### FR-3F-020: Slash Command System

The interactive REPL (`weft agent`) supports slash commands prefixed with `/`.
Commands are registered in a `SlashCommandRegistry` and dispatched before
messages are sent to the LLM.

**Existing commands** (in `agent.rs`):
- `/help` -- show help
- `/exit` / `/quit` -- exit session
- `/clear` -- clear session
- `/tools` -- list tools

**New commands**:

| Command | Description | Implementation |
|---------|-------------|----------------|
| `/skills` | List available skills | `SkillRegistry::list()` |
| `/skills <name>` | Show skill details | `SkillRegistry::get()` |
| `/skill <name> [args]` | Invoke a skill | Load skill, render prompt, send to LLM |
| `/agents` | List available agents | `AgentRegistry::list()` |
| `/agent <name>` | Switch to a named agent | Reconfigure session |
| `/memory` | Show memory summary | `MemoryStore::read_long_term()` |
| `/memory search <q>` | Search memory | `MemoryStore::search()` |
| `/config` | Show current config | Display effective config |
| `/config <key>` | Show specific config value | Dot-notation lookup |
| `/workspace` | Show workspace info | Display paths, skill/agent counts |
| `/model <name>` | Switch model mid-session | Override effective model |
| `/session` | Show current session info | Session ID, message count, etc. |

#### FR-3F-021: Slash Command Registry

```rust
/// Trait for implementing interactive slash commands.
#[async_trait]
pub trait SlashCommand: Send + Sync {
    /// Command name (without the `/` prefix).
    fn name(&self) -> &str;

    /// Short description shown in /help.
    fn description(&self) -> &str;

    /// Detailed help text shown in /help <command>.
    fn help(&self) -> &str;

    /// Execute the command with the given arguments.
    /// Returns Some(output) to print, or None if handled internally.
    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> Result<Option<String>>;

    /// Tab-completion candidates for the given partial input.
    fn completions(&self, partial: &str) -> Vec<String> {
        Vec::new()
    }
}

/// Context passed to slash commands during execution.
pub struct CommandContext {
    pub config: Config,
    pub skill_registry: Arc<SkillRegistry<NativePlatform>>,
    pub agent_registry: Arc<AgentRegistry<NativePlatform>>,
    pub memory: Arc<MemoryStore<NativePlatform>>,
    pub session_id: String,
    pub effective_model: String,
    pub tool_names: Vec<String>,
}

/// Registry of all available slash commands.
pub struct SlashCommandRegistry {
    commands: HashMap<String, Box<dyn SlashCommand>>,
}

impl SlashCommandRegistry {
    pub fn new() -> Self;

    /// Register a slash command.
    pub fn register(&mut self, cmd: Box<dyn SlashCommand>);

    /// Try to dispatch a slash command.
    pub async fn dispatch(&self, input: &str, ctx: &mut CommandContext) -> Result<DispatchResult>;

    /// List all registered commands.
    pub fn list(&self) -> Vec<(&str, &str)>;
}
```

#### FR-3F-022: Skill Invocation from Slash Command

When the user types `/skill research quantum computing`:

1. Parse: command = "skill", skill_name = "research", args = ["quantum", "computing"]
2. Look up skill in `SkillRegistry`
3. If not found, print error
4. If `context == Fork`, log warning "subagent fork not yet supported, running inline"
   and fall through to inline execution (Phase 4 follow-up)
5. If `context == Inline` (or Fork deferred), render the skill prompt with `$ARGUMENTS` substituted
6. Send the rendered prompt to the LLM as a user message
7. The skill's `allowed_tools` are temporarily added to the permission set

When the user types `/research quantum computing` (skill name directly as command):
- Skills with `user_invocable: true` are registered as top-level slash commands
- Same execution flow as `/skill <name> <args>`

---

### 2.4 Helpers

#### FR-3F-030: Helper Module

Helpers are utility functions that skills, commands, and agent internals can use.
They are stateless functions grouped by domain, NOT tools (they are not exposed
to the LLM).

```rust
// clawft-core/src/agent/helpers.rs

/// File system helpers (for skills that need path manipulation).
pub mod fs_helpers {
    /// Resolve a path relative to the workspace.
    pub fn resolve_workspace_path(workspace: &Path, relative: &str) -> PathBuf;

    /// Find files matching a glob pattern in a directory.
    pub async fn glob_files<P: Platform>(platform: &P, base: &Path, pattern: &str) -> Result<Vec<PathBuf>>;

    /// Read a file if it exists, return None otherwise.
    pub async fn read_optional<P: Platform>(platform: &P, path: &Path) -> Result<Option<String>>;
}

/// Config helpers.
pub mod config_helpers {
    /// Get a config value by dot-notation path (e.g., "agents.defaults.model").
    pub fn get_by_path(config: &Config, path: &str) -> Option<serde_json::Value>;

    /// Resolve the effective workspace directory.
    pub fn resolve_workspace(config: &Config, platform_home: &Path) -> PathBuf;

    /// Resolve the effective skills directories (3 search paths, highest priority first):
    /// 1. <project>/.clawft/skills/ (if project_dir is Some)
    /// 2. ~/.clawft/skills/
    /// 3. ~/.clawft/workspace/skills/
    pub fn resolve_skill_paths(config: &Config, platform_home: &Path, project_dir: Option<&Path>) -> Vec<PathBuf>;
}

/// Template helpers (for skill prompt rendering).
pub mod template_helpers {
    /// Substitute $ARGUMENTS, $ARGUMENTS[N], $N, $SESSION_ID in a template.
    pub fn render_template(template: &str, args: &[String], session_id: &str) -> String;

    /// Extract variable references from a template.
    pub fn extract_variables(template: &str) -> Vec<String>;
}

/// Git helpers (for workspace-aware operations).
pub mod git_helpers {
    /// Detect if a directory is inside a git repository.
    pub async fn is_git_repo<P: Platform>(platform: &P, dir: &Path) -> bool;

    /// Get the git root directory for a path.
    pub async fn git_root<P: Platform>(platform: &P, dir: &Path) -> Result<Option<PathBuf>>;

    /// Get the current branch name.
    pub async fn current_branch<P: Platform>(platform: &P, dir: &Path) -> Result<Option<String>>;
}
```

---

### 2.5 Help System

#### FR-3F-040: CLI Help

```bash
# Top-level help
weft help                    # Same as weft --help
weft help skills             # Detailed help on skills system
weft help agents             # Detailed help on agent definitions
weft help commands           # List all slash commands with descriptions
weft help config             # Config file format and locations

# Per-command help
weft skills --help           # Skills subcommand help
weft agents --help           # Agents subcommand help
weft skills show --help      # Specific subcommand help
```

#### FR-3F-041: Interactive Help

```
> /help
Commands:
  /help [topic]        Show help (topics: skills, agents, commands, config)
  /skills              List available skills
  /skill <name> [args] Invoke a skill
  /agents              List available agents
  /agent <name>        Switch to a named agent
  /tools               List available tools
  /memory              Show memory summary
  /config              Show current configuration
  /model <name>        Switch model
  /session             Show session info
  /workspace           Show workspace info
  /clear               Clear session history
  /exit                Exit the session

> /help skills
Skills are reusable prompts that teach the agent specialized behaviors.

Locations (highest priority first):
  1. .clawft/skills/     (project-level)
  2. ~/.clawft/skills/   (personal)
  3. ~/.clawft/workspace/skills/ (workspace)

Formats:
  - SKILL.md with YAML frontmatter (recommended)
  - skill.json + prompt.md (legacy)

Usage:
  /skills                List all skills
  /skills <name>         Show skill details
  /skill <name> [args]   Invoke a skill
  /<skill-name> [args]   Direct invocation (if user-invocable)
```

#### FR-3F-042: Help Topics

Each help topic is a static markdown string compiled into the binary:

```rust
pub struct HelpSystem {
    topics: HashMap<String, HelpTopic>,
}

pub struct HelpTopic {
    pub name: String,
    pub summary: String,
    pub body: String,
}

impl HelpSystem {
    pub fn new() -> Self;
    pub fn get(&self, topic: &str) -> Option<&HelpTopic>;
    pub fn list_topics(&self) -> Vec<(&str, &str)>;
}
```

Topics: `skills`, `agents`, `commands`, `config`, `memory`, `tools`, `workspace`.

---

### 2.6 CLI Subcommands

#### FR-3F-050: `weft skills` Subcommand

```
weft skills list [--scope <project|user|all>] [--format <table|json>]
weft skills show <name>
weft skills run <name> [args...]
```

**`weft skills list`**:
```
NAME            DESCRIPTION                     SCOPE     FORMAT
research        Deep research on topics         user      SKILL.md
code-review     Review code for quality         project   SKILL.md
summarize       Summarize documents             user      legacy
deploy          Deploy to production            project   SKILL.md
```

**`weft skills show <name>`**:
```
Skill: research
Description: Deep research on a topic
Version: 1.0.0
Format: SKILL.md
Scope: user (~/.clawft/skills/research/)
Variables: topic, depth
Argument Hint: [topic] [depth]
User Invocable: yes
Model Invocable: yes
Context: inline
Allowed Tools: read_file, web_search, web_fetch
```

**`weft skills run <name> [args]`**:
Invokes the skill in single-message mode (non-interactive). Renders the
skill prompt with the given arguments and sends it through the agent loop.

#### FR-3F-051: `weft agents` Subcommand

```
weft agents list [--format <table|json>]
weft agents show <name>
```

**`weft agents list`**:
```
NAME            DESCRIPTION                     MODEL                              SCOPE
default         General-purpose agent           (config default)                   built-in
explore         Read-only exploration           (config default)                   built-in
coder           Code-focused agent              (config default)                   built-in
researcher      Deep research agent             anthropic/claude-sonnet-4-20250514 user
reviewer        Code review specialist          anthropic/claude-sonnet-4-20250514 project
```

**`weft agents show <name>`**:
```
Agent: researcher
Description: Deep research agent with web access
Model: anthropic/claude-sonnet-4-20250514
Temperature: 0.3
Max Tool Iterations: 20
Scope: user (~/.clawft/agents/researcher.yaml)
Tools: read_file, web_search, web_fetch, memory_read, memory_write
Skills: research, summarize
```

#### FR-3F-052: `weft help` Subcommand

```
weft help                # General help
weft help skills         # Skills system documentation
weft help agents         # Agent definitions documentation
weft help commands       # Interactive commands reference
weft help config         # Configuration reference
```

Output is rendered as terminal-friendly text (no markdown rendering needed).

---

### 2.7 Workspace Integration (3G Tie-in)

#### FR-3F-060: Project Detection

When `weft agent` starts, detect the current project for project-level
skill/agent discovery:

1. Check `$CLAWFT_PROJECT` environment variable
2. Walk up from `cwd` looking for `.clawft/` directory
3. If none found, project-level paths are not used

Project detection uses only the `.clawft/` marker directory. There is no `.git/` fallback --
a project must explicitly opt in to clawft by having a `.clawft/` directory.

The detected project root is used to resolve `.clawft/skills/` and `.clawft/agents/`.

---

### 2.8 Compatibility Requirements

#### NFR-3F-001: Backward Compatibility

- Existing `skill.json` + `prompt.md` skills MUST continue to work
- Existing `~/.clawft/workspace/skills/` directory MUST continue to be discovered
  (level 3 in the discovery chain)
- No changes to the existing `SkillsLoader` public API (it remains for internal use
  by the new `SkillRegistry` which wraps it)

#### NFR-3F-002: OpenClaw Compatibility

- `SKILL.md` files with `metadata.openclaw` blocks MUST be loadable
- `requires.bins` and `requires.env` fields MUST be validated (warn if requirements
  not met, but still load the skill)
- Skills from ClawHub (installed via future `weft skills install`) should work
  without modification

#### NFR-3F-003: Claude Code Compatibility

- `SKILL.md` frontmatter fields from Claude Code's specification MUST be parsed
  and respected: `name`, `description`, `disable-model-invocation`, `user-invocable`,
  `allowed-tools`, `context`, `agent`, `argument-hint`
- `$ARGUMENTS`, `$ARGUMENTS[N]`, `${N}` substitutions MUST work
- `context: fork` MUST be parsed and accepted in skill definitions. However, subagent
  fork is not yet supported -- log a warning "subagent fork not yet supported, running
  inline" and execute the skill inline. Track as Phase 4 follow-up.

#### NFR-3F-004: Performance

- Skill discovery (full scan of all paths) MUST complete in < 100ms for up to
  100 skills
- Skill prompt rendering MUST complete in < 1ms
- Agent definition loading MUST complete in < 50ms

### Security Considerations

#### SEC-SKILL-01: YAML Parsing Depth Limits
All YAML frontmatter parsing MUST limit recursion depth to prevent stack overflow from deeply nested structures. Use `serde_yaml`'s built-in depth limit or pre-validate structure depth.

#### SEC-SKILL-02: Directory Name Validation
Skill and agent directory names MUST match `[a-zA-Z0-9_-]+`. Reject names containing `.`, `/`, `\`, or any path separator. This prevents path traversal via crafted directory names.

#### SEC-SKILL-03: Skill Prompt Size Limits
Skill prompts are injected into the LLM context. A configurable `max_skill_prompt_bytes` (default: 32KB) MUST be enforced. Log a warning and truncate if exceeded.

#### SEC-SKILL-04: Tool Permission Intersection
When a skill declares `allowed_tools`, the effective tool set MUST be the intersection (not union) of the skill's `allowed_tools` and the current session's permitted tools. A skill cannot escalate tool access beyond what the user/agent already has.

#### SEC-SKILL-05: Agent Model Validation
Agent definitions specifying a `model` field MUST be validated against the configured providers list. Reject unknown model strings with a clear error to prevent LLM traffic redirection to untrusted endpoints.

#### SEC-SKILL-06: Project Skill Trust
When project-level skills or agents (from `.clawft/skills/` or `.clawft/agents/`) override user-level or global skills, log a warning indicating the override. Consider a `--trust-project-skills` flag for explicit opt-in on first encounter (similar to Claude Code's trust model).

#### SEC-SKILL-07: Metadata Field Limits
The `metadata: Option<serde_json::Value>` field in skill definitions allows arbitrary nested data. Enforce a maximum serialized size of 8KB and a maximum nesting depth of 10 to prevent resource exhaustion.

#### SEC-SKILL-08: MCP Tool Namespace Isolation
When a skill or agent specifies `allowed_tools` (or `tools` for agents), MCP-namespaced tools (`{server}__{tool}`) MUST match either by exact name or by prefix glob (`{server}__*`). Empty `allowed_tools` / `tools: []` means "all currently registered tools" including all MCP tools from all configured servers. **Project-scoped skills SHOULD NOT automatically get access to all MCP tools** -- when a project-level skill declares `allowed_tools`, it must explicitly list or glob the MCP tools it needs. This prevents a project skill from silently gaining access to sensitive MCP server capabilities (e.g., a project skill should not automatically get `payments__*` tools just because the user has a payments MCP server configured).

---

## 3. Pseudocode

### 3.1 SKILL.md Parser

```rust
pub fn parse_skill_md(content: &str, dir_name: &str) -> Result<SkillDefinition> {
    // Check for YAML frontmatter
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        // No frontmatter: entire file is the prompt
        return Ok(SkillDefinition {
            name: dir_name.to_string(),
            description: extract_first_paragraph(content),
            prompt: Some(content.to_string()),
            source_format: SkillFormat::SkillMd,
            ..Default::default()
        });
    }

    // Find closing ---
    let rest = &content[4..]; // skip opening "---\n"
    let end_idx = rest.find("\n---\n")
        .or_else(|| rest.find("\n---\r\n"))
        .ok_or_else(|| ClawftError::PluginLoadFailed {
            plugin: format!("skill/{dir_name}: unterminated YAML frontmatter"),
        })?;

    let yaml_str = &rest[..end_idx];
    let body_start = end_idx + 5; // skip "\n---\n"
    let body = rest[body_start..].trim_start();

    // Parse YAML frontmatter
    let mut def: SkillDefinition = serde_yaml::from_str(yaml_str)
        .map_err(|e| ClawftError::PluginLoadFailed {
            plugin: format!("skill/{dir_name}: invalid YAML frontmatter: {e}"),
        })?;

    // Apply defaults
    if def.name.is_empty() {
        def.name = dir_name.to_string();
    }
    if def.description.is_empty() {
        def.description = extract_first_paragraph(body);
    }

    def.prompt = if body.is_empty() { None } else { Some(body.to_string()) };
    def.source_format = SkillFormat::SkillMd;

    Ok(def)
}

fn extract_first_paragraph(text: &str) -> String {
    text.lines()
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}
```

### 3.2 Skill Registry Discovery

```rust
impl<P: Platform> SkillRegistry<P> {
    pub async fn discover(&self) -> Result<()> {
        let mut merged: HashMap<String, SkillDefinition> = HashMap::new();

        // Iterate from LOWEST priority to HIGHEST (so higher overwrites lower)
        for search_path in self.search_paths.iter().rev() {
            if !self.platform.fs().exists(&search_path.path).await {
                continue;
            }

            let entries = self.platform.fs().list_dir(&search_path.path).await
                .map_err(ClawftError::Io)?;

            for entry in entries {
                let dir_name = match entry.file_name() {
                    Some(name) => name.to_string_lossy().into_owned(),
                    None => continue,
                };

                // Try SKILL.md first, then skill.json
                let skill_md_path = entry.join("SKILL.md");
                let skill_json_path = entry.join("skill.json");

                let skill = if self.platform.fs().exists(&skill_md_path).await {
                    let content = self.platform.fs().read_to_string(&skill_md_path).await
                        .map_err(|e| ClawftError::PluginLoadFailed {
                            plugin: format!("skill/{dir_name}: {e}"),
                        })?;
                    let mut def = parse_skill_md(&content, &dir_name)?;
                    def.source_path = entry.clone();
                    Some(def)
                } else if self.platform.fs().exists(&skill_json_path).await {
                    // Legacy format: delegate to existing parser
                    let json_content = self.platform.fs().read_to_string(&skill_json_path).await
                        .map_err(|e| ClawftError::PluginLoadFailed {
                            plugin: format!("skill/{dir_name}: {e}"),
                        })?;
                    let legacy: LegacySkillJson = serde_json::from_str(&json_content)
                        .map_err(|e| ClawftError::PluginLoadFailed {
                            plugin: format!("skill/{dir_name}: {e}"),
                        })?;

                    // Load prompt.md
                    let prompt_path = entry.join("prompt.md");
                    let prompt = if self.platform.fs().exists(&prompt_path).await {
                        self.platform.fs().read_to_string(&prompt_path).await.ok()
                    } else {
                        None
                    };

                    Some(SkillDefinition::from_legacy(legacy, prompt, entry.clone()))
                } else {
                    None
                };

                if let Some(mut s) = skill {
                    // Tag with scope
                    // (scope is determined by which search_path matched)
                    merged.insert(s.name.clone(), s);
                }
            }
        }

        let mut cache = self.skills.write().await;
        *cache = merged;
        Ok(())
    }
}
```

### 3.3 Slash Command Dispatch

```rust
/// Result of slash command dispatch.
pub enum DispatchResult {
    /// Not a recognized command -- pass input through to LLM as-is.
    NotACommand,
    /// Command handled internally (e.g., /help, /clear) -- no LLM action needed.
    Handled,
    /// Skill invocation -- the rendered prompt should be sent to the LLM as a user message.
    SkillInvocation(String),
}

impl SlashCommandRegistry {
    pub async fn dispatch(&self, input: &str, ctx: &mut CommandContext) -> Result<DispatchResult> {
        let input = input.trim();
        if !input.starts_with('/') {
            return Ok(DispatchResult::NotACommand);
        }

        let without_slash = &input[1..];
        let (cmd_name, args) = match without_slash.split_once(' ') {
            Some((name, rest)) => (name, rest.trim()),
            None => (without_slash, ""),
        };

        // 1. Check built-in commands first
        if let Some(cmd) = self.commands.get(cmd_name) {
            match cmd.execute(args, ctx).await? {
                Some(output) => println!("{output}"),
                None => {} // Command handled its own output
            }
            return Ok(DispatchResult::Handled);
        }

        // 2. Check if it's a skill name (user-invocable skills act as commands)
        let skills = ctx.skill_registry.list().await;
        if let Some(skill_summary) = skills.iter().find(|s| s.name == cmd_name && s.user_invocable) {
            // Invoke skill
            let skill = ctx.skill_registry.get(cmd_name).await?
                .ok_or_else(|| anyhow::anyhow!("skill '{cmd_name}' not found"))?;
            let args_vec: Vec<String> = if args.is_empty() {
                vec![]
            } else {
                shell_words::split(args)?
            };
            let rendered = SkillRegistry::<NativePlatform>::render_prompt(&skill, &args_vec);
            println!("[Invoking skill: {cmd_name}]");
            return Ok(DispatchResult::SkillInvocation(rendered));
        }

        // 3. Unknown command
        println!("Unknown command: /{cmd_name}. Type /help for available commands.");
        Ok(DispatchResult::Handled)
    }
}
```

### 3.4 Agent Selection

```rust
pub async fn apply_agent_definition(
    ctx: &mut AppContext,
    agent_def: &AgentDefinition,
) -> Result<()> {
    // Override model
    if let Some(ref model) = agent_def.model {
        ctx.config_mut().agents.defaults.model = model.clone();
    }

    // Override temperature
    if let Some(temp) = agent_def.temperature {
        ctx.config_mut().agents.defaults.temperature = temp;
    }

    // Override max tool iterations
    if let Some(max_iter) = agent_def.max_tool_iterations {
        ctx.config_mut().agents.defaults.max_tool_iterations = max_iter as i32;
    }

    // Filter tools if agent specifies a subset.
    // NOTE: Store the original ToolRegistry tool list before applying agent tool filters.
    // Restore when switching agents via `/agent default`. Do not mutate the registry
    // in place -- use a filtered view.
    if !agent_def.tools.is_empty() {
        let allowed: std::collections::HashSet<&str> =
            agent_def.tools.iter().map(String::as_str).collect();
        ctx.tools_mut().retain(|name| allowed.contains(name));
    }

    // Prepend agent system prompt
    if let Some(ref prompt) = agent_def.system_prompt {
        // This will be injected by the ContextBuilder
    }

    // Preload agent skills
    for skill_name in &agent_def.skills {
        if let Err(e) = ctx.skill_registry().get(skill_name).await {
            tracing::warn!(skill = %skill_name, error = %e, "agent skill not found");
        }
    }

    Ok(())
}
```

### 3.4.1 MCP Tool Interaction

MCP (Model Context Protocol) tools are registered with a namespaced naming convention:
`{server}__{tool}` (double underscore separator). For example, a GitHub MCP server
registered as `github` exposes tools like `github__create_repo`, `github__list_issues`, etc.

**Tool name matching with globs**:

Both `SkillDefinition.allowed_tools` and `AgentDefinition.tools` support prefix glob
matching for MCP tool namespaces. The `retain()` filter in `apply_agent_definition`
(and the analogous filter in skill activation) uses this logic:

```rust
/// Check if a tool name matches any entry in the allowed list.
/// Supports exact match and prefix glob (e.g., "github__*").
fn tool_matches(tool_name: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|pattern| {
        if let Some(prefix) = pattern.strip_suffix("__*") {
            // Prefix glob: match any tool from this MCP server
            tool_name.starts_with(&format!("{prefix}__"))
        } else if pattern.ends_with('*') {
            // Generic prefix glob (e.g., "read_*")
            tool_name.starts_with(pattern.trim_end_matches('*'))
        } else {
            // Exact match
            tool_name == pattern
        }
    })
}
```

**Example configurations**:

```yaml
# Skill: allow built-in file tools + all RVF MCP tools
allowed_tools:
  - read_file
  - web_fetch
  - rvf__*          # all tools from the "rvf" MCP server

# Agent: allow specific MCP tools only
tools:
  - read_file
  - write_file
  - github__create_repo     # exact MCP tool
  - github__list_issues     # exact MCP tool
  - slack__*                # all Slack MCP tools
```

**Security note**: When `allowed_tools` / `tools` is empty (the default), ALL registered
tools are available including every tool from every configured MCP server. See SEC-SKILL-08
for namespace isolation requirements.

### 3.5 Template Rendering

```rust
pub fn render_template(template: &str, args: &[String], session_id: &str) -> String {
    let mut result = template.to_string();

    // Replace $SESSION_ID
    result = result.replace("$SESSION_ID", session_id);
    result = result.replace("${SESSION_ID}", session_id);
    // Also support Claude Code's ${CLAUDE_SESSION_ID} alias
    result = result.replace("${CLAUDE_SESSION_ID}", session_id);

    // Replace $ARGUMENTS[N] and ${N} (specific indexed args)
    // Note: Use braced ${N} syntax only -- bare $N is NOT supported to avoid
    // collision with dollar amounts in prompts (e.g., "$100").
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("$ARGUMENTS[{i}]"), arg);
        result = result.replace(&format!("${{{i}}}"), arg);
    }

    // Replace $ARGUMENTS (all args joined)
    let all_args = args.join(" ");
    if result.contains("$ARGUMENTS") {
        result = result.replace("$ARGUMENTS", &all_args);
    } else if !all_args.is_empty() {
        // If $ARGUMENTS not in template, append
        result.push_str(&format!("\n\nARGUMENTS: {all_args}"));
    }

    result
}
```

---

## 4. Architecture

### 4.1 Module Dependency Graph

```
clawft-types/src/skill.rs
  SkillDefinition, SkillContext, SkillFormat, SkillScope
  AgentDefinition, AgentSummary, SkillSummary
      |
      v
clawft-core/src/agent/skills_v2.rs
  SkillRegistry, parse_skill_md(), render_prompt()
  (depends on: clawft-types, clawft-platform)
      |
clawft-core/src/agent/agents.rs
  AgentRegistry, AgentLoader
  (depends on: clawft-types, clawft-platform)
      |
clawft-core/src/agent/helpers.rs
  fs_helpers, config_helpers, template_helpers, git_helpers
  (depends on: clawft-types, clawft-platform)
      |
      v
clawft-core/src/agent/context.rs [EDIT]
  ContextBuilder: integrate SkillRegistry, AgentDefinition
      |
      v
clawft-cli/src/interactive/mod.rs
  SlashCommandRegistry, SlashCommand trait
  (depends on: clawft-core)
      |
clawft-cli/src/interactive/builtins.rs
  HelpCommand, SkillsCommand, AgentsCommand, ConfigCommand, etc.
      |
clawft-cli/src/commands/skills_cmd.rs
  weft skills {list,show,run}
      |
clawft-cli/src/commands/agents_cmd.rs
  weft agents {list,show}
      |
clawft-cli/src/commands/help_cmd.rs
  weft help <topic>
      |
clawft-cli/src/commands/agent.rs [EDIT]
  run_interactive(): integrate SlashCommandRegistry
```

### 4.2 Data Flow: Skill Invocation

```
User types: /research quantum computing
                |
                v
     SlashCommandRegistry::dispatch()
                |
     Check builtins -> not found
                |
     Check skill registry -> found "research" (user_invocable=true)
                |
                v
     SkillRegistry::get("research")
                |
                v
     SkillRegistry::render_prompt(skill, ["quantum", "computing"])
        - Replace $ARGUMENTS -> "quantum computing"
        - Replace ${0} -> "quantum"
        - Replace ${1} -> "computing"
                |
                v
     Rendered prompt sent to MessageBus as InboundMessage
                |
                v
     AgentLoop processes it with skill's allowed_tools active
                |
                v
     Response displayed to user
```

### 4.3 Data Flow: Agent Selection

```
User runs: weft agent --agent researcher
                |
                v
     AgentRegistry::get("researcher")
                |
                v
     AgentDefinition {
        model: "anthropic/claude-sonnet-4-20250514",
        tools: ["read_file", "web_search", "web_fetch"],
        skills: ["research", "summarize"],
        temperature: 0.3,
     }
                |
                v
     apply_agent_definition(ctx, &agent_def)
        - Override model in config
        - Filter tool registry
        - Mark skills for preloading
                |
                v
     ContextBuilder::build_messages()
        - System prompt includes agent's system_prompt
        - Preloaded skill prompts included
                |
                v
     AgentLoop runs with restricted tool set
```

### 4.4 Crate Changes Summary

| Crate | File | Change Type | Description |
|-------|------|-------------|-------------|
| clawft-types | `src/skill.rs` | NEW | Shared skill/agent metadata types |
| clawft-types | `src/lib.rs` | EDIT | Add `pub mod skill;` |
| clawft-core | `src/agent/skills_v2.rs` | NEW | SkillRegistry, SKILL.md parser |
| clawft-core | `src/agent/agents.rs` | NEW | AgentRegistry, AgentLoader |
| clawft-core | `src/agent/helpers.rs` | NEW | Utility helpers |
| clawft-core | `src/agent/mod.rs` | EDIT | Add new modules |
| clawft-core | `src/agent/context.rs` | EDIT | Integrate SkillRegistry + AgentDef |
| clawft-core | `Cargo.toml` | EDIT | Add `serde_yaml` dependency |
| clawft-cli | `src/interactive/mod.rs` | NEW | SlashCommandRegistry |
| clawft-cli | `src/interactive/registry.rs` | NEW | Registry implementation |
| clawft-cli | `src/interactive/builtins.rs` | NEW | Built-in slash commands |
| clawft-cli | `src/commands/skills_cmd.rs` | NEW | `weft skills` subcommand |
| clawft-cli | `src/commands/agents_cmd.rs` | NEW | `weft agents` subcommand |
| clawft-cli | `src/commands/help_cmd.rs` | NEW | `weft help` subcommand |
| clawft-cli | `src/commands/agent.rs` | EDIT | Integrate SlashCommandRegistry |
| clawft-cli | `src/commands/mod.rs` | EDIT | Add new command modules |
| clawft-cli | `src/main.rs` | EDIT | Add Skills, Agents, Help subcommands |

### 4.5 New Dependencies

| Dependency | Crate | Purpose |
|------------|-------|---------|
| `serde_yaml` (0.9) | clawft-core | Parse YAML frontmatter in SKILL.md |
| `shell-words` (1.1) | clawft-cli | Parse quoted arguments in skill invocation |

Both are lightweight and have no transitive bloat.

---

## 5. Refinement (TDD Test Plan)

### 5.1 SKILL.md Parser Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skill_md_full_frontmatter() {
        let content = r#"---
name: research
description: Deep research on a topic
version: 2.0.0
variables:
  - topic
  - depth
argument-hint: "[topic] [depth]"
disable-model-invocation: false
user-invocable: true
allowed-tools:
  - read_file
  - web_search
---

When researching $ARGUMENTS:

1. Start with a broad overview
2. Provide concrete examples
"#;
        let skill = parse_skill_md(content, "research").unwrap();
        assert_eq!(skill.name, "research");
        assert_eq!(skill.description, "Deep research on a topic");
        assert_eq!(skill.version, "2.0.0");
        assert_eq!(skill.variables, vec!["topic", "depth"]);
        assert_eq!(skill.argument_hint.as_deref(), Some("[topic] [depth]"));
        assert!(!skill.disable_model_invocation);
        assert!(skill.user_invocable);
        assert_eq!(skill.allowed_tools, vec!["read_file", "web_search"]);
        assert!(skill.prompt.unwrap().contains("When researching $ARGUMENTS"));
    }

    #[test]
    fn parse_skill_md_minimal_frontmatter() {
        let content = "---\ndescription: A simple skill\n---\n\nDo the thing.";
        let skill = parse_skill_md(content, "simple").unwrap();
        assert_eq!(skill.name, "simple"); // inferred from dir_name
        assert_eq!(skill.description, "A simple skill");
        assert_eq!(skill.prompt.as_deref(), Some("Do the thing."));
    }

    #[test]
    fn parse_skill_md_no_frontmatter() {
        let content = "Just instructions without any frontmatter.";
        let skill = parse_skill_md(content, "bare").unwrap();
        assert_eq!(skill.name, "bare");
        assert_eq!(skill.prompt.as_deref(), Some("Just instructions without any frontmatter."));
    }

    #[test]
    fn parse_skill_md_unterminated_frontmatter() {
        let content = "---\nname: broken\n\nNo closing delimiter.";
        let result = parse_skill_md(content, "broken");
        assert!(result.is_err());
    }

    #[test]
    fn parse_skill_md_empty_body() {
        let content = "---\nname: empty\ndescription: Empty body\n---\n";
        let skill = parse_skill_md(content, "empty").unwrap();
        assert!(skill.prompt.is_none());
    }

    #[test]
    fn parse_skill_md_openclaw_metadata() {
        let content = r#"---
name: api-skill
description: Call an API
metadata:
  openclaw:
    requires:
      bins: [curl]
      env: [API_KEY]
    primaryEnv: API_KEY
---

Use curl to call the API.
"#;
        let skill = parse_skill_md(content, "api-skill").unwrap();
        assert!(skill.metadata.is_some());
        let meta = skill.metadata.unwrap();
        assert!(meta.get("openclaw").is_some());
    }

    #[test]
    fn parse_skill_md_fork_context() {
        let content = "---\nname: forked\ncontext: fork\nagent: explore\n---\n\nRun in isolation.";
        let skill = parse_skill_md(content, "forked").unwrap();
        assert!(matches!(skill.context, SkillContext::Fork));
        assert_eq!(skill.agent.as_deref(), Some("explore"));
    }
}
```

### 5.2 Template Rendering Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template_arguments() {
        let result = render_template(
            "Research $ARGUMENTS thoroughly.",
            &["quantum computing".into()],
            "sess-1",
        );
        assert_eq!(result, "Research quantum computing thoroughly.");
    }

    #[test]
    fn render_template_indexed_args() {
        let result = render_template(
            "Migrate ${0} from ${1} to ${2}.",
            &["SearchBar".into(), "React".into(), "Vue".into()],
            "sess-1",
        );
        assert_eq!(result, "Migrate SearchBar from React to Vue.");
    }

    #[test]
    fn render_template_arguments_n_syntax() {
        let result = render_template(
            "Convert $ARGUMENTS[0] to $ARGUMENTS[1].",
            &["Celsius".into(), "Fahrenheit".into()],
            "sess-1",
        );
        assert_eq!(result, "Convert Celsius to Fahrenheit.");
    }

    #[test]
    fn render_template_session_id() {
        let result = render_template(
            "Log to ${SESSION_ID}.log",
            &[],
            "abc-123",
        );
        assert_eq!(result, "Log to abc-123.log");
    }

    #[test]
    fn render_template_claude_session_id_alias() {
        let result = render_template(
            "Session: ${CLAUDE_SESSION_ID}",
            &[],
            "xyz-789",
        );
        assert_eq!(result, "Session: xyz-789");
    }

    #[test]
    fn render_template_args_appended_when_no_placeholder() {
        let result = render_template(
            "Just instructions with no placeholders.",
            &["extra args".into()],
            "sess-1",
        );
        assert!(result.contains("ARGUMENTS: extra args"));
    }

    #[test]
    fn render_template_no_args_no_append() {
        let result = render_template(
            "No args here.",
            &[],
            "sess-1",
        );
        assert!(!result.contains("ARGUMENTS:"));
        assert_eq!(result, "No args here.");
    }
}
```

### 5.3 Skill Registry Tests

```rust
#[tokio::test]
async fn skill_registry_discovers_skill_md() {
    let dir = temp_dir("registry_md");
    create_skill_md(&dir, "my-skill", "---\nname: my-skill\ndescription: Test\n---\n\nPrompt.");
    let registry = test_registry(&dir);
    registry.discover().await.unwrap();
    let skills = registry.list().await;
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "my-skill");
    assert!(matches!(skills[0].format, SkillFormat::SkillMd));
}

#[tokio::test]
async fn skill_registry_discovers_legacy() {
    let dir = temp_dir("registry_legacy");
    create_legacy_skill(&dir, "old-skill", "Old skill desc", Some("prompt text"));
    let registry = test_registry(&dir);
    registry.discover().await.unwrap();
    let skills = registry.list().await;
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "old-skill");
    assert!(matches!(skills[0].format, SkillFormat::Legacy));
}

#[tokio::test]
async fn skill_registry_skill_md_takes_precedence_over_json() {
    let dir = temp_dir("registry_precedence");
    // Create both formats in same directory
    create_skill_md(&dir, "dual", "---\nname: dual\ndescription: From SKILL.md\n---\n\nMD prompt.");
    create_legacy_skill_in(&dir.join("dual"), "dual", "From skill.json", Some("JSON prompt."));
    let registry = test_registry(&dir);
    registry.discover().await.unwrap();
    let skill = registry.get("dual").await.unwrap().unwrap();
    assert!(matches!(skill.source_format, SkillFormat::SkillMd));
    assert_eq!(skill.description, "From SKILL.md");
}

#[tokio::test]
async fn skill_registry_higher_priority_wins() {
    let project_dir = temp_dir("project_skills");
    let user_dir = temp_dir("user_skills");
    create_skill_md(&project_dir, "shared", "---\nname: shared\ndescription: Project version\n---\n\nProject.");
    create_skill_md(&user_dir, "shared", "---\nname: shared\ndescription: User version\n---\n\nUser.");

    let registry = test_registry_multi(vec![
        SkillSearchPath { path: project_dir, scope: SkillScope::Project },
        SkillSearchPath { path: user_dir, scope: SkillScope::User },
    ]);
    registry.discover().await.unwrap();
    let skill = registry.get("shared").await.unwrap().unwrap();
    assert_eq!(skill.description, "Project version");
}

#[tokio::test]
async fn skill_registry_empty_when_no_dirs_exist() {
    let registry = test_registry(&PathBuf::from("/nonexistent/path"));
    registry.discover().await.unwrap();
    assert!(registry.list().await.is_empty());
}
```

### 5.4 Agent Registry Tests

```rust
#[tokio::test]
async fn agent_registry_discovers_yaml() {
    let dir = temp_dir("agents_yaml");
    let yaml = r#"
name: researcher
description: Research agent
model: anthropic/claude-sonnet-4-20250514
tools: [read_file, web_search]
skills: [research]
temperature: 0.3
"#;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("researcher.yaml"), yaml).await.unwrap();

    let registry = test_agent_registry(&dir);
    registry.discover().await.unwrap();
    let agents = registry.list().await;
    // Built-ins + 1 custom
    assert!(agents.iter().any(|a| a.name == "researcher"));
    let agent = registry.get("researcher").await.unwrap();
    assert_eq!(agent.model.as_deref(), Some("anthropic/claude-sonnet-4-20250514"));
    assert_eq!(agent.tools, vec!["read_file", "web_search"]);
}

#[tokio::test]
async fn agent_registry_discovers_agent_md() {
    let dir = temp_dir("agents_md");
    let content = r#"---
name: coder
description: Code-focused agent
model: openai/gpt-4o
tools: [read_file, write_file, edit_file, exec_shell]
---

You are a code specialist. Write clean, tested, documented code.
"#;
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("coder.md"), content).await.unwrap();

    let registry = test_agent_registry(&dir);
    registry.discover().await.unwrap();
    let agent = registry.get("coder").await.unwrap();
    assert_eq!(agent.system_prompt.as_deref(), Some("You are a code specialist. Write clean, tested, documented code."));
}

#[tokio::test]
async fn agent_registry_has_builtins() {
    let registry = test_agent_registry(&PathBuf::from("/nonexistent"));
    registry.discover().await.unwrap();
    let agents = registry.list().await;
    assert!(agents.iter().any(|a| a.name == "default"));
    assert!(agents.iter().any(|a| a.name == "explore"));
    assert!(agents.iter().any(|a| a.name == "coder"));
    assert!(agents.iter().any(|a| a.name == "planner"));
}

#[tokio::test]
async fn agent_registry_custom_overrides_builtin() {
    let dir = temp_dir("agents_override");
    let yaml = "name: explore\ndescription: Custom explorer\nmodel: custom/model\n";
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("explore.yaml"), yaml).await.unwrap();

    let registry = test_agent_registry(&dir);
    registry.discover().await.unwrap();
    let agent = registry.get("explore").await.unwrap();
    assert_eq!(agent.model.as_deref(), Some("custom/model"));
    assert_eq!(agent.description, "Custom explorer");
}
```

### 5.5 Slash Command Registry Tests

```rust
#[tokio::test]
async fn slash_dispatch_builtin_help() {
    let mut ctx = test_command_context().await;
    let registry = test_slash_registry();
    let result = registry.dispatch("/help", &mut ctx).await.unwrap();
    assert!(matches!(result, DispatchResult::Handled));
}

#[tokio::test]
async fn slash_dispatch_skills_list() {
    let mut ctx = test_command_context().await;
    let registry = test_slash_registry();
    let result = registry.dispatch("/skills", &mut ctx).await.unwrap();
    assert!(matches!(result, DispatchResult::Handled));
}

#[tokio::test]
async fn slash_dispatch_unknown_command() {
    let mut ctx = test_command_context().await;
    let registry = test_slash_registry();
    let result = registry.dispatch("/nonexistent", &mut ctx).await.unwrap();
    assert!(matches!(result, DispatchResult::Handled)); // still handled (prints error message)
}

#[tokio::test]
async fn slash_dispatch_not_a_command() {
    let mut ctx = test_command_context().await;
    let registry = test_slash_registry();
    let result = registry.dispatch("just a message", &mut ctx).await.unwrap();
    assert!(matches!(result, DispatchResult::NotACommand)); // not a slash command, pass to LLM
}

#[tokio::test]
async fn slash_dispatch_skill_as_command() {
    let mut ctx = test_command_context().await;
    // Register a skill named "research" with user_invocable=true
    ctx.skill_registry.register_test_skill("research", true).await;
    let registry = test_slash_registry();
    let result = registry.dispatch("/research quantum computing", &mut ctx).await.unwrap();
    assert!(matches!(result, DispatchResult::SkillInvocation(_)));
}

#[tokio::test]
async fn slash_dispatch_agent_switch() {
    let mut ctx = test_command_context().await;
    let registry = test_slash_registry();
    let result = registry.dispatch("/agent explore", &mut ctx).await.unwrap();
    assert!(matches!(result, DispatchResult::Handled));
}
```

### 5.6 Helper Tests

```rust
#[test]
fn config_get_by_path_agents_model() {
    let config = test_config();
    let val = config_helpers::get_by_path(&config, "agents.defaults.model");
    assert!(val.is_some());
}

#[test]
fn config_get_by_path_invalid() {
    let config = test_config();
    let val = config_helpers::get_by_path(&config, "nonexistent.path");
    assert!(val.is_none());
}

#[test]
fn resolve_workspace_path_relative() {
    let result = fs_helpers::resolve_workspace_path(
        Path::new("/home/user/.clawft/workspace"),
        "skills/research",
    );
    assert_eq!(result, PathBuf::from("/home/user/.clawft/workspace/skills/research"));
}

#[test]
fn template_extract_variables() {
    let vars = template_helpers::extract_variables("Research $ARGUMENTS[0] about ${1}");
    assert!(vars.contains(&"ARGUMENTS[0]".to_string()));
    assert!(vars.contains(&"1".to_string()));
}
```

### 5.7 Test Coverage Requirements

| Module | Unit Test Coverage | Critical Paths (100%) |
|--------|-------------------|-----------------------|
| `skills_v2.rs` (parser) | >= 90% | YAML parsing, frontmatter extraction, precedence |
| `skills_v2.rs` (registry) | >= 85% | Discovery, dedup, priority merge |
| `agents.rs` | >= 85% | YAML loading, builtin registration, override |
| `helpers.rs` | >= 90% | Template rendering, path resolution |
| `interactive/` | >= 80% | Dispatch, skill-as-command |
| `commands/skills_cmd.rs` | >= 80% | List, show, run |
| `commands/agents_cmd.rs` | >= 80% | List, show |

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation

- [ ] All unit tests passing (run `cargo test -p clawft-core -p clawft-cli -p clawft-types`)
- [ ] SKILL.md parser handles all Claude Code frontmatter fields
- [ ] SKILL.md parser handles OpenClaw metadata blocks
- [ ] Legacy `skill.json` + `prompt.md` still works
- [ ] Skill discovery chain respects priority order
- [ ] Agent YAML loading works
- [ ] Agent AGENT.md loading works
- [ ] Built-in agents registered correctly
- [ ] Template rendering handles all substitution patterns
- [ ] Slash command dispatch handles all registered commands
- [ ] Skills with `user_invocable: true` work as slash commands
- [ ] `--agent <name>` CLI flag works

### 6.2 CLI Integration

- [ ] `weft skills list` subcommand registered in `main.rs`
- [ ] `weft skills show <name>` subcommand registered
- [ ] `weft skills run <name> [args]` subcommand registered
- [ ] `weft agents list` subcommand registered
- [ ] `weft agents show <name>` subcommand registered
- [ ] `weft help <topic>` subcommand registered
- [ ] `weft agent --agent <name>` flag added to AgentArgs
- [ ] Interactive REPL integrated with SlashCommandRegistry
- [ ] New slash commands: `/skills`, `/skill`, `/agents`, `/agent`, `/memory`,
      `/config`, `/workspace`, `/model`, `/session`

### 6.3 Type System Integration

- [ ] `SkillDefinition` in `clawft-types/src/skill.rs`
- [ ] `AgentDefinition` in `clawft-types/src/skill.rs`
- [ ] `SkillScope`, `SkillFormat`, `SkillContext` enums in types crate
- [ ] All new types derive `Serialize, Deserialize, Clone, Debug`
- [ ] serde rename attributes for kebab-case YAML fields

### 6.4 Core Integration

- [ ] `SkillRegistry` accessible from `AppContext` (bootstrap)
- [ ] `AgentRegistry` accessible from `AppContext` (bootstrap)
- [ ] `ContextBuilder` uses `SkillRegistry` instead of raw `SkillsLoader`
      (or wraps it for backward compat)
- [ ] `AgentLoop` respects `AgentDefinition` tool filters
- [ ] `AgentLoop` respects `AgentDefinition` model override

### 6.5 Backward Compatibility

- [ ] `SkillsLoader` (existing) continues to compile and pass all existing tests
- [ ] Existing `skill.json` + `prompt.md` skills discovered by new registry
- [ ] `~/.clawft/workspace/skills/` still discovered (lowest priority, level 3)
- [ ] No changes to `Config` struct (agent definition is separate from config)

### 6.6 Dependency Audit

- [ ] `serde_yaml` added to `clawft-core/Cargo.toml` workspace deps
- [ ] `shell-words` added to `clawft-cli/Cargo.toml`
- [ ] Both dependencies are MIT-compatible
- [ ] No binary size regression > 200KB from new deps

### 6.7 Error Handling

- [ ] Invalid YAML frontmatter: clear error message with file path
- [ ] Missing skill: "Skill '<name>' not found. Run /skills to see available skills."
- [ ] Missing agent: "Agent '<name>' not found. Run /agents to see available agents."
- [ ] Invalid agent YAML: clear error message with file path
- [ ] Skill with unmet `requires.bins`: warning logged, skill still loadable
- [ ] Skill with unmet `requires.env`: warning logged, skill still loadable

### 6.8 Documentation (help topics)

- [ ] `weft help skills` topic content written
- [ ] `weft help agents` topic content written
- [ ] `weft help commands` topic content written
- [ ] `weft help config` topic content written
- [ ] `/help` output updated with new commands
- [ ] Update clawft/docs/guides/tool-calls.md with MCP namespace glob examples for allowed_tools

### 6.9 Build Verification

- [ ] `cargo build --release` succeeds
- [ ] `cargo clippy --workspace` clean (no warnings)
- [ ] `cargo test --workspace` passes
- [ ] Binary size checked (should not exceed 200KB growth)

### 6.10 Manual Testing

- [ ] Create a SKILL.md skill, verify it appears in `weft skills list`
- [ ] Create a `skill.json` skill, verify it appears alongside SKILL.md skills
- [ ] Create an `agent.yaml` file, verify `weft agents list` shows it
- [ ] Run `weft agent --agent <name>`, verify model and tools are correct
- [ ] In interactive mode, type `/skills` and verify output
- [ ] In interactive mode, type `/<skill-name> args` and verify invocation
- [ ] In interactive mode, type `/agent <name>` and verify switch
- [ ] In interactive mode, type `/help skills` and verify help text
- [ ] Verify project-level skills override user-level skills of same name

---

## Cross-Stream Integration Requirements

### Reuse Existing Infrastructure

- **Platform abstraction**: All filesystem access MUST go through `Platform` trait
  (no raw `std::fs` or `tokio::fs` in production code)
- **Config loading**: Use existing `load_config()` from `commands/mod.rs`
- **Test utilities**: Use `NativePlatform` for tests, create `temp_dir` helpers
  matching the pattern in `skills.rs` tests

### Integration with Phase 3G (Workspaces)

The `SkillRegistry` and `AgentRegistry` accept an optional `project_dir: Option<PathBuf>`
parameter. When Phase 3G implements workspace management, it will pass the active
workspace root to these registries. Until then, project detection uses the
`.clawft/` walk-up heuristic defined in FR-3F-060.

### Integration with ContextBuilder

The existing `ContextBuilder` takes `Arc<SkillsLoader<P>>`. The new `SkillRegistry`
wraps the legacy `SkillsLoader` functionality. Migration approach:

1. `ContextBuilder` gains an optional `Arc<SkillRegistry<P>>` field
2. If `SkillRegistry` is available, use it for skill lookup
3. If not, fall back to the existing `SkillsLoader` (backward compat)
4. Once all callers migrate, the `SkillsLoader` fallback can be removed

---

## Implementation Order

| Step | Description | Files | Tests First |
|------|-------------|-------|-------------|
| 1 | Shared types | `clawft-types/src/skill.rs` | Yes (serde roundtrip) |
| 2 | SKILL.md parser | `clawft-core/src/agent/skills_v2.rs` (parser only) | Yes (parse tests) |
| 3 | Template renderer | `clawft-core/src/agent/helpers.rs` (template_helpers) | Yes (render tests) |
| 4 | Skill registry | `clawft-core/src/agent/skills_v2.rs` (registry) | Yes (discovery tests) |
| 5 | Agent types + registry | `clawft-core/src/agent/agents.rs` | Yes (load + builtin tests) |
| 6 | Other helpers | `clawft-core/src/agent/helpers.rs` (fs, config, git) | Yes |
| 7 | Slash command framework | `clawft-cli/src/interactive/` | Yes (dispatch tests) |
| 8 | Built-in slash commands | `clawft-cli/src/interactive/builtins.rs` | Yes |
| 9 | CLI subcommands | `clawft-cli/src/commands/{skills,agents,help}_cmd.rs` | Yes |
| 10 | CLI main.rs + agent.rs | Integration | Integration tests |
| 11 | ContextBuilder integration | `clawft-core/src/agent/context.rs` | Integration tests |
| 12 | Help topic content | Static strings in help_cmd.rs | Manual review |

---

## Notes for Implementation Agent

1. **Read existing `skills.rs` first** to understand the current `Skill` struct and `SkillsLoader` API. The new `SkillRegistry` MUST NOT break existing functionality.
2. **Use TDD London School**: Write failing tests first, then implement. Every module above has explicit test cases.
3. **YAML frontmatter uses kebab-case**: Use `#[serde(rename = "disable-model-invocation")]` or `#[serde(rename_all = "kebab-case")]` on the frontmatter struct.
4. **Do not add `serde_yaml` to clawft-types**: Types crate must remain zero-dep (beyond serde). The YAML parsing lives in clawft-core.
5. **The `prompt` field is `#[serde(skip)]`**: Just like the existing `Skill` struct, the prompt is loaded from the file, not serialized. This is critical for the cache to work.
6. **Agent definitions are NOT config**: They are separate files in `~/.clawft/agents/`. Do not add them to `config.json`.
7. **Built-in agents are constants**: Define them as `const` or `static` in `agents.rs`, not loaded from disk.
8. **Slash commands are synchronous output**: They print to stdout and return. They do NOT go through the LLM pipeline (except `/skill` which sends a rendered prompt).
9. **Binary size**: `serde_yaml` adds ~100-150KB. Ensure it is acceptable. If too large, implement a minimal YAML frontmatter parser (~100 lines) instead.
10. **File ownership**: This stream owns `clawft-core/src/agent/{skills_v2,agents,helpers}.rs` and `clawft-cli/src/{interactive/,commands/{skills,agents,help}_cmd.rs}`. Do not modify files owned by other streams without coordination.

---

## Research Sources

- [Extend Claude with skills - Claude Code Docs](https://code.claude.com/docs/en/skills)
- [OpenClaw Skills Documentation](https://docs.openclaw.ai/tools/skills)
- [ClawHub - Public Skill Registry](https://github.com/openclaw/clawhub)
- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks)
- [Claude Code Settings - CLAUDE.md](https://code.claude.com/docs/en/settings)
- [Claude Code Skills vs Slash Commands 2026](https://yingtu.ai/blog/claude-code-skills-vs-slash-commands)
- [Inside Claude Code Skills: Structure, prompts, invocation](https://mikhail.io/2025/10/claude-code-skills/)
- [OpenClaw Custom Skill Creation Guide](https://zenvanriel.nl/ai-engineer-blog/openclaw-custom-skill-creation-guide/)
