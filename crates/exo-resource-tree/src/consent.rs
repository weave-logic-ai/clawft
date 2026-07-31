//! Lightweight consent evaluation for the K1 ACL engine (WEFT-554).
//!
//! The SPARC design (`.planning/sparc/weftos/13-exo-resource-tree.md` §5.1)
//! routes collected policies through `exo_consent::evaluate` with a
//! principal risk score. There is no standalone `exo_consent` crate in
//! this workspace yet — this module provides the **foundation** API the
//! ACL engine calls, with a deny-by-policy path and risk-threshold gate.
//!
//! Residual (tracked as follow-up): full BailmentPolicy types, GDPR
//! ConsentProof signing via grantor key material, and cognitum-gate
//! Defer mapping.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::did::Did;
use crate::model::Action;

/// Effect of a consent policy match (mirrors ACL Allow/Deny without
/// importing the permission module — avoids a cycle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentEffect {
    Allow,
    Deny,
}

/// Result of consent evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentDecision {
    /// Principal may proceed.
    Allow,
    /// Explicit or default denial.
    Deny {
        /// Human-readable reason (for audit / UI).
        reason: String,
    },
    /// Defer to a higher gate (risk / human). Maps to cognitum-gate Defer.
    Defer {
        /// Why evaluation deferred.
        reason: String,
    },
}

/// Risk score for a principal (0–100). Higher = riskier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskScore {
    /// Magnitude 0–100.
    pub score: u8,
    /// Confidence in basis points (0–10000).
    pub confidence_bps: u16,
}

impl RiskScore {
    /// Neutral default used when no score is known.
    pub const NEUTRAL: Self = Self {
        score: 50,
        confidence_bps: 5000,
    };

    /// Construct a clamped score.
    pub fn new(score: u8, confidence_bps: u16) -> Self {
        Self {
            score: score.min(100),
            confidence_bps: confidence_bps.min(10_000),
        }
    }
}

impl Default for RiskScore {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// A consent-layer policy entry (mirrors ACL effect semantics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentPolicy {
    /// Who this policy applies to. `None` = any principal DID.
    pub principal: Option<Did>,
    /// Action this policy gates.
    pub action: Action,
    /// Allow or Deny.
    pub effect: ConsentEffect,
    /// Optional free-form reason stored for Deny audit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ConsentPolicy {
    /// Explicit allow for a DID + action.
    pub fn allow(principal: Did, action: Action) -> Self {
        Self {
            principal: Some(principal),
            action,
            effect: ConsentEffect::Allow,
            reason: None,
        }
    }

    /// Explicit deny for a DID + action.
    pub fn deny(principal: Did, action: Action, reason: impl Into<String>) -> Self {
        Self {
            principal: Some(principal),
            action,
            effect: ConsentEffect::Deny,
            reason: Some(reason.into()),
        }
    }

    fn matches(&self, principal: &Did, action: &Action) -> bool {
        if &self.action != action {
            return false;
        }
        match &self.principal {
            Some(p) => p == principal,
            None => true,
        }
    }
}

/// GDPR-oriented consent proof emitted on grant (foundation stub).
///
/// Full Ed25519-attested proofs are residual until `exo_consent` lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentProof {
    /// Grantor DID string.
    pub grantor: String,
    /// Grantee DID string.
    pub grantee: String,
    /// Purpose / scope label (e.g. `delegation:operator`).
    pub purpose: String,
    /// When the proof was created.
    pub created_at: DateTime<Utc>,
}

/// Create an unsigned consent proof (audit foundation).
pub fn create_consent_proof(
    grantor: &Did,
    grantee: &Did,
    purpose: impl Into<String>,
) -> ConsentProof {
    ConsentProof {
        grantor: grantor.as_str().to_string(),
        grantee: grantee.as_str().to_string(),
        purpose: purpose.into(),
        created_at: Utc::now(),
    }
}

/// Evaluate collected consent policies for `(principal, action, risk)`.
///
/// Algorithm (deny-by-policy, most-specific first among matching):
/// 1. If risk exceeds `risk_deny_threshold` → **Deny** (high risk).
/// 2. If risk exceeds `risk_defer_threshold` (but not deny) → **Defer**.
/// 3. First matching **Deny** policy → **Deny** with policy reason.
/// 4. First matching **Allow** policy → **Allow**.
/// 5. No match → **Deny** default.
pub fn evaluate(
    policies: &[ConsentPolicy],
    principal: &Did,
    action: &Action,
    risk: &RiskScore,
    risk_deny_threshold: u8,
    risk_defer_threshold: u8,
) -> ConsentDecision {
    if risk.score >= risk_deny_threshold {
        return ConsentDecision::Deny {
            reason: format!(
                "risk score {} >= deny threshold {}",
                risk.score, risk_deny_threshold
            ),
        };
    }
    if risk.score >= risk_defer_threshold {
        return ConsentDecision::Defer {
            reason: format!(
                "risk score {} >= defer threshold {}",
                risk.score, risk_defer_threshold
            ),
        };
    }

    let mut first_allow = false;
    for policy in policies {
        if !policy.matches(principal, action) {
            continue;
        }
        match policy.effect {
            ConsentEffect::Deny => {
                return ConsentDecision::Deny {
                    reason: policy
                        .reason
                        .clone()
                        .unwrap_or_else(|| "consent policy deny".into()),
                };
            }
            ConsentEffect::Allow => {
                first_allow = true;
            }
        }
    }

    if first_allow {
        ConsentDecision::Allow
    } else {
        ConsentDecision::Deny {
            reason: "no matching consent allow policy".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_by_policy() {
        let did = Did::weft("alice").unwrap();
        let policies = vec![ConsentPolicy::deny(
            did.clone(),
            Action::Admin,
            "admin locked",
        )];
        let d = evaluate(
            &policies,
            &did,
            &Action::Admin,
            &RiskScore::NEUTRAL,
            90,
            70,
        );
        assert_eq!(
            d,
            ConsentDecision::Deny {
                reason: "admin locked".into()
            }
        );
    }

    #[test]
    fn allow_when_policy_matches() {
        let did = Did::weft("bob").unwrap();
        let policies = vec![ConsentPolicy::allow(did.clone(), Action::Read)];
        let d = evaluate(
            &policies,
            &did,
            &Action::Read,
            &RiskScore::NEUTRAL,
            90,
            70,
        );
        assert_eq!(d, ConsentDecision::Allow);
    }

    #[test]
    fn high_risk_denies() {
        let did = Did::from_str_unchecked("eve");
        let d = evaluate(
            &[],
            &did,
            &Action::Write,
            &RiskScore::new(95, 9000),
            90,
            70,
        );
        assert!(matches!(d, ConsentDecision::Deny { .. }));
    }

    #[test]
    fn mid_risk_defers() {
        let did = Did::from_str_unchecked("mallory");
        let d = evaluate(
            &[ConsentPolicy::allow(did.clone(), Action::Read)],
            &did,
            &Action::Read,
            &RiskScore::new(75, 8000),
            90,
            70,
        );
        assert!(matches!(d, ConsentDecision::Defer { .. }));
    }

    #[test]
    fn consent_proof_records_dids() {
        let g = Did::weft("admin").unwrap();
        let d = Did::weft("worker").unwrap();
        let proof = create_consent_proof(&g, &d, "delegation:operator");
        assert_eq!(proof.grantor, "did:weft:admin");
        assert_eq!(proof.grantee, "did:weft:worker");
        assert_eq!(proof.purpose, "delegation:operator");
    }
}
