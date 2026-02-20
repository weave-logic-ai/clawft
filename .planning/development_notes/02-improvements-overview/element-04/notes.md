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
