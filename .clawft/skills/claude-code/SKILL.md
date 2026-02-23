---
name: claude-code
description: Access Claude Code capabilities via MCP
version: 1.0.0
allowed-tools:
  - claude-code__*
user-invocable: true
argument-hint: Task description or command
---

# Claude Code MCP Skill

This skill provides access to Claude Code's capabilities via the `claude-code`
MCP server. It exposes a broad set of development tools for file operations,
code search, execution, and web access.

## Available Tool Categories

### File Operations
- **Read** -- Read file contents from the local filesystem
- **Write** -- Write or overwrite files
- **Edit** -- Perform exact string replacements in files

### Search
- **Glob** -- Fast file pattern matching (e.g. `**/*.rs`, `src/**/*.ts`)
- **Grep** -- Content search with regex support, powered by ripgrep

### Execution
- **Bash** -- Execute shell commands with optional timeout

### Web Access
- **WebFetch** -- Fetch and process web page content
- **WebSearch** -- Search the web for up-to-date information

### Notebooks
- **NotebookEdit** -- Edit Jupyter notebook cells (code and markdown)

## Usage Patterns

- **Research tasks**: Use WebSearch and WebFetch to gather information, then
  Read/Grep to cross-reference with local code.
- **Code generation**: Use Read to understand existing code, then Write/Edit
  to produce new or modified files.
- **File manipulation**: Use Glob to discover files, Read to inspect them,
  and Edit/Write to make changes.
- **Exploration**: Use Bash for git operations, build commands, and other
  CLI tasks. Use Grep for codebase-wide content searches.

## Configuration Note

The `claude-code` MCP server is configured as `internal_only`, meaning it is
not exposed to external consumers. Tools are namespaced as `claude-code__<tool>`
(e.g. `claude-code__Read`, `claude-code__Bash`).
