# Development Assignment: Element 04 - Plugin & Skill System

> **Status**: Ready for development
> **Created**: 2026-02-19
> **Element**: 04 (SPARC)
> **Workstream**: C (Plugin & Skill System)
> **CRITICAL PATH**: All downstream feature work (Elements 06-10) depends on C1.

---

## Quick Reference

| Field | Value |
|-------|-------|
| **Branch** | `sprint/phase-5-5C` |
| **Crates touched** | `clawft-plugin` (new), `clawft-wasm`, `clawft-core`, `clawft-channels`, `clawft-cli`, `clawft-services` |
| **Timeline** | Weeks 3-8 |
| **Priority** | P0 (C1), P1 (C2, C3, C4, C6), P2 (C5, C7, C4a) |
| **Dependencies IN** | Element 03 B3 (file splits), A4 (SecretRef), A9 (feature gates) |
| **Blocks** | Elements 06, 07, 08, 09, 10 (all feature work uses plugin traits) |

---

## Internal Dependency Graph

```
C1 (trait crate) [Week 3-4, P0, NO DEPS]
  |
  +---> C2 (WASM host) [Week 4-5, P1]
  |       |
  +---> C3 (skill loader) [Week 5-6, P1]
  |       |     also depends on: 03-B3 (skills_v2.rs YAML parser replacement)
  |       |
  |       +---> C4 (hot-reload) [Week 6-7, P1] -- requires C2 + C3
  |       |       |
  |       |       +---> C4a (autonomous creation) [Week 8+, P2 stretch]
  |       |
  |       +---> C5 (slash-commands) [Week 7, P2]
  |       |
  |       +---> C6 (MCP exposure) [Week 7-8, P1]
  |
  +---> C7 (unified PluginHost) [Week 8, P2]
```

---

## Concurrent Work Units

### Unit 1: C1 - Plugin Trait Crate (Week 3-4, P0, NO DEPS)

**Goal**: Create the new `clawft-plugin` crate that defines ALL plugin traits. This is the foundation for the entire plugin system and unblocks all downstream work.

**Crate**: `crates/clawft-plugin/` (new)

**Cargo.toml dependencies** (minimal):
- `serde`, `serde_json` (manifest parsing)
- `async-trait` (async trait methods)
- `thiserror` (error types)

#### 1.1 Trait Definitions

Define the following six traits. All traits must be `Send + Sync` and use `async_trait` where async methods are needed.

**`Tool` trait**:
```rust
use async_trait::async_trait;
use serde_json::Value;

/// A tool that can be invoked by the agent loop.
///
/// Tools receive JSON parameters and return a JSON result.
/// They are exposed through MCP `tools/list` and `tools/call`.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (used in MCP `tools/call`).
    fn name(&self) -> &str;

    /// Human-readable description for LLM tool selection.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with the given JSON parameters.
    ///
    /// `ctx` provides access to the KeyValueStore and other host services.
    async fn execute(&self, params: Value, ctx: &dyn ToolContext) -> Result<Value, PluginError>;
}
```

**`ChannelAdapter` trait** (replaces the current `Channel` trait in `clawft-channels/src/traits.rs`):
```rust
/// A bidirectional connection to a chat platform.
///
/// This is the plugin-system version of the existing `Channel` trait
/// in `clawft-channels`. It adds support for binary payloads (for
/// voice/media forward-compat) via `MessagePayload`.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn metadata(&self) -> ChannelAdapterMetadata;
    fn status(&self) -> AdapterStatus;

    async fn start(
        &self,
        host: Arc<dyn AdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError>;

    async fn send(&self, payload: &MessagePayload) -> Result<String, PluginError>;
}

/// Payload types for channel messages.
///
/// Supports text, structured JSON, and binary (audio/media) payloads.
/// Binary variant provides forward-compat for voice (Workstream G).
pub enum MessagePayload {
    Text {
        channel: String,
        chat_id: String,
        content: String,
        reply_to: Option<String>,
        metadata: HashMap<String, Value>,
    },
    Structured {
        channel: String,
        chat_id: String,
        data: Value,
        metadata: HashMap<String, Value>,
    },
    Binary {
        channel: String,
        chat_id: String,
        mime_type: String,
        data: Vec<u8>,
        metadata: HashMap<String, Value>,
    },
}
```

**`PipelineStage` trait**:
```rust
/// A stage in the agent pipeline (Learner, Router, Assembler, Executor, Scorer).
///
/// Plugins can inject custom pipeline stages to modify message processing.
#[async_trait]
pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &str;
    fn stage_type(&self) -> PipelineStageType;
    async fn process(&self, input: PipelineInput) -> Result<PipelineOutput, PluginError>;
}

pub enum PipelineStageType {
    PreProcess,
    PostProcess,
    Transform,
    Filter,
}
```

**`Skill` trait**:
```rust
/// A skill that can be loaded from SKILL.md or registered programmatically.
///
/// Skills extend the agent's capabilities by providing instructions,
/// tool access control, and execution context.
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    fn variables(&self) -> &[String];
    fn allowed_tools(&self) -> &[String];
    fn instructions(&self) -> &str;
    fn is_user_invocable(&self) -> bool;

    /// Execute a tool provided by this skill.
    ///
    /// Only applicable for skills that provide their own tools.
    fn execute_tool(&self, _tool_name: &str, _params: Value) -> Result<Value, PluginError> {
        Err(PluginError::NotImplemented("skill does not provide tools".into()))
    }
}
```

**`MemoryBackend` trait**:
```rust
/// A pluggable memory storage backend.
///
/// Allows plugins to provide custom memory storage implementations
/// (vector stores, databases, cloud storage, etc.).
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    fn name(&self) -> &str;

    async fn store(&self, key: &str, value: &[u8], metadata: Option<Value>) -> Result<(), PluginError>;
    async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, PluginError>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>, PluginError>;
    async fn delete(&self, key: &str) -> Result<(), PluginError>;
}
```

**`VoiceHandler` trait** (PLACEHOLDER -- no implementation required):
```rust
/// Placeholder trait for voice processing plugins.
///
/// This trait is reserved for Workstream G (Voice). No implementations
/// are expected during this sprint. The trait definition ensures that
/// the plugin manifest schema reserves the `voice` capability type
/// and that the `ChannelAdapter` binary payload path is tested.
#[async_trait]
pub trait VoiceHandler: Send + Sync {
    fn name(&self) -> &str;
    async fn process_audio(&self, audio: &[u8], mime_type: &str) -> Result<Vec<u8>, PluginError>;
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>, PluginError>;
}
```

#### 1.2 KeyValueStore Trait (Cross-Element Contract)

From the cross-element integration spec (`01-cross-element-integration.md` Section 3.1):

```rust
/// Simple key-value store for plugin state persistence.
///
/// Tool plugins use `ToolContext::key_value_store()` for state.
/// Backed by the agent's workspace directory at runtime:
/// `~/.clawft/agents/<agentId>/tool_state/<plugin_name>/`
///
/// Plugin crates depend on `clawft-plugin` (for this trait) only,
/// never on `clawft-core` memory modules.
pub trait KeyValueStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, PluginError>;
    fn set(&self, key: &str, value: &[u8]) -> Result<(), PluginError>;
    fn delete(&self, key: &str) -> Result<(), PluginError>;
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>, PluginError>;
}
```

#### 1.3 ToolContext Trait

```rust
/// Host services available to tools during execution.
pub trait ToolContext: Send + Sync {
    fn key_value_store(&self) -> &dyn KeyValueStore;
    fn plugin_id(&self) -> &str;
    fn agent_id(&self) -> &str;
}
```

#### 1.4 Plugin Manifest Schema

Define `PluginManifest` and `PluginPermissions` structs for `clawft.plugin.json` / `clawft.plugin.yaml`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<PluginCapability>,
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub resources: PluginResourceConfig,
    pub wasm_module: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,    // Skill directories shipped with plugin
    #[serde(default)]
    pub tools: Vec<String>,     // Tool names provided by plugin
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Tool,
    Channel,
    PipelineStage,
    Skill,
    MemoryBackend,
    Voice,  // Reserved, no implementation this sprint
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub filesystem: Vec<String>,
    #[serde(default)]
    pub env_vars: Vec<String>,
    #[serde(default)]
    pub shell: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginResourceConfig {
    pub max_fuel: Option<u64>,
    pub max_memory_mb: Option<usize>,
    pub max_http_requests_per_minute: Option<u64>,
    pub max_log_messages_per_minute: Option<u64>,
    pub max_execution_seconds: Option<u64>,
    pub max_table_elements: Option<u32>,
}
```

#### 1.5 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin load failed: {0}")]
    LoadFailed(String),
    #[error("plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

#### C1 Acceptance Criteria

- [ ] `cargo build -p clawft-plugin` compiles cleanly
- [ ] All six traits defined: `Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler`
- [ ] `KeyValueStore` trait defined with `get/set/delete/list_keys`
- [ ] `ToolContext` trait defined
- [ ] `PluginManifest` struct parses JSON and YAML
- [ ] `PluginPermissions` struct with `network`, `filesystem`, `env_vars`, `shell`
- [ ] `PluginResourceConfig` struct with all resource limit fields
- [ ] `PluginCapability` enum includes `Voice` as reserved variant
- [ ] `MessagePayload::Binary` variant exists for voice forward-compat
- [ ] `PluginError` enum covers all error cases
- [ ] Feature flag `voice` wired in `Cargo.toml` as empty no-op
- [ ] Unit tests for manifest parsing (valid + invalid)
- [ ] Unit tests for `PluginCapability` serde round-trip
- [ ] `cargo clippy -p clawft-plugin -- -D warnings` clean

#### C1 Test Requirements

- `test_manifest_parse_json` -- Parse valid `clawft.plugin.json`
- `test_manifest_parse_yaml` -- Parse valid `clawft.plugin.yaml`
- `test_manifest_missing_id_fails` -- Reject manifest without `id`
- `test_manifest_invalid_version_fails` -- Reject non-semver `version`
- `test_manifest_empty_capabilities_fails` -- Reject empty `capabilities`
- `test_plugin_capability_serde_roundtrip` -- All capability variants round-trip
- `test_permissions_default_is_empty` -- Default `PluginPermissions` denies all
- `test_resource_config_defaults` -- Verify default resource limits
- `test_message_payload_binary_variant` -- Construct and match `Binary` variant
- `test_voice_feature_flag_compiles` -- Build with `--features voice` succeeds

---

### Unit 2: C2 - WASM Plugin Host (Week 4-5, P1, DEPENDS ON C1)

**Goal**: Implement the WASM plugin host using `wasmtime` + `wit` component model, completing the stub implementations in `clawft-wasm` and adding the security sandbox.

**Crates**: `clawft-wasm`, `clawft-core`

#### 2.1 Current State of `clawft-wasm`

The `clawft-wasm` crate currently contains **all stubs**. Here is the current code that must be replaced:

**`crates/clawft-wasm/src/fs.rs`** -- All 8 methods return `Unsupported`:
```rust
// CURRENT (stub) -- every method returns this:
pub fn read_to_string(&self, _path: &Path) -> std::io::Result<String> {
    Err(unsupported("read_to_string"))
}
pub fn write_string(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
    Err(unsupported("write_string"))
}
pub fn exists(&self, _path: &Path) -> bool {
    false
}
// ... all stubs
```

**`crates/clawft-wasm/src/http.rs`** -- Request method returns error:
```rust
// CURRENT (stub):
pub fn request(
    &self,
    _method: &str,
    _url: &str,
    _headers: &HashMap<String, String>,
    _body: Option<&[u8]>,
) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
    Err(
        "WASI HTTP not yet implemented: waiting for wasi:http/outgoing-handler stabilisation"
            .into(),
    )
}
```

**`crates/clawft-wasm/src/env.rs`** -- In-memory `WasiEnvironment` (this is functional but standalone, not implementing the `Platform::Environment` trait):
```rust
// CURRENT: Standalone struct with matching signatures but no trait impl
pub struct WasiEnvironment {
    vars: Mutex<HashMap<String, String>>,
}
impl WasiEnvironment {
    pub fn get_var(&self, name: &str) -> Option<String> { ... }
    pub fn set_var(&self, name: &str, value: &str) { ... }
    pub fn remove_var(&self, name: &str) { ... }
}
```

**`crates/clawft-wasm/src/lib.rs`** -- `init()` and `process_message()` are stubs:
```rust
// CURRENT (stubs):
pub fn init() -> i32 {
    // Phase 3A Week 11: Will load config from WASI filesystem
    0
}
pub fn process_message(input: &str) -> String {
    format!("clawft-wasm v{}: received '{}' (pipeline not yet wired)", VERSION, input)
}
```

#### 2.2 Implementation Tasks

1. **Add `wasmtime` dependency** to `clawft-wasm/Cargo.toml` and `clawft-core/Cargo.toml` (behind `wasm-plugins` feature flag)

2. **Define WIT interface** (`clawft:plugin@0.1.0`) with five host functions:
   ```wit
   package clawft:plugin@0.1.0;

   interface host {
       http-request: func(method: string, url: string, headers: list<tuple<string, string>>, body: option<string>) -> result<string, string>;
       read-file: func(path: string) -> result<string, string>;
       write-file: func(path: string, content: string) -> result<_, string>;
       get-env: func(name: string) -> option<string>;
       log: func(level: u8, message: string);
   }

   interface plugin {
       execute-tool: func(name: string, params: string) -> result<string, string>;
   }
   ```

3. **Implement `PluginSandbox`** (from WASM security spec Section 4.1) with:
   - `PluginPermissions` enforcement
   - `NetworkAllowlist` for HTTP host validation
   - `RateCounter` for HTTP and log rate limiting
   - Canonicalized filesystem path validation
   - Env var allowlist + implicit deny patterns

4. **Implement host functions** with full security enforcement:
   - `validate_http_request()` -- scheme check, allowlist, SSRF (reuse A6 `is_private_ip()`), rate limit, body size (see security spec Section 4.2)
   - `validate_file_access()` -- canonicalize, sandbox containment, symlink traversal, size check (see security spec Section 4.3)
   - `validate_env_access()` -- allowlist check, implicit deny patterns, audit log (see security spec Section 4.4)
   - `log` -- rate limit, message size truncation, level mapping

5. **Complete `WasiFileSystem`** -- Replace stubs with real WASI filesystem ops scoped to pre-opened directories

6. **Implement WASM HTTP client** via `wasi:http/outgoing-handler` or `reqwest` bridge

7. **Wire `init()` and `process_message()`** in `clawft-wasm/src/lib.rs`

8. **Size enforcement** -- <300KB uncompressed, <120KB gzipped per plugin module

9. **Fuel metering** -- Configure `wasmtime::Config::consume_fuel(true)`, set per-invocation budget (default: 1B units)

10. **Memory limits** -- Configure `StoreLimitsBuilder` (default: 16MB, max 256MB)

#### 2.3 Resource Limits Table

| Resource | Default | Manifest Key | Hard Maximum |
|----------|---------|-------------|-------------|
| Fuel budget | 1,000,000,000 (~1s CPU) | `resources.max_fuel` | 10,000,000,000 |
| Memory | 16 MB | `resources.max_memory_mb` | 256 MB |
| Table elements | 10,000 | `resources.max_table_elements` | 100,000 |
| Plugin binary | <300 KB uncompressed | global config | -- |
| Plugin gzipped | <120 KB | global config | -- |
| HTTP requests | 10/minute | `resources.max_http_requests_per_minute` | -- |
| Log messages | 100/minute | `resources.max_log_messages_per_minute` | -- |
| Execution timeout | 30 seconds | `resources.max_execution_seconds` | -- |
| HTTP body size | 1 MB | -- | -- |
| HTTP response size | 4 MB | -- | -- |
| File read size | 8 MB | -- | -- |
| File write size | 4 MB | -- | -- |
| Log message size | 4 KB | -- | -- |

#### C2 Acceptance Criteria

- [ ] `wasmtime` integrated with `wit` component model
- [ ] All 5 WIT host functions implemented with security enforcement
- [ ] `WasiFileSystem` fully functional (all stubs replaced)
- [ ] WASM HTTP client operational
- [ ] Fuel metering enabled and configurable
- [ ] Memory limits via `StoreLimits` enforced
- [ ] Plugin binary size checked at install (<300KB / <120KB gzip)
- [ ] `PluginSandbox` validates all host function calls against permissions
- [ ] `init()` and `process_message()` wired to plugin pipeline
- [ ] Audit logging for all host function calls
- [ ] All 45 security tests pass (T01-T45, see Section below)

#### C2 Security Tests (MANDATORY -- 45 tests)

All tests from the WASM security spec must pass before Element 04 receives GO verdict.

**HTTP Request Tests (T01-T13)**:

| # | Test | Expected |
|---|------|----------|
| T01 | HTTP to domain in allowlist | Succeeds |
| T02 | HTTP to domain NOT in allowlist | `Err("host not in network allowlist")` |
| T03 | HTTP with `file://` scheme | `Err("scheme not allowed: file")` |
| T04 | HTTP with `data:` scheme | `Err("scheme not allowed: data")` |
| T05 | HTTP to `http://127.0.0.1/` | `Err("request to private/reserved IP denied")` |
| T06 | HTTP to `http://[::ffff:127.0.0.1]/` | `Err("request to private/reserved IP denied")` |
| T07 | HTTP to `http://169.254.169.254/` (cloud metadata) | `Err("request to private/reserved IP denied")` |
| T08 | HTTP to `http://10.0.0.1/` (RFC 1918) | `Err("request to private/reserved IP denied")` |
| T09 | Empty `permissions.network` sends HTTP | `Err("network access not permitted")` |
| T10 | 11th HTTP request within 1 minute | `Err("rate limit exceeded: HTTP requests")` |
| T11 | HTTP with 2 MB body | `Err("request body too large")` |
| T12 | `"network": ["*.example.com"]` to `sub.example.com` | Succeeds |
| T13 | `"network": ["*.example.com"]` to `example.com` | `Err("host not in network allowlist")` |

**Filesystem Tests (T14-T22)**:

| # | Test | Expected |
|---|------|----------|
| T14 | Read file within allowed path | Contents returned |
| T15 | Read `/etc/passwd` | `Err("filesystem access denied")` |
| T16 | Read via `../../etc/passwd` | `Err("filesystem access denied")` |
| T17 | Read via symlink outside sandbox | `Err("symlink points outside sandbox")` |
| T18 | Write within allowed path | Succeeds |
| T19 | Write outside allowed path | `Err("filesystem access denied")` |
| T20 | Read file >8 MB | `Err("file too large")` |
| T21 | Empty `permissions.filesystem` reads any file | `Err("filesystem access not permitted")` |
| T22 | Write via symlink outside sandbox | `Err("symlink points outside sandbox")` |

**Environment Variable Tests (T23-T27)**:

| # | Test | Expected |
|---|------|----------|
| T23 | Read env var in allowlist (set) | `Some(value)` |
| T24 | Read env var in allowlist (not set) | `None` |
| T25 | Read env var NOT in allowlist | `None` |
| T26 | Read `OPENAI_API_KEY` without allowlist | `None` |
| T27 | Empty `permissions.env_vars` reads any var | `None` |

**Resource Limit Tests (T28-T32)**:

| # | Test | Expected |
|---|------|----------|
| T28 | Infinite loop (exceeds fuel) | `PluginError::ResourceExhausted` |
| T29 | Allocate >16 MB | `PluginError::ResourceExhausted` |
| T30 | Exceed 30s wall-clock timeout | `PluginError::ResourceExhausted` |
| T31 | Custom `max_fuel = 500M` exhausted | Trap at lower threshold |
| T32 | Custom `max_memory_mb = 8` exceeded | Trap at lower threshold |

**Log Rate Limit Tests (T33-T35)**:

| # | Test | Expected |
|---|------|----------|
| T33 | 100 log messages in <1 minute | All logged |
| T34 | 101st message in same minute | Silently dropped + throttle warning |
| T35 | Log message >4 KB | Truncated with `... [truncated]` |

**Lifecycle Security Tests (T36-T42)**:

| # | Test | Expected |
|---|------|----------|
| T36 | Install WASM >300 KB | Rejected |
| T37 | Install ClawHub plugin without signature | Rejected |
| T38 | Install local plugin without signature | Warning, proceeds |
| T39 | Plugin requests `shell: true` on first run | User prompted |
| T40 | Plugin requests env var access | User prompted |
| T41 | Version upgrade adds new `network` permission | Re-prompt for new only |
| T42 | All host function calls produce audit entries | Verified |

**Cross-Cutting Tests (T43-T45)**:

| # | Test | Expected |
|---|------|----------|
| T43 | Two concurrent plugins can't read each other's memory | Isolated Stores |
| T44 | Plugin A rate limit doesn't affect Plugin B | Independent counters |
| T45 | Fuel resets between invocations | Second call gets full budget |

---

### Unit 3: C3 - Skill Loader (Week 5-6, P1, DEPENDS ON C1)

**Goal**: Replace the hand-rolled YAML parser in `skills_v2.rs` with `serde_yaml`, implement local skill discovery with proper precedence, and add auto-registration as WASM or native wrapper.

**Crate**: `clawft-core/src/agent/` (new file: `skill_loader.rs`, refactored from `skills_v2.rs`)

#### 3.1 Current Hand-Rolled YAML Parser to Replace

The current parser lives in `crates/clawft-core/src/agent/skills_v2.rs:207-273`. It is a minimal frontmatter parser that does NOT handle:
- Nested structures
- Multi-line values
- Quoted strings with special characters
- Flow sequences `[a, b, c]`
- Anchors/aliases

```rust
// CURRENT CODE (crates/clawft-core/src/agent/skills_v2.rs:207-273):
/// Minimal YAML frontmatter parser.
///
/// Supports:
/// - Scalar values: `key: value`
/// - Boolean values: `key: true` / `key: false`
/// - Sequences: lines starting with `  - item` under a key
///
/// This avoids pulling in `serde_yaml` as a dependency.
fn parse_yaml_frontmatter(
    yaml: &str,
) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
    let mut map: HashMap<String, serde_json::Value> = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_list: Vec<String> = Vec::new();

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("- ") {
            if current_key.is_some() {
                current_list.push(stripped.trim().to_string());
                continue;
            }
            return Err(format!("list item without parent key: {trimmed}"));
        }
        // ... (flush list, parse key:value, parse_scalar)
    }
    Ok(map)
}
```

**Replacement**: Use `serde_yaml::from_str` to parse frontmatter into a `SkillFrontmatter` struct:

```rust
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: Option<String>,
    version: Option<String>,
    variables: Option<Vec<String>>,
    #[serde(alias = "allowed-tools")]
    allowed_tools: Option<Vec<String>>,
    #[serde(alias = "user-invocable")]
    user_invocable: Option<bool>,
    #[serde(alias = "disable-model-invocation")]
    disable_model_invocation: Option<bool>,
    #[serde(alias = "argument-hint")]
    argument_hint: Option<String>,
    #[serde(flatten)]
    metadata: HashMap<String, serde_json::Value>,
}
```

#### 3.2 Skill Discovery Directories

The current `SkillRegistry::discover()` already implements the three-level priority (`crates/clawft-core/src/agent/skills_v2.rs:347-421`):

```rust
// CURRENT CODE (skills_v2.rs:347-421):
pub fn discover(
    workspace_dir: Option<&Path>,
    user_dir: Option<&Path>,
    builtin_skills: Vec<SkillDefinition>,
) -> Result<Self> {
    // 1. Built-in (lowest)
    // 2. User ~/.clawft/skills/ (medium)
    // 3. Workspace .clawft/skills/ (highest)
}
```

**Enhancements needed**:
- Add managed skills directory: `~/.clawft/skills` (this is the user dir, already present)
- Add bundled skills path for plugin-shipped skills
- Add WASM skill auto-registration (skills with `.wasm` module auto-load through WASM host)
- Add OpenClaw SKILL.md compatibility (already partial -- extend metadata support)

#### 3.3 Load Operations Are Blocking

The current `load_dir()` uses `std::fs::read_dir` and `std::fs::read_to_string` (see `skills_v2.rs:434-536`), which blocks the Tokio executor. This is flagged as D11 in improvements.md.

**Fix**: Replace with `tokio::fs` equivalents as part of C3:
```rust
// CURRENT (blocking):
let entries = std::fs::read_dir(dir).map_err(ClawftError::Io)?;
// REPLACEMENT:
let mut entries = tokio::fs::read_dir(dir).await.map_err(ClawftError::Io)?;
```

#### C3 Acceptance Criteria

- [ ] `serde_yaml` replaces hand-rolled parser in `parse_yaml_frontmatter()`
- [ ] `SkillFrontmatter` struct with proper deserialization
- [ ] All existing `skills_v2.rs` tests still pass (regression)
- [ ] New tests for nested YAML, multi-line values, flow sequences
- [ ] Local skill discovery: workspace > managed > bundled
- [ ] WASM skill auto-registration (skills with `.wasm` load through WASM host)
- [ ] OpenClaw SKILL.md compatibility verified
- [ ] Blocking `std::fs` replaced with `tokio::fs` (D11)
- [ ] `cargo test -p clawft-core -- skills_v2` passes

#### C3 Test Requirements

- All existing tests in `skills_v2.rs:598-1158` must continue to pass
- `test_serde_yaml_nested_structures` -- YAML with nested maps
- `test_serde_yaml_multiline_values` -- Multi-line string values
- `test_serde_yaml_flow_sequences` -- `[a, b, c]` syntax
- `test_wasm_skill_registration` -- WASM module auto-detected
- `test_openclaw_skill_compat` -- OpenClaw metadata preserved

---

### Unit 4: C4 - Hot-Reload & Dynamic Loading (Week 6-7, P1, DEPENDS ON C2+C3)

**Goal**: Implement runtime skill loading with file-system watching, skill precedence layering, plugin-shipped skills, and the `weft skill install` CLI command.

**Crates**: `clawft-core/src/agent/skill_watcher.rs` (new), `clawft-cli`

#### 4.1 Implementation Tasks

1. **File-system watcher**: Use `notify` crate to watch skill directories for changes
   - Watch: workspace `.clawft/skills/`, user `~/.clawft/skills/`, managed skills
   - Debounce rapid changes (default: 500ms)
   - On change: reload affected skill, update registry

2. **Skill precedence layering**: workspace > managed/local > bundled
   - Same-name skill at higher level overrides lower
   - Removal at higher level reveals lower-level skill

3. **Plugin-shipped skills**: Plugins declare skill directories in manifest
   - `clawft.plugin.json` field: `"skills": ["skills/"]`
   - Plugin skills load when plugin is enabled
   - Participate in normal precedence (below workspace, above bundled)

4. **Atomic skill swap**: New skill version loads alongside old; in-flight calls complete on old before swap

5. **`weft skill install <path>` CLI command**:
   - Copy skill directory to `~/.clawft/skills/<name>/`
   - Validate SKILL.md before installing
   - Hot-reload picks up the new skill automatically

#### C4 Acceptance Criteria

- [ ] `notify` crate file-system watcher watches all skill directories
- [ ] Changes detected within 2 seconds (debounced)
- [ ] Skill precedence: workspace > managed > bundled verified
- [ ] Plugin-shipped skills load from manifest `skills` field
- [ ] Atomic swap prevents in-flight tool call interruption
- [ ] `weft skill install <path>` copies and validates
- [ ] `weft skill install <path>` triggers hot-reload
- [ ] `weft skill list` shows all loaded skills with source
- [ ] `weft skill remove <name>` removes from managed directory

---

### Unit 5: C5+C6 - Commands + MCP Exposure (Week 7, P2+P1)

**Goal**: Wire the existing slash-command registry to agent commands, and expose loaded skills through MCP.

#### 5.1 C5 -- Slash-Command Framework

**Current state**: The interactive module (`crates/clawft-cli/src/interactive/`) has a fully functional `SlashCommandRegistry` and `builtins` module, but they are **dead code** -- `agent.rs` implements commands inline with `match`.

**`crates/clawft-cli/src/interactive/registry.rs`** already defines:
```rust
// CURRENT (functional but unused):
pub struct SlashCommandRegistry {
    commands: HashMap<String, Box<dyn SlashCommand>>,
}
impl SlashCommandRegistry {
    pub fn dispatch(&self, input: &str, ctx: &mut InteractiveContext)
        -> Option<anyhow::Result<String>> { ... }
}
```

**`crates/clawft-cli/src/interactive/builtins.rs`** already registers 8 commands:
```rust
// CURRENT (functional but unused):
pub fn register_builtins(registry: &mut SlashCommandRegistry) {
    registry.register(Box::new(HelpCommand));
    registry.register(Box::new(SkillsCommand));
    registry.register(Box::new(UseCommand));
    registry.register(Box::new(AgentCommand));
    registry.register(Box::new(ClearCommand));
    registry.register(Box::new(StatusCommand));
    registry.register(Box::new(QuitCommand));
    registry.register(Box::new(ToolsCommand));
}
```

**Implementation tasks**:
1. Replace inline `match` in `agent.rs` with `SlashCommandRegistry::dispatch()`
2. Add `register_skill_commands()` -- skills with `user_invocable: true` register as `/skill_name`
3. Command name collision detection: if skill command conflicts with builtin, log error
4. Update `/help` output to include skill-contributed commands

#### 5.2 C6 -- MCP Skill Exposure

**Current state**: `crates/clawft-services/src/mcp/server.rs` handles `tools/list` and `tools/call` via `CompositeToolProvider`:

```rust
// CURRENT (crates/clawft-services/src/mcp/server.rs:17-19):
const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "clawft";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
```

**Implementation tasks**:
1. `tools/list` includes loaded skill tools with JSON Schema parameter definitions
2. `tools/call` for a skill-provided tool routes through `skill.execute_tool()`
3. Hot-reload updates MCP tool listing without server restart (watch for `SkillRegistry` changes)

#### C5 Acceptance Criteria

- [ ] Agent commands routed through `SlashCommandRegistry`, not inline match
- [ ] Skills can contribute commands that appear in `/help`
- [ ] Command name collisions produce clear error
- [ ] All existing interactive tests pass

#### C6 Acceptance Criteria

- [ ] MCP `tools/list` includes loaded skill tools with JSON Schema
- [ ] MCP `tools/call` routes through `skill.execute_tool()`
- [ ] Hot-reload updates MCP tool listing without restart

---

### Unit 6: C7 - PluginHost Unification (Week 8, P2)

**Goal**: Migrate existing channels through the unified `PluginHost` and make lifecycle operations concurrent.

**Crate**: `clawft-channels/src/host.rs`

#### 6.1 Current `PluginHost` State

The current `PluginHost` (`crates/clawft-channels/src/host.rs:27-210`) manages channels but has two issues:

1. **`start_all()` is sequential** (lines 89-100):
```rust
// CURRENT (sequential):
pub async fn start_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let channels = self.channels.read().await;
    let names: Vec<String> = channels.keys().cloned().collect();
    drop(channels);
    let mut results = Vec::with_capacity(names.len());
    for name in names {
        let result = self.start_channel(&name).await;  // Sequential!
        results.push((name, result));
    }
    results
}
```

2. **`stop_all()` is sequential** (lines 106-117):
```rust
// CURRENT (sequential):
pub async fn stop_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let tokens = self.cancel_tokens.read().await;
    let names: Vec<String> = tokens.keys().cloned().collect();
    drop(tokens);
    let mut results = Vec::with_capacity(names.len());
    for name in names {
        let result = self.stop_channel(&name).await;  // Sequential!
        results.push((name, result));
    }
    results
}
```

**Replacements**: Use `futures::future::join_all` for concurrent execution:

```rust
// REPLACEMENT for start_all():
pub async fn start_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
    let channels = self.channels.read().await;
    let names: Vec<String> = channels.keys().cloned().collect();
    drop(channels);

    let futures: Vec<_> = names.iter().map(|name| {
        let name = name.clone();
        async move {
            let result = self.start_channel(&name).await;
            (name, result)
        }
    }).collect();

    futures::future::join_all(futures).await
}
```

#### 6.2 Implementation Tasks

1. **Channel migration**: Existing Telegram, Discord, Slack channels work through unified `PluginHost` with `ChannelAdapter` trait (from C1) without behavior changes
2. **Concurrent `start_all`/`stop_all`**: Replace sequential loops with `join_all`
3. **SOUL.md injection**: Read SOUL.md personality content and inject into the Assembler pipeline stage system prompt

#### C7 Acceptance Criteria

- [ ] Existing Telegram, Discord, Slack channels work through unified PluginHost
- [ ] No behavior changes for existing channels
- [ ] `start_all()` executes concurrently
- [ ] `stop_all()` executes concurrently
- [ ] SOUL.md content injected into Assembler pipeline stage
- [ ] All existing `host.rs` tests pass

---

### Unit 7: C4a - Autonomous Skill Creation (Week 8+, P2 STRETCH)

**Goal**: Enable agent to detect repeated patterns and auto-generate skills.

**Crate**: `clawft-core`

**This is a stretch goal. Only implement if C1-C7 are complete.**

#### 7.1 Implementation Tasks

1. **Pattern detection**: Agent loop tracks repeated task patterns
   - Configurable threshold (default: 3 repetitions)
   - Pattern matching based on tool call sequences
   - Disabled by default, opt-in via config

2. **Skill generation**: Auto-write `SKILL.md` + implementation
   - Generated skills follow same validation as manual skills
   - Minimal permissions: no shell, no network, filesystem limited to workspace

3. **WASM compilation**: Compile generated native skills to WASM

4. **Managed install**: Install into `~/.clawft/skills/` in "pending" state
   - User must approve before activation
   - Display generated skill details for review

#### C4a Acceptance Criteria

- [ ] Pattern detection threshold configurable (default: 3)
- [ ] Generated SKILL.md passes same validation as manual skills
- [ ] User prompted for approval before install
- [ ] Autonomous creation disabled by default, opt-in only
- [ ] Auto-generated skills have minimal permissions (no shell, no network, workspace-only FS)

---

## Security Requirements Summary

### Host-Function Permission Enforcement

| Host Function | Permission Check |
|---------------|-----------------|
| `http-request` | URL parse -> scheme check -> network allowlist -> SSRF `is_private_ip()` -> rate limit -> body size |
| `read-file` | Canonicalize -> sandbox containment -> symlink traversal -> file size (8MB max) |
| `write-file` | Resolve parent -> sandbox containment -> symlink traversal -> write size (4MB max) -> atomic write |
| `get-env` | Allowlist check -> implicit deny patterns -> return `None` (not error) for denied |
| `log` | Rate limit (100/min) -> message truncation (4KB) -> level mapping |

### Resource Limits

| Resource | Default | Configurable? |
|----------|---------|---------------|
| WASM fuel metering | 1,000,000,000 units (~1s CPU) | Yes, via manifest |
| WASM memory | 16 MB per plugin | Yes, via manifest |
| Plugin binary size | <300 KB uncompressed, <120 KB gzipped | Yes, via global config |
| HTTP requests | 10/minute per plugin | Yes, via manifest |
| Log messages | 100/minute per plugin | Yes, via manifest |
| Execution timeout | 30 seconds | Yes, via manifest |

---

## Exit Criteria Checklist

### Core (C1-C4)
- [ ] `clawft-plugin` crate compiles with all trait definitions
- [ ] At least one plugin implements each of the six traits
- [ ] WASM plugin host loads and runs a test plugin
- [ ] `weft skill install <path>` works for local skills
- [ ] Hot-reload detects file changes within 2 seconds
- [ ] Skill precedence (workspace > managed > bundled) verified
- [ ] VoiceHandler trait placeholder exists (forward-compat)
- [ ] All existing tests pass

### C4a (Autonomous Skill Creation)
- [ ] Pattern detection threshold configurable (default: 3)
- [ ] Generated SKILL.md passes same validation as manual skills
- [ ] User prompted for approval before install
- [ ] Autonomous skill creation disabled by default, opt-in
- [ ] Auto-generated skills have minimal permissions

### C5 (Slash-Command Framework)
- [ ] Agent commands routed through registry, not inline match
- [ ] Skills can contribute commands appearing in `/help`
- [ ] Command name collisions produce clear error

### C6 (MCP Skill Exposure)
- [ ] MCP `tools/list` includes loaded skill tools with JSON Schema
- [ ] MCP `tools/call` routes through `skill.execute_tool()`
- [ ] Hot-reload updates MCP tool listing without restart

### C7 (PluginHost Unification)
- [ ] Existing channels work through unified PluginHost
- [ ] `start_all()` / `stop_all()` execute concurrently
- [ ] SOUL.md injected into Assembler pipeline stage

### Security
- [ ] Every WIT host function validates against `PluginPermissions`
- [ ] WASM fuel metering enabled (configurable, default 1B units)
- [ ] WASM memory limits via `StoreLimits` (default 16MB)
- [ ] `read-file`/`write-file` canonicalize paths, reject external symlinks
- [ ] `http-request` applies SSRF check + network allowlist
- [ ] `get-env` returns `None` for non-permitted vars
- [ ] Rate limiting on `http-request` and `log`
- [ ] Audit logging for all host function calls
- [ ] ClawHub installs require signature verification
- [ ] First-run permission approval implemented
- [ ] All 45 security tests (T01-T45) pass

---

## Development Notes Location

`.planning/development_notes/02-improvements-overview/element-04/`

Subdirectories:
- `c1-plugin-trait-crate/` -- Trait design decisions, API evolution notes
- `c2-wasm-host/` -- wasmtime integration notes, WIT interface versioning
- `c3-skill-loader/` -- serde_yaml migration notes, discovery path resolution
- `c4-hot-reload/` -- notify crate config, debounce tuning
- `c5-slash-commands/` -- Registry wiring notes
- `c6-mcp-exposure/` -- MCP protocol integration notes
- `c7-pluginhost/` -- Channel migration log
- `c4a-autonomous/` -- Pattern detection algorithm notes
- `security/` -- Security review responses, test results
