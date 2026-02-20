//! Microsoft Teams channel configuration types.

use serde::{Deserialize, Serialize};

use clawft_types::secret::SecretString;

/// Configuration for the Microsoft Teams channel adapter.
///
/// Connects to Microsoft Teams via the Bot Framework / Graph API.
/// Uses Azure AD for authentication (client credentials flow).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsAdapterConfig {
    /// Azure AD tenant ID.
    #[serde(default, alias = "tenantId")]
    pub tenant_id: String,

    /// Azure AD application (client) ID.
    #[serde(default, alias = "clientId")]
    pub client_id: String,

    /// Azure AD client secret (via SecretString).
    #[serde(default, alias = "clientSecret")]
    pub client_secret: SecretString,

    /// Bot Framework app ID (may differ from client_id).
    #[serde(default, alias = "botAppId")]
    pub bot_app_id: String,

    /// Teams to monitor. Empty = all teams the bot is added to.
    #[serde(default)]
    pub teams: Vec<String>,

    /// Channels within teams to monitor. Empty = all channels.
    #[serde(default)]
    pub channels: Vec<String>,

    /// Allowed user principal names. Empty = allow all.
    #[serde(default, alias = "allowedUsers")]
    pub allowed_users: Vec<String>,

    /// Microsoft Graph API base URL.
    #[serde(default = "default_graph_url", alias = "graphUrl")]
    pub graph_url: String,
}

fn default_graph_url() -> String {
    "https://graph.microsoft.com/v1.0".into()
}

impl Default for TeamsAdapterConfig {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            client_id: String::new(),
            client_secret: SecretString::default(),
            bot_app_id: String::new(),
            teams: Vec::new(),
            channels: Vec::new(),
            allowed_users: Vec::new(),
            graph_url: default_graph_url(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let cfg = TeamsAdapterConfig::default();
        assert!(cfg.tenant_id.is_empty());
        assert!(cfg.client_id.is_empty());
        assert!(cfg.bot_app_id.is_empty());
        assert!(cfg.teams.is_empty());
        assert!(cfg.channels.is_empty());
        assert!(cfg.allowed_users.is_empty());
        assert_eq!(
            cfg.graph_url,
            "https://graph.microsoft.com/v1.0"
        );
    }

    #[test]
    fn client_secret_uses_secret_string() {
        let cfg = TeamsAdapterConfig {
            client_secret: SecretString::new("super-secret"),
            ..Default::default()
        };
        let debug = format!("{:?}", cfg);
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn config_serde_roundtrip() {
        let json = r#"{
            "tenantId": "tenant-abc",
            "clientId": "client-123",
            "clientSecret": "secret-xyz",
            "botAppId": "bot-456",
            "teams": ["team-1"],
            "channels": ["channel-1"],
            "allowedUsers": ["user@company.com"]
        }"#;
        let cfg: TeamsAdapterConfig =
            serde_json::from_str(json).unwrap();
        assert_eq!(cfg.tenant_id, "tenant-abc");
        assert_eq!(cfg.client_id, "client-123");
        assert_eq!(cfg.bot_app_id, "bot-456");
        assert_eq!(cfg.teams, vec!["team-1"]);
        assert_eq!(cfg.channels, vec!["channel-1"]);
        assert_eq!(cfg.allowed_users, vec!["user@company.com"]);
    }
}
