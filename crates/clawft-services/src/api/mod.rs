//! REST + WebSocket API for the ClawFT web dashboard.
//!
//! Provides HTTP endpoints for agent management, session browsing,
//! tool inspection, and real-time WebSocket events.

pub mod auth;
pub mod delegation;
pub mod handlers;
pub mod monitoring;
pub mod ws;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Shared state accessible by all API handlers.
#[derive(Clone)]
pub struct ApiState {
    /// Tool registry access.
    pub tools: Arc<dyn ToolRegistryAccess>,
    /// Session manager access.
    pub sessions: Arc<dyn SessionAccess>,
    /// Agent registry access.
    pub agents: Arc<dyn AgentAccess>,
    /// Message bus for WebSocket broadcasting.
    pub bus: Arc<dyn BusAccess>,
    /// Auth token store.
    pub auth: Arc<auth::TokenStore>,
}

/// Trait for tool registry access (decouples API from Platform generics).
pub trait ToolRegistryAccess: Send + Sync {
    /// List all registered tools.
    fn list_tools(&self) -> Vec<ToolInfo>;
    /// Get the JSON schema for a named tool.
    fn tool_schema(&self, name: &str) -> Option<serde_json::Value>;
}

/// Summary info for a registered tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// Trait for session access.
pub trait SessionAccess: Send + Sync {
    /// List all active sessions.
    fn list_sessions(&self) -> Vec<SessionInfo>;
    /// Get details of a specific session.
    fn get_session(&self, key: &str) -> Option<SessionDetail>;
    /// Delete a session by key. Returns true if it existed.
    fn delete_session(&self, key: &str) -> bool;
}

/// Summary info for a session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub key: String,
    pub message_count: usize,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Full detail for a session, including message history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionDetail {
    pub key: String,
    pub messages: Vec<serde_json::Value>,
}

/// Trait for agent registry access.
pub trait AgentAccess: Send + Sync {
    /// List all registered agents.
    fn list_agents(&self) -> Vec<AgentInfo>;
    /// Get details for a specific agent by name.
    fn get_agent(&self, name: &str) -> Option<AgentInfo>;
}

/// Summary info for a registered agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
    pub model: String,
    pub skills: Vec<String>,
}

/// Trait for message bus access (used by WebSocket broadcasting).
pub trait BusAccess: Send + Sync {
    /// Send a message to a specific channel and chat.
    fn send_message(&self, channel: &str, chat_id: &str, content: &str);
}

/// Build the API router with all routes.
pub fn build_router(state: ApiState, cors_origins: &[String]) -> Router {
    let cors = if cors_origins.is_empty() {
        CorsLayer::permissive()
    } else {
        let origins: Vec<_> = cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Router::new()
        .nest("/api", handlers::api_routes())
        .route("/ws", axum::routing::get(ws::ws_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
