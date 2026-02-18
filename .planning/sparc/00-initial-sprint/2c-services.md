# SPARC Implementation Plan: Stream 2C - Services

**Timeline**: Week 7-11
**Owned Crates**: `clawft-services`
**Dependencies**: Phase 1 complete (MessageBus, Agent, config system), clawft-llm for Codex OAuth

---

## 1. Agent Instructions

### Python Source Files to Read
```
repos/nanobot/nanobot/cron/service.py                   # Cron scheduling (main service)
repos/nanobot/nanobot/cron/types.py                     # Cron job types + schemas
repos/nanobot/nanobot/heartbeat/service.py              # Heartbeat timer service
repos/nanobot/nanobot/agent/tools/mcp.py                # MCP client (JSON-RPC)
repos/nanobot/nanobot/providers/openai_codex_provider.py # Codex OAuth provider
```

### Planning Documents (MUST READ)
```
repos/nanobot/.planning/02-technical-requirements.md    # clawft-services spec
repos/nanobot/.planning/03-development-guide.md         # Stream 2C timeline
repos/nanobot/.planning/06-provider-layer-options.md    # litellm-rs sidecar details (optional)
repos/nanobot/.planning/01-project-overview.md          # Services overview
```

### Module Structure
```
clawft-services/
├── Cargo.toml                      # Dependencies: cron 0.15, serde_json, tokio
├── src/
│   ├── lib.rs                      # Service trait + registry
│   ├── cron/
│   │   ├── mod.rs                  # CronService + CronJob types
│   │   ├── scheduler.rs            # Cron scheduling engine
│   │   ├── storage.rs              # JSONL persistence
│   │   └── executor.rs             # Job execution via MessageBus
│   ├── heartbeat/
│   │   ├── mod.rs                  # HeartbeatService
│   │   └── timer.rs                # Interval-based timer
│   ├── mcp/
│   │   ├── mod.rs                  # McpClient + McpTool
│   │   ├── stdio_transport.rs     # JSON-RPC over stdio
│   │   ├── http_transport.rs      # JSON-RPC over HTTP
│   │   ├── tool_registry.rs       # Dynamic tool registration
│   │   └── error.rs                # McpError type
│   └── codex/
│       ├── mod.rs                  # CodexOAuthProvider (if not in clawft-llm)
│       └── oauth_flow.rs           # Browser-based OAuth
├── tests/
│   ├── cron_tests.rs               # Cron service tests
│   ├── heartbeat_tests.rs          # Heartbeat service tests
│   ├── mcp_stdio_tests.rs          # MCP stdio transport tests
│   └── mcp_http_tests.rs           # MCP HTTP transport tests
```

---

## 2. Specification

### 2.1 Cron Service (Week 7-8)

#### CronJob Type
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,           // Cron expression: "0 0 * * *"
    pub prompt: String,              // LLM prompt to send
    pub enabled: bool,
    pub last_run: Option<SystemTime>,
    pub next_run: Option<SystemTime>,
    pub created_at: SystemTime,
}
```

#### Cron Schedule Parsing
- **Library**: `cron` crate (0.15)
- **Syntax**: Standard cron format: `min hour day month weekday`
- **Examples**:
  - `0 0 * * *` - Daily at midnight
  - `0 */6 * * *` - Every 6 hours
  - `*/15 * * * *` - Every 15 minutes

#### Persistence (JSONL)
- **File**: `~/.config/weft/cron_jobs.jsonl`
- **Format**: One JSON object per line (append-only)
- **Schema**:
```json
{"type":"create","job":{"id":"abc123","name":"daily-summary","schedule":"0 0 * * *","prompt":"Summarize today's work","enabled":true,"created_at":1234567890}}
{"type":"update","job_id":"abc123","field":"enabled","value":false}
{"type":"delete","job_id":"abc123"}
```

#### Execution Flow
```
1. CronScheduler runs in background task (tokio::spawn)
2. Every minute: check all enabled jobs
3. For each job where now >= next_run:
   a. Post InboundMessage to MessageBus with cron job prompt
   b. Update last_run, compute next_run
   c. Write update to JSONL
```

#### API
```rust
pub struct CronService {
    scheduler: Arc<RwLock<CronScheduler>>,
    storage: CronStorage,
    message_bus: Arc<MessageBus>,
}

impl CronService {
    pub async fn new(config: CronConfig, message_bus: Arc<MessageBus>) -> Result<Self>;
    pub async fn add_job(&self, job: CronJob) -> Result<String>;
    pub async fn remove_job(&self, job_id: &str) -> Result<()>;
    pub async fn enable_job(&self, job_id: &str, enabled: bool) -> Result<()>;
    pub async fn list_jobs(&self) -> Result<Vec<CronJob>>;
    pub async fn run_job_now(&self, job_id: &str) -> Result<()>;
}
```

### 2.2 Cron CLI Commands (Week 8)

#### `weft cron list`
- Display table of all cron jobs
- Columns: ID, Name, Schedule, Enabled, Last Run, Next Run
- Example output:
```
ID       NAME            SCHEDULE      ENABLED  LAST RUN              NEXT RUN
abc123   daily-summary   0 0 * * *     true     2025-01-15 00:00:00   2025-01-16 00:00:00
def456   hourly-check    0 * * * *     false    -                     -
```

#### `weft cron add`
- Prompt for: name, schedule, prompt
- Validate cron expression
- Add to storage, start scheduler
- Example: `weft cron add --name daily-summary --schedule "0 0 * * *" --prompt "Summarize today's work"`

#### `weft cron remove <job_id>`
- Remove job from storage
- Stop scheduler for job
- Example: `weft cron remove abc123`

#### `weft cron enable <job_id>` / `weft cron disable <job_id>`
- Toggle enabled flag
- Update storage
- Example: `weft cron enable abc123`

#### `weft cron run <job_id>`
- Execute job immediately (bypass schedule)
- Update last_run timestamp
- Example: `weft cron run abc123`

### 2.3 Heartbeat Service (Week 9)

#### HeartbeatService Type
```rust
pub struct HeartbeatService {
    interval: Duration,
    prompt: String,
    message_bus: Arc<MessageBus>,
    cancel: CancellationToken,
}
```

#### Configuration
```yaml
heartbeat:
  enabled: true
  interval_minutes: 30
  prompt: "Check for updates and summarize recent activity"
```

#### Execution Flow
```
1. HeartbeatService runs in background task (tokio::spawn)
2. Every interval (default: 30 minutes):
   a. Post InboundMessage to MessageBus with heartbeat prompt
   b. Log heartbeat execution
3. Cancellable via CancellationToken
```

#### API
```rust
impl HeartbeatService {
    pub async fn new(config: HeartbeatConfig, message_bus: Arc<MessageBus>) -> Result<Self>;
    pub async fn start(&self, cancel: CancellationToken) -> Result<()>;
    pub fn is_running(&self) -> bool;
}
```

### 2.4 MCP Client (Week 10)

#### JSON-RPC 2.0 Protocol
- **Specification**: https://www.jsonrpc.org/specification
- **Request**:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```
- **Response**:
```json
{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"calculator","description":"Math operations","input_schema":{...}}]}}
```
- **Error**:
```json
{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
```

#### stdio Transport
- **Communication**: Stdin/stdout with child process
- **Process spawn**: `tokio::process::Command`
- **Message framing**: Newline-delimited JSON
- **Example**: `npx @modelcontextprotocol/server-example`

#### HTTP Transport
- **Communication**: POST requests to HTTP endpoint
- **Headers**: `Content-Type: application/json`
- **Endpoint**: Configurable URL (e.g., `http://localhost:3000/mcp`)

#### Tool Discovery
```
1. Send tools/list request to MCP server
2. Parse response for tool definitions
3. For each tool:
   a. Create McpTool wrapper (implements clawft-core Tool trait)
   b. Register with ToolRegistry
4. Tools now available to Agent
```

#### Tool Execution
```
1. Agent calls Tool::execute(params)
2. McpTool converts params to JSON-RPC request
3. Send tools/call request to MCP server
4. Parse response, return result to Agent
```

#### API
```rust
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    request_id: AtomicU64,
}

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
}

pub struct StdioTransport {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub struct HttpTransport {
    client: reqwest::Client,
    endpoint: Url,
}

impl McpClient {
    pub async fn new(transport: Box<dyn McpTransport>) -> Result<Self>;
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>>;
    pub async fn call_tool(&self, name: &str, params: serde_json::Value) -> Result<serde_json::Value>;
}

pub struct McpTool {
    client: Arc<McpClient>,
    definition: ToolDefinition,
}

#[async_trait]
impl Tool for McpTool {
    async fn execute(&self, params: ToolParams) -> Result<ToolResult>;
}
```

### 2.5 Codex OAuth Provider (Week 11)

#### OAuth Flow (Browser-Based)
```
1. User runs: weft provider login codex
2. CLI starts local HTTP server on http://localhost:8765
3. CLI opens browser to https://api.openai.com/v1/oauth/authorize?client_id=...&redirect_uri=http://localhost:8765/callback
4. User logs in via OpenAI OAuth
5. Browser redirects to http://localhost:8765/callback?code=...
6. CLI exchanges code for access_token via POST /oauth/token
7. CLI stores token in config file
8. HTTP server shuts down
```

#### SSE Streaming (Codex-Specific)
- **Protocol**: Server-Sent Events (text/event-stream)
- **Endpoint**: `https://api.openai.com/v1/chat/completions` with `stream=true`
- **Format**:
```
data: {"choices":[{"delta":{"content":"Hello"}}]}
data: {"choices":[{"delta":{"content":" world"}}]}
data: [DONE]
```

#### API
```rust
pub struct CodexOAuthProvider {
    client: reqwest::Client,
    access_token: String,
    base_url: Url,
}

impl CodexOAuthProvider {
    pub async fn from_oauth_code(code: &str) -> Result<Self>;
    pub async fn from_stored_token(token: &str) -> Result<Self>;
}

#[async_trait]
impl Provider for CodexOAuthProvider {
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn stream_completion(&self, request: ChatRequest) -> Result<impl Stream<Item = ChatChunk>>;
}
```

### 2.6 Optional: litellm-rs Sidecar (Week 11, ~1.5 days LOE)

#### When to Use litellm-rs
- Needed for exotic providers not supported by OpenAI-compatible API
- Examples: Vertex AI (GCP), Bedrock (AWS), Azure OpenAI (non-standard auth)
- Alternative: Write native Rust provider for each (more work)

#### litellm-rs Architecture
```
clawft-llm (Rust)
  ↓
SidecarTransport (Rust)
  ↓ HTTP
litellm-rs (Python sidecar process)
  ↓
Exotic provider (Vertex, Bedrock, etc.)
```

#### SidecarTransport
```rust
pub struct SidecarTransport {
    sidecar_url: Url,  // http://localhost:8080
    client: reqwest::Client,
}

#[async_trait]
impl Transport for SidecarTransport {
    async fn send_request(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Forward to litellm-rs sidecar
        let response = self.client.post(&self.sidecar_url)
            .json(&request)
            .send().await?;
        Ok(response.json().await?)
    }
}
```

#### litellm-rs Sidecar Process
- **Language**: Python (uses litellm library)
- **Protocol**: HTTP server on localhost:8080
- **Startup**: `tokio::process::Command::new("litellm-rs").spawn()`
- **Shutdown**: Kill process on drop

#### Decision Point
- **Option A (Recommended)**: Skip litellm-rs for Phase 2, add in Phase 3 if needed
- **Option B**: Implement SidecarTransport (~1.5 days LOE), add to clawft-llm

---

## 3. Pseudocode

### 3.1 Cron Service Implementation
```rust
pub struct CronScheduler {
    jobs: HashMap<String, CronJob>,
    schedules: HashMap<String, Schedule>,  // From cron crate
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            schedules: HashMap::new(),
        }
    }

    pub fn add_job(&mut self, job: CronJob) -> Result<()> {
        let schedule = Schedule::from_str(&job.schedule)?;
        let next_run = schedule.upcoming(Utc).next().ok_or("Invalid schedule")?;

        let mut job = job;
        job.next_run = Some(next_run.into());

        self.schedules.insert(job.id.clone(), schedule);
        self.jobs.insert(job.id.clone(), job);

        Ok(())
    }

    pub fn remove_job(&mut self, job_id: &str) -> Result<()> {
        self.jobs.remove(job_id).ok_or("Job not found")?;
        self.schedules.remove(job_id);
        Ok(())
    }

    pub fn get_due_jobs(&mut self) -> Vec<CronJob> {
        let now = SystemTime::now();
        let mut due_jobs = Vec::new();

        for (job_id, job) in &mut self.jobs {
            if !job.enabled {
                continue;
            }

            if let Some(next_run) = job.next_run {
                if now >= next_run {
                    due_jobs.push(job.clone());

                    // Update next_run
                    if let Some(schedule) = self.schedules.get(job_id) {
                        if let Some(next) = schedule.upcoming(Utc).next() {
                            job.next_run = Some(next.into());
                            job.last_run = Some(now);
                        }
                    }
                }
            }
        }

        due_jobs
    }
}

async fn cron_scheduler_task(
    scheduler: Arc<RwLock<CronScheduler>>,
    storage: CronStorage,
    message_bus: Arc<MessageBus>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute

    loop {
        select! {
            _ = cancel.cancelled() => return Ok(()),

            _ = interval.tick() => {
                let due_jobs = {
                    let mut scheduler = scheduler.write().await;
                    scheduler.get_due_jobs()
                };

                for job in due_jobs {
                    tracing::info!("Executing cron job: {}", job.name);

                    // Post to MessageBus
                    let msg = InboundMessage {
                        text: job.prompt.clone(),
                        channel: "cron".to_string(),
                        user: "system".to_string(),
                        metadata: json!({"job_id": job.id, "job_name": job.name}),
                    };

                    if let Err(e) = message_bus.send(msg).await {
                        tracing::error!("Failed to send cron job message: {}", e);
                    }

                    // Persist update
                    storage.update_job(&job).await?;
                }
            }
        }
    }
}
```

### 3.2 Heartbeat Service Implementation
```rust
impl HeartbeatService {
    pub async fn start(&self, cancel: CancellationToken) -> Result<()> {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            select! {
                _ = cancel.cancelled() => {
                    tracing::info!("Heartbeat service stopped");
                    return Ok(());
                }

                _ = interval.tick() => {
                    tracing::info!("Heartbeat tick");

                    let msg = InboundMessage {
                        text: self.prompt.clone(),
                        channel: "heartbeat".to_string(),
                        user: "system".to_string(),
                        metadata: json!({"timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()}),
                    };

                    if let Err(e) = self.message_bus.send(msg).await {
                        tracing::error!("Failed to send heartbeat message: {}", e);
                    }
                }
            }
        }
    }
}
```

### 3.3 MCP stdio Transport
```rust
impl StdioTransport {
    pub async fn new(command: &str, args: &[String]) -> Result<Self> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or("Failed to capture stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Write request to stdin
        let request_json = serde_json::to_string(&request)?;
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        // Read response from stdout
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(anyhow!("JSON-RPC error: {} ({})", error.message, error.code));
        }

        Ok(response)
    }
}
```

### 3.4 MCP Tool Discovery + Execution
```rust
impl McpClient {
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id.fetch_add(1, Ordering::SeqCst),
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let response = self.transport.send_request(request).await?;

        let tools: Vec<ToolDefinition> = serde_json::from_value(response.result.unwrap())?;
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id.fetch_add(1, Ordering::SeqCst),
            method: "tools/call".to_string(),
            params: json!({"name": name, "arguments": params}),
        };

        let response = self.transport.send_request(request).await?;
        Ok(response.result.unwrap())
    }
}

#[async_trait]
impl Tool for McpTool {
    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let result = self.client.call_tool(&self.definition.name, params.into()).await?;

        Ok(ToolResult {
            output: result.to_string(),
            metadata: json!({"tool": self.definition.name}),
        })
    }

    fn name(&self) -> &str {
        &self.definition.name
    }

    fn description(&self) -> &str {
        &self.definition.description
    }
}

// Register MCP tools with ToolRegistry
pub async fn register_mcp_tools(mcp_client: Arc<McpClient>, registry: &mut ToolRegistry) -> Result<()> {
    let tools = mcp_client.list_tools().await?;

    for tool_def in tools {
        let mcp_tool = McpTool {
            client: mcp_client.clone(),
            definition: tool_def,
        };

        registry.register(Box::new(mcp_tool));
    }

    Ok(())
}
```

### 3.5 Codex OAuth Flow
```rust
pub async fn codex_oauth_login() -> Result<String> {
    // 1. Start local HTTP server
    let listener = TcpListener::bind("127.0.0.1:8765").await?;
    let (tx, mut rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);

        // Read HTTP request
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await.unwrap();

        // Parse code from query params
        if let Some(code) = extract_code_from_request(&request_line) {
            tx.send(code).unwrap();
        }

        // Send HTTP response
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 30\r\n\r\n<html>Login successful!</html>";
        reader.get_mut().write_all(response.as_bytes()).await.unwrap();
    });

    // 2. Open browser
    let auth_url = format!(
        "https://api.openai.com/v1/oauth/authorize?client_id={}&redirect_uri=http://localhost:8765/callback&response_type=code",
        CODEX_CLIENT_ID
    );
    open::that(auth_url)?;

    println!("Opening browser for OAuth login...");

    // 3. Wait for callback
    let code = rx.await?;

    // 4. Exchange code for token
    let token_response = reqwest::Client::new()
        .post("https://api.openai.com/v1/oauth/token")
        .json(&json!({
            "client_id": CODEX_CLIENT_ID,
            "client_secret": CODEX_CLIENT_SECRET,
            "code": code,
            "grant_type": "authorization_code",
            "redirect_uri": "http://localhost:8765/callback"
        }))
        .send().await?
        .json::<TokenResponse>().await?;

    Ok(token_response.access_token)
}
```

---

## 4. Architecture

### 4.1 clawft-services Architecture
```
clawft-services
├── CronService
│   ├── CronScheduler (in-memory job scheduler)
│   ├── CronStorage (JSONL persistence)
│   └── MessageBus integration (post job prompts)
│
├── HeartbeatService
│   ├── Timer (tokio interval)
│   └── MessageBus integration (post heartbeat prompts)
│
├── McpClient
│   ├── McpTransport trait (stdio, HTTP)
│   ├── StdioTransport (child process)
│   ├── HttpTransport (HTTP POST)
│   ├── ToolRegistry integration (register MCP tools)
│   └── Tool trait implementation (McpTool)
│
└── CodexOAuthProvider (if not in clawft-llm)
    ├── OAuth flow (browser-based)
    └── Provider trait implementation (SSE streaming)
```

### 4.2 Service Lifecycle
```
weft CLI startup
  ↓
Load config (services.yaml)
  ↓
Initialize MessageBus
  ↓
Spawn services concurrently:
  ├─ CronService::start()
  │   ├─ Load jobs from JSONL
  │   ├─ Initialize CronScheduler
  │   └─ Start background task (check every minute)
  │
  ├─ HeartbeatService::start()
  │   ├─ Start timer (interval from config)
  │   └─ Post heartbeat messages to MessageBus
  │
  └─ McpClient::initialize()
      ├─ Spawn MCP server process (stdio)
      ├─ Discover tools (tools/list)
      └─ Register tools with ToolRegistry
  ↓
Services run until shutdown
  ↓
Graceful shutdown:
  ├─ Cancel CancellationToken
  ├─ Wait for background tasks
  └─ Close MCP server process
```

### 4.3 Dependency Graph
```
clawft-services
├── clawft-core (MessageBus, Tool trait, config)
├── clawft-llm (Provider trait for Codex OAuth)
├── cron 0.15 (cron expression parsing)
├── serde_json 1.0 (JSON-RPC, JSONL)
├── tokio 1.42 (async runtime, process spawn)
├── reqwest 0.12 (HTTP client for OAuth, MCP HTTP)
└── open 5.0 (open browser for OAuth)
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 Cron Service Tests

#### Unit Tests
```rust
// tests/cron_tests.rs
#[test]
fn test_cron_schedule_parsing() {
    let schedule = Schedule::from_str("0 0 * * *").unwrap();
    let next_run = schedule.upcoming(Utc).next().unwrap();
    assert!(next_run > Utc::now());
}

#[test]
fn test_cron_scheduler_add_job() {
    let mut scheduler = CronScheduler::new();
    let job = CronJob {
        id: "test".to_string(),
        name: "test job".to_string(),
        schedule: "0 0 * * *".to_string(),
        prompt: "test prompt".to_string(),
        enabled: true,
        last_run: None,
        next_run: None,
        created_at: SystemTime::now(),
    };

    scheduler.add_job(job.clone()).unwrap();

    assert_eq!(scheduler.jobs.len(), 1);
    assert!(scheduler.jobs.get("test").unwrap().next_run.is_some());
}

#[test]
fn test_cron_scheduler_get_due_jobs() {
    let mut scheduler = CronScheduler::new();

    // Job that should run immediately (cron: every minute)
    let job = CronJob {
        id: "test".to_string(),
        schedule: "* * * * *".to_string(),
        enabled: true,
        next_run: Some(SystemTime::now() - Duration::from_secs(60)), // Past
        ..Default::default()
    };

    scheduler.add_job(job).unwrap();

    let due_jobs = scheduler.get_due_jobs();
    assert_eq!(due_jobs.len(), 1);
}

#[tokio::test]
async fn test_cron_storage_jsonl() {
    let tmpdir = TempDir::new().unwrap();
    let storage = CronStorage::new(tmpdir.path().join("cron_jobs.jsonl"));

    let job = CronJob {
        id: "test".to_string(),
        name: "test job".to_string(),
        schedule: "0 0 * * *".to_string(),
        ..Default::default()
    };

    storage.create_job(&job).await.unwrap();

    // Verify JSONL file
    let content = tokio::fs::read_to_string(tmpdir.path().join("cron_jobs.jsonl")).await.unwrap();
    assert!(content.contains("\"type\":\"create\""));
    assert!(content.contains("\"id\":\"test\""));

    // Reload from storage
    let loaded_jobs = storage.load_jobs().await.unwrap();
    assert_eq!(loaded_jobs.len(), 1);
    assert_eq!(loaded_jobs[0].id, "test");
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_cron_service_execute_job() {
    let (message_bus, mut receiver) = MessageBus::new();
    let message_bus = Arc::new(message_bus);

    let mut cron_service = CronService::new(CronConfig::default(), message_bus.clone()).await.unwrap();

    let job = CronJob {
        id: "test".to_string(),
        schedule: "* * * * *".to_string(), // Every minute
        prompt: "test prompt".to_string(),
        enabled: true,
        next_run: Some(SystemTime::now() - Duration::from_secs(10)), // Past (should run immediately)
        ..Default::default()
    };

    cron_service.add_job(job).await.unwrap();

    // Wait for job execution (within 60 seconds)
    let msg = timeout(Duration::from_secs(65), receiver.recv()).await.unwrap().unwrap();

    assert_eq!(msg.text, "test prompt");
    assert_eq!(msg.channel, "cron");
}
```

### 5.2 Heartbeat Service Tests

```rust
// tests/heartbeat_tests.rs
#[tokio::test]
async fn test_heartbeat_service_sends_messages() {
    let (message_bus, mut receiver) = MessageBus::new();
    let message_bus = Arc::new(message_bus);

    let config = HeartbeatConfig {
        interval_minutes: 1, // 1 minute for testing
        prompt: "heartbeat".to_string(),
    };

    let service = HeartbeatService::new(config, message_bus);
    let cancel = CancellationToken::new();

    tokio::spawn(service.start(cancel.clone()));

    // Wait for first heartbeat (within 65 seconds)
    let msg = timeout(Duration::from_secs(65), receiver.recv()).await.unwrap().unwrap();

    assert_eq!(msg.text, "heartbeat");
    assert_eq!(msg.channel, "heartbeat");

    // Cancel service
    cancel.cancel();
}
```

### 5.3 MCP Client Tests

#### Unit Tests
```rust
// tests/mcp_stdio_tests.rs
#[tokio::test]
async fn test_stdio_transport_list_tools() {
    let transport = StdioTransport::new("npx", &["@modelcontextprotocol/server-example".to_string()]).await.unwrap();
    let client = McpClient::new(Box::new(transport)).await.unwrap();

    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t.name == "calculator"));
}

#[tokio::test]
async fn test_stdio_transport_call_tool() {
    let transport = StdioTransport::new("npx", &["@modelcontextprotocol/server-example".to_string()]).await.unwrap();
    let client = McpClient::new(Box::new(transport)).await.unwrap();

    let result = client.call_tool("calculator", json!({"operation": "add", "a": 2, "b": 3})).await.unwrap();
    assert_eq!(result["result"], 5);
}

// tests/mcp_http_tests.rs
#[tokio::test]
async fn test_http_transport_list_tools() {
    let mock_server = MockServer::start();
    mock_server.expect_post("/")
        .with_json_body(json!({"jsonrpc":"2.0","id":0,"method":"tools/list","params":{}}))
        .respond_with(json!({"jsonrpc":"2.0","id":0,"result":{"tools":[{"name":"test","description":"test tool","input_schema":{}}]}}));

    let transport = HttpTransport::new(mock_server.url()).await.unwrap();
    let client = McpClient::new(Box::new(transport)).await.unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "test");
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_mcp_tool_registry_integration() {
    let transport = StdioTransport::new("npx", &["@modelcontextprotocol/server-example".to_string()]).await.unwrap();
    let mcp_client = Arc::new(McpClient::new(Box::new(transport)).await.unwrap());

    let mut registry = ToolRegistry::new();
    register_mcp_tools(mcp_client, &mut registry).await.unwrap();

    // Verify tools registered
    let tool = registry.get("calculator").unwrap();
    assert_eq!(tool.name(), "calculator");

    // Execute tool
    let result = tool.execute(json!({"operation": "add", "a": 5, "b": 7})).await.unwrap();
    assert_eq!(result.output, "12");
}
```

### 5.4 Codex OAuth Tests (Manual)
```
Manual test procedure:
1. Run: weft provider login codex
2. Verify browser opens to OAuth page
3. Log in with OpenAI account
4. Verify redirect to localhost:8765/callback
5. Verify "Login successful!" message in browser
6. Verify token stored in ~/.config/weft/config.yaml
7. Test provider: weft chat --provider codex "Hello"
```

### 5.5 Test Coverage Requirements
- **Unit test coverage**: >85% for all modules
- **Integration test coverage**: >70% for end-to-end flows
- **Critical paths**: 100% coverage for cron scheduling, MCP JSON-RPC, OAuth flow

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [x] All unit tests passing (>85% coverage)
- [ ] All integration tests passing (>70% coverage)
- [x] Cron service tested with real cron jobs (daily, hourly, custom)
- [x] Heartbeat service tested with 1-minute interval
- [ ] MCP client tested with stdio transport (npx @modelcontextprotocol/server-example)
- [ ] MCP client tested with HTTP transport (mock server)
- [ ] Codex OAuth flow manually tested (browser login)
- [x] JSONL persistence tested (create, update, delete, reload)

### 6.2 Configuration Integration
- [x] CronConfig added to clawft-core Config::services
- [x] HeartbeatConfig added to clawft-core Config::services
- [x] McpConfig added to clawft-core Config::services
- [ ] CodexConfig added to clawft-core Config::providers (if not in clawft-llm)
- [ ] Environment variable loading for MCP_SERVER_COMMAND, CODEX_CLIENT_ID, etc.

### 6.3 MessageBus Integration
- [x] CronService posts InboundMessage to MessageBus
- [x] HeartbeatService posts InboundMessage to MessageBus
- [ ] Agent receives and processes cron/heartbeat messages
- [x] Metadata includes job_id, job_name, timestamp

### 6.4 Tool Registry Integration
- [x] McpTool implements clawft-core Tool trait
- [ ] MCP tools registered with ToolRegistry on startup
- [ ] Agent can execute MCP tools via Tool::execute()
- [x] Tool discovery (tools/list) runs on MCP server startup
- [x] Tool execution (tools/call) forwards params correctly

### 6.5 CLI Integration (Stream 2D)
- [x] `weft cron list` displays cron jobs table
- [x] `weft cron add` adds new cron job
- [x] `weft cron remove <job_id>` removes job
- [x] `weft cron enable/disable <job_id>` toggles enabled flag
- [x] `weft cron run <job_id>` executes job immediately
- [ ] `weft provider login codex` starts OAuth flow

### 6.6 Provider Integration (clawft-llm)
- [ ] CodexOAuthProvider added to clawft-llm providers (or clawft-services)
- [ ] SSE streaming tested with Codex endpoint
- [ ] Token refresh implemented (if needed)
- [ ] Provider registered in clawft-llm ProviderRegistry

### 6.7 Documentation
- [ ] README.md for clawft-services with setup instructions
- [ ] Cron job examples (daily summary, hourly check, custom schedules)
- [ ] MCP server examples (stdio, HTTP)
- [ ] Codex OAuth setup guide
- [ ] Migration guide from nanobot services to clawft-services

### 6.8 Performance Benchmarks
- [ ] Cron scheduler latency <1s (check jobs every minute)
- [ ] Heartbeat service latency <100ms (post message)
- [ ] MCP tool execution latency <500ms (stdio), <200ms (HTTP)
- [ ] OAuth flow completion time <30s (manual)

### 6.9 Error Handling Validation
- [x] Invalid cron expression returns error
- [ ] MCP server crash triggers reconnect
- [ ] OAuth timeout returns error (60s)
- [x] JSON-RPC errors logged and propagated
- [x] JSONL corruption recovery (skip invalid lines)

### 6.10 Optional: litellm-rs Sidecar
- [ ] SidecarTransport implemented in clawft-llm
- [ ] litellm-rs sidecar process spawned on startup
- [ ] HTTP communication tested (localhost:8080)
- [ ] Graceful shutdown (kill sidecar process)
- [ ] Vertex AI provider tested via sidecar (if needed)

### 6.11 Final Review
- [ ] Code review by at least 2 reviewers
- [ ] Security audit of OAuth token storage
- [ ] Performance profiling (flamegraph)
- [ ] Memory leak testing (valgrind/miri)
- [ ] Changelog updated with new services
- [ ] Version bumped (clawft-services 0.1.0)

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockFileSystem, MockEnvironment, MockHttpClient};`
- **Import mocks from 1B**: `use clawft_core::test_utils::MockMessageBus;`
- **Use shared fixtures**: Load `tests/fixtures/config.json` for cron schedule configs

### MessageBus Integration Tests (Required)
```rust
#[tokio::test]
async fn test_cron_job_publishes_to_message_bus() {
    let bus = Arc::new(MockMessageBus::new());
    let cron = CronService::new(bus.clone(), config);
    // Trigger a scheduled job
    cron.execute_job("test-job").await.unwrap();
    // Verify message published
    let messages = bus.consumed_inbound();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("cron"));
}
```

### Security Tests (Required)
- Cron expression injection prevention (validate before execution)
- OAuth token storage permissions (0600 file mode)
- MCP JSON-RPC input validation (malformed requests)

### Coverage Target
- Unit test coverage: >= 80% (measured via `cargo-tarpaulin`)
- Critical paths (cron execution, MCP transport, OAuth flow): 100%

---

## Notes for Implementation Agent

1. **Read Python source files first** to understand exact service implementations
2. **Use TDD London School**: Mock MessageBus/Transport, write failing tests, then implement
3. **Parallel file operations**: Create all test files + implementation files in single message
4. **Cron expression validation**: Use `cron` crate's error handling
5. **JSONL format**: Append-only, one JSON object per line, replay on load
6. **MCP JSON-RPC**: Strict adherence to 2.0 spec (jsonrpc, id, method, params)
7. **stdio transport**: Newline-delimited JSON, handle process crashes
8. **OAuth security**: Store token in config file (0600 permissions)
9. **Codex SSE streaming**: Parse `data:` lines, handle `[DONE]` marker
10. **litellm-rs decision**: Recommend skipping for Phase 2, add in Phase 3 if exotic providers needed
11. **Error types**: Define CronError, McpError, OAuthError with From<std::io::Error> impls
12. **Logging**: Use tracing for debug logs, structured fields for job execution, tool calls
13. **Graceful shutdown**: CancellationToken for all background tasks
14. **Tool registration**: Dynamic tool discovery on MCP server startup
15. **Heartbeat interval**: Configurable via config file (default: 30 minutes)
