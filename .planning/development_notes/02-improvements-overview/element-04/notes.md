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

### Remaining
- C2: wasmtime engine integration, fuel metering, memory limits (deferred)
- C7: PluginHost unification (deferred)
- C4a: Autonomous skill creation (stretch goal)
