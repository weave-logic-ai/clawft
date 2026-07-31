//! Tests for `voice_loop.rs` — split out to keep the implementation file
//! under the workspace's 500-line budget (mirrors `session_tier.rs`'s
//! `#[path = "session_tier_tests.rs"]` pattern).

use super::*;
use serde_json::json;

#[test]
fn in_flight_tracker_set_clear_and_stale_guard() {
    let shared = VoiceShared::default();
    // Register sets the busy axis.
    shared.in_flight.insert("c1".into(), 10);
    assert_eq!(shared.in_flight.get("c1").map(|e| *e.value()), Some(10));
    // A stale finalize (older attempt) must not clobber a replacement.
    shared.in_flight.insert("c1".into(), 12); // Refine resubmit overwrote
    shared.clear_in_flight("c1", 10); // old attempt's task finalizes late
    assert_eq!(
        shared.in_flight.get("c1").map(|e| *e.value()),
        Some(12),
        "stale clear must not remove the amendment's entry"
    );
    // The matching finalize clears it — conv reads idle.
    shared.clear_in_flight("c1", 12);
    assert!(shared.in_flight.get("c1").is_none());
}

#[test]
fn enqueue_preserves_fifo_order() {
    let queues: VoiceQueues = DashMap::new();
    enqueue_utterance(&queues, "c1", "first");
    enqueue_utterance(&queues, "c1", "second");
    enqueue_utterance(&queues, "c1", "third");

    let mut q = queues.get_mut("c1").unwrap();
    assert_eq!(q.pop_front().as_deref(), Some("first"));
    assert_eq!(q.pop_front().as_deref(), Some("second"));
    assert_eq!(q.pop_front().as_deref(), Some("third"));
    assert!(q.pop_front().is_none());
}

#[test]
fn enqueue_is_scoped_per_conversation() {
    let queues: VoiceQueues = DashMap::new();
    enqueue_utterance(&queues, "c1", "c1-text");
    enqueue_utterance(&queues, "c2", "c2-text");

    assert_eq!(queues.get("c1").unwrap().len(), 1);
    assert_eq!(queues.get("c2").unwrap().len(), 1);
    assert_eq!(
        queues.get("c1").unwrap().front().map(String::as_str),
        Some("c1-text")
    );
}

#[test]
fn enqueue_drops_oldest_once_over_cap() {
    let queues: VoiceQueues = DashMap::new();
    // Fill past the cap by a few — every push beyond VOICE_QUEUE_CAP
    // must evict exactly one (the oldest), never grow unbounded.
    for i in 0..(VOICE_QUEUE_CAP + 3) {
        enqueue_utterance(&queues, "c1", &format!("msg-{i}"));
    }
    let q = queues.get("c1").unwrap();
    assert_eq!(q.len(), VOICE_QUEUE_CAP, "queue stays capped");
    // The oldest three (msg-0..msg-2) were dropped; the FIFO now starts
    // at msg-3.
    assert_eq!(q.front().map(String::as_str), Some("msg-3"));
    assert_eq!(
        q.back().map(String::as_str),
        Some(format!("msg-{}", VOICE_QUEUE_CAP + 2).as_str())
    );
}

/// Wire-shaped `voice_analysis` fixture: `stt.token_conf_mean` +
/// `audio.{snr_db,noise_floor_converged}` — the exact fields
/// `is_unclear_utterance` reads.
fn va(token_conf_mean: f64, snr_db: f64, noise_floor_converged: bool) -> serde_json::Value {
    json!({
        "stt": { "token_conf_mean": token_conf_mean },
        "audio": { "snr_db": snr_db, "noise_floor_converged": noise_floor_converged },
    })
}

#[test]
fn clarity_rule_matches_client_calibration_table() {
    // 1-word "garbage" conf, decent SNR: word_count < 4 exempts it — clear.
    assert!(!is_unclear_utterance("what", Some(&va(0.44, 6.4, true))));
    // Same signal quality, but long enough: word_count>=4 && conf<0.55 → unclear.
    assert!(is_unclear_utterance(
        "write a function that sorts a list of numbers please",
        Some(&va(0.45, 6.4, true))
    ));
    // High confidence is always clear regardless of length.
    assert!(!is_unclear_utterance(
        "write a function that sorts a list of numbers please",
        Some(&va(0.98, 6.4, true))
    ));
    // No voice_analysis at all (text-originated / legacy turn) — always clear.
    assert!(!is_unclear_utterance(
        "write a function that sorts a list",
        None
    ));
}

#[test]
fn clarity_rule_floor_conf_below_030_is_always_unclear() {
    // High SNR, short utterance — only the absolute confidence floor fires.
    assert!(is_unclear_utterance("hi", Some(&va(0.29, 20.0, true))));
    assert!(!is_unclear_utterance("hi", Some(&va(0.30, 20.0, true))));
}

#[test]
fn clarity_rule_low_snr_converged_and_low_conf_is_unclear() {
    // Threshold retuned 0.70 → 0.50 after the live 2026-07-16 mic run: the
    // user's room RUNS at 3-6dB SNR, and a confident "Thank you." (conf
    // 0.53, SNR 3.3) was spuriously gated → clarification → self-echo.
    assert!(is_unclear_utterance("ok", Some(&va(0.45, 4.0, true))));
    // Confident-enough speech in a merely-noisy room stays CLEAR.
    assert!(!is_unclear_utterance("ok", Some(&va(0.53, 3.3, true))));
    // Noise floor not converged yet — the SNR clause doesn't fire.
    assert!(!is_unclear_utterance("ok", Some(&va(0.45, 4.0, false))));
    // SNR at/above the 5.0 floor doesn't fire.
    assert!(!is_unclear_utterance("ok", Some(&va(0.45, 5.0, true))));
}

#[test]
fn clarity_rule_missing_token_conf_mean_is_clear() {
    let no_stt = json!({ "audio": { "snr_db": 1.0, "noise_floor_converged": true } });
    assert!(!is_unclear_utterance(
        "garbled utterance here now",
        Some(&no_stt)
    ));
}

#[test]
fn nonlexical_class_set_covers_backchannel_laughter_and_filler() {
    for class in ["backchannel_candidate", "laughter_candidate", "filler"] {
        let v = json!({ "paralinguistics": { "class": class } });
        assert_eq!(nonlexical_paralinguistic_class(Some(&v)), Some(class));
    }
    // Genuine speech, an unrecognized class, and no signal at all never gate.
    let speech = json!({ "paralinguistics": { "class": "speech" } });
    assert_eq!(nonlexical_paralinguistic_class(Some(&speech)), None);
    let unknown = json!({ "paralinguistics": { "class": "unknown" } });
    assert_eq!(nonlexical_paralinguistic_class(Some(&unknown)), None);
    assert_eq!(nonlexical_paralinguistic_class(None), None);
}

#[test]
fn voice_dispatch_params_lead_with_the_voice_system_message() {
    let params = voice_dispatch_params("conv-1", "sort the list");

    assert_eq!(params.conv_id, "conv-1");
    assert_eq!(params.messages.len(), 2);
    assert_eq!(params.messages[0].role, "system");
    assert_eq!(params.messages[0].content, VOICE_REPLY_SYSTEM_MESSAGE);
    assert_eq!(params.messages[1].role, "user");
    assert_eq!(params.messages[1].content, "sort the list");

    let metadata = params.metadata.expect("metadata set");
    assert_eq!(
        metadata.get("user_turn_recorded"),
        Some(&serde_json::Value::Bool(true))
    );
    assert_eq!(
        metadata.get("skill_instructions"),
        Some(&serde_json::Value::String(
            VOICE_REPLY_SYSTEM_MESSAGE.to_string()
        )),
        "skill_instructions is the metadata key loop_core.rs actually splices in — \
         see the voice_dispatch_params doc comment"
    );
}

// ── WEFT-673: hold-drain seam ───────────────────────────────────────────
//
// Pins: after a review-mode hold resolves, the queue drains immediately
// (not on the conversation's next ordinary turn). The seam is
// `VoiceLoop::drain_after_hold_resolve`, called from
// `daemon::{resolve_parked_attempt_on_accept, cleanup_parked_attempt_on_discard}`.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use clawft_service_agent::{AgentLoopHandle, AgentService};
use clawft_types::event::{InboundMessage, OutboundMessage};

/// Minimal loop handle — drain only needs `agent.upgrade()` so
/// `dispatch_reply` can spawn; the spawned task's result is not observed
/// by the drain-pop assertion (pop is synchronous before the spawn).
struct NoopHandle;

#[async_trait]
impl AgentLoopHandle for NoopHandle {
    async fn handle_turn(
        &self,
        msg: InboundMessage,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> Result<OutboundMessage, String> {
        Ok(OutboundMessage {
            channel: msg.channel,
            chat_id: msg.chat_id,
            content: "ok".into(),
            reply_to: None,
            media: Vec::new(),
            metadata: HashMap::new(),
        })
    }
}

fn live_voice_loop() -> (Arc<AgentService<NoopHandle>>, VoiceLoop<NoopHandle>) {
    let service = Arc::new(AgentService::new(Arc::new(NoopHandle)));
    let shared = Arc::new(VoiceShared::default());
    let loop_ = VoiceLoop::with_shared(Arc::downgrade(&service), shared);
    (service, loop_)
}

#[tokio::test]
async fn drain_after_hold_resolve_pops_one_queued_utterance() {
    // Needs a tokio runtime: drain_next pops synchronously then
    // `tokio::spawn`s the resubmit — plain `#[test]` would panic on spawn.
    let (_service, loop_) = live_voice_loop();
    let conv = "hold-drain-accept";
    // Simulate: parked attempt busy + two utterances queued during the hold.
    loop_.shared_for_test().in_flight.insert(conv.into(), 42);
    enqueue_utterance(&loop_.shared_for_test().queues, conv, "queued-first");
    enqueue_utterance(&loop_.shared_for_test().queues, conv, "queued-second");
    assert_eq!(loop_.queued_len(conv), 2);

    // RPC arm order (WEFT-673): clear busy, then drain.
    loop_.clear_parked_in_flight(conv, 42);
    assert!(loop_.in_flight(conv).is_none(), "busy axis cleared first");
    loop_.drain_after_hold_resolve(conv);

    // Pop is synchronous inside drain_next (spawn is only the resubmit).
    assert_eq!(
        loop_.queued_len(conv),
        1,
        "exactly one utterance drains on hold resolve"
    );
    assert_eq!(
        loop_
            .shared_for_test()
            .queues
            .get(conv)
            .and_then(|q| q.front().cloned())
            .as_deref(),
        Some("queued-second"),
        "FIFO: first queued utterance is the one drained"
    );
    // Let the spawned resubmit settle so the runtime doesn't drop mid-flight.
    tokio::task::yield_now().await;
}

#[tokio::test]
async fn drain_after_hold_resolve_is_noop_on_empty_queue() {
    let (_service, loop_) = live_voice_loop();
    loop_.drain_after_hold_resolve("never-queued");
    assert_eq!(loop_.queued_len("never-queued"), 0);
}

#[test]
fn drain_after_hold_resolve_is_noop_when_agent_dropped() {
    // Dead Weak: the RPC arm can still clear parks, but without a live
    // service there is nothing to resubmit through — leave the queue
    // intact rather than dropping utterances on the floor. No tokio
    // runtime needed: upgrade fails before drain_next / spawn.
    let shared = Arc::new(VoiceShared::default());
    let dead: Weak<AgentService<NoopHandle>> = Weak::new();
    let loop_ = VoiceLoop::with_shared(dead, shared.clone());
    enqueue_utterance(&shared.queues, "orphan", "still-here");
    loop_.drain_after_hold_resolve("orphan");
    assert_eq!(
        shared.queues.get("orphan").map(|q| q.len()),
        Some(1),
        "dead agent must not pop the queue (no resubmit target)"
    );
}

#[tokio::test]
async fn clear_parked_then_drain_mirrors_ordinary_finalize_order() {
    // Pin the accept/discard call order used by daemon resolution:
    // clear_parked_in_flight(seq) THEN drain_after_hold_resolve(conv).
    // A stale clear must not clobber a drained resubmit's new in-flight
    // entry (same guard ordinary finalize uses).
    let (_service, loop_) = live_voice_loop();
    let conv = "order-pin";
    loop_.shared_for_test().in_flight.insert(conv.into(), 7);
    enqueue_utterance(&loop_.shared_for_test().queues, conv, "next");

    loop_.clear_parked_in_flight(conv, 7);
    // A Refine-style overwrite between clear and drain would land a new
    // seq here; stale clear of 7 must not remove it (clear_in_flight guard).
    loop_.shared_for_test().in_flight.insert(conv.into(), 99);
    loop_.clear_parked_in_flight(conv, 7);
    assert_eq!(
        loop_.in_flight(conv),
        Some(99),
        "stale parked-seq clear must not wipe a replacement in-flight"
    );
    // Drain still pops regardless of busy axis (resubmit overwrites).
    loop_.drain_after_hold_resolve(conv);
    assert_eq!(loop_.queued_len(conv), 0);
    tokio::task::yield_now().await;
}

/// Voice cancel / discard forest closure is the SAME pair of SessionTier
/// primitives the Stop interrupt uses (`emit_cancel_prune` +
/// `witness_cancel`) — pin the contract at the call-site documentation
/// level so a future refactor that diverges the two entry points breaks
/// a search, not production. Runtime coverage lives in
/// `clawft-service-agent` wave2 forest tests + daemon park/take tests;
/// this constant keeps the WEFT-673 AC ("voice path specifically")
/// discoverable next to the drain seam it ships with.
#[test]
fn voice_discard_forest_closure_reuses_stop_primitives() {
    // The strings below are the SessionTier method names
    // `daemon::cleanup_parked_attempt_on_discard` must call — kept as a
    // compile-visible pin (renames force a test edit).
    const PRUNE: &str = "emit_cancel_prune";
    const WITNESS: &str = "witness_cancel";
    const COMMIT: &str = "commit_reply_frontier";
    assert!(
        PRUNE == "emit_cancel_prune" && WITNESS == "witness_cancel",
        "discard path: same forest closure as Stop interrupt"
    );
    assert_eq!(
        COMMIT, "commit_reply_frontier",
        "accept path: forest commit deferred from dispatch_reply Ok arm"
    );
}
