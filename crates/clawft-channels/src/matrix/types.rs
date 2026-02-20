//! Matrix channel configuration types.

use serde::{Deserialize, Serialize};

use clawft_types::secret::SecretString;

/// Configuration for the Matrix channel adapter.
///
/// Connects to a Matrix homeserver via the client-server API.
/// Credential fields use [`SecretString`] to prevent exposure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatrixAdapterConfig {
    /// Homeserver URL (e.g. `"https://matrix.org"`).
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
            "allowedUsers": ["@admin:matrix.org"]
        }"#;
        let cfg: MatrixAdapterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.homeserver_url, "https://matrix.org");
        assert_eq!(cfg.user_id, "@bot:matrix.org");
        assert!(cfg.auto_accept_invites);
        assert_eq!(cfg.auto_join_rooms, vec!["!room1:matrix.org"]);
        assert_eq!(cfg.allowed_users, vec!["@admin:matrix.org"]);
    }
}
