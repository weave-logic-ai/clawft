//! Interactive defer (human-in-the-loop) seam — WEFT-331.
//!
//! When the effect gate returns [`GateDecision::Defer`](super::gate::GateDecision::Defer),
//! the agent loop optionally consults a [`DeferInteractor`] instead of
//! immediately returning `{"deferred":true,"reason":…}` to the model.
//!
//! # Contract
//!
//! 1. **No interactor attached** — legacy v1 behaviour: structured
//!    deferred tool-result; the loop continues so the model can re-plan.
//! 2. **Interactor attached** — the loop suspends until the interactor
//!    returns a [`DeferOutcome`]. On [`DeferOutcome::Allow`] the tool
//!    proceeds through sandbox + dispatch; on deny / cancel / timeout
//!    the tool does **not** run and a structured deny (or cancelled)
//!    envelope is returned to the model.
//!
//! # Timeout / default-deny
//!
//! Interactive waiters **must** enforce a wall-clock timeout (default
//! [`clawft_types::agent_chat::DEFER_DEFAULT_TIMEOUT_MS`]). On expiry
//! return [`DeferOutcome::Timeout`] so the loop default-denies the tool
//! rather than hanging the turn forever. Cancel tokens (WEFT-323)
//! also abort the wait — mapped to [`DeferOutcome::Cancelled`].

use async_trait::async_trait;
use clawft_plugin::CancellationToken;
use clawft_types::agent_chat::{DEFER_DEFAULT_TIMEOUT_MS, DeferPromptEvent, DeferUserDecision};

/// Request the interactor presents to a human (or automated policy).
#[derive(Debug, Clone)]
pub struct DeferRequest {
    /// Conversation id of the suspended turn.
    pub conv_id: String,
    /// Tool name the gate deferred.
    pub tool: String,
    /// Gate reason.
    pub reason: String,
    /// Truncated JSON args for UI preview.
    pub arguments_preview: String,
    /// Timeout budget for this wait (ms).
    pub timeout_ms: u64,
}

impl DeferRequest {
    /// Build a wire [`DeferPromptEvent`] once the broker assigns an id.
    pub fn into_prompt_event(self, defer_id: impl Into<String>) -> DeferPromptEvent {
        DeferPromptEvent {
            defer_id: defer_id.into(),
            conv_id: self.conv_id,
            tool: self.tool,
            reason: self.reason,
            arguments_preview: self.arguments_preview,
            timeout_ms: if self.timeout_ms == 0 {
                DEFER_DEFAULT_TIMEOUT_MS
            } else {
                self.timeout_ms
            },
            ts_ms: None,
        }
    }
}

/// Outcome of an interactive defer wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeferOutcome {
    /// Human (or policy) approved — run the tool.
    Allow,
    /// Explicit refuse — tool does not run.
    Deny {
        /// Reason echoed into the tool-result envelope.
        reason: String,
    },
    /// Human cancelled the defer prompt — tool does not run.
    Cancel,
    /// Wait timed out — **default-deny** path.
    Timeout,
    /// Per-conv cancel token tripped while waiting.
    Cancelled,
}

impl DeferOutcome {
    /// Map a wire [`DeferUserDecision`] into an outcome (no timeout/cancel).
    pub fn from_user(decision: DeferUserDecision, deny_reason: impl Into<String>) -> Self {
        match decision {
            DeferUserDecision::Allow => Self::Allow,
            DeferUserDecision::Deny => Self::Deny {
                reason: deny_reason.into(),
            },
            DeferUserDecision::Cancel => Self::Cancel,
        }
    }

    /// Build the structured tool-result body for non-allow outcomes.
    ///
    /// Allow is not represented here — the caller falls through to
    /// sandbox + dispatch instead.
    pub fn tool_result_json(&self, gate_reason: &str) -> Option<String> {
        match self {
            Self::Allow => None,
            Self::Deny { reason } => Some(
                serde_json::json!({
                    "denied": true,
                    "reason": reason,
                })
                .to_string(),
            ),
            Self::Cancel => Some(
                serde_json::json!({
                    "deferred": true,
                    "cancelled": true,
                    "reason": format!("user cancelled defer: {gate_reason}"),
                })
                .to_string(),
            ),
            Self::Timeout => Some(
                serde_json::json!({
                    "denied": true,
                    "reason": format!("defer timed out (default-deny): {gate_reason}"),
                    "timeout": true,
                })
                .to_string(),
            ),
            Self::Cancelled => Some(
                serde_json::json!({
                    "denied": true,
                    "reason": format!("defer aborted (turn cancelled): {gate_reason}"),
                    "cancelled": true,
                })
                .to_string(),
            ),
        }
    }
}

/// Human-in-the-loop hook consulted on [`GateDecision::Defer`](super::gate::GateDecision::Defer).
///
/// Implementations **must** be cancel-safe: when `cancel` is tripped the
/// future should resolve promptly with [`DeferOutcome::Cancelled`].
#[async_trait]
pub trait DeferInteractor: Send + Sync + 'static {
    /// Suspend until a human decision (or timeout / cancel).
    async fn await_decision(
        &self,
        request: DeferRequest,
        cancel: &CancellationToken,
    ) -> DeferOutcome;
}

/// Test / auto-approve interactor that always allows.
#[derive(Debug, Default, Clone, Copy)]
pub struct AlwaysAllowDefer;

#[async_trait]
impl DeferInteractor for AlwaysAllowDefer {
    async fn await_decision(
        &self,
        _request: DeferRequest,
        _cancel: &CancellationToken,
    ) -> DeferOutcome {
        DeferOutcome::Allow
    }
}

/// Test interactor that always denies with a fixed reason.
#[derive(Debug, Clone)]
pub struct AlwaysDenyDefer {
    /// Deny reason stamped into the tool result.
    pub reason: String,
}

#[async_trait]
impl DeferInteractor for AlwaysDenyDefer {
    async fn await_decision(
        &self,
        _request: DeferRequest,
        _cancel: &CancellationToken,
    ) -> DeferOutcome {
        DeferOutcome::Deny {
            reason: self.reason.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_has_no_tool_result_envelope() {
        assert!(DeferOutcome::Allow.tool_result_json("x").is_none());
    }

    #[test]
    fn timeout_is_default_deny() {
        let body = DeferOutcome::Timeout
            .tool_result_json("needs review")
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["denied"], true);
        assert_eq!(v["timeout"], true);
        assert!(v["reason"].as_str().unwrap().contains("timed out"));
    }

    #[test]
    fn from_user_maps_decisions() {
        assert_eq!(
            DeferOutcome::from_user(DeferUserDecision::Allow, "r"),
            DeferOutcome::Allow
        );
        assert_eq!(
            DeferOutcome::from_user(DeferUserDecision::Cancel, "r"),
            DeferOutcome::Cancel
        );
    }

    #[tokio::test]
    async fn always_allow_interactor() {
        let out = AlwaysAllowDefer
            .await_decision(
                DeferRequest {
                    conv_id: "c".into(),
                    tool: "echo".into(),
                    reason: "review".into(),
                    arguments_preview: "{}".into(),
                    timeout_ms: 1_000,
                },
                &CancellationToken::new(),
            )
            .await;
        assert_eq!(out, DeferOutcome::Allow);
    }
}
