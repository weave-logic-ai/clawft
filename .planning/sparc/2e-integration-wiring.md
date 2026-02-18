# SPARC Implementation Plan: Stream 2E - Integration Wiring (P0)

**Timeline**: Week 7-8 (CRITICAL PATH -- unblocks all E2E testing)
**Owned Crates**: `clawft-cli` (primary), `clawft-core` (adapter module only)
**Dependencies**: Phase 1 complete (clawft-core pipeline, clawft-llm Provider, MessageBus, AgentLoop, bootstrap, markdown converters)

---

## 1. Agent Instructions

### Source Files to Read (MUST READ before implementation)

```
# clawft-core: pipeline + bootstrap
clawft/crates/clawft-core/src/pipeline/transport.rs     # LlmProvider trait + OpenAiCompatTransport
clawft/crates/clawft-core/src/pipeline/traits.rs        # LlmTransport, TransportRequest, PipelineRegistry
clawft/crates/clawft-core/src/bootstrap.rs              # AppContext, build_default_pipeline()
clawft/crates/clawft-core/src/bus.rs                    # MessageBus (inbound/outbound channels)
clawft/crates/clawft-core/src/agent/loop_core.rs        # AgentLoop::run(), process_message(), run_tool_loop()

# clawft-llm: standalone provider crate
clawft/crates/clawft-llm/src/provider.rs                # Provider trait (name, complete)
clawft/crates/clawft-llm/src/openai_compat.rs           # OpenAiCompatProvider (new, with_api_key, config)
clawft/crates/clawft-llm/src/types.rs                   # ChatRequest, ChatMessage, ChatResponse, ToolCall
clawft/crates/clawft-llm/src/config.rs                  # ProviderConfig, builtin_providers()
clawft/crates/clawft-llm/src/router.rs                  # ProviderRouter (route, strip_prefix, with_builtins)

# clawft-cli: current placeholder state
clawft/crates/clawft-cli/src/commands/agent.rs           # run_single_message(), run_interactive() -- PLACEHOLDERS
clawft/crates/clawft-cli/src/commands/gateway.rs         # run() -- missing agent loop + outbound dispatch
clawft/crates/clawft-cli/src/commands/mod.rs             # load_config, expand_workspace, make_channel_host
clawft/crates/clawft-cli/src/markdown/mod.rs             # MarkdownConverter trait
clawft/crates/clawft-cli/src/markdown/telegram.rs        # TelegramMarkdownConverter
clawft/crates/clawft-cli/src/markdown/slack.rs           # SlackMarkdownConverter
clawft/crates/clawft-cli/src/markdown/discord.rs         # DiscordMarkdownConverter

# clawft-types: shared types
clawft/crates/clawft-types/src/config.rs                 # Config, ProvidersConfig, AgentsConfig
clawft/crates/clawft-types/src/event.rs                  # InboundMessage, OutboundMessage
```

### Planning Documents (MUST READ)
```
repos/nanobot/.planning/02-technical-requirements.md     # Architecture decisions
repos/nanobot/.planning/03-development-guide.md          # Stream timelines
repos/nanobot/.planning/sparc/1b-core-engine.md          # Pipeline + AgentLoop spec
repos/nanobot/.planning/sparc/1c-provider-tools-cli-telegram.md  # CLI + provider wiring spec
repos/nanobot/.planning/sparc/00-orchestrator.md         # Cross-stream integration test strategy
```

### Key Architectural Constraint

The `clawft-llm` crate is a standalone library with **no dependency on clawft-types or clawft-core**. It has its own type system (`clawft_llm::types::{ChatRequest, ChatResponse, ChatMessage}`) which is distinct from the pipeline's type system (`clawft_core::pipeline::traits::{ChatRequest, LlmMessage, TransportRequest}`). The integration wiring must bridge these two type systems through a thin adapter that implements `clawft_core::pipeline::transport::LlmProvider` by delegating to `clawft_llm::Provider`.

### Module Structure (files created or modified by this stream)
```
clawft/crates/clawft-core/src/
    pipeline/
        llm_adapter.rs          # NEW (~150-250 lines): LlmProvider adapter wrapping clawft_llm::Provider
        mod.rs                  # MODIFIED: add `pub mod llm_adapter;`
    bootstrap.rs                # MODIFIED: add build_live_pipeline() constructor

clawft/crates/clawft-cli/src/
    commands/
        agent.rs                # MODIFIED: wire real AppContext + AgentLoop into run_single_message/run_interactive
        gateway.rs              # MODIFIED: add agent loop background task + outbound dispatch loop
    markdown/
        dispatch.rs             # NEW (~30-50 lines): select + apply converter by channel name
        mod.rs                  # MODIFIED: add `pub mod dispatch;`
```

---

## 2. Specification

### 2.1 LLM Adapter Bridge

#### Problem

Two independent type systems exist:

| Concept | `clawft_core::pipeline::transport` | `clawft_llm` |
|---------|--------------------------------------|--------------|
| Provider trait | `LlmProvider` (JSON-in/JSON-out) | `Provider` (typed `ChatRequest`/`ChatResponse`) |
| Request type | `(model, messages: Vec<Value>, tools: Vec<Value>, max_tokens, temperature)` | `ChatRequest { model, messages: Vec<ChatMessage>, tools, max_tokens, temperature, stream }` |
| Response type | `serde_json::Value` (OpenAI JSON format) | `ChatResponse { id, choices, usage, model }` |
| Message type | `serde_json::Value` with `role`/`content` keys | `ChatMessage { role, content, tool_call_id, tool_calls }` |
| Config type | None (provider is injected) | `ProviderConfig { name, base_url, api_key_env, model_prefix, default_model, headers }` |

The `LlmProvider` trait in `transport.rs` uses raw `serde_json::Value` for messages and responses so that it does not import `clawft-llm` types directly. The adapter converts between these representations.

#### Requirements

- [ ] Create `clawft-core/src/pipeline/llm_adapter.rs` containing `ClawftLlmAdapter`
- [ ] `ClawftLlmAdapter` wraps an `Arc<dyn clawft_llm::Provider>` and implements `clawft_core::pipeline::transport::LlmProvider`
- [ ] The `complete()` method converts JSON `messages` to `Vec<ChatMessage>`, calls `provider.complete()`, then serializes the `ChatResponse` back to `serde_json::Value`
- [ ] JSON-to-`ChatMessage` conversion handles all roles: `system`, `user`, `assistant`, `tool`
- [ ] `tool_call_id` is forwarded for `tool` role messages
- [ ] Tool definitions (`Vec<serde_json::Value>`) are forwarded directly (both systems use JSON)
- [ ] `ChatResponse` is serialized to the OpenAI JSON format that `convert_response()` in `transport.rs` already parses
- [ ] Provider errors (`clawft_llm::ProviderError`) are mapped to `String` as required by the `LlmProvider::complete()` return type
- [ ] A factory function `create_adapter_from_config(config: &Config) -> Option<Arc<dyn LlmProvider>>` reads `config.agents.defaults.model`, routes it via `ProviderRouter`, and returns a configured adapter
- [ ] If the model string has no recognized prefix and no API key is available, returns `None` (fallback to stub transport)
- [ ] Register the module in `pipeline/mod.rs`

#### Acceptance Criteria

- [ ] `ClawftLlmAdapter` passes unit tests with a mock `clawft_llm::Provider`
- [ ] Text responses, tool-call responses, and error responses all roundtrip correctly
- [ ] `create_adapter_from_config()` correctly creates an adapter for `"openai/gpt-4o"` when `OPENAI_API_KEY` is set
- [ ] `create_adapter_from_config()` returns `None` when no key is available and model has no prefix
- [ ] Integration test: `AppContext::new()` with a live adapter produces a configured `OpenAiCompatTransport`

### 2.2 Wire `weft agent` CLI

#### Current State

`agent.rs` has:
- `run()`: loads config, creates `MessageBus`, `SessionManager`, `ToolRegistry`, but never creates an `AppContext` or `AgentLoop`. Calls `run_single_message()` or `run_interactive()` with bare model string.
- `run_single_message()`: prints placeholder text.
- `run_interactive()`: REPL that prints placeholder for every input.

#### Requirements

- [ ] `run()` creates `AppContext::new(config, platform)` instead of standalone bus/sessions/tools
- [ ] After `AppContext::new()`, call `create_adapter_from_config()` to get an `LlmProvider`; if `Some`, build a pipeline with `OpenAiCompatTransport::with_provider()` and call `ctx.set_pipeline()`
- [ ] Register tools on `ctx.tools_mut()` (same `clawft_tools::register_all()` call that exists now)
- [ ] Clone `ctx.bus()` inbound sender and outbound sender before consuming the context
- [ ] Apply model override from `--model` flag by mutating `config.agents.defaults.model` before constructing `AppContext`
- [ ] `run_single_message(bus, agent_loop)` publishes an `InboundMessage` to the bus, calls `agent_loop.process_message()` or waits for outbound, then prints the response content to stdout
- [ ] `run_interactive(bus, agent_loop)` loops: read stdin line, publish `InboundMessage`, call pipeline, print outbound content
- [ ] `/tools` command lists registered tool names
- [ ] `/clear` command resets the session (clear message history for the session key)
- [ ] Error handling: if the LLM returns an error, print it to stderr and continue the REPL (do not crash)
- [ ] Single-message mode exits with code 0 on success, 1 on LLM error

#### Acceptance Criteria

- [ ] `weft agent -m "hello"` with a mock provider prints the LLM response to stdout
- [ ] `weft agent` in interactive mode accepts input, processes it, prints response
- [ ] `/exit` and `/quit` exit cleanly
- [ ] `/tools` lists at least the built-in tools
- [ ] `--model` flag overrides the configured model
- [ ] LLM errors do not crash interactive mode
- [ ] Unit tests cover: argument parsing, help text, error paths

### 2.3 Wire `weft gateway` CLI

#### Current State

`gateway.rs`:
- Creates `MessageBus` and `PluginHost` with channel factories
- Registers Telegram channel
- Starts channels and blocks on Ctrl+C
- **Missing**: No `AgentLoop` is created, so inbound messages from channels go nowhere. No outbound dispatch loop exists.

#### Requirements

- [ ] After creating the bus, create `AppContext::new(config, platform)` with the same bus reference
- [ ] Wire the LLM adapter (same as agent command): `create_adapter_from_config()` -> `set_pipeline()`
- [ ] Register tools on `ctx.tools_mut()`
- [ ] Clone `ctx.bus()` reference before consuming to `into_agent_loop()`
- [ ] Spawn `agent_loop.run()` as a background `tokio::spawn` task
- [ ] Spawn an outbound dispatch loop as a second `tokio::spawn` task
- [ ] The outbound dispatch loop: `while let Some(msg) = bus.consume_outbound().await { route_to_channel(msg) }`
- [ ] `route_to_channel()` looks up the channel by `msg.channel` in the `PluginHost`, applies the markdown converter, calls `channel.send()`
- [ ] Pass a `CancellationToken` to both spawned tasks; cancel it on Ctrl+C for graceful shutdown
- [ ] Register Slack and Discord channel factories when their config sections are enabled (future-proofing; the factories may not exist yet, so use `#[cfg(feature = "...")]` guards or conditional registration)
- [ ] Log warnings for outbound messages targeting channels that are not running

#### Acceptance Criteria

- [ ] Gateway starts, receives inbound from Telegram, processes through agent loop, dispatches outbound back to Telegram
- [ ] Outbound messages are converted via the appropriate markdown converter
- [ ] Ctrl+C triggers graceful shutdown (cancellation token -> agent loop stops -> channels stop)
- [ ] Messages to non-existent channels produce a warning log, not a crash
- [ ] Unit tests: gateway args, dispatch routing logic

### 2.4 Wire Markdown Converters

#### Current State

Three converters exist (`TelegramMarkdownConverter`, `SlackMarkdownConverter`, `DiscordMarkdownConverter`) implementing the `MarkdownConverter` trait. They are tested but not wired into the outbound dispatch path. The trait has `#[allow(dead_code)]`.

#### Requirements

- [ ] Create `clawft-cli/src/markdown/dispatch.rs` with `MarkdownDispatcher`
- [ ] `MarkdownDispatcher` holds a `HashMap<String, Box<dyn MarkdownConverter>>` mapping channel names to converters
- [ ] Constructor registers the built-in converters: `"telegram"` -> `TelegramMarkdownConverter`, `"slack"` -> `SlackMarkdownConverter`, `"discord"` -> `DiscordMarkdownConverter`
- [ ] `fn convert(&self, channel: &str, content: &str) -> String` -- looks up converter, applies it, returns converted string. Falls back to identity (no conversion) for unknown channels.
- [ ] Remove `#[allow(dead_code)]` from the `MarkdownConverter` trait once wired
- [ ] The gateway outbound dispatch loop uses `MarkdownDispatcher::convert()` before sending

#### Acceptance Criteria

- [ ] Telegram outbound messages have HTML formatting applied
- [ ] Slack outbound messages have mrkdwn formatting applied
- [ ] Discord outbound messages pass through mostly unchanged
- [ ] Unknown channels get raw content without panic
- [ ] Unit tests: dispatch to each converter, fallback for unknown channel

---

## 3. Pseudocode

### 3.1 LLM Adapter Bridge (`llm_adapter.rs`)

```
/// Adapter that bridges clawft_llm::Provider to clawft_core::pipeline::transport::LlmProvider.
struct ClawftLlmAdapter {
    provider: Arc<dyn clawft_llm::Provider>,
}

impl ClawftLlmAdapter {
    fn new(provider: Arc<dyn clawft_llm::Provider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl LlmProvider for ClawftLlmAdapter {
    async fn complete(
        &self,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        max_tokens: Option<i32>,
        temperature: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        // 1. Convert JSON messages to clawft_llm::ChatMessage
        let chat_messages: Vec<ChatMessage> = messages.iter().map(|msg| {
            let role = msg["role"].as_str().unwrap_or("user");
            let content = msg["content"].as_str().unwrap_or("");
            let tool_call_id = msg.get("tool_call_id").and_then(|v| v.as_str()).map(String::from);
            ChatMessage {
                role: role.into(),
                content: content.into(),
                tool_call_id,
                tool_calls: None,  // tool_calls are in the response, not request
            }
        }).collect();

        // 2. Build ChatRequest
        let request = ChatRequest {
            model: model.into(),
            messages: chat_messages,
            max_tokens,
            temperature,
            tools: tools.to_vec(),
            stream: None,
        };

        // 3. Call the underlying provider
        let response = self.provider.complete(&request)
            .await
            .map_err(|e| e.to_string())?;

        // 4. Serialize ChatResponse to serde_json::Value (OpenAI format)
        //    The ChatResponse already has the OpenAI shape (id, choices, usage, model)
        //    so serde_json::to_value() produces the correct format.
        serde_json::to_value(&response)
            .map_err(|e| format!("failed to serialize response: {e}"))
    }
}

/// Create an LlmProvider adapter from application config.
///
/// Returns None if no provider could be configured (e.g. missing API key).
fn create_adapter_from_config(config: &Config) -> Option<Arc<dyn LlmProvider>> {
    let model = &config.agents.defaults.model;
    if model.is_empty() {
        return None;
    }

    // Use clawft_llm's ProviderRouter with builtin providers
    let router = ProviderRouter::with_builtins();

    // Check if the model can be routed to a provider
    let (provider_ref, _stripped_model) = router.route(model)?;

    // We need to own the provider. ProviderRouter returns a reference,
    // so we rebuild the provider from config.
    // Alternative: extract the provider config from the router's route result
    // and construct a fresh OpenAiCompatProvider.
    let (prefix_opt, _bare_model) = ProviderRouter::strip_prefix(model);
    let prefix_name = prefix_opt.unwrap_or_else(|| "openai".into());

    // Look up the matching builtin config
    let builtin_configs = clawft_llm::config::builtin_providers();
    let provider_config = builtin_configs.into_iter()
        .find(|c| c.name == prefix_name)?;

    // Try to resolve the API key -- if it fails, return None (no crash)
    let api_key = std::env::var(&provider_config.api_key_env).ok()?;

    let provider = Arc::new(
        OpenAiCompatProvider::with_api_key(provider_config, api_key)
    ) as Arc<dyn clawft_llm::Provider>;

    Some(Arc::new(ClawftLlmAdapter::new(provider)))
}
```

### 3.2 Wire `weft agent` CLI (`agent.rs`)

```
pub async fn run(args: AgentArgs) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let mut config = load_config(&*platform, args.config.as_deref()).await?;

    // Apply model override
    if let Some(ref model) = args.model {
        config.agents.defaults.model = model.clone();
    }

    info!(model = %config.agents.defaults.model, "initializing agent");

    // Create AppContext (initializes bus, sessions, memory, skills, pipeline)
    let mut ctx = AppContext::new(config.clone(), platform.clone()).await?;

    // Register tools
    let workspace = expand_workspace(&config.agents.defaults.workspace);
    clawft_tools::register_all(ctx.tools_mut(), platform.clone(), workspace);
    info!(tools = ctx.tools().len(), "tool registry initialized");

    // Wire live LLM provider (if available)
    if let Some(adapter) = create_adapter_from_config(&config) {
        let pipeline = build_live_pipeline(&config, adapter);
        ctx.set_pipeline(pipeline);
        info!("LLM provider configured");
    } else {
        tracing::warn!("no LLM provider configured -- agent will return errors");
    }

    // Clone bus before consuming into agent_loop
    let bus = ctx.bus().clone();

    // Convert to agent loop
    let agent_loop = ctx.into_agent_loop();

    if let Some(ref message) = args.message {
        run_single_message(&bus, &agent_loop, message).await
    } else {
        run_interactive(&bus, &agent_loop).await
    }
}

async fn run_single_message(
    bus: &Arc<MessageBus>,
    agent_loop: &AgentLoop<NativePlatform>,
    message: &str,
) -> anyhow::Result<()> {
    // Publish as InboundMessage on "cli" channel
    let inbound = InboundMessage {
        channel: "cli".into(),
        sender_id: "user".into(),
        chat_id: "cli-session".into(),
        content: message.into(),
        timestamp: chrono::Utc::now(),
        media: vec![],
        metadata: HashMap::new(),
    };
    bus.publish_inbound(inbound)?;

    // Consume and process
    let msg = bus.consume_inbound().await
        .ok_or_else(|| anyhow::anyhow!("bus closed unexpectedly"))?;

    agent_loop.process_message(msg).await
        .map_err(|e| anyhow::anyhow!("agent error: {e}"))?;

    // Consume outbound
    let outbound = bus.consume_outbound().await
        .ok_or_else(|| anyhow::anyhow!("no response from agent"))?;

    println!("{}", outbound.content);
    Ok(())
}

async fn run_interactive(
    bus: &Arc<MessageBus>,
    agent_loop: &AgentLoop<NativePlatform>,
) -> anyhow::Result<()> {
    println!("weft agent -- interactive mode (type /help for commands)");
    println!("Model: {}", agent_loop.config().defaults.model);
    println!();

    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin).lines();
    let mut message_count: u64 = 0;

    loop {
        eprint!("> ");
        std::io::stderr().flush().ok();

        let line = match reader.next_line().await? {
            Some(l) => l,
            None => break,
        };
        let input = line.trim();
        if input.is_empty() { continue; }

        match input {
            "/exit" | "/quit" => break,
            "/clear" => {
                // TODO: reset session for "cli:interactive" key
                println!("[session cleared]");
                continue;
            }
            "/help" => { print_help(); continue; }
            "/tools" => {
                // List tools from the agent_loop (needs accessor or stored ref)
                println!("[tool listing]");
                continue;
            }
            _ => {}
        }

        // Publish message
        message_count += 1;
        let inbound = InboundMessage {
            channel: "cli".into(),
            sender_id: "user".into(),
            chat_id: "interactive".into(),
            content: input.into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        bus.publish_inbound(inbound)?;

        // Process
        let msg = bus.consume_inbound().await
            .ok_or_else(|| anyhow::anyhow!("bus closed"))?;

        match agent_loop.process_message(msg).await {
            Ok(()) => {
                if let Some(outbound) = bus.consume_outbound().await {
                    println!("{}", outbound.content);
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                eprintln!();
            }
        }
    }

    println!("Goodbye.");
    Ok(())
}
```

**Note**: `process_message()` is currently `async fn process_message(&self, ...)` which is private. We need to either:
- (a) Make it `pub` on `AgentLoop`, or
- (b) Use `agent_loop.run()` in a background task and rely on bus-based message passing.

Option (b) is preferable for the gateway (long-running), while the agent CLI may use either approach. The pseudocode above uses approach (a) for simplicity. If `process_message` stays private, the agent CLI will instead:
1. Spawn `agent_loop.run()` as a background task.
2. Publish to bus.
3. Await `bus.consume_outbound()`.

### 3.3 Wire `weft gateway` CLI (`gateway.rs`)

```
pub async fn run(args: GatewayArgs) -> anyhow::Result<()> {
    info!("starting weft gateway");

    let platform = Arc::new(NativePlatform::new());
    let config = load_config(&*platform, args.config.as_deref()).await?;

    // Create message bus (shared between AppContext and PluginHost)
    let bus = Arc::new(MessageBus::new());

    // Create channel host bridge (channels -> bus inbound)
    let host = make_channel_host(bus.clone());

    // Create plugin host and register channel factories
    let plugin_host = PluginHost::new(host);
    register_channel_factories(&plugin_host, &config).await?;

    // Start channels
    let start_results = plugin_host.start_all().await;
    // ... (existing start/error handling logic) ...

    // Create AppContext sharing the same bus
    let mut ctx = AppContext::with_bus(config.clone(), platform.clone(), bus.clone()).await?;
    // NOTE: AppContext::with_bus() is a new constructor that accepts an existing bus.
    // Alternative: if we don't want to add with_bus(), we can create the AppContext first,
    // clone its bus, and pass it to the channel host.

    // Wire LLM adapter
    if let Some(adapter) = create_adapter_from_config(&config) {
        let pipeline = build_live_pipeline(&config, adapter);
        ctx.set_pipeline(pipeline);
        info!("LLM provider configured");
    } else {
        tracing::warn!("no LLM provider configured");
    }

    // Register tools
    let workspace = expand_workspace(&config.agents.defaults.workspace);
    clawft_tools::register_all(ctx.tools_mut(), platform.clone(), workspace);

    // Clone bus for outbound dispatch before consuming ctx
    let dispatch_bus = bus.clone();

    // Create agent loop and spawn
    let agent_loop = ctx.into_agent_loop();
    let cancel = tokio_util::sync::CancellationToken::new();

    // Spawn agent loop
    let cancel_agent = cancel.clone();
    let agent_handle = tokio::spawn(async move {
        tokio::select! {
            result = agent_loop.run() => {
                if let Err(e) = result {
                    error!("agent loop error: {e}");
                }
            }
            _ = cancel_agent.cancelled() => {
                info!("agent loop cancelled");
            }
        }
    });

    // Spawn outbound dispatch loop
    let cancel_dispatch = cancel.clone();
    let dispatcher = MarkdownDispatcher::new();
    let dispatch_plugin_host = plugin_host.clone();  // PluginHost must be Clone or Arc
    let dispatch_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = dispatch_bus.consume_outbound() => {
                    match msg {
                        Some(outbound) => {
                            let converted = dispatcher.convert(&outbound.channel, &outbound.content);
                            let send_msg = OutboundMessage {
                                content: converted,
                                ..outbound
                            };
                            if let Err(e) = dispatch_plugin_host.send(&send_msg.channel, &send_msg).await {
                                warn!(channel = %send_msg.channel, "outbound dispatch failed: {e}");
                            }
                        }
                        None => break,  // bus closed
                    }
                }
                _ = cancel_dispatch.cancelled() => {
                    info!("outbound dispatch cancelled");
                    break;
                }
            }
        }
    });

    info!(channels = started_count, "gateway running -- press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("shutting down gateway");

    // Cancel background tasks
    cancel.cancel();
    let _ = tokio::join!(agent_handle, dispatch_handle);

    // Stop channels
    let stop_results = plugin_host.stop_all().await;
    // ... (existing stop logic) ...

    info!("gateway shutdown complete");
    Ok(())
}
```

**Note**: The above requires either `PluginHost` to expose a `send(channel_name, msg)` method or the gateway to maintain its own channel lookup table. If `PluginHost` does not have this API, the outbound dispatch loop instead iterates its registered channels and calls `channel.send()` directly.

### 3.4 Markdown Dispatch (`dispatch.rs`)

```
use std::collections::HashMap;
use super::{MarkdownConverter, telegram, slack, discord};

/// Dispatches markdown conversion based on channel name.
pub struct MarkdownDispatcher {
    converters: HashMap<String, Box<dyn MarkdownConverter + Send + Sync>>,
}

impl MarkdownDispatcher {
    /// Create a dispatcher with all built-in converters registered.
    pub fn new() -> Self {
        let mut converters: HashMap<String, Box<dyn MarkdownConverter + Send + Sync>> = HashMap::new();
        converters.insert("telegram".into(), Box::new(telegram::TelegramMarkdownConverter));
        converters.insert("slack".into(), Box::new(slack::SlackMarkdownConverter));
        converters.insert("discord".into(), Box::new(discord::DiscordMarkdownConverter));
        Self { converters }
    }

    /// Convert content for the given channel.
    ///
    /// Returns the original content unchanged if no converter is registered
    /// for the channel (identity fallback).
    pub fn convert(&self, channel: &str, content: &str) -> String {
        match self.converters.get(channel) {
            Some(converter) => converter.convert(content),
            None => content.to_string(),
        }
    }
}
```

---

## 4. Architecture

### 4.1 Data Flow Diagram

```
                            weft agent -m "hello"
                            |
                            v
                    +-----------------+
                    | AgentCommand    |
                    |  run()          |
                    +--------+--------+
                             |
             +---------------v---------------+
             |        AppContext::new()       |
             |  +---------------------------+|
             |  | MessageBus                ||
             |  | SessionManager            ||
             |  | MemoryStore               ||
             |  | SkillsLoader              ||
             |  | ToolRegistry              ||
             |  | PipelineRegistry ------+  ||
             |  +------------------------|--+|
             +---------------------------|---+
                                         |
           +-----------------------------v------------------------------+
           |                     PipelineRegistry                       |
           |  +----------+  +--------+  +----------+  +----------+     |
           |  |Classifier|->|Router  |->|Assembler |->|Transport |     |
           |  +----------+  +--------+  +----------+  +----+-----+     |
           |                                                |           |
           +------------------------------------------------|-----------+
                                                            |
                              +-----------------------------v----------+
                              |    OpenAiCompatTransport               |
                              |      provider: Arc<dyn LlmProvider>    |
                              +-----------------------------+----------+
                                                            |
                              +-----------------------------v----------+
                              |    ClawftLlmAdapter                    |
                              |      provider: Arc<dyn llm::Provider>  |
                              |    converts JSON <-> ChatRequest       |
                              +-----------------------------+----------+
                                                            |
                              +-----------------------------v----------+
                              |    OpenAiCompatProvider                 |
                              |      config: ProviderConfig             |
                              |      http: reqwest::Client              |
                              |    makes actual HTTP calls              |
                              +----------------------------------------+
```

### 4.2 Type Bridge Map

```
Request path (top-down):
  pipeline::TransportRequest
    -> OpenAiCompatTransport::complete()
       converts to (model, Vec<serde_json::Value>, Vec<Value>, max_tokens, temperature)
    -> LlmProvider::complete()  [trait method, JSON args]
    -> ClawftLlmAdapter::complete()
       converts JSON messages -> Vec<ChatMessage>
       builds clawft_llm::ChatRequest
    -> llm::Provider::complete(&ChatRequest)
    -> OpenAiCompatProvider makes HTTP call

Response path (bottom-up):
  llm::ChatResponse (from HTTP response deserialization)
    -> ClawftLlmAdapter serializes to serde_json::Value
    -> returns to OpenAiCompatTransport
    -> convert_response() parses JSON into clawft_types::LlmResponse
    -> PipelineRegistry returns LlmResponse to AgentLoop
```

### 4.3 Gateway Outbound Flow

```
AgentLoop
  |  process_message() dispatches OutboundMessage to bus
  v
MessageBus (outbound channel)
  |  consume_outbound()
  v
Outbound Dispatch Loop (tokio::spawn)
  |  1. Read msg.channel
  |  2. MarkdownDispatcher.convert(channel, content)
  |  3. Look up Channel in PluginHost
  |  4. channel.send(converted_msg)
  v
Channel Plugin (Telegram/Slack/Discord)
  |  Sends via platform-specific API
  v
User
```

### 4.4 Module Dependency Graph (new additions in bold)

```
clawft-types (no changes)
     |
clawft-platform (no changes)
     |
clawft-llm (no changes, standalone)
     |
clawft-core
  |  pipeline/traits.rs       (no changes)
  |  pipeline/transport.rs    (no changes)
  |  **pipeline/llm_adapter.rs**  (NEW: depends on clawft-llm)
  |  **bootstrap.rs**             (MODIFIED: new build_live_pipeline fn)
  |  bus.rs                   (no changes)
  |  agent/loop_core.rs       (MINOR: make process_message pub if needed)
     |
clawft-channels (no changes)
     |
clawft-tools (no changes)
     |
clawft-cli
  |  **commands/agent.rs**        (MODIFIED: wire real agent loop)
  |  **commands/gateway.rs**      (MODIFIED: add agent loop + dispatch)
  |  **markdown/dispatch.rs**     (NEW: converter routing)
```

### 4.5 Cargo.toml Changes

**`clawft-core/Cargo.toml`**: Add `clawft-llm` as an optional dependency:
```toml
[dependencies]
clawft-llm = { path = "../clawft-llm", optional = true }

[features]
default = ["llm"]
llm = ["dep:clawft-llm"]
```

This keeps `clawft-core` usable without `clawft-llm` (e.g., WASM builds). The `llm_adapter` module is gated behind `#[cfg(feature = "llm")]`.

**`clawft-cli/Cargo.toml`**: Already depends on `clawft-core` -- ensure the `llm` feature is enabled:
```toml
clawft-core = { path = "../clawft-core", features = ["llm"] }
tokio-util = { version = "0.7", features = ["rt"] }  # CancellationToken
```

---

## 5. Refinement

### 5.1 Error Handling

#### Adapter Errors

The `LlmProvider::complete()` returns `Result<Value, String>`. The adapter must map all `clawft_llm::ProviderError` variants to descriptive strings:

```rust
match self.provider.complete(&request).await {
    Ok(response) => { /* serialize */ },
    Err(clawft_llm::ProviderError::Auth(msg)) =>
        Err(format!("authentication failed: {msg}")),
    Err(clawft_llm::ProviderError::RateLimit { retry_after }) =>
        Err(format!("rate limited, retry after {retry_after:?}")),
    Err(clawft_llm::ProviderError::Network(e)) =>
        Err(format!("network error: {e}")),
    Err(clawft_llm::ProviderError::InvalidResponse(msg)) =>
        Err(format!("invalid response from provider: {msg}")),
    Err(e) => Err(e.to_string()),
}
```

#### CLI Error Handling

- Agent single-message mode: print error to stderr, exit 1.
- Agent interactive mode: print error to stderr, continue REPL.
- Gateway: log errors per-message, never crash the loop.

#### Missing API Key

`create_adapter_from_config()` returns `None` instead of panicking. The CLI prints a human-readable warning:

```
warn!("no API key found for provider '{}' (set {} in environment)",
      provider_name, provider_config.api_key_env);
```

### 5.2 Edge Cases

1. **Empty model string**: `config.agents.defaults.model` is empty. `create_adapter_from_config()` returns `None`. Agent falls back to stub transport that returns an error explaining no provider is configured.

2. **Model with no prefix**: e.g. `"gpt-4o"` without `"openai/"` prefix. `ProviderRouter::route()` falls through to the default provider (first builtin, which is `openai`). Adapter is created for OpenAI.

3. **Multiple tool iterations in agent CLI**: Works identically to gateway because both use the same `AgentLoop::process_message()` / `run_tool_loop()` implementation.

4. **Bus closure during interactive mode**: `consume_outbound()` returns `None`. Print error and break the REPL gracefully.

5. **Concurrent inbound in gateway**: `AgentLoop::run()` processes one message at a time (sequential loop). This is correct for v1. Future optimization: spawn per-message tasks.

6. **Unicode content**: Both `ChatMessage` and `OutboundMessage` use `String`, which is always valid UTF-8. No special handling needed.

7. **Large responses**: The markdown converters may produce output larger than the channel's message limit (Telegram: 4096 chars, Discord: 2000 chars). Chunking is a channel plugin responsibility (Stream 2A), not an integration wiring concern.

### 5.3 Testing Strategy

#### Unit Tests (~20 tests)

**`llm_adapter.rs` tests** (~10 tests):
- `adapter_text_response_roundtrip` -- mock Provider returns text, verify JSON shape
- `adapter_tool_call_response_roundtrip` -- mock Provider returns tool calls
- `adapter_error_maps_to_string` -- ProviderError becomes Err(String)
- `adapter_converts_system_message` -- system role survives roundtrip
- `adapter_converts_tool_result_message` -- tool_call_id is forwarded
- `adapter_forwards_tools_array` -- tools JSON is passed through unchanged
- `adapter_forwards_max_tokens_and_temperature` -- parameters forwarded
- `create_adapter_from_config_returns_none_without_key` -- env var not set
- `create_adapter_from_config_returns_some_with_key` -- env var set (mock)
- `create_adapter_from_config_empty_model` -- returns None

**`dispatch.rs` tests** (~5 tests):
- `dispatch_telegram_applies_html` -- verify HTML output
- `dispatch_slack_applies_mrkdwn` -- verify mrkdwn output
- `dispatch_discord_passthrough` -- verify passthrough
- `dispatch_unknown_channel_identity` -- returns unchanged content
- `dispatch_new_registers_all_builtins` -- all three channels present

**`agent.rs` tests** (~3 tests):
- `agent_args_defaults` (existing)
- `agent_args_with_message` (existing)
- `agent_args_with_model_override` (existing, verify it's applied)

**`gateway.rs` tests** (~2 tests):
- `gateway_args_defaults` (existing)
- `gateway_args_with_config` (existing)

#### Integration Tests (~5 tests)

In `clawft/tests/integration/` or `clawft-cli/tests/`:

- `agent_single_message_with_mock_provider` -- create AppContext with mock Provider, run single message, verify stdout output
- `agent_interactive_exit` -- spawn interactive, send "/exit", verify clean shutdown
- `gateway_inbound_to_outbound_with_mock` -- publish InboundMessage, verify OutboundMessage appears with converted markdown
- `adapter_full_pipeline_integration` -- create AppContext with ClawftLlmAdapter wrapping a mock, verify PipelineRegistry.complete() returns valid LlmResponse
- `gateway_graceful_shutdown` -- start gateway, send cancellation, verify all tasks stop

#### Coverage Targets

- `llm_adapter.rs`: >= 90% line coverage
- `dispatch.rs`: >= 95% line coverage
- `agent.rs`: >= 80% line coverage (interactive mode is hard to test fully)
- `gateway.rs`: >= 70% line coverage (background tasks are hard to test)
- Overall stream: >= 80%

### 5.4 Performance Considerations

- **Adapter overhead**: The JSON serialization roundtrip in `ClawftLlmAdapter` adds < 1ms per call. Negligible compared to LLM latency (500ms-5s).
- **Bus throughput**: Unbounded MPSC channels handle thousands of messages per second. No bottleneck.
- **Markdown conversion**: `pulldown-cmark` parsing is O(n) in content length. Sub-millisecond for typical messages (< 10KB).

---

## 6. Completion

### 6.1 Implementation Order

| Step | Task | Est. Lines | Depends On |
|------|------|-----------|------------|
| 1 | `pipeline/llm_adapter.rs` + tests | 150-250 | -- |
| 2 | `pipeline/mod.rs` registration | 2 | Step 1 |
| 3 | `bootstrap.rs` `build_live_pipeline()` | 20-30 | Step 1 |
| 4 | `markdown/dispatch.rs` + tests | 30-50 | -- |
| 5 | `markdown/mod.rs` registration | 2 | Step 4 |
| 6 | Wire `agent.rs` | 100-200 | Steps 1-3 |
| 7 | Wire `gateway.rs` | 80-150 | Steps 1-5 |
| 8 | Integration tests | 100-150 | Steps 6-7 |

Steps 1 and 4 can be implemented in parallel. Steps 6 and 7 can be implemented in parallel after steps 1-5.

### 6.2 Integration Checklist

- [ ] `clawft-core/Cargo.toml` has `clawft-llm` as optional dependency with `llm` feature
- [ ] `clawft-cli/Cargo.toml` enables `clawft-core/llm` feature
- [ ] `clawft-cli/Cargo.toml` has `tokio-util` for `CancellationToken`
- [ ] `pipeline/llm_adapter.rs` created and registered in `pipeline/mod.rs`
- [ ] `bootstrap.rs` has `build_live_pipeline()` function
- [ ] `markdown/dispatch.rs` created and registered in `markdown/mod.rs`
- [ ] `#[allow(dead_code)]` removed from `MarkdownConverter` trait
- [ ] `agent.rs` creates `AppContext`, wires adapter, uses real `AgentLoop`
- [ ] `gateway.rs` spawns `AgentLoop::run()` as background task
- [ ] `gateway.rs` spawns outbound dispatch loop with `MarkdownDispatcher`
- [ ] `gateway.rs` uses `CancellationToken` for graceful shutdown
- [ ] All existing tests still pass (`cargo test --workspace`)
- [ ] New unit tests pass
- [ ] New integration tests pass
- [ ] `cargo clippy --workspace` has no new warnings
- [ ] `cargo build --release` succeeds
- [ ] Manual smoke test: `weft agent -m "hello"` with a real API key returns a response
- [ ] Manual smoke test: `weft gateway` with Telegram sends and receives messages

### 6.3 Quality Gates

| Gate | Criteria | Tool |
|------|----------|------|
| Compilation | `cargo build --workspace` succeeds | CI |
| Linting | `cargo clippy --workspace -- -D warnings` passes | CI |
| Unit tests | `cargo test --workspace` passes | CI |
| Coverage | >= 80% overall, >= 90% for `llm_adapter.rs` | `cargo-tarpaulin` |
| No regressions | All pre-existing tests still pass | CI |
| Feature gate | `cargo build --workspace --no-default-features` succeeds (WASM path) | CI |
| Manual E2E | `weft agent -m "What is 2+2?"` returns "4" (or similar) with real API key | Developer |

### 6.4 Cross-Stream Dependencies

| This Stream Provides | Consumer Stream |
|----------------------|-----------------|
| Working `weft agent` CLI | Stream 2D (CLI completion, --format, --session flags) |
| Working `weft gateway` outbound dispatch | Stream 2A (Slack + Discord channel plugins) |
| `MarkdownDispatcher` | Stream 2A (new converters register here) |
| `ClawftLlmAdapter` | Stream 2B (RVF integration may need custom Provider) |
| `build_live_pipeline()` | Stream 3A (WASM builds disable `llm` feature, use stub) |

### 6.5 Rollback Plan

If the adapter introduces type incompatibilities:
1. Feature-gate the entire `llm_adapter` module behind `#[cfg(feature = "llm")]`
2. The `agent.rs` and `gateway.rs` changes are guarded by `if let Some(adapter) = ...`
3. Without the feature, everything falls back to stub transport (existing behavior)
4. No breaking changes to any public API

### 6.6 Security Considerations

- API keys are resolved from environment variables at runtime, never stored in config files or logs
- The adapter does not log request content (may contain PII)
- The adapter does not log the API key value
- `tracing::debug!` logs include model name and message count, not message content
- The `with_api_key()` constructor is used instead of environment-based resolution to avoid TOCTOU issues in the adapter (key resolved once at startup)
