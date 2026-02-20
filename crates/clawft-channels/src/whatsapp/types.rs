//! WhatsApp channel configuration and message types.

use serde::{Deserialize, Serialize};

use clawft_types::secret::SecretString;

/// Configuration for the WhatsApp channel adapter.
///
/// All credential fields use [`SecretString`] to prevent accidental exposure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppAdapterConfig {
    /// WhatsApp Business phone number ID.
    #[serde(default, alias = "phoneNumberId")]
    pub phone_number_id: String,

    /// App access token (via SecretString).
    #[serde(default, alias = "accessToken")]
    pub access_token: SecretString,

    /// Webhook verify token (via SecretString -- not plaintext).
    #[serde(default, alias = "verifyToken")]
    pub verify_token: SecretString,

    /// Cloud API base URL.
    #[serde(default = "default_api_url", alias = "apiUrl")]
    pub api_url: String,

    /// API version.
    #[serde(default = "default_api_version", alias = "apiVersion")]
    pub api_version: String,

    /// Allowed phone numbers. Empty = allow all.
    #[serde(default, alias = "allowedNumbers")]
    pub allowed_numbers: Vec<String>,
}

fn default_api_url() -> String {
    "https://graph.facebook.com".into()
}
fn default_api_version() -> String {
    "v21.0".into()
}

impl Default for WhatsAppAdapterConfig {
    fn default() -> Self {
        Self {
            phone_number_id: String::new(),
            access_token: SecretString::default(),
            verify_token: SecretString::default(),
            api_url: default_api_url(),
            api_version: default_api_version(),
            allowed_numbers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let cfg = WhatsAppAdapterConfig::default();
        assert_eq!(cfg.api_url, "https://graph.facebook.com");
        assert_eq!(cfg.api_version, "v21.0");
        assert!(cfg.allowed_numbers.is_empty());
    }

    #[test]
    fn verify_token_uses_secret_string() {
        let cfg = WhatsAppAdapterConfig {
            verify_token: SecretString::new("my-secret"),
            ..Default::default()
        };
        let debug = format!("{:?}", cfg);
        assert!(!debug.contains("my-secret"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn config_serde_roundtrip() {
        let json = r#"{
            "phoneNumberId": "12345",
            "accessToken": "token123",
            "verifyToken": "verify123",
            "allowedNumbers": ["+1234567890"]
        }"#;
        let cfg: WhatsAppAdapterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.phone_number_id, "12345");
        assert_eq!(cfg.allowed_numbers, vec!["+1234567890"]);
    }
}
