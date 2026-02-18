# Workspaces Guide

A workspace is a project-level container that scopes configuration, sessions,
memory, skills, agents, and hooks to a single directory tree. Each workspace
has its own `.clawft/` directory and optional `CLAWFT.md` instructions file.

## Directory Structure

When you create a workspace, clawft scaffolds the following layout:

```
my-project/
  CLAWFT.md                   # Project-level instructions (optional)
  .clawft/
    config.json               # Workspace config overrides
    MEMORY.md                 # Persistent workspace memory
    HISTORY.md                # Workspace history log
    sessions/                 # Session state files
    memory/                   # Vector memory storage
    skills/                   # Custom skill definitions
    agents/                   # Agent configurations
    hooks/                    # Lifecycle hooks
```

All resources inside `.clawft/` are scoped to this workspace. Sessions created
here do not appear in other workspaces, and memory stored here is isolated from
the global store.

## Workspace Discovery

When clawft starts, it resolves the active workspace through a three-step
discovery chain:

| Priority | Source | Notes |
|----------|--------|-------|
| 1 | `$CLAWFT_WORKSPACE` environment variable | Must point to a directory containing `.clawft/`. |
| 2 | Walk from current directory upward | Stops at the first parent with a `.clawft/` directory. |
| 3 | `~/.clawft/` | Global fallback (always exists after first run). |

The upward walk is the most common path. If you `cd` into any subdirectory of
a workspace, clawft automatically discovers the workspace root above you.

```sh
# Force a specific workspace regardless of cwd
export CLAWFT_WORKSPACE=/home/user/projects/my-api
weft workspace status
```

## CLI Commands

All workspace commands live under `weft workspace`.

### Create a workspace

```sh
weft workspace create my-project            # creates ./my-project/.clawft/
weft workspace create my-project --dir /opt # creates /opt/my-project/.clawft/
```

This creates the `.clawft/` directory tree, an empty `config.json`, starter
`CLAWFT.md`, and registers the workspace in the global registry at
`~/.clawft/workspaces.json`.

### List workspaces

```sh
weft workspace list         # registered workspaces with valid paths
weft workspace list --all   # include entries whose paths no longer exist
```

Output is a table with name, path, status, and last-accessed timestamp.

### Load a workspace

```sh
weft workspace load my-project         # by registered name
weft workspace load /path/to/project   # by filesystem path
```

Loading a workspace prints its scoped resource paths (sessions, memory, skills).

### Check status

```sh
weft workspace status
```

Shows the discovered workspace name, path, session count, whether `config.json`
and `CLAWFT.md` exist, and scoped resource directories.

### Delete a workspace

```sh
weft workspace delete my-project       # prompts for confirmation
weft workspace delete my-project -y    # skip confirmation
```

Deletion removes the entry from the global registry only. Files on disk are
not touched.

### Workspace configuration

Per-workspace config overrides live in `.clawft/config.json`. Use dot-notation
keys to read and write individual values:

```sh
weft workspace config set agents.defaults.model openai/gpt-4o
weft workspace config set agents.defaults.max_tokens 4096
weft workspace config get agents.defaults.model
weft workspace config reset   # resets to empty {}
```

Values are auto-parsed: `42` becomes a number, `true`/`false` become booleans,
`null` removes the key, and everything else is stored as a string.

## Config Merging

Configuration is resolved through a three-level merge:

```
compiled defaults  <  ~/.clawft/config.json (global)  <  .clawft/config.json (workspace)
```

Each layer overrides the one before it. The merge follows these rules:

| Type | Behavior |
|------|----------|
| Objects | Recursively merged (workspace keys override global keys). |
| Arrays | Replaced entirely (not concatenated). |
| Scalars | Right side wins. |
| `null` | Removes the key from the base config. |

Both `snake_case` and `camelCase` keys are accepted. Keys are normalized to
`snake_case` before merging, so `maxTokens` and `max_tokens` refer to the same
field.

### Example: removing a global MCP server in a workspace

If your global config defines a `slack` MCP server but a particular project
does not need it:

```json
// .clawft/config.json
{
  "tools": {
    "mcp_servers": {
      "slack": null,
      "project-db": { "command": "npx", "args": ["-y", "project-db-mcp"] }
    }
  }
}
```

The `null` removes `slack` from the merged result, while `project-db` is added.
All other global MCP servers are preserved.

## CLAWFT.md

`CLAWFT.md` is a Markdown file at the workspace root that provides project-level
instructions to the agent. It serves the same purpose as `.cursorrules` or
`CLAUDE.md` in other tools.

### Basic usage

```markdown
# My API Project

This is a REST API built with Actix-web and PostgreSQL.

## Rules

- Always use parameterized SQL queries.
- Follow the repository's existing error handling pattern.
- Run `cargo test` before suggesting changes are complete.
```

### Import syntax

Lines starting with `@` import another file inline:

```markdown
# My Project

@prompts/safety.md
@agents/researcher.md

## Project-specific instructions

...
```

Imports are resolved relative to the directory containing the CLAWFT.md file.
Imported files can themselves contain `@` imports, up to a maximum depth of 5.

### Hierarchical loading

clawft walks up from the current directory looking for `CLAWFT.md` files,
stopping at the nearest `.git` boundary. This lets you define org-wide
instructions at the repository root and project-specific instructions in
subdirectories:

```
repo/                     # .git lives here
  CLAWFT.md               # org-wide rules (loaded second)
  services/
    api/
      CLAWFT.md           # project-specific rules (loaded first, takes precedence)
```

Files are collected from most specific (closest to cwd) to most general
(highest ancestor before `.git`).

### Security constraints

- **Path traversal blocked**: Import paths containing `..` are rejected.
- **Absolute paths blocked**: Import paths like `/etc/passwd` are rejected.
- **Max depth 5**: Recursive imports are bounded to prevent circular imports.
- **Graceful failure**: If an imported file is missing, a comment is inlined
  and processing continues.

## Scoped Resources

Each workspace isolates the following resources inside `.clawft/`:

| Resource | Path | Description |
|----------|------|-------------|
| Sessions | `.clawft/sessions/` | Conversation state and history |
| Memory | `.clawft/memory/` | Vector memory and knowledge base |
| Skills | `.clawft/skills/` | Custom skill definitions |
| Agents | `.clawft/agents/` | Agent configurations |
| Hooks | `.clawft/hooks/` | Lifecycle hooks (pre-task, post-edit, etc.) |

These paths are resolved relative to the discovered workspace root, so
switching workspaces automatically switches all scoped resources.

## Examples

### Setting up a workspace for an existing project

```sh
cd ~/projects/my-api
weft workspace create my-api --dir ~/projects
cd my-api

# Customize the model for this project
weft workspace config set agents.defaults.model anthropic/claude-sonnet-4-20250514
weft workspace config set agents.defaults.max_tokens 4096

# Add project-specific MCP servers
weft workspace config set tools.mcp_servers.my-db.command npx
```

### Using CLAWFT.md with shared prompts

```
my-api/
  CLAWFT.md
  prompts/
    safety.md
    code-style.md
  .clawft/
    config.json
    ...
```

```markdown
# my-api

@prompts/safety.md
@prompts/code-style.md

## API-specific rules

- All endpoints must return JSON.
- Use 404 for missing resources, 422 for validation errors.
```

### Checking which workspace is active

```sh
weft workspace status
```

```
Workspace: my-api
  Path:       /home/user/projects/my-api
  Sessions:   3
  Has config: yes
  CLAWFT.md:  yes

Scoped resource paths:
  Sessions: /home/user/projects/my-api/.clawft/sessions
  Memory:   /home/user/projects/my-api/.clawft/memory
  Skills:   /home/user/projects/my-api/.clawft/skills
```
