# Development Notes: Element 04 - Plugin & Skill System

**Workstream**: C
**Weeks**: 3-8

---

## Implementation Log

### 2026-02-19: C1 Plugin Trait Crate -- COMPLETE
- Created `crates/clawft-plugin/` workspace crate
- 6 core traits: Tool, ChannelAdapter, PipelineStage, Skill, MemoryBackend, VoiceHandler
- 3 supporting traits: KeyValueStore, ToolContext, ChannelAdapterHost
- PluginManifest with JSON deserialization, PluginPermissions, PluginResourceConfig
- MessagePayload enum (Text/Structured/Binary) for voice/UI forward-compat
- PluginError enum with thiserror
- `voice` feature flag reserved as no-op
- All tests pass, clippy clean
- Branch: sprint/phase-5

### 2026-02-20: C2 WASM Plugin Host -- IN PROGRESS (60%)
- WIT interface `clawft:plugin@0.1.0` defined in `crates/clawft-wasm/wit/plugin.wit`
  - 5 host imports: http-request, read-file, write-file, get-env, log
  - 3 plugin exports: init, execute-tool, describe
- SandboxedFileSystem implemented (replacing stubs in fs.rs)
  - 8 filesystem operations with path canonicalization + symlink escape detection
  - Read 8MB / Write 4MB size limits
  - 14 new tests
- SandboxedHttpClient implemented (http.rs)
  - Network allowlist (exact + wildcard), SSRF protection, rate limiting
  - 6 new tests
- PluginSandbox runtime security context (sandbox.rs, new file)
  - NetworkAllowlist parser, RateCounter, env var filtering
  - 5 clippy fixes applied
- 116 tests passing with wasm-plugins feature
- 30/45 security tests complete
- Remaining: wasmtime engine integration, fuel metering, memory limits, audit logging, 15 security tests

### 2026-02-20: C3 Skill Loader -- COMPLETE
- `skills_v2.rs` rewritten with serde_yaml replacing hand-rolled YAML parser (+261 lines)
- Local skill discovery: workspace, ~/.clawft/skills, bundled directories
- WASM auto-registration from SKILL.md
- 40 skill tests passing

### 2026-02-20: C4 Hot-Reload & Dynamic Loading -- COMPLETE
- `skill_watcher.rs` created with `notify` crate for file-system watching
- Skill precedence: workspace > managed/local > bundled
- Atomic swap with drain for in-flight calls
- 10 watcher tests passing

### 2026-02-20: C5 Slash-Command Framework -- COMPLETE
- Interactive slash-command framework wired in `clawft-cli/src/interactive/`
- Skills contribute commands to `/help` output
- Command name collision detection
- `builtins.rs` +146 lines, `registry.rs` +14 lines
- 44 interactive tests passing

### 2026-02-20: C6 MCP Skill Exposure -- COMPLETE
- `SkillToolProvider` in `clawft-services/src/mcp/provider.rs`
- MCP `tools/list` includes loaded skill tools with JSON Schema
- MCP `tools/call` routes through skill's `execute_tool()`
- Hot-reload updates MCP tool listing via RwLock-based refresh
- 25 new MCP tests (22 SkillToolProvider + 3 integration)
- 161 total MCP tests passing

### 2026-02-20: C2 WASM Plugin Host -- COMPLETE (95%)
- Wasmtime 29 engine integration (feature-gated behind `wasm-plugins`)
  - `WasmPluginEngine` struct with configurable `wasmtime::Config` (fuel enabled)
  - `HostState` struct with `HostFunctionDispatcher` + `StoreLimits`
  - `create_store()` with fuel metering (default 1B units) and memory limits (default 16MB)
  - `create_linker()` with 5 host function imports: http-request, read-file, write-file, get-env, log
  - `execute_tool()` with fresh store per invocation (fuel resets between calls)
  - `read_str()` helper for reading strings from WASM linear memory
- 10 new security tests added (T28-T29, T31-T32, T37-T40, T43, T45):
  - T28: Fuel exhaustion via infinite loop WAT module
  - T29: Memory limit enforcement via StoreLimits
  - T31: Custom fuel budget configuration
  - T32: Custom memory limit configuration
  - T37: Audit logging for all 5 host functions
  - T38: Audit logging for denied operations
  - T39: Permission escalation prevention
  - T40: Multi-plugin isolation
  - T43: Wasmtime store isolation between plugins
  - T45: Fuel resets between invocations
- 158 total tests passing in clawft-wasm (up from 144)
- Clippy clean
- Remaining: T30 (30s wall-clock timeout), T41 (version upgrade re-prompt), T42 (lifecycle)
  - HTTP execution still returns validation-passed stub (reqwest integration not wired)

### 2026-02-20: C7 PluginHost Unification -- COMPLETE
- Created `crates/clawft-channels/src/plugin_host.rs` (618 lines)
- **ChannelAdapterShim**: bridges existing `Channel` trait to `ChannelAdapter` trait
  - Converts `OutboundMessage` to `MessagePayload` and back
  - Binary payload returns `PluginError::NotImplemented`
  - Status proxy, capabilities proxy (supports_threads, supports_media)
- **SoulConfig**: loads and injects SOUL.md personality content
  - Search order: workspace/.clawft/SOUL.md -> workspace/SOUL.md -> ~/.clawft/SOUL.md
  - `inject_into_prompt()` appends personality section to system prompt
  - `is_stale()` detects file modifications for hot-reload support
- **ChannelAdapterHostBridge**: bridges `ChannelAdapterHost` to `ChannelHost`
  - Converts `MessagePayload` variants to `InboundMessage` content strings
  - Binary payload handled gracefully with warning log + placeholder text
- start_all()/stop_all() already concurrent (pre-existing via `futures_util::future::join_all`)
- 172 tests passing in clawft-channels (all existing + 14 new plugin_host tests)
- Added `dirs` dependency for SoulConfig home directory resolution

### 2026-02-20: C4a Autonomous Skill Creation -- COMPLETE (stretch goal)
- Created `crates/clawft-core/src/agent/skill_autogen.rs` (631 lines)
- **AutogenConfig**: disabled by default, threshold=3, max_pending=10
- **PatternDetector**: sliding window pattern detection
  - Records tool calls into bounded window (max 30 entries)
  - Extracts all subsequences of length 2..=10
  - Fires when count >= threshold, prevents duplicate reporting
  - Configurable threshold, reset support
- **generate_skill_md()**: creates SKILL.md frontmatter from detected pattern
  - autogenerated: true, user-invocable: false
  - Minimal permissions (no shell, no network)
- **install_pending_skill()**: writes SKILL.md + .pending marker file
- **approve_skill()**: removes .pending marker
- **reject_skill()**: removes entire skill directory
- **is_pending()**: checks for .pending marker
- 19 tests passing, all patterns verified
- Clippy clean

### 2026-02-20: C2 WASM Security Tests T30 + T42 -- COMPLETE (C2 at 98%)
- **T30: Wall-clock timeout enforcement** (`t30_wall_clock_timeout`)
  - Enabled `epoch_interruption(true)` on wasmtime `Config` in `WasmPluginEngine::new()`
  - `create_store()` now calls `store.set_epoch_deadline(1)` for epoch-based timeout
  - `execute_tool()` spawns a background thread that calls `engine.increment_epoch()`
    after `config.timeout_secs` has elapsed, causing running WASM to trap
  - Added `call_func_with_timeout()` public method for direct typed function calls
    with wall-clock timeout (used by tests and advanced callers)
  - Test uses a WAT infinite loop with 10B fuel budget (would never exhaust via fuel alone)
    and a 100ms timeout; verifies the trap fires via epoch interruption, not fuel exhaustion
  - Verifies elapsed time is bounded (< 600ms) and significant fuel remains
- **T42: Complete audit logging verification** (`t42_complete_audit_logging_verification`)
  - Exercises all 5 host functions (http-request, read-file, write-file, get-env, log)
    through the HostFunctionDispatcher with full permissions
  - Verifies every call produced an audit entry (5 total)
  - Verifies each entry has the correct operation type (function name)
  - Verifies each entry has the correct params_summary containing the target
  - Verifies all 5 entries are permitted (not denied)
  - Verifies timestamps are monotonically increasing
  - Additionally tests 3 denied operations (unauthorized domain, file outside sandbox,
    unpermitted env var) and verifies they produce denied audit entries with error messages
  - Total: 8 audit entries verified (5 permitted + 3 denied)
- 160 tests passing in clawft-wasm with wasm-plugins feature (up from 158)
- 41 tests passing with default features (no regression)
- Clippy clean with `-D warnings`
- engine.rs is now ~1700 lines (implementation ~700 + tests ~1000)

### 2026-02-20: C2 T41 Version Upgrade Permission Re-Prompt -- COMPLETE (C2 at 100%)
- **PermissionDiff** struct added to `clawft-plugin/src/manifest.rs`
  - Fields: `new_network`, `new_filesystem`, `new_env_vars`, `shell_escalation`
  - `is_empty()` method for checking if re-prompt is needed
  - `PluginPermissions::diff(approved, requested)` computes delta
  - `PartialEq` derive added to `PluginPermissions` for test assertions
  - `PermissionDiff` re-exported from `clawft-plugin` crate root
  - 11 unit tests covering: identical perms, new network/fs/env, shell escalation,
    shell downgrade, empty approved, removed perms, wildcard detection
- **PermissionStore** added as `clawft-wasm/src/permission_store.rs` (new file)
  - `ApprovedRecord` struct (version, permissions, approved_at) with JSON serde
  - `PermissionStore::new(base_dir)` with `load()` and `save()` methods
  - JSON files at `{base_dir}/{plugin_id}/approved_permissions.json`
  - 3 unit tests: roundtrip, unknown plugin, overwrite on upgrade
- **PermissionApprover** trait + implementations
  - `PermissionApprover` trait with `approve(plugin_id, diff) -> bool`
  - `AutoApprover`: always returns true (non-interactive mode)
  - `MockApprover`: records calls, returns configured response (testing)
  - 2 unit tests for approver implementations
- **T41 integration test** (`t41_version_upgrade_permission_reprompt`)
  - Creates temp dir for PermissionStore
  - v1 manifest: `network: []`, `filesystem: ["/tmp"]`, `env_vars: ["HOME"]`, `shell: false`
  - Saves v1 approved record
  - v2 manifest: `network: ["api.example.com"]`, `filesystem: ["/tmp"]`, `env_vars: ["HOME", "API_KEY"]`
  - Computes diff: asserts `new_network: ["api.example.com"]`, `new_env_vars: ["API_KEY"]`,
    empty filesystem, no shell escalation
  - MockApprover verifies it receives exactly the correct diff
  - After approval, saves v2 record and verifies re-loading shows empty diff
- **Edge case tests**: shell escalation triggers re-prompt, identical perms = no prompt
- 73 tests passing in clawft-plugin (up from 62, +11 diff tests)
- 169 tests passing in clawft-wasm with wasm-plugins feature (up from 160, +9)
- Clippy clean with `-D warnings` on both crates

### Remaining
- HTTP execution wiring (reqwest integration for actual HTTP calls from WASM)
- engine.rs may benefit from splitting tests into a separate file
