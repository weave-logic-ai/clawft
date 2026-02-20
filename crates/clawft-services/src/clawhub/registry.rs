//! ClawHub skill registry client and types.
//!
//! Implements the REST API contract (Contract #20):
//!
//! ```text
//! GET  /api/v1/skills/search?q=&limit=&offset=
//! POST /api/v1/skills/publish
//! POST /api/v1/skills/install
//!
//! Auth: Bearer <token>
//! Response: { "ok": bool, "data": T, "error": Option<String>, "pagination": {...} }
//! ```

use serde::{Deserialize, Serialize};

/// ClawHub API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether the request succeeded.
    pub ok: bool,
    /// Response data (present on success).
    pub data: Option<T>,
    /// Error message (present on failure).
    pub error: Option<String>,
    /// Pagination info (present for list/search endpoints).
    pub pagination: Option<Pagination>,
}

/// Pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

/// A skill entry in the ClawHub registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    /// Unique skill identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Semantic version.
    pub version: String,
    /// Author identifier.
    pub author: String,
    /// Star count.
    pub stars: u32,
    /// Content hash (SHA-256) for verification.
    pub content_hash: String,
    /// Whether the skill is signed.
    pub signed: bool,
    /// Signature (if signed).
    pub signature: Option<String>,
    /// Publication timestamp (ISO 8601).
    pub published_at: String,
    /// Tags for categorization.
    pub tags: Vec<String>,
}

/// Request body for publishing a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Semantic version.
    pub version: String,
    /// Skill content (base64-encoded archive).
    pub content: String,
    /// Content hash (SHA-256).
    pub content_hash: String,
    /// Digital signature.
    pub signature: Option<String>,
    /// Tags.
    pub tags: Vec<String>,
}

/// Result of installing a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstallResult {
    /// Whether installation succeeded.
    pub success: bool,
    /// Installed skill path.
    pub install_path: Option<String>,
    /// Security scan results (if scan was run).
    pub security_scan_passed: Option<bool>,
    /// Error message (if failed).
    pub error: Option<String>,
}

/// ClawHub client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawHubConfig {
    /// Registry API base URL.
    pub api_url: String,
    /// API token for authentication.
    pub api_token: Option<String>,
    /// Whether to allow unsigned skills (local dev only).
    pub allow_unsigned: bool,
}

impl Default for ClawHubConfig {
    fn default() -> Self {
        Self {
            api_url: "https://hub.clawft.dev/api/v1".to_string(),
            api_token: None,
            allow_unsigned: false,
        }
    }
}

/// ClawHub registry client.
pub struct ClawHubClient {
    config: ClawHubConfig,
}

impl ClawHubClient {
    /// Create a new client with the given config.
    pub fn new(config: ClawHubConfig) -> Self {
        Self { config }
    }

    /// Search for skills by query.
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<ApiResponse<Vec<SkillEntry>>, String> {
        let _url = format!(
            "{}/skills/search?q={}&limit={}&offset={}",
            self.config.api_url,
            urlencoding::encode(query),
            limit,
            offset,
        );

        // Stub: return empty results until the server is available.
        Ok(ApiResponse {
            ok: true,
            data: Some(Vec::new()),
            error: None,
            pagination: Some(Pagination {
                total: 0,
                offset,
                limit,
            }),
        })
    }

    /// Publish a skill to the registry.
    pub async fn publish(
        &self,
        request: &PublishRequest,
    ) -> Result<ApiResponse<SkillEntry>, String> {
        // Require signature unless allow_unsigned is set.
        if request.signature.is_none() && !self.config.allow_unsigned {
            return Ok(ApiResponse {
                ok: false,
                data: None,
                error: Some(
                    "skill must be signed for publication. \
                     Use --allow-unsigned for local dev only."
                        .into(),
                ),
                pagination: None,
            });
        }

        if self.config.allow_unsigned && request.signature.is_none() {
            tracing::warn!(
                skill = %request.name,
                "publishing unsigned skill (--allow-unsigned flag used)"
            );
        }

        // Stub: return a mock entry.
        Ok(ApiResponse {
            ok: true,
            data: Some(SkillEntry {
                id: format!("skill-{}", &request.content_hash[..8]),
                name: request.name.clone(),
                description: request.description.clone(),
                version: request.version.clone(),
                author: "local".into(),
                stars: 0,
                content_hash: request.content_hash.clone(),
                signed: request.signature.is_some(),
                signature: request.signature.clone(),
                published_at: chrono::Utc::now().to_rfc3339(),
                tags: request.tags.clone(),
            }),
            error: None,
            pagination: None,
        })
    }

    /// Install a skill from the registry.
    pub async fn install(
        &self,
        skill_id: &str,
        _install_dir: &str,
    ) -> Result<SkillInstallResult, String> {
        // Stub: skill installation will be implemented when the registry
        // server is available.
        tracing::info!(
            skill = %skill_id,
            "installing skill from ClawHub"
        );

        Ok(SkillInstallResult {
            success: false,
            install_path: None,
            security_scan_passed: None,
            error: Some(format!(
                "ClawHub registry not yet available for skill '{skill_id}'"
            )),
        })
    }

    /// Get the client configuration.
    pub fn config(&self) -> &ClawHubConfig {
        &self.config
    }
}

/// URL-encode a string for use in query parameters.
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::with_capacity(s.len());
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => {
                    encoded.push_str(&format!("%{byte:02X}"));
                }
            }
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ClawHubConfig::default();
        assert!(!config.allow_unsigned);
        assert!(config.api_token.is_none());
    }

    #[test]
    fn api_response_serialization() {
        let response: ApiResponse<Vec<SkillEntry>> = ApiResponse {
            ok: true,
            data: Some(vec![]),
            error: None,
            pagination: Some(Pagination {
                total: 0,
                offset: 0,
                limit: 10,
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"ok\":true"));
    }

    #[test]
    fn publish_request_requires_signature() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = ClawHubClient::new(ClawHubConfig::default());
        let request = PublishRequest {
            name: "test-skill".into(),
            description: "A test skill".into(),
            version: "1.0.0".into(),
            content: "base64content".into(),
            content_hash: "abc123def456".into(),
            signature: None,
            tags: vec!["test".into()],
        };
        let result = rt.block_on(client.publish(&request)).unwrap();
        assert!(!result.ok, "unsigned publish should fail");
        assert!(result.error.unwrap().contains("signed"));
    }

    #[test]
    fn publish_request_allows_unsigned_when_flagged() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let config = ClawHubConfig {
            allow_unsigned: true,
            ..Default::default()
        };
        let client = ClawHubClient::new(config);
        let request = PublishRequest {
            name: "test-skill".into(),
            description: "A test skill".into(),
            version: "1.0.0".into(),
            content: "base64content".into(),
            content_hash: "abc123def456".into(),
            signature: None,
            tags: vec![],
        };
        let result = rt.block_on(client.publish(&request)).unwrap();
        assert!(result.ok, "unsigned publish with allow_unsigned should succeed");
    }

    #[test]
    fn urlencoding_basic() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("test"), "test");
        assert_eq!(urlencoding::encode("a+b"), "a%2Bb");
    }
}
