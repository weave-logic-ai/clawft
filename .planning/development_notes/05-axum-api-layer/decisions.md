# Axum API Layer -- Decisions

**Scope**: REST + WebSocket API for the ClawFT web dashboard
**Module**: `crates/clawft-services/src/api/`

---

### D-1: Arc-wrapping ToolRegistry and SessionManager
**Date**: 2026-02-26
**Status**: Accepted
**Context**: `AppContext::into_agent_loop()` consumes `self`, but the API server needs concurrent access to the same `ToolRegistry`, `SessionManager`, `MemoryStore`, and `SkillsLoader` instances. The API layer starts before the agent loop begins processing messages.
**Decision**: Changed `AppContext` fields from owned values to `Arc<>` wrappers. The `ToolRegistry`, `SessionManager`, `MemoryStore`, and `SkillsLoader` are cloned (via `Arc::clone`) into bridge structs before `into_agent_loop()` consumes the context. The `tools_mut()` accessor uses `Arc::get_mut().expect()` which panics if any other `Arc` references exist, so all tool registration must complete before the API bridges are constructed.
**Consequences**: Tool registration is confined to a single-owner phase during bootstrap. Attempting to register tools after the API bridges are created will panic. This is an acceptable constraint because tool registration is a startup-only activity.

---

### D-2: Trait-object bridge pattern
**Date**: 2026-02-26
**Status**: Accepted
**Context**: Core services like `SessionManager<P>` and `SkillsLoader<P>` carry a `Platform` generic (`P: Platform`). Axum handlers use `State<ApiState>` which must be `Clone + Send + Sync` with no generic parameters. Propagating `<P: Platform>` through the API layer would infect every handler signature and prevent using `dyn` dispatch.
**Decision**: Introduced bridge structs (`ToolBridge`, `SessionBridge<P>`, `AgentBridge`, `BusBridge`, `SkillBridge<P>`, `MemoryBridge<P>`, `ConfigBridge`, `ChannelBridge`) that implement API-layer access traits (`ToolRegistryAccess`, `SessionAccess`, `AgentAccess`, etc.). Each bridge wraps an `Arc`-reference to the real core service. The `<P>` generic is erased at the `Arc<dyn Trait>` boundary -- `ApiState` stores `Arc<dyn SessionAccess>`, never `Arc<SessionBridge<NativePlatform>>`.
**Consequences**: The entire `api/` module tree is free of `<P: Platform>` generics. Only `bridge.rs` knows about `Platform`. Adding a new platform (e.g., WASM) requires only a new bridge constructor, not changes to handlers. The tradeoff is an extra allocation layer and dynamic dispatch overhead, both negligible for API handler latency.

---

### D-3: block_in_place for async-to-sync bridging
**Date**: 2026-02-26
**Status**: Accepted
**Context**: The `*Access` traits (`SessionAccess`, `SkillAccess`, `MemoryAccess`) are sync (not `async fn`) for handler ergonomics. However, the underlying `SessionManager`, `SkillsLoader`, and `MemoryStore` methods are async. Calling `Handle::current().block_on()` directly from a tokio worker thread panics with "Cannot block the current thread from within a runtime."
**Decision**: Use `tokio::task::block_in_place()` to move the current thread out of the tokio worker pool, then call `Handle::current().block_on()` inside the closure. This is the standard pattern for running async code from a sync context within a multi-threaded tokio runtime.
**Consequences**: Requires the `multi_thread` tokio runtime (not `current_thread`). While `block_in_place` temporarily removes a worker thread from the pool, the overhead is acceptable for API request latencies in the low hundreds of milliseconds. Each bridge method follows the same `block_in_place(|| Handle::current().block_on(async { ... }))` pattern for consistency.

---

### D-4: TopicBroadcaster with tokio broadcast channels
**Date**: 2026-02-26
**Status**: Accepted
**Context**: The WebSocket handler needs to forward real-time events (agent status, new messages, pipeline stages) to connected clients. Multiple clients can subscribe to the same topic. The gateway dispatch loop and chat handlers need to publish events without knowing about individual WebSocket connections.
**Decision**: `TopicBroadcaster` manages a `HashMap<String, broadcast::Sender<String>>` behind an `Arc<RwLock<_>>`. Each topic gets a `broadcast::channel(256)` on first use. WebSocket handlers subscribe by calling `broadcaster.subscribe(topic)` to get a `broadcast::Receiver`. Each subscription spawns a dedicated forwarding task that reads from the receiver and writes to the client's `WebSocket` sender (wrapped in `Arc<Mutex<_>>`). Lagged messages are silently skipped.
**Consequences**: Capacity of 256 messages per topic means a slow consumer that falls more than 256 messages behind will miss events (the `RecvError::Lagged` branch skips them). This is intentional for real-time UIs where stale data is worse than missing data. Topic channels are created lazily and never garbage-collected, which is fine for the small set of well-known topic names.

---

### D-5: Feature flag chain for the `api` feature
**Date**: 2026-02-26
**Status**: Accepted
**Context**: The Axum API is optional -- the gateway can run with only messaging channels and no HTTP server. Dependencies like `axum`, `tower-http`, and `futures-util` should not be compiled when the API is not enabled.
**Decision**: The `api` feature flag flows from `clawft-cli` to `clawft-services`. In `clawft-services/Cargo.toml`, `api = ["dep:axum", "dep:axum-extra", "dep:tower-http", "dep:futures-util", "dep:clawft-core", "dep:clawft-platform"]`. The `clawft-core` and `clawft-platform` dependencies are optional and only pulled in by the `api` feature because `bridge.rs` imports concrete types from those crates. Without the `api` feature, `clawft-services` only depends on `clawft-types` for its MCP, cron, and delegation modules.
**Consequences**: Building without `--features api` produces a smaller binary with no HTTP server capability. The `api/` module is conditionally compiled. The `clawft-cli` crate gates the `weft ui` and `weft gateway` (with `api_enabled`) commands behind the `api` feature.

---

### D-6: Gateway API-only mode
**Date**: 2026-02-26
**Status**: Accepted
**Context**: The gateway command previously bailed out if no messaging channels (Telegram, Slack, Discord) were configured, since without channels there was nothing to dispatch. However, users may want to run the API server alone for web-only usage (chat via REST/WebSocket, no external channels).
**Decision**: Relaxed the "no channels = bail" check. When `config.gateway.api_enabled` is true, the Axum server starts even if no messaging channels are configured. The gateway dispatch loop still runs but has no channel plugins to poll; the API endpoints and WebSocket remain fully functional for web-originated chat sessions.
**Consequences**: `weft gateway` with `api_enabled = true` and all channels disabled is a valid deployment mode. The dispatch loop is idle but harmless. The web channel (`"web"` type) is always added to the channel list when the API is enabled.

---

### D-7: Auth middleware disabled by default
**Date**: 2026-02-26
**Status**: Accepted
**Context**: An auth middleware and `TokenStore` exist in `api/auth.rs` that can validate Bearer tokens on API requests. However, the UI does not yet have a login flow, and requiring tokens during local development adds friction.
**Decision**: The auth middleware is defined but NOT wired into the Axum router. A comment in `build_router()` shows how to enable it (wrapping the `/api` nest with `axum::middleware::from_fn_with_state(state.clone(), auth::auth_middleware)`). Left disabled until the UI implements a login page.
**Consequences**: All API endpoints are unauthenticated in the current build. The `POST /api/auth/token` endpoint works and creates tokens, but they are not required. This is acceptable for local development but must be addressed before any network-exposed deployment.

---

### D-8: Stub routes for cron and voice
**Date**: 2026-02-26
**Status**: Accepted
**Context**: The UI has pages for cron job management and voice settings that expect API endpoints to exist. Full backend implementations for these features are deferred.
**Decision**: `cron_api.rs` and `voice_api.rs` define route trees, request/response types, and handler functions, but the handlers return placeholder data (empty lists, default settings, `{ "success": true }` for mutations). This allows the UI to render without errors while the backend implementations are built out.
**Consequences**: The API contract is defined and stable. Frontend development can proceed against the stubs. The handlers are clearly marked for future implementation. No data is persisted by stub handlers.

---

### D-9: SSE streaming via unfold
**Date**: 2026-02-26
**Status**: Accepted
**Context**: The chat page needs real-time streaming of agent responses. Axum's `Sse` type requires an `impl Stream<Item = Result<Event, Infallible>>`. The event source is a `broadcast::Receiver<String>` from `TopicBroadcaster`.
**Decision**: Use `futures_util::stream::unfold` to convert the `broadcast::Receiver` into an SSE-compatible `Stream`. The unfold state is the receiver itself. Each `recv().await` yields an `Ok(Event::default().data(msg))`. `RecvError::Lagged` continues the loop (skips missed messages). `RecvError::Closed` returns `None` to terminate the stream.
**Consequences**: The endpoint `GET /api/sessions/{key}/stream` returns an `Sse` response that the browser can consume with `EventSource`. Lagged messages are silently dropped, matching the WebSocket behavior. The stream terminates cleanly when the broadcast channel is closed (e.g., server shutdown).

---

### D-10: SPA fallback for production
**Date**: 2026-02-26
**Status**: Accepted
**Context**: In production, the Vite-built frontend (static files) needs to be served from the same port as the API. Client-side routing requires that any path not matching `/api` or `/ws` returns `index.html`.
**Decision**: When the `--ui-dir` CLI argument is provided, a `tower_http::services::ServeDir` fallback is added to the Axum router with `append_index_html_on_directories(true)`. This serves static files from the specified directory and falls back to `index.html` for unmatched paths, enabling SPA routing.
**Consequences**: A single `weft ui --ui-dir ./ui/dist` command serves both the API and the frontend on port 18789. During development, the Vite dev server handles static assets and proxies `/api` and `/ws` to the Axum backend, so the fallback is unused. The fallback is only active when `--ui-dir` is explicitly provided.
