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
