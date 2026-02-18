# SPARC Implementation Plan: Stream 1C - Provider + Tools + CLI + Telegram

**Stream**: 1C - Provider + Tools + CLI + Telegram (Phase 1: Warp, Week 3-8)
**Owner**: Application engineer
**Depends on**: Stream 1A platform API + Channel trait (week 3), Stream 1B tool registry (week 4)
**Branch**: `weft/provider-tools-cli`
**Timeline**: Week 3-8 (6 weeks)

## Agent Instructions

This is a **planning document**, NOT implementation code. When working on this stream:

1. **Read dependencies first**:
   - `/home/aepod/dev/barni/repos/nanobot/.planning/02-technical-requirements.md` - clawft-llm spec, tool table, channel plugin API
   - `/home/aepod/dev/barni/repos/nanobot/.planning/03-development-guide.md` - Stream 1C schedule
   - `/home/aepod/dev/barni/repos/nanobot/.planning/06-provider-layer-options.md` - Provider extraction plan, provider table with quirks
   - `/home/aepod/dev/barni/repos/nanobot/.planning/05-ruvector-crates.md` - Pipeline trait details (Section 11)

2. **Work incrementally by week** (Week 3 → Week 8):
   - Week 3: Provider extraction only
   - Week 4: Provider features only
   - Week 5-6: Tools implementation
   - Week 7: Telegram plugin + CLI agent
   - Week 8: CLI gateway + onboard/status

3. **Test-driven development**:
   - Write tests BEFORE implementation (TDD London School - mock-first)
   - Use mock HTTP servers for provider tests
   - Use telegram API fixtures for channel tests
   - ALWAYS run `cargo test` after changes

4. **File organization**:
   - clawft-llm is a **separate repository** (NOT in clawft monorepo)
   - clawft-tools goes in `clawft/crates/tools/`
   - Telegram plugin goes in `clawft/crates/channels/telegram/`
   - CLI commands go in `clawft/crates/cli/src/commands/`

5. **Validation before marking complete**:
   - Real Telegram bot receives and responds to messages
   - CLI interactive mode with history and markdown rendering works
   - `weft status` shows correct provider configuration
   - All tests pass (`cargo test --workspace`)

6. **Security rules**:
   - NEVER commit API keys or secrets
   - Always validate file paths (prevent directory traversal)
   - Always sanitize shell commands (prevent injection)
   - Run `npx @claude-flow/cli@latest security scan` after changes

---

## 1. Specification

### 1.1 Week 3: clawft-llm Extraction

**Goal**: Extract `clawft-llm` from `barni/src/layer3/barni-providers/` as a standalone library crate in a separate repository.

#### Requirements

**R1.1**: **Repository Setup**
- Create new repository `clawft-llm` (NOT in clawft monorepo)
- Copy 11 source files from `barni/src/layer3/barni-providers/src/llm/` (10,563 lines)
- Preserve existing structure: `providers/`, `traits/`, `types/`, `failover/`
- Initialize with Apache 2.0 license, README, CI/CD

**R1.2**: **Dependency Removal**
- Remove ALL barni-specific imports from 2 files:
  - `request.rs`: Replace `RequestId`, `SessionId`, `TenantId` newtypes with `Option<String>`
  - `failover.rs`: Internalize `CircuitBreaker` (~150-200 lines) + `now_ms()` helper
- Remove barni-config dependency (use serde/toml directly)
- Remove barni-tracing dependency (use tracing directly)

**R1.3**: **4 Native Providers**
- Preserve existing implementations:
  - `AnthropicProvider` - Claude 3.x, prompt caching, thinking tokens
  - `OpenAiProvider` - GPT-4o, o1, structured outputs
  - `BedrockProvider` - AWS authentication, Claude via Bedrock
  - `GeminiProvider` - Gemini 1.5/2.0, Google Cloud auth

**R1.4**: **Config-Driven OpenAI-Compatible Provider**
- Add `OpenAiCompatProvider` for ANY base_url:
  - Configurable endpoint: `base_url`, `api_key`, `model_prefix`
  - Supports 15 providers via configuration (Groq, DeepSeek, Mistral, etc.)
  - Handles quirks via provider-specific logic (e.g., DeepSeek `reasoning_content`)
- Add provider config struct:
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct ProviderConfig {
      pub name: String,
      pub base_url: String,
      pub api_key_env: String,
      pub model_prefix: Option<String>,
      pub quirks: Option<Vec<ProviderQuirk>>,
  }
  ```

**R1.5**: **Streaming + Tool Calling**
- Preserve full SSE streaming support (all providers)
- Preserve tool calling support (Anthropic, OpenAI, Gemini)
- Preserve 4 failover strategies: Retry, Failover, LoadBalance, CircuitBreaker

**R1.6**: **Testing**
- Port ALL existing tests from barni-providers
- Add mock HTTP server tests for each provider
- Add integration tests with real API keys (CI only)
- Target: >90% code coverage

#### Acceptance Criteria

- [x] clawft-llm compiles as standalone crate
- [x] No barni dependencies (only std Rust crates)
- [ ] All 4 native providers work (tested via mocks)
- [ ] OpenAiCompatProvider works with Groq, DeepSeek, Mistral (tested via mocks)
- [x] All tests pass (`cargo test`)
- [ ] Published to crates.io as v0.1.0

---

### 1.2 Week 4: Provider Features

**Goal**: Add model routing, aliasing, and JSON repair to `clawft-llm`.

#### Requirements

**R2.1**: **Prefix-Based Model Routing**
- Auto-route models based on prefix:
  - `anthropic/claude-3-sonnet-20240229` → AnthropicProvider
  - `openai/gpt-4o` → OpenAiProvider
  - `bedrock/anthropic.claude-3-sonnet-20240229-v1:0` → BedrockProvider
  - `gemini/gemini-2.0-flash-exp` → GeminiProvider
  - `groq/llama-3.1-70b` → OpenAiCompatProvider (Groq config)
- Implement `ProviderRouter` struct:
  ```rust
  pub struct ProviderRouter {
      providers: HashMap<String, Box<dyn Provider>>,
      aliases: HashMap<String, String>,
  }

  impl ProviderRouter {
      pub fn route(&self, model: &str) -> Result<&dyn Provider>;
  }
  ```

**R2.2**: **Model Aliasing**
- Support short aliases for common models:
  - `claude-3-sonnet` → `anthropic/claude-3-sonnet-20240229`
  - `gpt-4o` → `openai/gpt-4o`
  - `llama-3-70b` → `groq/llama-3.1-70b-versatile`
- Load aliases from config file (`~/.config/clawft/aliases.toml`)

**R2.3**: **JSON Repair**
- Repair malformed JSON in LLM tool call responses:
  - Missing closing braces: `{"key": "value"` → `{"key": "value"}`
  - Unquoted keys: `{key: "value"}` → `{"key": "value"}`
  - Trailing commas: `{"a": 1,}` → `{"a": 1}`
  - Single quotes: `{'key': 'value'}` → `{"key": "value"}`
- Implement `repair_json(input: &str) -> Result<String>` function
- Use regex + heuristics (no external JSON repair crate)

**R2.4**: **Testing**
- Test routing with all provider prefixes
- Test aliasing with common model names
- Test JSON repair with 20+ malformed examples
- Target: >90% code coverage

#### Acceptance Criteria

- [x] Model routing works for all 19 providers (ProviderRouter implemented)
- [x] Aliases resolve correctly
- [ ] JSON repair fixes 95% of common malformed outputs
- [x] All tests pass (`cargo test`)
- [ ] Published to crates.io as v0.2.0

---

### 1.3 Week 5: File + Shell Tools

**Goal**: Implement file and shell execution tools in `clawft-tools` crate.

#### Requirements

**R3.1**: **File Tools**
- Implement 4 file tools (port from `nanobot/agent/tools/filesystem.py`):
  - `read_file(path: String) -> Result<String>` - Read file contents
  - `write_file(path: String, content: String) -> Result<()>` - Write/overwrite file
  - `edit_file(path: String, old: String, new: String) -> Result<()>` - Find/replace edit
  - `list_directory(path: String) -> Result<Vec<FileInfo>>` - List dir contents
- Path validation:
  - Resolve canonical paths (prevent `../` traversal)
  - Restrict to workspace directory (configurable via env var)
  - Reject absolute paths outside workspace
- Error handling:
  - Return `ToolError::NotFound` if file doesn't exist
  - Return `ToolError::PermissionDenied` if not readable/writable
  - Return `ToolError::InvalidPath` if path validation fails

**R3.2**: **Shell Execution Tool**
- Implement `exec_shell(command: String, timeout_secs: u64) -> Result<ExecResult>` (port from `nanobot/agent/tools/shell.py`)
- Feature gate: `tool-exec` (disabled by default for security)
- Security measures:
  - Timeout (default 30s, max 300s)
  - Workspace restriction (chdir to workspace before exec)
  - Command sanitization (reject `rm -rf /`, `sudo`, etc.)
  - Capture stdout + stderr separately
- Return struct:
  ```rust
  pub struct ExecResult {
      pub exit_code: i32,
      pub stdout: String,
      pub stderr: String,
      pub duration_ms: u64,
  }
  ```

**R3.3**: **Tool Trait Implementation**
- All tools implement `Tool` trait from clawft-platform:
  ```rust
  #[async_trait]
  pub trait Tool: Send + Sync {
      fn name(&self) -> &str;
      fn description(&self) -> &str;
      fn parameters(&self) -> serde_json::Value; // JSON Schema
      async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;
  }
  ```
- Register tools via `ToolRegistry::register_all()` function

**R3.4**: **Testing**
- Test file tools with tempdir fixtures
- Test shell exec with simple commands (`echo`, `ls`)
- Test path validation (reject `../etc/passwd`)
- Test timeout enforcement
- Target: >90% code coverage

#### Acceptance Criteria

- [x] All 5 tools compile and implement Tool trait
- [x] File tools work with test fixtures
- [x] Shell exec works with timeout and workspace restriction
- [x] Path validation rejects malicious paths
- [x] All tests pass (`cargo test`)

---

### 1.4 Week 6: Web + Bus Tools

**Goal**: Implement web search, web fetch, message, and spawn tools in `clawft-tools`.

#### Requirements

**R4.1**: **Web Search Tool**
- Implement `search_web(query: String, num_results: u32) -> Result<Vec<SearchResult>>` (port from `nanobot/agent/tools/web.py`)
- Use Brave Search API (config: `BRAVE_API_KEY` env var)
- Feature gate: `tool-web`
- Return struct:
  ```rust
  pub struct SearchResult {
      pub title: String,
      pub url: String,
      pub description: String,
      pub published_date: Option<String>,
  }
  ```

**R4.2**: **Web Fetch Tool**
- Implement `fetch_web(url: String) -> Result<String>` (port from `nanobot/agent/tools/web.py`)
- Use readability extraction (mozilla/readability port or trafilatura-like logic)
- Feature gate: `tool-web`
- Return markdown-formatted article text
- Error handling:
  - Return `ToolError::HttpError` if fetch fails
  - Return `ToolError::ParseError` if readability fails

**R4.3**: **Message Tool**
- Implement `send_message(recipient: String, content: String) -> Result<()>` (port from `nanobot/agent/tools/message.py`)
- Use message bus from clawft-platform (via `MessageBus` trait)
- Feature gate: `tool-spawn` (message requires bus)
- Recipient types:
  - `agent:<agent_id>` - Send to specific agent
  - `channel:<channel_id>` - Send to channel (broadcast)

**R4.4**: **Spawn Tool**
- Implement `spawn_agent(agent_type: String, task: String) -> Result<String>` (port from `nanobot/agent/tools/spawn.py`)
- Use agent factory from clawft-platform (via `AgentFactory` trait)
- Feature gate: `tool-spawn`
- Return spawned agent ID
- Agent types: `researcher`, `coder`, `tester`, `reviewer`

**R4.5**: **Testing**
- Test web search with mock Brave API server
- Test web fetch with mock HTTP server + sample HTML
- Test message tool with mock message bus
- Test spawn tool with mock agent factory
- Target: >90% code coverage

#### Acceptance Criteria

- [ ] Web search returns valid results (tested via mock)
- [ ] Web fetch extracts readable text (tested via mock)
- [x] Message tool sends to bus (tested via mock)
- [ ] Spawn tool creates agent (tested via mock)
- [x] All tests pass (`cargo test`)

---

### 1.5 Week 7: Telegram Plugin + CLI Agent

**Goal**: Implement Telegram channel plugin and CLI agent command.

#### Requirements

**R5.1**: **Telegram Channel Plugin**
- Create `clawft/crates/channels/telegram/` plugin crate (port from `nanobot/channels/telegram.py`)
- Implement `Channel` trait from clawft-platform:
  ```rust
  #[async_trait]
  pub trait Channel: Send + Sync {
      async fn start(&mut self) -> Result<()>;
      async fn stop(&mut self) -> Result<()>;
      async fn send(&self, message: OutboundMessage) -> Result<()>;
      fn subscribe(&self) -> mpsc::Receiver<InboundMessage>;
  }
  ```
- Use long-polling via Telegram Bot API (NOT WebSocket):
  - `getUpdates` endpoint with offset tracking
  - Poll interval: 1s
  - Timeout: 30s (long-polling parameter)
- Custom REST client (~200 lines, no external Telegram SDK):
  ```rust
  pub struct TelegramClient {
      token: String,
      http: reqwest::Client,
  }

  impl TelegramClient {
      pub async fn get_updates(&self, offset: i64) -> Result<Vec<Update>>;
      pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<()>;
  }
  ```
- Message parsing:
  - Parse Telegram `Update` → `InboundMessage` (extract text, user, chat_id)
  - Format `OutboundMessage` → Telegram `sendMessage` request (markdown rendering)
- Voice support (optional, week 7 stretch goal):
  - Download voice message via `getFile` endpoint
  - Convert to text via Whisper API (OpenAI or Groq)

**R5.2**: **CLI Agent Command**
- Implement `weft agent` command in `clawft/crates/cli/src/commands/agent.rs`
- Two modes:
  - **Interactive mode**: `weft agent` (no args)
    - Use rustyline for REPL with history
    - Render agent responses with markdown (termimad or similar)
    - Support `/exit`, `/clear`, `/help` commands
  - **Single-message mode**: `weft agent -m "hello"`
    - Send message, print response, exit
- Agent initialization:
  - Load config from `~/.config/clawft/agent.toml`
  - Initialize provider (default: anthropic/claude-3-sonnet)
  - Initialize tools (file, shell, web, message, spawn)
  - Create agent instance via `AgentFactory`

**R5.3**: **Testing**
- Test Telegram client with mock API server (fixtures for `getUpdates`, `sendMessage`)
- Test channel plugin with mock InboundMessage/OutboundMessage
- Test CLI agent interactive mode (mock stdin/stdout)
- Test CLI agent single-message mode (integration test)
- Target: >85% code coverage

#### Acceptance Criteria

- [ ] Telegram plugin receives and sends messages (tested via mock API)
- [ ] CLI interactive mode works with history and markdown rendering
- [ ] CLI single-message mode works
- [ ] All tests pass (`cargo test`)

---

### 1.6 Week 8: CLI Gateway + Onboard/Status

**Goal**: Implement CLI gateway command and onboard/status commands.

#### Requirements

**R6.1**: **CLI Gateway Command**
- Implement `weft gateway` command in `clawft/crates/cli/src/commands/gateway.rs`
- Gateway responsibilities:
  - **Plugin host**: Load and initialize channel plugins (Telegram, Slack, Discord)
  - **Agent loop**: Process InboundMessage → Agent → OutboundMessage
  - **Dispatch**: Route OutboundMessage to correct channel
- Plugin loading:
  - Scan `~/.config/clawft/plugins/` directory
  - Load dynamic libraries (`.so`, `.dylib`, `.dll`)
  - Validate plugin implements `ChannelPlugin` trait
  - Initialize plugin with config
- Agent loop:
  ```rust
  loop {
      let msg = rx.recv().await?; // From channel plugin
      let response = agent.process(msg).await?;
      channel.send(response).await?;
  }
  ```
- Graceful shutdown:
  - Handle SIGINT/SIGTERM
  - Stop all plugins
  - Flush message queue

**R6.2**: **CLI Onboard Command**
- Implement `weft onboard` command in `clawft/crates/cli/src/commands/onboard.rs`
- Interactive wizard:
  - Create `~/.config/clawft/` directory
  - Create `agent.toml` config file (provider, model, tools)
  - Create `plugins.toml` config file (enabled channels)
  - Create `workspace/` directory (for file tools)
  - Prompt for API keys (Anthropic, OpenAI, Brave, Telegram)
  - Validate API keys (test provider connection)
- Example `agent.toml`:
  ```toml
  [provider]
  name = "anthropic"
  model = "claude-3-sonnet-20240229"

  [tools]
  file = true
  shell = false
  web = true
  spawn = true
  ```

**R6.3**: **CLI Status Command**
- Implement `weft status` command in `clawft/crates/cli/src/commands/status.rs`
- Display:
  - Config file location (`~/.config/clawft/agent.toml`)
  - Provider status (name, model, API key set)
  - Tool status (enabled/disabled)
  - Plugin status (loaded channels)
  - Workspace directory
- Test provider connection (send test request to LLM API)
- Color-coded output (green = OK, red = error)

**R6.4**: **Testing**
- Test gateway with mock channel plugins
- Test onboard wizard (mock stdin)
- Test status command (integration test)
- Target: >85% code coverage

#### Acceptance Criteria

- [ ] `weft gateway` starts and runs agent loop (tested with mock plugin)
- [ ] `weft onboard` creates config files and validates API keys
- [ ] `weft status` shows correct provider and tool status
- [ ] All tests pass (`cargo test`)

---

## 2. Pseudocode

### 2.1 Provider Routing Algorithm

```rust
// Week 4: Prefix-based model routing
struct ProviderRouter {
    providers: HashMap<String, Box<dyn Provider>>,
    aliases: HashMap<String, String>,
    openai_compat_configs: HashMap<String, ProviderConfig>,
}

impl ProviderRouter {
    fn route(&self, model: &str) -> Result<&dyn Provider> {
        // Step 1: Resolve alias
        let resolved_model = self.aliases.get(model).unwrap_or(model);

        // Step 2: Extract prefix (before /)
        let (prefix, model_name) = match resolved_model.split_once('/') {
            Some((p, m)) => (p, m),
            None => return Err(ProviderError::InvalidModelFormat),
        };

        // Step 3: Match prefix to provider
        match prefix {
            "anthropic" => self.providers.get("anthropic"),
            "openai" => self.providers.get("openai"),
            "bedrock" => self.providers.get("bedrock"),
            "gemini" => self.providers.get("gemini"),
            _ => {
                // Step 4: Check if OpenAI-compatible provider
                if let Some(config) = self.openai_compat_configs.get(prefix) {
                    // Create OpenAiCompatProvider on-the-fly
                    self.create_compat_provider(config)
                } else {
                    Err(ProviderError::UnknownProvider(prefix))
                }
            }
        }
    }
}
```

### 2.2 JSON Repair Algorithm

```rust
// Week 4: JSON repair for malformed LLM outputs
fn repair_json(input: &str) -> Result<String> {
    let mut repaired = input.to_string();

    // Step 1: Add missing closing braces
    let open_braces = repaired.matches('{').count();
    let close_braces = repaired.matches('}').count();
    if open_braces > close_braces {
        repaired.push_str(&"}".repeat(open_braces - close_braces));
    }

    // Step 2: Quote unquoted keys (regex: {key: → {"key":)
    let key_regex = Regex::new(r#"\{([a-zA-Z_][a-zA-Z0-9_]*)\s*:"#)?;
    repaired = key_regex.replace_all(&repaired, r#"{"$1":"#).to_string();

    // Step 3: Remove trailing commas
    let trailing_comma_regex = Regex::new(r#",\s*([}\]])"#)?;
    repaired = trailing_comma_regex.replace_all(&repaired, r#"$1"#).to_string();

    // Step 4: Replace single quotes with double quotes
    repaired = repaired.replace("'", "\"");

    // Step 5: Validate repaired JSON
    serde_json::from_str::<serde_json::Value>(&repaired)?;

    Ok(repaired)
}
```

### 2.3 Telegram Long-Polling Algorithm

```rust
// Week 7: Telegram long-polling with offset tracking
async fn telegram_polling_loop(client: TelegramClient, tx: mpsc::Sender<InboundMessage>) {
    let mut offset: i64 = 0;

    loop {
        // Step 1: Poll for updates (30s timeout)
        match client.get_updates(offset).await {
            Ok(updates) => {
                for update in updates {
                    // Step 2: Parse update → InboundMessage
                    if let Some(message) = update.message {
                        let inbound = InboundMessage {
                            id: message.message_id.to_string(),
                            channel_id: "telegram".to_string(),
                            user_id: message.from.id.to_string(),
                            content: message.text.unwrap_or_default(),
                            metadata: serde_json::json!({
                                "chat_id": message.chat.id,
                            }),
                        };

                        // Step 3: Send to agent loop
                        tx.send(inbound).await.ok();
                    }

                    // Step 4: Update offset (next update_id)
                    offset = update.update_id + 1;
                }
            }
            Err(e) => {
                eprintln!("Polling error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
```

### 2.4 CLI Interactive Mode Algorithm

```rust
// Week 7: CLI interactive mode with rustyline
async fn interactive_mode(agent: Agent) -> Result<()> {
    let mut rl = Editor::<()>::new()?;

    // Step 1: Load history from ~/.clawft_history
    let history_path = dirs::home_dir().unwrap().join(".clawft_history");
    rl.load_history(&history_path).ok();

    loop {
        // Step 2: Read user input
        let readline = rl.readline("weft> ");
        match readline {
            Ok(line) => {
                // Step 3: Handle commands
                match line.trim() {
                    "/exit" => break,
                    "/clear" => {
                        rl.clear_history();
                        continue;
                    }
                    "/help" => {
                        println!("Commands: /exit, /clear, /help");
                        continue;
                    }
                    _ => {}
                }

                // Step 4: Add to history
                rl.add_history_entry(&line);

                // Step 5: Send to agent
                let response = agent.process_message(&line).await?;

                // Step 6: Render markdown response
                let skin = MadSkin::default();
                skin.print_text(&response);
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => return Err(err.into()),
        }
    }

    // Step 7: Save history
    rl.save_history(&history_path)?;
    Ok(())
}
```

### 2.5 Gateway Plugin Loading Algorithm

```rust
// Week 8: Dynamic plugin loading
async fn load_plugins(plugin_dir: &Path) -> Result<Vec<Box<dyn ChannelPlugin>>> {
    let mut plugins = Vec::new();

    // Step 1: Scan plugin directory
    for entry in fs::read_dir(plugin_dir)? {
        let path = entry?.path();

        // Step 2: Load dynamic library
        if path.extension() == Some(OsStr::new("so")) {
            unsafe {
                let lib = Library::new(&path)?;

                // Step 3: Get plugin factory function
                let factory: Symbol<PluginFactory> = lib.get(b"create_plugin")?;

                // Step 4: Create plugin instance
                let plugin = factory();

                // Step 5: Initialize plugin
                let config = load_plugin_config(&path)?;
                plugin.init(config).await?;

                plugins.push(plugin);
            }
        }
    }

    Ok(plugins)
}

// Plugin gateway agent loop
async fn gateway_loop(plugins: Vec<Box<dyn ChannelPlugin>>, agent: Agent) {
    // Step 1: Start all plugins
    for plugin in &plugins {
        plugin.start().await.ok();
    }

    // Step 2: Multiplex all channel receivers
    let (tx, mut rx) = mpsc::channel(100);
    for plugin in &plugins {
        let plugin_rx = plugin.subscribe();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = plugin_rx.recv().await {
                tx_clone.send(msg).await.ok();
            }
        });
    }

    // Step 3: Process messages
    while let Some(inbound) = rx.recv().await {
        // Step 4: Agent processing
        let response = agent.process(inbound).await.ok();

        // Step 5: Route response to channel
        if let Some(outbound) = response {
            let channel_id = &outbound.channel_id;
            if let Some(plugin) = plugins.iter().find(|p| p.id() == channel_id) {
                plugin.send(outbound).await.ok();
            }
        }
    }
}
```

---

## 3. Architecture

### 3.1 Crate Structure

```
clawft-llm/                   # Separate repository (NOT in clawft monorepo)
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── anthropic.rs      # AnthropicProvider
│   │   ├── openai.rs         # OpenAiProvider
│   │   ├── bedrock.rs        # BedrockProvider
│   │   ├── gemini.rs         # GeminiProvider
│   │   └── openai_compat.rs  # OpenAiCompatProvider (NEW)
│   ├── traits/
│   │   ├── mod.rs
│   │   └── provider.rs       # Provider trait
│   ├── types/
│   │   ├── mod.rs
│   │   ├── request.rs        # ChatRequest, Message (NO barni deps)
│   │   └── response.rs       # ChatResponse, ToolCall
│   ├── failover/
│   │   ├── mod.rs
│   │   ├── circuit_breaker.rs # Internalized CircuitBreaker
│   │   └── strategies.rs     # 4 failover strategies
│   ├── router.rs             # ProviderRouter (NEW - Week 4)
│   └── json_repair.rs        # repair_json() (NEW - Week 4)
└── tests/
    ├── anthropic_tests.rs
    ├── openai_tests.rs
    ├── bedrock_tests.rs
    ├── gemini_tests.rs
    ├── openai_compat_tests.rs # NEW
    ├── router_tests.rs        # NEW
    └── json_repair_tests.rs   # NEW

clawft/crates/tools/          # clawft-tools crate
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── file/
│   │   ├── mod.rs
│   │   ├── read.rs           # ReadFileTool
│   │   ├── write.rs          # WriteFileTool
│   │   ├── edit.rs           # EditFileTool
│   │   └── list.rs           # ListDirectoryTool
│   ├── shell/
│   │   ├── mod.rs
│   │   └── exec.rs           # ExecShellTool (feature: tool-exec)
│   ├── web/
│   │   ├── mod.rs
│   │   ├── search.rs         # SearchWebTool (feature: tool-web)
│   │   └── fetch.rs          # FetchWebTool (feature: tool-web)
│   ├── bus/
│   │   ├── mod.rs
│   │   ├── message.rs        # SendMessageTool (feature: tool-spawn)
│   │   └── spawn.rs          # SpawnAgentTool (feature: tool-spawn)
│   └── registry.rs           # ToolRegistry::register_all()
└── tests/
    ├── file_tests.rs
    ├── shell_tests.rs
    ├── web_tests.rs
    └── bus_tests.rs

clawft/crates/channels/telegram/  # Telegram plugin
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── client.rs             # TelegramClient (REST API)
│   ├── polling.rs            # Long-polling loop
│   ├── types.rs              # Update, Message, Chat
│   └── plugin.rs             # TelegramPlugin (implements Channel trait)
└── tests/
    ├── client_tests.rs
    └── plugin_tests.rs

clawft/crates/cli/            # CLI binary
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── agent.rs          # `weft agent` (Week 7)
│   │   ├── gateway.rs        # `weft gateway` (Week 8)
│   │   ├── onboard.rs        # `weft onboard` (Week 8)
│   │   └── status.rs         # `weft status` (Week 8)
│   └── config.rs             # Config loading
└── tests/
    ├── agent_tests.rs
    ├── gateway_tests.rs
    ├── onboard_tests.rs
    └── status_tests.rs
```

### 3.2 Dependency Graph

```
clawft-cli
  └─> clawft-platform (Platform, Agent, Channel traits)
  └─> clawft-tools (ToolRegistry)
  └─> clawft-llm (ProviderRouter)
  └─> telegram plugin (dynamic library)

clawft-tools
  └─> clawft-platform (Tool trait)

telegram plugin
  └─> clawft-platform (Channel trait)

clawft-llm
  └─> (NO clawft dependencies - standalone library)
```

### 3.3 Provider Configuration

**Example `~/.config/clawft/providers.toml`:**

```toml
# Native providers (built-in)
[providers.anthropic]
type = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"

[providers.openai]
type = "openai"
api_key_env = "OPENAI_API_KEY"

# OpenAI-compatible providers (config-driven)
[providers.groq]
type = "openai_compat"
base_url = "https://api.groq.com/openai/v1"
api_key_env = "GROQ_API_KEY"
model_prefix = "groq"

[providers.deepseek]
type = "openai_compat"
base_url = "https://api.deepseek.com"
api_key_env = "DEEPSEEK_API_KEY"
model_prefix = "deepseek"
quirks = ["reasoning_content"]  # Extract from reasoning_content field

[providers.ollama]
type = "openai_compat"
base_url = "http://localhost:11434/v1"
model_prefix = "ollama"
# No API key for local Ollama

# Model aliases
[aliases]
"claude-3-sonnet" = "anthropic/claude-3-sonnet-20240229"
"gpt-4o" = "openai/gpt-4o"
"llama-3-70b" = "groq/llama-3.1-70b-versatile"
"deepseek-r1" = "deepseek/deepseek-reasoner"
```

### 3.4 Tool Feature Flags

**Cargo.toml for clawft-tools:**

```toml
[features]
default = ["file"]  # File tools always included
file = []           # Read, write, edit, list_directory
tool-exec = []      # Shell execution (security risk - opt-in)
tool-web = ["reqwest", "scraper"]  # Web search + fetch
tool-spawn = []     # Message + spawn (requires bus)
all-tools = ["tool-exec", "tool-web", "tool-spawn"]
```

### 3.5 Channel Plugin Interface

```rust
// clawft-platform/src/channel.rs
#[async_trait]
pub trait Channel: Send + Sync {
    /// Start the channel (begin polling/listening)
    async fn start(&mut self) -> Result<()>;

    /// Stop the channel (cleanup)
    async fn stop(&mut self) -> Result<()>;

    /// Send outbound message
    async fn send(&self, message: OutboundMessage) -> Result<()>;

    /// Subscribe to inbound messages (mpsc channel)
    fn subscribe(&self) -> mpsc::Receiver<InboundMessage>;
}

// Plugin factory function (exported by each plugin .so)
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Channel> {
    Box::new(TelegramPlugin::new())
}
```

---

## 4. Refinement (TDD Test Plan)

### 4.1 Week 3: clawft-llm Extraction Tests

**Test File**: `clawft-llm/tests/provider_tests.rs`

```rust
#[tokio::test]
async fn test_anthropic_provider_no_barni_deps() {
    // Verify NO barni imports compile successfully
    let provider = AnthropicProvider::new("test-key");
    assert_eq!(provider.name(), "anthropic");
}

#[tokio::test]
async fn test_circuit_breaker_internalized() {
    // Verify internalized CircuitBreaker works
    let mut cb = CircuitBreaker::new(3, Duration::from_secs(60));
    assert_eq!(cb.state(), CircuitBreakerState::Closed);

    // Trip circuit after 3 failures
    for _ in 0..3 {
        cb.record_failure();
    }
    assert_eq!(cb.state(), CircuitBreakerState::Open);
}

#[tokio::test]
async fn test_openai_compat_provider_groq() {
    // Mock HTTP server returning Groq-style response
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "Hello from Groq"}}]
        })))
        .mount(&mock_server)
        .await;

    let config = ProviderConfig {
        name: "groq".to_string(),
        base_url: mock_server.uri(),
        api_key_env: "GROQ_API_KEY".to_string(),
        model_prefix: Some("groq".to_string()),
        quirks: None,
    };

    let provider = OpenAiCompatProvider::new(config);
    let response = provider.chat(ChatRequest::new("groq/llama-3.1-70b", "Hello")).await.unwrap();
    assert_eq!(response.content, "Hello from Groq");
}
```

**Coverage Target**: >90% (ALL provider code paths tested)

**Provider Test Matrix**: All 14 providers from the registry MUST be tested. Use parameterized tests with mock HTTP servers:

```rust
#[tokio::test]
async fn test_all_providers_via_mock_server() {
    // Each provider has unique request/response format quirks
    let test_cases = vec![
        ("openai", "https://api.openai.com", None),
        ("anthropic", "https://api.anthropic.com", Some("anthropic-version")),
        ("groq", "https://api.groq.com/openai", None),
        ("cohere", "https://api.cohere.ai", None),
        ("mistral", "https://api.mistral.ai", None),
        ("together", "https://api.together.xyz", None),
        ("fireworks", "https://api.fireworks.ai/inference", None),
        ("perplexity", "https://api.perplexity.ai", None),
        ("gemini", "https://generativelanguage.googleapis.com", None),
        ("openrouter", "https://openrouter.ai/api", None),
        ("replicate", "https://api.replicate.com", None),
        ("huggingface", "https://api-inference.huggingface.co", None),
        ("custom", "https://custom.example.com", None),
        ("ollama", "http://localhost:11434", None),
    ];

    for (name, _base_url, extra_header) in &test_cases {
        let mock_server = MockServer::start().await;
        mock_server.mock_chat_completion(name);
        let config = ProviderConfig {
            name: name.to_string(),
            base_url: mock_server.url(),
            ..Default::default()
        };
        let provider = OpenAiCompatProvider::new(config);
        let result = provider.chat(ChatRequest::simple("Hello")).await;
        assert!(result.is_ok(), "Provider {} failed: {:?}", name, result.err());
    }
}
```

This ensures all 14 providers are covered, not just 3-4 representative ones. Each mock server validates the provider sends correct headers, auth format, and request structure.

---

### 4.2 Week 4: Router + JSON Repair Tests

**Test File**: `clawft-llm/tests/router_tests.rs`

```rust
#[test]
fn test_model_routing_anthropic() {
    let router = ProviderRouter::new();
    let provider = router.route("anthropic/claude-3-sonnet-20240229").unwrap();
    assert_eq!(provider.name(), "anthropic");
}

#[test]
fn test_model_routing_openai_compat_groq() {
    let router = ProviderRouter::new();
    let provider = router.route("groq/llama-3.1-70b-versatile").unwrap();
    assert_eq!(provider.name(), "groq");
}

#[test]
fn test_alias_resolution() {
    let router = ProviderRouter::new();
    router.add_alias("claude-3-sonnet", "anthropic/claude-3-sonnet-20240229");
    let provider = router.route("claude-3-sonnet").unwrap();
    assert_eq!(provider.name(), "anthropic");
}

#[test]
fn test_json_repair_missing_closing_brace() {
    let input = r#"{"key": "value""#;
    let repaired = repair_json(input).unwrap();
    assert_eq!(repaired, r#"{"key": "value"}"#);
}

#[test]
fn test_json_repair_unquoted_keys() {
    let input = r#"{key: "value"}"#;
    let repaired = repair_json(input).unwrap();
    assert_eq!(repaired, r#"{"key": "value"}"#);
}

#[test]
fn test_json_repair_trailing_comma() {
    let input = r#"{"a": 1, "b": 2,}"#;
    let repaired = repair_json(input).unwrap();
    assert_eq!(repaired, r#"{"a": 1, "b": 2}"#);
}
```

**Coverage Target**: >90%

---

### 4.3 Week 5: File + Shell Tool Tests

**Test File**: `clawft-tools/tests/file_tests.rs`

```rust
#[tokio::test]
async fn test_read_file_success() {
    let tempdir = tempfile::tempdir().unwrap();
    let file_path = tempdir.path().join("test.txt");
    fs::write(&file_path, "Hello, world!").unwrap();

    let tool = ReadFileTool::new(tempdir.path());
    let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();
    assert_eq!(result.output, "Hello, world!");
}

#[tokio::test]
async fn test_write_file_success() {
    let tempdir = tempfile::tempdir().unwrap();
    let tool = WriteFileTool::new(tempdir.path());
    tool.execute(json!({"path": "test.txt", "content": "New content"})).await.unwrap();

    let content = fs::read_to_string(tempdir.path().join("test.txt")).unwrap();
    assert_eq!(content, "New content");
}

#[tokio::test]
async fn test_path_validation_rejects_traversal() {
    let tempdir = tempfile::tempdir().unwrap();
    let tool = ReadFileTool::new(tempdir.path());
    let result = tool.execute(json!({"path": "../etc/passwd"})).await;
    assert!(matches!(result, Err(ToolError::InvalidPath(_))));
}
```

**Test File**: `clawft-tools/tests/shell_tests.rs`

```rust
#[tokio::test]
async fn test_exec_shell_success() {
    let tempdir = tempfile::tempdir().unwrap();
    let tool = ExecShellTool::new(tempdir.path());
    let result = tool.execute(json!({"command": "echo hello"})).await.unwrap();
    assert_eq!(result.output.trim(), "hello");
}

#[tokio::test]
async fn test_exec_shell_timeout() {
    let tempdir = tempfile::tempdir().unwrap();
    let tool = ExecShellTool::new(tempdir.path());
    let result = tool.execute(json!({"command": "sleep 10", "timeout_secs": 1})).await;
    assert!(matches!(result, Err(ToolError::Timeout(_))));
}

#[tokio::test]
async fn test_exec_shell_rejects_dangerous_commands() {
    let tempdir = tempfile::tempdir().unwrap();
    let tool = ExecShellTool::new(tempdir.path());
    let result = tool.execute(json!({"command": "rm -rf /"})).await;
    assert!(matches!(result, Err(ToolError::SecurityViolation(_))));
}
```

**Coverage Target**: >90%

---

### 4.4 Week 6: Web + Bus Tool Tests

**Test File**: `clawft-tools/tests/web_tests.rs`

```rust
#[tokio::test]
async fn test_web_search_brave_api() {
    // Mock Brave API server
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/web/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                {"title": "Result 1", "url": "https://example.com", "description": "Test"}
            ]
        })))
        .mount(&mock_server)
        .await;

    let tool = SearchWebTool::new(&mock_server.uri(), "test-key");
    let result = tool.execute(json!({"query": "test query"})).await.unwrap();
    assert_eq!(result.output.len(), 1);
}

#[tokio::test]
async fn test_web_fetch_readability() {
    // Mock HTTP server with HTML content
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<html><body><article><h1>Title</h1><p>Content</p></article></body></html>"#
        ))
        .mount(&mock_server)
        .await;

    let tool = FetchWebTool::new();
    let result = tool.execute(json!({"url": mock_server.uri()})).await.unwrap();
    assert!(result.output.contains("Title"));
    assert!(result.output.contains("Content"));
}
```

**Test File**: `clawft-tools/tests/bus_tests.rs`

```rust
#[tokio::test]
async fn test_send_message_tool() {
    let (tx, mut rx) = mpsc::channel(10);
    let mock_bus = MockMessageBus::new(tx);

    let tool = SendMessageTool::new(Arc::new(mock_bus));
    tool.execute(json!({"recipient": "agent:test", "content": "Hello"})).await.unwrap();

    let msg = rx.recv().await.unwrap();
    assert_eq!(msg.recipient, "agent:test");
    assert_eq!(msg.content, "Hello");
}

#[tokio::test]
async fn test_spawn_agent_tool() {
    let mock_factory = MockAgentFactory::new();

    let tool = SpawnAgentTool::new(Arc::new(mock_factory));
    let result = tool.execute(json!({"agent_type": "researcher", "task": "Test task"})).await.unwrap();
    assert!(result.output.starts_with("agent-"));
}
```

**Coverage Target**: >90%

---

### 4.5 Week 7: Telegram + CLI Tests

**Test File**: `clawft/crates/channels/telegram/tests/client_tests.rs`

```rust
#[tokio::test]
async fn test_telegram_get_updates() {
    // Mock Telegram API server
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bottest-token/getUpdates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [
                {
                    "update_id": 1,
                    "message": {
                        "message_id": 1,
                        "from": {"id": 123, "first_name": "Test"},
                        "chat": {"id": 123},
                        "text": "Hello"
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = TelegramClient::new("test-token", &mock_server.uri());
    let updates = client.get_updates(0).await.unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].message.text, Some("Hello".to_string()));
}

#[tokio::test]
async fn test_telegram_send_message() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/bottest-token/sendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true
        })))
        .mount(&mock_server)
        .await;

    let client = TelegramClient::new("test-token", &mock_server.uri());
    client.send_message(123, "Hello").await.unwrap();
}
```

**Test File**: `clawft/crates/cli/tests/agent_tests.rs`

```rust
#[tokio::test]
async fn test_agent_interactive_mode() {
    // Mock stdin with test input
    let input = "Hello\n/exit\n";
    let mock_stdin = Cursor::new(input.as_bytes());

    let agent = MockAgent::new();
    let result = interactive_mode_with_stdin(agent, mock_stdin).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_single_message_mode() {
    let agent = MockAgent::new();
    let result = single_message_mode(agent, "Hello").await.unwrap();
    assert!(result.contains("Hello"));
}
```

**Coverage Target**: >85%

---

### 4.6 Week 8: Gateway + Onboard/Status Tests

**Test File**: `clawft/crates/cli/tests/gateway_tests.rs`

```rust
#[tokio::test]
async fn test_gateway_plugin_loading() {
    let plugin_dir = tempfile::tempdir().unwrap();
    // Create mock plugin .so file (not implemented - placeholder)
    let plugins = load_plugins(plugin_dir.path()).await.unwrap();
    assert!(plugins.is_empty()); // No plugins in test
}

#[tokio::test]
async fn test_gateway_agent_loop() {
    let mock_plugin = MockChannelPlugin::new();
    let mock_agent = MockAgent::new();

    // Spawn gateway loop
    let handle = tokio::spawn(async move {
        gateway_loop(vec![Box::new(mock_plugin)], mock_agent).await;
    });

    // Send test message via plugin
    // Verify agent processes it

    handle.abort();
}
```

**Test File**: `clawft/crates/cli/tests/onboard_tests.rs`

```rust
#[tokio::test]
async fn test_onboard_creates_config() {
    let tempdir = tempfile::tempdir().unwrap();
    env::set_var("HOME", tempdir.path());

    // Mock stdin with wizard responses
    let input = "anthropic\nclaude-3-sonnet\ntest-key\n";
    let mock_stdin = Cursor::new(input.as_bytes());

    onboard_with_stdin(mock_stdin).await.unwrap();

    let config_path = tempdir.path().join(".config/clawft/agent.toml");
    assert!(config_path.exists());
}
```

**Test File**: `clawft/crates/cli/tests/status_tests.rs`

```rust
#[tokio::test]
async fn test_status_command() {
    let tempdir = tempfile::tempdir().unwrap();
    env::set_var("HOME", tempdir.path());

    // Create test config
    let config_path = tempdir.path().join(".config/clawft/agent.toml");
    fs::write(&config_path, r#"
        [provider]
        name = "anthropic"
        model = "claude-3-sonnet-20240229"
    "#).unwrap();

    let output = status_command().await.unwrap();
    assert!(output.contains("Provider: anthropic"));
    assert!(output.contains("Model: claude-3-sonnet-20240229"));
}
```

**Coverage Target**: >85%

---

## 5. Completion (Phase 1 Milestone Checklist)

### 5.1 Week 3 Deliverables

- [ ] **clawft-llm repository created**
  - [ ] Repository initialized with Apache 2.0 license
  - [ ] CI/CD pipeline configured (GitHub Actions)
  - [ ] README with usage examples

- [ ] **Provider extraction complete**
  - [ ] 11 source files copied from barni-providers
  - [ ] NO barni dependencies (verified via `cargo check`)
  - [ ] CircuitBreaker internalized (~150-200 lines)
  - [ ] `request.rs` uses `Option<String>` (no newtype imports)

- [ ] **4 native providers working**
  - [ ] AnthropicProvider (tested via mock)
  - [ ] OpenAiProvider (tested via mock)
  - [ ] BedrockProvider (tested via mock)
  - [ ] GeminiProvider (tested via mock)

- [ ] **OpenAiCompatProvider implemented**
  - [ ] Config-driven base_url support
  - [ ] Works with Groq (tested via mock)
  - [ ] Works with DeepSeek (tested via mock)
  - [ ] Works with Mistral (tested via mock)

- [ ] **Testing & documentation**
  - [ ] All tests pass (`cargo test`)
  - [ ] Code coverage >90%
  - [ ] Published to crates.io as v0.1.0

---

### 5.2 Week 4 Deliverables

- [ ] **Provider routing implemented**
  - [ ] `ProviderRouter` struct created
  - [ ] Prefix-based routing works (tested)
  - [ ] All 19 providers routable (4 native + 15 compat)

- [ ] **Model aliasing implemented**
  - [ ] Alias config file support (`~/.config/clawft/aliases.toml`)
  - [ ] Common aliases work: `claude-3-sonnet`, `gpt-4o`, `llama-3-70b`

- [ ] **JSON repair implemented**
  - [ ] `repair_json()` function created
  - [ ] Fixes 95% of common malformed outputs (tested with 20+ examples)
  - [ ] Regex-based (no external crate)

- [ ] **Testing & documentation**
  - [ ] All tests pass (`cargo test`)
  - [ ] Code coverage >90%
  - [ ] Published to crates.io as v0.2.0

---

### 5.3 Week 5 Deliverables

- [ ] **File tools implemented**
  - [ ] `ReadFileTool` (read_file)
  - [ ] `WriteFileTool` (write_file)
  - [ ] `EditFileTool` (edit_file - find/replace)
  - [ ] `ListDirectoryTool` (list_directory)

- [ ] **Path validation working**
  - [ ] Canonical path resolution (prevent `../` traversal)
  - [ ] Workspace restriction enforced
  - [ ] Rejects absolute paths outside workspace

- [ ] **Shell execution tool implemented**
  - [ ] `ExecShellTool` (exec_shell)
  - [ ] Feature gate: `tool-exec`
  - [ ] Timeout enforcement (tested)
  - [ ] Command sanitization (rejects `rm -rf /`, `sudo`)

- [ ] **Tool trait integration**
  - [ ] All 5 tools implement `Tool` trait
  - [ ] `ToolRegistry::register_all()` function works

- [ ] **Testing & documentation**
  - [ ] All tests pass (`cargo test`)
  - [ ] Code coverage >90%
  - [ ] Path validation tested with malicious inputs

---

### 5.4 Week 6 Deliverables

- [ ] **Web search tool implemented**
  - [ ] `SearchWebTool` (search_web)
  - [ ] Brave Search API integration (tested via mock)
  - [ ] Feature gate: `tool-web`

- [ ] **Web fetch tool implemented**
  - [ ] `FetchWebTool` (fetch_web)
  - [ ] Readability extraction (tested via mock HTML)
  - [ ] Markdown output

- [ ] **Message tool implemented**
  - [ ] `SendMessageTool` (send_message)
  - [ ] Message bus integration (tested via mock)
  - [ ] Feature gate: `tool-spawn`

- [ ] **Spawn tool implemented**
  - [ ] `SpawnAgentTool` (spawn_agent)
  - [ ] Agent factory integration (tested via mock)
  - [ ] Returns spawned agent ID

- [ ] **Testing & documentation**
  - [ ] All tests pass (`cargo test`)
  - [ ] Code coverage >90%
  - [ ] All 9 tools documented in README

---

### 5.5 Week 7 Deliverables

- [ ] **Telegram plugin implemented**
  - [ ] `TelegramPlugin` implements `Channel` trait
  - [ ] `TelegramClient` REST client (~200 lines)
  - [ ] Long-polling with offset tracking (tested via mock)
  - [ ] Send message works (tested via mock)

- [ ] **Message parsing working**
  - [ ] Telegram `Update` → `InboundMessage` conversion
  - [ ] `OutboundMessage` → Telegram API formatting

- [ ] **CLI agent command implemented**
  - [ ] `weft agent` interactive mode (rustyline)
  - [ ] `weft agent -m "msg"` single-message mode
  - [ ] Markdown rendering (termimad or similar)
  - [ ] History support (`~/.clawft_history`)

- [ ] **Testing & documentation**
  - [ ] All tests pass (`cargo test`)
  - [ ] Code coverage >85%
  - [ ] CLI usage examples in README

---

### 5.6 Week 8 Deliverables

- [ ] **CLI gateway command implemented**
  - [ ] `weft gateway` starts plugin host
  - [ ] Plugin loading from `~/.config/clawft/plugins/`
  - [ ] Agent loop processes InboundMessage → OutboundMessage
  - [ ] Graceful shutdown (SIGINT/SIGTERM)

- [ ] **CLI onboard command implemented**
  - [ ] `weft onboard` interactive wizard
  - [ ] Creates `~/.config/clawft/` directory
  - [ ] Creates `agent.toml` config file
  - [ ] Creates `plugins.toml` config file
  - [ ] Validates API keys (test provider connection)

- [ ] **CLI status command implemented**
  - [ ] `weft status` shows config location
  - [ ] Shows provider status (name, model, API key set)
  - [ ] Shows tool status (enabled/disabled)
  - [ ] Shows plugin status (loaded channels)
  - [ ] Color-coded output (green = OK, red = error)

- [ ] **Testing & documentation**
  - [ ] All tests pass (`cargo test`)
  - [ ] Code coverage >85%
  - [ ] Full CLI documentation in README

---

### 5.7 Phase 1 Integration Testing

- [ ] **End-to-end Telegram bot test**
  - [ ] Real Telegram bot created (test account)
  - [ ] Bot receives messages via long-polling
  - [ ] Bot responds using clawft-llm provider
  - [ ] Bot uses file tools (read/write/edit)
  - [ ] Tested manually with real Telegram client

- [ ] **CLI integration test**
  - [ ] `weft onboard` creates config successfully
  - [ ] `weft status` shows correct provider config
  - [ ] `weft agent` interactive mode works
  - [ ] `weft gateway` runs with Telegram plugin

- [ ] **Performance validation**
  - [ ] Provider routing <10ms latency
  - [ ] JSON repair <5ms latency
  - [ ] File tools <50ms latency
  - [ ] Shell exec timeout works correctly
  - [ ] Telegram polling latency <1s

- [ ] **Security validation**
  - [ ] Path validation prevents directory traversal
  - [ ] Shell exec rejects dangerous commands
  - [ ] API keys never logged or exposed
  - [ ] `npx @claude-flow/cli@latest security scan` passes

---

### 5.8 Documentation & Release

- [ ] **clawft-llm documentation**
  - [ ] README with usage examples
  - [ ] API documentation (rustdoc)
  - [ ] Provider table with all 19 providers
  - [ ] Migration guide from barni-providers

- [ ] **clawft-tools documentation**
  - [ ] README with tool descriptions
  - [ ] Feature flag documentation
  - [ ] Security guidelines for `tool-exec`

- [ ] **Telegram plugin documentation**
  - [ ] README with setup instructions
  - [ ] Telegram Bot API configuration guide
  - [ ] Voice message support (if implemented)

- [ ] **CLI documentation**
  - [ ] README with all commands
  - [ ] Configuration file examples
  - [ ] Troubleshooting guide

- [ ] **Release artifacts**
  - [ ] clawft-llm v0.2.0 published to crates.io
  - [ ] clawft v0.1.0 published to crates.io
  - [ ] Binary releases for Linux, macOS, Windows (CI/CD)
  - [ ] Release notes with changelog

---

### 5.9 Validation Criteria (MUST PASS)

✅ **Criterion 1**: Real Telegram bot receives and responds to messages via plugin
- [ ] Telegram bot created with BotFather
- [ ] Bot token configured in `~/.config/clawft/plugins.toml`
- [ ] `weft gateway` starts successfully
- [ ] Send message to bot via Telegram client
- [ ] Bot responds with LLM-generated text
- [ ] Response time <5 seconds

✅ **Criterion 2**: CLI interactive mode with history and markdown rendering
- [ ] `weft agent` starts interactive mode
- [ ] User can type messages and receive responses
- [ ] Markdown rendering works (bold, code blocks, lists)
- [ ] Command history works (up/down arrows)
- [ ] `/exit`, `/clear`, `/help` commands work
- [ ] History persists to `~/.clawft_history`

✅ **Criterion 3**: `weft status` shows correct provider configuration
- [ ] `weft onboard` creates config successfully
- [ ] `weft status` displays provider name
- [ ] `weft status` displays model name
- [ ] `weft status` shows API key status (set/not set)
- [ ] `weft status` shows enabled tools
- [ ] `weft status` tests provider connection (green = OK)

---

## 6. Risk Mitigation

### 6.1 Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **barni dependency removal breaks provider logic** | High | Port tests FIRST, verify behavior with mocks before removing imports |
| **OpenAI-compat quirks not handled** | Medium | Test with ALL 15 providers, maintain quirk database |
| **JSON repair too aggressive (breaks valid JSON)** | Medium | Validate repaired JSON, add test cases for edge cases |
| **Path traversal vulnerability in file tools** | High | Use canonical path resolution, extensive security testing |
| **Shell exec security holes** | Critical | Feature-gate by default, sanitize commands, timeout enforcement |
| **Telegram API rate limiting** | Medium | Implement exponential backoff, respect Telegram limits |
| **Dynamic plugin loading fails on Windows** | Low | Test on all platforms, document platform-specific issues |

### 6.2 Schedule Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Provider extraction takes >1 week** | Medium | Prioritize minimal viable extraction, skip optional features |
| **Tool implementation blocked on platform API** | High | Coordinate with Stream 1A, stub Platform trait if needed |
| **Telegram plugin blocked on Channel trait** | High | Coordinate with Stream 1A, implement Channel trait stub if needed |
| **Week 8 integration issues delay Phase 1** | High | Start integration testing early (week 7), allocate buffer week |

### 6.3 Dependency Risks

| Dependency | Risk | Mitigation |
|------------|------|------------|
| **Stream 1A Platform API** | Channel trait not ready by week 3 | Stub Channel trait in telegram plugin, refactor later |
| **Stream 1B Tool Registry** | Tool trait not ready by week 4 | Define Tool trait locally, merge with 1B later |
| **Brave Search API** | API key required for web search | Document API key requirement, provide mock for tests |
| **Telegram Bot API** | Bot token required for testing | Document bot creation, use mock API for CI tests |

---

## 7. Success Metrics

### 7.1 Code Quality Metrics

- **Test coverage**: >90% for clawft-llm, >85% for other crates
- **Clippy warnings**: 0 (all lints must pass)
- **Compilation time**: <30s for full workspace build
- **Binary size**: <10MB for `weft` binary (release mode)

### 7.2 Performance Metrics

- **Provider routing latency**: <10ms (99th percentile)
- **JSON repair latency**: <5ms (99th percentile)
- **File tool latency**: <50ms (99th percentile)
- **Telegram message round-trip**: <5s (including LLM inference)

### 7.3 Integration Metrics

- **Phase 1 validation**: All 3 criteria pass (Telegram bot, CLI interactive, status command)
- **End-to-end test**: Manual test with real Telegram bot completes successfully
- **Security scan**: No critical or high severity issues

---

## 8. Future Work (Phase 2+)

### 8.1 Additional Providers

- **Phase 2**: Add Vertex AI, Claude Vertex, Azure OpenAI (native providers)
- **Phase 3**: Add Together AI, Fireworks AI, Cerebras (config-driven)

### 8.2 Additional Tools

- **Phase 2**: Add cron tool (scheduled tasks), git tool (repository operations)
- **Phase 3**: Add database tool (SQL queries), HTTP tool (REST API calls)

### 8.3 Additional Channels

- **Phase 2**: Add Slack plugin, Discord plugin
- **Phase 3**: Add WhatsApp plugin, SMS plugin

### 8.4 Advanced Features

- **Phase 2**: Add voice message support (Whisper API for Telegram)
- **Phase 3**: Add image generation tool (DALL-E, Stable Diffusion)
- **Phase 4**: Add multi-agent coordination (spawn + message tools)

---

## Appendix A: Python Source File Mapping

| Python File | Rust Crate | Rust File | Lines | Notes |
|-------------|------------|-----------|-------|-------|
| `barni/src/layer3/barni-providers/src/llm/*` | clawft-llm | (ALL) | 10,563 | Extract as standalone library |
| `nanobot/agent/tools/filesystem.py` | clawft-tools | `file/` | ~300 | Port to Rust |
| `nanobot/agent/tools/shell.py` | clawft-tools | `shell/exec.rs` | ~150 | Port to Rust |
| `nanobot/agent/tools/web.py` | clawft-tools | `web/` | ~200 | Port to Rust |
| `nanobot/agent/tools/message.py` | clawft-tools | `bus/message.rs` | ~100 | Port to Rust |
| `nanobot/agent/tools/spawn.py` | clawft-tools | `bus/spawn.rs` | ~100 | Port to Rust |
| `nanobot/channels/telegram.py` | telegram | (ALL) | ~400 | Port to Rust |
| `nanobot/cli/commands.py` | clawft-cli | `commands/` | ~500 | Port to Rust |

---

## Appendix B: Provider Table (19 Providers)

| Provider | Type | Base URL | Model Prefix | Quirks |
|----------|------|----------|--------------|--------|
| Anthropic | Native | api.anthropic.com | `anthropic/` | Prompt caching, thinking tokens |
| OpenAI | Native | api.openai.com | `openai/` | Structured outputs, o1 reasoning |
| Bedrock | Native | bedrock-runtime.{region}.amazonaws.com | `bedrock/` | AWS auth, Claude via Bedrock |
| Gemini | Native | generativelanguage.googleapis.com | `gemini/` | Google Cloud auth |
| Groq | Compat | api.groq.com/openai/v1 | `groq/` | - |
| DeepSeek | Compat | api.deepseek.com | `deepseek/` | `reasoning_content` field |
| Mistral | Compat | api.mistral.ai/v1 | `mistral/` | Temp 0.0-0.7 |
| OpenRouter | Compat | openrouter.ai/api/v1 | `openrouter/` | Model routing |
| Together | Compat | api.together.xyz/v1 | `together/` | - |
| Fireworks | Compat | api.fireworks.ai/inference/v1 | `fireworks/` | - |
| Perplexity | Compat | api.perplexity.ai | `perplexity/` | `search_results` metadata |
| xAI/Grok | Compat | api.x.ai/v1 | `xai/` | - |
| Ollama | Compat | localhost:11434/v1 | `ollama/` | No auth (local) |
| Azure OpenAI | Compat | {resource}.openai.azure.com/openai/v1 | `azure/` | `api-version` param |
| Cerebras | Compat | api.cerebras.ai/v1 | `cerebras/` | - |
| SambaNova | Compat | api.sambanova.ai/v1 | `sambanova/` | - |
| DeepInfra | Compat | api.deepinfra.com/v1/openai | `deepinfra/` | - |
| HuggingFace | Compat | router.huggingface.co/v1 | `huggingface/` | `:provider` suffix |
| Replicate | Compat | api.replicate.com/v1 | `replicate/` | - |

---

## Appendix C: Feature Flag Table

| Feature Flag | Crates | Default | Security Risk | Description |
|--------------|--------|---------|---------------|-------------|
| `file` | tools | ✅ Yes | Low | File read/write/edit/list tools |
| `tool-exec` | tools | ❌ No | **CRITICAL** | Shell execution tool |
| `tool-web` | tools | ❌ No | Low | Web search + fetch tools |
| `tool-spawn` | tools | ❌ No | Medium | Message + spawn tools |
| `all-tools` | tools | ❌ No | **CRITICAL** | Enable ALL tools (including exec) |

---

**END OF SPARC PLAN**
