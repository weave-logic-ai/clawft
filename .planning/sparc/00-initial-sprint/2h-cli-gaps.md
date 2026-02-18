# SPARC Implementation Plan: Stream 2H - CLI Gaps & Missing Tools

**Timeline**: Week 9-11
**Owned Crates**: `clawft-cli` (new subcommands), `clawft-tools` (new tool impls)
**Dependencies**: Phase 1 complete (SessionManager, MemoryStore, MessageBus, Pipeline, Tool trait, Config)

---

## 1. Agent Instructions

### Rust Source Files to Read
```
clawft/crates/clawft-cli/src/main.rs              # 317 lines - current Cli enum + subcommands
clawft/crates/clawft-cli/src/commands/mod.rs       # 155 lines - load_config, expand_workspace, helpers
clawft/crates/clawft-cli/src/commands/status.rs    # StatusArgs + diagnostics output
clawft/crates/clawft-cli/src/commands/agent.rs     # AgentArgs + agent loop entry
clawft/crates/clawft-cli/Cargo.toml               # Current deps (clap, tokio, comfy-table, etc.)
clawft/crates/clawft-core/src/session.rs           # SessionManager: list_sessions, load_session, save_session, invalidate
clawft/crates/clawft-core/src/agent/memory.rs      # MemoryStore: read_long_term, read_history, search, paths
clawft/crates/clawft-core/src/bus.rs               # MessageBus: outbound_sender, publish_inbound
clawft/crates/clawft-core/src/tools/registry.rs    # Tool trait: name, description, parameters, execute
clawft/crates/clawft-core/src/pipeline/traits.rs   # Pipeline, PipelineRegistry, complete()
clawft/crates/clawft-tools/src/lib.rs              # register_all pattern
clawft/crates/clawft-tools/src/file_tools.rs       # ReadFileTool/WriteFileTool pattern
clawft/crates/clawft-tools/src/memory_tool.rs      # MemoryReadTool/MemoryWriteTool pattern
clawft/crates/clawft-tools/src/shell_tool.rs       # ShellExecTool pattern
clawft/crates/clawft-types/src/config.rs           # Config struct (agents, channels, providers, tools)
clawft/crates/clawft-types/src/event.rs            # OutboundMessage struct
```

### Planning Documents (MUST READ)
```
.planning/02-technical-requirements.md  # CLI + tool architecture
.planning/03-development-guide.md       # Stream 2H timeline
.planning/01-project-overview.md        # CLI feature requirements
```

### Module Structure
```
clawft-cli/src/
  main.rs                    # Add Sessions, Memory, Config, Completions to Commands enum
  commands/
    mod.rs                   # (unchanged)
    sessions.rs              # NEW: list, show, delete subcommands
    memory.rs                # NEW: show, search, history subcommands
    config.rs                # NEW: show, path, validate subcommands
    completions.rs           # NEW: shell completion generation

clawft-tools/src/
  lib.rs                     # Update register_all to include new tools
  web_search_tool.rs         # NEW: web_search tool
  web_fetch_tool.rs          # NEW: web_fetch tool
  message_tool.rs            # NEW: message (bus outbound) tool
  spawn_tool.rs              # NEW: spawn (subagent) tool stub
```

---

## 2. Specification

### 2.1 `weft sessions` Subcommand

#### Requirements
- `weft sessions list` lists all session keys with creation date, updated date, and message count
- `weft sessions show <id>` displays the full message history of a session in readable format
- `weft sessions delete <id>` removes a session JSONL file from disk and invalidates the cache
- All subcommands accept `--config <path>` for config override
- Output uses `comfy-table` for tabular data (consistent with existing `weft channels status`)

#### SessionManager API Surface Used
- `SessionManager::new(platform)` or `SessionManager::with_dir(platform, dir)` for construction
- `list_sessions() -> Result<Vec<String>>` for enumerating session keys
- `load_session(key) -> Result<Session>` for reading session data
- `invalidate(key)` for cache eviction after delete
- `session_path(key) -> PathBuf` (private; we add a `delete_session` public method or delete via platform.fs().remove_file)

#### Acceptance Criteria
- [ ] `weft sessions list` prints a table with columns: Key, Created, Updated, Messages
- [ ] `weft sessions list` with no sessions prints "No sessions found."
- [ ] `weft sessions show <id>` prints role-tagged messages with timestamps
- [ ] `weft sessions show <id>` with unknown ID prints error and exits 1
- [ ] `weft sessions delete <id>` removes the file and prints confirmation
- [ ] `weft sessions delete <id>` with unknown ID prints error and exits 1
- [ ] `weft sessions delete --all` deletes all sessions after confirmation prompt
- [ ] All three subcommands respect `--config` override
- [ ] Unit tests verify clap parsing for all session subcommands

### 2.2 `weft memory` Subcommand

#### Requirements
- `weft memory show` prints the full content of MEMORY.md to stdout
- `weft memory search <query>` searches paragraphs (case-insensitive) and prints matches
- `weft memory history` prints the content of HISTORY.md to stdout
- All subcommands accept `--config <path>` for config override

#### MemoryStore API Surface Used
- `MemoryStore::new(platform) -> Result<Self>` for construction
- `read_long_term() -> Result<String>` for MEMORY.md content
- `read_history() -> Result<String>` for HISTORY.md content
- `search(query, max_results) -> Vec<String>` for paragraph search
- `memory_path() -> &PathBuf` for diagnostics
- `history_path() -> &PathBuf` for diagnostics

#### Acceptance Criteria
- [ ] `weft memory show` prints MEMORY.md content to stdout
- [ ] `weft memory show` with no memory file prints "No memory file found." and exits 0
- [ ] `weft memory search <query>` prints matching paragraphs numbered
- [ ] `weft memory search <query>` with `--limit N` caps results
- [ ] `weft memory search <query>` with no matches prints "No matches found."
- [ ] `weft memory history` prints HISTORY.md content
- [ ] `weft memory history` with no history file prints "No history file found." and exits 0
- [ ] Unit tests verify clap parsing for all memory subcommands

### 2.3 `weft config` Subcommand

#### Requirements
- `weft config show` loads and displays the resolved Config as pretty-printed JSON
- Sensitive values (API keys, tokens) in `providers` section are masked as `"***"` before display
- `weft config path` prints the discovered config file path (or "No config file found")
- `weft config validate` parses the config and reports errors/warnings

#### Config API Surface Used
- `load_config(platform, override) -> Result<Config>` from commands/mod.rs
- `discover_config_path(platform) -> Option<PathBuf>` from commands/mod.rs
- `serde_json::to_string_pretty` for display
- Config struct: `agents`, `channels`, `providers`, `gateway`, `tools`

#### Acceptance Criteria
- [ ] `weft config show` prints valid JSON with sensitive values masked
- [ ] `weft config show --unmask` prints all values (with warning banner)
- [ ] `weft config path` prints absolute path to config file
- [ ] `weft config path` with no config file prints "No config file found." and exits 0
- [ ] `weft config validate` with valid config prints "Configuration is valid."
- [ ] `weft config validate` with invalid JSON exits 1 with parse error details
- [ ] `weft config validate` warns about missing recommended fields (e.g., no provider configured)
- [ ] API keys containing strings like `sk-`, `xoxb-`, `Bot ` are detected for masking
- [ ] Unit tests verify clap parsing for all config subcommands

### 2.4 Shell Completion Generation

#### Requirements
- Add `clap_complete` as a dependency to clawft-cli
- `weft completions <shell>` writes completion script to stdout
- Supported shells: bash, zsh, fish, powershell
- Follows standard `clap_complete::generate` pattern

#### Acceptance Criteria
- [ ] `weft completions bash` outputs valid bash completion script
- [ ] `weft completions zsh` outputs valid zsh completion script
- [ ] `weft completions fish` outputs valid fish completion script
- [ ] `weft completions powershell` outputs valid PowerShell completion script
- [ ] Output can be piped to a file or sourced directly
- [ ] Help text explains usage: `eval "$(weft completions bash)"`
- [ ] Unit test verifies clap parsing of completions subcommand

### 2.5 `web_search` Tool

#### Requirements
- Implements `Tool` trait with name `"web_search"`
- Accepts `query` (required string), `num_results` (optional u32, default 5)
- Calls a configurable search API endpoint via reqwest (default: SearXNG or similar)
- Returns structured results: array of `{title, url, snippet}`
- Handles rate limits (429 status) with retry-after
- Handles timeouts (configurable, default 10s)
- Errors on missing/invalid API endpoint configuration

#### Acceptance Criteria
- [ ] Tool name is `"web_search"` and description explains purpose
- [ ] Parameter schema declares `query` as required, `num_results` as optional
- [ ] Missing `query` returns `ToolError::InvalidArgs`
- [ ] Successful search returns JSON array of `{title, url, snippet}` objects
- [ ] HTTP 429 triggers retry with backoff (up to 2 retries)
- [ ] HTTP timeout returns `ToolError::Timeout`
- [ ] Network errors return `ToolError::ExecutionFailed` with message
- [ ] `num_results` clamps to range 1..=20
- [ ] Unit tests cover: missing query, clamping, error cases (mock HTTP)
- [ ] Integration test with mock HTTP server validates roundtrip

### 2.6 `web_fetch` Tool

#### Requirements
- Implements `Tool` trait with name `"web_fetch"`
- Accepts `url` (required string), `max_length` (optional u32, default 50000)
- Uses reqwest to GET the URL with configurable timeout (default 15s)
- Strips HTML tags to produce plain text (basic regex-based or simple state machine)
- Enforces `max_length` truncation on response body
- Handles redirects (up to 5 hops), timeouts, non-200 status codes
- Rejects non-http/https schemes

#### Acceptance Criteria
- [ ] Tool name is `"web_fetch"` and description explains purpose
- [ ] Parameter schema declares `url` as required, `max_length` as optional
- [ ] Missing `url` returns `ToolError::InvalidArgs`
- [ ] Invalid URL scheme (ftp://, file://) returns `ToolError::InvalidArgs`
- [ ] Successful fetch returns `{content, url, content_type, truncated}`
- [ ] HTML content has tags stripped to plain text
- [ ] Response body exceeding `max_length` is truncated, `truncated: true`
- [ ] Non-HTML content types (application/json, text/plain) passed through as-is
- [ ] HTTP errors (404, 500) return `ToolError::ExecutionFailed` with status code
- [ ] Timeout returns `ToolError::Timeout`
- [ ] `max_length` clamps to range 100..=200000
- [ ] Unit tests cover: missing URL, invalid scheme, truncation, HTML stripping
- [ ] Integration test with mock HTTP server validates fetch + strip

### 2.7 `message` Tool

#### Requirements
- Implements `Tool` trait with name `"message"`
- Accepts `channel` (required string), `chat_id` (required string), `content` (required string)
- Posts an `OutboundMessage` to the bus's outbound sender
- Returns confirmation with the channel and chat_id

#### OutboundMessage Fields Used
```rust
OutboundMessage {
    channel: String,
    chat_id: String,
    content: String,
    reply_to: Option<String>,  // optional, from args
    media: Vec<String>,        // optional, from args
}
```

#### Acceptance Criteria
- [ ] Tool name is `"message"` and description explains purpose
- [ ] Parameter schema declares `channel`, `chat_id`, `content` as required
- [ ] Optional parameters: `reply_to` (string), `media` (array of strings)
- [ ] Missing required field returns `ToolError::InvalidArgs`
- [ ] Successful send returns `{sent: true, channel, chat_id}`
- [ ] Bus closed/dropped returns `ToolError::ExecutionFailed`
- [ ] Unit test with mock bus sender verifies OutboundMessage fields

### 2.8 `spawn` Tool (Stub with Extension Point)

#### Requirements
- Implements `Tool` trait with name `"spawn"`
- Accepts `prompt` (required string), `model` (optional string), `system_prompt` (optional string)
- Level 0 implementation: constructs a one-shot `ChatRequest` and calls `pipeline.complete()`
- Returns the LLM response content
- Designed with clear extension points for future: tool subset selection, budget limits, recursion depth

#### Acceptance Criteria
- [ ] Tool name is `"spawn"` and description explains purpose
- [ ] Parameter schema declares `prompt` as required; `model`, `system_prompt` as optional
- [ ] Missing `prompt` returns `ToolError::InvalidArgs`
- [ ] Successful spawn returns `{content, model_used, tokens_used}`
- [ ] Pipeline errors are mapped to `ToolError::ExecutionFailed`
- [ ] Struct has placeholder fields for future expansion: `max_tools`, `max_depth`, `budget`
- [ ] Unit test verifies parameter validation
- [ ] Integration test with mock pipeline verifies complete() is called correctly

---

## 3. Pseudocode

### 3.1 `weft sessions` CLI Module

```
// commands/sessions.rs

/// Clap subcommand enum for `weft sessions`.
#[derive(Subcommand)]
enum SessionsAction {
    /// List all sessions.
    List {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Show a session's messages.
    Show {
        /// Session key (e.g. "telegram:12345").
        id: String,
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Delete a session.
    Delete {
        /// Session key to delete, or "--all" for all sessions.
        id: String,
        #[arg(long)]
        all: bool,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        #[arg(short, long)]
        config: Option<String>,
    },
}

async fn sessions_list(platform, config) -> Result<()> {
    let mgr = SessionManager::new(platform.clone()).await?;
    let keys = mgr.list_sessions().await?;

    if keys.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(["Key", "Created", "Updated", "Messages"]);

    for key in &keys {
        match mgr.load_session(key).await {
            Ok(session) => {
                table.add_row([
                    key.clone(),
                    session.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    session.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    session.messages.len().to_string(),
                ]);
            }
            Err(e) => {
                table.add_row([key.clone(), "?".into(), "?".into(), format!("error: {e}")]);
            }
        }
    }

    println!("{table}");
    Ok(())
}

async fn sessions_show(platform, id, config) -> Result<()> {
    let mgr = SessionManager::new(platform.clone()).await?;
    let session = mgr.load_session(&id).await
        .map_err(|_| anyhow!("session not found: {id}"))?;

    println!("Session: {}", session.key);
    println!("Created: {}", session.created_at);
    println!("Updated: {}", session.updated_at);
    println!("Messages: {}\n", session.messages.len());

    for (i, msg) in session.messages.iter().enumerate() {
        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let ts = msg.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        println!("[{i}] {role} ({ts}):");
        println!("  {content}\n");
    }
    Ok(())
}

async fn sessions_delete(platform, id, yes, config) -> Result<()> {
    let mgr = SessionManager::new(platform.clone()).await?;

    // Verify session exists by loading it.
    mgr.load_session(&id).await
        .map_err(|_| anyhow!("session not found: {id}"))?;

    if !yes {
        eprint!("Delete session '{id}'? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Delete the JSONL file.
    let path = mgr.sessions_dir.join(format!("{}.jsonl", id.replace(':', "_")));
    platform.fs().remove_file(&path).await?;
    mgr.invalidate(&id).await;

    println!("Deleted session: {id}");
    Ok(())
}
```

### 3.2 `weft memory` CLI Module

```
// commands/memory.rs

#[derive(Subcommand)]
enum MemoryAction {
    /// Display MEMORY.md content.
    Show {
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Search memory paragraphs.
    Search {
        /// Search query string.
        query: String,
        /// Maximum number of results.
        #[arg(long, default_value = "10")]
        limit: usize,
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Display HISTORY.md content.
    History {
        #[arg(short, long)]
        config: Option<String>,
    },
}

async fn memory_show(platform) -> Result<()> {
    let store = MemoryStore::new(platform)?;
    let content = store.read_long_term().await?;

    if content.is_empty() {
        println!("No memory file found.");
        println!("  Expected at: {}", store.memory_path().display());
        return Ok(());
    }

    println!("{content}");
    Ok(())
}

async fn memory_search(platform, query, limit) -> Result<()> {
    let store = MemoryStore::new(platform)?;
    let results = store.search(&query, limit).await;

    if results.is_empty() {
        println!("No matches found for: {query}");
        return Ok(());
    }

    println!("Found {} match(es) for \"{query}\":\n", results.len());
    for (i, paragraph) in results.iter().enumerate() {
        println!("--- Match {} ---", i + 1);
        println!("{paragraph}\n");
    }
    Ok(())
}

async fn memory_history(platform) -> Result<()> {
    let store = MemoryStore::new(platform)?;
    let content = store.read_history().await?;

    if content.is_empty() {
        println!("No history file found.");
        println!("  Expected at: {}", store.history_path().display());
        return Ok(());
    }

    println!("{content}");
    Ok(())
}
```

### 3.3 `weft config` CLI Module

```
// commands/config.rs

#[derive(Subcommand)]
enum ConfigAction {
    /// Display resolved configuration.
    Show {
        /// Show unmasked sensitive values (DANGER).
        #[arg(long)]
        unmask: bool,
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Show config file path.
    Path,
    /// Validate configuration.
    Validate {
        #[arg(short, long)]
        config: Option<String>,
    },
}

/// Patterns that indicate sensitive values in config JSON.
const SENSITIVE_KEY_PATTERNS: &[&str] = &[
    "api_key", "token", "secret", "password", "credential",
];

/// Mask sensitive values in a JSON tree.
fn mask_sensitive(value: &mut serde_json::Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                let key_lower = key.to_lowercase();
                if SENSITIVE_KEY_PATTERNS.iter().any(|p| key_lower.contains(p)) {
                    if val.is_string() {
                        *val = Value::String("***".to_string());
                    }
                } else {
                    mask_sensitive(val);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                mask_sensitive(item);
            }
        }
        _ => {}
    }
}

async fn config_show(platform, config_override, unmask) -> Result<()> {
    let cfg = load_config(&platform, config_override.as_deref()).await?;
    let mut json_val = serde_json::to_value(&cfg)?;

    if unmask {
        eprintln!("WARNING: Displaying unmasked configuration. Do not share this output.");
    } else {
        mask_sensitive(&mut json_val);
    }

    println!("{}", serde_json::to_string_pretty(&json_val)?);
    Ok(())
}

fn config_path(platform) {
    match discover_config_path(&platform) {
        Some(path) => println!("{}", path.display()),
        None => println!("No config file found."),
    }
}

async fn config_validate(platform, config_override) -> Result<()> {
    match load_config(&platform, config_override.as_deref()).await {
        Ok(cfg) => {
            println!("Configuration is valid.");

            // Warn about missing recommended fields.
            if cfg.providers == ProvidersConfig::default() {
                eprintln!("  Warning: no LLM providers configured.");
            }
            if cfg.channels == ChannelsConfig::default() {
                eprintln!("  Warning: no channels configured.");
            }
        }
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}
```

### 3.4 Shell Completions Module

```
// commands/completions.rs

use clap::CommandFactory;
use clap_complete::{generate, Shell};

#[derive(clap::Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    shell: Shell,
}

pub fn run(args: CompletionsArgs) {
    let mut cmd = crate::Cli::command();
    generate(args.shell, &mut cmd, "weft", &mut std::io::stdout());
}
```

### 3.5 `web_search` Tool

```
// clawft-tools/src/web_search_tool.rs

pub struct WebSearchTool<P: Platform> {
    platform: Arc<P>,
    /// Base URL for search API (e.g. "https://searx.example.com/search").
    endpoint: String,
    /// Request timeout.
    timeout: Duration,
}

impl<P: Platform> WebSearchTool<P> {
    pub fn new(platform: Arc<P>, endpoint: String, timeout: Duration) -> Self { ... }
    pub fn from_config(platform: Arc<P>, tools_config: &ToolsConfig) -> Option<Self> { ... }
}

#[async_trait]
impl<P: Platform + 'static> Tool for WebSearchTool<P> {
    fn name(&self) -> &str { "web_search" }

    fn description(&self) -> &str {
        "Search the web and return structured results (title, URL, snippet)."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (1-20, default 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let query = required_str(&args, "query")?;
        let num = args.get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .clamp(1, 20);

        let url = format!("{}?q={}&format=json&count={}",
            self.endpoint,
            urlencoding::encode(&query),
            num);

        let response = self.platform.http()
            .request("GET", &url, &HashMap::new(), None)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("search request failed: {e}")))?;

        if response.status == 429 {
            // Retry once after 1s
            tokio::time::sleep(Duration::from_secs(1)).await;
            let retry = self.platform.http()
                .request("GET", &url, &HashMap::new(), None)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("search retry failed: {e}")))?;
            if retry.status == 429 {
                return Err(ToolError::ExecutionFailed("rate limited by search API".into()));
            }
            return parse_search_results(&retry.body, num);
        }

        if response.status != 200 {
            return Err(ToolError::ExecutionFailed(
                format!("search API returned HTTP {}", response.status)
            ));
        }

        parse_search_results(&response.body, num)
    }
}

fn parse_search_results(body: &[u8], max: u64) -> Result<Value, ToolError> {
    let json: Value = serde_json::from_slice(body)
        .map_err(|e| ToolError::ExecutionFailed(format!("invalid search response: {e}")))?;

    let results = json.get("results")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![]);

    let items: Vec<Value> = results.iter()
        .take(max as usize)
        .map(|r| json!({
            "title": r.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "url": r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
            "snippet": r.get("content").and_then(|v| v.as_str()).unwrap_or(""),
        }))
        .collect();

    Ok(json!({
        "results": items,
        "count": items.len(),
    }))
}
```

### 3.6 `web_fetch` Tool

```
// clawft-tools/src/web_fetch_tool.rs

/// Maximum response body size to prevent memory exhaustion.
const ABSOLUTE_MAX_LENGTH: usize = 200_000;
const DEFAULT_MAX_LENGTH: usize = 50_000;
const DEFAULT_TIMEOUT_SECS: u64 = 15;

pub struct WebFetchTool<P: Platform> {
    platform: Arc<P>,
    timeout: Duration,
}

impl<P: Platform> WebFetchTool<P> {
    pub fn new(platform: Arc<P>, timeout: Duration) -> Self { ... }
}

#[async_trait]
impl<P: Platform + 'static> Tool for WebFetchTool<P> {
    fn name(&self) -> &str { "web_fetch" }

    fn description(&self) -> &str {
        "Fetch a web page and return its text content (HTML tags stripped)."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (http or https only)"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum content length in characters (100-200000, default 50000)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let url_str = required_str(&args, "url")?;
        let max_len = args.get("max_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_MAX_LENGTH as u64)
            .clamp(100, ABSOLUTE_MAX_LENGTH as u64) as usize;

        // Validate scheme.
        if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
            return Err(ToolError::InvalidArgs(
                format!("unsupported URL scheme (only http/https): {url_str}")
            ));
        }

        let headers = HashMap::from([
            ("User-Agent".into(), "clawft/0.1".into()),
            ("Accept".into(), "text/html, text/plain, application/json".into()),
        ]);

        let response = self.platform.http()
            .request("GET", &url_str, &headers, None)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("fetch failed: {e}")))?;

        if response.status >= 400 {
            return Err(ToolError::ExecutionFailed(
                format!("HTTP {} from {url_str}", response.status)
            ));
        }

        let body = String::from_utf8_lossy(&response.body);
        let content_type = response.headers.get("content-type")
            .cloned()
            .unwrap_or_default();

        // Strip HTML if content-type indicates HTML.
        let text = if content_type.contains("text/html") {
            strip_html_tags(&body)
        } else {
            body.to_string()
        };

        let truncated = text.len() > max_len;
        let output = if truncated {
            &text[..max_len]
        } else {
            &text
        };

        Ok(json!({
            "content": output,
            "url": url_str,
            "content_type": content_type,
            "truncated": truncated,
            "length": output.len(),
        }))
    }
}

/// Basic HTML tag stripping.
///
/// Removes `<script>` and `<style>` blocks entirely, then strips remaining
/// tags. Decodes common HTML entities (&amp;, &lt;, &gt;, &quot;, &nbsp;).
/// This is intentionally simple; a full HTML parser is out of scope.
fn strip_html_tags(html: &str) -> String {
    // Step 1: Remove <script>...</script> and <style>...</style> blocks.
    let no_script = regex_remove(html, r"(?is)<script[^>]*>.*?</script>");
    let no_style = regex_remove(&no_script, r"(?is)<style[^>]*>.*?</style>");

    // Step 2: Replace <br>, <p>, <div>, <li> with newlines for readability.
    let with_breaks = no_style
        .replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n")
        .replace("</p>", "\n").replace("</div>", "\n").replace("</li>", "\n");

    // Step 3: Strip all remaining tags.
    let stripped = regex_remove(&with_breaks, r"<[^>]+>");

    // Step 4: Decode common entities.
    stripped
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        // Collapse whitespace.
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
```

### 3.7 `message` Tool

```
// clawft-tools/src/message_tool.rs

use tokio::sync::mpsc;

pub struct MessageTool {
    /// Cloned sender handle to the bus's outbound channel.
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
}

impl MessageTool {
    pub fn new(outbound_tx: mpsc::UnboundedSender<OutboundMessage>) -> Self {
        Self { outbound_tx }
    }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str { "message" }

    fn description(&self) -> &str {
        "Send a message to a channel via the message bus."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Target channel name (e.g. 'telegram', 'slack')"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Target chat/conversation identifier"
                },
                "content": {
                    "type": "string",
                    "description": "Message text to send"
                },
                "reply_to": {
                    "type": "string",
                    "description": "Optional message ID to reply to"
                }
            },
            "required": ["channel", "chat_id", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let channel = required_str(&args, "channel")?;
        let chat_id = required_str(&args, "chat_id")?;
        let content = required_str(&args, "content")?;
        let reply_to = args.get("reply_to").and_then(|v| v.as_str()).map(String::from);

        let msg = OutboundMessage {
            channel: channel.clone(),
            chat_id: chat_id.clone(),
            content,
            reply_to,
            media: Vec::new(),
        };

        self.outbound_tx.send(msg)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("failed to send outbound message: {e}")
            ))?;

        Ok(json!({
            "sent": true,
            "channel": channel,
            "chat_id": chat_id,
        }))
    }
}
```

### 3.8 `spawn` Tool (Level 0 Stub)

```
// clawft-tools/src/spawn_tool.rs

use clawft_core::pipeline::traits::PipelineRegistry;

/// Spawn a subagent that runs a single LLM completion.
///
/// Level 0 implementation: one-shot pipeline.complete() call.
/// Future levels will add: tool subset selection, recursion depth,
/// budget limits, and multi-turn agent loops.
pub struct SpawnTool {
    /// Pipeline registry for executing subagent completions.
    pipeline: Arc<PipelineRegistry>,

    // -- Extension points for future levels --
    // max_tools: Option<Vec<String>>,  // Level 1: restrict tool set
    // max_depth: u32,                  // Level 2: recursion limit
    // budget_tokens: Option<u64>,      // Level 3: token budget
}

impl SpawnTool {
    pub fn new(pipeline: Arc<PipelineRegistry>) -> Self {
        Self { pipeline }
    }
}

#[async_trait]
impl Tool for SpawnTool {
    fn name(&self) -> &str { "spawn" }

    fn description(&self) -> &str {
        "Spawn a subagent to complete a task. Returns the LLM response."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The task prompt for the subagent"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override (e.g. 'openai/gpt-4')"
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt for the subagent"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let prompt = required_str(&args, "prompt")?;
        let model = args.get("model").and_then(|v| v.as_str()).map(String::from);
        let system_prompt = args.get("system_prompt")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Build a ChatRequest for the subagent.
        let mut messages = Vec::new();
        if let Some(sys) = &system_prompt {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(&prompt));

        let request = ChatRequest {
            messages,
            model: model.clone(),
            ..Default::default()
        };

        let response = self.pipeline.complete(&request).await
            .map_err(|e| ToolError::ExecutionFailed(
                format!("subagent completion failed: {e}")
            ))?;

        Ok(json!({
            "content": response.content,
            "model_used": response.model.unwrap_or_default(),
            "tokens_used": response.usage.map(|u| u.total_tokens).unwrap_or(0),
        }))
    }
}
```

---

## 4. Architecture

### 4.1 CLI Command Hierarchy (After Changes)

```
weft
├── agent          (existing)
├── gateway        (existing)
├── status         (existing)
├── channels       (existing)
│   └── status
├── cron           (existing)
│   ├── list / add / remove / enable / disable / run
├── sessions       (NEW)
│   ├── list
│   ├── show <id>
│   └── delete <id>
├── memory         (NEW)
│   ├── show
│   ├── search <query>
│   └── history
├── config         (NEW)
│   ├── show
│   ├── path
│   └── validate
└── completions    (NEW)
    └── <bash|zsh|fish|powershell>
```

### 4.2 Main.rs Changes

The `Commands` enum in `main.rs` gains four new variants:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing variants unchanged ...

    /// Manage conversation sessions.
    Sessions {
        #[command(subcommand)]
        action: SessionsAction,
    },

    /// Inspect and search agent memory.
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },

    /// Show and validate configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Generate shell completion scripts.
    Completions(commands::completions::CompletionsArgs),
}
```

The `match cli.command` block gains four new arms following the existing
pattern (construct platform, load config, dispatch to module function).

### 4.3 Dependency Changes

**clawft-cli/Cargo.toml additions:**
```toml
clap_complete = { version = "4" }
```

**clawft-tools/Cargo.toml additions:**
```toml
regex = { workspace = true }        # For HTML stripping in web_fetch
urlencoding = "2"                   # For query encoding in web_search
```

No new workspace dependencies are needed -- `regex` and `tokio` are
already workspace deps. `urlencoding` is small (no transitive deps)
and gets added to the workspace `[dependencies]` table.

### 4.4 Tool Registration Changes

In `clawft-tools/src/lib.rs`, the `register_all` function signature
expands to accept additional constructor arguments:

```rust
pub fn register_all<P: Platform + 'static>(
    registry: &mut ToolRegistry,
    platform: Arc<P>,
    workspace_dir: PathBuf,
    outbound_tx: Option<mpsc::UnboundedSender<OutboundMessage>>,
    pipeline: Option<Arc<PipelineRegistry>>,
    tools_config: &ToolsConfig,
) {
    // ... existing tool registrations ...

    // New tools (conditionally registered based on config/availability).
    if let Some(tool) = WebSearchTool::from_config(platform.clone(), tools_config) {
        registry.register(Arc::new(tool));
    }
    registry.register(Arc::new(WebFetchTool::new(
        platform.clone(),
        Duration::from_secs(DEFAULT_TIMEOUT_SECS),
    )));
    if let Some(tx) = outbound_tx {
        registry.register(Arc::new(MessageTool::new(tx)));
    }
    if let Some(pl) = pipeline {
        registry.register(Arc::new(SpawnTool::new(pl)));
    }
}
```

### 4.5 SessionManager Extension

The `SessionManager` currently has no `delete_session` method. The
delete functionality requires either:

1. **Option A**: Add `pub async fn delete_session(&self, key: &str)` to
   `SessionManager` in clawft-core (preferred -- keeps FS logic centralized).
2. **Option B**: Use `platform.fs().remove_file()` directly in the CLI
   command module (leaks implementation detail).

**Decision**: Option A. Add this method to SessionManager:

```rust
/// Delete a session from disk and invalidate the cache.
///
/// Returns an error if the session file does not exist.
pub async fn delete_session(&self, key: &str) -> clawft_types::Result<()> {
    let path = self.session_path(key);
    if !self.platform.fs().exists(&path).await {
        return Err(ClawftError::ConfigInvalid {
            reason: format!("session not found: {key}"),
        });
    }
    self.platform.fs().remove_file(&path).await.map_err(ClawftError::Io)?;
    self.invalidate(key).await;
    debug!(key = key, "deleted session");
    Ok(())
}
```

### 4.6 Error Handling Strategy

**CLI commands**: Use `anyhow::Result<()>` for all command functions.
Errors print to stderr and exit with code 1. Non-error "not found"
states (e.g., empty session list) print a message to stdout and exit 0.

**Tools**: Return `ToolError` variants mapped to the failure mode:
- Missing/invalid arguments -> `ToolError::InvalidArgs`
- Network errors -> `ToolError::ExecutionFailed`
- Timeouts -> `ToolError::Timeout`
- Invalid URL scheme -> `ToolError::InvalidArgs`

### 4.7 Module Dependency Graph

```
clawft-cli
  ├── clawft-core (SessionManager, MemoryStore, Pipeline)
  ├── clawft-platform (NativePlatform, Platform trait)
  ├── clawft-types (Config, OutboundMessage)
  ├── clap + clap_complete
  └── comfy-table

clawft-tools
  ├── clawft-core (Tool trait, ToolRegistry)
  ├── clawft-platform (Platform, HttpClient)
  ├── clawft-types (OutboundMessage, ToolsConfig)
  ├── regex (HTML stripping)
  └── urlencoding (search query encoding)
```

---

## 5. Refinement

### 5.1 TDD Strategy

#### Phase 1: CLI Parsing Tests (Red-Green for clap structs)

All CLI tests go in the existing `main.rs` `#[cfg(test)] mod tests`
block, following the established pattern of `Cli::try_parse_from`.

```rust
// -- Sessions parsing tests --

#[test]
fn cli_sessions_list_parses() {
    let result = Cli::try_parse_from(["weft", "sessions", "list"]);
    assert!(result.is_ok());
}

#[test]
fn cli_sessions_show_parses() {
    let result = Cli::try_parse_from(["weft", "sessions", "show", "telegram:123"]);
    assert!(result.is_ok());
}

#[test]
fn cli_sessions_delete_parses() {
    let result = Cli::try_parse_from(["weft", "sessions", "delete", "telegram:123"]);
    assert!(result.is_ok());
}

#[test]
fn cli_sessions_delete_with_yes_parses() {
    let result = Cli::try_parse_from(["weft", "sessions", "delete", "telegram:123", "-y"]);
    assert!(result.is_ok());
}

// -- Memory parsing tests --

#[test]
fn cli_memory_show_parses() {
    let result = Cli::try_parse_from(["weft", "memory", "show"]);
    assert!(result.is_ok());
}

#[test]
fn cli_memory_search_parses() {
    let result = Cli::try_parse_from(["weft", "memory", "search", "authentication"]);
    assert!(result.is_ok());
}

#[test]
fn cli_memory_search_with_limit_parses() {
    let result = Cli::try_parse_from(["weft", "memory", "search", "auth", "--limit", "5"]);
    assert!(result.is_ok());
}

#[test]
fn cli_memory_history_parses() {
    let result = Cli::try_parse_from(["weft", "memory", "history"]);
    assert!(result.is_ok());
}

// -- Config parsing tests --

#[test]
fn cli_config_show_parses() {
    let result = Cli::try_parse_from(["weft", "config", "show"]);
    assert!(result.is_ok());
}

#[test]
fn cli_config_show_unmask_parses() {
    let result = Cli::try_parse_from(["weft", "config", "show", "--unmask"]);
    assert!(result.is_ok());
}

#[test]
fn cli_config_path_parses() {
    let result = Cli::try_parse_from(["weft", "config", "path"]);
    assert!(result.is_ok());
}

#[test]
fn cli_config_validate_parses() {
    let result = Cli::try_parse_from(["weft", "config", "validate"]);
    assert!(result.is_ok());
}

// -- Completions parsing tests --

#[test]
fn cli_completions_bash_parses() {
    let result = Cli::try_parse_from(["weft", "completions", "bash"]);
    assert!(result.is_ok());
}

#[test]
fn cli_completions_zsh_parses() {
    let result = Cli::try_parse_from(["weft", "completions", "zsh"]);
    assert!(result.is_ok());
}

// -- Updated has_all_subcommands test --

#[test]
fn cli_has_all_subcommands() {
    let cmd = Cli::command();
    let sub_names: Vec<&str> = cmd
        .get_subcommands()
        .map(|s| s.get_name())
        .collect();
    assert!(sub_names.contains(&"agent"));
    assert!(sub_names.contains(&"gateway"));
    assert!(sub_names.contains(&"status"));
    assert!(sub_names.contains(&"channels"));
    assert!(sub_names.contains(&"cron"));
    assert!(sub_names.contains(&"sessions"));
    assert!(sub_names.contains(&"memory"));
    assert!(sub_names.contains(&"config"));
    assert!(sub_names.contains(&"completions"));
}
```

#### Phase 2: Config Masking Unit Tests

```rust
// commands/config.rs #[cfg(test)]

#[test]
fn mask_sensitive_masks_api_key() {
    let mut val = json!({
        "providers": {
            "openai": {
                "api_key": "sk-1234567890abcdef"
            }
        }
    });
    mask_sensitive(&mut val);
    assert_eq!(val["providers"]["openai"]["api_key"], "***");
}

#[test]
fn mask_sensitive_preserves_non_sensitive() {
    let mut val = json!({
        "agents": {
            "defaults": {
                "model": "openai/gpt-4",
                "workspace": "~/workspace"
            }
        }
    });
    let original = val.clone();
    mask_sensitive(&mut val);
    assert_eq!(val, original);
}

#[test]
fn mask_sensitive_handles_nested_tokens() {
    let mut val = json!({
        "channels": {
            "telegram": {
                "bot_token": "123:ABCdef"
            },
            "slack": {
                "app_token": "xapp-1-A0",
                "bot_token": "xoxb-1-2"
            }
        }
    });
    mask_sensitive(&mut val);
    assert_eq!(val["channels"]["telegram"]["bot_token"], "***");
    assert_eq!(val["channels"]["slack"]["app_token"], "***");
    assert_eq!(val["channels"]["slack"]["bot_token"], "***");
}

#[test]
fn mask_sensitive_handles_arrays() {
    let mut val = json!({
        "items": [
            {"api_key": "secret1"},
            {"name": "visible"}
        ]
    });
    mask_sensitive(&mut val);
    assert_eq!(val["items"][0]["api_key"], "***");
    assert_eq!(val["items"][1]["name"], "visible");
}
```

#### Phase 3: HTML Stripping Tests

```rust
// clawft-tools/src/web_fetch_tool.rs #[cfg(test)]

#[test]
fn strip_html_removes_tags() {
    let html = "<p>Hello <b>world</b></p>";
    let text = strip_html_tags(html);
    assert_eq!(text, "Hello world");
}

#[test]
fn strip_html_removes_script_blocks() {
    let html = "<p>Before</p><script>alert('xss')</script><p>After</p>";
    let text = strip_html_tags(html);
    assert!(text.contains("Before"));
    assert!(text.contains("After"));
    assert!(!text.contains("alert"));
    assert!(!text.contains("script"));
}

#[test]
fn strip_html_removes_style_blocks() {
    let html = "<style>body{color:red}</style><p>Content</p>";
    let text = strip_html_tags(html);
    assert!(text.contains("Content"));
    assert!(!text.contains("color"));
}

#[test]
fn strip_html_decodes_entities() {
    let html = "A &amp; B &lt; C &gt; D &quot;E&quot;";
    let text = strip_html_tags(html);
    assert!(text.contains("A & B < C > D \"E\""));
}

#[test]
fn strip_html_converts_breaks_to_newlines() {
    let html = "Line 1<br>Line 2<br/>Line 3";
    let text = strip_html_tags(html);
    assert!(text.contains("Line 1\nLine 2\nLine 3"));
}

#[test]
fn strip_html_handles_empty_input() {
    assert_eq!(strip_html_tags(""), "");
}

#[test]
fn strip_html_handles_plain_text() {
    assert_eq!(strip_html_tags("Just text"), "Just text");
}
```

#### Phase 4: Tool Parameter Validation Tests

```rust
// clawft-tools/src/web_search_tool.rs #[cfg(test)]

#[tokio::test]
async fn web_search_missing_query_returns_error() {
    let tool = make_test_search_tool();
    let err = tool.execute(json!({})).await.unwrap_err();
    assert!(matches!(err, ToolError::InvalidArgs(_)));
}

#[tokio::test]
async fn web_search_clamps_num_results() {
    // Verify clamping happens at the argument parsing level.
    let tool = make_test_search_tool();
    // num_results=100 should be clamped to 20 internally.
    let args = json!({"query": "test", "num_results": 100});
    // With mock HTTP, just verify no panic. Clamping is internal.
    let _ = tool.execute(args).await;
}

// clawft-tools/src/web_fetch_tool.rs #[cfg(test)]

#[tokio::test]
async fn web_fetch_missing_url_returns_error() {
    let tool = make_test_fetch_tool();
    let err = tool.execute(json!({})).await.unwrap_err();
    assert!(matches!(err, ToolError::InvalidArgs(_)));
}

#[tokio::test]
async fn web_fetch_rejects_ftp_scheme() {
    let tool = make_test_fetch_tool();
    let err = tool.execute(json!({"url": "ftp://example.com/file"})).await.unwrap_err();
    assert!(matches!(err, ToolError::InvalidArgs(_)));
}

#[tokio::test]
async fn web_fetch_rejects_file_scheme() {
    let tool = make_test_fetch_tool();
    let err = tool.execute(json!({"url": "file:///etc/passwd"})).await.unwrap_err();
    assert!(matches!(err, ToolError::InvalidArgs(_)));
}

// clawft-tools/src/message_tool.rs #[cfg(test)]

#[tokio::test]
async fn message_tool_missing_channel_returns_error() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let tool = MessageTool::new(tx);
    let err = tool.execute(json!({"chat_id": "1", "content": "hi"})).await.unwrap_err();
    assert!(matches!(err, ToolError::InvalidArgs(_)));
}

#[tokio::test]
async fn message_tool_sends_outbound() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let tool = MessageTool::new(tx);

    let result = tool.execute(json!({
        "channel": "telegram",
        "chat_id": "12345",
        "content": "Hello!",
    })).await.unwrap();

    assert_eq!(result["sent"], true);
    assert_eq!(result["channel"], "telegram");

    let msg = rx.try_recv().unwrap();
    assert_eq!(msg.channel, "telegram");
    assert_eq!(msg.chat_id, "12345");
    assert_eq!(msg.content, "Hello!");
}

// clawft-tools/src/spawn_tool.rs #[cfg(test)]

#[tokio::test]
async fn spawn_tool_missing_prompt_returns_error() {
    let tool = make_test_spawn_tool();
    let err = tool.execute(json!({})).await.unwrap_err();
    assert!(matches!(err, ToolError::InvalidArgs(_)));
}
```

#### Phase 5: SessionManager.delete_session Tests

```rust
// clawft-core/src/session.rs #[cfg(test)]

#[tokio::test]
async fn delete_session_removes_file() {
    let platform = make_platform();
    let mgr = make_manager(platform.clone());

    let session = Session::new("delete:test");
    mgr.save_session(&session).await.unwrap();

    mgr.delete_session("delete:test").await.unwrap();

    // File should be gone.
    let path = PathBuf::from(
        "/mock-home/.clawft/workspace/sessions/delete_test.jsonl",
    );
    assert!(!platform.fs.exists(&path).await);
}

#[tokio::test]
async fn delete_session_invalidates_cache() {
    let platform = make_platform();
    let mgr = make_manager(platform);

    let session = Session::new("cache:delete");
    mgr.save_session(&session).await.unwrap();

    mgr.delete_session("cache:delete").await.unwrap();

    let cache = mgr.active_sessions.lock().await;
    assert!(!cache.contains_key("cache:delete"));
}

#[tokio::test]
async fn delete_session_nonexistent_returns_error() {
    let platform = make_platform();
    let mgr = make_manager(platform);

    let result = mgr.delete_session("nonexistent:key").await;
    assert!(result.is_err());
}
```

### 5.2 Test Coverage Requirements

- **Unit test coverage**: >= 80% for all new modules
- **Critical paths**: 100% coverage for `mask_sensitive`, `strip_html_tags`, parameter validation
- **CLI parsing**: Every subcommand variant has a parse test
- **Tool trait compliance**: Every new tool has tests for `name()`, `description()`, `parameters()` schema validity, and `execute()` error paths

### 5.3 Implementation Order

1. **SessionManager.delete_session** (clawft-core) -- prerequisite for CLI
2. **CLI modules**: sessions.rs, memory.rs, config.rs, completions.rs in parallel
3. **main.rs**: Wire up new Commands enum variants + match arms
4. **Cargo.toml**: Add clap_complete dep
5. **Tool implementations**: web_search, web_fetch, message, spawn in parallel
6. **lib.rs**: Update register_all with new tools + expanded signature
7. **Integration**: Verify `cargo build` and `cargo test --workspace` pass

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [ ] All unit tests passing (`cargo test -p clawft-cli`)
- [ ] All unit tests passing (`cargo test -p clawft-tools`)
- [ ] All unit tests passing (`cargo test -p clawft-core` -- for delete_session)
- [ ] `cargo clippy --workspace` reports no warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo build --release` succeeds

### 6.2 CLI Integration
- [ ] `weft --help` lists all 9 subcommands (agent, gateway, status, channels, cron, sessions, memory, config, completions)
- [ ] `weft sessions list` works with real sessions directory
- [ ] `weft sessions show <id>` works with a real session file
- [ ] `weft sessions delete <id>` deletes a real session file
- [ ] `weft memory show` works with real MEMORY.md
- [ ] `weft memory search <query>` returns correct matches
- [ ] `weft memory history` works with real HISTORY.md
- [ ] `weft config show` displays resolved config with masking
- [ ] `weft config path` shows correct discovered path
- [ ] `weft config validate` validates real config file
- [ ] `weft completions bash | head` produces valid completion script
- [ ] `weft completions zsh | head` produces valid completion script

### 6.3 Tool Integration
- [ ] `web_search` tool appears in `ToolRegistry::list()` when search endpoint is configured
- [ ] `web_fetch` tool appears in `ToolRegistry::list()` always
- [ ] `message` tool appears when outbound_tx is provided
- [ ] `spawn` tool appears when pipeline is provided
- [ ] All new tools appear in `ToolRegistry::schemas()` with correct OpenAI format
- [ ] Agent loop can invoke `web_search` via tool-use response
- [ ] Agent loop can invoke `web_fetch` via tool-use response
- [ ] Agent loop can invoke `message` via tool-use response
- [ ] Agent loop can invoke `spawn` via tool-use response

### 6.4 Backward Compatibility
- [ ] Existing `weft agent`, `weft gateway`, `weft status` commands unchanged
- [ ] Existing `weft channels status` command unchanged
- [ ] Existing `weft cron` subcommands unchanged
- [ ] `register_all` callers updated to pass new parameters
- [ ] No breaking changes to Tool trait or ToolRegistry API

### 6.5 Documentation
- [ ] `weft --help` descriptions are clear for all new subcommands
- [ ] Tool descriptions and parameter schemas are self-documenting
- [ ] Code-level doc comments on all public functions/structs

### 6.6 Error Handling Validation
- [ ] `weft sessions show nonexistent` prints clear error, exits 1
- [ ] `weft sessions delete nonexistent` prints clear error, exits 1
- [ ] `weft config validate` with malformed JSON exits 1 with details
- [ ] `web_search` with no configured endpoint returns descriptive error
- [ ] `web_fetch` with invalid URL returns descriptive error
- [ ] `message` with closed bus returns descriptive error
- [ ] `spawn` with pipeline failure returns descriptive error

### 6.7 Performance
- [ ] `weft sessions list` with 100+ sessions completes in <1s
- [ ] `weft config show` completes in <100ms
- [ ] HTML stripping handles 200KB pages in <50ms
- [ ] No unnecessary allocations in `mask_sensitive` (operates in-place)

### 6.8 Final Review
- [ ] Code review completed
- [ ] No hardcoded secrets or credentials
- [ ] All file paths go through platform abstraction (no raw std::fs)
- [ ] Workspace Cargo.toml updated if new deps added
- [ ] Version bumped if needed

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockPlatform, MockFileSystem};` for CLI command tests that need filesystem
- **Import mocks from session.rs**: Reuse `MockFs`, `MockPlatform` pattern from existing session.rs tests
- **Import from bus.rs**: Reuse `make_outbound` helper for message tool tests
- **Use shared fixtures**: Load `tests/fixtures/config.json` for config validation tests

### Security Tests (Required)
- Config masking must not leak sensitive values through Debug/Display impls
- `web_fetch` must reject `file://` and other local schemes
- `web_search` query must be URL-encoded to prevent injection
- `spawn` must not expose internal pipeline errors to the agent (sanitize error messages)
- `message` tool must not allow empty channel/chat_id (prevent broadcast to all)

### Coverage Target
- Unit test coverage: >= 80% (measured via `cargo-tarpaulin`)
- Critical paths (mask_sensitive, strip_html, URL validation, parameter validation): 100%

---

## Notes for Implementation Agent

1. **Read existing CLI command modules first** to match patterns exactly (config loading, platform construction, error handling)
2. **Use TDD London School**: Write failing tests for clap parsing first, then implement structs
3. **Parallel file operations**: Create all 4 CLI modules + 4 tool modules in a single message
4. **`register_all` signature change is breaking**: Update all callers (agent.rs, gateway.rs) in the same commit
5. **`clap_complete` integration**: The `Shell` enum from `clap_complete` implements `clap::ValueEnum`, so it works directly as a clap arg type
6. **HTML stripping**: Use `regex` crate (already in workspace) rather than pulling in a full HTML parser -- keeps binary size small
7. **`web_search` is config-dependent**: If no search endpoint is configured, the tool is simply not registered (no runtime error)
8. **`spawn` tool is a stub**: Keep it simple (one-shot completion), document extension points with comments, do not over-engineer
9. **Logging**: Use `tracing` for debug logs with structured fields, matching existing tool patterns
10. **`SessionManager.delete_session`**: Add to clawft-core first, then wire into CLI -- this is a prerequisite
