# Phase C1: Plugin Trait Crate

> **Element:** 04 -- Plugin & Skill System
> **Phase:** C1
> **Timeline:** Week 3-4
> **Priority:** P0 (Critical Path)
> **Crate:** `crates/clawft-plugin/` (NEW)
> **Dependencies IN:** Element 03 B3 (file splits), A4 (SecretRef), A9 (feature gates)
> **Blocks:** C2, C3, C4, C4a, C5, C6, C7, Elements 06-10
> **Status:** Planning

---

## 1. Overview

Phase C1 creates the `clawft-plugin` crate -- a new foundational crate containing the 6 plugin traits, manifest schema, permission types, and error types that ALL downstream feature work depends on. This is the P0 critical path: nothing in Elements 06-10 can begin until C1 is complete.

The crate defines the contract between the host (clawft-core, clawft-wasm, clawft-channels) and plugins (WASM modules, native extensions, skills). Every plugin capability -- tools, channels, pipeline stages, skills, memory backends, and future voice support -- implements one or more of these traits.

---

## 2. Current Code

### Existing Channel Traits (to be superseded)

The current `Channel` trait lives in `crates/clawft-channels/src/traits.rs` and will be superseded by the new `ChannelAdapter` trait in `clawft-plugin`. The existing traits:

```rust
// crates/clawft-channels/src/traits.rs (current)
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn metadata(&self) -> ChannelMetadata;
    fn status(&self) -> ChannelStatus;
    fn is_allowed(&self, sender_id: &str) -> bool;
    async fn start(&self, host: Arc<dyn ChannelHost>, cancel: CancellationToken) -> Result<(), ChannelError>;
    async fn send(&self, msg: &OutboundMessage) -> Result<MessageId, ChannelError>;
}

#[async_trait]
pub trait ChannelHost: Send + Sync {
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError>;
    async fn register_command(&self, cmd: Command) -> Result<(), ChannelError>;
    async fn publish_inbound(...) -> Result<(), ChannelError>;
}

pub trait ChannelFactory: Send + Sync {
    fn channel_name(&self) -> &str;
    fn build(&self, config: &serde_json::Value) -> Result<Arc<dyn Channel>, ChannelError>;
}
```

Supporting types: `ChannelMetadata`, `ChannelStatus`, `MessageId`, `Command`, `CommandParameter`.

**Migration strategy:** The existing `Channel` trait in `clawft-channels` is NOT deleted in C1. Instead, `ChannelAdapter` is defined in `clawft-plugin` as the new contract. C7 (PluginHost Unification) handles the migration of existing channel implementations to `ChannelAdapter`. During the transition, `clawft-channels` depends on `clawft-plugin` and implements a bridge adapter.

### No Existing Plugin Crate

There is no `crates/clawft-plugin/` directory today. This crate is created from scratch.

---

## 3. Implementation Tasks

### Task C1.1: Create crate scaffold

Create `crates/clawft-plugin/` with:

```
crates/clawft-plugin/
  Cargo.toml
  src/
    lib.rs          -- re-exports
    traits.rs       -- 6 plugin traits + KeyValueStore + ToolContext
    manifest.rs     -- PluginManifest, PluginCapability, PluginPermissions, PluginResourceConfig
    error.rs        -- PluginError enum
    message.rs      -- MessagePayload enum
```

### Task C1.2: Cargo.toml

```toml
[package]
name = "clawft-plugin"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Plugin trait definitions for clawft"

[features]
default = []
voice = []   # Empty no-op -- reserves the feature flag for Workstream G

[dependencies]
async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

Add `clawft-plugin` to the workspace `[members]` list in the root `Cargo.toml`.

### Task C1.3: Define 6 Plugin Traits

All traits must be `Send + Sync`. Async methods use `#[async_trait]`.

#### 3a. `Tool` Trait

```rust
use async_trait::async_trait;

/// A tool that can be invoked by an agent or exposed via MCP.
///
/// Tools are the primary extension point for adding new capabilities.
/// Each tool declares its name, description, and a JSON Schema for
/// its parameters. The host routes `execute()` calls based on the
/// tool name.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., `"web_search"`, `"file_read"`).
    fn name(&self) -> &str;

    /// Human-readable description of what the tool does.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's parameters.
    ///
    /// Returns a `serde_json::Value` representing a JSON Schema object.
    /// The host uses this schema for validation and for MCP `tools/list`.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given parameters and context.
    ///
    /// `params` is a JSON object matching `parameters_schema()`.
    /// Returns a JSON value with the tool's result, or a `PluginError`.
    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &dyn ToolContext,
    ) -> Result<serde_json::Value, PluginError>;
}
```

#### 3b. `ChannelAdapter` Trait

Replaces the existing `Channel` trait in `clawft-channels/src/traits.rs`. Adds `MessagePayload` support for voice forward-compatibility.

```rust
/// Message payload types for channel communication.
///
/// Supports text, structured data, and binary payloads. The `Binary`
/// variant is reserved for voice/audio data (Workstream G).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MessagePayload {
    /// Plain text message.
    Text(String),
    /// Structured data (JSON object).
    Structured(serde_json::Value),
    /// Binary data with MIME type (e.g., audio/wav for voice).
    Binary {
        mime_type: String,
        data: Vec<u8>,
    },
}

/// A channel adapter for connecting to external messaging platforms.
///
/// Replaces the existing `Channel` trait with a plugin-oriented design
/// that supports `MessagePayload` variants for text, structured, and
/// binary (voice) content.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Unique channel identifier (e.g., `"telegram"`, `"slack"`).
    fn name(&self) -> &str;

    /// Human-readable display name.
    fn display_name(&self) -> &str;

    /// Whether this adapter supports threaded conversations.
    fn supports_threads(&self) -> bool;

    /// Whether this adapter supports media/binary payloads.
    fn supports_media(&self) -> bool;

    /// Start receiving messages. Long-lived -- runs until cancelled.
    async fn start(
        &self,
        host: std::sync::Arc<dyn ChannelAdapterHost>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<(), PluginError>;

    /// Send a message payload through this channel.
    async fn send(
        &self,
        target: &str,
        payload: &MessagePayload,
    ) -> Result<String, PluginError>;
}

/// Host services exposed to channel adapters.
#[async_trait]
pub trait ChannelAdapterHost: Send + Sync {
    /// Deliver an inbound message payload to the agent pipeline.
    async fn deliver_inbound(
        &self,
        channel: &str,
        sender_id: &str,
        chat_id: &str,
        payload: MessagePayload,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), PluginError>;
}
```

#### 3c. `PipelineStage` Trait

```rust
/// Types of pipeline stages.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PipelineStageType {
    /// Pre-processing (input validation, normalization).
    PreProcess,
    /// Core processing (LLM calls, tool routing).
    Process,
    /// Post-processing (response formatting, filtering).
    PostProcess,
    /// Observation / logging (read-only tap on the pipeline).
    Observer,
}

/// A stage in the agent processing pipeline.
///
/// Pipeline stages are composed in order: PreProcess -> Process ->
/// PostProcess, with Observers receiving copies at each step.
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Stage name (e.g., `"assembler"`, `"tool_router"`).
    fn name(&self) -> &str;

    /// What type of stage this is.
    fn stage_type(&self) -> PipelineStageType;

    /// Process input and return output.
    ///
    /// `input` is a JSON value representing the current pipeline state.
    /// Returns the transformed pipeline state.
    async fn process(
        &self,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError>;
}
```

#### 3d. `Skill` Trait

```rust
/// A skill is a high-level agent capability composed of tools,
/// instructions, and configuration.
///
/// Skills are the primary unit of agent customization. They can be
/// loaded from SKILL.md files, bundled with plugins, or
/// auto-generated. Skills can contribute tools that appear in MCP
/// `tools/list` and can be invoked via slash commands.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Skill name (e.g., `"code-review"`, `"git-commit"`).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Semantic version string.
    fn version(&self) -> &str;

    /// Template variables the skill accepts (name -> description).
    fn variables(&self) -> std::collections::HashMap<String, String>;

    /// Tool names this skill is allowed to invoke.
    fn allowed_tools(&self) -> Vec<String>;

    /// System instructions injected when the skill is active.
    fn instructions(&self) -> &str;

    /// Whether this skill can be invoked directly by users (e.g., via /command).
    fn is_user_invocable(&self) -> bool;

    /// Execute a tool provided by this skill.
    ///
    /// `tool_name` is the specific tool within this skill to call.
    /// `params` is a JSON object of tool parameters.
    /// `ctx` is the execution context providing key-value store access.
    async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        ctx: &dyn ToolContext,
    ) -> Result<serde_json::Value, PluginError>;
}
```

#### 3e. `MemoryBackend` Trait

```rust
/// A pluggable memory storage backend.
///
/// Supports key-value storage with optional namespace isolation,
/// TTL, tags, and semantic search. Implementations may use
/// in-memory stores, SQLite, HNSW indices, or external services.
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Store a value with optional metadata.
    async fn store(
        &self,
        key: &str,
        value: &str,
        namespace: Option<&str>,
        ttl_seconds: Option<u64>,
        tags: Option<Vec<String>>,
    ) -> Result<(), PluginError>;

    /// Retrieve a value by key.
    async fn retrieve(
        &self,
        key: &str,
        namespace: Option<&str>,
    ) -> Result<Option<String>, PluginError>;

    /// Search for values matching a query string.
    async fn search(
        &self,
        query: &str,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, String, f64)>, PluginError>;

    /// Delete a value by key.
    async fn delete(
        &self,
        key: &str,
        namespace: Option<&str>,
    ) -> Result<bool, PluginError>;
}
```

#### 3f. `VoiceHandler` Trait (Placeholder)

```rust
/// Placeholder trait for voice/audio processing (Workstream G).
///
/// This trait is defined now to reserve the capability type and
/// ensure forward-compatibility. Implementations will be added
/// when Workstream G begins. The `voice` feature flag gates
/// any voice-specific dependencies.
#[async_trait]
pub trait VoiceHandler: Send + Sync {
    /// Process raw audio input and return a transcription or response.
    ///
    /// `audio_data` is raw audio bytes. `mime_type` indicates the format
    /// (e.g., `"audio/wav"`, `"audio/opus"`).
    async fn process_audio(
        &self,
        audio_data: &[u8],
        mime_type: &str,
    ) -> Result<String, PluginError>;

    /// Synthesize text into audio output.
    ///
    /// Returns audio bytes and the MIME type of the output format.
    async fn synthesize(
        &self,
        text: &str,
    ) -> Result<(Vec<u8>, String), PluginError>;
}
```

### Task C1.4: Define Supporting Traits

#### `KeyValueStore` Trait (Cross-Element Contract)

From `01-cross-element-integration.md` -- this is the shared contract that `ToolContext` exposes to plugins for state access.

```rust
/// Key-value store interface exposed to plugins via ToolContext.
///
/// This is the cross-element contract defined in the integration spec.
/// Implementations may be backed by in-memory maps, SQLite, or the
/// agent's memory system.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Get a value by key. Returns None if not found.
    async fn get(&self, key: &str) -> Result<Option<String>, PluginError>;

    /// Set a value for a key.
    async fn set(&self, key: &str, value: &str) -> Result<(), PluginError>;

    /// Delete a key. Returns true if the key existed.
    async fn delete(&self, key: &str) -> Result<bool, PluginError>;

    /// List all keys with an optional prefix filter.
    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, PluginError>;
}
```

#### `ToolContext` Trait

```rust
/// Execution context passed to Tool::execute() and Skill::execute_tool().
///
/// Provides access to the key-value store, plugin identity, and
/// agent identity. This is the plugin's window into the host.
pub trait ToolContext: Send + Sync {
    /// Access the key-value store for plugin state.
    fn key_value_store(&self) -> &dyn KeyValueStore;

    /// The ID of the plugin that owns this tool.
    fn plugin_id(&self) -> &str;

    /// The ID of the agent invoking this tool.
    fn agent_id(&self) -> &str;
}
```

### Task C1.5: Define Plugin Manifest Types

```rust
use serde::{Deserialize, Serialize};

/// Plugin manifest parsed from `clawft.plugin.json` or `.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (reverse-domain, e.g., `"com.example.my-plugin"`).
    pub id: String,

    /// Human-readable plugin name.
    pub name: String,

    /// Semantic version string (must be valid semver).
    pub version: String,

    /// Capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,

    /// Permissions the plugin requests.
    #[serde(default)]
    pub permissions: PluginPermissions,

    /// Resource limits configuration.
    #[serde(default)]
    pub resources: PluginResourceConfig,

    /// Path to the WASM module (relative to plugin directory).
    #[serde(default)]
    pub wasm_module: Option<String>,

    /// Skills provided by this plugin.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Tools provided by this plugin.
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Plugin capability types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Tool,
    Channel,
    PipelineStage,
    Skill,
    MemoryBackend,
    /// Reserved for Workstream G.
    Voice,
}

/// Permissions requested by a plugin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    /// Allowed network hosts. Empty = no network. `["*"]` = all hosts.
    #[serde(default)]
    pub network: Vec<String>,

    /// Allowed filesystem paths.
    #[serde(default)]
    pub filesystem: Vec<String>,

    /// Allowed environment variable names.
    #[serde(default)]
    pub env_vars: Vec<String>,

    /// Whether the plugin can execute shell commands.
    #[serde(default)]
    pub shell: bool,
}

/// Resource limits for plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResourceConfig {
    /// Maximum WASM fuel per invocation (default: 1,000,000,000).
    #[serde(default = "default_max_fuel")]
    pub max_fuel: u64,

    /// Maximum WASM memory in MB (default: 16).
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: usize,

    /// Maximum HTTP requests per minute (default: 10).
    #[serde(default = "default_max_http_rpm")]
    pub max_http_requests_per_minute: u64,

    /// Maximum log messages per minute (default: 100).
    #[serde(default = "default_max_log_rpm")]
    pub max_log_messages_per_minute: u64,

    /// Maximum execution wall-clock seconds (default: 30).
    #[serde(default = "default_max_exec_seconds")]
    pub max_execution_seconds: u64,

    /// Maximum WASM table elements (default: 10,000).
    #[serde(default = "default_max_table_elements")]
    pub max_table_elements: u32,
}

fn default_max_fuel() -> u64 { 1_000_000_000 }
fn default_max_memory_mb() -> usize { 16 }
fn default_max_http_rpm() -> u64 { 10 }
fn default_max_log_rpm() -> u64 { 100 }
fn default_max_exec_seconds() -> u64 { 30 }
fn default_max_table_elements() -> u32 { 10_000 }

impl Default for PluginResourceConfig {
    fn default() -> Self {
        Self {
            max_fuel: default_max_fuel(),
            max_memory_mb: default_max_memory_mb(),
            max_http_requests_per_minute: default_max_http_rpm(),
            max_log_messages_per_minute: default_max_log_rpm(),
            max_execution_seconds: default_max_exec_seconds(),
            max_table_elements: default_max_table_elements(),
        }
    }
}
```

### Task C1.6: Define PluginError

```rust
use thiserror::Error;

/// Errors produced by plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin failed to load (bad manifest, missing WASM module, etc.).
    #[error("plugin load failed: {0}")]
    LoadFailed(String),

    /// Plugin execution failed at runtime.
    #[error("plugin execution failed: {0}")]
    ExecutionFailed(String),

    /// Operation denied by the permission sandbox.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Plugin exceeded a resource limit (fuel, memory, rate limit).
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Requested capability or feature is not implemented.
    #[error("not implemented: {0}")]
    NotImplemented(String),

    /// I/O error during plugin operation.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### Task C1.7: Manifest Validation

Implement a `PluginManifest::validate()` method:

```rust
impl PluginManifest {
    /// Validate the manifest. Returns an error describing the first
    /// validation failure, or Ok(()) if the manifest is valid.
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.id.is_empty() {
            return Err(PluginError::LoadFailed("manifest: id is required".into()));
        }
        // Validate semver
        if semver::Version::parse(&self.version).is_err() {
            return Err(PluginError::LoadFailed(
                format!("manifest: invalid semver version '{}'", self.version),
            ));
        }
        if self.capabilities.is_empty() {
            return Err(PluginError::LoadFailed(
                "manifest: at least one capability is required".into(),
            ));
        }
        Ok(())
    }

    /// Parse a manifest from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, PluginError> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse a manifest from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, PluginError> {
        // Note: serde_yaml is NOT a dependency of clawft-plugin.
        // YAML parsing happens in clawft-core's skill loader (C3).
        // This method is a placeholder that returns NotImplemented
        // until C3 adds the serde_yaml dep to whichever crate
        // handles YAML loading.
        Err(PluginError::NotImplemented(
            "YAML manifest parsing deferred to C3 skill loader".into(),
        ))
    }
}
```

**Note on YAML:** Adding `serde_yaml` to `clawft-plugin` would increase the dependency footprint of a crate that WASM plugins may depend on. Instead, YAML manifest parsing will be handled in the loader layer (C3) which calls `serde_yaml::from_str()` and then constructs a `PluginManifest`. The `from_yaml()` method on `PluginManifest` is a convenience stub that will be wired in C3. For C1, JSON parsing is the required deliverable.

### Task C1.8: Wire lib.rs re-exports

```rust
// crates/clawft-plugin/src/lib.rs

pub mod error;
pub mod manifest;
pub mod message;
pub mod traits;

// Re-export core types at crate root for convenience
pub use error::PluginError;
pub use manifest::{
    PluginCapability, PluginManifest, PluginPermissions, PluginResourceConfig,
};
pub use message::MessagePayload;
pub use traits::{
    ChannelAdapter, ChannelAdapterHost, KeyValueStore, MemoryBackend,
    PipelineStage, PipelineStageType, Skill, Tool, ToolContext, VoiceHandler,
};
```

---

## 4. Concurrency Plan

C1 is a single-crate, single-developer task. No parallel execution needed. However, once C1 merges:

- C2 (WASM Host) and C3 (Skill Loader) can begin in parallel since they consume different traits
- C7 (PluginHost Unification) can begin channel migration planning

The traits in C1 are designed to be consumed concurrently by multiple downstream crates without lock contention -- all trait objects are `Send + Sync`, and the `ToolContext` uses `&dyn` references (no `Arc` wrapping needed at the trait boundary).

---

## 5. Tests Required

All tests in `crates/clawft-plugin/src/` using `#[cfg(test)]` modules.

### Manifest Tests

| Test | Description |
|------|-------------|
| `test_manifest_parse_json` | Parse a valid JSON manifest string into `PluginManifest`. Verify all fields round-trip correctly. |
| `test_manifest_parse_yaml` | Call `from_yaml()` and verify it returns `PluginError::NotImplemented` (stub until C3). |
| `test_manifest_missing_id_fails` | JSON with `"id": ""` fails validation with `LoadFailed` error containing "id is required". |
| `test_manifest_invalid_version_fails` | JSON with `"version": "not-semver"` fails validation with `LoadFailed` error containing "invalid semver". |
| `test_manifest_empty_capabilities_fails` | JSON with `"capabilities": []` fails validation with `LoadFailed` error containing "at least one capability". |

### Serde Tests

| Test | Description |
|------|-------------|
| `test_plugin_capability_serde_roundtrip` | Serialize each `PluginCapability` variant to JSON and deserialize back. Verify equality. All 6 variants: Tool, Channel, PipelineStage, Skill, MemoryBackend, Voice. |

### Default/Permission Tests

| Test | Description |
|------|-------------|
| `test_permissions_default_is_empty` | `PluginPermissions::default()` has empty `network`, empty `filesystem`, empty `env_vars`, `shell: false`. |
| `test_resource_config_defaults` | `PluginResourceConfig::default()` has `max_fuel: 1_000_000_000`, `max_memory_mb: 16`, `max_http_requests_per_minute: 10`, `max_log_messages_per_minute: 100`, `max_execution_seconds: 30`, `max_table_elements: 10_000`. |

### MessagePayload Tests

| Test | Description |
|------|-------------|
| `test_message_payload_binary_variant` | Construct `MessagePayload::Binary { mime_type: "audio/wav".into(), data: vec![0u8; 16] }`. Serialize to JSON and verify it round-trips. |

### Feature Flag Tests

| Test | Description |
|------|-------------|
| `test_voice_feature_flag_compiles` | Compile the crate with `--features voice`. Verify `VoiceHandler` trait is available. (This is a build test, not a runtime test -- verified by CI running `cargo build -p clawft-plugin --features voice`.) |

### Trait Object Safety Tests

| Test | Description |
|------|-------------|
| `test_traits_are_send_sync` | Compile-time assertion that all 6 trait objects + `KeyValueStore` + `ToolContext` are `Send + Sync`. Use the `fn assert_send_sync<T: Send + Sync>() {}` pattern. |

---

## 6. Acceptance Criteria

- [ ] `cargo build -p clawft-plugin` compiles cleanly
- [ ] `cargo build -p clawft-plugin --features voice` compiles cleanly
- [ ] All 6 traits defined: `Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler`
- [ ] All 6 traits are `Send + Sync` (verified by compile-time test)
- [ ] `KeyValueStore` trait defined with `get`, `set`, `delete`, `list_keys`
- [ ] `ToolContext` trait defined with `key_value_store()`, `plugin_id()`, `agent_id()`
- [ ] `PluginManifest` parses from JSON with validation
- [ ] `PluginManifest::validate()` rejects empty id, invalid semver, empty capabilities
- [ ] `PluginCapability::Voice` variant exists and serializes correctly
- [ ] `MessagePayload::Binary` variant exists with `mime_type` and `data` fields
- [ ] `PluginPermissions::default()` returns all-empty/false
- [ ] `PluginResourceConfig::default()` returns documented defaults
- [ ] `PluginError` has all 7 variants: LoadFailed, ExecutionFailed, PermissionDenied, ResourceExhausted, NotImplemented, Io, Serialization
- [ ] Feature flag `voice` is wired as empty no-op
- [ ] `cargo clippy -p clawft-plugin -- -D warnings` is clean
- [ ] `cargo test -p clawft-plugin` -- all tests pass
- [ ] Crate added to workspace `[members]`

---

## 7. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Trait design needs revision after C2/C3 integration | Medium | Low | Traits are behind `async_trait` which allows signature evolution. Breaking changes are acceptable pre-1.0. |
| `async_trait` overhead in WASM context | Low | Low | `async_trait` compiles to `Box<dyn Future>`. For WASM plugins, the WIT interface boundary already involves serialization. Overhead is negligible. |
| `tokio_util::sync::CancellationToken` in `ChannelAdapter::start` | Low | Medium | This ties `clawft-plugin` to tokio-util. If WASM plugins need a different cancellation mechanism, a plugin-local `CancellationToken` wrapper can be added later. For now, all channel adapters run on tokio. |
| Semver validation requires `semver` crate dependency | Low | Low | The `semver` crate is small (pure Rust, no deps). Add it as a dependency of `clawft-plugin` or perform basic regex validation instead. Prefer the `semver` crate for correctness. |
