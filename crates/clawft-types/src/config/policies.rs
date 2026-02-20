//! Security policy configuration types.
//!
//! Defines [`CommandPolicyConfig`] (command execution allowlist/denylist)
//! and [`UrlPolicyConfig`] (SSRF protection for URL fetching).

use serde::{Deserialize, Serialize};

/// Command execution security policy configuration.
///
/// Controls which commands the shell and spawn tools are allowed to execute.
/// In allowlist mode (default), only explicitly permitted commands can run.
/// In denylist mode, any command not matching a blocked pattern is allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicyConfig {
    /// Policy mode: "allowlist" (default, recommended) or "denylist".
    #[serde(default = "default_policy_mode")]
    pub mode: String,

    /// Permitted command basenames when in allowlist mode.
    /// Overrides defaults when non-empty.
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Blocked command patterns when in denylist mode.
    /// Overrides defaults when non-empty.
    #[serde(default)]
    pub denylist: Vec<String>,
}

fn default_policy_mode() -> String {
    "allowlist".to_string()
}

impl Default for CommandPolicyConfig {
    fn default() -> Self {
        Self {
            mode: default_policy_mode(),
            allowlist: Vec::new(),
            denylist: Vec::new(),
        }
    }
}

/// URL safety policy configuration for SSRF protection.
///
/// Controls which URLs the web fetch tool is allowed to access.
/// When enabled (default), requests to private networks, loopback
/// addresses, and cloud metadata endpoints are blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlPolicyConfig {
    /// Whether URL safety validation is enabled.
    #[serde(default = "default_url_policy_enabled")]
    pub enabled: bool,

    /// Allow requests to private/internal IP ranges.
    #[serde(default, alias = "allowPrivate")]
    pub allow_private: bool,

    /// Domains that bypass all safety checks.
    #[serde(default, alias = "allowedDomains")]
    pub allowed_domains: Vec<String>,

    /// Domains that are always blocked.
    #[serde(default, alias = "blockedDomains")]
    pub blocked_domains: Vec<String>,
}

fn default_url_policy_enabled() -> bool {
    true
}

impl Default for UrlPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: default_url_policy_enabled(),
            allow_private: false,
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }
}
