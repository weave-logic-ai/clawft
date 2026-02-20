# Development Notes: Element 07 - Dev Tools & Apps

**Workstream**: F
**Weeks**: 5-10
**Status**: F-Core complete (4/10 items), F-Advanced/F-MCP pending
**Completed**: 2026-02-20 (F-Core)
**Agent**: Agent-07 (a73994c)

---

## Implementation Log

### F1: Git Tool Plugin -- DONE
- New crate: `crates/clawft-plugin-git/`
- 7 tools implementing `Tool` trait: GitStatusTool, GitDiffTool, GitCommitTool, GitBranchTool, GitLogTool, GitBlameTool, GitCloneTool
- `all_git_tools()` factory for batch registration
- Types: GitConfig, GitStatusResult, FileStatus, GitDiffResult, DiffPatch, GitLogEntry, GitBlameLine
- Operations: Pure functions using `git2` crate
- 15 tests

### F2: Cargo/Build Integration -- DONE
- New crate: `crates/clawft-plugin-cargo/`
- `CargoTool` struct implementing `Tool` trait for 5 subcommands (build, test, clippy, check, publish)
- `CargoConfig`, `CargoResult`, `CargoSubcommand`, `CargoFlags` with input validation
- Subprocess execution with 5-minute timeout and 1MB output truncation
- 18 tests

### F6: OAuth2 Helper -- DONE (P0, unblocks E5a)
- New crate: `crates/clawft-plugin-oauth2/`
- 4 tools: OAuth2AuthorizeTool, OAuth2CallbackTool, OAuth2RefreshTool, RestRequestTool
- `OAuth2ProviderConfig`, `StoredTokens`, `AuthorizationState`, `ProviderPreset`
- TokenStore with 0600 file permissions, atomic writes
- PKCE support, state parameter CSRF protection
- 19 tests

### F9a: Core MCP Client -- DONE (pre-existing)
- Already implemented in `clawft-services/src/mcp/mod.rs`
- McpClient (connect, list_tools, call_tool, send_raw)
- McpSession (initialize handshake, initialized notification, capabilities)
- No additional code needed

### Remaining
- F3 (tree-sitter), F4 (browser CDP), F5 (calendar), F7 (Docker) -- F-Advanced
- F8 (MCP IDE), F9b (full MCP client) -- F-MCP
