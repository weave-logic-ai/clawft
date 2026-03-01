//! Authentication middleware and token management.

use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory token store for API authentication.
pub struct TokenStore {
    tokens: RwLock<HashMap<String, TokenInfo>>,
}

/// Metadata for an issued API token.
#[derive(Clone)]
pub struct TokenInfo {
    pub created_at: std::time::Instant,
    pub ttl_secs: u64,
}

impl TokenStore {
    /// Create a new empty token store.
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a new API token with the given TTL in seconds.
    pub fn generate_token(&self, ttl_secs: u64) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        let info = TokenInfo {
            created_at: std::time::Instant::now(),
            ttl_secs,
        };
        self.tokens.write().unwrap().insert(token.clone(), info);
        token
    }

    /// Validate a token. Returns true if valid and not expired.
    pub fn validate(&self, token: &str) -> bool {
        let tokens = self.tokens.read().unwrap();
        if let Some(info) = tokens.get(token) {
            info.created_at.elapsed().as_secs() < info.ttl_secs
        } else {
            false
        }
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Tower middleware that validates Bearer tokens on protected routes.
///
/// Requests to `/api/auth/token` and `/api/health` are exempt from
/// authentication. All other `/api/*` routes require a valid Bearer
/// token in the `Authorization` header.
///
/// # Usage
///
/// This middleware is **not** enabled by default to keep the development
/// workflow frictionless. To activate it, wrap the `/api` nest in
/// `build_router()`:
///
/// ```ignore
/// use axum::middleware;
///
/// Router::new()
///     .nest("/api", handlers::api_routes()
///         .layer(middleware::from_fn_with_state(
///             state.clone(), auth::auth_middleware)))
/// ```
pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<super::ApiState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    let path = request.uri().path();

    // Skip auth for token creation and health-check endpoints.
    if path == "/api/auth/token" || path == "/api/health" {
        return Ok(next.run(request).await);
    }

    // Extract and validate the Bearer token.
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if state.auth.validate(token) {
                Ok(next.run(request).await)
            } else {
                Err(axum::http::StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(axum::http::StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_store_generate_and_validate() {
        let store = TokenStore::new();
        let token = store.generate_token(3600);
        assert!(store.validate(&token));
    }

    #[test]
    fn token_store_rejects_unknown() {
        let store = TokenStore::new();
        assert!(!store.validate("not-a-real-token"));
    }

    #[test]
    fn token_store_default() {
        let store = TokenStore::default();
        assert!(!store.validate("anything"));
    }
}
