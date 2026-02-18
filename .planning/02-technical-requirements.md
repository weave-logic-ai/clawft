# Technical Requirements: clawft

## 1. Workspace Architecture

```
repos/nanobot/
  Cargo.toml                      # Workspace root
  rust-toolchain.toml             # Pin Rust version (1.85+, edition 2024)
  .cargo/config.toml              # Build flags, WASM profile
  crates/
    clawft-types/                 # Zero-dep core types (the yarn)
    clawft-platform/              # Platform abstraction layer (the loom)
    clawft-core/                  # Agent loop, context, memory, skills, bus
    clawft-tools/                 # Built-in tools (feature-gated)
    clawft-channels/              # Channel plugin host + plugin implementations
    clawft-services/              # Cron, heartbeat, MCP client, MCP server (McpServerShell, ToolProvider)
    clawft-cli/                   # Native CLI binary (`weft`)
    clawft-wasm/                  # WASM entrypoint (optional)
  # Existing Python code remains untouched:
  nanobot/                        # Python package
  bridge/                         # Node.js WhatsApp bridge
  pyproject.toml                  # Python project config

# External standalone library (separate repo or sibling dir):
repos/clawft-llm/                 # Standalone LLM provider library
  Cargo.toml                      # Independent crate, publishable to crates.io
  src/
    lib.rs                        # Provider trait, registry, failover, circuit breaker
    provider.rs                   # Provider trait + ProviderRegistry
    openai.rs                     # OpenAI provider (+ generic OpenAI-compat via base_url)
    anthropic.rs                  # Anthropic provider
    bedrock.rs                    # AWS Bedrock provider
    gemini.rs                     # Google Gemini provider
    failover.rs                   # FailoverController (4 strategies)
    circuit_breaker.rs            # Lock-free CircuitBreaker (WASM-safe)
    cost.rs                       # UsageTracker + ModelCatalog
    request.rs                    # CompletionRequest builder
    response.rs                   # CompletionResponse + StreamChunk types
```

## 2. Crate Dependency Graph

```
clawft-llm            (standalone library; depends on: reqwest, serde, async-trait)
                      ↑ external dependency (git or crates.io)

clawft-types          (zero deps beyond serde)
    |
clawft-platform       (depends on: types)
    |
clawft-core           (depends on: types, platform, clawft-llm)
    |
    +-- clawft-tools       (depends on: types, platform, core)
    +-- clawft-channels    (depends on: types, platform, core)  <- plugin host
    +-- clawft-services    (depends on: types, platform, core)
    |
clawft-cli            (depends on: all above) -> binary: `weft`
clawft-wasm           (depends on: types, platform, core, clawft-llm)
```

## 3. Crate Specifications

### clawft-types

**Purpose**: All shared data structures. Zero non-serde dependencies. WASM-safe.

**Contains**:
- `Config`, `ChannelsConfig`, `ProvidersConfig`, `AgentDefaults`, `ToolsConfig` (from `schema.py`)
- `InboundMessage`, `OutboundMessage` (from `events.py`)
- `LlmResponse`, `ToolCallRequest` (from `providers/base.py`)
- `ProviderSpec`, `PROVIDERS` registry (from `registry.py`)
- `Session`, `CronJob`, `CronSchedule` (from `session/manager.py`, `cron/types.py`)
- Error types (`ClawftError`, `ConfigError`, `ProviderError`, `ToolError`, `ChannelError`)

**Key design decisions**:
- `#[derive(Serialize, Deserialize, Clone, Debug)]` on all types
- `#[serde(default)]` on all optional fields to match Python's defaults
- `#[serde(rename_all = "camelCase")]` where config.json uses camelCase
- Provider registry as `static PROVIDERS: &[ProviderSpec]` (const array)

```rust
// Config must read existing Python-generated config.json
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
    /// Pipeline configuration per task type (optional, Level 1+).
    /// See 05-ruvector-crates.md section 11 for pipeline config format.
    #[serde(default)]
    pub pipelines: Option<HashMap<String, PipelineConfig>>,
    /// Tiered routing and permission configuration.
    /// See 08-tiered-router.md for full specification.
    #[serde(default)]
    pub routing: Option<RoutingConfig>,
}

/// Agent-level configuration (system prompt, model, context window, etc.)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AgentsConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_context_window")]
    pub context_window: usize,
    #[serde(default)]
    pub memory_consolidation: bool,
    #[serde(default)]
    pub skills_dir: Option<PathBuf>,
    #[serde(default)]
    pub workspace_dir: Option<PathBuf>,
}

/// Channel configuration -- each key is a channel name, value is channel-specific config.
/// This matches Python nanobot's `channels` dict in config.json.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: Option<serde_json::Value>,
    #[serde(default)]
    pub slack: Option<serde_json::Value>,
    #[serde(default)]
    pub discord: Option<serde_json::Value>,
    /// Additional channels via serde_json::Value for extensibility.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Provider configuration -- how to register and configure LLM providers.
/// Supports both named providers (anthropic, openai, bedrock, gemini)
/// and user-registered OpenAI-compatible endpoints via `custom` entries.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ProvidersConfig {
    /// Named provider configs (key = provider name from ProviderSpec registry).
    /// Each value contains at minimum `api_key` or `api_key_env`.
    #[serde(default)]
    pub providers: HashMap<String, ProviderEntry>,
    /// Default provider to use when no routing decision is made.
    #[serde(default)]
    pub default_provider: Option<String>,
    /// Default model to use (e.g., "anthropic/claude-sonnet-4-20250514").
    #[serde(default)]
    pub default_model: Option<String>,
    /// Failover strategy: "sequential", "weighted_round_robin", "lowest_latency", "cost_optimized"
    #[serde(default)]
    pub failover_strategy: Option<String>,
}

/// A single provider entry in config.json.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProviderEntry {
    /// API key (direct value, prefer api_key_env for security).
    pub api_key: Option<String>,
    /// Environment variable name containing the API key.
    pub api_key_env: Option<String>,
    /// Base URL for OpenAI-compatible providers (e.g., "https://api.groq.com/openai/v1").
    /// If set, this provider is treated as OpenAI-compatible regardless of its name.
    pub base_url: Option<String>,
    /// Custom headers (e.g., for Azure OpenAI: {"api-version": "2024-02-15"}).
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Model aliases for this provider (e.g., {"fast": "llama-3.1-8b"}).
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
    /// Whether this provider is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Gateway configuration -- server-level settings.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GatewayConfig {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    /// Channels to start (if empty, starts all enabled channels).
    #[serde(default)]
    pub enabled_channels: Vec<String>,
}

/// Tools configuration -- enable/disable and configure built-in tools.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ToolsConfig {
    /// Whether shell exec tool is enabled (default: true for native, false for WASM).
    #[serde(default)]
    pub exec_enabled: Option<bool>,
    /// Shell exec timeout in seconds.
    #[serde(default)]
    pub exec_timeout_secs: Option<u64>,
    /// Allowed directories for file tools (empty = workspace only).
    #[serde(default)]
    pub allowed_paths: Vec<PathBuf>,
    /// Web search API key (e.g., Brave Search).
    #[serde(default)]
    pub web_search_api_key: Option<String>,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}
```

### clawft-platform

**Purpose**: Trait-based abstraction for I/O, HTTP, timers, and environment. The loom on which all platform-specific threads are woven -- enables WASM portability.

**Traits**:
```rust
#[async_trait(?Send)]
pub trait HttpClient {
    async fn post_json(&self, url: &str, body: &[u8], headers: &[(&str, &str)]) -> Result<Vec<u8>>;
    async fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>>;
    async fn post_form(&self, url: &str, form: &MultipartForm, headers: &[(&str, &str)]) -> Result<Vec<u8>>;
}

pub trait FileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write_string(&self, path: &Path, content: &str) -> Result<()>;
    fn append_string(&self, path: &Path, content: &str) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn glob(&self, base: &Path, pattern: &str) -> Result<Vec<PathBuf>>;
}

pub trait Environment {
    fn var(&self, key: &str) -> Option<String>;
    fn home_dir(&self) -> Option<PathBuf>;
    fn current_dir(&self) -> Result<PathBuf>;
    fn now(&self) -> DateTime<Utc>;
    fn platform(&self) -> &str;
}

pub trait ProcessSpawner {
    async fn exec(&self, cmd: &str, args: &[&str], cwd: &Path, timeout: Duration) -> Result<ProcessOutput>;
}
```

**Implementations** (compile-time selected via `cfg`):
- `NativeHttpClient` -- wraps `reqwest::Client`
- `NativeFileSystem` -- wraps `std::fs`
- `NativeEnvironment` -- wraps `std::env`, `dirs`, `chrono`
- `NativeProcessSpawner` -- wraps `tokio::process::Command`
- (Future) `WasiHttpClient`, `WasiFileSystem`, `WasiEnvironment`

**Config file discovery algorithm**:
1. Check `$CLAWFT_CONFIG` environment variable (explicit override)
2. Check `~/.clawft/config.json`
3. If none found, use built-in defaults (no providers, no channels)

Workspace directory: `$CLAWFT_WORKSPACE` → `~/.clawft/workspace/`

**Project workspace discovery algorithm** (Phase 3G):
1. Walk from `current_dir()` up to filesystem root, looking for `.clawft/config.json`
2. If found, that directory is the **project root**; `.clawft/` is the **project workspace**
3. If not found, no project context -- global workspace only

The project workspace overlays the global workspace. See Section 10 for full hierarchy.

### clawft-core

**Purpose**: The agent engine. Contains the message bus, agent loop, context builder, memory store, skills loader, and session manager.

**Key components**:

| Component | Python Source | Rust Module | Notes |
|-----------|-------------|-------------|-------|
| MessageBus | `bus/queue.py` | `bus.rs` | `tokio::sync::mpsc` channels |
| AgentLoop | `agent/loop.py` | `agent/loop.rs` | Async message processing + tool iteration |
| ContextBuilder | `agent/context.py` | `agent/context.rs` | System prompt assembly |
| MemoryStore | `agent/memory.py` | `agent/memory.rs` | MEMORY.md + HISTORY.md + ruvector search |
| SkillsLoader | `agent/skills.py` | `agent/skills.rs` | Progressive skill loading |
| SessionManager | `session/manager.py` | `session.rs` | JSONL session persistence |
| SubagentManager | `agent/subagent.py` | `agent/subagent.rs` | Background task spawning |
| ToolRegistry | `agent/tools/registry.py` | `tools/registry.rs` | Dynamic tool dispatch |

**MessageBus design**:
```rust
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Mutex<mpsc::UnboundedReceiver<InboundMessage>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::UnboundedReceiver<OutboundMessage>>,
}
```

**Tool trait** (the interface all built-in and custom tools implement):
```rust
/// Every tool must implement this trait. The ToolRegistry dispatches by name.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "read_file", "exec", "web_search").
    fn name(&self) -> &str;

    /// Human-readable description for the LLM's tool-use prompt.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters (used in OpenAI function calling format).
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments. Returns JSON-serializable result.
    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}
```

**AgentLoop design**:
```rust
pub struct AgentLoop<P: Platform> {
    bus: Arc<MessageBus>,
    pipeline: PipelineRegistry,       // Pluggable 6-stage pipeline (see below)
    tools: ToolRegistry,
    context: ContextBuilder<P>,       // Implements ContextAssembler trait
    sessions: SessionManager<P>,
    config: AgentConfig,
    platform: Arc<P>,
}
```

The `AgentLoop` uses `PipelineRegistry` (not a raw provider) to dispatch LLM calls. The pipeline selects the correct transport (clawft-llm provider) based on task classification. At Level 0 (no ruvector), the pipeline consists of `KeywordClassifier` + `StaticRouter` + `TokenBudgetAssembler` + `OpenAiCompatTransport` + `NoopScorer` + `NoopLearner`.

**ContextBuilder / ContextAssembler relationship**: `ContextBuilder` is the concrete clawft-core component that assembles system prompts, memory context, skill docs, and conversation history. It implements the `ContextAssembler` pipeline trait, so it can be replaced by `AttentionAssembler` (ruvector-attention) at higher intelligence levels.

Where `Platform` is a trait bundle:
```rust
pub trait Platform: HttpClient + FileSystem + Environment + Send + Sync + 'static {}
```

### clawft-llm (Standalone LLM Provider Library)

**Purpose**: Standalone LLM provider library extracted from barni-providers. Lives in its own repository/directory, NOT as a workspace member. Both barni and clawft depend on it. Provides HTTP transport, streaming, tool calling, failover, circuit breaker, and cost tracking.

**Extraction source**: `barni/src/layer3/barni-providers/` (10,563 lines across 11 source files). Only 2 files have barni-crate imports: `request.rs` (imports `RequestId`/`SessionId`/`TenantId` newtypes from barni-common) and `failover.rs` (imports `CircuitBreaker` ~200 lines and `now_ms()` helper from barni-common). `barni-protocol` is listed in Cargo.toml but never imported (phantom dependency). Total internalization: ~415 lines of CircuitBreaker + UUID newtypes → generic `Option<String>` metadata.

**Key features**:
- 4 native providers: Anthropic, OpenAI, Bedrock, Gemini
- Config-driven `OpenAiCompatProvider` for ANY OpenAI-compatible endpoint (Groq, DeepSeek, Mistral, etc.)
- Full SSE streaming with proper delta types
- Tool calling fully modeled
- 4 failover strategies (Sequential, WeightedRoundRobin, LowestLatency, CostOptimized)
- Lock-free CircuitBreaker (WASM-safe, uses atomics)
- Cost tracking with real pricing data
- Builder pattern for CompletionRequest

See [06-provider-layer-options.md](06-provider-layer-options.md) for the full pro/con analysis and decision rationale.

**Pluggable pipeline**: The pipeline architecture is documented below under the [clawft-core](#clawft-core) section. clawft-llm provides only the `LlmTransport` implementations (`OpenAiCompatTransport`, `AnthropicTransport`, etc.) that plug into the pipeline. clawft-core owns the pipeline traits and all other stage implementations.

See [05-ruvector-crates.md](05-ruvector-crates.md) sections 10-11 for the full pluggable architecture details.

### clawft-core: Pluggable Pipeline (6-stage)

The provider pipeline traits live in **clawft-core**, NOT in clawft-llm. This separation keeps the standalone library focused on HTTP transport while clawft-core owns the intelligence orchestration.

**Pipeline Traits** (6-stage pipeline, each stage pluggable):

```rust
/// Stage 1: Classify the task to determine routing strategy.
pub trait TaskClassifier: Send + Sync {
    fn classify(&self, request: &ChatRequest) -> TaskProfile;
}

/// Stage 2: Select the best provider/model for this task.
#[async_trait]
pub trait ModelRouter: Send + Sync {
    async fn route(&self, request: &ChatRequest, profile: &TaskProfile) -> RoutingDecision;
    fn update(&self, decision: &RoutingDecision, outcome: &ResponseOutcome);
}

/// Stage 3: Build context (system prompt, memories, skills) for the task.
#[async_trait]
pub trait ContextAssembler: Send + Sync {
    async fn assemble(&self, request: &ChatRequest, profile: &TaskProfile) -> AssembledContext;
}

/// Stage 4: Execute the LLM call via HTTP.
#[async_trait]
pub trait LlmTransport: Send + Sync {
    async fn complete(&self, request: &TransportRequest) -> Result<LlmResponse>;
    async fn complete_stream(&self, request: &TransportRequest) -> Result<ResponseStream>;
}

/// Stage 5: Score the response quality.
pub trait QualityScorer: Send + Sync {
    fn score(&self, request: &ChatRequest, response: &LlmResponse) -> QualityScore;
}

/// Stage 6: Learn from the interaction.
pub trait LearningBackend: Send + Sync {
    fn record(&self, trajectory: &Trajectory);
    fn adapt(&self, signal: &LearningSignal);
}
```

**Task profiles** drive pipeline selection per task type:

```rust
pub struct TaskProfile {
    pub task_type: TaskType,
    pub complexity: f32,              // 0.0 - 1.0
    pub urgency: Urgency,            // Low, Normal, High
    pub budget_class: BudgetClass,   // Minimal, Standard, Premium
    pub required_capabilities: Vec<Capability>,  // ToolCalling, LargeContext, Streaming, Vision
}

pub enum TaskType {
    Research,     // Broad access, accuracy-focused
    Development,  // Fast, tool-call reliable
    Creative,     // High quality, voice consistent
    Triage,       // Cheap and fast
    Analysis,     // Reasoning-capable
    Custom(String),
}
```

**Pipeline registry** maps task types to configured pipelines:

```rust
pub struct PipelineRegistry {
    pipelines: HashMap<TaskType, Pipeline>,
    default: Pipeline,
}

pub struct Pipeline {
    classifier: Arc<dyn TaskClassifier>,
    router: Arc<dyn ModelRouter>,
    context: Arc<dyn ContextAssembler>,
    transport: Arc<dyn LlmTransport>,
    scorer: Arc<dyn QualityScorer>,
    learner: Arc<dyn LearningBackend>,
}
```

**Built-in implementations (Level 0, no ruvector)** — all in clawft-core except `OpenAiCompatTransport` which wraps clawft-llm providers:

```rust
/// Direct HTTP provider for OpenAI-compatible APIs (wraps clawft-llm::Provider)
pub struct OpenAiCompatTransport<H: HttpClient> {
    http: Arc<H>,
    api_key: String,
    api_base: String,
    default_model: String,
    extra_headers: HashMap<String, String>,
    provider_name: String,
}

/// Static config.json-based routing (same as Python nanobot)
pub struct StaticRouter {
    registry: Vec<ProviderSpec>,
}

/// Keyword/regex task classification
pub struct KeywordClassifier {
    patterns: Vec<(Regex, TaskType)>,
}

/// Token-budget context truncation
pub struct TokenBudgetAssembler { max_tokens: usize }

/// No-op scorer and learner for Level 0
pub struct NoopScorer;
pub struct NoopLearner;
```

**ruvector implementations (Level 1+, feature-gated)** — all in clawft-core:

```rust
/// ruvllm-backed complexity analysis + HNSW routing
#[cfg(feature = "ruvllm")]
pub struct ComplexityClassifier { analyzer: ruvllm::TaskComplexityAnalyzer }

#[cfg(feature = "ruvllm")]
pub struct HnswModelRouter {
    hnsw: ruvllm::HnswRouter,
    quality: ruvllm::QualityScoringEngine,
}

/// Neural routing with CircuitBreaker
#[cfg(feature = "tiny-dancer")]
pub struct NeuralRouter {
    fast_grnn: tiny_dancer::FastGRNN,
    breaker: tiny_dancer::CircuitBreaker,
    uncertainty: tiny_dancer::UncertaintyEstimator,
}

/// Attention-based context assembly
#[cfg(feature = "attention")]
pub struct AttentionAssembler { /* FlashAttention, MoE, InfoBottleneck */ }

/// SONA self-learning backend
#[cfg(feature = "sona")]
pub struct SonaLearner { engine: sona::SonaEngine }
```

**Provider routing flow** (all levels):
1. `TaskClassifier::classify()` -> `TaskProfile` (keyword match OR ruvllm complexity analysis)
2. `PipelineRegistry::get(profile.task_type)` -> select pipeline for this task
3. `ModelRouter::route()` -> `RoutingDecision` (static match OR HNSW+neural routing)
4. `ContextAssembler::assemble()` -> `AssembledContext` (truncation OR attention-based)
5. `LlmTransport::complete()` -> HTTP POST to selected provider
6. `QualityScorer::score()` -> `QualityScore` (noop OR 5-dimension scoring)
7. `LearningBackend::record()` -> persist trajectory (noop OR SONA micro-LoRA)
8. `ModelRouter::update()` -> update routing policies for next request

**Fallback chain**: Each pipeline stage falls back gracefully. If ruvllm feature is disabled, `KeywordClassifier` and `StaticRouter` are used. The transport layer is always clawft's own `OpenAiCompatTransport`.

See [05-ruvector-crates.md](05-ruvector-crates.md) sections 10-11 for litellm-rs assessment and full pluggable architecture details.

### clawft-tools (Built-in Tools)

**Purpose**: Built-in tool implementations, each behind a feature flag.

| Tool | Feature | External Deps |
|------|---------|---------------|
| read_file | (always) | platform::FileSystem |
| write_file | (always) | platform::FileSystem |
| edit_file | (always) | platform::FileSystem |
| list_dir | (always) | platform::FileSystem |
| exec | `tool-exec` | platform::ProcessSpawner |
| web_search | `tool-web` | platform::HttpClient |
| web_fetch | `tool-web` | platform::HttpClient, readability |
| message | (always) | MessageBus |
| spawn | `tool-spawn` | SubagentManager |
| cron | `tool-cron` | CronService |

### clawft-channels (Plugin Architecture)

**Purpose**: Channel plugin host + plugin implementations. Each channel is a thread woven into the fabric -- a plugin that implements the `Channel` trait. The plugin host manages lifecycle, message routing, and error recovery.

**Plugin Host API**:
```rust
/// The trait every channel plugin must implement.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Unique channel identifier (e.g., "telegram", "slack", "discord").
    fn name(&self) -> &str;

    /// Start receiving messages. Runs until cancellation.
    async fn start(
        &self,
        host: Arc<dyn ChannelHost>,
        cancel: CancellationToken,
    ) -> Result<()>;

    /// Send an outbound message to the channel.
    async fn send(&self, msg: &OutboundMessage) -> Result<()>;

    /// Whether the channel is currently connected and running.
    fn is_running(&self) -> bool;
}

/// Services the plugin host exposes to channel plugins.
#[async_trait]
pub trait ChannelHost: Send + Sync {
    /// Deliver an inbound message to the agent pipeline.
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<()>;

    /// Get agent configuration.
    fn config(&self) -> &Config;

    /// Get the HTTP client for making API calls.
    fn http(&self) -> &dyn HttpClient;
}

/// Factory for creating channel instances from config.
pub trait ChannelFactory: Send + Sync {
    /// The channel name this factory creates.
    fn channel_name(&self) -> &str;

    /// Create a channel instance from a JSON config value.
    fn build(&self, config: serde_json::Value) -> Result<Box<dyn Channel>>;
}
```

**Plugin Registry** (compile-time, feature-flag gated):
```rust
/// Returns all available channel factories based on enabled features.
pub fn available_channels() -> Vec<Box<dyn ChannelFactory>> {
    let mut channels: Vec<Box<dyn ChannelFactory>> = Vec::new();

    #[cfg(feature = "telegram")]
    channels.push(Box::new(telegram::TelegramFactory));

    #[cfg(feature = "slack")]
    channels.push(Box::new(slack::SlackFactory));

    #[cfg(feature = "discord")]
    channels.push(Box::new(discord::DiscordFactory));

    channels
}
```

**Plugin implementations** (each in a submodule, behind feature flag):

| Plugin | Feature | Transport | Key Deps |
|--------|---------|-----------|----------|
| Telegram | `telegram` | HTTP long-polling + REST | reqwest |
| Slack | `slack` | Socket Mode WebSocket + REST | tokio-tungstenite, hmac-sha256 |
| Discord | `discord` | Gateway WebSocket + REST | tokio-tungstenite, ed25519-dalek |

**Plugin host responsibilities**:
- Discover enabled plugins via `available_channels()`
- Extract per-channel config from `Config.channels` as `serde_json::Value`
- Call `factory.build(config)` for each enabled channel
- Manage `start()`/`send()` lifecycle with cancellation tokens
- Route outbound messages to correct channel by name
- Handle plugin crashes with backoff retry

**Plugin responsibilities**:
- Implement transport (HTTP, WebSocket, etc.)
- Parse platform-specific messages into `InboundMessage`
- Format `OutboundMessage` for platform (markdown -> HTML/mrkdwn)
- Manage platform auth (API keys, tokens, OAuth)
- Report errors via `Result` (host handles recovery)

### clawft-services

**Purpose**: Background services -- cron, heartbeat, MCP client, and MCP server infrastructure (McpServerShell, ToolProvider trait, CompositeToolProvider, middleware pipeline).

### clawft-cli (binary: `weft`)

**Purpose**: The native binary. CLI/interactive mode is a core component, NOT a plugin.

Commands:
- `weft onboard` -- initialize config and workspace
- `weft gateway` -- start multi-channel server (loads channel plugins)
- `weft agent [-m "msg"]` -- interactive/single-message mode
- `weft status` -- show config and provider status
- `weft channels status` -- show channel plugin status
- `weft cron {list,add,remove,enable,run}` -- cron management

## 4. Dependency Map

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
tokio-util = { version = "0.7", features = ["rt"] }  # CancellationToken

# HTTP & networking
reqwest = { version = "0.12", default-features = false, features = ["json", "stream", "rustls-tls"] }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }

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
base64 = "0.22"
url = "2.5"

# CLI
clap = { version = "4.5", features = ["derive"] }

# RVF (RuVector Format) - vector intelligence (optional, Tier 1)
rvf-runtime = { version = "0.1", optional = true }
rvf-types = { version = "0.1", optional = true }
rvf-index = { version = "0.1", optional = true }
rvf-quant = { version = "0.1", optional = true }
rvf-adapters-agentdb = { version = "0.1", optional = true }
rvf-crypto = { version = "0.1", optional = true }

# ruvector intelligence crates (optional, Tier 1-2)
ruvllm = { version = "0.1", default-features = false, features = ["minimal"], optional = true }
ruvector-core = { version = "0.1", optional = true }
ruvector-tiny-dancer-core = { version = "0.1", optional = true }
ruvector-attention = { version = "0.1", optional = true }
ruvector-temporal-tensor = { version = "0.1", optional = true }
sona = { version = "0.1", optional = true }

# WASM-specific crates (clawft-wasm only)
micro-hnsw-wasm = { version = "0.1", optional = true }

# Standalone LLM provider library (external dependency)
clawft-llm = { git = "https://github.com/<org>/clawft-llm", branch = "main" }
# Or during development: clawft-llm = { path = "../clawft-llm" }

# Internal workspace crates
clawft-types = { path = "crates/clawft-types" }
clawft-platform = { path = "crates/clawft-platform" }
clawft-core = { path = "crates/clawft-core" }
clawft-tools = { path = "crates/clawft-tools" }
clawft-channels = { path = "crates/clawft-channels" }
clawft-services = { path = "crates/clawft-services" }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### Feature Flags

Feature flags follow a two-level scheme: **clawft-core** owns per-crate feature gates, **clawft-cli** exposes user-facing bundles.

```toml
# clawft-core/Cargo.toml
[features]
default = []

# RVF persistence
rvf = ["dep:rvf-runtime", "dep:rvf-types", "dep:rvf-index"]
rvf-agentdb = ["rvf", "dep:rvf-adapters-agentdb", "dep:ruvector-core"]
rvf-crypto = ["rvf", "dep:rvf-crypto"]

# Intelligence crates
ruvllm = ["dep:ruvllm"]
tiny-dancer = ["dep:ruvector-tiny-dancer-core"]
intelligent-routing = ["ruvllm", "tiny-dancer"]
sona = ["dep:sona"]
attention = ["dep:ruvector-attention"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]

# clawft-cli/Cargo.toml
[features]
default = ["channel-telegram", "all-tools"]

# Channel plugins
all-channels = ["channel-telegram", "channel-slack", "channel-discord"]
channel-telegram = ["clawft-channels/telegram"]
channel-slack = ["clawft-channels/slack"]
channel-discord = ["clawft-channels/discord"]

# Tools
all-tools = ["tool-exec", "tool-web", "tool-spawn", "tool-cron"]
tool-exec = ["clawft-tools/exec"]
tool-web = ["clawft-tools/web"]
tool-spawn = ["clawft-tools/spawn"]
tool-cron = ["clawft-services/cron"]

# RVF intelligence bundles. See 04-rvf-integration.md and 05-ruvector-crates.md
# Note: clawft-llm is standalone (no ruvector features). Intelligence lives in clawft-core.
ruvector = [
    "clawft-core/rvf-agentdb",
    "clawft-core/intelligent-routing",
    "clawft-core/sona",
    "clawft-core/attention",
    "clawft-core/temporal-tensor",
]
ruvector-full = ["ruvector", "clawft-core/rvf-crypto"]

minimal = ["channel-telegram"]  # Smallest useful build (no tools, no ruvector)
```

**Default**: `channel-telegram` + `all-tools`. The `ruvector` bundle is opt-in to keep the default binary small (~5 MB). Users add `--features ruvector` for intelligent routing.

```toml
# clawft-wasm/Cargo.toml
[features]
default = ["micro-hnsw", "temporal-tensor", "sona-wasm"]
micro-hnsw = ["dep:micro-hnsw-wasm"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]
sona-wasm = ["dep:sona"]
coherence = ["dep:cognitum-gate-kernel"]
```

## 5. RVF Integration Details

Full specification: [04-rvf-integration.md](04-rvf-integration.md)

### What RVF provides

| Component | Used For | Crate |
|-----------|----------|-------|
| RvfStore | Vector storage, query, ingest, compact | `rvf-runtime` |
| Progressive HNSW | Three-tier search (A=70%, B=85%, C=95% recall) | `rvf-index` |
| Auto-tiering | fp16/PQ/binary quantization by access frequency | `rvf-quant` |
| POLICY_KERNEL | Model routing policy parameters | `rvf-types` (segment) |
| COST_CURVE | Provider cost/latency/quality curves | `rvf-types` (segment) |
| WITNESS | Cryptographic audit trail | `rvf-crypto` |
| WASM microkernel | < 8 KB vector search for edge deployment | `rvf-wasm` |
| AgentDB adapter | Vector store CRUD interface | `rvf-adapters/agentdb` |
| Agentic-flow adapter | Swarm coordination (future) | `rvf-adapters/agentic-flow` |

### Integration points in clawft

1. **clawft-core/memory**: `MemoryStore` uses `RvfVectorStore` for semantic search over MEMORY.md content
2. **clawft-core/session**: `SessionManager` indexes session summaries in rvf for semantic retrieval
3. **clawft-core**: `IntelligentRouter` uses POLICY_KERNEL + COST_CURVE segments for learned routing (clawft-llm provides transport)
4. **clawft-core/agent**: Witness log tracks agent actions via rvf WITNESS segments

### WASM considerations

- `rvf-wasm` is a `no_std` microkernel with 14 C-ABI exports, < 8 KB after `wasm-opt`
- Core vector operations (distance, top-k) work in WASM linear memory
- The full `rvf-runtime` has `#[cfg(not(target_arch = "wasm32"))]` gates for parallel/SIMD
- clawft-wasm uses `rvf-wasm` microkernel instead of full runtime

## 6. Python-to-Rust Dependency Map

| Python Package | Rust Replacement | Size Impact |
|----------------|-----------------|-------------|
| typer | clap 4.5 | ~300 KB |
| litellm | RVF (rvf-runtime + rvf-index) + custom routing | ~260 KB (feature-gated) |
| pydantic | serde + serde_json | ~150 KB |
| pydantic-settings | config 0.14 + serde | ~50 KB |
| httpx | reqwest 0.12 | ~800 KB |
| websockets | tokio-tungstenite 0.24 | ~200 KB |
| loguru | tracing + tracing-subscriber | ~200 KB |
| rich | crossterm + comfy-table + indicatif | ~250 KB |
| prompt-toolkit | rustyline 14 | ~150 KB |
| readability-lxml | readability 0.3 | ~200 KB |
| croniter | cron 0.15 | ~30 KB |
| json-repair | Custom ~100 lines | 0 |
| python-telegram-bot | Custom REST client ~200 lines | 0 |
| slack-sdk | Custom WS+REST ~150 lines | 0 |
| slackify-markdown | Custom ~50 lines | 0 |
| mcp | Custom JSON-RPC ~150 lines | 0 |
| oauth-cli-kit | oauth2 4.4 | ~150 KB |

**Custom code total**: ~900 lines replacing ~1,800 lines of SDK usage (channels + routing).

## 7. Binary Size Targets

| Configuration | Estimated Size (stripped) | Includes |
|--------------|--------------------------|----------|
| Minimal (Telegram only, no ruvector) | ~3-4 MB | CLI + 1 channel + core tools |
| Default (all channels + ruvector) | ~8-12 MB | CLI + all channels + all tools + intelligence |
| WASM core | ~150-300 KB (uncompressed) | Agent loop + HTTP provider + file tools |
| WASM core (gzipped) | ~60-120 KB | Same, compressed |

## 8. WASM Technical Details

### Build Configuration

```toml
# .cargo/config.toml
[target.wasm32-wasip2]
runner = "wasmtime"
rustflags = ["-C", "link-arg=--import-memory"]

[profile.release-wasm]
inherits = "release"
opt-level = "z"       # Size over speed
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

The WASM build uses the `talc` allocator (< 1 KB overhead) instead of the default allocator:
```rust
// In clawft-wasm/src/lib.rs
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: talc::Talck<talc::locking::AssumeUnlockable, talc::ClaimOnOom> =
    talc::Talc::new(unsafe { talc::ClaimOnOom::new(talc::Span::empty()) }).lock();
```

Build command: `cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm`
Followed by: `wasm-opt -Oz -o clawft.wasm target/wasm32-wasip2/release-wasm/clawft-wasm.wasm`

### Target
- Primary WASM target: `wasm32-wasip2` (Tier 2 in Rust 1.82+)
- Runtime targets: WAMR (IoT), Wasmtime (edge/cloud)

### What runs in WASM
- Agent loop, LLM provider (HTTP), file tools, config loading, session/memory, JSON serialization
- rvf-wasm microkernel (< 8 KB, vector search via C-ABI). See [04-rvf-integration.md](04-rvf-integration.md)

### What does NOT run in WASM
- Shell exec tool, channel plugins (WebSocket), CLI (terminal I/O), interactive prompt

## 9. Testing Strategy

| Level | Tool | Scope |
|-------|------|-------|
| Unit | `cargo test` | Each crate independently |
| Integration | `cargo test --workspace` | Cross-crate communication |
| Config compat | Dedicated test | Load Python-generated config.json |
| Session compat | Dedicated test | Load Python-generated .jsonl files |
| Provider | Mock HTTP server | All 14 provider request/response formats |
| Channel plugin | Mock ChannelHost | Each plugin's start/send/parse cycle |
| Plugin registry | Feature flag test | Verify correct plugins enabled per feature |
| WASM | `wasmtime` in CI | Core agent loop + file tools |
| Binary size | CI check | Assert binary size < threshold |
| Workspace | Config merge test | Project config overlays global correctly |
| Skills chain | Discovery test | Project skills override global by name |
| MCP server (Mode 1) | Integration test | McpServerShell advertises tools via `tools/list` and dispatches `tools/call` |
| MCP client (Mode 2) | Integration test | clawft connects to external MCP servers and invokes their tools |

## 10. Workspace & Project Architecture (Phase 3G)

### Directory Structure

```
~/.clawft/                          # Global workspace (user-wide)
  config.json                       # Global configuration
  workspace/
    skills/                         # Global skills
      my-global-skill/
        SKILL.md
    memory/
      MEMORY.md
      HISTORY.md
    sessions/
      *.jsonl

~/my-project/                       # Any project directory
  .clawft/                          # Project workspace (project-specific)
    config.json                     # Project config (overrides global)
    skills/                         # Project-local skills
      project-skill/
        SKILL.md
    memory/
      MEMORY.md                     # Project-scoped memory
    agents/                         # Project-scoped agent definitions
      AGENTS.md
    mcp/
      servers.json                  # Project-scoped MCP server configs
  src/                              # Project source code (not managed by clawft)
  ...
```

### Config Hierarchy

Configuration is resolved by deep-merging layers. Later layers override earlier ones at the leaf level.

```
Priority (lowest to highest):
  1. Built-in defaults (compiled into binary)
  2. Global config:    ~/.clawft/config.json
  3. Project config:   .clawft/config.json (if project workspace found)
  4. Environment vars: $CLAWFT_* overrides (highest priority)
```

**Merge rules**:
- Scalar values: project wins
- Objects/maps: deep merge (project keys override, global-only keys preserved)
- Arrays: replace (project value wins if present)
- Maps (e.g., `mcp_servers`): deep-merge (project keys override, global-only keys preserved)
- `null` in project config: explicitly removes the global value

**Implementation**: `ConfigLoader::load()` in `clawft-platform` returns a merged `Config`. The project workspace path (if any) is stored in `Config.project_root: Option<PathBuf>`.

### Skills & Agents Discovery Chain

Skills are discovered from multiple directories and merged, with project-local skills taking precedence when names collide.

```
Discovery order (checked in sequence, highest precedence wins):
  1. Project skills:  <project>/.clawft/skills/       (project-scoped, via cwd walk-up)
  2. User skills:     ~/.clawft/skills/                (personal user-level)
  3. Global skills:   ~/.clawft/workspace/skills/      (global workspace)
```

**Merge semantics**: All directories are scanned. If a skill name (directory name) appears in multiple locations, the highest-precedence version wins. The result is a union of all skill names with project versions preferred.

**Agent definitions** follow the same chain:
```
  1. Project agents:  <project>/.clawft/agents/        (project-scoped, via cwd walk-up)
  2. User agents:     ~/.clawft/agents/                 (personal user-level)
  3. Built-in agents: compiled into binary               (default, explore, coder, planner)
```

### MCP Tool Delegation Architecture

clawft supports two MCP integration modes:

**Mode 1: MCP Server (Phase 3F) -- clawft exposes built-in tools TO external clients**
`weft mcp-server` starts clawft as an MCP server. External orchestrators (Claude Code, claude-flow) invoke clawft's tools via the MCP protocol. Uses newline-delimited JSON stdio framing (NOT Content-Length framing). Implemented via `McpServerShell` + pluggable `ToolProvider` trait in `clawft-services/src/mcp/`.
```
[Claude Code] --MCP stdio--> [McpServerShell] --> CompositeToolProvider --> ToolRegistry
[claude-flow] --MCP stdio--> [McpServerShell] --> CompositeToolProvider --> ToolRegistry
```

**Mode 2: MCP Client (existing, Phase 2) -- clawft consumes tools FROM external MCP servers**
clawft connects to external MCP servers as a client, making their tools available to the agent.
```
[MCP Server A] <--stdio--> [clawft MCP client] --> ToolRegistry
[MCP Server B] <--HTTP-->  [clawft MCP client] --> ToolRegistry
```

Config sources (merged):
- Global: `~/.clawft/config.json` → `tools.mcp_servers[]`
- Project: `.clawft/mcp/servers.json` or `.clawft/config.json` → `tools.mcp_servers[]`

**Delegation protocol**:
- Transport: stdio (primary, for Claude Code integration) and HTTP (for remote orchestrators)
- clawft advertises its registered tools via `tools/list`
- External clients invoke tools via `tools/call`, routed through the standard `ToolRegistry`
- The agent's full pipeline (context, memory, skills) is available for delegated calls

### Claude / claude-flow Integration Requirements

| Requirement | Description | Implementation |
|-------------|-------------|----------------|
| MCP server mode (Mode 1) | `weft mcp-server` starts clawft as an MCP stdio server via McpServerShell | `clawft-services/src/mcp/server.rs` |
| Tool advertisement | All registered tools exposed via MCP `tools/list` | `ToolRegistry::to_mcp_tools()` |
| Project-aware | MCP server inherits project workspace context from cwd | `ConfigLoader` project discovery |
| Skill passthrough | Skills loaded from project + global are available to delegated calls | `SkillsLoader` chain |
| Session isolation | Delegated calls use a dedicated session or caller-specified session ID | `SessionManager` |

### `weft init` Command

Scaffolds a project workspace in the current directory:

```
weft init [--minimal]

Creates:
  .clawft/
    config.json         # Minimal project config (model, system_prompt overrides)
    skills/             # Empty skills directory
    memory/             # Empty memory directory (MEMORY.md created on first use)
    agents/             # Empty agents directory

With --minimal:
  .clawft/
    config.json         # Only config file, no subdirectories
```

The generated `config.json` contains only override fields (not a full copy of global config):
```json
{
  "agents": {
    "system_prompt": "You are a coding assistant for this project.",
    "model": null
  }
}
```

## 11. Tiered Router & Permission System

The tiered router extends the pipeline's `ModelRouter` trait (Stage 2) with permission-aware, cost-aware tier selection. It replaces `StaticRouter` when `routing.mode = "tiered"` in config.json. Backward-compatible: when `mode = "static"` (or absent), the existing `StaticRouter` is used unchanged.

### 11.1 Permission Types

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Permission level for a user or request context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// Anonymous/unauthenticated - most restrictive
    ZeroTrust,
    /// Authenticated user - standard access
    User,
    /// Administrator - full access
    Admin,
}

/// Granular permission capabilities for a given level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCapabilities {
    /// Which provider/model pairs are allowed (empty = all for this level's tiers).
    #[serde(default)]
    pub model_access: Vec<String>,

    /// Which tools can be invoked. Empty = none. ["*"] = all.
    #[serde(default)]
    pub tool_access: Vec<String>,

    /// Tools explicitly denied (overrides tool_access).
    #[serde(default)]
    pub tool_deny: Vec<String>,

    /// Maximum context window tokens.
    #[serde(default = "default_max_context")]
    pub max_context_tokens: usize,

    /// Maximum output tokens per response.
    #[serde(default = "default_max_output")]
    pub max_output_tokens: usize,

    /// Requests per minute rate limit (0 = unlimited).
    #[serde(default)]
    pub rate_limit_rpm: u32,

    /// Whether streaming responses are allowed.
    #[serde(default)]
    pub streaming_allowed: bool,

    /// Whether complexity-based escalation to higher tiers is allowed.
    #[serde(default)]
    pub escalation_allowed: bool,

    /// Complexity threshold (0.0-1.0) above which escalation triggers.
    #[serde(default = "default_escalation_threshold")]
    pub escalation_threshold: f32,

    /// Maximum allowed model tiers (by tier name).
    #[serde(default)]
    pub allowed_tiers: Vec<String>,

    /// Extensible custom permissions for future features.
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

fn default_max_context() -> usize { 4096 }
fn default_max_output() -> usize { 1024 }
fn default_escalation_threshold() -> f32 { 0.6 }
```

### 11.2 Model Tier Configuration

```rust
/// A model tier groups models by cost and capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTier {
    /// Tier name (e.g., "free", "standard", "premium", "elite").
    pub name: String,

    /// Models available in this tier, in preference order.
    /// Format: "provider/model" (e.g., "anthropic/claude-haiku-3.5").
    pub models: Vec<String>,

    /// Complexity range this tier handles [min, max] (0.0-1.0).
    pub complexity_range: (f32, f32),

    /// Estimated cost per 1K tokens (USD) for cost tracking.
    #[serde(default)]
    pub cost_per_1k_tokens: f64,

    /// Maximum context window for models in this tier.
    #[serde(default)]
    pub max_context_tokens: Option<usize>,

    /// Priority (lower = preferred when tiers overlap). Default: 0.
    #[serde(default)]
    pub priority: i32,
}
```

### 11.3 Routing Configuration

```rust
/// Top-level routing configuration in config.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    /// Routing mode: "static" (default, backward compat) or "tiered".
    #[serde(default = "default_routing_mode")]
    pub mode: String,

    /// Model tiers (only used when mode = "tiered").
    #[serde(default)]
    pub tiers: Vec<ModelTier>,

    /// Permission definitions per level.
    #[serde(default)]
    pub permissions: HashMap<PermissionLevel, PermissionCapabilities>,

    /// Per-user permission overrides (key = user identifier from channel).
    #[serde(default)]
    pub user_overrides: HashMap<String, PermissionCapabilities>,

    /// Cost budget configuration.
    #[serde(default)]
    pub cost_budgets: Option<CostBudgetConfig>,

    /// Escalation configuration.
    #[serde(default)]
    pub escalation: Option<EscalationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBudgetConfig {
    /// Daily spend limit in USD (0 = unlimited).
    #[serde(default)]
    pub daily_limit_usd: f64,

    /// Monthly spend limit in USD (0 = unlimited).
    #[serde(default)]
    pub monthly_limit_usd: f64,

    /// Per-level budget overrides.
    #[serde(default)]
    pub per_level: HashMap<PermissionLevel, CostBudgetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// Whether escalation is enabled globally.
    #[serde(default)]
    pub enabled: bool,

    /// Default complexity threshold for escalation.
    #[serde(default = "default_escalation_threshold")]
    pub threshold: f32,

    /// Maximum tier that escalation can reach.
    #[serde(default)]
    pub max_tier: Option<String>,
}

fn default_routing_mode() -> String { "static".into() }
```

### 11.4 TieredRouter Implementation

```rust
/// TieredRouter implements ModelRouter with permission-aware tier selection.
pub struct TieredRouter {
    tiers: Vec<ModelTier>,
    permissions: HashMap<PermissionLevel, PermissionCapabilities>,
    user_overrides: HashMap<String, PermissionCapabilities>,
    escalation: Option<EscalationConfig>,
    cost_tracker: Arc<CostTracker>,
    rate_limiter: Arc<RateLimiter>,
}

#[async_trait]
impl ModelRouter for TieredRouter {
    async fn route(&self, request: &ChatRequest, profile: &TaskProfile) -> RoutingDecision {
        // 1. Resolve permission level from request context
        // 2. Get effective capabilities (level defaults + user overrides)
        // 3. Filter tiers by allowed_tiers in capabilities
        // 4. Select tier matching complexity range
        // 5. Check escalation if complexity > threshold
        // 6. Apply rate limit check
        // 7. Apply cost budget check
        // 8. Select model from tier (preference order, availability)
        // 9. Fall back to lower tier if needed
        // 10. Return RoutingDecision with full reason
    }

    fn update(&self, decision: &RoutingDecision, outcome: &ResponseOutcome) {
        // Track cost from response token usage
        // Update model performance metrics for future routing
    }
}
```

### 11.5 Auth Context Threading

The user's permission level flows through the system from channel to router:

```
Channel (Discord/Slack/Telegram)
  -> extracts user_id from platform message
  -> InboundMessage.sender_id
  -> AgentLoop receives InboundMessage
  -> Resolves PermissionLevel from sender_id via RoutingConfig
  -> Attaches to ChatRequest metadata (or TaskProfile extension)
  -> TieredRouter reads permission from request context
  -> RoutingDecision respects permission boundaries
```

Key implementation notes:
- `InboundMessage` already carries `sender_id: String` (set by channel plugins)
- `AgentLoop` resolves the permission level before constructing the `ChatRequest`
- The `RoutingConfig.user_overrides` map uses channel-prefixed user IDs (e.g., `"discord:123456789"`, `"slack:U04ABC123"`, `"telegram:98765"`)
- If no override exists, the user's `PermissionLevel` is determined by channel-level config or defaults to `ZeroTrust`
- The `TaskProfile` is extended with `permission_level: PermissionLevel` so the router can access it without parsing request metadata

### 11.6 Config JSON Format

Complete example showing the `routing` section in `config.json`:

```json
{
  "routing": {
    "mode": "tiered",
    "tiers": [
      {
        "name": "free",
        "models": ["openrouter/meta-llama/llama-3.1-8b-instruct:free", "groq/llama-3.1-8b-instant"],
        "complexity_range": [0.0, 0.3],
        "cost_per_1k_tokens": 0.0
      },
      {
        "name": "standard",
        "models": ["anthropic/claude-haiku-3.5", "openai/gpt-4o-mini"],
        "complexity_range": [0.0, 0.7],
        "cost_per_1k_tokens": 0.001
      },
      {
        "name": "premium",
        "models": ["anthropic/claude-sonnet-4-20250514", "openai/gpt-4o"],
        "complexity_range": [0.3, 1.0],
        "cost_per_1k_tokens": 0.01
      },
      {
        "name": "elite",
        "models": ["anthropic/claude-opus-4-5", "openai/o1"],
        "complexity_range": [0.7, 1.0],
        "cost_per_1k_tokens": 0.05
      }
    ],
    "permissions": {
      "zero_trust": {
        "allowed_tiers": ["free"],
        "tool_access": [],
        "max_context_tokens": 4096,
        "max_output_tokens": 1024,
        "rate_limit_rpm": 10,
        "streaming_allowed": false,
        "escalation_allowed": false
      },
      "user": {
        "allowed_tiers": ["free", "standard"],
        "tool_access": ["read_file", "list_dir", "web_search", "web_fetch", "message"],
        "max_context_tokens": 16384,
        "max_output_tokens": 4096,
        "rate_limit_rpm": 60,
        "streaming_allowed": true,
        "escalation_allowed": true,
        "escalation_threshold": 0.6
      },
      "admin": {
        "allowed_tiers": ["free", "standard", "premium", "elite"],
        "tool_access": ["*"],
        "max_context_tokens": 131072,
        "max_output_tokens": 16384,
        "rate_limit_rpm": 0,
        "streaming_allowed": true,
        "escalation_allowed": true,
        "escalation_threshold": 0.3
      }
    },
    "user_overrides": {
      "discord:123456789": {
        "allowed_tiers": ["free", "standard", "premium"],
        "escalation_allowed": true
      }
    },
    "cost_budgets": {
      "daily_limit_usd": 5.0,
      "monthly_limit_usd": 100.0
    },
    "escalation": {
      "enabled": true,
      "threshold": 0.6,
      "max_tier": "premium"
    }
  }
}
```

### 11.7 Crate Ownership

| Component | Crate | Notes |
|-----------|-------|-------|
| Permission types | clawft-types | `PermissionLevel`, `PermissionCapabilities`, `ModelTier`, `RoutingConfig` |
| TieredRouter | clawft-core/pipeline | Implements `ModelRouter` trait |
| CostTracker | clawft-core/pipeline | Tracks daily/monthly spend per user/level |
| RateLimiter | clawft-core/pipeline | Token bucket per user/level |
| Auth context | clawft-core/agent | Resolves user -> `PermissionLevel` from config |
| Config parsing | clawft-types | `RoutingConfig` deserialization |
