//! Voice Wave 2 §W2.1 — the non-blocking voice→agent loop.
//!
//! Wires the (already-landed) interrupt brain into the daemon's recorded-turn
//! path. Every `user` turn that arrives via `agent.turn.record` is routed:
//!
//! ```text
//!   agent.turn.record (user)                       [gate: kernel.agent.voice_loop]
//!     │  1. busy read: VoiceLoop::in_flight(conv) — the loop's OWN attempt
//!     │     tracker, read BEFORE the record (talk_loop::current_turn is
//!     │     overwritten by every anchored turn's registration)
//!     │  2. record as today (sink → anchor → index_turn, voice_analysis intact)
//!     ▼
//!   SessionTier::project_interrupt_signals ── InterruptRouter::route
//!     │
//!     ├─ Turn (idle)      → ReplySubmitter::submit_reply — register the reply
//!     │                     attempt Frontier (busy state), spawn the agent.chat
//!     │                     generation off-path; capture NEVER blocks on it
//!     ├─ Stop / Refine    → AgentService::execute_interrupt (§W2.3/§W2.4:
//!     │                     cancel → prune tombstone → Contradicts → witness)
//!     ├─ Backchannel      → no turn — executor emits `Backchannel 0x60`, the
//!     │                     loop draws a `Continuer` cross-ref (§W2.6)
//!     └─ Queue            → recorded, held behind the in-flight turn in the
//!                           shared [`VoiceQueues`]; the submitter drains one
//!                           on each reply commit/failure (§W2.6)
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

use std::collections::VecDeque;
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use dashmap::DashMap;
use tracing::{debug, info, warn};

use clawft_service_agent::interrupt_router::{InterruptAction, InterruptCtx, InterruptRouter};
use clawft_service_agent::{
    AgentChatMessage, AgentChatParams, AgentLoopHandle, AgentService, AgentServiceError,
    ReplySubmitter,
};

/// Bounded per-conversation FIFO of utterance texts routed to
/// [`InterruptAction::Queue`] (§W2.2 row 4, WEFT-650) — utterances that
/// arrived while the agent was busy on a topically-discontinuous ask. Held
/// behind the in-flight reply and drained one at a time once it finalizes
/// (commits) or fails; see [`enqueue_utterance`] and the drain in
/// [`DaemonReplySubmitter::submit_reply`]'s spawned dispatch task.
pub type VoiceQueues = DashMap<String, VecDeque<String>>;

/// The voice loop's OWN busy axis: conv → the in-flight reply attempt's seq.
///
/// `talk_loop::current_turn` is NOT a reliable busy read for the voice loop:
/// `index_turn` registers EVERY anchored turn (a mid-flight backchannel or
/// queued ask overwrites the pointer), and that turn's commit-tick then clears
/// it — wiping the busy state while the attempt is still generating (found
/// live, conv w26-probe). This map is maintained on the attempt lifecycle
/// only: set on register, cleared on finalize (commit/failure) and on a Stop
/// prune; a Refine resubmit overwrites it with the amendment's seq.
pub type InFlightAttempts = DashMap<String, u64>;

/// State shared between [`VoiceLoop`] (router half) and
/// [`DaemonReplySubmitter`] (dispatch half): the queued-utterance FIFOs and
/// the in-flight attempt tracker. One `Arc<VoiceShared>` per daemon, built at
/// boot and handed to both constructors.
#[derive(Default)]
pub struct VoiceShared {
    /// Queue-arm backlog, drained on reply finalize.
    pub queues: VoiceQueues,
    /// The busy axis (see [`InFlightAttempts`]).
    pub in_flight: InFlightAttempts,
}

impl VoiceShared {
    /// Clear `conv_id`'s in-flight entry only if it still names `seq` —
    /// finalize/prune paths use this so a stale task can never clobber the
    /// amendment that replaced it.
    fn clear_in_flight(&self, conv_id: &str, seq: u64) {
        self.in_flight.remove_if(conv_id, |_, v| *v == seq);
    }
}

/// Cap on a single conversation's queued backlog. This is a human speaking
/// backlog, not a work queue — a runaway talker drops the oldest utterance
/// (with a warning) rather than growing state without bound.
const VOICE_QUEUE_CAP: usize = 8;

/// Push `text` onto `conv_id`'s queue, dropping the oldest queued utterance
/// (with a `warn!`) once the conversation is already at [`VOICE_QUEUE_CAP`].
pub fn enqueue_utterance(queues: &VoiceQueues, conv_id: &str, text: &str) {
    let mut q = queues.entry(conv_id.to_string()).or_default();
    if q.len() >= VOICE_QUEUE_CAP {
        let dropped = q.pop_front();
        warn!(
            conv_id,
            dropped = ?dropped,
            cap = VOICE_QUEUE_CAP,
            "voice loop: queue full, dropping oldest queued utterance"
        );
    }
    q.push_back(text.to_string());
}

/// The §W2.1 register-early/commit-late reply submitter (daemon side).
///
/// Holds a `Weak` back-reference to the service that owns it (the service
/// holds the submitter via `set_reply_submitter`, so a strong reference here
/// would be an `Arc` cycle — the same shape as the subagent spawner's
/// late-wired `Weak`), plus the [`VoiceQueues`] shared with [`VoiceLoop`] so a
/// commit/failure can drain the conversation's next queued utterance
/// (WEFT-650).
pub struct DaemonReplySubmitter<H: AgentLoopHandle> {
    agent: Weak<AgentService<H>>,
    shared: Arc<VoiceShared>,
}

impl<H: AgentLoopHandle> DaemonReplySubmitter<H> {
    /// Back-compat constructor: private, unshared state. Nothing else can
    /// push onto its queue or read its busy axis — daemon wiring must use
    /// [`Self::with_shared`], sharing the same `Arc<VoiceShared>` passed to
    /// [`VoiceLoop::with_shared`].
    pub fn new(agent: Weak<AgentService<H>>) -> Self {
        Self::with_shared(agent, Arc::new(VoiceShared::default()))
    }

    /// Build a submitter sharing state with a [`VoiceLoop`] — the wiring the
    /// WEFT-650 Queue-arm drain and the busy axis depend on.
    pub fn with_shared(agent: Weak<AgentService<H>>, shared: Arc<VoiceShared>) -> Self {
        Self { agent, shared }
    }
}

#[async_trait]
impl<H: AgentLoopHandle> ReplySubmitter for DaemonReplySubmitter<H> {
    async fn submit_reply(&self, conv_id: &str, goal_text: &str) -> Option<u64> {
        let agent = self.agent.upgrade()?;
        dispatch_reply(
            agent,
            self.shared.clone(),
            conv_id.to_string(),
            goal_text.to_string(),
        )
        .await
    }
}

/// Voice-shaped reply system message (WEFT-659): every voice-loop dispatch
/// gets this so replies read as spoken conversation, not TEXT chat (live
/// feedback: markdown lists / "Could you please rephrase..." replies).
/// Spirit-parity with clawft-channels' `VoiceAnswerPolicy::system_policy`;
/// not imported (avoids a weave→channels dep).
const VOICE_REPLY_SYSTEM_MESSAGE: &str = "You are replying through a text-to-speech voice interface in a live spoken conversation. Reply in at most three short spoken sentences. Plain conversational prose only — no markdown, no lists, no headings, no code blocks. If you truly cannot parse the request, ask one brief clarifying question.";

/// Build a voice-loop dispatch's [`AgentChatParams`]: voice system message
/// leads `messages`, then the goal text, plus `user_turn_recorded` (§W2.1 —
/// stops double-recording the already-landed spoken turn). Pure fn.
///
/// SURPRISE: `inbound_from_params` (service.rs) only reads the LAST
/// `role == "user"` message off `messages`, silently dropping a leading
/// `system` one before it reaches the LLM. So the voice message is ALSO
/// threaded through `metadata["skill_instructions"]`, which `loop_core.rs`
/// ("4b.") actually splices in. Both must stay populated: `messages[0]` for
/// shape/tests, metadata for delivery.
fn voice_dispatch_params(conv_id: &str, goal_text: &str) -> AgentChatParams {
    let metadata = serde_json::Map::from_iter([
        (
            "user_turn_recorded".to_string(),
            serde_json::Value::Bool(true),
        ),
        (
            "skill_instructions".to_string(),
            serde_json::Value::String(VOICE_REPLY_SYSTEM_MESSAGE.to_string()),
        ),
    ]);
    let system = AgentChatMessage {
        role: "system".into(),
        content: VOICE_REPLY_SYSTEM_MESSAGE.into(),
    };
    let user = AgentChatMessage {
        role: "user".into(),
        content: goal_text.to_string(),
    };
    AgentChatParams {
        messages: vec![system, user],
        temperature: None,
        max_tokens: None,
        conv_id: conv_id.to_string(),
        metadata: Some(metadata),
    }
}

/// Register-early/commit-late reply dispatch (§W2.1), plus the WEFT-650
/// drain-on-finalize: once the spawned `agent.chat` generation commits or
/// fails, pop `conv_id`'s next queued utterance (if any) and resubmit it
/// through this same path. A `Cancelled` attempt does NOT drain — the
/// Refine resubmit that caused the cancel will drain when IT finalizes,
/// so a queued utterance is never double-drained by both the pruned
/// attempt and its replacement.
///
/// Free function (rather than a `&self` method) so the drain can recurse by
/// spawning a fresh call rather than needing an `Arc<Self>` handle back to
/// the submitter.
async fn dispatch_reply<H: AgentLoopHandle>(
    agent: Arc<AgentService<H>>,
    shared: Arc<VoiceShared>,
    conv_id: String,
    goal_text: String,
) -> Option<u64> {
    // Register the attempt Frontier first (this IS the busy state and the
    // prune/Contradicts target); degrade to dispatch-only when the tier or
    // forest isn't attached.
    let seq = match agent.session_tier() {
        Some(tier) => tier.register_reply_frontier(&conv_id, &goal_text).await,
        None => None,
    };
    // The voice loop's busy axis: this conv now has an in-flight attempt.
    // (A Refine resubmit lands here too, overwriting the pruned attempt's
    // entry with the amendment's seq.)
    if let Some(s) = seq {
        shared.in_flight.insert(conv_id.clone(), s);
    }
    // See `voice_dispatch_params` for the voice-shaping + double-record
    // guard this builds (§W2.1 / WEFT-659).
    let params = voice_dispatch_params(&conv_id, &goal_text);
    // Off-path generation: capture never blocks on the reply (§W2.1).
    let conv = conv_id.clone();
    tokio::spawn(async move {
        match agent.dispatch(params).await {
            Ok(_) => {
                // Generation finalized — commit the attempt Frontier
                // (commit-late). The reply text itself landed through the
                // loop's sink → index_turn path as its own committed turn.
                if let (Some(tier), Some(seq)) = (agent.session_tier(), seq) {
                    tier.commit_reply_frontier(&conv, seq);
                    shared.clear_in_flight(&conv, seq);
                }
                drain_next(agent, shared, conv);
            }
            Err(AgentServiceError::Cancelled(_)) => {
                // A STOP/Refine interrupted this attempt: the executor
                // owns the forest closure (prune tombstone + Contradicts
                // + witness) — nothing to do here, and no drain (the
                // resubmit that caused the cancel drains on ITS finalize).
                debug!(conv_id = %conv, "voice loop: reply attempt cancelled (executor owns the prune)");
            }
            Err(e) => {
                // Genuine failure — prune + witness the dangling attempt
                // so the forest never holds a silently-abandoned Frontier.
                warn!(conv_id = %conv, error = %e, "voice loop: reply dispatch failed; pruning attempt");
                if let (Some(tier), Some(seq)) = (agent.session_tier(), seq) {
                    let pruned = tier.emit_cancel_prune(&conv, Some(seq), None);
                    tier.witness_cancel(&conv, pruned);
                    shared.clear_in_flight(&conv, seq);
                }
                drain_next(agent, shared, conv);
            }
        }
    });
    seq
}

/// Pop `conv_id`'s next queued utterance (if any) and resubmit it through
/// [`dispatch_reply`] on a fresh spawned task — the WEFT-650 drain-on-commit.
/// A no-op when the conversation has nothing queued.
fn drain_next<H: AgentLoopHandle>(
    agent: Arc<AgentService<H>>,
    shared: Arc<VoiceShared>,
    conv_id: String,
) {
    let next = shared
        .queues
        .get_mut(&conv_id)
        .and_then(|mut q| q.pop_front());
    if let Some(text) = next {
        info!(conv_id = %conv_id, "voice loop: draining queued utterance on commit");
        tokio::spawn(async move {
            dispatch_reply(agent, shared, conv_id, text).await;
        });
    }
}

/// Non-lexical paralinguistic classes (WEFT-659): a listener sound, not a
/// request. Lock-step with `SessionTier::project_interrupt_signals`'s
/// widened `is_backchannel` (session_tier.rs) — same classes route to
/// `InterruptAction::Backchannel` while busy.
const NONLEXICAL_PARALINGUISTIC_CLASSES: [&str; 3] =
    ["backchannel_candidate", "laughter_candidate", "filler"];

/// `voice_analysis.paralinguistics.class`, if one of
/// [`NONLEXICAL_PARALINGUISTIC_CLASSES`]; `None` otherwise (incl. absent).
fn nonlexical_paralinguistic_class(voice_analysis: Option<&serde_json::Value>) -> Option<&str> {
    let class = voice_analysis?
        .get("paralinguistics")?
        .get("class")?
        .as_str()?;
    NONLEXICAL_PARALINGUISTIC_CLASSES
        .contains(&class)
        .then_some(class)
}

/// `voice_analysis.stt.token_conf_mean`, if present.
fn stt_token_conf_mean(voice_analysis: Option<&serde_json::Value>) -> Option<f64> {
    voice_analysis?.get("stt")?.get("token_conf_mean")?.as_f64()
}

/// `voice_analysis.audio.snr_db`, if present.
fn audio_snr_db(voice_analysis: Option<&serde_json::Value>) -> Option<f64> {
    voice_analysis?.get("audio")?.get("snr_db")?.as_f64()
}

/// `voice_analysis.audio.noise_floor_converged`, if present.
fn audio_noise_floor_converged(voice_analysis: Option<&serde_json::Value>) -> Option<bool> {
    voice_analysis?
        .get("audio")?
        .get("noise_floor_converged")?
        .as_bool()
}

/// Daemon-side clarity gate (WEFT-659). Mirrors the client rule EXACTLY —
/// `clawft-channels/src/voice/talkmode.rs`'s `utterance_clarity` is
/// canonical, the two MUST stay in lock-step (live feedback: token-conf mean
/// 0.44, SNR 6.4dB routed a full agent-loop reply to garbage STT). Missing
/// fields (no `voice_analysis`, no `stt.token_conf_mean`) are always CLEAR.
/// - `word_count >= 4 && token_conf_mean < 0.55` → unclear
/// - `token_conf_mean < 0.30` → unclear
/// - `snr_db < 5.0 && noise_floor_converged && token_conf_mean < 0.70` → unclear
fn is_unclear_utterance(text: &str, voice_analysis: Option<&serde_json::Value>) -> bool {
    let Some(token_conf_mean) = stt_token_conf_mean(voice_analysis) else {
        return false;
    };
    let word_count = text.split_whitespace().count();
    if word_count >= 4 && token_conf_mean < 0.55 {
        return true;
    }
    if token_conf_mean < 0.30 {
        return true;
    }
    if let (Some(snr_db), Some(true)) = (
        audio_snr_db(voice_analysis),
        audio_noise_floor_converged(voice_analysis),
    ) && snr_db < 5.0
        && token_conf_mean < 0.70
    {
        return true;
    }
    false
}

/// The routing half: consumes one recorded user utterance and drives the
/// router → submitter/executor. Held in a daemon `OnceLock` — its presence IS
/// the `[kernel.agent].voice_loop` gate (set at boot only when the flag and
/// its `talk_loop` prerequisite hold).
pub struct VoiceLoop<H: AgentLoopHandle> {
    agent: Weak<AgentService<H>>,
    router: InterruptRouter,
    /// Shared with the daemon's [`DaemonReplySubmitter`]: the Queue arm
    /// pushes onto `shared.queues` (drained on the submitter's
    /// commit/failure) and the busy axis reads `shared.in_flight`.
    shared: Arc<VoiceShared>,
}

impl<H: AgentLoopHandle> VoiceLoop<H> {
    /// Back-compat constructor: private, unshared state — the Queue arm
    /// records but never drains and the busy axis reads empty. Daemon wiring
    /// must use [`Self::with_shared`], sharing the same `Arc<VoiceShared>`
    /// passed to [`DaemonReplySubmitter::with_shared`].
    pub fn new(agent: Weak<AgentService<H>>) -> Self {
        Self::with_shared(agent, Arc::new(VoiceShared::default()))
    }

    /// Build a loop sharing state with the [`DaemonReplySubmitter`] — the
    /// wiring the Queue-arm drain and the busy axis depend on.
    pub fn with_shared(agent: Weak<AgentService<H>>, shared: Arc<VoiceShared>) -> Self {
        Self {
            agent,
            router: InterruptRouter::new(),
            shared,
        }
    }

    /// The conversation's in-flight reply attempt seq, if any — the voice
    /// loop's busy axis (see [`InFlightAttempts`]). The daemon's
    /// `agent.turn.record` arm reads this BEFORE recording the utterance.
    pub fn in_flight(&self, conv_id: &str) -> Option<u64> {
        self.shared.in_flight.get(conv_id).map(|e| *e.value())
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

        // Non-lexical gate (WEFT-659), BEFORE routing: idle → skip dispatch
        // (live feedback: "Mm." tagged `filler` got a full reply while
        // idle). Busy → fall through: the router's widened `is_backchannel`
        // (same class set) routes to `Backchannel`.
        if let Some(class) = nonlexical_paralinguistic_class(voice_analysis)
            && in_flight_before.is_none()
        {
            debug!(
                conv_id,
                class, "voice loop: non-lexical utterance while idle — no dispatch"
            );
            return;
        }

        // Unclear gate (WEFT-659), after the non-lexical check: skip on a
        // low-confidence decomposition — no synthetic reply, the talk-mode
        // client speaks its own clarification.
        if is_unclear_utterance(text, voice_analysis) {
            warn!(
                conv_id,
                word_count = text.split_whitespace().count(),
                token_conf_mean = ?stt_token_conf_mean(voice_analysis),
                snr_db = ?audio_snr_db(voice_analysis),
                noise_floor_converged = ?audio_noise_floor_converged(voice_analysis),
                "voice loop: utterance below clarity threshold — skipping dispatch"
            );
            return;
        }

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
                    warn!(
                        conv_id,
                        "voice loop: no reply submitter wired; turn recorded only"
                    );
                }
            }
            action @ InterruptAction::Backchannel => {
                // Never a turn: the executor emits the `Backchannel` impulse →
                // the loop's next tick draws a `Continuer` cross-ref onto the
                // in-flight turn (ADR-062; no-op when the turn commits first —
                // a benign race). The agent keeps working.
                agent
                    .execute_interrupt(
                        action,
                        InterruptCtx {
                            conv_id: conv_id.to_string(),
                            in_flight_seq: in_flight_before,
                        },
                    )
                    .await;
                debug!(
                    conv_id,
                    "voice loop: backchannel — Continuer emitted, agent keeps working"
                );
            }
            InterruptAction::Queue => {
                // Recorded and held behind the in-flight turn; drained by
                // `DaemonReplySubmitter`'s dispatch task once it commits or
                // fails (WEFT-650).
                enqueue_utterance(&self.shared.queues, conv_id, text);
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
                // Busy-axis maintenance: a Refine's resubmit already replaced
                // the tracker entry with the amendment's seq (dispatch_reply
                // inserts on register); a bare Stop prunes without a
                // replacement, so clear the pruned attempt — the conv reads
                // idle again. remove-if-equal keeps a just-inserted amendment
                // entry safe either way.
                if let (Some(pruned), true) = (outcome.pruned_seq, outcome.amendment_seq.is_none())
                {
                    self.shared.clear_in_flight(conv_id, pruned);
                }
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

#[cfg(test)]
#[path = "voice_loop_tests.rs"]
mod tests;
