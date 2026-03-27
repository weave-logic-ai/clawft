//! Delegation certificate lifecycle -- K1 deferred, stub only in K0.
//!
//! ## K1 scope
//! - DelegationCert: grant/revoke with Ed25519 signatures
//! - Certificate chain validation
//! - Time-bounded delegation with expiry

use serde::{Deserialize, Serialize};

use crate::model::ResourceId;

/// A delegation certificate granting temporary access.
///
/// **K0 stub**: Type definition only. Lifecycle operations in K1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationCert {
    /// Who granted the delegation.
    pub grantor: String,
    /// Who receives the delegation.
    pub grantee: String,
    /// Resource scope of the delegation.
    pub resource: ResourceId,
    /// When the delegation was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the delegation expires (None = no expiry).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this certificate has been revoked.
    pub revoked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn delegation_cert_serde_roundtrip() {
        let cert = DelegationCert {
            grantor: "admin".to_string(),
            grantee: "agent-007".to_string(),
            resource: ResourceId::new("/apps/secret"),
            created_at: Utc::now(),
            expires_at: None,
            revoked: false,
        };
        let json = serde_json::to_string(&cert).unwrap();
        let back: DelegationCert = serde_json::from_str(&json).unwrap();
        assert_eq!(back.grantor, "admin");
        assert_eq!(back.grantee, "agent-007");
        assert_eq!(back.resource, ResourceId::new("/apps/secret"));
        assert!(!back.revoked);
    }
}
