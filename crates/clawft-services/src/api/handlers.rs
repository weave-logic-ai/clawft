//! HTTP request handlers for the REST API.

use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};

use super::ApiState;

/// Build all API routes.
pub fn api_routes() -> Router<ApiState> {
    Router::new()
        // Agent endpoints
        .route("/agents", get(list_agents))
        .route("/agents/{name}", get(get_agent))
        // Session endpoints
        .route("/sessions", get(list_sessions))
        .route("/sessions/{key}", get(get_session))
        .route("/sessions/{key}", delete(delete_session))
        // Tool endpoints
        .route("/tools", get(list_tools))
        .route("/tools/{name}/schema", get(get_tool_schema))
        // Auth
        .route("/auth/token", post(create_token))
        // Health check
        .route("/health", get(health_check))
        // Delegation monitoring
        .merge(super::delegation::delegation_routes())
        // System monitoring
        .merge(super::monitoring::monitoring_routes())
}

async fn list_agents(State(state): State<ApiState>) -> Json<Vec<super::AgentInfo>> {
    Json(state.agents.list_agents())
}

async fn get_agent(
    State(state): State<ApiState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Option<super::AgentInfo>> {
    Json(state.agents.get_agent(&name))
}

async fn list_sessions(State(state): State<ApiState>) -> Json<Vec<super::SessionInfo>> {
    Json(state.sessions.list_sessions())
}

async fn get_session(
    State(state): State<ApiState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Json<Option<super::SessionDetail>> {
    Json(state.sessions.get_session(&key))
}

async fn delete_session(
    State(state): State<ApiState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Json<bool> {
    Json(state.sessions.delete_session(&key))
}

async fn list_tools(State(state): State<ApiState>) -> Json<Vec<super::ToolInfo>> {
    Json(state.tools.list_tools())
}

async fn get_tool_schema(
    State(state): State<ApiState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<Option<serde_json::Value>> {
    Json(state.tools.tool_schema(&name))
}

async fn create_token(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let token = state.auth.generate_token(86400); // 24h TTL
    Json(serde_json::json!({ "token": token }))
}

/// Server start time, set once at process start.
static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Returns basic health status, version, and uptime.
async fn health_check() -> Json<serde_json::Value> {
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_secs = start.elapsed().as_secs();
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime_secs
    }))
}

// TODO: Add Content-Security-Policy (CSP) middleware via tower layer.
// Example: tower_http::set_header::SetResponseHeaderLayer for CSP headers.
//
// TODO: Add rate limiting middleware via tower::limit::RateLimitLayer
// or a dedicated crate like tower-governor for production use.
// Configuration should be per-endpoint with sensible defaults:
//   - /api/auth/token: 5 req/min
//   - /api/delegation/*: 60 req/min
//   - /api/monitoring/*: 30 req/min
