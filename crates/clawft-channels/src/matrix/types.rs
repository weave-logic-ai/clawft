//! Matrix channel configuration types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use clawft_types::secret::SecretString;

/// Configuration for the Matrix channel adapter.
///
/// Connects to a Matrix homeserver via the client-server API.
/// Credential fields use [`SecretString`] to prevent exposure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixAdapterConfig {
    /// Homeserver URL (e.g. `"https://matrix.org"`).
    ///
    /// Tests point this at a local `wiremock` server URI.
    #[serde(default, alias = "homeserverUrl")]
    pub homeserver_url: String,

    /// Access token for the bot user (via SecretString).
    #[serde(default, alias = "accessToken")]
    pub access_token: SecretString,

    /// Bot user ID (e.g. `"@bot:matrix.org"`).
    #[serde(default, alias = "userId")]
    pub user_id: String,

    /// Rooms to join on startup.
    #[serde(default, alias = "autoJoinRooms")]
    pub auto_join_rooms: Vec<String>,

    /// Whether to accept room invitations automatically.
    #[serde(default, alias = "autoAcceptInvites")]
    pub auto_accept_invites: bool,

    /// Allowed user IDs. Empty = allow all.
    #[serde(default, alias = "allowedUsers")]
    pub allowed_users: Vec<String>,

    /// Directory for durable state (since-token file). When unset, uses
    /// `$XDG_DATA_HOME/clawft/matrix` (or platform equivalent via `dirs`).
    #[serde(default, alias = "stateDir")]
    pub state_dir: Option<PathBuf>,

    /// Long-poll timeout for `GET /_matrix/client/v3/sync` in milliseconds.
    /// Defaults to 30_000. Tests use a short value so the mock returns quickly.
    #[serde(default = "default_sync_timeout_ms", alias = "syncTimeoutMs")]
    pub sync_timeout_ms: u64,
}

fn default_sync_timeout_ms() -> u64 {
    30_000
}

impl Default for MatrixAdapterConfig {
    fn default() -> Self {
        Self {
            homeserver_url: String::new(),
            access_token: SecretString::default(),
            user_id: String::new(),
            auto_join_rooms: Vec::new(),
            auto_accept_invites: false,
            allowed_users: Vec::new(),
            state_dir: None,
            sync_timeout_ms: default_sync_timeout_ms(),
        }
    }
}

impl MatrixAdapterConfig {
    /// Resolved state directory for the since-token file.
    pub fn resolved_state_dir(&self) -> PathBuf {
        self.state_dir.clone().unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("clawft")
                .join("matrix")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let cfg = MatrixAdapterConfig::default();
        assert!(cfg.homeserver_url.is_empty());
        assert!(cfg.user_id.is_empty());
        assert!(!cfg.auto_accept_invites);
        assert!(cfg.auto_join_rooms.is_empty());
        assert!(cfg.allowed_users.is_empty());
        assert!(cfg.state_dir.is_none());
        assert_eq!(cfg.sync_timeout_ms, 30_000);
    }

    #[test]
    fn access_token_uses_secret_string() {
        let cfg = MatrixAdapterConfig {
            access_token: SecretString::new("my-token"),
            ..Default::default()
        };
        let debug = format!("{:?}", cfg);
        assert!(!debug.contains("my-token"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn config_serde_roundtrip() {
        let json = r#"{
            "homeserverUrl": "https://matrix.org",
            "accessToken": "syt_abc123",
            "userId": "@bot:matrix.org",
            "autoJoinRooms": ["!room1:matrix.org"],
            "autoAcceptInvites": true,
            "allowedUsers": ["@admin:matrix.org"],
            "stateDir": "/tmp/matrix-state",
            "syncTimeoutMs": 5000
        }"#;
        let cfg: MatrixAdapterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.homeserver_url, "https://matrix.org");
        assert_eq!(cfg.user_id, "@bot:matrix.org");
        assert!(cfg.auto_accept_invites);
        assert_eq!(cfg.auto_join_rooms, vec!["!room1:matrix.org"]);
        assert_eq!(cfg.allowed_users, vec!["@admin:matrix.org"]);
        assert_eq!(
            cfg.state_dir.as_deref(),
            Some(std::path::Path::new("/tmp/matrix-state"))
        );
        assert_eq!(cfg.sync_timeout_ms, 5000);
    }

    #[test]
    fn resolved_state_dir_prefers_explicit() {
        let cfg = MatrixAdapterConfig {
            state_dir: Some(PathBuf::from("/custom/state")),
            ..Default::default()
        };
        assert_eq!(cfg.resolved_state_dir(), PathBuf::from("/custom/state"));
    }
}
