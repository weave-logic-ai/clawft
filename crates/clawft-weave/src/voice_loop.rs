//! Voice Wave 2 §W2.1 — the non-blocking voice→agent loop.
//!
//! Wires the (already-landed) interrupt brain into the daemon's recorded-turn
//! path. Every `user` turn that arrives via `agent.turn.record` is routed:
//!
//! ```text
//!   agent.turn.record (user)                       [gate: kernel.agent.voice_loop]
//!     │  1. busy read: talk_loop::current_turn(conv) — BEFORE the record
//!     │     (index_turn registers every anchored turn, overwriting the read)
//!     │  2. record as today (sink → anchor → index_turn, voice_analysis intact)
//!     ▼
//!   SessionTier::project_interrupt_signals ── InterruptRouter::route
//!     │
//!     ├─ Turn (idle)      → ReplySubmitter::submit_reply — register the reply
//!     │                     attempt Frontier (busy state), spawn the agent.chat
//!     │                     generation off-path; capture NEVER blocks on it
//!     ├─ Stop / Refine    → AgentService::execute_interrupt (§W2.3/§W2.4:
//!     │                     cancel → prune tombstone → Contradicts → witness)
//!     ├─ Backchannel      → no turn (Continuer impulse emission = WEFT-650)
//!     └─ Queue            → recorded, held behind the in-flight turn
//!                           (commit-drain = WEFT-650)
//! ```
//!
//! [`DaemonReplySubmitter`] is the §W2.1 register-early/commit-late reply path:
//! `submit_reply` mints the attempt node via
//! `SessionTier::register_reply_frontier` (metadata `goal` = the submitted
//! text, NO EndOfUtterance — the node stays Frontier, which IS the durable
//! busy state), then spawns the `agent.chat` dispatch. On finalize it emits
//! the commit (`commit_reply_frontier`); on cancel it leaves the node to the
//! interrupt executor's prune; on any other failure it prunes + witnesses so
//! no attempt dangles.

use std::sync::Weak;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use clawft_service_agent::interrupt_router::{InterruptAction, InterruptCtx, InterruptRouter};
use clawft_service_agent::{
    AgentChatMessage, AgentChatParams, AgentLoopHandle, AgentService, AgentServiceError,
    ReplySubmitter,
};

/// The §W2.1 register-early/commit-late reply submitter (daemon side).
///
/// Holds a `Weak` back-reference to the service that owns it (the service
/// holds the submitter via `set_reply_submitter`, so a strong reference here
/// would be an `Arc` cycle — the same shape as the subagent spawner's
/// late-wired `Weak`).
pub struct DaemonReplySubmitter<H: AgentLoopHandle> {
    agent: Weak<AgentService<H>>,
}

impl<H: AgentLoopHandle> DaemonReplySubmitter<H> {
    pub fn new(agent: Weak<AgentService<H>>) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl<H: AgentLoopHandle> ReplySubmitter for DaemonReplySubmitter<H> {
    async fn submit_reply(&self, conv_id: &str, goal_text: &str) -> Option<u64> {
        let agent = self.agent.upgrade()?;
        // Register the attempt Frontier first (this IS the busy state and the
        // prune/Contradicts target); degrade to dispatch-only when the tier or
        // forest isn't attached.
        let seq = agent
            .session_tier()
            .and_then(|tier| tier.register_reply_frontier(conv_id, goal_text));
        let params = AgentChatParams {
            messages: vec![AgentChatMessage {
                role: "user".into(),
                content: goal_text.to_string(),
            }],
            temperature: None,
            max_tokens: None,
            conv_id: conv_id.to_string(),
            // The spoken user turn already landed via `agent.turn.record`
            // (with its Wave-1 voice decomposition); the submitted goal text
            // is that turn (or, on a Refine, the synthetic goal+amendment the
            // attempt node's metadata carries) — recording it again as a
            // plain user turn would double it on the sink and the forest.
            metadata: Some(
                [(
                    "user_turn_recorded".to_string(),
                    serde_json::Value::Bool(true),
                )]
                .into_iter()
                .collect(),
            ),
        };
        // Off-path generation: capture never blocks on the reply (§W2.1).
        let conv = conv_id.to_string();
        tokio::spawn(async move {
            match agent.dispatch(params).await {
                Ok(_) => {
                    // Generation finalized — commit the attempt Frontier
                    // (commit-late). The reply text itself landed through the
                    // loop's sink → index_turn path as its own committed turn.
                    if let (Some(tier), Some(seq)) = (agent.session_tier(), seq) {
                        tier.commit_reply_frontier(&conv, seq);
                    }
                }
                Err(AgentServiceError::Cancelled(_)) => {
                    // A STOP/Refine interrupted this attempt: the executor
                    // owns the forest closure (prune tombstone + Contradicts
                    // + witness) — nothing to do here.
                    debug!(conv_id = %conv, "voice loop: reply attempt cancelled (executor owns the prune)");
                }
                Err(e) => {
                    // Genuine failure — prune + witness the dangling attempt
                    // so the forest never holds a silently-abandoned Frontier.
                    warn!(conv_id = %conv, error = %e, "voice loop: reply dispatch failed; pruning attempt");
                    if let (Some(tier), Some(seq)) = (agent.session_tier(), seq) {
                        let pruned = tier.emit_cancel_prune(&conv, Some(seq), None);
                        tier.witness_cancel(&conv, pruned);
                    }
                }
            }
        });
        seq
    }
}

/// The routing half: consumes one recorded user utterance and drives the
/// router → submitter/executor. Held in a daemon `OnceLock` — its presence IS
/// the `[kernel.agent].voice_loop` gate (set at boot only when the flag and
/// its `talk_loop` prerequisite hold).
pub struct VoiceLoop<H: AgentLoopHandle> {
    agent: Weak<AgentService<H>>,
    router: InterruptRouter,
}

impl<H: AgentLoopHandle> VoiceLoop<H> {
    pub fn new(agent: Weak<AgentService<H>>) -> Self {
        Self {
            agent,
            router: InterruptRouter::new(),
        }
    }

    /// Route one just-recorded `user` utterance. `in_flight_before` is the
    /// conversation's in-flight turn seq read BEFORE the utterance was
    /// recorded — the load-bearing busy axis (recording registers the user
    /// turn with the loop, overwriting `current_turn`).
    ///
    /// Fast by construction: idle turns and Refine resubmits spawn their
    /// generation off-path (`submit_reply`); Stop executes the cancel/prune/
    /// witness closure inline (sync graph work). Never fails the record —
    /// routing problems log and return.
    pub async fn route_user_turn(
        &self,
        conv_id: &str,
        text: &str,
        voice_analysis: Option<&serde_json::Value>,
        in_flight_before: Option<u64>,
    ) {
        let Some(agent) = self.agent.upgrade() else {
            return;
        };
        let Some(tier) = agent.session_tier() else {
            return;
        };
        let signals = tier.project_interrupt_signals(
            conv_id,
            text,
            voice_analysis,
            in_flight_before.is_some(),
        );
        let action = self.router.route(&signals);
        info!(
            conv_id,
            busy = signals.busy,
            intent = ?signals.intent,
            action = ?action,
            "voice loop: routed utterance"
        );
        match action {
            InterruptAction::Turn => {
                // Idle: a normal turn → generate the text reply, off-path.
                if let Some(submitter) = agent.reply_submitter() {
                    submitter.submit_reply(conv_id, text).await;
                } else {
                    warn!(conv_id, "voice loop: no reply submitter wired; turn recorded only");
                }
            }
            InterruptAction::Backchannel => {
                // Never a turn. The Continuer-crossref impulse emission is the
                // WEFT-650 (§W2.6) follow-up; the decomposition is already on
                // the recorded turn.
                debug!(conv_id, "voice loop: backchannel — agent keeps working");
            }
            InterruptAction::Queue => {
                // Recorded and held behind the in-flight turn. The
                // drain-on-commit executor is the WEFT-650 follow-up.
                debug!(conv_id, "voice loop: queued behind in-flight turn");
            }
            action @ (InterruptAction::Stop | InterruptAction::Refine { .. }) => {
                let outcome = agent
                    .execute_interrupt(
                        action,
                        InterruptCtx {
                            conv_id: conv_id.to_string(),
                            in_flight_seq: in_flight_before,
                        },
                    )
                    .await;
                info!(
                    conv_id,
                    pruned_seq = ?outcome.pruned_seq,
                    amendment_seq = ?outcome.amendment_seq,
                    contradicts = outcome.contradicts,
                    witnessed = outcome.witnessed,
                    "voice loop: interrupt executed"
                );
            }
        }
    }
}
