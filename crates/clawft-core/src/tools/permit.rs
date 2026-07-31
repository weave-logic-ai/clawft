//! Per-tool Permit token + proof-of-permission API (WEFT-341).
//!
//! The agent effect gate returns an opaque string on
//! [`GateDecision::Permit`](crate::agent::gate::GateDecision::Permit).
//! That string alone is not enough for a tool (or the registry) to
//! assert "this specific agent was just permitted to run *this*
//! tool" — there is no tool binding, no TTL, and no integrity tag.
//!
//! This module mints a **per-tool** [`ToolPermitToken`] from the gate
//! payload and exposes a proof-of-permission API that tools / the
//! agent loop can present:
//!
//! 1. [`PermitIssuer::issue`] — bind `(agent_id, tool, gate_token)`
//!    into a MAC'd token with a TTL.
//! 2. [`PermitIssuer::verify`] / [`present_proof`] — check the MAC,
//!    tool binding, agent binding, and expiry.
//!
//! Verification outcomes map cleanly onto the three shapes required
//! by the acceptance criteria:
//!
//! | Shape | [`PermitProofError`] |
//! |-------|----------------------|
//! | allow | `Ok(())` |
//! | deny  | `Missing`, `ToolMismatch`, `AgentMismatch`, `InvalidMac`, `Denied` |
//! | expired | `Expired { .. }` |
//!
//! Back-compat: callers that pass no token still work. The registry
//! only verifies when a token is presented (or when
//! `require_permit` is set on the issuer context). Tools that do not
//! care about proofs simply ignore the token.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

/// Default TTL for a freshly issued per-tool permit (60 seconds).
///
/// Matches the TileZero kernel PermitToken default window so an
/// agent-side token and a kernel receipt share the same liveness
/// expectation.
pub const DEFAULT_PERMIT_TTL_NS: u64 = 60_000_000_000;

/// Fixed domain-separation prefix mixed into the HMAC key material so
/// a cost-tracker or identity HMAC key cannot be reused as a permit
/// key by accident.
const KEY_DOMAIN: &[u8] = b"weftos-tool-permit-v1:";

/// Per-tool Permit token — proof that a gate check allowed a specific
/// agent to invoke a specific tool for a bounded window.
///
/// Issued by [`PermitIssuer::issue`] after
/// [`GateDecision::Permit`](crate::agent::gate::GateDecision::Permit)
/// and presented to [`ToolRegistry::execute_with_permit`](super::registry::ToolRegistry::execute_with_permit)
/// (or any tool that wants to re-check permission mid-execution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPermitToken {
    /// Agent principal the gate permitted.
    pub agent_id: String,
    /// Bare tool name (e.g. `write_file`), not the `tool.`-prefixed
    /// action string the gate saw.
    pub tool: String,
    /// Full gate action string (e.g. `tool.write_file`) for audit
    /// correlation with chain / kernel events.
    pub action: String,
    /// Issue time as Unix epoch nanoseconds.
    pub issued_at_ns: u64,
    /// Validity window in nanoseconds from [`Self::issued_at_ns`].
    pub ttl_ns: u64,
    /// Opaque payload from the gate decision (`"noop"`, hex-encoded
    /// TileZero bytes, `"kernel-permit"`, …). Bound into the MAC so a
    /// token cannot be re-used with a different gate receipt.
    pub gate_token: String,
    /// Hex-encoded HMAC-SHA256 over the signable content.
    pub mac: String,
}

impl ToolPermitToken {
    /// Returns `true` when `now_ns` is within the token's validity
    /// window (inclusive of the expiry instant).
    pub fn is_valid_time(&self, now_ns: u64) -> bool {
        let expiry = self.issued_at_ns.saturating_add(self.ttl_ns);
        now_ns <= expiry
    }

    /// Nanosecond expiry instant (`issued_at_ns + ttl_ns`, saturating).
    pub fn expires_at_ns(&self) -> u64 {
        self.issued_at_ns.saturating_add(self.ttl_ns)
    }

    /// Canonical signable payload used by issue/verify.
    ///
    /// Layout (length-prefixed UTF-8 fields + fixed-width integers):
    /// ```text
    /// agent_id || 0x00 || tool || 0x00 || action || 0x00
    ///   || issued_at_ns LE || ttl_ns LE || gate_token
    /// ```
    fn signable_content(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            self.agent_id.len()
                + self.tool.len()
                + self.action.len()
                + self.gate_token.len()
                + 16
                + 3,
        );
        out.extend_from_slice(self.agent_id.as_bytes());
        out.push(0);
        out.extend_from_slice(self.tool.as_bytes());
        out.push(0);
        out.extend_from_slice(self.action.as_bytes());
        out.push(0);
        out.extend_from_slice(&self.issued_at_ns.to_le_bytes());
        out.extend_from_slice(&self.ttl_ns.to_le_bytes());
        out.extend_from_slice(self.gate_token.as_bytes());
        out
    }
}

/// Failure shapes for proof-of-permission verification.
///
/// Grouped so tests and call sites can pattern-match the three
/// acceptance-criteria buckets: **allow** (`Ok`), **deny** (every
/// non-expired variant), **expired** ([`Self::Expired`]).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PermitProofError {
    /// No permit token was presented where one is required.
    #[error("permit token missing")]
    Missing,

    /// Token MAC does not match the expected HMAC.
    #[error("permit token MAC invalid")]
    InvalidMac,

    /// Token was issued for a different tool than the one being run.
    #[error("permit tool mismatch: expected '{expected}', got '{actual}'")]
    ToolMismatch {
        /// Tool name bound into the token.
        expected: String,
        /// Tool name being invoked.
        actual: String,
    },

    /// Token was issued for a different agent principal.
    #[error("permit agent mismatch: expected '{expected}', got '{actual}'")]
    AgentMismatch {
        /// Agent id bound into the token.
        expected: String,
        /// Agent id presenting the token.
        actual: String,
    },

    /// Token is past its TTL.
    #[error("permit token expired (issued_at_ns={issued_at_ns}, ttl_ns={ttl_ns}, now_ns={now_ns})")]
    Expired {
        /// When the token was issued.
        issued_at_ns: u64,
        /// Validity window.
        ttl_ns: u64,
        /// Clock reading used for the check.
        now_ns: u64,
    },

    /// Explicit deny shape for callers that re-present a gate deny
    /// reason through the same proof channel (e.g. a tool that
    /// re-checks after a deferred allow was never granted).
    #[error("permit denied: {reason}")]
    Denied {
        /// Human-readable denial reason.
        reason: String,
    },
}

impl PermitProofError {
    /// `true` for the expired shape (distinct from other denials).
    pub fn is_expired(&self) -> bool {
        matches!(self, PermitProofError::Expired { .. })
    }

    /// `true` for any non-expired denial shape (missing / mac / mismatch / denied).
    pub fn is_deny(&self) -> bool {
        !self.is_expired()
    }
}

/// Issues and verifies [`ToolPermitToken`]s with HMAC-SHA256.
///
/// Cheap to clone via [`Arc`]; one issuer per process (or per
/// agent-loop) is the usual deployment. Keys never leave process
/// memory — these tokens are not meant for cross-host transport
/// (kernel TileZero tokens cover that path).
#[derive(Debug, Clone)]
pub struct PermitIssuer {
    key: Arc<Vec<u8>>,
    default_ttl_ns: u64,
}

impl PermitIssuer {
    /// Build an issuer from raw key material (any length; HMAC-SHA256
    /// accepts arbitrary keys). The domain prefix is mixed in so a
    /// reused secret from another subsystem cannot forge permits.
    pub fn from_key(key: impl AsRef<[u8]>) -> Self {
        let mut material = KEY_DOMAIN.to_vec();
        material.extend_from_slice(key.as_ref());
        Self {
            key: Arc::new(material),
            default_ttl_ns: DEFAULT_PERMIT_TTL_NS,
        }
    }

    /// Ephemeral issuer with a process-local random-ish key.
    ///
    /// Uses a mix of a fixed seed and the current wall-clock nanos so
    /// two processes don't share a default key without coordination.
    /// Prefer [`Self::from_key`] when operators supply a secret.
    pub fn ephemeral() -> Self {
        let now = now_ns();
        let mut seed = b"ephemeral:".to_vec();
        seed.extend_from_slice(&now.to_le_bytes());
        // Mix in a process-id-ish value when available.
        seed.extend_from_slice(&std::process::id().to_le_bytes());
        Self::from_key(seed)
    }

    /// Override the default TTL used by [`Self::issue`].
    pub fn with_ttl_ns(mut self, ttl_ns: u64) -> Self {
        self.default_ttl_ns = ttl_ns;
        self
    }

    /// Default TTL in nanoseconds.
    pub fn default_ttl_ns(&self) -> u64 {
        self.default_ttl_ns
    }

    /// Issue a per-tool permit for `(agent_id, tool)` carrying the
    /// opaque gate token from
    /// [`GateDecision::Permit`](crate::agent::gate::GateDecision::Permit).
    ///
    /// `tool` is the bare tool name; the action string is derived as
    /// `tool.{tool}` to match the gate's action convention.
    pub fn issue(&self, agent_id: &str, tool: &str, gate_token: &str) -> ToolPermitToken {
        self.issue_at(agent_id, tool, gate_token, now_ns(), self.default_ttl_ns)
    }

    /// Issue with an explicit clock + TTL (testable).
    pub fn issue_at(
        &self,
        agent_id: &str,
        tool: &str,
        gate_token: &str,
        issued_at_ns: u64,
        ttl_ns: u64,
    ) -> ToolPermitToken {
        let mut token = ToolPermitToken {
            agent_id: agent_id.to_string(),
            tool: tool.to_string(),
            action: format!("tool.{tool}"),
            issued_at_ns,
            ttl_ns,
            gate_token: gate_token.to_string(),
            mac: String::new(),
        };
        token.mac = self.mac_hex(&token);
        token
    }

    /// Verify `token` as proof that `agent_id` may run `tool` *now*.
    pub fn verify(
        &self,
        token: &ToolPermitToken,
        agent_id: &str,
        tool: &str,
    ) -> Result<(), PermitProofError> {
        self.verify_at(token, agent_id, tool, now_ns())
    }

    /// Verify with an explicit clock (testable).
    pub fn verify_at(
        &self,
        token: &ToolPermitToken,
        agent_id: &str,
        tool: &str,
        now_ns: u64,
    ) -> Result<(), PermitProofError> {
        if token.tool != tool {
            return Err(PermitProofError::ToolMismatch {
                expected: token.tool.clone(),
                actual: tool.to_string(),
            });
        }
        if token.agent_id != agent_id {
            return Err(PermitProofError::AgentMismatch {
                expected: token.agent_id.clone(),
                actual: agent_id.to_string(),
            });
        }
        let expected = self.mac_hex(token);
        if !constant_time_eq(expected.as_bytes(), token.mac.as_bytes()) {
            return Err(PermitProofError::InvalidMac);
        }
        if !token.is_valid_time(now_ns) {
            return Err(PermitProofError::Expired {
                issued_at_ns: token.issued_at_ns,
                ttl_ns: token.ttl_ns,
                now_ns,
            });
        }
        Ok(())
    }

    fn mac_hex(&self, token: &ToolPermitToken) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .expect("HMAC-SHA256 accepts any key length");
        mac.update(&token.signable_content());
        hex_encode(&mac.finalize().into_bytes())
    }
}

impl Default for PermitIssuer {
    fn default() -> Self {
        Self::ephemeral()
    }
}

/// Proof-of-permission presentation API.
///
/// Tools and the agent loop call this before side-effecting work to
/// assert the gate-issued permit still holds. Behaviour:
///
/// - `token = None` and `required = false` → **allow** (back-compat)
/// - `token = None` and `required = true`  → **deny** ([`PermitProofError::Missing`])
/// - `token = Some(_)` → full verify via `issuer`
pub fn present_proof(
    issuer: &PermitIssuer,
    token: Option<&ToolPermitToken>,
    agent_id: &str,
    tool: &str,
    required: bool,
) -> Result<(), PermitProofError> {
    present_proof_at(issuer, token, agent_id, tool, required, now_ns())
}

/// [`present_proof`] with an explicit clock (testable).
pub fn present_proof_at(
    issuer: &PermitIssuer,
    token: Option<&ToolPermitToken>,
    agent_id: &str,
    tool: &str,
    required: bool,
    now_ns: u64,
) -> Result<(), PermitProofError> {
    match token {
        None if required => Err(PermitProofError::Missing),
        None => Ok(()),
        Some(t) => issuer.verify_at(t, agent_id, tool, now_ns),
    }
}

/// Current Unix time in nanoseconds (0 on clock failure).
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Lowercase hex encode without a `hex` crate dependency.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Constant-time equality for equal-length byte slices. Different
/// lengths short-circuit as unequal (lengths are public for hex MACs).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issuer() -> PermitIssuer {
        PermitIssuer::from_key(b"test-permit-key").with_ttl_ns(1_000_000_000)
    }

    // ── allow ──────────────────────────────────────────────────────

    #[test]
    fn allow_fresh_token_verifies() {
        let iss = issuer();
        let tok = iss.issue_at("agent-1", "write_file", "noop", 1_000, 5_000);
        assert!(iss.verify_at(&tok, "agent-1", "write_file", 1_000).is_ok());
        assert!(iss.verify_at(&tok, "agent-1", "write_file", 6_000).is_ok());
    }

    #[test]
    fn allow_present_proof_optional_none() {
        let iss = issuer();
        assert!(present_proof_at(&iss, None, "a", "echo", false, 0).is_ok());
    }

    #[test]
    fn allow_present_proof_with_token() {
        let iss = issuer();
        let tok = iss.issue_at("a", "echo", "kernel-permit", 10, 100);
        assert!(present_proof_at(&iss, Some(&tok), "a", "echo", true, 50).is_ok());
    }

    #[test]
    fn issue_binds_tool_action_and_gate_token() {
        let iss = issuer();
        let tok = iss.issue("agent-x", "shell_exec", "abcd");
        assert_eq!(tok.tool, "shell_exec");
        assert_eq!(tok.action, "tool.shell_exec");
        assert_eq!(tok.agent_id, "agent-x");
        assert_eq!(tok.gate_token, "abcd");
        assert!(!tok.mac.is_empty());
        assert_eq!(tok.mac.len(), 64); // sha256 hex
    }

    // ── deny ───────────────────────────────────────────────────────

    #[test]
    fn deny_missing_when_required() {
        let iss = issuer();
        let err = present_proof_at(&iss, None, "a", "echo", true, 0).unwrap_err();
        assert!(matches!(err, PermitProofError::Missing));
        assert!(err.is_deny());
        assert!(!err.is_expired());
    }

    #[test]
    fn deny_tool_mismatch() {
        let iss = issuer();
        let tok = iss.issue_at("a", "write_file", "t", 0, 100);
        let err = iss.verify_at(&tok, "a", "read_file", 0).unwrap_err();
        match &err {
            PermitProofError::ToolMismatch { expected, actual } => {
                assert_eq!(expected, "write_file");
                assert_eq!(actual, "read_file");
            }
            other => panic!("expected ToolMismatch, got {other:?}"),
        }
        assert!(err.is_deny());
    }

    #[test]
    fn deny_agent_mismatch() {
        let iss = issuer();
        let tok = iss.issue_at("agent-a", "echo", "t", 0, 100);
        let err = iss.verify_at(&tok, "agent-b", "echo", 0).unwrap_err();
        match &err {
            PermitProofError::AgentMismatch { expected, actual } => {
                assert_eq!(expected, "agent-a");
                assert_eq!(actual, "agent-b");
            }
            other => panic!("expected AgentMismatch, got {other:?}"),
        }
        assert!(err.is_deny());
    }

    #[test]
    fn deny_invalid_mac() {
        let iss = issuer();
        let mut tok = iss.issue_at("a", "echo", "t", 0, 100);
        tok.mac = "00".repeat(32);
        let err = iss.verify_at(&tok, "a", "echo", 0).unwrap_err();
        assert!(matches!(err, PermitProofError::InvalidMac));
    }

    #[test]
    fn deny_tampered_gate_token_breaks_mac() {
        let iss = issuer();
        let mut tok = iss.issue_at("a", "echo", "real-token", 0, 100);
        tok.gate_token = "forged".into();
        // MAC still covers original content fields including gate_token
        // so recompute would differ — verify must fail.
        let err = iss.verify_at(&tok, "a", "echo", 0).unwrap_err();
        assert!(matches!(err, PermitProofError::InvalidMac));
    }

    #[test]
    fn deny_explicit_denied_variant() {
        let err = PermitProofError::Denied {
            reason: "policy blocked".into(),
        };
        assert!(err.is_deny());
        assert!(!err.is_expired());
        assert!(err.to_string().contains("policy blocked"));
    }

    #[test]
    fn deny_wrong_issuer_key() {
        let a = PermitIssuer::from_key(b"key-a");
        let b = PermitIssuer::from_key(b"key-b");
        let tok = a.issue_at("x", "echo", "t", 0, 100);
        let err = b.verify_at(&tok, "x", "echo", 0).unwrap_err();
        assert!(matches!(err, PermitProofError::InvalidMac));
    }

    // ── expired ────────────────────────────────────────────────────

    #[test]
    fn expired_past_ttl() {
        let iss = issuer();
        let tok = iss.issue_at("a", "echo", "t", 1_000, 100);
        // valid through 1_100 inclusive; 1_101 is expired
        assert!(iss.verify_at(&tok, "a", "echo", 1_100).is_ok());
        let err = iss.verify_at(&tok, "a", "echo", 1_101).unwrap_err();
        match &err {
            PermitProofError::Expired {
                issued_at_ns,
                ttl_ns,
                now_ns,
            } => {
                assert_eq!(*issued_at_ns, 1_000);
                assert_eq!(*ttl_ns, 100);
                assert_eq!(*now_ns, 1_101);
            }
            other => panic!("expected Expired, got {other:?}"),
        }
        assert!(err.is_expired());
        assert!(!err.is_deny());
    }

    #[test]
    fn expired_present_proof() {
        let iss = issuer();
        let tok = iss.issue_at("a", "echo", "t", 0, 10);
        let err = present_proof_at(&iss, Some(&tok), "a", "echo", true, 11).unwrap_err();
        assert!(err.is_expired());
    }

    #[test]
    fn is_valid_time_boundary() {
        let tok = ToolPermitToken {
            agent_id: "a".into(),
            tool: "t".into(),
            action: "tool.t".into(),
            issued_at_ns: 50,
            ttl_ns: 10,
            gate_token: "g".into(),
            mac: String::new(),
        };
        assert!(tok.is_valid_time(50));
        assert!(tok.is_valid_time(60));
        assert!(!tok.is_valid_time(61));
        assert_eq!(tok.expires_at_ns(), 60);
    }

    // ── serde round-trip ───────────────────────────────────────────

    #[test]
    fn serde_round_trip() {
        let iss = issuer();
        let tok = iss.issue_at("agent", "web_fetch", "deadbeef", 42, 99);
        let json = serde_json::to_string(&tok).unwrap();
        let restored: ToolPermitToken = serde_json::from_str(&json).unwrap();
        assert_eq!(tok, restored);
        // Restored token still verifies with the same issuer.
        assert!(iss.verify_at(&restored, "agent", "web_fetch", 42).is_ok());
    }
}
