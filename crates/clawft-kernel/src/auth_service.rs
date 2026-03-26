//! Centralized credential management service -- Plan 9 Factotum pattern (K5-G2).
//!
//! The [`AuthService`] manages external credentials centrally so that agents
//! never hold raw secrets. Instead, agents request scoped, time-limited tokens
//! via IPC. All credential access is audited.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::KernelError;
use crate::health::HealthStatus;
use crate::process::Pid;
use crate::service::{ServiceType, SystemService};

// ---------------------------------------------------------------------------
// CredentialType
// ---------------------------------------------------------------------------

/// Classification of a stored credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CredentialType {
    /// API key (e.g., OpenAI, GitHub).
    ApiKey,
    /// Bearer/OAuth token.
    BearerToken,
    /// TLS client certificate.
    Certificate,
    /// User-defined credential type.
    Custom(String),
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey => write!(f, "api_key"),
            Self::BearerToken => write!(f, "bearer_token"),
            Self::Certificate => write!(f, "certificate"),
            Self::Custom(s) => write!(f, "custom({s})"),
        }
    }
}

// ---------------------------------------------------------------------------
// StoredCredential
// ---------------------------------------------------------------------------

/// An encrypted credential stored by the AuthService.
#[derive(Debug, Clone)]
pub struct StoredCredential {
    /// Human-readable credential name.
    pub name: String,
    /// Credential classification.
    pub credential_type: CredentialType,
    /// Encrypted credential value (never exposed directly).
    encrypted_value: Vec<u8>,
    /// Agent IDs allowed to request tokens for this credential.
    pub allowed_agents: Vec<String>,
    /// When the credential was registered.
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// IssuedToken
// ---------------------------------------------------------------------------

/// A scoped, time-limited token issued to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedToken {
    /// Unique token identifier.
    pub token_id: String,
    /// Name of the credential this token grants access to.
    pub credential_name: String,
    /// PID of the agent the token was issued to.
    pub issued_to: Pid,
    /// When the token was issued.
    pub issued_at: DateTime<Utc>,
    /// When the token expires.
    pub expires_at: DateTime<Utc>,
    /// Scoped operations this token permits.
    pub scope: Vec<String>,
}

impl IssuedToken {
    /// Check whether the token has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

// ---------------------------------------------------------------------------
// CredentialRequest / CredentialGrant
// ---------------------------------------------------------------------------

/// Request from an agent to obtain a credential token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRequest {
    /// Name of the credential to access.
    pub credential_name: String,
    /// PID of the requesting agent.
    pub requester_pid: Pid,
    /// Agent ID for authorization check.
    pub agent_id: String,
    /// Requested operations scope.
    pub scope: Vec<String>,
    /// Requested time-to-live.
    pub ttl_secs: u64,
}

/// Response granting (or denying) a credential request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialGrant {
    /// Token granted.
    Granted(IssuedToken),
    /// Request denied.
    Denied { reason: String },
}

// ---------------------------------------------------------------------------
// AuthService
// ---------------------------------------------------------------------------

/// Centralized credential management service (Factotum pattern).
///
/// - Credentials are registered once, encrypted at rest.
/// - Agents request scoped tokens; raw credentials are never exposed.
/// - Token issuance and access are audited.
pub struct AuthService {
    /// Registered credentials.
    credentials: DashMap<String, StoredCredential>,
    /// Active tokens.
    active_tokens: DashMap<String, IssuedToken>,
    /// Audit log.
    audit_log: std::sync::RwLock<Vec<AuditEntry>>,
    /// Encryption key for credentials.
    encryption_key: [u8; 32],
}

/// An audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// What happened.
    pub action: String,
    /// Who did it.
    pub agent_id: String,
    /// Credential involved.
    pub credential_name: String,
    /// When it happened.
    pub timestamp: DateTime<Utc>,
    /// Whether it was allowed.
    pub allowed: bool,
}

impl AuthService {
    /// Create a new AuthService with the given encryption key.
    pub fn new(encryption_key: [u8; 32]) -> Self {
        Self {
            credentials: DashMap::new(),
            active_tokens: DashMap::new(),
            audit_log: std::sync::RwLock::new(Vec::new()),
            encryption_key,
        }
    }

    /// Create with a default (zero) encryption key (testing only).
    pub fn new_default() -> Self {
        Self::new([0u8; 32])
    }

    // ── Credential registration ───────────────────────────────────

    /// Register a new credential.
    pub fn register_credential(
        &self,
        name: &str,
        credential_type: CredentialType,
        value: &[u8],
        allowed_agents: Vec<String>,
    ) -> Result<(), KernelError> {
        if self.credentials.contains_key(name) {
            return Err(KernelError::Service(format!(
                "credential already registered: {name}"
            )));
        }

        let encrypted = self.xor_encrypt(value);
        self.credentials.insert(
            name.to_string(),
            StoredCredential {
                name: name.to_string(),
                credential_type,
                encrypted_value: encrypted,
                allowed_agents,
                created_at: Utc::now(),
            },
        );

        info!(name, "credential registered");
        Ok(())
    }

    /// Update an existing credential's value (rotation).
    pub fn rotate_credential(
        &self,
        name: &str,
        new_value: &[u8],
    ) -> Result<(), KernelError> {
        let mut cred = self
            .credentials
            .get_mut(name)
            .ok_or_else(|| KernelError::Service(format!("credential not found: {name}")))?;
        cred.encrypted_value = self.xor_encrypt(new_value);
        info!(name, "credential rotated");
        Ok(())
    }

    // ── Token issuance ────────────────────────────────────────────

    /// Request a scoped, time-limited token.
    pub fn request_token(
        &self,
        request: &CredentialRequest,
    ) -> Result<IssuedToken, KernelError> {
        let cred = self
            .credentials
            .get(&request.credential_name)
            .ok_or_else(|| {
                KernelError::Service(format!(
                    "credential not found: {}",
                    request.credential_name
                ))
            })?;

        // Authorization check.
        if !cred.allowed_agents.is_empty()
            && !cred.allowed_agents.contains(&request.agent_id)
        {
            self.audit("token.denied", &request.agent_id, &request.credential_name, false);
            warn!(
                agent_id = %request.agent_id,
                credential = %request.credential_name,
                "token request denied"
            );
            return Err(KernelError::CapabilityDenied {
                pid: request.requester_pid,
                action: "request_token".into(),
                reason: format!(
                    "agent '{}' not authorized for credential '{}'",
                    request.agent_id, request.credential_name
                ),
            });
        }

        let ttl = Duration::from_secs(request.ttl_secs.max(1));
        let token = IssuedToken {
            token_id: uuid::Uuid::new_v4().to_string(),
            credential_name: request.credential_name.clone(),
            issued_to: request.requester_pid,
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::hours(1)),
            scope: request.scope.clone(),
        };

        self.active_tokens
            .insert(token.token_id.clone(), token.clone());
        self.audit("token.issued", &request.agent_id, &request.credential_name, true);

        info!(
            token_id = %token.token_id,
            credential = %request.credential_name,
            agent = %request.agent_id,
            ttl_secs = request.ttl_secs,
            "token issued"
        );
        Ok(token)
    }

    /// Validate an issued token. Returns `Err` if expired or not found.
    pub fn validate_token(&self, token_id: &str) -> Result<IssuedToken, KernelError> {
        let token = self
            .active_tokens
            .get(token_id)
            .ok_or_else(|| KernelError::Service("token not found".into()))?;

        if token.is_expired() {
            return Err(KernelError::Service("token expired".into()));
        }

        Ok(token.clone())
    }

    /// Revoke an active token.
    pub fn revoke_token(&self, token_id: &str) -> bool {
        self.active_tokens.remove(token_id).is_some()
    }

    /// List all active (non-expired) tokens.
    pub fn active_token_count(&self) -> usize {
        self.active_tokens
            .iter()
            .filter(|t| !t.value().is_expired())
            .count()
    }

    // ── Audit ─────────────────────────────────────────────────────

    fn audit(&self, action: &str, agent_id: &str, credential_name: &str, allowed: bool) {
        if let Ok(mut log) = self.audit_log.write() {
            log.push(AuditEntry {
                action: action.to_string(),
                agent_id: agent_id.to_string(),
                credential_name: credential_name.to_string(),
                timestamp: Utc::now(),
                allowed,
            });
        }
    }

    /// Get the audit log.
    pub fn audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.read().map(|l| l.clone()).unwrap_or_default()
    }

    // ── Encryption ────────────────────────────────────────────────

    fn xor_encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ self.encryption_key[i % 32])
            .collect()
    }
}

#[async_trait]
impl SystemService for AuthService {
    fn name(&self) -> &str {
        "auth-service"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("auth service started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            credentials = self.credentials.len(),
            active_tokens = self.active_token_count(),
            "auth service stopped"
        );
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(n: u64) -> Pid {
        n
    }

    fn make_request(cred: &str, agent: &str, pid_val: u64) -> CredentialRequest {
        CredentialRequest {
            credential_name: cred.to_string(),
            requester_pid: pid(pid_val),
            agent_id: agent.to_string(),
            scope: vec!["read".to_string()],
            ttl_secs: 3600,
        }
    }

    #[test]
    fn register_and_request_token() {
        let svc = AuthService::new_default();
        svc.register_credential(
            "github-key",
            CredentialType::ApiKey,
            b"ghp_secret_value",
            vec!["deploy-agent".to_string()],
        )
        .unwrap();

        let req = make_request("github-key", "deploy-agent", 1);
        let token = svc.request_token(&req).unwrap();
        assert!(!token.token_id.is_empty());
        assert_eq!(token.credential_name, "github-key");
    }

    #[test]
    fn unauthorized_agent_denied() {
        let svc = AuthService::new_default();
        svc.register_credential(
            "secret",
            CredentialType::BearerToken,
            b"token",
            vec!["allowed-agent".to_string()],
        )
        .unwrap();

        let req = make_request("secret", "other-agent", 2);
        let result = svc.request_token(&req);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("denied") || err.contains("authorized"), "got: {err}");
    }

    #[test]
    fn empty_allowed_agents_permits_all() {
        let svc = AuthService::new_default();
        svc.register_credential(
            "open-cred",
            CredentialType::ApiKey,
            b"value",
            vec![], // any agent
        )
        .unwrap();

        let req = make_request("open-cred", "random-agent", 5);
        let token = svc.request_token(&req).unwrap();
        assert!(!token.is_expired());
    }

    #[test]
    fn validate_token() {
        let svc = AuthService::new_default();
        svc.register_credential("cred", CredentialType::ApiKey, b"val", vec![])
            .unwrap();
        let req = make_request("cred", "agent", 1);
        let token = svc.request_token(&req).unwrap();
        let validated = svc.validate_token(&token.token_id).unwrap();
        assert_eq!(validated.token_id, token.token_id);
    }

    #[test]
    fn validate_nonexistent_token_fails() {
        let svc = AuthService::new_default();
        assert!(svc.validate_token("no-such-token").is_err());
    }

    #[test]
    fn revoke_token() {
        let svc = AuthService::new_default();
        svc.register_credential("cred", CredentialType::ApiKey, b"val", vec![])
            .unwrap();
        let req = make_request("cred", "agent", 1);
        let token = svc.request_token(&req).unwrap();
        assert!(svc.revoke_token(&token.token_id));
        assert!(svc.validate_token(&token.token_id).is_err());
    }

    #[test]
    fn credential_rotation_preserves_tokens() {
        let svc = AuthService::new_default();
        svc.register_credential("rotate-cred", CredentialType::ApiKey, b"old_val", vec![])
            .unwrap();
        let req = make_request("rotate-cred", "agent", 1);
        let token = svc.request_token(&req).unwrap();

        // Rotate credential.
        svc.rotate_credential("rotate-cred", b"new_val").unwrap();

        // Existing token still valid.
        let validated = svc.validate_token(&token.token_id).unwrap();
        assert_eq!(validated.credential_name, "rotate-cred");
    }

    #[test]
    fn raw_credential_never_exposed() {
        let key = [0xAB; 32];
        let svc = AuthService::new(key);
        svc.register_credential("secret", CredentialType::ApiKey, b"raw_secret", vec![])
            .unwrap();

        // The stored value should be encrypted, not raw.
        let cred = svc.credentials.get("secret").unwrap();
        assert_ne!(cred.encrypted_value, b"raw_secret");
    }

    #[test]
    fn audit_log_records_events() {
        let svc = AuthService::new_default();
        svc.register_credential(
            "audited",
            CredentialType::ApiKey,
            b"val",
            vec!["agent-a".to_string()],
        )
        .unwrap();

        // Successful request.
        let req = make_request("audited", "agent-a", 1);
        svc.request_token(&req).unwrap();

        // Failed request.
        let bad_req = make_request("audited", "agent-b", 2);
        let _ = svc.request_token(&bad_req);

        let log = svc.audit_log();
        assert_eq!(log.len(), 2);
        assert!(log[0].allowed);
        assert!(!log[1].allowed);
    }

    #[test]
    fn duplicate_credential_registration_fails() {
        let svc = AuthService::new_default();
        svc.register_credential("dup", CredentialType::ApiKey, b"val", vec![])
            .unwrap();
        let result = svc.register_credential("dup", CredentialType::ApiKey, b"val2", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn request_nonexistent_credential_fails() {
        let svc = AuthService::new_default();
        let req = make_request("missing", "agent", 1);
        assert!(svc.request_token(&req).is_err());
    }

    #[tokio::test]
    async fn system_service_impl() {
        let svc = AuthService::new_default();
        assert_eq!(svc.name(), "auth-service");
        assert_eq!(svc.service_type(), ServiceType::Core);
        svc.start().await.unwrap();
        assert_eq!(svc.health_check().await, HealthStatus::Healthy);
        svc.stop().await.unwrap();
    }
}
