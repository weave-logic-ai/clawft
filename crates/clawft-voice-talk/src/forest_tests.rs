//! Tests for the forest bridge (ADR-062 Phase 6.1): the P5 impulse sink and the
//! [`LoopObserver`] that makes the P2 loop the single turn-taking orchestrator.

use super::*;
use clawft_channels::voice::capture::{ImpulseSink, VoiceImpulse};
use clawft_channels::voice::talkmode::{ConversationEvent, ConversationObserver};
use clawft_kernel::CausalEdgeType;
use clawft_kernel::ImpulseType;
use clawft_kernel::context_graft::NodeState;

fn forest() -> Arc<TalkForest> {
    Arc::new(TalkForest::new("conv-test", 8))
}

#[test]
fn kernel_impulse_sink_maps_capture_signals_to_kernel_codes() {
    let f = forest();
    let sink = KernelImpulseSink::new(f.impulses().clone(), [9u8; 32]);

    // Onset → TurnClaim (floor claim); endpoint → EndOfUtterance.
    sink.emit(VoiceImpulse::SpeechStart, 10);
    sink.emit(VoiceImpulse::EndOfUtterance, 20);

    let drained = f.impulses().drain_ready();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].impulse_type, ImpulseType::TurnClaim);
    assert_eq!(drained[1].impulse_type, ImpulseType::EndOfUtterance);
    // HLC order preserved (the queue sorts ascending).
    assert!(drained[0].hlc_timestamp < drained[1].hlc_timestamp);
}

#[test]
fn user_turn_dual_writes_and_loop_commits_it() {
    let f = forest();
    let obs = LoopObserver::new(f.clone());

    obs.observe(ConversationEvent::UserTurn {
        text: "tell me about Puyo please".into(),
        speaker: None,
        speaker_name: None,
        voice_analysis: None,
    });

    // The turn is dual-written as a Frontier on both substrates and registered
    // with the loop (the observer does NOT commit — the loop does).
    assert_eq!(f.view().state(1), Some(NodeState::Frontier));
    assert_eq!(f.talk_loop().current_turn("conv-test"), Some(1));

    // The loop's next tick drains the observer's EOU impulse and commits.
    let r = f.talk_loop().tick();
    assert_eq!(r.commits, 1);
    assert_eq!(f.view().state(1), Some(NodeState::Committed));
    assert_eq!(f.talk_loop().current_turn("conv-test"), None);
}

#[test]
fn full_turn_renders_speculative_then_committed_and_barge_in_contradicts() {
    let f = forest();
    let obs = LoopObserver::new(f.clone());

    obs.observe(ConversationEvent::UserTurn {
        text: "what's the capital of France".into(),
        speaker: None,
        speaker_name: None,
        voice_analysis: None,
    });
    f.talk_loop().tick(); // loop commits the user turn
    let after_turn = f.graph().node_count();

    obs.observe(ConversationEvent::SpeculativeReply {
        ack: "one sec.".into(),
    });
    obs.observe(ConversationEvent::CommittedReply {
        answer: "Paris.".into(),
    });

    // Two reply nodes landed; the committed answer Enables the speculative ack.
    // Node ids are 1-based monotonic, so id == (node_count just before add).
    assert_eq!(f.graph().node_count(), after_turn + 2);
    let committed = after_turn + 2; // spec = after_turn+1, committed = after_turn+2
    let enables = f
        .graph()
        .get_edges_by_type(committed, &CausalEdgeType::Enables);
    assert_eq!(enables.len(), 1, "committed answer supersedes the ack");

    // Barge-in: an interrupt node Contradicts the in-flight reply AND a
    // TurnClaim impulse is emitted for the loop (the floor claim).
    obs.observe(ConversationEvent::Interrupted);
    let interrupt = after_turn + 3;
    let contradicts = f
        .graph()
        .get_edges_by_type(interrupt, &CausalEdgeType::Contradicts);
    assert_eq!(contradicts.len(), 1);
    let drained = f.impulses().drain_ready();
    assert!(
        drained
            .iter()
            .any(|i| i.impulse_type == ImpulseType::TurnClaim),
        "barge-in claims the floor via a TurnClaim impulse"
    );
}

#[test]
fn partial_transcript_feeds_midstream_and_emits_coherence_alert_on_stall() {
    // WEFT-715: LoopObserver partials → token ring → CoherenceAlert on the
    // shared forest impulse queue (analysis only; no graph write).
    let f = forest();
    let obs = LoopObserver::new(f.clone());

    // Default TemporalMidstreamAnalyzer stall_run is 4 identical tokens.
    obs.observe(ConversationEvent::PartialTranscript {
        text: "x x x x".into(),
    });

    assert!(f.midstream().ring_len() >= 4);
    let drained = f.impulses().drain_ready();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].impulse_type, ImpulseType::CoherenceAlert);
    assert_eq!(drained[0].payload["source"], "midstream");
    assert_eq!(drained[0].payload["reason"], "repeat");
}

#[test]
fn midstream_tick_silent_when_ring_is_diverse() {
    let f = forest();
    f.midstream().on_partial("hello world from user", 1);
    assert!(f.on_midstream_tick(2).is_none());
    assert!(f.impulses().drain_ready().is_empty());
}

#[test]
fn midstream_iu_diff_available_on_forest() {
    // WEFT-714: IU prefix-diff via temporal analyzer (last-token refine → Extend).
    let f = forest();
    f.midstream().set_stable_prefix("hello runing");
    assert_eq!(
        f.midstream().diff_against_stable("hello running"),
        crate::PartialDiffVerdict::Extend
    );
}
