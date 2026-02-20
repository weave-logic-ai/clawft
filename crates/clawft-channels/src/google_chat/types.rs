//! Google Chat channel configuration types.

use serde::{Deserialize, Serialize};

/// Configuration for the Google Chat channel adapter.
///
/// Connects to Google Chat via the Chat API. Requires a
/// Google Cloud service account with the Chat API enabled.
///
/// OAuth2 integration (service account credentials) is deferred
/// to Workstream F6. This skeleton defines the configuration
/// shape and adapter structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoogleChatAdapterConfig {
    /// Google Cloud project ID.
    #[serde(default, alias = "projectId")]
    pub project_id: String,

    /// Path to the service account JSON key file.
    /// Should be set via environment variable in production.
    #[serde(default, alias = "serviceAccountKeyPath")]
    pub service_account_key_path: String,

    /// Spaces (rooms) to listen to. Empty = all spaces the bot is in.
    #[serde(default)]
    pub spaces: Vec<String>,

    /// Allowed user emails. Empty = allow all.
    #[serde(default, alias = "allowedUsers")]
    pub allowed_users: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let cfg = GoogleChatAdapterConfig::default();
        assert!(cfg.project_id.is_empty());
        assert!(cfg.service_account_key_path.is_empty());
        assert!(cfg.spaces.is_empty());
        assert!(cfg.allowed_users.is_empty());
    }

    #[test]
    fn config_serde_roundtrip() {
        let json = r#"{
            "projectId": "my-project-123",
            "serviceAccountKeyPath": "/etc/keys/sa.json",
            "spaces": ["spaces/AAAA"],
            "allowedUsers": ["admin@company.com"]
        }"#;
        let cfg: GoogleChatAdapterConfig =
            serde_json::from_str(json).unwrap();
        assert_eq!(cfg.project_id, "my-project-123");
        assert_eq!(
            cfg.service_account_key_path,
            "/etc/keys/sa.json"
        );
        assert_eq!(cfg.spaces, vec!["spaces/AAAA"]);
        assert_eq!(cfg.allowed_users, vec!["admin@company.com"]);
    }
}
