# Phase A: Axum REST/WebSocket API Layer

> **Element:** UI -- Backend API for Web Dashboard
> **Phase:** A (A1, A2, A3, A4)
> **Priority:** P0 (Critical Path -- UI is unusable without real backend)
> **Crates:** `crates/clawft-services/src/api/` (primary), `crates/clawft-cli/` (gateway + new `weft ui`), `crates/clawft-core/` (Arc-wrapping), `crates/clawft-types/` (response types)
> **Dependencies IN:** Existing `AppContext<P>`, `ToolRegistry`, `SessionManager<P>`, `MemoryStore<P>`, `SkillsLoader<P>`, `MessageBus`, `AgentRegistry`, `GatewayConfig` (already has `api_port`, `cors_origins`, `api_enabled`)
> **Blocks:** All UI views that call `api-client.ts` (Dashboard, WebChat, Agent Management, Session Explorer, Tool Browser, Skill Browser, Memory Explorer, Config Editor, Cron Dashboard, Delegation Monitor, Monitoring)
> **Status:** Planning
> **Orchestrator Ref:** `ui/00-orchestrator.md` Section 2, `ui/01-phase-S1-foundation-core-views.md` Section 3.1

---

## 1. Overview

The React UI (`ui/`) currently runs entirely on MSW mock data. The Axum REST + WebSocket API skeleton exists in `crates/clawft-services/src/api/` with 6 files (mod.rs, handlers.rs, auth.rs, ws.rs, delegation.rs, monitoring.rs), but all handlers return hardcoded mock data and nothing is wired to real core services. The gateway command (`weft gateway`) has no HTTP listener.

This plan bridges the gap: implement trait-object wrappers that connect the API layer to real `AppContext` services, add the missing endpoint handlers (skills, memory, config, cron, channels, chat/streaming, voice), wire the Axum listener into the gateway lifecycle, implement real WebSocket topic routing, and add the `weft ui` convenience command.

**Key constraint:** `AppContext::into_agent_loop()` consumes the context. All shared service references must be extracted and Arc-wrapped **before** that call. The existing `ApiState` trait-object design (`ToolRegistryAccess`, `SessionAccess`, `AgentAccess`, `BusAccess`) already accounts for this -- we need to implement the wrappers and add new traits for the remaining domains.

### What Exists

| Component | Location | Status |
|-----------|----------|--------|
| `ApiState` + 4 trait definitions | `api/mod.rs` | Skeleton, needs new traits |
| `api_routes()` + 12 handlers | `api/handlers.rs` | Returns mock data, needs real wiring |
| `TokenStore` (in-memory) | `api/auth.rs` | Functional, needs middleware extraction |
| WebSocket upgrade handler | `api/ws.rs` | Stub, needs topic routing + broadcasting |
| Delegation routes + types | `api/delegation.rs` | Mock data, needs DelegationManager |
| Monitoring routes + types | `api/monitoring.rs` | Mock data, needs MetricsCollector |
| `GatewayConfig.api_port` | `config/mod.rs:291` | Ready (default 18789) |
| `GatewayConfig.cors_origins` | `config/mod.rs:295` | Ready (default `["http://localhost:5173"]`) |
| `GatewayConfig.api_enabled` | `config/mod.rs:300` | Ready (default false) |
| `build_router()` | `api/mod.rs:98` | Functional, needs static serving + new routes |
| `api` feature flag | `clawft-services/Cargo.toml:19` | Ready, not wired from CLI |
| API client spec (43 endpoints) | `ui/src/lib/api-client.ts` | Complete TypeScript contract |

### What's Missing

| Gap | Fix |
|-----|-----|
| No `api` feature passed from CLI to services | Add `api` feature to `clawft-cli/Cargo.toml` |
| `ToolRegistry`, `SessionManager` not Arc-wrapped in AppContext | Refactor to `Arc<ToolRegistry>`, `Arc<SessionManager<P>>` |
| No trait wrappers implementing `ToolRegistryAccess` etc. | Implement bridge structs in `api/bridge.rs` |
| Missing traits for skills, memory, config, cron, channels, voice | Add to `api/mod.rs` |
| Missing route handlers (22 of 43 endpoints) | Add in `api/handlers.rs` + new modules |
| No HTTP listener in `gateway.rs` | Add conditional Axum server |
| No `weft ui` command | New CLI command or `--api` flag on gateway |
| No auth middleware (Bearer token validation) | Tower middleware layer |
| No static file serving for `ui/dist/` | `tower-http::services::ServeDir` |
| No WebSocket topic routing / broadcasting | `tokio::sync::broadcast` channels |
| No chat streaming (SSE or WS) | Server-Sent Events for agent responses |

---

## 2. Current Code

### 2.1 AppContext Accessor Methods

```rust
// crates/clawft-core/src/bootstrap.rs (current)
pub fn bus(&self) -> &Arc<MessageBus>           // ✅ Arc
pub fn tools(&self) -> &ToolRegistry            // ❌ not Arc
pub fn tools_mut(&mut self) -> &mut ToolRegistry // ❌ not Arc
pub fn memory(&self) -> &Arc<MemoryStore<P>>    // ✅ Arc
pub fn skills(&self) -> &Arc<SkillsLoader<P>>   // ✅ Arc
// sessions: SessionManager<P>                  // ❌ not Arc, no pub getter
```

### 2.2 into_agent_loop Consumption

```rust
// crates/clawft-core/src/bootstrap.rs:165
pub fn into_agent_loop(self) -> AgentLoop<P> {
    AgentLoop::new(
        self.config.agents,
        self.platform,
        self.bus,       // moved
        self.pipeline,
        self.tools,     // moved
        self.context,
        self.sessions,  // moved
        resolver,
    );
}
```

### 2.3 GatewayConfig (already extended)

```rust
// crates/clawft-types/src/config/mod.rs:273
pub struct GatewayConfig {
    pub host: String,                          // "0.0.0.0"
    pub port: u16,                             // 18790
    pub heartbeat_interval_minutes: u64,
    pub heartbeat_prompt: String,
    pub api_port: u16,                         // 18789
    pub cors_origins: Vec<String>,             // ["http://localhost:5173"]
    pub api_enabled: bool,                     // false
}
```

### 2.4 Feature Flags

```toml
# clawft-services/Cargo.toml
api = ["dep:axum", "dep:axum-extra", "dep:tower-http", "dep:futures-util"]

# clawft-cli/Cargo.toml (MISSING `api` passthrough)
default = ["channels", "services", "delegate"]
services = ["dep:clawft-services"]   # does NOT pass `api` feature
```

---

## 3. Architecture

### 3.1 Service Sharing via Arc

Before `into_agent_loop()` consumes the context, we extract Arc-wrapped references:

```text
┌─────────────────────────────────────────┐
│               AppContext<P>             │
│                                         │
│  bus: Arc<MessageBus>         ──┐       │
│  tools: Arc<ToolRegistry>     ──┤       │  clone Arc
│  sessions: Arc<SessionMgr<P>> ──┤───────┼──────────> ApiState
│  memory: Arc<MemoryStore<P>>  ──┤       │
│  skills: Arc<SkillsLoader<P>> ──┘       │
│  config: Config               ──────────┼──────────> ApiState
│  pipeline: PipelineRegistry             │
│  context: ContextBuilder<P>             │
│                                         │
│  into_agent_loop() ─────────────────────┼──────────> AgentLoop<P>
└─────────────────────────────────────────┘
```

Both `AgentLoop` and `ApiState` hold `Arc` refs to the same underlying data. Reads are concurrent-safe (tools and sessions use internal locking). Writes to sessions from the API (create/delete) are visible to the agent loop.

### 3.2 Trait Object Bridge

```text
                 ApiState
                    │
     ┌──────────────┼──────────────┐
     │              │              │
     ▼              ▼              ▼
Arc<dyn          Arc<dyn       Arc<dyn
ToolRegistry     Session       Agent
Access>          Access>       Access>
     │              │              │
     ▼              ▼              ▼
ToolBridge      SessionBridge  AgentBridge
     │              │              │
     ▼              ▼              ▼
Arc<ToolRegistry> Arc<SessionMgr> AgentRegistry
                                  (snapshot)
```

Bridge structs in `api/bridge.rs` implement the traits using concrete references. The Platform generic `P` is erased at this boundary.

### 3.3 Gateway Lifecycle (with API)

```text
1. Load config & bootstrap AppContext
2. Register core tools
3. Extract Arc refs for API (bus, tools, sessions, memory, skills)
4. Build ApiState with bridge wrappers
5. Build Axum Router (REST + WS + static)
6. Start Axum listener on api_port (tokio task)
7. Start channels + agent loop (existing)
8. Wait for Ctrl+C → cancel all
```

### 3.4 Endpoint Coverage Matrix

| Domain | Endpoints | Existing Routes | Existing Handlers | Need |
|--------|-----------|-----------------|-------------------|------|
| Agents | 4 | 2 (list, get) | 2 (mock → real) | +2 (start, stop) |
| Sessions | 5 | 3 (list, get, delete) | 3 (mock → real) | +2 (create, export) |
| Chat | 1 | 0 | 0 | +1 (send message) |
| Tools | 2 | 2 (list, schema) | 2 (mock → real) | 0 |
| Health | 1 | 1 | 1 (partially real) | 0 |
| Auth | 1 | 1 | 1 (functional) | 0 |
| Skills | 4 | 0 | 0 | +4 (list, install, uninstall, search) |
| Memory | 4 | 0 | 0 | +4 (list, search, create, delete) |
| Config | 2 | 0 | 0 | +2 (get, save) |
| Cron | 5 | 0 | 0 | +5 (list, create, update, delete, run) |
| Channels | 1 | 0 | 0 | +1 (list status) |
| Delegation | 5 | 5 | 5 (mock → real) | 0 (wire later) |
| Monitoring | 3 | 3 | 3 (mock → real) | 0 (wire later) |
| Voice | 4 | 0 | 0 | +4 (status, settings, test-mic, test-speaker) |
| **Total** | **43** | **17** | **17** | **+26 new** |

---

## 4. Deliverables

### 4.1 Phase A1: Core Infrastructure (Agent 1 -- Backend Architect)

Wire the feature flags, Arc-wrap shared services, implement bridge structs, add the HTTP listener to the gateway, and wire existing handlers to real data.

#### 4.1.1 Feature Flag Wiring

**File:** `crates/clawft-cli/Cargo.toml`

Add `api` feature that passes through to `clawft-services`:

```toml
[features]
default = ["channels", "services", "delegate", "api"]
channels = ["dep:clawft-channels"]
services = ["dep:clawft-services"]
api = ["clawft-services/api"]
# ...
```

#### 4.1.2 Arc-Wrap Shared Services in AppContext

**File:** `crates/clawft-core/src/bootstrap.rs`

Change `tools` field from `ToolRegistry` to `Arc<ToolRegistry>` and `sessions` from `SessionManager<P>` to `Arc<SessionManager<P>>`. Update `into_agent_loop()` to pass `Arc::clone()` instead of moving. Add public getter for sessions.

Changes:
- `tools: ToolRegistry` → `tools: Arc<ToolRegistry>`
- `sessions: SessionManager<P>` → `Arc<SessionManager<P>>`
- `tools_mut()` → uses `Arc::get_mut()` (only works before sharing)
- Add `pub fn sessions(&self) -> &Arc<SessionManager<P>>`
- `into_agent_loop()` clones Arc refs instead of moving

**File:** `crates/clawft-core/src/agent/loop_core.rs`

Update `AgentLoop` fields to accept `Arc<ToolRegistry>` and `Arc<SessionManager<P>>`.

#### 4.1.3 Bridge Implementations

**File:** `crates/clawft-services/src/api/bridge.rs` (new)

Implement bridge structs that wrap Arc-ed core services and implement the `ApiState` traits:

```rust
pub struct ToolBridge(pub Arc<ToolRegistry>);

impl ToolRegistryAccess for ToolBridge {
    fn list_tools(&self) -> Vec<ToolInfo> {
        self.0.list().into_iter().map(|name| {
            let desc = self.0.get(&name)
                .map(|t| t.description().to_string())
                .unwrap_or_default();
            ToolInfo { name, description: desc }
        }).collect()
    }
    fn tool_schema(&self, name: &str) -> Option<serde_json::Value> {
        self.0.get(name).map(|t| t.parameters())
    }
}

pub struct SessionBridge<P: Platform>(pub Arc<SessionManager<P>>);
// impl SessionAccess for SessionBridge<P>

pub struct AgentBridge { agents: Vec<AgentInfo> }
// impl AgentAccess for AgentBridge (snapshot from AgentRegistry discovery)

pub struct BusBridge(pub Arc<MessageBus>);
// impl BusAccess for BusBridge
```

The `SessionBridge` requires async methods but the traits are sync. Two options:
- **Option A:** Make traits async (change return types to use `tokio::runtime::Handle::block_on`)
- **Option B:** Cache session list on a timer and serve from cache

**Recommended: Option A** -- Use `tokio::task::block_in_place` + `Handle::block_on` inside the sync trait methods for the initial implementation, then migrate to async traits in A3.

#### 4.1.4 Extend ApiState

**File:** `crates/clawft-services/src/api/mod.rs`

Add new trait-object fields and traits for the remaining domains:

```rust
pub struct ApiState {
    // Existing
    pub tools: Arc<dyn ToolRegistryAccess>,
    pub sessions: Arc<dyn SessionAccess>,
    pub agents: Arc<dyn AgentAccess>,
    pub bus: Arc<dyn BusAccess>,
    pub auth: Arc<auth::TokenStore>,
    // New
    pub skills: Arc<dyn SkillAccess>,
    pub memory: Arc<dyn MemoryAccess>,
    pub config: Arc<dyn ConfigAccess>,
    pub cron: Arc<dyn CronAccess>,
    pub channels: Arc<dyn ChannelAccess>,
}

pub trait SkillAccess: Send + Sync {
    fn list_skills(&self) -> Vec<SkillInfo>;
    fn get_skill(&self, name: &str) -> Option<SkillDetail>;
    fn install_skill(&self, id: &str) -> Result<(), String>;
    fn uninstall_skill(&self, name: &str) -> Result<(), String>;
}

pub trait MemoryAccess: Send + Sync {
    fn list_entries(&self) -> Vec<MemoryEntryInfo>;
    fn search(&self, query: &str, threshold: f64) -> Vec<MemoryEntryInfo>;
    fn store(&self, key: &str, value: &str, namespace: &str, tags: &[String]) -> MemoryEntryInfo;
    fn delete(&self, key: &str) -> bool;
}

pub trait ConfigAccess: Send + Sync {
    fn get_config(&self) -> serde_json::Value;
    fn save_config(&self, config: serde_json::Value) -> Result<(), String>;
}

pub trait CronAccess: Send + Sync {
    fn list_jobs(&self) -> Vec<CronJobInfo>;
    fn create_job(&self, job: CronJobCreate) -> Result<CronJobInfo, String>;
    fn update_job(&self, id: &str, job: CronJobUpdate) -> Result<CronJobInfo, String>;
    fn delete_job(&self, id: &str) -> Result<(), String>;
    fn run_now(&self, id: &str) -> Result<(), String>;
}

pub trait ChannelAccess: Send + Sync {
    fn list_channels(&self) -> Vec<ChannelStatusInfo>;
}
```

#### 4.1.5 Gateway HTTP Listener

**File:** `crates/clawft-cli/src/commands/gateway.rs`

Add conditional Axum server startup when `config.gateway.api_enabled`:

```rust
// After register_core_tools, before into_agent_loop:
#[cfg(feature = "api")]
let api_handle = if config.gateway.api_enabled {
    use clawft_services::api;

    // Build bridge wrappers
    let api_state = build_api_state(&ctx, &config);

    // Build router
    let router = api::build_router(api_state, &config.gateway.cors_origins);

    // Bind and serve
    let addr = format!("{}:{}", config.gateway.host, config.gateway.api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr = %addr, "REST/WS API listening");

    let cancel_api = cancel.clone();
    Some(tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(cancel_api.cancelled_owned())
            .await
            .ok();
    }))
} else {
    None
};
```

Add `build_api_state()` helper that extracts Arc refs from `AppContext` and creates bridge wrappers.

#### 4.1.6 Wire Existing Handlers to Real Data

**File:** `crates/clawft-services/src/api/handlers.rs`

The existing handlers already call `state.tools.list_tools()`, `state.sessions.list_sessions()`, etc. Once bridge wrappers are connected, these handlers automatically return real data. No handler code changes needed for the 12 existing handlers -- only the bridge implementations need to work correctly.

**Exit Criteria A1:**
- [ ] `cargo check -p clawft-cli --features api` compiles
- [ ] `cargo test -p clawft-cli` passes
- [ ] `cargo test -p clawft-services --features api` passes
- [ ] `weft gateway --config test.json` with `api_enabled: true` starts HTTP listener on port 18789
- [ ] `curl http://localhost:18789/api/health` returns `{"status":"ok","version":"0.1.0",...}`
- [ ] `curl http://localhost:18789/api/tools` returns real tool registry data
- [ ] `curl http://localhost:18789/api/sessions` returns real session list
- [ ] `curl http://localhost:18789/api/agents` returns real agent definitions
- [ ] CORS headers present on responses for configured origins
- [ ] Existing `weft gateway` (without `api_enabled`) still works unchanged

---

### 4.2 Phase A2: Missing Endpoint Handlers (Agent 2 -- API Developer)

Implement the 26 missing endpoint handlers for skills, memory, config, cron, channels, chat, and voice.

#### 4.2.1 Skills Endpoints

**File:** `crates/clawft-services/src/api/skills.rs` (new)

```rust
pub fn skills_routes() -> Router<ApiState> {
    Router::new()
        .route("/skills", get(list_skills))
        .route("/skills/install", post(install_skill))
        .route("/skills/{name}", delete(uninstall_skill))
        .route("/skills/registry/search", get(search_registry))
}
```

| Endpoint | Core Service | Method |
|----------|-------------|--------|
| `GET /api/skills` | `SkillsLoader::list_skills()` + `load_skill()` | List all local skills |
| `POST /api/skills/install` | `SkillsLoader::install()` (if exists) or filesystem write | Install from ClawHub |
| `DELETE /api/skills/{name}` | `SkillsLoader` + filesystem delete | Remove skill directory |
| `GET /api/skills/registry/search?q=` | HTTP fetch to ClawHub API | Search remote registry |

#### 4.2.2 Memory Endpoints

**File:** `crates/clawft-services/src/api/memory.rs` (new)

```rust
pub fn memory_routes() -> Router<ApiState> {
    Router::new()
        .route("/memory", get(list_memory))
        .route("/memory", post(create_memory))
        .route("/memory/search", get(search_memory))    // GET with query params
        .route("/memory/{key}", delete(delete_memory))
}
```

| Endpoint | Core Service | Method |
|----------|-------------|--------|
| `GET /api/memory` | `MemoryStore::search("", 100)` | List all entries |
| `GET /api/memory/search?q=&threshold=` | `MemoryStore::search(query, limit)` | Semantic search |
| `POST /api/memory` | `MemoryStore` write | Create entry |
| `DELETE /api/memory/{key}` | `MemoryStore` delete | Delete entry |

**Note:** The UI's `api-client.ts` uses `POST /api/memory/search` with query params but `MemoryStore::search()` is a simple substring search. For now, wire the substring search; semantic search can be added when vector-memory feature is enabled.

#### 4.2.3 Config Endpoints

**File:** `crates/clawft-services/src/api/config_api.rs` (new, avoid conflict with `config` module)

```rust
pub fn config_routes() -> Router<ApiState> {
    Router::new()
        .route("/config", get(get_config))
        .route("/config", put(save_config))
}
```

| Endpoint | Implementation |
|----------|---------------|
| `GET /api/config` | Return sanitized config (strip API keys, secrets) |
| `PUT /api/config` | Validate, merge with existing, write to config file |

**Security:** The `GET /api/config` handler MUST strip all `SecretString` fields (API keys) before returning. Use `serde_json::to_value()` which already serializes `SecretString` as `""`.

#### 4.2.4 Cron Endpoints

**File:** `crates/clawft-services/src/api/cron_api.rs` (new)

```rust
pub fn cron_routes() -> Router<ApiState> {
    Router::new()
        .route("/cron", get(list_cron_jobs))
        .route("/cron", post(create_cron_job))
        .route("/cron/{id}", put(update_cron_job))
        .route("/cron/{id}", delete(delete_cron_job))
        .route("/cron/{id}/run", post(run_cron_job))
}
```

| Endpoint | Core Service |
|----------|-------------|
| `GET /api/cron` | `CronService` internal job list |
| `POST /api/cron` | `CronService::add_job()` |
| `PUT /api/cron/{id}` | `CronService::update_job()` |
| `DELETE /api/cron/{id}` | `CronService::remove_job()` |
| `POST /api/cron/{id}/run` | `CronService::trigger()` |

**Note:** `CronService` is currently created inside `gateway.rs` and not shared. Need to Arc-wrap it and add to `ApiState`, or add a `CronAccess` bridge that holds `Arc<CronService>`.

#### 4.2.5 Channel Status Endpoint

**File:** `crates/clawft-services/src/api/channels_api.rs` (new)

```rust
pub fn channel_routes() -> Router<ApiState> {
    Router::new()
        .route("/channels", get(list_channels))
}
```

Returns status of configured channels (Telegram, Slack, Discord) -- name, enabled flag, connected status. Bridge to `PluginHost::list_channels()` or build from config.

#### 4.2.6 Chat Endpoint (Send Message)

**File:** `crates/clawft-services/src/api/chat.rs` (new)

```rust
pub fn chat_routes() -> Router<ApiState> {
    Router::new()
        .route("/sessions/{key}/messages", post(send_message))
        .route("/sessions", post(create_session))
        .route("/sessions/{key}/export", get(export_session))
}
```

| Endpoint | Implementation |
|----------|---------------|
| `POST /api/sessions/{key}/messages` | Inject into MessageBus as `InboundMessage` with `channel: "web"` |
| `POST /api/sessions` | Create session via `SessionManager::get_or_create()` |
| `GET /api/sessions/{key}/export` | Load full session and return messages array |

The chat `send_message` handler is critical. It:
1. Receives `{ content: string }` from the UI
2. Creates an `InboundMessage` with `channel: "web"`, `chat_id: session_key`
3. Sends it to the `MessageBus` inbound channel
4. The agent loop picks it up, processes it, and produces an `OutboundMessage`
5. The outbound message is routed back via WebSocket (A3) or returned synchronously

**For A2:** Return synchronously by subscribing to the outbound channel and waiting for the matching response with a timeout. Full streaming via SSE/WS deferred to A3.

#### 4.2.7 Agent Start/Stop Endpoints

**File:** `crates/clawft-services/src/api/handlers.rs` (extend existing)

Add handlers for agent lifecycle:

```rust
.route("/agents/{name}/start", post(start_agent))
.route("/agents/{name}/stop", post(stop_agent))
```

These send control messages to the agent system. For initial implementation, these can toggle an agent's active state in the registry.

#### 4.2.8 Voice Endpoints (Stub)

**File:** `crates/clawft-services/src/api/voice_api.rs` (new)

```rust
pub fn voice_routes() -> Router<ApiState> {
    Router::new()
        .route("/voice/status", get(voice_status))
        .route("/voice/settings", put(update_voice_settings))
        .route("/voice/test-mic", post(test_mic))
        .route("/voice/test-speaker", post(test_speaker))
}
```

Initially returns sensible defaults. Real voice integration depends on the voice workstream (V-VOICE phases).

#### 4.2.9 Register All New Routes

**File:** `crates/clawft-services/src/api/handlers.rs`

Update `api_routes()` to merge all new route modules:

```rust
pub fn api_routes() -> Router<ApiState> {
    Router::new()
        // Existing routes...
        .merge(super::skills::skills_routes())
        .merge(super::memory::memory_routes())
        .merge(super::config_api::config_routes())
        .merge(super::cron_api::cron_routes())
        .merge(super::channels_api::channel_routes())
        .merge(super::chat::chat_routes())
        .merge(super::voice_api::voice_routes())
        .merge(super::delegation::delegation_routes())
        .merge(super::monitoring::monitoring_routes())
}
```

**Exit Criteria A2:**
- [ ] All 43 endpoints defined in `api-client.ts` have corresponding Axum routes
- [ ] `GET /api/skills` returns real skills from `SkillsLoader`
- [ ] `GET /api/memory` returns real memory entries from `MemoryStore`
- [ ] `GET /api/config` returns sanitized config (no API keys exposed)
- [ ] `PUT /api/config` writes config changes to disk
- [ ] `GET /api/cron` returns real cron jobs
- [ ] `POST /api/sessions/{key}/messages` sends message through pipeline and returns response
- [ ] `GET /api/channels` returns channel status from config
- [ ] `cargo test -p clawft-services --features api` -- all new handler tests pass
- [ ] `npm run dev` (UI) connects to real backend and displays real data

---

### 4.3 Phase A3: WebSocket + Streaming (Agent 3 -- Real-Time Specialist)

Implement real WebSocket topic routing for live updates and Server-Sent Events for chat streaming.

#### 4.3.1 Topic-Based Broadcasting

**File:** `crates/clawft-services/src/api/ws.rs` (rewrite)

Replace the stub WebSocket handler with a topic-based pub/sub system:

```rust
use tokio::sync::broadcast;

pub struct TopicBroadcaster {
    topics: HashMap<String, broadcast::Sender<String>>,
}

impl TopicBroadcaster {
    pub fn new() -> Self { /* create channels for known topics */ }
    pub fn publish(&self, topic: &str, msg: serde_json::Value) { /* send to channel */ }
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<String> { /* get rx */ }
}
```

**Topics:**
- `agents` -- agent status changes (started, stopped, error)
- `sessions` -- new session, message appended
- `sessions:{key}` -- messages for a specific session (chat streaming)
- `tools` -- tool invocation events
- `pipeline` -- pipeline run events (for monitoring)
- `delegation` -- delegation status changes
- `cron` -- cron job execution events

#### 4.3.2 Chat Response Streaming

Two approaches (implement both, UI chooses):

**SSE (Server-Sent Events):**
```rust
// GET /api/sessions/{key}/stream
async fn stream_session(
    State(state): State<ApiState>,
    Path(key): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe to session topic, yield events as SSE
}
```

**WebSocket:**
The existing `/ws` endpoint, when client sends `{"type":"subscribe","topic":"sessions:sess-123"}`, starts forwarding agent output for that session.

#### 4.3.3 Agent Loop Integration

**File:** `crates/clawft-cli/src/commands/gateway.rs`

Wire the outbound dispatch loop to also publish to the `TopicBroadcaster`:

```rust
// In dispatch loop, after sending to channel:
if let Some(broadcaster) = &broadcaster {
    broadcaster.publish(
        &format!("sessions:{}", outbound.chat_id),
        serde_json::json!({
            "type": "message",
            "content": outbound.content,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );
}
```

#### 4.3.4 Async Trait Migration

Migrate `SessionAccess`, `MemoryAccess`, etc. from sync to async traits using `async_trait`. This removes the need for `block_in_place` hacks from A1. The handlers are already async, so async traits are natural.

**Exit Criteria A3:**
- [ ] WebSocket connection receives real-time agent status updates
- [ ] WebSocket subscribe to `sessions:{key}` receives live chat messages
- [ ] SSE endpoint `/api/sessions/{key}/stream` streams agent responses
- [ ] `POST /api/sessions/{key}/messages` initiates processing; response streams via WS/SSE
- [ ] Multiple concurrent WebSocket connections work correctly
- [ ] WebSocket heartbeat/ping keeps connections alive
- [ ] Topic unsubscribe works correctly
- [ ] Connection cleanup on disconnect (no leaked subscriptions)

---

### 4.4 Phase A4: CLI, Auth, Static Serving, Polish (Agent 4 -- Integration Engineer)

#### 4.4.1 `weft ui` CLI Command

**File:** `crates/clawft-cli/src/commands/ui_cmd.rs` (new)

Convenience command that starts the gateway with `api_enabled: true` and opens the browser:

```rust
#[derive(Args)]
pub struct UiArgs {
    /// Config file path override.
    #[arg(short, long)]
    pub config: Option<String>,
    /// Port for the UI API (overrides config).
    #[arg(short, long)]
    pub port: Option<u16>,
    /// Don't open the browser automatically.
    #[arg(long)]
    pub no_open: bool,
    /// Serve the built UI from this directory.
    #[arg(long)]
    pub ui_dir: Option<String>,
}
```

Implementation:
1. Load config, force `config.gateway.api_enabled = true`
2. Override port if `--port` given
3. Delegate to `gateway::run()` (or shared inner function)
4. After listener is bound, open browser to `http://localhost:{port}`

**File:** `crates/clawft-cli/src/main.rs`

Register `Ui(commands::ui_cmd::UiArgs)` in the `Commands` enum.

#### 4.4.2 Auth Middleware

**File:** `crates/clawft-services/src/api/auth.rs` (extend)

Add Tower middleware that validates Bearer tokens on all `/api/*` routes except `/api/auth/token` and `/api/health`:

```rust
pub fn auth_layer(store: Arc<TokenStore>) -> impl Layer<...> {
    // Extract Authorization header
    // Validate token against store
    // Return 401 if invalid
    // Pass through for exempt routes
}
```

#### 4.4.3 Static File Serving

**File:** `crates/clawft-services/src/api/mod.rs` (extend `build_router`)

Serve the built UI from `ui/dist/` (or a configurable directory) as a fallback for non-API routes:

```rust
use tower_http::services::ServeDir;

let router = Router::new()
    .nest("/api", handlers::api_routes())
    .route("/ws", get(ws::ws_handler))
    .fallback_service(ServeDir::new(ui_dist_dir).append_index_html_on_directories(true))
    .layer(cors)
    .layer(TraceLayer::new_for_http())
    .with_state(state);
```

This enables single-binary deployment: `weft ui` serves both the API and the React SPA.

#### 4.4.4 Update Help Text

**File:** `crates/clawft-cli/src/help_text.rs`

Add `ui` topic describing `weft ui` command and API configuration.

#### 4.4.5 Remove MSW in Production

The UI already handles this (`main.tsx` only starts MSW when `import.meta.env.DEV`), but verify the production build (`npm run build`) excludes MSW code from the bundle.

**Exit Criteria A4:**
- [ ] `weft ui` starts gateway + API + opens browser
- [ ] `weft ui --port 9000` overrides port
- [ ] `weft ui --no-open` skips browser open
- [ ] Auth middleware rejects requests without valid Bearer token (except health + auth endpoints)
- [ ] `weft ui --ui-dir ./ui/dist` serves the React SPA
- [ ] SPA routes (e.g., `/agents`, `/sessions`) serve `index.html` (SPA fallback)
- [ ] `weft help ui` shows usage documentation
- [ ] `npm run build` produces bundle without MSW code
- [ ] `cargo check -p clawft-cli` compiles with all features
- [ ] Full end-to-end: `npm run build` → `weft ui --ui-dir ui/dist` → browser shows real data

---

## 5. Agent Assignment

| Phase | Agent | Type | Parallelism | Dependencies |
|-------|-------|------|-------------|-------------|
| A1 | Backend Architect | `coder` | -- | None (foundation) |
| A2 | API Developer | `coder` | Parallel with A1 completion | A1 (bridge structs, ApiState traits) |
| A3 | Real-Time Specialist | `coder` | After A2 | A2 (handlers must exist) |
| A4 | Integration Engineer | `coder` | Parallel with A3 | A1 (gateway listener), A2 (routes exist) |

**Recommended execution:**
1. A1 first (blocking -- everything depends on Arc-wrapping + gateway listener)
2. A2 + A4 in parallel after A1 (A2 adds handlers, A4 adds CLI/auth/static)
3. A3 after A2 (streaming depends on handler infrastructure)

```text
A1 ──────────> A2 ──────────> A3
                   \
                    A4 ────────>
```

---

## 6. Files Created

| File | Purpose |
|------|---------|
| `crates/clawft-services/src/api/bridge.rs` | Trait-object wrappers for AppContext → ApiState |
| `crates/clawft-services/src/api/skills.rs` | Skills CRUD endpoints |
| `crates/clawft-services/src/api/memory.rs` | Memory CRUD + search endpoints |
| `crates/clawft-services/src/api/config_api.rs` | Config get/save endpoints |
| `crates/clawft-services/src/api/cron_api.rs` | Cron job CRUD + trigger endpoints |
| `crates/clawft-services/src/api/channels_api.rs` | Channel status endpoint |
| `crates/clawft-services/src/api/chat.rs` | Chat send + session create/export |
| `crates/clawft-services/src/api/voice_api.rs` | Voice status + settings stubs |
| `crates/clawft-cli/src/commands/ui_cmd.rs` | `weft ui` CLI command |

## 7. Files Modified

| File | Change |
|------|--------|
| `crates/clawft-cli/Cargo.toml` | Add `api` feature flag |
| `crates/clawft-core/src/bootstrap.rs` | Arc-wrap `tools`, `sessions`; add `sessions()` getter |
| `crates/clawft-core/src/agent/loop_core.rs` | Accept `Arc<ToolRegistry>`, `Arc<SessionManager<P>>` |
| `crates/clawft-cli/src/commands/gateway.rs` | Add Axum listener, `build_api_state()`, broadcaster |
| `crates/clawft-services/src/api/mod.rs` | New traits, extend `ApiState`, optional static serving |
| `crates/clawft-services/src/api/handlers.rs` | Merge new route modules, add agent start/stop |
| `crates/clawft-services/src/api/auth.rs` | Add Tower auth middleware |
| `crates/clawft-services/src/api/ws.rs` | Replace stub with topic-based pub/sub |
| `crates/clawft-services/src/api/delegation.rs` | Wire to real DelegationManager (when available) |
| `crates/clawft-services/src/api/monitoring.rs` | Wire to real MetricsCollector (when available) |
| `crates/clawft-cli/src/commands/mod.rs` | Add `pub mod ui_cmd;` |
| `crates/clawft-cli/src/main.rs` | Add `Ui` command variant + dispatch |
| `crates/clawft-cli/src/help_text.rs` | Add `ui` help topic |

---

## 8. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Arc-wrapping `ToolRegistry` breaks `tools_mut()` callers | Medium | High | `Arc::get_mut()` works before cloning; `Arc::make_mut()` for post-clone writes; audit all callers |
| Async-in-sync bridge (A1) causes deadlocks | Medium | High | Use `tokio::task::block_in_place` carefully; migrate to async traits in A3 |
| `into_agent_loop()` refactor breaks existing CLI commands | Low | High | Keep backward-compatible: Arc wrapping is invisible to callers that don't share |
| Session file locking between agent loop and API | Low | Medium | SessionManager already uses `Arc<Mutex<HashMap>>` for cache; JSONL appends are atomic |
| WebSocket connections leak memory | Medium | Medium | Connection tracking with cleanup on disconnect; idle timeout |
| CORS misconfiguration blocks UI in production | Low | Medium | Default `["http://localhost:5173"]` works for dev; configurable for prod |
| Config save endpoint writes invalid config | Medium | High | Validate against Config schema before writing; keep backup of previous config |
| `CronService` not shareable (created inside gateway.rs) | Medium | Medium | Arc-wrap CronService; pass Arc into both gateway loop and API bridge |

---

## 9. Security Requirements

- [ ] Auth middleware validates Bearer tokens on all `/api/*` routes (except `/api/auth/token`, `/api/health`)
- [ ] `GET /api/config` strips all `SecretString` fields (API keys never exposed via REST)
- [ ] `PUT /api/config` validates input against Config schema before writing
- [ ] Session keys are validated against path traversal (`validate_session_id()`)
- [ ] Memory keys are validated and sanitized
- [ ] WebSocket connections are auth-validated on upgrade (token in query param or first message)
- [ ] CORS is restricted to configured origins (not `*` in production)
- [ ] Rate limiting on `/api/auth/token` (5 req/min) and mutation endpoints (60 req/min)
- [ ] No stack traces or internal errors exposed in HTTP responses
- [ ] CSP headers set on HTML responses from static serving

---

## 10. Testing Strategy

### Unit Tests
- Bridge struct implementations (mock core services, verify trait methods)
- Auth middleware (valid token, invalid token, expired token, missing token)
- Each handler in isolation (mock ApiState traits)

### Integration Tests
- Full request lifecycle: create token → authenticated request → real data
- Session create → send message → receive response → export
- Config get → modify → save → get (verify persistence)
- WebSocket connect → subscribe → receive event → unsubscribe

### End-to-End
- `weft ui --ui-dir ui/dist` → browser loads → dashboard shows real agents/sessions
- Chat flow: type message → see streaming response → message persisted to session
- MSW disabled in production build (no mock data leaks)

---

## 11. Schedule

| Phase | Scope | Estimated Effort | Dependencies |
|-------|-------|-----------------|-------------|
| A1 | Core infrastructure | Foundation | None |
| A2 | 26 new handlers | Bulk implementation | A1 |
| A3 | WebSocket + streaming | Real-time layer | A2 |
| A4 | CLI + auth + static | Polish | A1 + A2 |
