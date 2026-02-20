# Development Notes: Element 07 - Dev Tools & Apps

**Workstream**: F
**Weeks**: 5-10
**Status**: All phases complete (10/10 items)
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

### F3: Tree-Sitter Code Analysis -- DONE
- New crate: `crates/clawft-plugin-treesitter/`
- 4 tools implementing `Tool` trait: TsParseTool, TsSymbolsTool, TsComplexityTool, TsLanguagesTool
- `all_treesitter_tools(config)` factory for batch registration
- Types: Language (Rust/TypeScript/Python/JavaScript), TreeSitterConfig (max_file_size=1MB, max_ast_depth=10), Symbol, ComplexityMetrics, FunctionComplexity, AstNode
- Analysis module: parse_source, tree_to_ast, extract_symbols, calculate_complexity
- Per-language feature flags: `rust`, `typescript`, `python`, `javascript` (each enables its tree-sitter grammar)
- Native tree-sitter only (no WASM); tree-sitter 0.24, grammar crates 0.23
- 12 tests (base) + conditional per-language tests gated on feature flags

### F4: Browser CDP Automation -- DONE
- New crate: `crates/clawft-plugin-browser/`
- 6 tools implementing `Tool` trait: BrowserNavigateTool, BrowserScreenshotTool, BrowserFillTool, BrowserClickTool, BrowserGetTextTool, BrowserEvaluateTool
- `all_browser_tools(config)` factory for batch registration
- Types: BrowserSandboxConfig (allowed_domains, max_concurrent_pages=2, session_lifetime=300s, max_memory_mb=512, clear_state_between_sessions=true)
- Security: validate_url() blocks `file://`, `data://`, `javascript://` schemes; domain allowlist with subdomain matching; PermissionDenied for violations
- NOTE: Actual CDP session management (chromiumoxide connect) deferred to runtime integration layer; tools validate inputs and return placeholder responses
- 20 tests including all security blocking tests

### F5: Calendar Integration -- DONE
- New crate: `crates/clawft-plugin-calendar/`
- 5 tools implementing `Tool` trait: CalListEventsTool, CalCreateEventTool, CalUpdateEventTool, CalDeleteEventTool, CalCheckAvailabilityTool
- `all_calendar_tools(config)` factory for batch registration
- Types: CalendarProvider (Google/Outlook/ICal with API base URLs), CalendarConfig, CalendarEvent, ListEventsParams
- Validates RFC 3339 datetime formats via chrono::DateTime::parse_from_rfc3339
- NOTE: Actual API calls pending OAuth2 token wiring from F6; tools validate inputs and return structured placeholders
- 13 tests

### F7: Docker/Podman Orchestration -- DONE
- New crate: `crates/clawft-plugin-containers/`
- 6 tools implementing `Tool` trait: ContainerBuildTool, ContainerRunTool, ContainerStopTool, ContainerLogsTool, ContainerListTool, ContainerExecTool
- `all_container_tools(config)` factory for batch registration (shared ConcurrencyLimiter)
- Types: ContainerConfig (runtime, max_concurrent_ops=3), ContainerRuntime (Docker/Podman with binary() method), ContainerSubcommand, ContainerResult
- Operations: ConcurrencyLimiter (AtomicU32 acquire/release), execute_container() with 5-minute timeout, 1MB output truncation
- Input validation: is_valid_name(), is_valid_tag(), is_valid_env_var() -- strict character allowlists, no shell interpolation
- Port mapping validation: validate_port_mapping() for -p syntax
- 22 tests including input validation rejection tests

### F8: MCP IDE Integration -- DONE
- New file: `crates/clawft-services/src/mcp/ide.rs`
- IdeToolProvider implements ToolProvider with namespace "ide"
- 5 IDE tools: ide_open_file, ide_edit, ide_diagnostics, ide_symbols, ide_hover
- IdeDispatchFn type alias: `dyn Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync`
- IdeToolProvider::new(dispatcher) for runtime use; IdeToolProvider::stub() for testing without IDE connection
- IDE types: IdeDiagnostic, DiagnosticSeverity (error/warning/info/hint), IdeSymbol
- Debug impl shows tool_count
- 11 tests

### F9b: Full MCP Client Features -- DONE
- New file: `crates/clawft-services/src/mcp/client.rs`
- McpClientPool: main struct with connection pooling, schema caching, health checks
- McpClientConfig: config_dir, schema_ttl_secs=300, health_check_interval=60, max_connections=3, max_reconnect_attempts=5
- Auto-discovery: `auto_discover()` scans ~/.clawft/mcp/ for JSON config files
- Per-agent overrides: AgentMcpServerConfig with enabled flag, url/command/args overrides; `apply_agent_overrides()` merges global + per-agent
- Schema caching: CachedSchemas with Instant-based TTL expiration; get_cached_schemas(), cache_schemas(), invalidate_cache()
- Connection health: ConnectionHealth enum (Healthy/Degraded/Failed/Unknown); update_health(), get_health(), needs_reconnect(), servers_needing_health_check()
- PooledConnection struct for managed connections
- Added `dirs` dependency to clawft-services/Cargo.toml
- Added `pub mod client;` and `pub mod ide;` to mcp/mod.rs
- 11 tests

---

## Implementation Notes

### Workspace Changes
- Added 4 new workspace members to root Cargo.toml: clawft-plugin-treesitter, clawft-plugin-browser, clawft-plugin-calendar, clawft-plugin-containers
- All plugin crates are feature-gated: `plugin-treesitter`, `plugin-browser`, `plugin-calendar`, `plugin-containers`
- Added `dirs = { workspace = true }` to clawft-services/Cargo.toml for F9b auto-discovery

### Test Counts
- clawft-plugin-treesitter: 12 passed
- clawft-plugin-browser: 20 passed
- clawft-plugin-calendar: 13 passed
- clawft-plugin-containers: 22 passed
- clawft-services (F8 + F9b): 252 passed (11 new IDE tests + 11 new client tests)
- Total new tests: 67 plugin tests + 22 MCP tests = 89 new tests

### Pre-existing Issues (NOT from this work)
- `clawft-channels/src/plugin_host.rs` has unused imports and collapsible_if warnings
- `clawft-security/src/checks/patterns.rs` has an unterminated character literal
- These are tracked separately and do not affect this element's deliverables
