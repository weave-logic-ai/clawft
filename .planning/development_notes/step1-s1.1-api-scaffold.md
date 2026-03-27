# S1.1 - Backend API Foundation (Axum REST Scaffold)

**Date**: 2026-02-24
**Branch**: `feature/three-workstream-implementation`
**Status**: Complete

## Files Created

| File | Purpose |
|------|---------|
| `crates/clawft-services/src/api/mod.rs` | Module root: trait definitions, data types, `build_router()` |
| `crates/clawft-services/src/api/auth.rs` | In-memory `TokenStore` with TTL-based token validation |
| `crates/clawft-services/src/api/handlers.rs` | REST handlers for agents, sessions, tools, auth |
| `crates/clawft-services/src/api/ws.rs` | WebSocket upgrade handler with subscribe/unsubscribe/ping |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-services/Cargo.toml` | Added `api` feature flag and axum/tower-http dependencies |
| `crates/clawft-services/src/lib.rs` | Added `#[cfg(feature = "api")] pub mod api;` |
| `Cargo.toml` (workspace) | Added `default-features = false` to `clawft-types` workspace dep (fixes edition 2024 compatibility with `clawft-platform`) |

## API Endpoint Summary

All REST endpoints are mounted under `/api`:

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/agents` | `list_agents` | List all registered agents |
| GET | `/api/agents/{name}` | `get_agent` | Get agent details by name |
| GET | `/api/sessions` | `list_sessions` | List all active sessions |
| GET | `/api/sessions/{key}` | `get_session` | Get session detail with messages |
| DELETE | `/api/sessions/{key}` | `delete_session` | Delete a session |
| GET | `/api/tools` | `list_tools` | List all registered tools |
| GET | `/api/tools/{name}/schema` | `get_tool_schema` | Get JSON schema for a tool |
| POST | `/api/auth/token` | `create_token` | Generate a 24h API token |
| GET | `/ws` | `ws_handler` | WebSocket upgrade for real-time events |

### WebSocket Commands

| Command | Response | Description |
|---------|----------|-------------|
| `{"type": "subscribe", "topic": "..."}` | `{"type": "subscribed", "topic": "..."}` | Subscribe to event topic |
| `{"type": "unsubscribe", "topic": "..."}` | `{"type": "unsubscribed", "topic": "..."}` | Unsubscribe from topic |
| `{"type": "ping"}` | `{"type": "pong"}` | Keep-alive ping |

## Dependency Versions

| Crate | Version | Notes |
|-------|---------|-------|
| `axum` | 0.8.x (resolved 0.8.8) | Features: ws, json, macros |
| `axum-extra` | 0.10.x (resolved 0.10.3) | Features: typed-header; compatible with axum 0.8 |
| `tower-http` | 0.6.x (resolved 0.6.8) | Features: cors, fs, trace |
| `futures-util` | 0.3 (workspace) | Already a workspace dependency |
| `uuid` | 1 (workspace) | Already in clawft-services deps |

Note: `tokio-tungstenite` was listed in the original plan but is not needed -- axum 0.8 bundles its own WebSocket support via the `ws` feature using `tungstenite` directly.

## Architecture Decisions

- **Trait-based decoupling**: The API module defines four traits (`ToolRegistryAccess`, `SessionAccess`, `AgentAccess`, `BusAccess`) so handlers are decoupled from concrete platform types. Implementors wire these up when constructing `ApiState`.
- **Feature-gated**: The entire `api` module is behind `#[cfg(feature = "api")]` so it adds zero cost to builds that don't need the web dashboard.
- **CORS**: Permissive by default; configurable via `cors_origins` parameter to `build_router()`.
- **Auth**: Simple in-memory token store; suitable for local development. Production would swap for JWT or session-based auth.

## Issues Encountered

1. **Workspace manifest error (pre-existing)**: `clawft-platform` uses `default-features = false` on `clawft-types`, which is disallowed in Rust 2024 edition unless the workspace root also sets `default-features = false`. Fixed by adding `default-features = false` to the workspace-level `clawft-types` dependency. This is a no-op for most crates since `clawft-types`' "native" feature only controls `dirs` for tilde expansion, which no downstream crate uses directly.

2. **axum-extra version**: The task spec suggested `axum-extra = "0.10"`, which resolved to 0.10.3 and is the correct pairing for axum 0.8. The latest axum-extra (0.12.5) targets a newer axum version.

## Verification

```
cargo check -p clawft-services --features api   # OK
cargo check -p clawft-services                   # OK (default, no api)
cargo test -p clawft-services --features api     # 259 passed
cargo test -p clawft-services                    # 259 passed
```
