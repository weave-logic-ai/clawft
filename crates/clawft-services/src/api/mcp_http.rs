//! MCP-over-HTTP endpoint for remote MCP clients.
//!
//! Serves the MCP tool surface (the same [`McpDispatcher`] pipeline the
//! stdio `weft mcp-server` uses) at `POST /mcp` using the Streamable
//! HTTP transport in stateless JSON mode: each request body carries one
//! JSON-RPC message (or a batch array), and the response body carries
//! the JSON-RPC response(s). Notifications are acknowledged with `202
//! Accepted` and no body. `GET /mcp` returns `405` — this server does
//! not offer an SSE event stream; every interaction is request/response.
//!
//! The primary consumer is the xAI Grok Voice Agent API, whose sessions
//! can be configured with a remote MCP tool
//! (`{"type": "mcp", "server_url": "https://…/mcp", "authorization":
//! "Bearer …"}`) so the voice agent can submit jobs, query job status,
//! and read sensors on a WeftOS node. Any spec-compliant Streamable
//! HTTP MCP client works the same way.
//!
//! # Auth
//!
//! The endpoint is gated by a static bearer token fixed at construction
//! time — deliberately *not* the [`super::auth::TokenStore`] used by
//! `/api/*`, because remote MCP clients are configured once with a
//! long-lived credential and cannot run the `/api/auth/token` issuance
//! flow. [`McpEndpoint::new`] refuses an empty token, so an
//! unauthenticated `/mcp` cannot be constructed.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::Value;
use tracing::{debug, warn};

use super::ApiState;
use crate::mcp::composite::CompositeToolProvider;
use crate::mcp::dispatch::McpDispatcher;
use crate::mcp::middleware::Middleware;

/// State for the `/mcp` endpoint: the shared dispatcher plus the static
/// bearer token that gates every request.
pub struct McpEndpoint {
    dispatcher: McpDispatcher,
    token: String,
}

impl McpEndpoint {
    /// Build an endpoint from a composite provider, middleware pipeline,
    /// and bearer token.
    ///
    /// Fails when `token` is empty: the endpoint may be exposed to the
    /// public internet (e.g. through a Tailscale Funnel or Cloudflare
    /// Tunnel so a cloud voice agent can reach it), so serving it
    /// without auth is never acceptable.
    pub fn new(
        provider: CompositeToolProvider,
        middlewares: Vec<Box<dyn Middleware>>,
        token: String,
    ) -> Result<Self, &'static str> {
        if token.trim().is_empty() {
            return Err("MCP endpoint bearer token must not be empty");
        }
        let mut dispatcher = McpDispatcher::new(provider);
        for mw in middlewares {
            dispatcher.add_middleware(mw);
        }
        Ok(Self { dispatcher, token })
    }

    /// Check an `Authorization: Bearer <token>` header value against the
    /// configured token, in constant time over the supplied bytes.
    fn authorize(&self, headers: &HeaderMap) -> bool {
        let supplied = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        constant_time_eq(supplied.as_bytes(), self.token.as_bytes())
    }
}

/// Constant-time byte comparison (length mismatch short-circuits, which
/// leaks only the token length).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Build the `/mcp` routes. Mounted outside the `/api` auth gate — the
/// endpoint enforces its own static-token auth in the handler.
pub fn mcp_routes() -> Router<ApiState> {
    Router::new().route("/mcp", post(handle_mcp).fallback(method_not_allowed))
}

async fn method_not_allowed() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "POST")],
        "MCP endpoint accepts POST only (stateless Streamable HTTP; no SSE stream)",
    )
        .into_response()
}

async fn handle_mcp(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let Some(endpoint) = state.mcp.as_ref() else {
        return (StatusCode::NOT_FOUND, "MCP endpoint not enabled").into_response();
    };

    if !endpoint.authorize(&headers) {
        warn!("rejected /mcp request with missing or invalid bearer token");
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            "invalid or missing bearer token",
        )
            .into_response();
    }

    let msg: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("invalid JSON body: {e}")).into_response();
        }
    };

    match msg {
        Value::Array(batch) => {
            let mut responses = Vec::new();
            for item in &batch {
                let resp = dispatch_stateless(endpoint, item).await;
                publish_activity(&state, item, resp.as_ref()).await;
                if let Some(resp) = resp {
                    responses.push(resp);
                }
            }
            if responses.is_empty() {
                StatusCode::ACCEPTED.into_response()
            } else {
                (StatusCode::OK, Json(Value::Array(responses))).into_response()
            }
        }
        single => {
            let resp = dispatch_stateless(endpoint, &single).await;
            publish_activity(&state, &single, resp.as_ref()).await;
            match resp {
                Some(resp) => (StatusCode::OK, Json(resp)).into_response(),
                None => StatusCode::ACCEPTED.into_response(),
            }
        }
    }
}

/// WebSocket topic that `/mcp` tool-call activity is published on.
///
/// The `/voice` client page subscribes to this over the gateway's `/ws`
/// so the user can *see* what the voice agent is doing — which tool ran,
/// with what arguments, and what came back — not just hear the answer.
pub const ACTIVITY_TOPIC: &str = "voice-activity";

/// Longest args/result preview forwarded to activity subscribers.
const PREVIEW_LIMIT: usize = 600;

/// Publish a `tools/call` event to [`ACTIVITY_TOPIC`]. Non-tool-call
/// messages (initialize, tools/list, notifications) are not published.
async fn publish_activity(state: &ApiState, msg: &Value, resp: Option<&Value>) {
    if msg.get("method").and_then(|m| m.as_str()) != Some("tools/call") {
        return;
    }
    let params = msg.get("params");
    let tool = params
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("(unknown)");
    let args_preview = params
        .and_then(|p| p.get("arguments"))
        .map(|a| truncate(&a.to_string(), PREVIEW_LIMIT))
        .unwrap_or_default();

    let result = resp.and_then(|r| r.get("result"));
    let is_error = result
        .and_then(|r| r.get("isError"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);
    let result_preview = result
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|t| truncate(&t, PREVIEW_LIMIT))
        .unwrap_or_default();

    state
        .broadcaster
        .publish(
            ACTIVITY_TOPIC,
            serde_json::json!({
                "kind": "tool_call",
                "tool": tool,
                "argsPreview": args_preview,
                "resultPreview": result_preview,
                "isError": is_error,
                "ts": chrono::Utc::now().to_rfc3339(),
            }),
        )
        .await;
}

/// Truncate to `limit` chars on a char boundary, appending an ellipsis.
fn truncate(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        s.to_string()
    } else {
        let cut: String = s.chars().take(limit).collect();
        format!("{cut}…")
    }
}

/// Dispatch one message in stateless mode.
///
/// HTTP requests may arrive on fresh connections with no session state,
/// so the handshake flag is pre-set: `initialize` is answered normally
/// (idempotently), and `tools/*` calls are honoured without requiring a
/// prior `initialize` on the same connection.
async fn dispatch_stateless(endpoint: &Arc<McpEndpoint>, msg: &Value) -> Option<Value> {
    let mut initialized = true;
    let resp = endpoint
        .dispatcher
        .handle_message(msg, &mut initialized)
        .await;
    debug!(
        method = msg.get("method").and_then(|m| m.as_str()).unwrap_or(""),
        responded = resp.is_some(),
        "dispatched /mcp message"
    );
    resp
}
