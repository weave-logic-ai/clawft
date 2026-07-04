//! Wire-format types for the `agent.chat` RPC.
//!
//! These types are the canonical home for the `agent.chat` request/
//! response shape consumed by the daemon (`clawft-weave`), the agent
//! service (`clawft-service-agent`), and panels / tools that talk to
//! either. Hosting them in `clawft-types` (the foundation crate that
//! every other clawft crate already depends on) lets `clawft-weave`
//! avoid importing service crates just for serde, and keeps the wire
//! shape under a single ownership boundary.
//!
//! History: prior to WEFT-498, `clawft-weave::protocol::AgentChat*` and
//! `clawft-service-agent::protocol::AgentChat*` carried duplicate 1:1
//! mirrors of these definitions plus a `From` bridge between them. The
//! mirrors existed because `clawft-service-agent` was added late in the
//! agent-core-v1 phasing and re-exporting upstream from `clawft-weave`
//! would have inverted the dep direction. With this module in place,
//! both crates re-export from here and the bridge impls collapse to
//! identity.
//!
//! `default_conv_id` is the single ephemeral-id generator the wire
//! format relies on; both panels and the daemon must agree on the
//! `ephemeral-{ts:013}-{n:06}` shape to keep legacy panel behaviour
//! through Phase A of the agent-core rollout.

use serde::{Deserialize, Serialize};

/// One message in an `agent.chat` conversation.
///
/// `role` is one of `"system"` / `"user"` / `"assistant"`. The daemon
/// prepends its own system prompt; any `system` from the panel is
/// appended after it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatMessage {
    /// `system` / `user` / `assistant`.
    pub role: String,
    /// Message content.
    pub content: String,
}

/// Parameters for the `agent.chat` RPC.
///
/// The panel sends the full conversation each turn; cross-request
/// state is owned by the substrate-backed `ConversationSink`
/// (agent-core-v1 Phase C3).
///
/// The `conv_id` field defaults to an ephemeral id when omitted so
/// legacy panels keep working through Phase A; Phase C callers supply
/// a stable id and observe per-conv mutex behaviour in
/// `clawft_service_agent::AgentService`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatParams {
    /// Full conversation history. Last entry should be `user`.
    pub messages: Vec<AgentChatMessage>,
    /// Sampling temperature; daemon default when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Hard cap on generated tokens per LLM call inside the loop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Conversation identifier. Drives the per-conv `DashMap<ConvId, _>`
    /// in `clawft_service_agent::AgentService` and the substrate JSONL
    /// path `derived/chat/<conv_id>/turns/<ulid>`. Defaults to an
    /// ephemeral id so legacy panels keep working.
    #[serde(default = "default_conv_id")]
    pub conv_id: String,
    /// Free-form per-turn metadata carried to the daemon agent loop.
    ///
    /// Threaded into `InboundMessage.metadata` by
    /// `clawft_service_agent`'s `inbound_from_params`, giving the daemon
    /// loop the same per-turn context the in-process REPL injects
    /// directly. Documented known keys consumed by `loop_core`:
    /// - `skill_instructions: String` — injected as a system note.
    /// - `allowed_tools: [String]` — tool-subset filter.
    /// - `model: String` — model override, carried to the router.
    /// - `provenance: object` — diagnostic (e.g.
    ///   `{"impulse_source":"agent.chat"}`) for the witness/audit trail.
    ///
    /// Additive and fully backward-compatible: absent on the wire when
    /// `None`, and older callers that omit it deserialize to `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Default conversation id when the caller omits `conv_id`.
///
/// Generates an ephemeral, timestamp-prefixed monotonic string so
/// successive default-id calls within the same millisecond do not
/// collide. Callers that need stable per-conversation behaviour must
/// supply their own id; this default exists only to keep the legacy
/// panel wire format working through Phase A of the agent-core
/// rollout.
pub fn default_conv_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("ephemeral-{ts:013}-{n:06}")
}

/// Summary of one tool call the agent executed during a chat turn.
///
/// Renders as a collapsible bubble in the panel between user and
/// assistant turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatToolCall {
    /// Tool name (e.g. `"read_file"`, `"list_directory"`).
    pub name: String,
    /// JSON-stringified arguments, truncated for UI preview.
    pub arguments_preview: String,
    /// Tool result, truncated for UI preview.
    pub result_preview: String,
    /// True when the tool ran without error.
    pub success: bool,
}

/// Result of `agent.chat`.
///
/// Several fields (`tool_calls`, `prompt_tokens`, `completion_tokens`,
/// `model`, `identity_source`) cannot always be populated end-to-end
/// from a generic `OutboundMessage` envelope; the agent service's
/// `dispatch` returns best-effort defaults (empty vec, 0, None) when
/// the underlying loop result type does not carry the data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatResult {
    /// Final assistant text after the tool loop terminates.
    pub assistant_text: String,
    /// Tool calls executed during the loop, in order.
    pub tool_calls: Vec<AgentChatToolCall>,
    /// Why the loop terminated: `"stop"`, `"length"`,
    /// `"max_iterations"`, etc.
    pub finish_reason: String,
    /// Number of LLM round-trips inside the loop.
    pub iterations: u32,
    /// Cumulative prompt tokens across iterations.
    pub prompt_tokens: u32,
    /// Cumulative completion tokens across iterations.
    pub completion_tokens: u32,
    /// Echoed model name (best-effort).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Identity descriptor surfaced to the panel — diagnostic for the
    /// drift-warning path. Daemon injects the loaded source (e.g.
    /// `"docs-fallback"`) at the wire boundary; service-side dispatch
    /// leaves this `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_source: Option<String>,
}

/// One externally-produced turn for the `agent.turn.record` RPC.
///
/// Unlike `agent.chat`, the daemon does not run the LLM loop for these —
/// the caller (e.g. the `weft voice talk` Talk-Mode loop) already produced
/// the exchange and only needs it recorded through the daemon's
/// `ConversationSink` so the substrate JSONL + `KernelTurnAnchor`
/// side-effects (witness chain / HNSW / causal graph / session tier)
/// apply exactly as they do for `agent.chat` turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedTurn {
    /// `user` / `assistant` / `system`.
    pub role: String,
    /// Turn text (for voice: the STT transcript or the spoken answer).
    pub content: String,
    /// Wall-clock ms when the turn was produced; the daemon stamps
    /// receipt time when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts_ms: Option<u64>,
}

/// Parameters for the `agent.turn.record` RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnRecordParams {
    /// Conversation identifier (same keyspace as `agent.chat` conv_ids;
    /// voice uses its Talk-Mode conv_id, e.g. `weft-talk`).
    pub conv_id: String,
    /// Producing channel, for provenance (e.g. `"voice.talk"`).
    #[serde(default)]
    pub channel: String,
    /// Turns to record, in order. Must be non-empty.
    pub turns: Vec<RecordedTurn>,
}

/// Result of `agent.turn.record`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnRecordResult {
    /// How many turns were durably published to the substrate.
    pub recorded: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_chat_params_omitted_conv_id_gets_default() {
        let json = r#"{"messages":[{"role":"user","content":"hi"}]}"#;
        let params: AgentChatParams = serde_json::from_str(json).unwrap();
        assert!(
            params.conv_id.starts_with("ephemeral-"),
            "default conv_id must be ephemeral-shaped, got {:?}",
            params.conv_id
        );
        assert_eq!(params.messages.len(), 1);
    }

    #[test]
    fn agent_chat_params_explicit_conv_id_round_trips() {
        let json = r#"{"messages":[],"conv_id":"01HQ123ABCXYZ"}"#;
        let params: AgentChatParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.conv_id, "01HQ123ABCXYZ");
    }

    #[test]
    fn agent_chat_params_default_conv_ids_are_distinct() {
        let a = default_conv_id();
        let b = default_conv_id();
        assert_ne!(a, b);
    }

    #[test]
    fn agent_chat_result_skips_optional_fields_when_none() {
        let r = AgentChatResult {
            assistant_text: "hi".into(),
            tool_calls: Vec::new(),
            finish_reason: "stop".into(),
            iterations: 1,
            prompt_tokens: 0,
            completion_tokens: 0,
            model: None,
            identity_source: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("\"model\""));
        assert!(!json.contains("\"identity_source\""));
        assert!(json.contains("\"tool_calls\":[]"));
    }
}
