//! Wire-format types for agent service RPCs.
//!
//! Since WEFT-498 the canonical `agent.chat` definitions live in
//! [`clawft_types::agent_chat`]. This module re-exports them so
//! existing imports
//! (`clawft_service_agent::AgentChatParams`,
//! `clawft_service_agent::protocol::AgentChatResult`) keep working
//! without forcing every consumer to follow the new path.
//!
//! WEFT-324 adds the public `chain.append` surface (`WitnessRecord`,
//! [`ChainAppendParams`], [`ChainAppendResult`]) used by
//! `weaver soul promote` and the future agent-side soul-journal write
//! path.

use serde::{Deserialize, Serialize};

pub use clawft_types::agent_chat::{
    AgentChatDeferDecideParams, AgentChatDeferDecideResult, AgentChatError, AgentChatMessage,
    AgentChatParams, AgentChatResult, AgentChatToolCall, DEFER_DECISION_ALLOW,
    DEFER_DECISION_CANCEL, DEFER_DECISION_DENY, DEFER_DEFAULT_TIMEOUT_MS, DeferPromptEvent,
    DeferUserDecision, GATE_AGENT_ID_META_KEY, STREAM_PHASE_AWAITING_DEFER, SpawnedTaskSummary,
    caller_principal_name, chat_defer_path, default_conv_id,
};

// ── chain.append (WEFT-324) ───────────────────────────────────────

/// Witness audit record accepted by the public `chain.append` RPC.
///
/// Mirrors the soul-promote payload that was previously written only
/// to `<workspace>/.weftos/audit/soul-promote.log`. The daemon
/// serializes this struct as the chain event payload; `kind` becomes
/// the event kind on the exochain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessRecord {
    /// Event kind (e.g. `"soul.promote"`).
    pub kind: String,
    /// ULID of every journal entry that contributed to the merge
    /// (soul promote). Other writers may use this field for related
    /// opaque ids.
    pub entries: Vec<String>,
    /// SHA-256 hex of content *before* the witnessed mutation.
    pub hash_before: String,
    /// SHA-256 hex of content *after* the witnessed mutation.
    pub hash_after: String,
    /// ISO-8601 (UTC) wall-clock at witness time.
    pub ts: String,
}

fn default_chain_append_source() -> String {
    "soul".into()
}

/// Parameters for the public `chain.append` RPC (WEFT-324).
///
/// Capability: [`Write`](crate) — gated via
/// `clawft_weave::capability::required_capability("chain.append")`.
/// Local UDS clients receive an implicit `admin` token from
/// [`clawft_rpc::DaemonClient::call`](clawft_rpc).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainAppendParams {
    /// Event source label on the chain (e.g. `"soul"`, `"agent"`).
    #[serde(default = "default_chain_append_source")]
    pub source: String,
    /// Witness record. Its `kind` becomes the chain event kind; the
    /// full record is serialized as the event payload.
    pub record: WitnessRecord,
}

/// Result of a successful `chain.append`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainAppendResult {
    /// Sequence number assigned by the chain.
    pub sequence: u64,
    /// Chain ID (0 = local node chain).
    pub chain_id: u32,
    /// Hex-encoded event hash.
    pub hash: String,
    /// Echo of the source label.
    pub source: String,
    /// Echo of the event kind.
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_append_params_round_trip() {
        let params = ChainAppendParams {
            source: "soul".into(),
            record: WitnessRecord {
                kind: "soul.promote".into(),
                entries: vec!["01HZA".into()],
                hash_before: "aa".repeat(32),
                hash_after: "bb".repeat(32),
                ts: "2026-07-30T00:00:00Z".into(),
            },
        };
        let v = serde_json::to_value(&params).unwrap();
        let back: ChainAppendParams = serde_json::from_value(v).unwrap();
        assert_eq!(back, params);
    }

    #[test]
    fn chain_append_params_default_source() {
        let v = serde_json::json!({
            "record": {
                "kind": "soul.promote",
                "entries": [],
                "hash_before": "0".repeat(64),
                "hash_after": "1".repeat(64),
                "ts": "t"
            }
        });
        let p: ChainAppendParams = serde_json::from_value(v).unwrap();
        assert_eq!(p.source, "soul");
        assert_eq!(p.record.kind, "soul.promote");
    }
}
