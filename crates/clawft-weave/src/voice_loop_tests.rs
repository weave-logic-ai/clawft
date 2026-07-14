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
fn clarity_rule_low_snr_converged_and_mid_conf_is_unclear() {
    assert!(is_unclear_utterance("ok", Some(&va(0.65, 4.0, true))));
    // Noise floor not converged yet — the SNR clause doesn't fire.
    assert!(!is_unclear_utterance("ok", Some(&va(0.65, 4.0, false))));
    // SNR at/above the 5.0 floor doesn't fire.
    assert!(!is_unclear_utterance("ok", Some(&va(0.65, 5.0, true))));
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
