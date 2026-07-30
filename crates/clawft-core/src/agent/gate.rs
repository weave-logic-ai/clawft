//! Effect gate seam.
//!
//! The agent loop calls [`EffectGate::check`] before each tool
//! dispatch with an [`EffectVector`](super::effects::EffectVector)
//! computed by [`effect_for_tool`](super::effects::effect_for_tool).
//! The gate decides whether the tool runs:
//!
//! - [`GateDecision::Permit`] — proceed with the tool execute.
//! - [`GateDecision::Defer`] — without a
//!   [`DeferInteractor`](super::defer::DeferInteractor): surface the
//!   reason as the tool result and continue so the model can re-plan.
//!   With an interactor (WEFT-331): suspend until allow / deny / cancel
//!   / timeout; allow falls through to sandbox + dispatch.
//! - [`GateDecision::Deny`]  — same handling as defer; the reason
//!   becomes the tool result and the loop continues. After
//!   [`GATE_DENIAL_ESCALATION_LIMIT`] consecutive Denys in one turn
//!   the loop emits [`EscalateToHumanEvent`](clawft_types::agent_chat::EscalateToHumanEvent)
//!   instead of silently burning the remaining tool-iteration budget
//!   (WEFT-345).
//!
//! Phase D2 wires a kernel-backed implementation that calls
//! [`clawft_kernel::gate::GateBackend::check`](../../../../clawft-kernel/src/gate.rs)
//! and maps the kernel's `EffectVector` ↔ ours. Until then the default
//! attached to [`AgentLoop`](super::loop_core::AgentLoop) is
//! [`NoopGate`], which always permits.
//!
//! The shape mirrors `clawft-kernel`'s
//! [`GateDecision`](../../../../clawft-kernel/src/gate.rs:14-34) so
//! the Phase D2 mapping is a one-liner per variant.

use async_trait::async_trait;
use clawft_types::agent_chat::GateDenialRecord;

use super::effects::EffectVector;

/// After this many consecutive gate [`GateDecision::Deny`] results
/// within one [`AgentLoop::run_tool_loop`](super::loop_core::AgentLoop)
/// turn, the loop surfaces `EscalateToHuman` (WEFT-345) rather than
/// continuing until max tool iterations.
///
/// Matches the planning circuit-breaker default of 3 (same as the
/// WEFT-651 identical-failure breaker).
pub const GATE_DENIAL_ESCALATION_LIMIT: u32 = 3;

/// Extract the deny reason from a tool-result body produced by
/// [`AgentLoop::execute_tool_with_guards`](super::loop_core::AgentLoop)
/// when the gate returned [`GateDecision::Deny`].
///
/// Returns `Some(reason)` only for the structured
/// `{"denied": true, "reason": ...}` shape — sandbox denials and
/// runtime errors use `"error"` and do **not** count toward the
/// WEFT-345 streak.
pub fn gate_denial_reason(result_json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(result_json).ok()?;
    let obj = value.as_object()?;
    if !obj.get("denied").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }
    let reason = obj
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("policy")
        .to_string();
    Some(reason)
}

/// Record one tool-result outcome against the consecutive gate-denial
/// streak (WEFT-345).
///
/// - Gate deny → push a [`GateDenialRecord`], increment `count`.
/// - Anything else (permit success, defer, sandbox/runtime error) →
///   reset the streak.
///
/// Returns `true` when `count >= limit` after this observation (the
/// caller should emit `EscalateToHuman` and halt the turn).
pub fn record_gate_denial_streak(
    count: &mut u32,
    denials: &mut Vec<GateDenialRecord>,
    tool: &str,
    result_json: &str,
    limit: u32,
) -> bool {
    match gate_denial_reason(result_json) {
        Some(reason) => {
            denials.push(GateDenialRecord {
                tool: tool.to_string(),
                reason,
            });
            *count = count.saturating_add(1);
            *count >= limit
        }
        None => {
            *count = 0;
            denials.clear();
            false
        }
    }
}

/// Outcome of an [`EffectGate::check`] call.
///
/// The `Permit` token is opaque — Phase D2's kernel-backed gate puts
/// the TileZero permit bytes here (encoded as base64 for readability).
/// Today the only producer is [`NoopGate`], which uses the literal
/// string `"noop"`.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum GateDecision {
    /// Action permitted; carry the opaque token forward to the
    /// witness chain entry the loop will write at audit time.
    Permit {
        /// Opaque permit token — implementation defined.
        token: String,
    },
    /// Decision deferred; carry the human-readable reason so the
    /// agent can summarise why for the model.
    Defer {
        /// Why the gate deferred.
        reason: String,
    },
    /// Action denied; carry the human-readable reason.
    Deny {
        /// Why the gate denied.
        reason: String,
    },
}

impl GateDecision {
    /// Returns `true` if the decision is `Permit`.
    pub fn is_permit(&self) -> bool {
        matches!(self, GateDecision::Permit { .. })
    }
}

/// Pre-execute policy gate. Implementations decide whether a tool
/// dispatch is allowed given the agent identity, the action name
/// (e.g. `tool.write_file`), and the action's effect vector.
#[async_trait]
pub trait EffectGate: Send + Sync + 'static {
    /// Inspect a proposed action and return the gate decision.
    ///
    /// Implementations MUST be cancel-safe — the caller can drop the
    /// future without leaving the gate in a broken state.
    async fn check(&self, agent_id: &str, action: &str, effect: &EffectVector) -> GateDecision;
}

/// Trivial gate that permits every action.
///
/// Default for [`AgentLoop`](super::loop_core::AgentLoop) so existing
/// callers see no behaviour change. Phase D2 swaps in the kernel-
/// backed implementation that enforces real policy.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopGate;

#[async_trait]
impl EffectGate for NoopGate {
    async fn check(&self, _agent_id: &str, _action: &str, _effect: &EffectVector) -> GateDecision {
        GateDecision::Permit {
            token: "noop".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn _coerce(_gate: &dyn EffectGate) {}

    #[tokio::test]
    async fn noop_gate_permits_everything() {
        let gate = NoopGate;
        let ev = EffectVector {
            risk: 0.99,
            security: 0.99,
            ..Default::default()
        };
        let decision = gate.check("agent-1", "tool.exec", &ev).await;
        assert!(decision.is_permit());
        match decision {
            GateDecision::Permit { token } => assert_eq!(token, "noop"),
            other => panic!("expected Permit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn noop_gate_permits_with_zero_effect() {
        let gate = NoopGate;
        let decision = gate
            .check("agent-1", "tool.read_file", &EffectVector::default())
            .await;
        assert!(decision.is_permit());
    }

    #[test]
    fn is_permit_matches_only_permit() {
        assert!(GateDecision::Permit { token: "x".into() }.is_permit());
        assert!(!GateDecision::Defer { reason: "x".into() }.is_permit());
        assert!(!GateDecision::Deny { reason: "x".into() }.is_permit());
    }

    #[test]
    fn gate_denial_reason_parses_structured_denied() {
        let body = r#"{"denied":true,"reason":"write blocked"}"#;
        assert_eq!(
            gate_denial_reason(body).as_deref(),
            Some("write blocked")
        );
    }

    #[test]
    fn gate_denial_reason_ignores_error_and_deferred() {
        assert!(gate_denial_reason(r#"{"error":"sandbox denied: x"}"#).is_none());
        assert!(gate_denial_reason(r#"{"deferred":true,"reason":"review"}"#).is_none());
        assert!(gate_denial_reason(r#"{"ok":true}"#).is_none());
        assert!(gate_denial_reason("not-json").is_none());
    }

    #[test]
    fn record_gate_denial_streak_trips_at_limit() {
        let mut count = 0;
        let mut denials = Vec::new();
        let deny = r#"{"denied":true,"reason":"no"}"#;
        assert!(!record_gate_denial_streak(
            &mut count,
            &mut denials,
            "a",
            deny,
            GATE_DENIAL_ESCALATION_LIMIT
        ));
        assert_eq!(count, 1);
        assert!(!record_gate_denial_streak(
            &mut count,
            &mut denials,
            "b",
            deny,
            GATE_DENIAL_ESCALATION_LIMIT
        ));
        assert_eq!(count, 2);
        assert!(record_gate_denial_streak(
            &mut count,
            &mut denials,
            "c",
            deny,
            GATE_DENIAL_ESCALATION_LIMIT
        ));
        assert_eq!(count, 3);
        assert_eq!(denials.len(), 3);
        assert_eq!(denials[2].tool, "c");
    }

    #[test]
    fn record_gate_denial_streak_resets_on_non_deny() {
        let mut count = 0;
        let mut denials = Vec::new();
        let deny = r#"{"denied":true,"reason":"no"}"#;
        assert!(!record_gate_denial_streak(
            &mut count,
            &mut denials,
            "a",
            deny,
            GATE_DENIAL_ESCALATION_LIMIT
        ));
        assert!(!record_gate_denial_streak(
            &mut count,
            &mut denials,
            "a",
            r#"{"ok":true}"#,
            GATE_DENIAL_ESCALATION_LIMIT
        ));
        assert_eq!(count, 0);
        assert!(denials.is_empty());
    }
}
