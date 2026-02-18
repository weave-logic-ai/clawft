# SPARC Plan: Stream 1A - Types + Platform + Plugin API

**Phase**: Warp (Week 1-3)
**Owner**: Foundation architect
**Branch**: `weft/types`
**Status**: BLOCKS all other streams

---

## Agent Instructions

### 🎯 Mission
You are implementing the foundational layer for the clawft Rust rewrite. This is the **warp** -- the foundation threads that all other streams depend on. Your work MUST be stable by end of week 2 to unblock Stream 1B (Core) and Stream 1C (Provider+Tools+CLI).

### 📁 Python Source Files to Port From
Read these files to understand the data models and behavior:

| Python File | Purpose | Output Rust Module |
|-------------|---------|-------------------|
| `nanobot/config/schema.py` | Pydantic config models | `clawft-types/src/config.rs` |
| `nanobot/config/loader.py` | Config discovery + loading | `clawft-platform/src/config_loader.rs` |
| `nanobot/providers/base.py` | Provider trait + response types | `clawft-types/src/provider.rs` |
| `nanobot/providers/registry.py` | Provider specs registry | `clawft-types/src/provider.rs` |
| `nanobot/channels/base.py` | Channel base class | `clawft-channels/src/trait.rs` |
| `nanobot/channels/manager.py` | Channel lifecycle | `clawft-channels/src/host.rs` |
| `nanobot/session/manager.py` | Session types | `clawft-types/src/session.rs` |
| `nanobot/cron/types.py` | Cron types | `clawft-types/src/cron.rs` |
| `nanobot/agent/events.py` | Message events | `clawft-types/src/event.rs` |
| `nanobot/agent/tools/base.py` | Tool trait (reference) | N/A (tool trait is in Stream 1B) |

### 📚 Planning Docs to Reference
- `repos/nanobot/.planning/02-technical-requirements.md` -- All struct definitions, trait signatures, workspace layout
- `repos/nanobot/.planning/03-development-guide.md` -- Phase overview, dependency graph, testing contracts
- `repos/nanobot/.planning/06-provider-layer-options.md` -- Provider architecture decisions (clawft-llm standalone library)

### 🏗️ Crate/Module Structure to Create

```
repos/nanobot/
├── Cargo.toml                           # Workspace root (create this)
├── rust-toolchain.toml                  # Pin Rust 1.85+, edition 2024
├── .cargo/config.toml                   # Build flags, WASM profile
├── crates/
│   ├── clawft-types/                    # Week 1-2
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                   # Re-exports
│   │       ├── config.rs                # Config, AgentsConfig, ChannelsConfig, ProvidersConfig, etc.
│   │       ├── event.rs                 # InboundMessage, OutboundMessage
│   │       ├── provider.rs              # LlmResponse, ToolCallRequest, ProviderSpec, PROVIDERS array
│   │       ├── session.rs               # Session, SessionTurn
│   │       ├── cron.rs                  # CronJob, CronSchedule
│   │       └── error.rs                 # ClawftError, ConfigError, etc.
│   ├── clawft-platform/                 # Week 2-3
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                   # Trait definitions + Platform trait bundle
│   │       ├── http.rs                  # HttpClient trait + NativeHttpClient (reqwest)
│   │       ├── fs.rs                    # FileSystem trait + NativeFileSystem (std::fs)
│   │       ├── env.rs                   # Environment trait + NativeEnvironment (std::env)
│   │       ├── process.rs               # ProcessSpawner trait + NativeProcessSpawner (tokio::process)
│   │       └── config_loader.rs         # load_config() with discovery algorithm
│   └── clawft-channels/                 # Week 2-3
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                   # Re-exports
│           ├── trait.rs                 # Channel, ChannelHost, ChannelFactory traits
│           ├── host.rs                  # PluginHost (lifecycle, registry, routing)
│           └── telegram.rs              # (Week 7, Stream 1C -- placeholder only)
```

### 🚨 Critical Success Criteria

**By End of Week 1**:
- [x] Workspace compiles with `cargo build --workspace`
- [x] `clawft-types` crate compiles with zero warnings
- [x] `Config` struct deserializes real `~/.nanobot/config.json` file
- [x] All error types defined and tested

**By End of Week 2 (BLOCKER DEADLINE)**:
- [x] `clawft-platform` trait API finalized and documented
- [x] `Channel`, `ChannelHost`, `ChannelFactory` traits finalized and documented
- [x] Provider registry (`PROVIDERS` array) compiles and is queryable
- [x] Stream 1B and 1C can start coding against stable APIs

**By End of Week 3**:
- [x] Native platform implementations (reqwest, std::fs, std::env, tokio::process) work
- [x] `load_config()` reads config.json with proper fallback chain
- [x] `PluginHost` manages lifecycle of mock channel plugins
- [x] All unit tests pass (`cargo test --workspace`)

### ⚠️ Definition of Done
1. Code compiles without warnings (`cargo clippy`)
2. All tests pass (`cargo test`)
3. No new unsafe blocks without justification
4. Public API has doc comments
5. Feature-gated code has appropriate cfg attributes
6. No hardcoded secrets or paths
7. Real config.json fixtures from Python nanobot deserialize correctly

### 🧪 Test Fixtures Required
Copy these from existing Python nanobot installation:
- `~/.nanobot/config.json` -- test config deserialization
- Test cases for config discovery algorithm (env var, ~/.clawft/, ~/.nanobot/ fallback)
- Mock `InboundMessage` / `OutboundMessage` for event type validation

### 🔗 Dependencies
- **Week 1**: No external blockers (workspace setup is independent)
- **Week 2**: Stream 1B depends on types API being stable
- **Week 3**: Stream 1C depends on platform traits + Channel trait

### 📝 Branching Strategy
1. Create branch `weft/types` from `main`
2. All work happens on `weft/types`
3. Merge to `weft/phase-1` when Week 3 validation passes
4. Do NOT merge to `main` until full Phase 1 milestone passes

---

## 1. Specification

### 1.1 Workspace Setup

**Purpose**: Initialize the Rust workspace with MSRV 1.85+, edition 2024, proper build flags for native and WASM targets.

**Requirements**:
- `Cargo.toml` workspace root with 8 member crates (clawft-types, clawft-platform, clawft-core, clawft-tools, clawft-channels, clawft-services, clawft-cli, clawft-wasm)
- `rust-toolchain.toml` pinning Rust 1.85 or later
- `.cargo/config.toml` with:
  - Default target for WASM builds
  - Release profile with `opt-level = "z"`, `lto = true`, `strip = true`
  - WASM-specific profile `release-wasm` with size optimizations
- Workspace dependencies for common crates (serde, tokio, reqwest, etc.)

**Constraints**:
- MSRV 1.85+ (edition 2024)
- All workspace deps must be MIT/Apache-2.0 compatible
- Default features MUST include only `channel-telegram` + `all-tools` (ruvector is opt-in)

**Validation**:
- `cargo build --workspace` compiles empty crates
- `cargo clippy --workspace` passes
- `cargo build --target wasm32-wasip2 -p clawft-types` succeeds (types crate is WASM-safe)

---

### 1.2 clawft-types: Config Structs (Week 1)

**Purpose**: Port all Pydantic config models from `nanobot/config/schema.py` to Rust structs with serde serialization.

**Requirements**:

#### Core Config Struct
Port the main `Config` model:
```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub pipelines: Option<HashMap<String, PipelineConfig>>,
}
```

#### Sub-Configs
All 20+ config models from `schema.py`:

| Model | Python Source | Required Fields | Optional Fields |
|-------|--------------|-----------------|----------------|
| `AgentsConfig` | `schema.py:Agent` | `model` (with default) | `system_prompt`, `context_window`, `memory_consolidation`, `skills_dir`, `workspace_dir` |
| `ChannelsConfig` | `schema.py:Channels` | none | `telegram`, `slack`, `discord`, `extra` (flattened HashMap) |
| `ProvidersConfig` | `schema.py:Providers` | `providers` (HashMap) | `default_provider`, `default_model`, `failover_strategy` |
| `ProviderEntry` | `schema.py:ProviderConfig` | none (all optional) | `api_key`, `api_key_env`, `base_url`, `headers`, `model_aliases`, `enabled` |
| `GatewayConfig` | `schema.py:Gateway` | none | `host`, `port`, `enabled_channels` |
| `ToolsConfig` | `schema.py:Tools` | none | `exec_enabled`, `exec_timeout_secs`, `allowed_paths`, `web_search_api_key`, `mcp_servers` |
| `PipelineConfig` | (future, RVF) | `stages` | `classifier`, `router`, `context`, `transport`, `scorer`, `learner` |

#### Key Design Decisions
1. **Default values**: All optional fields get `#[serde(default)]` to match Python's `Field(default=...)` behavior
2. **Naming convention**: Use snake_case (Rust convention) but support camelCase deserialization where config.json uses it via `#[serde(rename_all = "camelCase")]` on specific structs
3. **Extensibility**: `ChannelsConfig` has `#[serde(flatten)] extra: HashMap<String, serde_json::Value>` to support future channel plugins not known at compile time
4. **Path types**: Use `PathBuf` for file paths, not `String`
5. **Validation**: Config deserialization must NOT fail on unknown fields (use `#[serde(default)]` liberally)

**Validation**:
- Deserialize real `~/.nanobot/config.json` files from test fixtures
- Round-trip test: serialize -> deserialize -> compare (must be identical)
- Test with missing optional fields (should use defaults)
- Test with unknown fields (should ignore, not error)

---

### 1.3 clawft-types: Event Types (Week 1)

**Purpose**: Port message event types from `nanobot/agent/events.py` for the message bus.

**Requirements**:

```rust
/// Inbound message from a channel (user input)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InboundMessage {
    pub id: String,                    // Unique message ID
    pub channel: String,               // Channel name (e.g., "telegram", "cli")
    pub user_id: String,               // User identifier (channel-specific)
    pub text: String,                  // Message text content
    pub metadata: HashMap<String, serde_json::Value>,  // Channel-specific metadata
    pub timestamp: DateTime<Utc>,      // Message timestamp
    #[serde(default)]
    pub session_id: Option<String>,    // Session ID (for multi-turn conversations)
    #[serde(default)]
    pub thread_id: Option<String>,     // Thread ID (for threaded channels)
}

/// Outbound message to a channel (agent response)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutboundMessage {
    pub id: String,                    // Response message ID
    pub channel: String,               // Target channel
    pub user_id: String,               // Target user
    pub text: String,                  // Response text (markdown)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,  // Channel-specific metadata
    #[serde(default)]
    pub in_reply_to: Option<String>,   // Original message ID
    #[serde(default)]
    pub thread_id: Option<String>,     // Thread ID
}
```

**Design Decisions**:
1. All fields are owned (not borrowed) for message bus `Send + Sync` requirements
2. `metadata` is an escape hatch for channel-specific data (e.g., Telegram's `message_id`, Slack's `ts`)
3. IDs are strings (not UUIDs) to support channel-native IDs

**Validation**:
- Serialize/deserialize round-trip
- Test with metadata containing nested JSON values
- Test with missing optional fields

---

### 1.4 clawft-types: Provider Types (Week 1)

**Purpose**: Port provider types from `nanobot/providers/base.py` and registry from `nanobot/providers/registry.py`.

**Requirements**:

```rust
/// LLM response from any provider
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,    // Text or tool use blocks
    pub stop_reason: StopReason,
    pub usage: Usage,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Content block (text or tool use)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

/// Why the LLM stopped generating
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}

/// Token usage stats
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Tool call request from the LLM
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Provider specification (from registry.py)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProviderSpec {
    pub name: String,                  // e.g., "anthropic", "openai"
    pub display_name: String,          // e.g., "Anthropic Claude"
    pub api_base: String,              // Default API base URL
    pub models: Vec<String>,           // Supported model names
    pub supports_streaming: bool,
    pub supports_tool_calling: bool,
    pub supports_vision: bool,
}

/// Static provider registry (from registry.py PROVIDERS list)
pub static PROVIDERS: &[ProviderSpec] = &[
    // Anthropic
    ProviderSpec {
        name: "anthropic",
        display_name: "Anthropic Claude",
        api_base: "https://api.anthropic.com/v1",
        models: &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022", "claude-3-opus-20240229"],
        supports_streaming: true,
        supports_tool_calling: true,
        supports_vision: true,
    },
    // OpenAI
    ProviderSpec {
        name: "openai",
        display_name: "OpenAI GPT",
        api_base: "https://api.openai.com/v1",
        models: &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
        supports_streaming: true,
        supports_tool_calling: true,
        supports_vision: true,
    },
    // ... (add all 14 providers from registry.py)
];
```

**Design Decisions**:
1. `ContentBlock` is an enum (tagged union) matching Anthropic's format (most expressive)
2. `PROVIDERS` is a static array for zero-cost registry lookups
3. Provider types are provider-agnostic (no Anthropic-specific or OpenAI-specific fields)
4. `LlmResponse` is the unified response type; clawft-llm providers adapt their wire formats to this

**Validation**:
- Registry contains all 14 providers from Python `registry.py`
- Each provider has correct `api_base` and capability flags
- Serialize/deserialize round-trip for all types

---

### 1.5 clawft-types: Session and Cron Types (Week 2)

**Purpose**: Port session and cron types for persistence and scheduling.

**Requirements**:

```rust
/// Session (conversation context)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub id: String,
    pub channel: String,
    pub user_id: String,
    pub turns: Vec<SessionTurn>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Single turn in a session
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionTurn {
    pub user_message: String,
    pub assistant_message: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
}

/// Cron job
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: CronSchedule,
    pub prompt: String,               // LLM prompt to execute
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
}

/// Cron schedule (uses cron syntax)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CronSchedule {
    pub expression: String,           // e.g., "0 0 * * *" (daily at midnight)
}
```

**Design Decisions**:
1. Sessions are append-only (turns are never modified after creation)
2. Session JSONL format (one JSON object per line) is handled by SessionManager in clawft-core, not in types
3. CronSchedule is just a string wrapper for now; actual parsing happens in clawft-services

**Validation**:
- Deserialize real session.jsonl files from `~/.nanobot/sessions/`
- Round-trip test for all types

---

### 1.6 clawft-types: Error Types (Week 1)

**Purpose**: Comprehensive error hierarchy for all clawft crates.

**Requirements**:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClawftError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found")]
    NotFound,

    #[error("Failed to parse config: {0}")]
    ParseError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Provider {provider} not found")]
    NotFound { provider: String },

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Request timeout")]
    Timeout,
}

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("Channel {channel} not found")]
    NotFound { channel: String },

    #[error("Failed to send message: {0}")]
    SendFailed(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),
}

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool {tool} not found")]
    NotFound { tool: String },

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

**Design Decisions**:
1. Use `thiserror` for ergonomic error derives
2. All errors are `Send + Sync` for async boundaries
3. Provider errors capture HTTP status codes for retry logic
4. Tool errors distinguish between "not found" (bug) and "execution failed" (user error)

**Validation**:
- All error types implement `Error`, `Debug`, `Display`
- Error conversion via `#[from]` works (`std::io::Error` -> `ClawftError::Io`)

---

### 1.7 clawft-platform: Trait Definitions (Week 2)

**Purpose**: Platform abstraction layer (PAL) for I/O, HTTP, environment, and process spawning. Enables WASM portability and testability.

**Requirements**:

#### HttpClient Trait
```rust
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait HttpClient {
    /// POST JSON data
    async fn post_json(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, ClawftError>;

    /// GET request
    async fn get(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, ClawftError>;

    /// POST multipart form data
    async fn post_form(
        &self,
        url: &str,
        form: &MultipartForm,
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, ClawftError>;
}

pub struct MultipartForm {
    pub fields: Vec<(String, String)>,
    pub files: Vec<(String, String, Vec<u8>)>,  // (field_name, filename, data)
}
```

**Design Notes**:
- `?Send` bound allows WASM (single-threaded) implementations
- Takes `&[u8]` instead of `&str` for binary-safe uploads
- Returns `Vec<u8>` instead of `String` for binary responses
- Headers are tuples (not HashMap) for ordered iteration

#### FileSystem Trait
```rust
use std::path::{Path, PathBuf};

pub trait FileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, ClawftError>;
    fn write_string(&self, path: &Path, content: &str) -> Result<(), ClawftError>;
    fn append_string(&self, path: &Path, content: &str) -> Result<(), ClawftError>;
    fn exists(&self, path: &Path) -> bool;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ClawftError>;
    fn create_dir_all(&self, path: &Path) -> Result<(), ClawftError>;
    fn remove_file(&self, path: &Path) -> Result<(), ClawftError>;

    /// Glob pattern matching (e.g., "*.md")
    fn glob(&self, base: &Path, pattern: &str) -> Result<Vec<PathBuf>, ClawftError>;
}
```

**Design Notes**:
- All paths are `&Path` (not `&str`) for proper path handling
- No async (file I/O is fast enough that blocking is fine)
- `glob()` is essential for skill discovery

#### Environment Trait
```rust
use chrono::{DateTime, Utc};

pub trait Environment {
    fn var(&self, key: &str) -> Option<String>;
    fn home_dir(&self) -> Option<PathBuf>;
    fn current_dir(&self) -> Result<PathBuf, ClawftError>;
    fn now(&self) -> DateTime<Utc>;
    fn platform(&self) -> &str;  // "linux", "macos", "windows", "wasm32"
}
```

#### ProcessSpawner Trait
```rust
use std::time::Duration;

#[async_trait]
pub trait ProcessSpawner {
    async fn exec(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        timeout: Duration,
    ) -> Result<ProcessOutput, ClawftError>;
}

pub struct ProcessOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}
```

**Design Notes**:
- Timeout is mandatory (no infinite blocking)
- Working directory is explicit (no implicit `std::env::current_dir()`)

#### Platform Trait Bundle
```rust
pub trait Platform: HttpClient + FileSystem + Environment + Send + Sync + 'static {}

// Blanket impl: any type implementing all 4 traits is a Platform
impl<T> Platform for T where T: HttpClient + FileSystem + Environment + Send + Sync + 'static {}
```

**Design Notes**:
- `ProcessSpawner` is NOT in the bundle (it's optional, only for native targets)
- `Send + Sync + 'static` required for async/multi-threaded use

**Validation**:
- All traits compile
- Platform trait bundle impl compiles
- Mock implementations for testing work

---

### 1.8 clawft-platform: Native Implementations (Week 3)

**Purpose**: Implement platform traits using standard Rust libraries.

**Requirements**:

#### NativeHttpClient
```rust
use reqwest::Client;

pub struct NativeHttpClient {
    client: Client,
}

impl NativeHttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .pool_idle_timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

#[async_trait]
impl HttpClient for NativeHttpClient {
    async fn post_json(&self, url: &str, body: &[u8], headers: &[(&str, &str)]) -> Result<Vec<u8>, ClawftError> {
        let mut req = self.client.post(url).body(body.to_vec());
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let resp = req.send().await.map_err(|e| /* convert to ClawftError */)?;
        let bytes = resp.bytes().await.map_err(|e| /* convert */)?;
        Ok(bytes.to_vec())
    }
    // ... implement get, post_form
}
```

**Design Notes**:
- Use `reqwest::Client` with connection pooling
- Timeout is 60 seconds by default
- rustls for TLS (not native-tls, to support WASM future)

#### NativeFileSystem
```rust
pub struct NativeFileSystem;

impl FileSystem for NativeFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, ClawftError> {
        std::fs::read_to_string(path).map_err(Into::into)
    }

    fn glob(&self, base: &Path, pattern: &str) -> Result<Vec<PathBuf>, ClawftError> {
        // Use glob crate
        let pattern_str = base.join(pattern).to_string_lossy().into_owned();
        let paths = glob::glob(&pattern_str)
            .map_err(|e| /* convert */)?
            .filter_map(Result::ok)
            .collect();
        Ok(paths)
    }
    // ... implement other methods
}
```

#### NativeEnvironment
```rust
pub struct NativeEnvironment;

impl Environment for NativeEnvironment {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn platform(&self) -> &str {
        #[cfg(target_os = "linux")] { "linux" }
        #[cfg(target_os = "macos")] { "macos" }
        #[cfg(target_os = "windows")] { "windows" }
        #[cfg(target_arch = "wasm32")] { "wasm32" }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows", target_arch = "wasm32")))]
        { "unknown" }
    }
    // ...
}
```

#### NativeProcessSpawner
```rust
use tokio::process::Command;
use tokio::time::timeout;

pub struct NativeProcessSpawner;

#[async_trait]
impl ProcessSpawner for NativeProcessSpawner {
    async fn exec(&self, cmd: &str, args: &[&str], cwd: &Path, timeout_dur: Duration) -> Result<ProcessOutput, ClawftError> {
        let mut child = Command::new(cmd)
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| /* convert */)?;

        let output = timeout(timeout_dur, child.wait_with_output())
            .await
            .map_err(|_| ClawftError::Timeout)?
            .map_err(|e| /* convert */)?;

        Ok(ProcessOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
```

**Validation**:
- HTTP client makes real requests (test against httpbin.org)
- FileSystem reads/writes real files in temp directory
- Environment returns correct values for home_dir, platform
- ProcessSpawner runs `echo hello` and captures stdout

---

### 1.9 clawft-platform: Config Loader (Week 3)

**Purpose**: Implement config discovery and loading algorithm.

**Requirements**:

```rust
pub fn load_config<E: Environment, F: FileSystem>(
    env: &E,
    fs: &F,
) -> Result<Config, ClawftError> {
    // 1. Check $CLAWFT_CONFIG env var
    if let Some(path_str) = env.var("CLAWFT_CONFIG") {
        let path = Path::new(&path_str);
        if fs.exists(path) {
            return load_config_from_path(fs, path);
        }
    }

    // 2. Check ~/.clawft/config.json
    if let Some(home) = env.home_dir() {
        let clawft_config = home.join(".clawft").join("config.json");
        if fs.exists(&clawft_config) {
            return load_config_from_path(fs, &clawft_config);
        }

        // 3. Fallback to ~/.nanobot/config.json (Python compatibility)
        let nanobot_config = home.join(".nanobot").join("config.json");
        if fs.exists(&nanobot_config) {
            return load_config_from_path(fs, &nanobot_config);
        }
    }

    // 4. No config found, use defaults
    Ok(Config::default())
}

fn load_config_from_path<F: FileSystem>(
    fs: &F,
    path: &Path,
) -> Result<Config, ClawftError> {
    let contents = fs.read_to_string(path)?;
    let config: Config = serde_json::from_str(&contents)
        .map_err(|e| ConfigError::ParseError(e.to_string()))?;
    Ok(config)
}
```

**Design Notes**:
- Algorithm matches Python nanobot exactly (CLAWFT_CONFIG -> ~/.clawft -> ~/.nanobot -> defaults)
- Returns `Config::default()` if no file found (not an error)
- Environment variable takes absolute precedence

**Validation**:
- Test with $CLAWFT_CONFIG set to a valid path
- Test with ~/.clawft/config.json existing
- Test with ~/.nanobot/config.json existing (and ~/.clawft absent)
- Test with no config file (should return defaults)
- Test with invalid JSON (should return ConfigError::ParseError)

---

### 1.10 clawft-channels: Channel Trait (Week 2)

**Purpose**: Define the plugin API for channel implementations.

**Requirements**:

```rust
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

/// The trait every channel plugin must implement
#[async_trait]
pub trait Channel: Send + Sync {
    /// Unique channel identifier (e.g., "telegram", "slack", "discord")
    fn name(&self) -> &str;

    /// Start receiving messages. Runs until cancellation.
    async fn start(
        &self,
        host: Arc<dyn ChannelHost>,
        cancel: CancellationToken,
    ) -> Result<(), ChannelError>;

    /// Send an outbound message to the channel
    async fn send(&self, msg: &OutboundMessage) -> Result<(), ChannelError>;

    /// Whether the channel is currently connected and running
    fn is_running(&self) -> bool;
}
```

**Design Notes**:
- `start()` is long-lived (runs until `cancel` is triggered)
- `send()` is one-shot (send a message and return)
- `is_running()` is non-blocking status check
- Host is passed to `start()` (not constructor) to allow dependency injection

> **Note on error types**: The `02-technical-requirements.md` uses shorthand `Result<()>` for these traits. This plan specifies the concrete error types (`Result<(), ChannelError>` for Channel/ChannelHost, `Result<(), ClawftError>` for Platform traits). The concrete types are the canonical definition for implementation. All channel-related methods use `ChannelError`, all platform-related methods use `ClawftError`.

#### ChannelHost Trait
```rust
/// Services the plugin host exposes to channel plugins
#[async_trait]
pub trait ChannelHost: Send + Sync {
    /// Deliver an inbound message to the agent pipeline
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError>;

    /// Get agent configuration
    fn config(&self) -> &Config;

    /// Get the HTTP client for making API calls
    fn http(&self) -> &dyn HttpClient;
}
```

**Design Notes**:
- `deliver_inbound()` is the only way for plugins to send messages to the agent
- `config()` gives plugins access to their channel-specific config section
- `http()` allows plugins to make API calls without bringing their own HTTP client

#### ChannelFactory Trait
```rust
/// Factory for creating channel instances from config
pub trait ChannelFactory: Send + Sync {
    /// The channel name this factory creates
    fn channel_name(&self) -> &str;

    /// Create a channel instance from a JSON config value
    fn build(&self, config: serde_json::Value) -> Result<Box<dyn Channel>, ChannelError>;
}
```

**Design Notes**:
- Factory pattern decouples plugin discovery from instantiation
- `config` is `serde_json::Value` for maximum flexibility (each channel has different config schema)
- Returns `Box<dyn Channel>` for dynamic dispatch

**Validation**:
- Mock channel implementation compiles
- Mock host implementation compiles
- Channel can be started and stopped via CancellationToken

---

### 1.11 clawft-channels: Plugin Host (Week 3)

**Purpose**: Manage lifecycle of channel plugins (discovery, instantiation, start/stop, message routing).

**Requirements**:

```rust
pub struct PluginHost<P: Platform> {
    config: Arc<Config>,
    platform: Arc<P>,
    channels: HashMap<String, Box<dyn Channel>>,
    cancel_tokens: HashMap<String, CancellationToken>,
    bus: Arc<MessageBus>,  // From clawft-core (not yet implemented)
}

impl<P: Platform> PluginHost<P> {
    /// Create a new plugin host
    pub fn new(config: Arc<Config>, platform: Arc<P>, bus: Arc<MessageBus>) -> Self {
        Self {
            config,
            platform,
            channels: HashMap::new(),
            cancel_tokens: HashMap::new(),
            bus,
        }
    }

    /// Discover and instantiate all enabled channel plugins
    pub fn load_plugins(&mut self) -> Result<(), ChannelError> {
        let factories = available_channels();  // From registry
        for factory in factories {
            let channel_name = factory.channel_name();

            // Extract channel-specific config from Config.channels
            let channel_config = extract_channel_config(&self.config.channels, channel_name)?;

            // Build the channel
            let channel = factory.build(channel_config)?;
            self.channels.insert(channel_name.to_string(), channel);
        }
        Ok(())
    }

    /// Start all loaded channels
    pub async fn start_all(&mut self) -> Result<(), ChannelError> {
        for (name, channel) in &self.channels {
            let cancel = CancellationToken::new();
            self.cancel_tokens.insert(name.clone(), cancel.clone());

            let host = Arc::new(HostImpl {
                config: self.config.clone(),
                platform: self.platform.clone(),
                bus: self.bus.clone(),
            });

            // Spawn channel in background task
            let channel_ref = /* ... need Arc<dyn Channel> ... */;
            tokio::spawn(async move {
                if let Err(e) = channel_ref.start(host, cancel).await {
                    tracing::error!("Channel {} crashed: {}", name, e);
                }
            });
        }
        Ok(())
    }

    /// Stop all channels
    pub async fn stop_all(&mut self) {
        for token in self.cancel_tokens.values() {
            token.cancel();
        }
        // Wait for all tasks to exit
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    /// Route an outbound message to the correct channel
    pub async fn route_outbound(&self, msg: &OutboundMessage) -> Result<(), ChannelError> {
        let channel = self.channels.get(&msg.channel)
            .ok_or_else(|| ChannelError::NotFound { channel: msg.channel.clone() })?;
        channel.send(msg).await
    }
}

/// ChannelHost implementation
struct HostImpl<P: Platform> {
    config: Arc<Config>,
    platform: Arc<P>,
    bus: Arc<MessageBus>,
}

#[async_trait]
impl<P: Platform> ChannelHost for HostImpl<P> {
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError> {
        self.bus.publish_inbound(msg).await.map_err(|e| /* convert */)
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn http(&self) -> &dyn HttpClient {
        self.platform.as_ref()
    }
}
```

**Design Notes**:
- Plugin host owns all channel instances
- Channels run in separate tokio tasks (not threads)
- Cancellation is graceful (via CancellationToken)
- Host implements ChannelHost trait for dependency injection

**Known Issues to Resolve**:
1. `Arc<dyn Channel>` needed for spawning (dyn trait is not Clone)
   - Solution: Channels must be `Arc<dyn Channel>` from the start
2. MessageBus from clawft-core not yet available
   - Solution: Use a mock MessageBus for testing in Week 3

**Validation**:
- Load mock channel plugins
- Start and stop plugins via CancellationToken
- Route outbound messages to correct plugin by channel name

---

## 2. Pseudocode

### 2.1 Config Discovery Algorithm

```
function load_config(env: Environment, fs: FileSystem) -> Config:
    # Step 1: Check environment variable
    if env.var("CLAWFT_CONFIG") exists:
        path = env.var("CLAWFT_CONFIG")
        if fs.exists(path):
            return parse_config_file(fs, path)
        else:
            log_warning("CLAWFT_CONFIG points to non-existent file")

    # Step 2: Check ~/.clawft/config.json
    home = env.home_dir()
    if home is not None:
        clawft_path = home / ".clawft" / "config.json"
        if fs.exists(clawft_path):
            return parse_config_file(fs, clawft_path)

        # Step 3: Fallback to ~/.nanobot/config.json
        nanobot_path = home / ".nanobot" / "config.json"
        if fs.exists(nanobot_path):
            return parse_config_file(fs, nanobot_path)

    # Step 4: No config found, use defaults
    log_info("No config file found, using defaults")
    return Config::default()

function parse_config_file(fs: FileSystem, path: Path) -> Config:
    contents = fs.read_to_string(path)
    config = deserialize_json(contents)
    validate_config(config)
    return config
```

### 2.2 Plugin Lifecycle

```
function load_plugins(host: PluginHost) -> Result:
    factories = get_all_channel_factories()

    for factory in factories:
        channel_name = factory.channel_name()

        # Extract channel-specific config
        if channel_name in host.config.channels:
            channel_config = host.config.channels[channel_name]
        else:
            log_debug("No config for channel {}", channel_name)
            continue

        # Build the channel
        channel = factory.build(channel_config)
        host.channels[channel_name] = channel

    return Ok

function start_all_channels(host: PluginHost) -> Result:
    for (name, channel) in host.channels:
        cancel_token = create_cancellation_token()
        host.cancel_tokens[name] = cancel_token

        # Create host implementation for dependency injection
        channel_host = HostImpl::new(host.config, host.platform, host.bus)

        # Spawn channel in background task
        spawn_task:
            result = channel.start(channel_host, cancel_token).await
            if result is Error:
                log_error("Channel {} failed: {}", name, result)
                # Retry with exponential backoff
                backoff_and_retry(channel, channel_host, cancel_token)

    return Ok

function route_outbound_message(host: PluginHost, msg: OutboundMessage) -> Result:
    if msg.channel not in host.channels:
        return Error::ChannelNotFound

    channel = host.channels[msg.channel]
    return channel.send(msg).await
```

### 2.3 Type Deserialization

```
# Example: Config deserialization with defaults
function deserialize_config(json_str: String) -> Config:
    # serde handles this, but conceptually:
    data = parse_json(json_str)

    config = Config {
        agents: if "agents" in data then deserialize(data["agents"]) else AgentsConfig::default(),
        channels: if "channels" in data then deserialize(data["channels"]) else ChannelsConfig::default(),
        providers: if "providers" in data then deserialize(data["providers"]) else ProvidersConfig::default(),
        gateway: if "gateway" in data then deserialize(data["gateway"]) else GatewayConfig::default(),
        tools: if "tools" in data then deserialize(data["tools"]) else ToolsConfig::default(),
        pipelines: if "pipelines" in data then Some(deserialize(data["pipelines"])) else None,
    }

    return config

# Example: ChannelsConfig with flattened extras
function deserialize_channels_config(data: JsonObject) -> ChannelsConfig:
    config = ChannelsConfig::default()

    if "telegram" in data:
        config.telegram = Some(data["telegram"])
    if "slack" in data:
        config.slack = Some(data["slack"])
    if "discord" in data:
        config.discord = Some(data["discord"])

    # Flatten all other keys into extras
    for (key, value) in data:
        if key not in ["telegram", "slack", "discord"]:
            config.extra[key] = value

    return config
```

---

## 3. Architecture

### 3.1 Crate Dependency Graph

```
clawft-types (zero deps beyond serde, chrono, thiserror)
    |
    v
clawft-platform (depends on: types, reqwest, tokio, glob, dirs)
    |
    v
clawft-channels (depends on: types, platform, tokio-util)
    |
    v
[Future streams will depend on all three]
```

**Key Design Decisions**:
1. **clawft-types is the foundation** -- zero non-serialization dependencies
2. **clawft-platform wraps external I/O libraries** -- provides trait-based abstraction
3. **clawft-channels owns plugin architecture** -- depends on platform for HTTP client

### 3.2 Trait Relationships

```
Platform Trait Bundle
├── HttpClient (async)
├── FileSystem (sync)
├── Environment (sync)
└── [ProcessSpawner (async, optional)]

Channel Plugin System
├── Channel (implemented by plugins)
├── ChannelHost (implemented by host, consumed by plugins)
└── ChannelFactory (implemented by plugins, consumed by host)
```

**Ownership Model**:
- **PluginHost owns** all `Box<dyn Channel>` instances
- **Channels receive** `Arc<dyn ChannelHost>` on start (dependency injection)
- **Factories are stateless** (no state, just constructors)

### 3.3 Module Organization

**clawft-types/src/**:
```
lib.rs              # Re-exports: pub use config::*; pub use event::*; etc.
config.rs           # Config, AgentsConfig, ProvidersConfig, etc. (~400 lines)
event.rs            # InboundMessage, OutboundMessage (~100 lines)
provider.rs         # LlmResponse, ProviderSpec, PROVIDERS (~300 lines)
session.rs          # Session, SessionTurn (~150 lines)
cron.rs             # CronJob, CronSchedule (~100 lines)
error.rs            # ClawftError, ConfigError, etc. (~200 lines)
```

**clawft-platform/src/**:
```
lib.rs              # Trait definitions + Platform bundle
http.rs             # HttpClient trait + NativeHttpClient (~200 lines)
fs.rs               # FileSystem trait + NativeFileSystem (~150 lines)
env.rs              # Environment trait + NativeEnvironment (~100 lines)
process.rs          # ProcessSpawner trait + NativeProcessSpawner (~100 lines)
config_loader.rs    # load_config() algorithm (~100 lines)
```

**clawft-channels/src/**:
```
lib.rs              # Re-exports
trait.rs            # Channel, ChannelHost, ChannelFactory traits (~150 lines)
host.rs             # PluginHost implementation (~300 lines)
registry.rs         # available_channels() function (~50 lines)
telegram.rs         # Placeholder (implementation in Stream 1C)
```

**Total LOC Estimate**:
- clawft-types: ~1,250 lines
- clawft-platform: ~650 lines
- clawft-channels: ~500 lines
- **Total: ~2,400 lines** (3 weeks is reasonable)

---

## 4. Refinement (TDD Test Plan)

### 4.1 Test Strategy

**Levels**:
1. **Unit tests**: Each crate tests its own public API
2. **Integration tests**: Cross-crate interactions (e.g., load_config using NativeFileSystem)
3. **Fixture tests**: Real config.json files from Python nanobot

**Test Fixtures Required**:
- `tests/fixtures/config.json` -- real Python nanobot config
- `tests/fixtures/minimal_config.json` -- minimal valid config
- `tests/fixtures/invalid_config.json` -- malformed JSON
- `tests/fixtures/session.jsonl` -- real session file

### 4.2 clawft-types Tests

#### Config Deserialization Tests
```rust
#[test]
fn test_deserialize_real_config() {
    let json = std::fs::read_to_string("tests/fixtures/config.json").unwrap();
    let config: Config = serde_json::from_str(&json).unwrap();
    assert!(config.providers.providers.contains_key("anthropic"));
}

#[test]
fn test_deserialize_minimal_config() {
    let json = r#"{}"#;
    let config: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(config.agents.model, default_model());  // Uses default
}

#[test]
fn test_deserialize_unknown_fields_ignored() {
    let json = r#"{"unknown_field": "value", "agents": {}}"#;
    let config: Config = serde_json::from_str(&json).unwrap();  // Should not error
}

#[test]
fn test_channels_config_flattened_extras() {
    let json = r#"{"telegram": {}, "custom_channel": {"key": "value"}}"#;
    let channels: ChannelsConfig = serde_json::from_str(&json).unwrap();
    assert!(channels.telegram.is_some());
    assert!(channels.extra.contains_key("custom_channel"));
}
```

#### Provider Registry Tests
```rust
#[test]
fn test_provider_registry_contains_all_providers() {
    assert_eq!(PROVIDERS.len(), 14);  // Python registry has 14
    assert!(PROVIDERS.iter().any(|p| p.name == "anthropic"));
    assert!(PROVIDERS.iter().any(|p| p.name == "openai"));
}

#[test]
fn test_provider_spec_capabilities() {
    let anthropic = PROVIDERS.iter().find(|p| p.name == "anthropic").unwrap();
    assert!(anthropic.supports_streaming);
    assert!(anthropic.supports_tool_calling);
    assert!(anthropic.supports_vision);
}
```

#### Event Type Tests
```rust
#[test]
fn test_inbound_message_round_trip() {
    let msg = InboundMessage {
        id: "msg-123".to_string(),
        channel: "telegram".to_string(),
        user_id: "user-456".to_string(),
        text: "hello".to_string(),
        metadata: HashMap::new(),
        timestamp: Utc::now(),
        session_id: None,
        thread_id: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: InboundMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg.id, deserialized.id);
}
```

#### Error Type Tests
```rust
#[test]
fn test_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let clawft_err: ClawftError = io_err.into();
    assert!(matches!(clawft_err, ClawftError::Io(_)));
}

#[test]
fn test_provider_error_display() {
    let err = ProviderError::ApiError { status: 429, message: "Rate limited".to_string() };
    let display = format!("{}", err);
    assert!(display.contains("429"));
    assert!(display.contains("Rate limited"));
}
```

### 4.3 clawft-platform Tests

#### HttpClient Tests
```rust
#[tokio::test]
async fn test_http_get_success() {
    let client = NativeHttpClient::new();
    let resp = client.get("https://httpbin.org/get", &[]).await.unwrap();
    assert!(!resp.is_empty());
}

#[tokio::test]
async fn test_http_post_json() {
    let client = NativeHttpClient::new();
    let body = r#"{"key": "value"}"#.as_bytes();
    let headers = &[("content-type", "application/json")];
    let resp = client.post_json("https://httpbin.org/post", body, headers).await.unwrap();
    assert!(!resp.is_empty());
}
```

#### FileSystem Tests
```rust
#[test]
fn test_fs_read_write() {
    let fs = NativeFileSystem;
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("clawft_test.txt");

    fs.write_string(&test_file, "hello").unwrap();
    assert!(fs.exists(&test_file));

    let contents = fs.read_to_string(&test_file).unwrap();
    assert_eq!(contents, "hello");

    fs.remove_file(&test_file).unwrap();
}

#[test]
fn test_fs_glob() {
    let fs = NativeFileSystem;
    let temp_dir = std::env::temp_dir();
    fs.write_string(&temp_dir.join("test1.md"), "").unwrap();
    fs.write_string(&temp_dir.join("test2.md"), "").unwrap();

    let matches = fs.glob(&temp_dir, "*.md").unwrap();
    assert!(matches.len() >= 2);
}
```

#### Environment Tests
```rust
#[test]
fn test_env_var() {
    let env = NativeEnvironment;
    std::env::set_var("CLAWFT_TEST", "value");
    assert_eq!(env.var("CLAWFT_TEST"), Some("value".to_string()));
}

#[test]
fn test_env_platform() {
    let env = NativeEnvironment;
    let platform = env.platform();
    assert!(["linux", "macos", "windows"].contains(&platform));
}
```

#### Config Loader Tests
```rust
#[test]
fn test_load_config_from_env_var() {
    let env = MockEnvironment::with_var("CLAWFT_CONFIG", "/path/to/config.json");
    let fs = MockFileSystem::with_file("/path/to/config.json", r#"{"agents": {}}"#);

    let config = load_config(&env, &fs).unwrap();
    assert!(config.agents.model == default_model());
}

#[test]
fn test_load_config_fallback_chain() {
    let env = MockEnvironment::with_home("/home/user");
    let fs = MockFileSystem::with_file("/home/user/.nanobot/config.json", r#"{}"#);

    // No CLAWFT_CONFIG, no ~/.clawft/config.json, should fallback to ~/.nanobot/
    let config = load_config(&env, &fs).unwrap();
    assert!(config.agents.model == default_model());
}

#[test]
fn test_load_config_no_file_returns_defaults() {
    let env = MockEnvironment::default();
    let fs = MockFileSystem::empty();

    let config = load_config(&env, &fs).unwrap();
    assert_eq!(config, Config::default());
}
```

### 4.4 clawft-channels Tests

#### Channel Trait Tests
```rust
struct MockChannel {
    running: AtomicBool,
}

#[async_trait]
impl Channel for MockChannel {
    fn name(&self) -> &str { "mock" }

    async fn start(&self, host: Arc<dyn ChannelHost>, cancel: CancellationToken) -> Result<(), ChannelError> {
        self.running.store(true, Ordering::SeqCst);
        cancel.cancelled().await;  // Wait for cancellation
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<(), ChannelError> {
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn test_channel_start_stop() {
    let channel = Arc::new(MockChannel { running: AtomicBool::new(false) });
    let host = Arc::new(MockChannelHost::default());
    let cancel = CancellationToken::new();

    let channel_clone = channel.clone();
    let cancel_clone = cancel.clone();
    let task = tokio::spawn(async move {
        channel_clone.start(host, cancel_clone).await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(channel.is_running());

    cancel.cancel();
    task.await.unwrap().unwrap();
    assert!(!channel.is_running());
}
```

#### Plugin Host Tests
```rust
#[tokio::test]
async fn test_plugin_host_load_plugins() {
    let config = Arc::new(Config::default());
    let platform = Arc::new(NativePlatform::new());
    let bus = Arc::new(MockMessageBus::new());

    let mut host = PluginHost::new(config, platform, bus);
    host.load_plugins().unwrap();

    // Should load all available channels (in this case, just mock)
    assert!(!host.channels.is_empty());
}

#[tokio::test]
async fn test_plugin_host_route_outbound() {
    let config = Arc::new(Config::default());
    let platform = Arc::new(NativePlatform::new());
    let bus = Arc::new(MockMessageBus::new());

    let mut host = PluginHost::new(config, platform, bus);
    host.channels.insert("mock".to_string(), Box::new(MockChannel::default()));

    let msg = OutboundMessage {
        id: "msg-1".to_string(),
        channel: "mock".to_string(),
        user_id: "user-1".to_string(),
        text: "hello".to_string(),
        metadata: HashMap::new(),
        in_reply_to: None,
        thread_id: None,
    };

    host.route_outbound(&msg).await.unwrap();
}

#[tokio::test]
async fn test_plugin_host_channel_not_found() {
    let config = Arc::new(Config::default());
    let platform = Arc::new(NativePlatform::new());
    let bus = Arc::new(MockMessageBus::new());

    let host = PluginHost::new(config, platform, bus);

    let msg = OutboundMessage {
        id: "msg-1".to_string(),
        channel: "nonexistent".to_string(),
        user_id: "user-1".to_string(),
        text: "hello".to_string(),
        metadata: HashMap::new(),
        in_reply_to: None,
        thread_id: None,
    };

    let result = host.route_outbound(&msg).await;
    assert!(matches!(result, Err(ChannelError::NotFound { .. })));
}
```

### 4.5 Mock Implementations for Testing

Create these mocks in a `test_utils` module for reuse:

```rust
// Mock Environment
pub struct MockEnvironment {
    vars: HashMap<String, String>,
    home: Option<PathBuf>,
}

impl Environment for MockEnvironment {
    fn var(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned()
    }
    fn home_dir(&self) -> Option<PathBuf> {
        self.home.clone()
    }
    // ...
}

// Mock FileSystem
pub struct MockFileSystem {
    files: HashMap<PathBuf, String>,
}

impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, ClawftError> {
        self.files.get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into())
    }
    // ...
}

// Mock ChannelHost
pub struct MockChannelHost {
    pub messages: Arc<Mutex<Vec<InboundMessage>>>,
}

#[async_trait]
impl ChannelHost for MockChannelHost {
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError> {
        self.messages.lock().unwrap().push(msg);
        Ok(())
    }
    fn config(&self) -> &Config {
        &Config::default()
    }
    fn http(&self) -> &dyn HttpClient {
        // Return a mock or panic (depends on test needs)
        unimplemented!()
    }
}
```

---

## 5. Completion (Integration & Validation)

### 5.1 Integration Checklist

**End of Week 1**:
- [x] Workspace Cargo.toml compiles all member crates
- [x] rust-toolchain.toml pins Rust 1.85 (via workspace rust-version)
- [x] clawft-types compiles with zero warnings
- [x] Config deserialization works with real config.json
- [x] All error types compile and convert correctly
- [x] Provider registry has all 14 providers

**End of Week 2 (BLOCKER DEADLINE)**:
- [x] clawft-platform trait API is stable (no more signature changes expected)
- [x] Channel, ChannelHost, ChannelFactory traits are stable
- [x] All trait methods have doc comments
- [x] Stream 1B can import and use types/platform crates
- [x] Stream 1C can import and use types/platform/channels crates

**End of Week 3**:
- [x] Native platform implementations work (HTTP, FS, Env, Process)
- [x] Config loader reads config.json with proper fallback chain
- [x] PluginHost manages lifecycle of mock channels
- [x] All unit tests pass (`cargo test --workspace`)
- [ ] All integration tests pass
- [x] No clippy warnings (`cargo clippy --workspace`)
- [x] No unsafe code without documentation

### 5.2 Validation Steps

#### Step 1: Workspace Compilation
```bash
cd repos/nanobot
cargo build --workspace
cargo clippy --workspace
cargo test --workspace
```

**Expected Result**: All commands succeed with zero warnings.

#### Step 2: Config Compatibility Test
```bash
# Copy real Python nanobot config
cp ~/.nanobot/config.json tests/fixtures/

# Run config deserialization test
cargo test --package clawft-types test_deserialize_real_config
```

**Expected Result**: Config deserializes correctly.

#### Step 3: Platform Implementations Test
```bash
# Run HTTP client test (requires internet)
cargo test --package clawft-platform test_http_get_success

# Run filesystem test
cargo test --package clawft-platform test_fs_read_write
```

**Expected Result**: All platform tests pass.

#### Step 4: Channel Plugin System Test
```bash
# Run plugin host tests
cargo test --package clawft-channels test_plugin_host
```

**Expected Result**: Plugin host loads, starts, and stops mock channels.

#### Step 5: WASM Compatibility Check
```bash
# clawft-types MUST compile to WASM
cargo build --target wasm32-wasip2 --package clawft-types
```

**Expected Result**: Types crate compiles to WASM with zero errors.

### 5.3 Documentation Checklist

- [x] All public structs have doc comments
- [x] All public traits have doc comments with usage examples
- [x] All public functions have doc comments
- [x] Config discovery algorithm is documented
- [x] Plugin API has usage examples in doc comments
- [x] Error types have examples of when they're returned

### 5.4 Unblock Downstream Streams

Once Week 2 validation passes:
1. Create git tag `types-api-stable-v1`
2. Notify Stream 1B (Core) that types API is stable
3. Notify Stream 1C (Provider+Tools+CLI) that platform traits are stable
4. Create RFC document for any final API tweaks (to be applied in Week 3 if needed)

### 5.5 Known Issues and Workarounds

**Issue 1**: MessageBus not yet available (Stream 1B)
- **Workaround**: Use `MockMessageBus` in clawft-channels tests

**Issue 2**: Channel plugins are `Box<dyn Channel>` (not Clone)
- **Workaround**: Wrap in `Arc<Mutex<Box<dyn Channel>>>` if needed for task spawning

**Issue 3**: `HttpClient::post_form` needs multipart encoding
- **Workaround**: Use reqwest's multipart feature in NativeHttpClient

### 5.6 Merge to Integration Branch

Once all Week 3 validation passes:
1. Run full test suite: `cargo test --workspace`
2. Run clippy: `cargo clippy --workspace`
3. Check binary size: `cargo build --release` (should be ~0 bytes, just types/platform libs)
4. Push to `weft/types` branch
5. Create PR to `weft/phase-1` with description:
   - "Stream 1A: Types + Platform + Plugin API (Week 1-3)"
   - Checklist of all deliverables
   - API stability guarantees
6. Request review from Stream 1B and 1C owners
7. Merge when approved

---

## Appendix A: Cargo.toml Templates

### Workspace Root Cargo.toml
```toml
[workspace]
resolver = "2"
members = [
    "crates/clawft-types",
    "crates/clawft-platform",
    "crates/clawft-core",
    "crates/clawft-tools",
    "crates/clawft-channels",
    "crates/clawft-services",
    "crates/clawft-cli",
    "crates/clawft-wasm",
]

[workspace.dependencies]
# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async
tokio = { version = "1.40", features = ["rt-multi-thread", "macros", "sync", "time", "process", "signal", "io-util"] }
async-trait = "0.1"
tokio-util = { version = "0.7", features = ["rt"] }

# HTTP
reqwest = { version = "0.12", default-features = false, features = ["json", "stream", "rustls-tls"] }

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
regex = "1.10"
uuid = { version = "1.0", features = ["v4"] }
dirs = "5.0"
glob = "0.3"

# Internal workspace crates
clawft-types = { path = "crates/clawft-types" }
clawft-platform = { path = "crates/clawft-platform" }
clawft-channels = { path = "crates/clawft-channels" }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### clawft-types/Cargo.toml
```toml
[package]
name = "clawft-types"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
# None (types crate has no external test deps)
```

### clawft-platform/Cargo.toml
```toml
[package]
name = "clawft-platform"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
clawft-types = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
dirs = { workspace = true }
glob = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

### clawft-channels/Cargo.toml
```toml
[package]
name = "clawft-channels"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
clawft-types = { workspace = true }
clawft-platform = { workspace = true }
tokio = { workspace = true }
tokio-util = { workspace = true }
async-trait = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

---

## Appendix B: Provider Registry (PROVIDERS Array)

Port all 14 providers from `nanobot/providers/registry.py`:

```rust
pub static PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        name: "anthropic",
        display_name: "Anthropic Claude",
        api_base: "https://api.anthropic.com/v1",
        models: &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022", "claude-3-opus-20240229"],
        supports_streaming: true,
        supports_tool_calling: true,
        supports_vision: true,
    },
    ProviderSpec {
        name: "openai",
        display_name: "OpenAI GPT",
        api_base: "https://api.openai.com/v1",
        models: &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"],
        supports_streaming: true,
        supports_tool_calling: true,
        supports_vision: true,
    },
    ProviderSpec {
        name: "bedrock",
        display_name: "AWS Bedrock",
        api_base: "https://bedrock-runtime.us-east-1.amazonaws.com",
        models: &["anthropic.claude-3-sonnet-20240229-v1:0"],
        supports_streaming: true,
        supports_tool_calling: true,
        supports_vision: false,
    },
    ProviderSpec {
        name: "gemini",
        display_name: "Google Gemini",
        api_base: "https://generativelanguage.googleapis.com/v1beta",
        models: &["gemini-1.5-pro", "gemini-1.5-flash"],
        supports_streaming: true,
        supports_tool_calling: true,
        supports_vision: true,
    },
    // Add remaining 10 providers from registry.py:
    // groq, deepseek, mistral, ollama, perplexity, cohere, together, fireworks, openrouter, xai
];
```

---

## Appendix C: Test Coverage Requirements

| Crate | Minimum Coverage | Critical Paths |
|-------|-----------------|----------------|
| clawft-types | 90% | Config deserialization, error conversion |
| clawft-platform | 85% | HTTP client, FileSystem, config loader |
| clawft-channels | 80% | Plugin lifecycle, message routing |

**Coverage Tools**:
- Use `cargo-tarpaulin` for coverage reports
- CI must fail if coverage drops below threshold

**Critical Test Cases** (MUST pass):
1. Real config.json from Python nanobot deserializes
2. Config discovery fallback chain works correctly
3. Platform traits work with mock implementations
4. Channel plugins start and stop gracefully
5. Error types convert correctly from std::io::Error, serde_json::Error, etc.

---

**End of SPARC Plan**
