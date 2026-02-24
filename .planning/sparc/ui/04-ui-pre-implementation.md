# UI Pre-Implementation Validation (UP1--UP6)

> **Element:** Web Dashboard + Live Canvas
> **Phase:** Week 0 -- Pre-Implementation
> **Timeline:** 1 week (5 working days)
> **Priority:** P0 (Gate -- UI sprints cannot begin until all UP tasks pass)
> **Created:** 2026-02-23
> **Directory:** Backend prototyping in `crates/clawft-services/src/api/` (workspace member); Frontend prototyping in `ui/` (standalone, own `package.json`)
> **Dependencies IN:** B5 (shared tool registry builder), existing GatewayConfig + Axum/MCP infrastructure in `clawft-services`
> **Blocks:** S1.1 (Backend API Foundation), S1.2 (Frontend Scaffolding), all subsequent UI sprints
> **Status:** Planning

---

## Table of Contents

1. [Overview](#1-overview)
2. [UP1: Axum REST API Layer Validation](#2-up1-axum-rest-api-layer-validation)
3. [UP2: WebSocket Protocol Design](#3-up2-websocket-protocol-design)
4. [UP3: Authentication Design](#4-up3-authentication-design)
5. [UP4: Frontend Scaffold Validation](#5-up4-frontend-scaffold-validation)
6. [UP5: shadcn/ui Component Audit](#6-up5-shadcnui-component-audit)
7. [UP6: Performance Budget Validation](#7-up6-performance-budget-validation)
8. [Go/No-Go Gate](#8-gono-go-gate)
9. [Week 0 Schedule](#9-week-0-schedule)
10. [Combined Exit Criteria](#10-combined-exit-criteria)

---

## 1. Overview

Week 0 is a validation gate. Its purpose is to prove that the UI technology stack -- both the Rust backend API layer and the React frontend -- works before committing to a 9-week sprint. Every UP task produces a concrete artifact and has binary pass/fail exit criteria. If any P0 UP task fails, the UI sprint plan must be revised before proceeding.

**Guiding principles:**

- Backend API prototyping happens inside the existing `clawft-services` crate, extending the workspace. Validated patterns are kept for sprint S1.1.
- Frontend prototyping lives in a standalone `ui/` directory with its own `package.json`. It is NOT a Rust workspace member.
- Every UP task produces a written report or committed artifact.
- The frontend can be developed, tested, and iterated independently of the backend using MSW (Mock Service Worker).
- Each UP task has explicit pass/fail criteria and a documented fallback when relevant.

**Success = all 6 UP tasks pass their exit criteria.**

**Current state of the codebase relevant to this validation:**

- `clawft-services` already has Axum as an indirect dependency via `tokio` and uses `reqwest` for HTTP. It does NOT currently have `axum` as a direct dependency.
- `clawft-services/Cargo.toml` lists features: `default = []`, `delegate`, `rvf`, `test-utils`, `clawhub`.
- The MCP server (`clawft-services/src/mcp/server.rs`) uses newline-delimited JSON-RPC over `AsyncBufRead`/`AsyncWrite`, not HTTP.
- The gateway (`clawft-cli/src/commands/gateway.rs`) starts channels + agent loop but does NOT serve HTTP endpoints.
- `GatewayConfig` has `host: String` (default `0.0.0.0`) and `port: u16` (default `18790`).
- The workspace `Cargo.toml` already includes `tokio-tungstenite` for WebSocket support.

---

## 2. UP1: Axum REST API Layer Validation

### 2.1 Objective

Verify that the existing `clawft-services` crate can be extended with Axum REST endpoints and that these routes can coexist with the existing MCP server infrastructure. The gateway command must be able to mount additional HTTP routes on the same listener.

### 2.2 Scope

- Add `axum`, `axum-extra`, `tower`, and `tower-http` dependencies to `clawft-services/Cargo.toml`
- Verify no version conflicts with existing workspace dependencies
- Build a prototype API router with a health endpoint
- Validate Bearer token middleware pattern
- Validate static file serving via `rust-embed`
- Decide: same port (18790) vs. separate port for API

### 2.3 Dependency Additions

The following dependencies must be added to `clawft-services/Cargo.toml`:

```toml
# crates/clawft-services/Cargo.toml -- additions for UI API

[features]
default = []
delegate = ["regex"]
rvf = []
test-utils = []
clawhub = []
ui = ["dep:axum", "dep:axum-extra", "dep:tower", "dep:tower-http", "dep:rust-embed"]

[dependencies]
# ... existing deps ...

# HTTP framework (gated behind ui feature)
axum = { version = "0.8", optional = true, features = ["ws", "json"] }
axum-extra = { version = "0.10", optional = true, features = ["typed-header"] }
tower = { version = "0.5", optional = true, features = ["util"] }
tower-http = { version = "0.6", optional = true, features = ["cors", "fs", "compression-gzip", "set-header", "trace"] }
rust-embed = { version = "8", optional = true }
```

**Validation step:** Run `cargo check -p clawft-services --features ui` and confirm zero errors.

### 2.4 Prototype Router

```rust
// crates/clawft-services/src/api/mod.rs -- prototype (pseudocode)

#[cfg(feature = "ui")]
pub mod routes;
#[cfg(feature = "ui")]
pub mod auth;
#[cfg(feature = "ui")]
pub mod ws;

use axum::{Router, Json, extract::State};
use axum::response::IntoResponse;
use axum::http::StatusCode;
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;

/// Shared application state for API handlers.
pub struct ApiState {
    // References to AppContext components:
    // - ToolRegistry (for GET /api/tools)
    // - SessionStore (for GET /api/sessions)
    // - Bus (for WebSocket event forwarding)
    // - Config (for reading current config)
    // Placeholder for prototype:
    pub startup_time: chrono::DateTime<chrono::Utc>,
}

/// Build the API router.
///
/// This function returns an Axum `Router` that can be merged with
/// the main gateway listener or served on its own port.
pub fn api_router(state: Arc<ApiState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)  // Tightened in production via config
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", axum::routing::get(health))
        .layer(cors)
        .with_state(state)
}

async fn health(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "uptime_seconds": (chrono::Utc::now() - state.startup_time).num_seconds(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
```

### 2.5 Bearer Token Middleware Prototype

```rust
// crates/clawft-services/src/api/auth.rs -- prototype (pseudocode)

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

/// Expected token, loaded at startup from ~/.clawft/ui-token or generated.
/// Stored in ApiState for O(1) comparison.
///
/// Middleware extracts Bearer token from Authorization header and
/// compares against the expected value using constant-time comparison.
pub async fn bearer_auth(
    State(state): State<Arc<ApiState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for health endpoint and token generation
    let path = request.uri().path();
    if path == "/api/health" || path == "/api/auth/token" {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            // Constant-time comparison to prevent timing attacks
            if constant_time_eq(token.as_bytes(), state.expected_token.as_bytes()) {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
```

### 2.6 Static File Serving Prototype

```rust
// crates/clawft-services/src/api/static_files.rs -- prototype (pseudocode)

#[cfg(feature = "ui")]
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../ui/dist/"]
struct UiAssets;

/// Serve embedded static files at the root path.
/// Falls back to index.html for SPA client-side routing.
pub async fn serve_static(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try exact match first
    if let Some(content) = UiAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
            content.data.to_vec(),
        ).into_response();
    }

    // SPA fallback: serve index.html for non-API routes
    if let Some(content) = UiAssets::get("index.html") {
        return (
            [(axum::http::header::CONTENT_TYPE, "text/html")],
            content.data.to_vec(),
        ).into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
```

### 2.7 Port Decision

Two options must be evaluated:

**Option A: Same port (18790) -- API routes coexist with gateway**

```text
:18790/                   -> Static files (ui/dist/)
:18790/api/*              -> REST API
:18790/ws                 -> WebSocket
:18790 (stdio/internal)   -> MCP JSON-RPC (unchanged, not HTTP)
```

Pros:
- Single port simplifies configuration, firewall rules, and Tailscale setup
- `weft ui` only needs to open one URL
- CORS is trivially same-origin in production

Cons:
- Axum routes and MCP stdio are on different transports (HTTP vs. stdio) -- no actual conflict
- Slightly more complex router composition in gateway startup code

**Option B: Separate port (18791 for API)**

Pros:
- Clean separation of concerns
- Can independently restart API without affecting channels

Cons:
- Two ports to configure, expose, and document
- CORS needed between ports even on localhost
- `weft ui` must know both ports

**Recommendation:** Option A (same port). The MCP server uses stdio transport, not HTTP, so there is no conflict. The Axum HTTP listener and MCP stdio server are completely independent. The gateway already listens on 18790 for heartbeat; adding API routes to the same listener is straightforward.

### 2.8 UP1 Exit Criteria

- [ ] `cargo check -p clawft-services --features ui` passes with zero errors
- [ ] `cargo check -p clawft-services` (without `ui` feature) still passes -- no regressions
- [ ] Prototype `GET /api/health` endpoint returns JSON with `status`, `uptime_seconds`, `version`
- [ ] Bearer token middleware rejects requests without valid token (returns 401)
- [ ] Bearer token middleware allows requests with valid token
- [ ] `rust-embed` compiles and can embed a test HTML file
- [ ] Port decision is documented with rationale
- [ ] No new `unsafe` code outside of dependencies
- [ ] Prototype compiles on Linux x86_64 (primary dev platform)

---

## 3. UP2: WebSocket Protocol Design

### 3.1 Objective

Lock down the WebSocket event/command protocol before implementation. Define all message types, serialization format, topic subscription model, and reconnection strategy. This protocol document becomes the contract between backend (Rust) and frontend (TypeScript).

### 3.2 Transport Design

**Connection endpoint:** `ws://localhost:18790/ws` (or `wss://` behind TLS)

**Authentication:** Bearer token passed as a query parameter on initial connection:
```
ws://localhost:18790/ws?token=<bearer-token>
```

The WebSocket upgrade handler validates the token before completing the handshake. Invalid or missing tokens result in HTTP 401 before the WebSocket upgrade.

**Message format:** All messages are UTF-8 JSON. Binary frames are reserved for future audio streaming and are rejected in v1.

**Framing:** Each WebSocket text frame contains exactly one JSON message. No newline delimiting within frames.

### 3.3 Protocol Overview

```text
Client                                    Server
  |                                          |
  |--- WS Connect (/ws?token=xxx) --------->|
  |<-- Connection Accepted ------------------|
  |                                          |
  |--- subscribe {topic: "agents"} -------->|
  |<-- subscribed {topic: "agents"} ---------|
  |                                          |
  |<-- event {topic: "agents", type: ...} ---|
  |<-- event {topic: "agents", type: ...} ---|
  |                                          |
  |--- unsubscribe {topic: "agents"} ------>|
  |<-- unsubscribed {topic: "agents"} ------|
  |                                          |
  |--- ping -------------------------------->|
  |<-- pong ---------------------------------|
  |                                          |
  |--- send_message {session, content} ----->|
  |<-- event {topic: "chat:session_id"} ----|
  |                                          |
```

### 3.4 Endpoint Decision

**Option A: Single `/ws` endpoint with topic-based subscription**

All event types flow through one WebSocket connection. Clients subscribe to topics to filter which events they receive.

**Option B: Multiple endpoints (`/ws/chat`, `/ws/canvas`, `/ws/agents`)**

Each view gets its own WebSocket connection.

**Decision: Option A (single `/ws` with topics)**

Rationale:
- Browsers limit concurrent WebSocket connections per host (typically 6-30)
- Single connection is simpler to manage for reconnection logic
- Topic subscription allows fine-grained filtering without multiple connections
- Mobile clients benefit from fewer persistent connections (battery, network)
- Dashboard views that display cross-cutting data (e.g., agent status + chat) work naturally

### 3.5 Topic Namespace

Topics follow a hierarchical naming convention:

| Topic | Description | Subscription Scope |
|-------|-------------|-------------------|
| `agents` | Agent lifecycle events (created, started, stopped, error) | Global |
| `agent:{agent_id}` | Events for a specific agent | Per-agent |
| `sessions` | Session lifecycle events (created, closed) | Global |
| `chat:{session_id}` | Chat messages for a specific session | Per-session |
| `tools` | Tool registry changes (registered, unregistered) | Global |
| `tool_calls` | All tool call events across sessions | Global |
| `tool_call:{session_id}` | Tool calls for a specific session | Per-session |
| `canvas:{session_id}` | Canvas commands for a specific session | Per-session |
| `skills` | Skill lifecycle events (installed, uninstalled, reloaded) | Global |
| `memory` | Memory store changes (stored, deleted) | Global |
| `config` | Configuration changes | Global |
| `delegation` | Delegation events (started, completed, failed) | Global |
| `cron` | Cron job events (triggered, completed, failed) | Global |
| `channels` | Channel status events (connected, disconnected, error) | Global |
| `system` | System-level events (health, metrics) | Global |

### 3.6 Server-to-Client Event Types (15 types)

All server-to-client messages follow this envelope:

```json
{
    "type": "event",
    "topic": "<topic_name>",
    "event": "<event_type>",
    "data": { ... },
    "timestamp": "2026-02-23T12:00:00.000Z",
    "id": "<uuid>"
}
```

#### Event Type Catalog

**1. `agent.status_changed`**
```json
{
    "type": "event",
    "topic": "agents",
    "event": "agent.status_changed",
    "data": {
        "agent_id": "uuid",
        "name": "string",
        "status": "idle | running | error | stopped",
        "previous_status": "idle | running | error | stopped",
        "model": "string",
        "uptime_seconds": 3600
    },
    "timestamp": "2026-02-23T12:00:00.000Z",
    "id": "uuid"
}
```

**2. `agent.error`**
```json
{
    "type": "event",
    "topic": "agent:<agent_id>",
    "event": "agent.error",
    "data": {
        "agent_id": "uuid",
        "error_type": "llm_error | tool_error | timeout | internal",
        "message": "string",
        "recoverable": true
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**3. `chat.message`**
```json
{
    "type": "event",
    "topic": "chat:<session_id>",
    "event": "chat.message",
    "data": {
        "session_id": "uuid",
        "message_id": "uuid",
        "role": "user | assistant | system",
        "content": "string",
        "is_partial": false,
        "channel": "web | telegram | slack | discord"
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**4. `chat.stream_chunk`**
```json
{
    "type": "event",
    "topic": "chat:<session_id>",
    "event": "chat.stream_chunk",
    "data": {
        "session_id": "uuid",
        "message_id": "uuid",
        "chunk": "string",
        "chunk_index": 0,
        "is_final": false
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**5. `chat.stream_end`**
```json
{
    "type": "event",
    "topic": "chat:<session_id>",
    "event": "chat.stream_end",
    "data": {
        "session_id": "uuid",
        "message_id": "uuid",
        "total_chunks": 42,
        "total_tokens": 256,
        "latency_ms": 1200
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**6. `session.created`**
```json
{
    "type": "event",
    "topic": "sessions",
    "event": "session.created",
    "data": {
        "session_id": "uuid",
        "agent_id": "uuid",
        "channel": "web | telegram | slack | discord",
        "user_id": "string"
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**7. `session.closed`**
```json
{
    "type": "event",
    "topic": "sessions",
    "event": "session.closed",
    "data": {
        "session_id": "uuid",
        "reason": "user_closed | timeout | error",
        "message_count": 42,
        "duration_seconds": 300
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**8. `tool_call.started`**
```json
{
    "type": "event",
    "topic": "tool_call:<session_id>",
    "event": "tool_call.started",
    "data": {
        "call_id": "uuid",
        "session_id": "uuid",
        "tool_name": "string",
        "arguments": {},
        "agent_id": "uuid"
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**9. `tool_call.completed`**
```json
{
    "type": "event",
    "topic": "tool_call:<session_id>",
    "event": "tool_call.completed",
    "data": {
        "call_id": "uuid",
        "session_id": "uuid",
        "tool_name": "string",
        "result": {},
        "is_error": false,
        "duration_ms": 150
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**10. `canvas.command`**
```json
{
    "type": "event",
    "topic": "canvas:<session_id>",
    "event": "canvas.command",
    "data": {
        "session_id": "uuid",
        "command": "render | update | remove | reset | snapshot",
        "element_id": "string",
        "element_type": "text | button | input | image | code | chart | table | form",
        "props": {},
        "position": { "order": 0 }
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**11. `skill.changed`**
```json
{
    "type": "event",
    "topic": "skills",
    "event": "skill.changed",
    "data": {
        "action": "installed | uninstalled | reloaded | updated",
        "skill_name": "string",
        "version": "string",
        "tools_provided": ["string"]
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**12. `delegation.status`**
```json
{
    "type": "event",
    "topic": "delegation",
    "event": "delegation.status",
    "data": {
        "delegation_id": "uuid",
        "session_id": "uuid",
        "target": "local | claude_code | claude_flow",
        "status": "pending | running | completed | failed",
        "tool_name": "string",
        "latency_ms": 500,
        "token_usage": { "input": 100, "output": 200 }
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**13. `config.changed`**
```json
{
    "type": "event",
    "topic": "config",
    "event": "config.changed",
    "data": {
        "section": "agents | providers | channels | tools | delegation | routing | gateway",
        "changed_keys": ["string"],
        "source": "api | file | cli"
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**14. `channel.status`**
```json
{
    "type": "event",
    "topic": "channels",
    "event": "channel.status",
    "data": {
        "channel_name": "telegram | slack | discord | web",
        "status": "connected | disconnected | error | reconnecting",
        "message_count": 1234,
        "error_message": "string | null"
    },
    "timestamp": "...",
    "id": "uuid"
}
```

**15. `system.health`**
```json
{
    "type": "event",
    "topic": "system",
    "event": "system.health",
    "data": {
        "uptime_seconds": 86400,
        "active_agents": 3,
        "active_sessions": 7,
        "active_ws_connections": 2,
        "memory_usage_mb": 128,
        "tool_count": 42
    },
    "timestamp": "...",
    "id": "uuid"
}
```

### 3.7 Client-to-Server Command Types (5 types)

All client-to-server messages follow this envelope:

```json
{
    "type": "command",
    "command": "<command_type>",
    "data": { ... },
    "request_id": "<uuid>"
}
```

The `request_id` is optional and used for request-response correlation (e.g., `send_message` expects a `chat.message` event with the same `request_id` in a correlation field).

#### Command Type Catalog

**1. `subscribe`**
```json
{
    "type": "command",
    "command": "subscribe",
    "data": {
        "topic": "agents"
    },
    "request_id": "uuid"
}
```

Server responds with:
```json
{
    "type": "response",
    "command": "subscribe",
    "data": {
        "topic": "agents",
        "success": true
    },
    "request_id": "uuid"
}
```

**2. `unsubscribe`**
```json
{
    "type": "command",
    "command": "unsubscribe",
    "data": {
        "topic": "agents"
    },
    "request_id": "uuid"
}
```

Server responds with:
```json
{
    "type": "response",
    "command": "unsubscribe",
    "data": {
        "topic": "agents",
        "success": true
    },
    "request_id": "uuid"
}
```

**3. `send_message`**
```json
{
    "type": "command",
    "command": "send_message",
    "data": {
        "session_id": "uuid",
        "content": "Hello, agent!",
        "create_session": false,
        "agent_id": "uuid | null"
    },
    "request_id": "uuid"
}
```

If `session_id` is null and `create_session` is true, a new session is created. The server publishes `chat.message` events to the `chat:<session_id>` topic.

**4. `canvas_interaction`**
```json
{
    "type": "command",
    "command": "canvas_interaction",
    "data": {
        "session_id": "uuid",
        "element_id": "string",
        "interaction": "click | submit | change | focus | blur",
        "value": "string | null",
        "form_data": {}
    },
    "request_id": "uuid"
}
```

Canvas interactions are routed back to the agent as tool call results for the `render_ui` tool.

**5. `ping`**
```json
{
    "type": "command",
    "command": "ping",
    "data": {},
    "request_id": "uuid"
}
```

Server responds with:
```json
{
    "type": "response",
    "command": "pong",
    "data": {
        "server_time": "2026-02-23T12:00:00.000Z"
    },
    "request_id": "uuid"
}
```

### 3.8 Subscription Management

Each WebSocket connection maintains a set of subscribed topics. Events are only forwarded to connections that have subscribed to the relevant topic.

**Implementation sketch (Rust, server side):**

```rust
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Per-connection subscription state.
struct ConnectionState {
    subscriptions: HashSet<String>,
    tx: tokio::sync::mpsc::UnboundedSender<String>,
}

/// Global subscription manager.
struct SubscriptionManager {
    connections: RwLock<HashMap<uuid::Uuid, ConnectionState>>,
}

impl SubscriptionManager {
    /// Publish an event to all connections subscribed to the given topic.
    async fn publish(&self, topic: &str, event_json: &str) {
        let conns = self.connections.read().await;
        for (_, conn) in conns.iter() {
            if conn.subscriptions.contains(topic) {
                let _ = conn.tx.send(event_json.to_string());
            }
        }
    }

    /// Subscribe a connection to a topic. Returns true if newly subscribed.
    async fn subscribe(&self, conn_id: uuid::Uuid, topic: String) -> bool {
        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.get_mut(&conn_id) {
            conn.subscriptions.insert(topic)
        } else {
            false
        }
    }

    /// Unsubscribe a connection from a topic. Returns true if was subscribed.
    async fn unsubscribe(&self, conn_id: uuid::Uuid, topic: &str) -> bool {
        let mut conns = self.connections.write().await;
        if let Some(conn) = conns.get_mut(&conn_id) {
            conn.subscriptions.remove(topic)
        } else {
            false
        }
    }
}
```

**Scalability target:** 100+ concurrent subscriptions across 10+ connections. The `RwLock<HashMap>` approach is sufficient for this scale. If we need 1000+ connections in the future, migrate to a sharded subscription map.

### 3.9 Reconnection Strategy

The client implements exponential backoff with jitter:

```typescript
// ui/src/lib/ws-client.ts -- reconnection pseudocode

const INITIAL_DELAY_MS = 1000;
const MAX_DELAY_MS = 30000;
const JITTER_FACTOR = 0.3;

function reconnectDelay(attempt: number): number {
    const base = Math.min(INITIAL_DELAY_MS * Math.pow(2, attempt), MAX_DELAY_MS);
    const jitter = base * JITTER_FACTOR * (Math.random() * 2 - 1);
    return Math.max(0, base + jitter);
}

// On reconnect:
// 1. Reconnect WebSocket with token
// 2. Re-subscribe to all previously active topics
// 3. For each chat session, fetch missed messages via REST API
//    GET /api/sessions/:id/messages?after=<last_seen_timestamp>
// 4. Reset reconnect attempt counter on successful message receipt
```

### 3.10 Message Throughput Target

**Target: 1000 messages/second** on a single WebSocket connection.

Benchmark methodology:
1. Server publishes 1000 JSON events in a tight loop
2. Client counts received messages and measures elapsed time
3. Measure: messages/second, p50 latency, p99 latency

This target accommodates:
- 10 concurrent sessions with 100 msg/sec each (heavy streaming)
- Canvas updates at 60 fps for one session (60 msg/sec)
- System health events at 1/sec
- Agent status events at ~10/sec peak

### 3.11 Server-Side Heartbeat

The server sends a `system.health` event every 30 seconds to all connections subscribed to the `system` topic. If a connection has not sent any message (including `ping`) for 60 seconds, the server closes it with a close frame (code 1000, reason "idle timeout").

The client should send a `ping` command at least every 45 seconds to keep the connection alive.

### 3.12 UP2 Exit Criteria

- [ ] WebSocket protocol document is complete with all 15 event types and 5 command types
- [ ] JSON schema for each event/command type is validated (parseable by both Rust `serde_json` and TypeScript)
- [ ] Prototype WS handler accepts connections, handles subscribe/unsubscribe, echoes test events
- [ ] Topic subscription model correctly filters events per connection
- [ ] Benchmark: 1000 msg/sec throughput on localhost (measured with prototype)
- [ ] Reconnection strategy documented with code sample
- [ ] Endpoint decision documented (single `/ws` with topics)
- [ ] Server heartbeat / idle timeout behavior specified

---

## 4. UP3: Authentication Design

### 4.1 Objective

Design the token-based auth system for local and remote access. The auth system must support the `weft ui` one-time URL flow for local development and a persistent token for API clients.

### 4.2 Auth Flow

```text
                     weft ui
                       |
                       v
              +-----------------+
              | Generate Token  |
              | (32 random hex) |
              +-----------------+
                       |
                       v
              +-----------------+
              | Write to        |
              | ~/.clawft/      |
              |   ui-token      |
              | (chmod 0600)    |
              +-----------------+
                       |
                       v
              +-----------------+
              | Start Gateway   |
              | (if not running)|
              +-----------------+
                       |
                       v
              +-----------------+
              | Open Browser    |
              | http://host:    |
              | 18790/?token=   |
              | <one-time-tok>  |
              +-----------------+
                       |
                       v
            Browser loads SPA
                       |
                       v
              +-----------------+
              | SPA extracts    |
              | ?token= from   |
              | URL, calls:     |
              | POST /api/auth/ |
              |   exchange      |
              | Body: {token}   |
              +-----------------+
                       |
                       v
              +-----------------+
              | Server validates|
              | one-time token, |
              | invalidates it, |
              | returns session |
              | token (24h TTL) |
              +-----------------+
                       |
                       v
              +-----------------+
              | SPA stores      |
              | session token   |
              | in localStorage |
              | Uses as Bearer  |
              | for all API/WS  |
              +-----------------+
```

### 4.3 Token Format Decision

**Option A: Opaque Token (random 32-byte hex)**

```
a1b2c3d4e5f6...64 hex characters...
```

- Stored in server memory (HashMap<String, TokenMetadata>)
- No crypto verification needed -- direct lookup
- Cannot be decoded by client (no payload)
- Requires server-side state

**Option B: JWT (RS256 or HS256)**

```
eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIiwiZXhwIjoxNzA5MDAwMDAwfQ.signature
```

- Self-contained (no server-side state needed for validation)
- Can carry claims (user_id, roles, expiry)
- Client can decode payload (non-secret)
- Requires HMAC key management

**Decision: Opaque Token**

Rationale:
- ClawFT is primarily a single-user local tool. JWT's multi-user claim system adds complexity without benefit for v1.
- Server-side token storage is trivial (single HashMap in memory, persisted to `~/.clawft/ui-token`).
- No HMAC key management, no rotation, no key distribution.
- Simpler to implement, audit, and debug.
- For future multi-user support (Tailscale auth), Tailscale provides identity headers directly -- JWT is not needed.

### 4.4 Token Specification

| Property | Value |
|----------|-------|
| **Format** | 64 hex characters (32 random bytes, hex-encoded) |
| **Generation** | `rand::thread_rng().gen::<[u8; 32]>()` hex-encoded |
| **Storage (server)** | `~/.clawft/ui-token` file, permissions `0600` |
| **Storage (client)** | `localStorage.setItem("clawft-ui-token", token)` |
| **Transmission** | `Authorization: Bearer <token>` header (REST), `?token=<token>` query param (WebSocket upgrade) |
| **TTL** | 24 hours (configurable in GatewayConfig) |
| **Refresh** | No refresh mechanism in v1. Token is regenerated by `weft ui`. |
| **Revocation** | Delete `~/.clawft/ui-token` or restart gateway with `--new-token` |

### 4.5 One-Time URL Token

The `weft ui` command generates a separate one-time token used only in the browser URL. This token is exchanged exactly once for a session token.

```text
One-time token:  ot_<32-random-hex>
Session token:   st_<32-random-hex>

The "ot_" and "st_" prefixes allow the server to distinguish token types
and reject one-time tokens used as Bearer tokens (and vice versa).
```

**Exchange endpoint:**

```
POST /api/auth/exchange
Content-Type: application/json

{
    "one_time_token": "ot_a1b2c3d4..."
}

Response 200:
{
    "token": "st_e5f6a7b8...",
    "expires_at": "2026-02-24T12:00:00.000Z"
}

Response 401 (invalid or already used):
{
    "error": "invalid_token",
    "message": "Token is invalid or has already been used"
}
```

### 4.6 GatewayConfig Extensions

```rust
// Additions to GatewayConfig in clawft-types/src/config/mod.rs

/// Gateway / HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_host")]
    pub host: String,

    #[serde(default = "default_gateway_port")]
    pub port: u16,

    #[serde(default, alias = "heartbeatIntervalMinutes")]
    pub heartbeat_interval_minutes: u64,

    #[serde(default = "default_heartbeat_prompt", alias = "heartbeatPrompt")]
    pub heartbeat_prompt: String,

    // --- New fields for UI auth ---

    /// CORS allowed origins (default: ["http://localhost:5173"])
    #[serde(default = "default_cors_origins", alias = "corsOrigins")]
    pub cors_origins: Vec<String>,

    /// UI auth token TTL in hours (default: 24)
    #[serde(default = "default_token_ttl_hours", alias = "tokenTtlHours")]
    pub token_ttl_hours: u64,

    /// Disable auth entirely (for development only, NEVER in production)
    #[serde(default, alias = "disableAuth")]
    pub disable_auth: bool,
}

fn default_cors_origins() -> Vec<String> {
    vec!["http://localhost:5173".into()]
}

fn default_token_ttl_hours() -> u64 {
    24
}
```

### 4.7 Middleware Performance

The Bearer middleware MUST have zero database lookups per request. Token validation is a single in-memory HashMap lookup: `O(1)` expected time.

**Benchmark target:** < 1 microsecond per token validation (excludes network I/O).

### 4.8 weft ui CLI Command

```rust
// crates/clawft-cli/src/commands/ui.rs -- specification (pseudocode)

/// Arguments for the `weft ui` subcommand.
#[derive(Args)]
pub struct UiArgs {
    /// Config file path
    #[arg(short, long)]
    pub config: Option<String>,

    /// Force generate a new token (invalidates existing)
    #[arg(long)]
    pub new_token: bool,

    /// Port override (default: from config, 18790)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Do not open browser automatically
    #[arg(long)]
    pub no_open: bool,
}

pub async fn run(args: UiArgs) -> anyhow::Result<()> {
    // 1. Load config
    // 2. Generate or load token from ~/.clawft/ui-token
    // 3. Generate one-time URL token
    // 4. Start gateway (if not already running)
    // 5. Open browser with: http://host:port/?token=ot_xxx
    // 6. Print token to stderr for manual use:
    //    eprintln!("UI available at: http://localhost:18790/")
    //    eprintln!("Session token: st_xxx (valid for 24h)")
    // 7. Wait for Ctrl+C
    Ok(())
}
```

### 4.9 UP3 Exit Criteria

- [ ] Token format specification is complete (opaque, 64 hex chars, prefixed)
- [ ] Auth flow diagram is documented (weft ui -> generate token -> browser -> exchange -> Bearer)
- [ ] One-time URL token exchange endpoint is specified
- [ ] Token storage location and permissions are documented (`~/.clawft/ui-token`, 0600)
- [ ] GatewayConfig extensions are specified (cors_origins, token_ttl_hours, disable_auth)
- [ ] Bearer middleware implementation sketch uses constant-time comparison
- [ ] Performance target documented (< 1us per validation, no DB lookups)
- [ ] `weft ui` CLI command specification is complete
- [ ] Decision documented: opaque token vs JWT with rationale

---

## 5. UP4: Frontend Scaffold Validation

### 5.1 Objective

Verify that Vite + React + shadcn/ui + TanStack Router all work together and produce a viable development environment. The frontend must be fully functional without any backend (MSW mocks all API calls).

### 5.2 Directory Structure

```
ui/
  package.json
  pnpm-lock.yaml
  tsconfig.json
  tsconfig.node.json
  vite.config.ts
  tailwind.config.ts
  postcss.config.js
  components.json                 # shadcn/ui config
  index.html
  public/
    favicon.svg
  src/
    main.tsx                      # React entry point
    App.tsx                       # Root component with router
    index.css                     # Tailwind directives
    vite-env.d.ts                 # Vite type declarations
    lib/
      api-client.ts               # Fetch wrapper with Bearer auth
      ws-client.ts                # Reconnecting WebSocket client
      utils.ts                    # cn() utility for tailwind merge
      types.ts                    # Shared TypeScript types
    hooks/
      use-auth.ts                 # Token extraction and storage
      use-websocket.ts            # WebSocket subscription hook
    routes/
      __root.tsx                  # TanStack Router root layout
      index.tsx                   # Dashboard home
      agents.tsx                  # Agent management
      chat.tsx                    # WebChat
      sessions.tsx                # Session explorer
      tools.tsx                   # Tool registry
    components/
      ui/                         # shadcn/ui components (auto-generated)
      layout/
        MainLayout.tsx            # Sidebar + header + content area
        Sidebar.tsx               # Collapsible navigation sidebar
    mocks/
      handlers.ts                 # MSW request handlers
      browser.ts                  # MSW browser worker setup
      fixtures/
        agents.json               # Mock agent data
        sessions.json             # Mock session data
        tools.json                # Mock tool data
  .env                            # VITE_API_URL, VITE_MOCK_API
  .env.development                # Development defaults
  Dockerfile                      # Multi-stage build
```

### 5.3 package.json

```json
{
    "name": "clawft-ui",
    "private": true,
    "version": "0.1.0",
    "type": "module",
    "scripts": {
        "dev": "vite",
        "build": "tsc -b && vite build",
        "lint": "eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0",
        "preview": "vite preview",
        "test": "vitest",
        "test:ui": "vitest --ui",
        "type-check": "tsc --noEmit",
        "analyze": "vite-bundle-visualizer"
    },
    "dependencies": {
        "react": "^19.0.0",
        "react-dom": "^19.0.0",
        "@tanstack/react-router": "^1.100.0",
        "@tanstack/router-devtools": "^1.100.0",
        "class-variance-authority": "^0.7.1",
        "clsx": "^2.1.1",
        "tailwind-merge": "^2.6.0",
        "lucide-react": "^0.468.0",
        "sonner": "^1.7.0",
        "zustand": "^5.0.0",
        "date-fns": "^4.1.0",
        "@tanstack/react-table": "^8.20.0",
        "@tanstack/react-query": "^5.62.0"
    },
    "devDependencies": {
        "@types/react": "^19.0.0",
        "@types/react-dom": "^19.0.0",
        "@vitejs/plugin-react": "^4.3.0",
        "typescript": "~5.7.0",
        "vite": "^6.0.0",
        "vitest": "^2.1.0",
        "eslint": "^9.15.0",
        "@eslint/js": "^9.15.0",
        "typescript-eslint": "^8.15.0",
        "eslint-plugin-react-hooks": "^5.0.0",
        "eslint-plugin-react-refresh": "^0.4.14",
        "globals": "^15.12.0",
        "tailwindcss": "^4.0.0",
        "@tailwindcss/vite": "^4.0.0",
        "postcss": "^8.4.49",
        "autoprefixer": "^10.4.20",
        "msw": "^2.6.0",
        "vite-bundle-visualizer": "^1.2.0",
        "@testing-library/react": "^16.0.0",
        "@testing-library/jest-dom": "^6.6.0",
        "jsdom": "^25.0.0"
    }
}
```

### 5.4 vite.config.ts

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

export default defineConfig({
    plugins: [react(), tailwindcss()],
    resolve: {
        alias: {
            "@": path.resolve(__dirname, "./src"),
        },
    },
    server: {
        port: 5173,
        proxy: {
            "/api": {
                target: "http://localhost:18790",
                changeOrigin: true,
            },
            "/ws": {
                target: "ws://localhost:18790",
                ws: true,
            },
        },
    },
    build: {
        target: "es2022",
        sourcemap: true,
        rollupOptions: {
            output: {
                manualChunks: {
                    // Split heavy dependencies into separate chunks
                    vendor: ["react", "react-dom"],
                    router: ["@tanstack/react-router"],
                    table: ["@tanstack/react-table"],
                    query: ["@tanstack/react-query"],
                },
            },
        },
    },
});
```

### 5.5 tsconfig.json

```json
{
    "compilerOptions": {
        "target": "ES2022",
        "useDefineForClassFields": true,
        "lib": ["ES2022", "DOM", "DOM.Iterable"],
        "module": "ESNext",
        "skipLibCheck": true,
        "moduleResolution": "bundler",
        "allowImportingTsExtensions": true,
        "isolatedModules": true,
        "moduleDetection": "force",
        "noEmit": true,
        "jsx": "react-jsx",
        "strict": true,
        "noUnusedLocals": true,
        "noUnusedParameters": true,
        "noFallthroughCasesInSwitch": true,
        "noUncheckedIndexedAccess": true,
        "paths": {
            "@/*": ["./src/*"]
        }
    },
    "include": ["src"],
    "references": [{ "path": "./tsconfig.node.json" }]
}
```

### 5.6 tailwind.config.ts

```typescript
import type { Config } from "tailwindcss";

export default {
    darkMode: "class",
    content: ["./index.html", "./src/**/*.{ts,tsx}"],
    theme: {
        extend: {
            colors: {
                // shadcn/ui CSS variable-based theme
                border: "hsl(var(--border))",
                input: "hsl(var(--input))",
                ring: "hsl(var(--ring))",
                background: "hsl(var(--background))",
                foreground: "hsl(var(--foreground))",
                primary: {
                    DEFAULT: "hsl(var(--primary))",
                    foreground: "hsl(var(--primary-foreground))",
                },
                secondary: {
                    DEFAULT: "hsl(var(--secondary))",
                    foreground: "hsl(var(--secondary-foreground))",
                },
                destructive: {
                    DEFAULT: "hsl(var(--destructive))",
                    foreground: "hsl(var(--destructive-foreground))",
                },
                muted: {
                    DEFAULT: "hsl(var(--muted))",
                    foreground: "hsl(var(--muted-foreground))",
                },
                accent: {
                    DEFAULT: "hsl(var(--accent))",
                    foreground: "hsl(var(--accent-foreground))",
                },
                popover: {
                    DEFAULT: "hsl(var(--popover))",
                    foreground: "hsl(var(--popover-foreground))",
                },
                card: {
                    DEFAULT: "hsl(var(--card))",
                    foreground: "hsl(var(--card-foreground))",
                },
            },
            borderRadius: {
                lg: "var(--radius)",
                md: "calc(var(--radius) - 2px)",
                sm: "calc(var(--radius) - 4px)",
            },
        },
    },
    plugins: [],
} satisfies Config;
```

### 5.7 MSW Mock Setup

```typescript
// ui/src/mocks/handlers.ts -- specification

import { http, HttpResponse } from "msw";

const API_BASE = "/api";

export const handlers = [
    // Health check
    http.get(`${API_BASE}/health`, () => {
        return HttpResponse.json({
            status: "ok",
            uptime_seconds: 3600,
            version: "0.1.0",
        });
    }),

    // Agent listing
    http.get(`${API_BASE}/agents`, () => {
        return HttpResponse.json({
            agents: [
                {
                    id: "550e8400-e29b-41d4-a716-446655440000",
                    name: "default",
                    status: "running",
                    model: "anthropic/claude-opus-4-5",
                    uptime_seconds: 1800,
                    session_count: 3,
                },
            ],
        });
    }),

    // Session listing
    http.get(`${API_BASE}/sessions`, () => {
        return HttpResponse.json({
            sessions: [
                {
                    id: "660e8400-e29b-41d4-a716-446655440001",
                    agent_id: "550e8400-e29b-41d4-a716-446655440000",
                    channel: "web",
                    message_count: 12,
                    created_at: "2026-02-23T10:00:00Z",
                    updated_at: "2026-02-23T11:30:00Z",
                },
            ],
        });
    }),

    // Tool listing
    http.get(`${API_BASE}/tools`, () => {
        return HttpResponse.json({
            tools: [
                {
                    name: "exec",
                    description: "Execute shell commands",
                    input_schema: {
                        type: "object",
                        properties: {
                            command: { type: "string" },
                        },
                        required: ["command"],
                    },
                },
                {
                    name: "web_search",
                    description: "Search the web",
                    input_schema: {
                        type: "object",
                        properties: {
                            query: { type: "string" },
                        },
                        required: ["query"],
                    },
                },
            ],
        });
    }),

    // Token exchange
    http.post(`${API_BASE}/auth/exchange`, async ({ request }) => {
        const body = await request.json() as { one_time_token: string };
        if (body.one_time_token?.startsWith("ot_")) {
            return HttpResponse.json({
                token: "st_mock_session_token_for_development",
                expires_at: "2026-02-24T12:00:00Z",
            });
        }
        return HttpResponse.json(
            { error: "invalid_token", message: "Invalid one-time token" },
            { status: 401 }
        );
    }),
];
```

```typescript
// ui/src/mocks/browser.ts

import { setupWorker } from "msw/browser";
import { handlers } from "./handlers";

export const worker = setupWorker(...handlers);
```

```typescript
// ui/src/main.tsx -- conditional MSW initialization

async function enableMocking() {
    if (import.meta.env.VITE_MOCK_API !== "true") {
        return;
    }
    const { worker } = await import("./mocks/browser");
    return worker.start({
        onUnhandledRequest: "warn",
    });
}

enableMocking().then(() => {
    const root = ReactDOM.createRoot(document.getElementById("root")!);
    root.render(
        <React.StrictMode>
            <App />
        </React.StrictMode>
    );
});
```

### 5.8 Proxy Configuration

During development, the Vite dev server proxies `/api/*` and `/ws` to the backend on port 18790. This eliminates CORS issues during local development.

In production (when `ui/dist/` is embedded via `rust-embed` or served from disk), all requests go to the same origin -- no proxy needed.

### 5.9 CI Configuration

```yaml
# .github/workflows/ui.yml -- specification

name: UI CI
on:
  push:
    paths: ["ui/**"]
  pull_request:
    paths: ["ui/**"]

jobs:
  lint-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: ui/pnpm-lock.yaml
      - run: pnpm install --frozen-lockfile
        working-directory: ui
      - run: pnpm lint
        working-directory: ui
      - run: pnpm type-check
        working-directory: ui
      - run: pnpm test -- --run
        working-directory: ui
      - run: pnpm build
        working-directory: ui
      - name: Check bundle size
        run: |
          TOTAL=$(du -sb ui/dist/ | cut -f1)
          GZIP=$(find ui/dist -name '*.js' -o -name '*.css' | xargs gzip -c | wc -c)
          echo "Total: ${TOTAL} bytes, Gzipped JS+CSS: ${GZIP} bytes"
          if [ "$GZIP" -gt 204800 ]; then
            echo "ERROR: Gzipped bundle exceeds 200 KB"
            exit 1
          fi
```

### 5.10 UP4 Exit Criteria

- [ ] `pnpm install` completes without errors or peer dependency warnings
- [ ] `pnpm dev` starts Vite dev server on `:5173` with HMR working
- [ ] `VITE_MOCK_API=true pnpm dev` runs fully without any backend connection
- [ ] MSW intercepts API requests and returns mock data
- [ ] TanStack Router file-based routing works (navigate between 2+ routes)
- [ ] shadcn/ui initialization succeeds (`components.json` created)
- [ ] At least one shadcn component (Button) renders correctly
- [ ] `pnpm build` produces `ui/dist/` directory
- [ ] Built bundle is under 200 KB gzipped (empty shell with router + shadcn)
- [ ] `pnpm lint` and `pnpm type-check` pass with zero errors
- [ ] Proxy configuration routes `/api/*` to backend in dev mode

---

## 6. UP5: shadcn/ui Component Audit

### 6.1 Objective

Verify that all shadcn/ui components needed for the UI dashboard exist, install correctly, and work with Tailwind CSS v4. Identify components that need to be built custom.

### 6.2 Required Components (14)

These components are needed across S1.3 and S2.x phases:

| # | Component | shadcn Name | Phase Needed | Purpose |
|---|-----------|-------------|-------------|---------|
| 1 | Button | `button` | S1.2 | Actions across all views |
| 2 | Card | `card` | S1.3 | Dashboard cards, agent cards, skill cards |
| 3 | Input | `input` | S1.3 | Search bars, forms |
| 4 | Badge | `badge` | S1.3 | Status indicators (running/stopped/error) |
| 5 | Table | `table` | S1.3 | Session explorer, memory list |
| 6 | Dialog | `dialog` | S1.3 | Confirmation dialogs, create forms |
| 7 | Dropdown Menu | `dropdown-menu` | S1.3 | Agent actions, context menus |
| 8 | Tabs | `tabs` | S2.4 | Config editor sections |
| 9 | Scroll Area | `scroll-area` | S1.3 | Chat message scrolling |
| 10 | Separator | `separator` | S1.3 | Visual dividers |
| 11 | Skeleton | `skeleton` | S1.3 | Loading states |
| 12 | Tooltip | `tooltip` | S1.3 | Hover information |
| 13 | Command | `command` | S1.3 | Command palette (Cmd+K) |
| 14 | Resizable | `resizable` | S2.1 | Canvas + Chat split view |

### 6.3 Installation Validation

Run the following commands and verify each installs without error:

```bash
cd ui/
npx shadcn@latest init
npx shadcn@latest add button card input badge table dialog dropdown-menu tabs scroll-area separator skeleton tooltip command resizable
```

**Pass criteria:** All 14 components install without errors and the `ui/src/components/ui/` directory contains the corresponding files.

### 6.4 Compatibility Matrix

| Component | shadcn v2 | Tailwind v4 | Dark Mode | Accessibility | Notes |
|-----------|-----------|-------------|-----------|---------------|-------|
| button | Verify | Verify | Verify | Verify | Must support all variants (default, destructive, outline, secondary, ghost, link) |
| card | Verify | Verify | Verify | Verify | |
| input | Verify | Verify | Verify | Verify | Must support `disabled`, `error` states |
| badge | Verify | Verify | Verify | Verify | |
| table | Verify | Verify | Verify | Verify | Base table only; DataTable is TanStack |
| dialog | Verify | Verify | Verify | Verify | Must handle focus trap |
| dropdown-menu | Verify | Verify | Verify | Verify | Must support keyboard navigation |
| tabs | Verify | Verify | Verify | Verify | |
| scroll-area | Verify | Verify | Verify | Verify | Must handle dynamic content height |
| separator | Verify | Verify | Verify | Verify | |
| skeleton | Verify | Verify | Verify | Verify | Must animate (pulse) |
| tooltip | Verify | Verify | Verify | Verify | |
| command | Verify | Verify | Verify | Verify | Must support fuzzy search |
| resizable | Verify | Verify | Verify | Verify | Must handle vertical + horizontal |

### 6.5 Performance Tests

#### DataTable with 1000 rows

The Session Explorer and Memory Explorer use `@tanstack/react-table` with shadcn's Table component. Test that 1000 rows render without jank.

**Test procedure:**

```typescript
// ui/src/__tests__/datatable-perf.test.tsx

import { render } from "@testing-library/react";

function generateRows(count: number) {
    return Array.from({ length: count }, (_, i) => ({
        id: `session-${i}`,
        agent_id: `agent-${i % 5}`,
        channel: ["web", "telegram", "slack", "discord"][i % 4],
        message_count: Math.floor(Math.random() * 100),
        created_at: new Date(Date.now() - i * 60000).toISOString(),
    }));
}

test("DataTable renders 1000 rows under 100ms", () => {
    const rows = generateRows(1000);
    const start = performance.now();
    const { container } = render(<DataTable columns={columns} data={rows} />);
    const elapsed = performance.now() - start;

    expect(elapsed).toBeLessThan(100);
    expect(container.querySelectorAll("tr").length).toBeGreaterThan(0);
});
```

**Pass criteria:** Render time < 100ms for 1000 rows. If this fails, enable virtualization via `@tanstack/react-virtual`.

#### ResizablePanel

Test that ResizablePanel works for the Canvas + Chat split view layout.

```typescript
test("ResizablePanel allows drag resize", () => {
    const { getByRole } = render(
        <ResizablePanelGroup direction="horizontal">
            <ResizablePanel defaultSize={50}>Chat</ResizablePanel>
            <ResizableHandle />
            <ResizablePanel defaultSize={50}>Canvas</ResizablePanel>
        </ResizablePanelGroup>
    );
    // Verify both panels render and handle is interactive
});
```

#### Command Palette with 50+ Items

Test that the Command component handles 50+ items with fuzzy search.

```typescript
test("CommandPalette searches 50 items without lag", () => {
    const items = Array.from({ length: 50 }, (_, i) => ({
        label: `Navigation Item ${i}`,
        value: `/route/${i}`,
    }));
    // Render Command with items, type search query, verify filtering
});
```

### 6.6 Custom Components Needed

Components NOT in shadcn that must be built for the dashboard:

| Component | Phase | Description | Complexity |
|-----------|-------|-------------|------------|
| `StatusBadge` | S1.3 | Colored badge with icon for agent/channel status (idle/running/error/stopped) | Low |
| `MessageBubble` | S1.3 | Chat message with role indicator, timestamp, tool call expansion | Medium |
| `ToolCallCard` | S1.3 | Expandable card showing tool name, arguments JSON tree, result | Medium |
| `JsonTreeView` | S1.3 | Collapsible JSON tree for tool schemas and results | Medium |
| `ConnectionIndicator` | S1.2 | WebSocket connection status dot (green/yellow/red) in header | Low |
| `CanvasElement` | S2.1 | Polymorphic renderer for Canvas element types | High |
| `StreamingText` | S1.3 | Text component that animates character-by-character for streaming | Low |
| `CodeBlock` | S1.3 | Syntax-highlighted code block with copy button | Medium |

### 6.7 Theme Configuration

```css
/* ui/src/index.css -- base theme variables */

@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
    :root {
        --background: 0 0% 100%;
        --foreground: 240 10% 3.9%;
        --card: 0 0% 100%;
        --card-foreground: 240 10% 3.9%;
        --popover: 0 0% 100%;
        --popover-foreground: 240 10% 3.9%;
        --primary: 240 5.9% 10%;
        --primary-foreground: 0 0% 98%;
        --secondary: 240 4.8% 95.9%;
        --secondary-foreground: 240 5.9% 10%;
        --muted: 240 4.8% 95.9%;
        --muted-foreground: 240 3.8% 46.1%;
        --accent: 240 4.8% 95.9%;
        --accent-foreground: 240 5.9% 10%;
        --destructive: 0 84.2% 60.2%;
        --destructive-foreground: 0 0% 98%;
        --border: 240 5.9% 90%;
        --input: 240 5.9% 90%;
        --ring: 240 5.9% 10%;
        --radius: 0.5rem;
    }

    .dark {
        --background: 240 10% 3.9%;
        --foreground: 0 0% 98%;
        --card: 240 10% 3.9%;
        --card-foreground: 0 0% 98%;
        --popover: 240 10% 3.9%;
        --popover-foreground: 0 0% 98%;
        --primary: 0 0% 98%;
        --primary-foreground: 240 5.9% 10%;
        --secondary: 240 3.7% 15.9%;
        --secondary-foreground: 0 0% 98%;
        --muted: 240 3.7% 15.9%;
        --muted-foreground: 240 5% 64.9%;
        --accent: 240 3.7% 15.9%;
        --accent-foreground: 0 0% 98%;
        --destructive: 0 62.8% 30.6%;
        --destructive-foreground: 0 0% 98%;
        --border: 240 3.7% 15.9%;
        --input: 240 3.7% 15.9%;
        --ring: 240 4.9% 83.9%;
    }
}
```

### 6.8 UP5 Exit Criteria

- [ ] All 14 shadcn/ui components install via `npx shadcn@latest add` without errors
- [ ] All components render correctly in light mode
- [ ] All components render correctly in dark mode (class-based toggle)
- [ ] DataTable (TanStack Table + shadcn Table) renders 1000 rows under 100ms
- [ ] ResizablePanel works for horizontal split (Canvas + Chat layout)
- [ ] Command palette handles 50+ items with fuzzy search under 50ms
- [ ] Compatibility matrix is populated with actual test results
- [ ] Custom component list is documented (8 components, complexity assessed)
- [ ] Dark/light theme CSS variables are configured and switching works

---

## 7. UP6: Performance Budget Validation

### 7.1 Objective

Establish baseline bundle size measurements and verify that performance targets are achievable before building the full dashboard. Define dynamic import boundaries for code splitting.

### 7.2 Bundle Size Targets

| Stage | Target (gzipped) | Contents |
|-------|------------------|----------|
| Empty Vite + React shell | < 50 KB | React, React-DOM, router |
| + shadcn/ui components (14) | < 100 KB | + component library CSS/JS |
| + Core views (S1.3) | < 150 KB | + dashboard, agents, chat, sessions, tools |
| + Canvas (S2.1) | < 200 KB | + Canvas renderer, element types |
| Full app (S3.x) | < 300 KB | + charts, code editor (lazy loaded) |

### 7.3 Dynamic Import Strategy

Heavy dependencies are loaded on demand via `React.lazy()` and dynamic `import()`:

```typescript
// ui/src/routes/canvas.tsx
const CanvasRenderer = React.lazy(() => import("@/components/canvas/CanvasRenderer"));

// ui/src/routes/delegation.tsx (charts)
const TokenUsageChart = React.lazy(() => import("@/components/charts/TokenUsageChart"));

// ui/src/components/canvas/CodeEditorElement.tsx (Monaco)
const MonacoEditor = React.lazy(() => import("@monaco-editor/react"));
```

**Code splitting boundaries:**

| Chunk | Contents | Trigger | Estimated Size |
|-------|----------|---------|---------------|
| `vendor` | react, react-dom | Always loaded | ~40 KB gz |
| `router` | @tanstack/react-router | Always loaded | ~15 KB gz |
| `table` | @tanstack/react-table | Session/Memory views | ~15 KB gz |
| `query` | @tanstack/react-query | Always loaded | ~10 KB gz |
| `canvas` | Canvas renderer + elements | Canvas route | ~20 KB gz |
| `charts` | recharts/visx | Delegation view | ~40 KB gz |
| `monaco` | Monaco editor | Canvas code element | ~500 KB gz |

Monaco editor at ~500 KB gzipped is the single largest dependency. It MUST be lazy-loaded and should only download when a user opens a Canvas session that contains a code editor element. Alternative: use CodeMirror (~80 KB gz) as the default and offer Monaco as an opt-in.

### 7.4 Web Vitals Targets

| Metric | Target | Tool |
|--------|--------|------|
| First Contentful Paint (FCP) | < 1.5s on simulated 3G | Lighthouse |
| Largest Contentful Paint (LCP) | < 2.5s | Lighthouse |
| Cumulative Layout Shift (CLS) | < 0.1 | Lighthouse |
| Time to Interactive (TTI) | < 3.5s | Lighthouse |
| Lighthouse Performance Score | > 90 | Lighthouse CI |

### 7.5 Measurement Methodology

**Bundle Analysis:**

```bash
cd ui/
pnpm build
pnpm analyze  # Opens vite-bundle-visualizer
```

Record:
- Total bundle size (uncompressed and gzipped)
- Per-chunk sizes
- Tree map visualization screenshot
- Largest dependencies by size

**Lighthouse Audit:**

```bash
# Build and serve production bundle
pnpm build && pnpm preview

# Run Lighthouse (separate terminal)
npx lighthouse http://localhost:4173 \
    --chrome-flags="--headless" \
    --output=json \
    --output-path=./reports/lighthouse.json \
    --throttling.cpuSlowdownMultiplier=4 \
    --throttling.throughputKbps=1600
```

**WebSocket Latency Measurement:**

```typescript
// Measure round-trip time for ping/pong
const start = performance.now();
ws.send(JSON.stringify({ type: "command", command: "ping", data: {}, request_id: crypto.randomUUID() }));
// On pong receipt:
const latency = performance.now() - start;
console.log(`WS round-trip: ${latency}ms`);
```

**Target:** < 50ms on localhost, < 200ms on LAN.

### 7.6 Performance Monitoring Setup

```typescript
// ui/src/lib/performance.ts

import { onCLS, onFCP, onLCP, onTTFB } from "web-vitals";

export function initPerformanceMonitoring() {
    onCLS(console.log);
    onFCP(console.log);
    onLCP(console.log);
    onTTFB(console.log);
}
```

Add `web-vitals` (< 2 KB) to dependencies. Metrics are logged to console in development and can be sent to a backend endpoint in production.

### 7.7 UP6 Exit Criteria

- [ ] Empty Vite + React build bundle size measured and documented (target: < 50 KB gzipped)
- [ ] With shadcn components bundle size measured (target: < 100 KB gzipped)
- [ ] Dynamic import splitting verified (canvas, charts, monaco load on demand)
- [ ] `vite-bundle-visualizer` tree map saved to reports
- [ ] Lighthouse performance score measured (target: > 90)
- [ ] First Contentful Paint measured on simulated 3G (target: < 1.5s)
- [ ] WebSocket round-trip latency measured on localhost (target: < 50ms)
- [ ] `web-vitals` integration verified (metrics log to console)
- [ ] Code editor decision documented (Monaco vs CodeMirror with size tradeoff)

---

## 8. Go/No-Go Gate

### 8.1 P0 Checklist (Must Pass)

All items in this section must pass for a **Go** decision.

| # | Validation Item | Pass Criteria | Fallback if Failed | Owner |
|---|----------------|---------------|-------------------|-------|
| 1 | Axum dependency addition | `cargo check --features ui` passes, no version conflicts | Pin specific compatible versions, or vendor `axum` as workspace dep | Backend |
| 2 | Existing build unaffected | `cargo check -p clawft-services` (no ui feature) still passes | Feature gate issue; fix cfg attributes | Backend |
| 3 | Health endpoint works | `GET /api/health` returns 200 JSON | Debug Axum router composition; Axum is well-documented | Backend |
| 4 | Bearer auth rejects invalid | Requests without valid token return 401 | Simplify to single static token comparison | Backend |
| 5 | WebSocket upgrade works | WS connection established alongside Axum REST routes | Use separate port (18791) for WS only | Backend |
| 6 | WS subscribe/unsubscribe | Client subscribes to topic, receives events for that topic only | Simplify to broadcast-all model for v1 | Backend |
| 7 | Frontend `pnpm install` | Zero errors, no unresolved peer deps | Pin all versions explicitly in package.json | Frontend |
| 8 | Frontend `pnpm dev` | Vite starts on :5173, HMR works | Fallback to Create React App if Vite has blockers | Frontend |
| 9 | MSW mock works | API requests intercepted, mock data returned | Use JSON fixture files with fetch mock instead of MSW | Frontend |
| 10 | shadcn/ui installs | All 14 components install, render in both themes | Use manual component copies from shadcn GitHub | Frontend |
| 11 | TanStack Router works | File-based routing, navigation between 2+ routes | Fall back to React Router v7 | Frontend |
| 12 | `pnpm build` succeeds | Production build completes, `dist/` created | Debug Vite config; ensure TypeScript strict mode passes | Frontend |
| 13 | Bundle under 200 KB gz | Empty shell with router + shadcn < 200 KB gzipped | Remove unused components, audit imports | Frontend |
| 14 | WS protocol document | All 15 event types + 5 command types documented with JSON schema | Defer less critical event types to sprint S1.1 | Both |
| 15 | Auth flow documented | Token format, exchange flow, CLI command specified | Simplify to static token file (no exchange flow) | Both |

### 8.2 P1 Checklist (Should Pass)

Items that should pass but have documented fallbacks.

| # | Validation Item | Pass Criteria | Fallback if Failed | Owner |
|---|----------------|---------------|-------------------|-------|
| 16 | `rust-embed` static serving | Embedded files served at root path | Serve from disk via `tower-http::services::ServeDir` | Backend |
| 17 | DataTable 1000 rows < 100ms | TanStack Table renders without jank | Enable row virtualization via `@tanstack/react-virtual` | Frontend |
| 18 | Command palette 50 items | Fuzzy search responds under 50ms | Reduce to 30 items, simplify search to prefix match | Frontend |
| 19 | ResizablePanel works | Drag resize for Canvas+Chat layout | Use CSS grid with manual resize handle | Frontend |
| 20 | Lighthouse score > 90 | Production build scores 90+ on desktop | Optimize in S3.5 (production hardening) | Frontend |
| 21 | WebSocket 1000 msg/sec | Throughput benchmark passes on localhost | Acceptable at 500 msg/sec; optimize in S3.5 | Backend |
| 22 | Dynamic import splitting | Canvas/charts/monaco load on demand | Accept larger initial bundle; optimize later | Frontend |
| 23 | Proxy config works | Vite proxies /api and /ws to backend | Use explicit API URL in env var | Frontend |
| 24 | CI config validated | lint + type-check + test + build all pass in CI | Run locally only until CI is configured | Frontend |
| 25 | Dockerfile builds | Multi-stage build produces nginx image | Defer containerization to S3.5 | Frontend |

### 8.3 Decision Matrix

| Outcome | Criteria | Action |
|---------|----------|--------|
| **Go** | All 15 P0 items pass | Proceed to S1.1 + S1.2 (Week 1) |
| **Conditional Go** | All P0 pass, 1-3 P1 items remain | Proceed to S1.1 + S1.2, carry remaining P1 items as Week 1 tasks |
| **No-Go: Backend** | P0 items 1-6 (Axum/WS) fail | Investigate alternative: use `actix-web` or standalone HTTP server process |
| **No-Go: Frontend** | P0 items 7-13 (Vite/React) fail | Investigate alternative: Svelte + SvelteKit or plain HTML+HTMX |
| **No-Go: Protocol** | P0 items 14-15 (protocol/auth) fail | Extend Week 0 by 2 days to complete design work |

**Decision authority:** Project lead
**Report location:** `docs/ui/week0-validation-report.md`

---

## 9. Week 0 Schedule

### Day 1 (Monday): UP1 + UP4 (Parallel)

**Backend track (UP1 -- Axum API Layer):**

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Add axum, axum-extra, tower-http, rust-embed to `clawft-services/Cargo.toml` behind `ui` feature | Updated Cargo.toml |
| Morning | Run `cargo check -p clawft-services --features ui` and fix any version conflicts | Successful check |
| Morning | Run `cargo check -p clawft-services` (without ui) to verify no regressions | Successful check |
| Afternoon | Implement prototype `GET /api/health` endpoint in `clawft-services/src/api/mod.rs` | Working endpoint |
| Afternoon | Implement Bearer token middleware prototype in `clawft-services/src/api/auth.rs` | Middleware that rejects invalid tokens |
| Afternoon | Test: `curl -H "Authorization: Bearer valid" http://localhost:18790/api/health` returns 200 | Manual test pass |
| Afternoon | Test: `curl http://localhost:18790/api/health` without token returns 401 | Manual test pass |

**Frontend track (UP4 -- Frontend Scaffold):**

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | `pnpm create vite ui -- --template react-ts` | Initialized project |
| Morning | Install all dependencies from package.json spec | `pnpm-lock.yaml` |
| Morning | Configure Tailwind CSS v4 with `@tailwindcss/vite` plugin | Working CSS |
| Morning | Initialize shadcn/ui: `npx shadcn@latest init` | `components.json` |
| Afternoon | Set up TanStack Router with file-based routing (root + index + agents) | Working router |
| Afternoon | Set up MSW with handlers for health, agents, sessions, tools | Mock API working |
| Afternoon | Configure Vite proxy for /api and /ws | `vite.config.ts` |
| Afternoon | Verify `pnpm dev` with `VITE_MOCK_API=true` | HMR working, mocks active |

### Day 2 (Tuesday): UP2 + UP5 (Parallel)

**Backend track (UP2 -- WebSocket Protocol):**

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Write WebSocket protocol document (all 15 event types) | Protocol spec |
| Morning | Write client-to-server command types (5 types) | Protocol spec |
| Morning | Define topic namespace and subscription rules | Protocol spec |
| Afternoon | Implement prototype WS upgrade handler in Axum | WS connects |
| Afternoon | Implement subscribe/unsubscribe command handling | Topic filtering works |
| Afternoon | Implement SubscriptionManager with publish/subscribe | Manager tested |
| Afternoon | Run throughput benchmark (target: 1000 msg/sec) | Benchmark result |

**Frontend track (UP5 -- shadcn Audit):**

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Install all 14 shadcn components | Components in ui/src/components/ui/ |
| Morning | Create test page rendering each component in light mode | Visual verification |
| Morning | Toggle dark mode class, verify all components in dark mode | Visual verification |
| Afternoon | Build DataTable prototype with TanStack Table + 1000 rows | Performance measurement |
| Afternoon | Build ResizablePanel prototype (horizontal split) | Working split view |
| Afternoon | Build Command palette prototype with 50+ items | Fuzzy search working |
| Afternoon | Fill compatibility matrix with actual results | Matrix documented |

### Day 3 (Wednesday): UP3 + UP6 (Parallel)

**Backend track (UP3 -- Authentication):**

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Finalize token format specification (opaque, prefixed) | Token spec |
| Morning | Implement token generation (32 random bytes, hex-encoded) | Token generator |
| Morning | Implement token file storage (`~/.clawft/ui-token`, 0600 perms) | File I/O |
| Morning | Implement one-time URL token exchange endpoint | POST /api/auth/exchange |
| Afternoon | Write GatewayConfig extensions spec (cors_origins, token_ttl_hours) | Config spec |
| Afternoon | Implement `weft ui` CLI command prototype (generate token + open browser) | CLI command |
| Afternoon | Test full flow: weft ui -> browser opens -> token exchange -> Bearer API | Integration test |

**Frontend track (UP6 -- Performance Budget):**

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Run `pnpm build` and measure empty shell bundle size | Measurement |
| Morning | Run `pnpm analyze` (vite-bundle-visualizer) and save tree map | Tree map report |
| Morning | Add web-vitals and verify metrics logging | web-vitals working |
| Afternoon | Configure dynamic imports for future heavy dependencies | Import boundaries |
| Afternoon | Run Lighthouse audit on production build (`pnpm preview`) | Lighthouse report |
| Afternoon | Measure WebSocket latency with ping/pong prototype | Latency measurement |
| Afternoon | Document code editor decision (Monaco vs CodeMirror) | Decision documented |

### Day 4 (Thursday): Integration Testing

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Connect frontend dev server to backend prototype (disable mocks) | Real API calls working |
| Morning | Verify: Dashboard loads agent data from real backend | GET /api/agents works |
| Morning | Verify: WebSocket connects and receives real events | WS subscription works |
| Morning | Verify: Bearer auth flow works end-to-end | Token exchange working |
| Afternoon | Test: `weft ui` generates token, opens browser, SPA loads, exchanges token | Full flow working |
| Afternoon | Test: `pnpm build` output serves correctly via rust-embed prototype | Static serving works |
| Afternoon | Run full test suite: `pnpm lint && pnpm type-check && pnpm test` | All pass |
| Afternoon | Run `cargo test -p clawft-services --features ui` | All pass |

### Day 5 (Friday): Go/No-Go Review

| Time | Task | Deliverable |
|------|------|-------------|
| Morning | Fill all exit criteria checklists (UP1-UP6) | Completed checklists |
| Morning | Write validation report for each UP task | 6 task reports |
| Morning | Document any issues found and their resolutions/fallbacks | Issue log |
| Afternoon | Review Go/No-Go gate checklist | Gate assessment |
| Afternoon | Write week0 validation summary report | `docs/ui/week0-validation-report.md` |
| Afternoon | If Go: finalize sprint S1.1/S1.2 task breakdown and assignments | Sprint plan ready |
| Afternoon | If No-Go: document blockers and revised timeline | Revised plan |

### Parallelism Notes

- **Day 1:** UP1 and UP4 are fully independent (backend vs frontend). Can be done by one person if needed (morning: backend, afternoon: frontend).
- **Day 2:** UP2 and UP5 are mostly independent. UP2 backend WS prototype can inform UP5's ws-client testing.
- **Day 3:** UP3 and UP6 are independent. UP3 backend auth prototype feeds Day 4 integration.
- **Day 4:** Integration requires BOTH backend and frontend prototypes to be working. This is the critical path -- if Day 1-3 tasks slip, Day 4 integration is at risk.
- **Day 5:** Documentation and review. Can start early if Day 4 finishes cleanly.

**Estimated total effort:**
- Backend developer: 5 days (UP1: 1d, UP2: 1d, UP3: 1d, integration: 1d, review: 1d)
- Frontend developer: 5 days (UP4: 1d, UP5: 1d, UP6: 1d, integration: 1d, review: 1d)
- Total: 10 person-days across 5 calendar days (2 developers in parallel)
- Single developer: 8-10 calendar days (sequential with some overlap)

---

## 10. Combined Exit Criteria

All six UP tasks must pass before UI sprints begin. This is the final gate checklist.

### UP1: Axum REST API Layer Validation

- [ ] `axum`, `axum-extra`, `tower-http`, `rust-embed` added to `clawft-services/Cargo.toml` behind `ui` feature
- [ ] `cargo check -p clawft-services --features ui` passes
- [ ] `cargo check -p clawft-services` (without `ui`) still passes
- [ ] Prototype `GET /api/health` returns JSON response
- [ ] Bearer token middleware rejects unauthenticated requests with 401
- [ ] Bearer token middleware allows authenticated requests
- [ ] `rust-embed` can embed and serve static HTML
- [ ] Port decision documented (same port 18790, rationale recorded)

### UP2: WebSocket Protocol Design

- [ ] Protocol document complete with 15 server-to-client event types
- [ ] Protocol document complete with 5 client-to-server command types
- [ ] Topic namespace defined (15 topics)
- [ ] Prototype WS handler accepts connections
- [ ] Subscribe/unsubscribe filtering works correctly
- [ ] Throughput benchmark: 1000 msg/sec on localhost
- [ ] Reconnection strategy documented
- [ ] Server heartbeat / idle timeout specified

### UP3: Authentication Design

- [ ] Token format specified (opaque, 64 hex chars, ot_/st_ prefixed)
- [ ] Auth flow documented (weft ui -> generate -> exchange -> Bearer)
- [ ] One-time URL token exchange endpoint specified
- [ ] Token file storage specified (~/.clawft/ui-token, 0600)
- [ ] GatewayConfig extensions specified (cors_origins, token_ttl_hours)
- [ ] Bearer middleware uses constant-time comparison
- [ ] `weft ui` CLI command specified
- [ ] Token format decision documented (opaque vs JWT, rationale)

### UP4: Frontend Scaffold Validation

- [ ] `pnpm install` succeeds without errors
- [ ] `pnpm dev` starts on :5173 with HMR
- [ ] MSW mocks work with `VITE_MOCK_API=true`
- [ ] TanStack Router file-based routing works
- [ ] shadcn/ui initialized, at least one component renders
- [ ] `pnpm build` produces dist/ under 200 KB gzipped
- [ ] `pnpm lint` and `pnpm type-check` pass
- [ ] Vite proxy configuration works for /api and /ws

### UP5: shadcn/ui Component Audit

- [ ] All 14 components install successfully
- [ ] All components render in light mode
- [ ] All components render in dark mode
- [ ] DataTable with 1000 rows renders under 100ms
- [ ] ResizablePanel works for split view
- [ ] Command palette handles 50+ items
- [ ] Compatibility matrix populated with actual results
- [ ] Custom component list documented (8 components)

### UP6: Performance Budget Validation

- [ ] Empty shell bundle < 50 KB gzipped (measured)
- [ ] Shell + shadcn < 100 KB gzipped (measured)
- [ ] Dynamic import splitting verified
- [ ] Bundle analysis tree map saved
- [ ] Lighthouse score > 90 (measured)
- [ ] FCP < 1.5s on simulated 3G (measured)
- [ ] WebSocket latency < 50ms on localhost (measured)
- [ ] web-vitals integration verified
- [ ] Code editor decision documented (Monaco vs CodeMirror)

### Go/No-Go Decision

| Outcome | Criteria | Action |
|---------|----------|--------|
| **Go** | All UP1-UP6 exit criteria pass | Proceed to S1.1 + S1.2 (Week 1) |
| **Conditional Go** | All P0 items pass, 1-3 P1 items remain | Proceed, carry remaining items into Week 1 |
| **No-Go** | Any P0 item fails without fallback | Investigate alternatives, extend Week 0, revise sprint plan |

**Report location:** `docs/ui/week0-validation-report.md`
**Decision authority:** Project lead
