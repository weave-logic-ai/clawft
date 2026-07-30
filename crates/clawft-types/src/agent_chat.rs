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
    /// - `tool_choice: string | object` — OpenAI `tool_choice` override
    ///   (`"auto"`/`"none"`/`"required"` or
    ///   `{"type":"function","function":{"name":…}}`). Requests a tool
    ///   selection for a turn; absent leaves it to the model. Enforcement
    ///   is provider-side (llama-server honors it only probabilistically
    ///   when a system prompt is present).
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

/// Summary of one subagent spawned during a chat turn (M4 D8).
///
/// Emitted when the agent's tool loop calls `agent_spawn`. Lets the
/// CLI / GUI render "agent kicked off N subagents (M done, K running)"
/// without re-deriving the spawn tree from the forest. `status` is the
/// stringified `TaskStatus` (`"running"` / `"completed"` / `"failed"` /
/// `"exhausted"` / `"cancelled"` / `"denied"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpawnedTaskSummary {
    /// Spawn task id (`SpawnHandle.task_id`).
    pub task_id: String,
    /// Child conversation id (`"sub:<parent>:<ulid>"`).
    pub child_conv_id: String,
    /// Task status at the time the parent turn finished.
    pub status: String,
}

/// One gate denial that contributed to a WEFT-345 escalation streak.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateDenialRecord {
    /// Tool name that the gate refused (e.g. `"write_file"`).
    pub tool: String,
    /// Human-readable deny reason from the effect gate.
    pub reason: String,
}

/// Well-known `finish_reason` when the tool loop escalates after
/// consecutive gate denials (WEFT-345).
///
/// Matches the wire string panels / RPC clients should switch on.
pub const FINISH_REASON_ESCALATE_TO_HUMAN: &str = "escalate_to_human";

/// Canonical governance decision name for the WEFT-345 path — same
/// spelling as `clawft_kernel::governance::GovernanceDecision::EscalateToHuman`.
pub const GOVERNANCE_DECISION_ESCALATE_TO_HUMAN: &str = "EscalateToHuman";

/// Governance escalation event when the agent loop hits the consecutive
/// gate-denial threshold (WEFT-345).
///
/// Surfaced on [`AgentLoopResultMeta::escalation`] / [`AgentChatResult::escalation`]
/// so panels can render an escalation prompt (W-UI/15) and a future
/// allow/abort/refine RPC can resume the turn. Until that RPC lands,
/// the loop still **halts** the turn after emitting this event — it
/// no longer fails silently with only a log line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscalateToHumanEvent {
    /// Always [`GOVERNANCE_DECISION_ESCALATE_TO_HUMAN`].
    pub decision: String,
    /// Number of consecutive gate denials that triggered escalation.
    pub denial_count: u32,
    /// Conversation the loop was processing.
    pub conv_id: String,
    /// Ordered denials (oldest first) that formed the streak.
    pub denials: Vec<GateDenialRecord>,
    /// Human-readable summary for logs / panel fallback text.
    pub summary: String,
}

impl EscalateToHumanEvent {
    /// Build a WEFT-345 escalation event from a denial streak.
    pub fn from_denials(conv_id: impl Into<String>, denials: Vec<GateDenialRecord>) -> Self {
        let conv_id = conv_id.into();
        let denial_count = denials.len() as u32;
        let last = denials.last();
        let summary = match last {
            Some(d) => format!(
                "agent: gate denied tool calls {denial_count}x; escalating to human \
                 (EscalateToHuman). Last: `{}`: {}",
                d.tool, d.reason
            ),
            None => format!(
                "agent: gate denied tool calls {denial_count}x; escalating to human \
                 (EscalateToHuman)."
            ),
        };
        Self {
            decision: GOVERNANCE_DECISION_ESCALATE_TO_HUMAN.to_string(),
            denial_count,
            conv_id,
            denials,
            summary,
        }
    }
}

/// Result of `agent.chat`.
///
/// Since WEFT-328 the loop threads a real [`AgentLoopResultMeta`] through
/// `OutboundMessage.metadata` under [`AGENT_LOOP_RESULT_META_KEY`], so
/// `tool_calls`, `prompt_tokens`, `completion_tokens`, `model`, and
/// `identity_source` carry actual values on the normal agent-loop path.
/// When the meta key is absent (non-loop outbound, older producer) the
/// agent service's `dispatch` falls back to C1-compatible defaults
/// (empty tool_calls, 0 tokens, `None` model/identity_source).
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
    /// Echoed model name (best-effort; from the provider response).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Identity descriptor surfaced to the panel — diagnostic for the
    /// drift-warning path (e.g. `"clawft"`, `"docs-fallback"`). Sourced
    /// from the turn's identity loader via the loop meta (WEFT-328).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_source: Option<String>,
    /// Model reasoning captured across the tool loop (Hermes `<think>` /
    /// llama.cpp `reasoning_content`), when the provider produced any —
    /// surfaced for observability (`weft voice watch`), never part of the
    /// spoken/printed reply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Subagents spawned during the turn (M4 D8). Absent on the wire
    /// when empty; older clients that omit it deserialize to an empty
    /// vec, so the field is fully backward-compatible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spawned_tasks: Vec<SpawnedTaskSummary>,
    /// WEFT-345: present when the tool loop stopped after consecutive
    /// gate denials and emitted `EscalateToHuman`. Absent on the wire
    /// when `None` so older panels ignore the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalation: Option<EscalateToHumanEvent>,
}

/// Well-known `OutboundMessage.metadata` key under which the daemon
/// agent loop stashes the enriched turn result (M4 D8 / WEFT-328).
///
/// `OutboundMessage` is a generic bus envelope with no first-class slots
/// for tool-call summaries, token counts, model, or identity source.
/// Rather than widen the envelope, the loop serializes an
/// [`AgentLoopResultMeta`] into the free-form `metadata` map under this
/// key; the agent service's `result_from_outbound` reads it back.
/// Mirrors the `AgentChatParams.metadata` precedent used inbound.
pub const AGENT_LOOP_RESULT_META_KEY: &str = "agent_loop_result";

/// Enriched loop-result summary threaded from the tool loop (in
/// `clawft-core`) to the agent service's `result_from_outbound` via
/// `OutboundMessage.metadata[AGENT_LOOP_RESULT_META_KEY]` (M4 D8 /
/// WEFT-328).
///
/// Every field is `#[serde(default)]` so a missing key or a partial
/// object degrades to the pre-M4 / C1 defaults rather than failing to
/// deserialize — keeping the panel wire shape backward-compatible.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentLoopResultMeta {
    /// Tool calls executed during the loop, in order.
    #[serde(default)]
    pub tool_calls: Vec<AgentChatToolCall>,
    /// Why the loop terminated (`"stop"`, `"max_iterations"`, …).
    #[serde(default)]
    pub finish_reason: String,
    /// Number of LLM round-trips inside the loop.
    #[serde(default)]
    pub iterations: u32,
    /// Subagents spawned during the turn.
    #[serde(default)]
    pub spawned_tasks: Vec<SpawnedTaskSummary>,
    /// Model that actually answered (from the provider response), when the
    /// pipeline reported one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Cumulative prompt tokens across the loop's LLM calls.
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Cumulative completion tokens across the loop's LLM calls.
    #[serde(default)]
    pub completion_tokens: u32,
    /// Reasoning captured across the loop (see
    /// [`AgentChatResult::reasoning`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Identity loader source for this turn (e.g. `"clawft"`). Populated
    /// when a `SystemPromptBuilder` successfully loads identity; absent
    /// when no builder is attached or the load failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_source: Option<String>,
    /// WEFT-345: consecutive gate-denial → `EscalateToHuman` event.
    /// Absent when the turn ended normally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalation: Option<EscalateToHumanEvent>,
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
    /// Optional per-utterance voice decomposition (Wave 1 §W1.2). Carried
    /// verbatim through `index_turn` to the sibling `voice_analysis` node
    /// metadata key (served verbatim by `conversation.graph`); its `emotion`
    /// sub-blob overrides the classification emotion axis with `tier:"voice"`
    /// (the voice > llm > keyword confidence hierarchy). Absent for text turns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_analysis: Option<serde_json::Value>,
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

// ── agent.chat_stream (WEFT-253) ─────────────────────────────────
//
// Progressive companion to `agent.chat`. Same request params; the
// daemon publishes incremental frames at
// `substrate/_derived/chat/<conv_id>/stream` while the turn runs, and
// the RPC response is the final [`AgentChatResult`] (identical shape
// to `agent.chat`). Panels poll the stream path (or relay it via
// substrate snapshot) to render tokens as they land, then apply the
// final response when the oneshot reply arrives.
//
// Native + WASM both work because the progressive channel is the
// substrate (already polled on both transports); the RPC itself stays
// request/response — no connection-takeover required on the panel
// wire. True per-token LLM streaming lands later as a daemon-side
// enhancement that writes the same frame shape mid-generation.

/// Substrate path for the progressive stream frame of a conversation.
///
/// Sibling of `status` / `meta` under the same `_derived/chat/<conv>/`
/// parent so the daemon's existing `chat` `DerivedWriteGrant` covers
/// it. Heartbeat's 2s `Replace` at `status` never clobbers stream
/// text.
pub fn chat_stream_path(conv_id: &str) -> String {
    format!("substrate/_derived/chat/{conv_id}/stream")
}

/// One progressive frame written to [`chat_stream_path`].
///
/// The panel treats `text` as the **accumulated** assistant draft
/// (not a delta), so a missed frame is self-healing — the next frame
/// carries the full string so far. `seq` is a monotonic counter the
/// panel can use to ignore stale out-of-order reads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentChatStreamFrame {
    /// Accumulated assistant text so far (empty while thinking /
    /// tool-running before the first token).
    #[serde(default)]
    pub text: String,
    /// Coarse phase for the heartbeat-style status label:
    /// `"thinking"` / `"generating"` / `"done"` / `"error"`.
    pub phase: String,
    /// Monotonic frame counter within this turn.
    pub seq: u64,
    /// True once the turn has terminated (success or error). The
    /// panel freezes the draft and waits for the RPC reply (or
    /// surfaces `error` when set).
    #[serde(default)]
    pub done: bool,
    /// Optional tool name when `phase == "tool"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// Set when `done && phase == "error"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Wall-clock ms the frame was published (panel age label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts_ms: Option<u64>,
}

impl AgentChatStreamFrame {
    /// Build a "thinking / waiting for model" frame.
    pub fn thinking(seq: u64) -> Self {
        Self {
            text: String::new(),
            phase: "thinking".into(),
            seq,
            done: false,
            tool_name: None,
            error: None,
            ts_ms: None,
        }
    }

    /// Build a progressive draft frame (`phase = "generating"`).
    pub fn generating(seq: u64, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            phase: "generating".into(),
            seq,
            done: false,
            tool_name: None,
            error: None,
            ts_ms: None,
        }
    }

    /// Build the terminal success frame (full text, `done = true`).
    pub fn done_ok(seq: u64, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            phase: "done".into(),
            seq,
            done: true,
            tool_name: None,
            error: None,
            ts_ms: None,
        }
    }

    /// Build the terminal error frame.
    pub fn done_err(seq: u64, error: impl Into<String>) -> Self {
        Self {
            text: String::new(),
            phase: "error".into(),
            seq,
            done: true,
            tool_name: None,
            error: Some(error.into()),
            ts_ms: None,
        }
    }
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
            reasoning: None,
            spawned_tasks: Vec::new(),
            escalation: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("\"model\""));
        assert!(!json.contains("\"identity_source\""));
        assert!(!json.contains("\"reasoning\""));
        assert!(json.contains("\"tool_calls\":[]"));
        // Empty spawned_tasks must not appear on the wire (backward-compat).
        assert!(!json.contains("\"spawned_tasks\""));
        assert!(!json.contains("\"escalation\""));
    }

    #[test]
    fn chat_stream_path_is_sibling_of_status() {
        let p = chat_stream_path("panel-1");
        assert_eq!(p, "substrate/_derived/chat/panel-1/stream");
        assert!(p.starts_with("substrate/_derived/chat/"));
        assert!(p.ends_with("/stream"));
    }

    #[test]
    fn stream_frame_round_trips_and_helpers() {
        let f = AgentChatStreamFrame::generating(3, "hel");
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["phase"], "generating");
        assert_eq!(v["text"], "hel");
        assert_eq!(v["seq"], 3);
        assert_eq!(v["done"], false);
        let back: AgentChatStreamFrame = serde_json::from_value(v).unwrap();
        assert_eq!(back, f);

        let done = AgentChatStreamFrame::done_ok(4, "hello");
        assert!(done.done);
        assert_eq!(done.phase, "done");

        let err = AgentChatStreamFrame::done_err(5, "boom");
        assert!(err.done);
        assert_eq!(err.error.as_deref(), Some("boom"));
    }

    #[test]
    fn agent_chat_result_serializes_spawned_tasks_when_present() {
        let r = AgentChatResult {
            assistant_text: "kicked off work".into(),
            tool_calls: Vec::new(),
            finish_reason: "stop".into(),
            iterations: 2,
            prompt_tokens: 0,
            completion_tokens: 0,
            model: None,
            identity_source: None,
            reasoning: None,
            spawned_tasks: vec![SpawnedTaskSummary {
                task_id: "task-1".into(),
                child_conv_id: "sub:P:01HQ".into(),
                status: "completed".into(),
            }],
            escalation: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"spawned_tasks\""));
        assert!(json.contains("\"task-1\""));
        // Round-trips back to the same summary.
        let back: AgentChatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.spawned_tasks.len(), 1);
        assert_eq!(back.spawned_tasks[0].status, "completed");
    }

    #[test]
    fn agent_chat_result_old_client_without_spawned_tasks_deserializes() {
        // A pre-M4 payload omits spawned_tasks entirely; it must
        // deserialize to an empty vec, not fail.
        let json = r#"{
            "assistant_text": "hi",
            "tool_calls": [],
            "finish_reason": "stop",
            "iterations": 1,
            "prompt_tokens": 0,
            "completion_tokens": 0
        }"#;
        let r: AgentChatResult = serde_json::from_str(json).unwrap();
        assert!(r.spawned_tasks.is_empty());
    }

    #[test]
    fn agent_loop_result_meta_round_trips_through_metadata_value() {
        // The loop stashes this as a serde_json::Value in
        // OutboundMessage.metadata; the service reads it back.
        let meta = AgentLoopResultMeta {
            tool_calls: vec![AgentChatToolCall {
                name: "agent_spawn".into(),
                arguments_preview: "{\"goal\":\"answer 2+2\"}".into(),
                result_preview: "{\"status\":\"completed\"}".into(),
                success: true,
            }],
            finish_reason: "stop".into(),
            iterations: 3,
            spawned_tasks: vec![SpawnedTaskSummary {
                task_id: "t1".into(),
                child_conv_id: "sub:P:01HQ".into(),
                status: "completed".into(),
            }],
            model: Some("hermes-4.3-36b".into()),
            prompt_tokens: 812,
            completion_tokens: 96,
            reasoning: Some("the user asked for a sum".into()),
            identity_source: Some("clawft".into()),
            escalation: None,
        };
        let value = serde_json::to_value(&meta).unwrap();
        let back: AgentLoopResultMeta = serde_json::from_value(value).unwrap();
        assert_eq!(back.iterations, 3);
        assert_eq!(back.tool_calls.len(), 1);
        assert_eq!(back.spawned_tasks.len(), 1);
        assert_eq!(back.spawned_tasks[0].task_id, "t1");
        assert_eq!(back.model.as_deref(), Some("hermes-4.3-36b"));
        assert_eq!(back.prompt_tokens, 812);
        assert_eq!(back.completion_tokens, 96);
        assert_eq!(back.reasoning.as_deref(), Some("the user asked for a sum"));
        assert_eq!(back.identity_source.as_deref(), Some("clawft"));
        assert!(back.escalation.is_none());
    }

    #[test]
    fn agent_loop_result_meta_partial_object_uses_defaults() {
        // A partial/absent object degrades to defaults rather than
        // failing to deserialize — C1 panel shape stays tolerant.
        let back: AgentLoopResultMeta = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(back.iterations, 0);
        assert!(back.finish_reason.is_empty());
        assert!(back.tool_calls.is_empty());
        assert!(back.spawned_tasks.is_empty());
        assert_eq!(back.prompt_tokens, 0);
        assert_eq!(back.completion_tokens, 0);
        assert!(back.model.is_none());
        assert!(back.identity_source.is_none());
        assert!(back.escalation.is_none());
    }

    #[test]
    fn escalate_to_human_event_from_denials() {
        let event = EscalateToHumanEvent::from_denials(
            "conv-1",
            vec![
                GateDenialRecord {
                    tool: "write_file".into(),
                    reason: "policy".into(),
                },
                GateDenialRecord {
                    tool: "exec_shell".into(),
                    reason: "high risk".into(),
                },
                GateDenialRecord {
                    tool: "write_file".into(),
                    reason: "policy".into(),
                },
            ],
        );
        assert_eq!(event.decision, GOVERNANCE_DECISION_ESCALATE_TO_HUMAN);
        assert_eq!(event.denial_count, 3);
        assert_eq!(event.conv_id, "conv-1");
        assert_eq!(event.denials.len(), 3);
        assert!(event.summary.contains("EscalateToHuman"));
        assert!(event.summary.contains("3x"));
    }

    #[test]
    fn agent_chat_result_escalation_round_trips() {
        let r = AgentChatResult {
            assistant_text: "escalated".into(),
            tool_calls: vec![],
            finish_reason: FINISH_REASON_ESCALATE_TO_HUMAN.into(),
            iterations: 3,
            prompt_tokens: 10,
            completion_tokens: 5,
            model: None,
            identity_source: None,
            reasoning: None,
            spawned_tasks: vec![],
            escalation: Some(EscalateToHumanEvent::from_denials(
                "c",
                vec![GateDenialRecord {
                    tool: "echo".into(),
                    reason: "blocked".into(),
                }],
            )),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("EscalateToHuman"));
        let back: AgentChatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.finish_reason, FINISH_REASON_ESCALATE_TO_HUMAN);
        assert_eq!(back.escalation.as_ref().unwrap().decision, "EscalateToHuman");
    }

    #[test]
    fn agent_chat_result_deserializes_nonzero_token_and_identity_fields() {
        // WEFT-328: panel / clients must accept real (non-default) values
        // for the five fields plumbed through the loop meta.
        let json = r#"{
            "assistant_text": "done",
            "tool_calls": [{"name":"echo","arguments_preview":"{}","result_preview":"ok","success":true}],
            "finish_reason": "stop",
            "iterations": 2,
            "prompt_tokens": 40,
            "completion_tokens": 22,
            "model": "hermes-4.3-36b",
            "identity_source": "clawft"
        }"#;
        let r: AgentChatResult = serde_json::from_str(json).unwrap();
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.prompt_tokens, 40);
        assert_eq!(r.completion_tokens, 22);
        assert_eq!(r.model.as_deref(), Some("hermes-4.3-36b"));
        assert_eq!(r.identity_source.as_deref(), Some("clawft"));
    }
}
