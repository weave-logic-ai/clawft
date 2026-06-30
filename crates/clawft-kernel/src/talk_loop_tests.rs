//! Tests for the Talk-Mode loop (ADR-062 Phase 2.1). Deterministic, scripted
//! impulses only — no audio, no LLM, no embedder.

use super::*;
use crate::causal::CausalGraph;
use crate::cognitive_tick::{CognitiveTick, CognitiveTickConfig};
use crate::coherence::CoherenceSignals;
use crate::context_graft::{ChunkMeta, NodeState, SessionView, content_hash};
use crate::crossref::{CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};
use crate::impulse::{ImpulseQueue, ImpulseType};
use serde_json::json;

const DIMS: usize = 8;

/// Build a fully wired loop over a fresh forest substrate.
fn make_loop() -> (
    Arc<ImpulseQueue>,
    Arc<CausalGraph>,
    Arc<CrossRefStore>,
    Arc<SessionView>,
    TalkModeLoop,
) {
    let iq = Arc::new(ImpulseQueue::new());
    let causal = Arc::new(CausalGraph::new());
    let crs = Arc::new(CrossRefStore::new());
    let view = Arc::new(SessionView::new("conv-1", DIMS));
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let l = TalkModeLoop::new(
        iq.clone(),
        causal.clone(),
        crs.clone(),
        view.clone(),
        tick,
        TalkModeConfig::default(),
    );
    (iq, causal, crs, view, l)
}

/// Seed a frontier turn node on both substrates and register it with the loop
/// (mimicking `session_forest::dual_write_turn` + `register_turn`). Returns the
/// causal node id and the turn's universal id.
fn seed_turn(
    view: &SessionView,
    causal: &CausalGraph,
    l: &TalkModeLoop,
    seq: u64,
    text: &str,
) -> (NodeId, UniversalNodeId) {
    let meta = ChunkMeta {
        chain_seq: seq,
        content_hash: content_hash(text),
        state: NodeState::Frontier,
        kind: "agent.chat.turn".into(),
        blob_hash: None,
        inline: Some(text.to_string()),
    };
    assert!(view.insert_vector(vec![0.1; DIMS], meta));
    let uid = UniversalNodeId::new(
        &StructureTag::CausalGraph,
        b"conv-1",
        seq,
        text.as_bytes(),
        b"turn",
    );
    let node = causal.add_node(
        format!("turn:conv-1:{seq}"),
        json!({ "chain_seq": seq, "state": "frontier" }),
    );
    l.register_turn(seq, node, uid.clone());
    (node, uid)
}

/// THE full SENSE→FLOOR→MUTATE path: a backchannel never becomes a node, and
/// an EndOfUtterance commits the in-flight turn (ADR-062 D5 done-when).
#[test]
fn backchannel_never_a_node_and_eou_commits() {
    let (iq, causal, crs, view, l) = make_loop();

    let (n100, _u100) = seed_turn(&view, &causal, &l, 100, "tell me about Puyo please");
    assert_eq!(l.current_turn(), Some(100));
    let nodes_before = causal.node_count();

    // A backchannel mid-utterance (mm-hmm), then end-of-utterance — emitted
    // out of nothing, drained in HLC order.
    let listener = [7u8; 32];
    iq.emit(
        StructureTag::CausalGraph.as_u8(),
        listener,
        StructureTag::CausalGraph.as_u8(),
        ImpulseType::Backchannel,
        json!({}),
        110,
    );
    iq.emit(
        0,
        [0u8; 32],
        0,
        ImpulseType::EndOfUtterance,
        json!({ "chain_seq": 100 }),
        120,
    );

    let r = l.tick();
    assert_eq!(r.impulses_sensed, 2);
    assert_eq!(r.backchannels, 1);
    assert_eq!(r.commits, 1);

    // INVARIANT: the backchannel added NO node.
    assert_eq!(causal.node_count(), nodes_before);

    // The backchannel landed as a Continuer cross-ref listener → turn-100.
    let listener_uid = UniversalNodeId::from_bytes(listener);
    let conts = crs.by_type(&listener_uid, &CrossRefType::Continuer);
    assert_eq!(conts.len(), 1);
    assert_eq!(conts[0].ref_type, CrossRefType::Continuer);

    // EOU committed turn 100 on BOTH substrates, and cleared the in-flight turn.
    assert_eq!(view.state(100), Some(NodeState::Committed));
    assert_eq!(
        causal.get_node(n100).unwrap().metadata["state"],
        "committed"
    );
    assert_eq!(l.current_turn(), None);
}

/// A TurnClaim (barge-in) prunes the in-flight node and draws a Contradicts
/// edge from the claiming turn (ADR-062 D5 done-when).
#[test]
fn turn_claim_prunes_and_contradicts() {
    let (iq, causal, _crs, view, l) = make_loop();

    // Register the claiming turn first, then the in-flight turn, so current=200.
    let (n300, _u300) = seed_turn(&view, &causal, &l, 300, "actually wait a moment");
    let (n200, _u200) = seed_turn(&view, &causal, &l, 200, "turn two here right now");
    assert_eq!(l.current_turn(), Some(200));

    iq.emit(
        0,
        [0u8; 32],
        0,
        ImpulseType::TurnClaim,
        json!({ "claim_seq": 300 }),
        210,
    );

    let r = l.tick();
    assert_eq!(r.prunes, 1);

    // The in-flight node is Pruned (hard rebase) on both substrates.
    assert_eq!(view.state(200), Some(NodeState::Pruned));
    assert_eq!(causal.get_node(n200).unwrap().metadata["state"], "pruned");

    // A Contradicts edge runs from the claiming turn (300) to the pruned (200).
    let contradicts = causal.get_edges_by_type(n300, &CausalEdgeType::Contradicts);
    assert_eq!(contradicts.len(), 1);
    assert_eq!(contradicts[0].target, n200);

    // The claiming turn is now in-flight.
    assert_eq!(l.current_turn(), Some(300));
}

/// A TurnShift opens the floor (drops the in-flight turn) without committing.
#[test]
fn turn_shift_hands_off_the_floor() {
    let (iq, causal, _crs, view, l) = make_loop();
    seed_turn(&view, &causal, &l, 100, "a turn that will hand off");
    assert_eq!(l.current_turn(), Some(100));

    iq.emit(0, [0u8; 32], 0, ImpulseType::TurnShift, json!({}), 50);
    let r = l.tick();
    assert_eq!(r.handoffs, 1);
    // Floor opened: no in-flight turn, and the turn was NOT committed.
    assert_eq!(l.current_turn(), None);
    assert_eq!(view.state(100), Some(NodeState::Frontier));
}

/// The coherence dampener softens the floor score below threshold (ADR-062 D6
/// — a dampener the loop applies, not a hard gate).
#[test]
fn coherence_dampener_softens_floor_score() {
    let (_iq, causal, _crs, view, l) = make_loop();
    seed_turn(&view, &causal, &l, 100, "an in-flight ready turn");

    // Drive coherence below threshold (all-zero signals → score 0 < 0.65).
    let broken = CoherenceSignals {
        relation_coverage: 0.0,
        relation_confidence: 0.0,
        structural_connectivity: 0.0,
        qap_closure: 0.0,
        topic_continuity: 0.0,
    };
    assert!(l.observe_coherence(&broken));

    let r = l.tick();
    assert!(r.floor_score > 0.0, "frontier candidate should score > 0");
    assert!(
        r.dampened_score < r.floor_score,
        "dampened {} should be below raw {}",
        r.dampened_score,
        r.floor_score
    );
    assert!((r.dampened_score - r.floor_score * 0.8).abs() < 1e-6);
}

/// The loop registers as a SystemService and shares the ADR-047 tick (start
/// flips the shared `CognitiveTick` running flag; health reflects it).
#[tokio::test]
async fn service_lifecycle_and_health() {
    let (_iq, _causal, _crs, _view, l) = make_loop();
    assert_eq!(l.name(), "voice.talk_mode");
    assert_eq!(l.service_type(), ServiceType::Core);
    assert_eq!(
        l.health_check().await,
        HealthStatus::Degraded("talk-mode tick not running".into())
    );
    l.start().await.unwrap();
    assert_eq!(l.health_check().await, HealthStatus::Healthy);
    l.stop().await.unwrap();
    assert!(matches!(l.health_check().await, HealthStatus::Degraded(_)));
}

/// An empty tick still runs the full path and records a tick.
#[test]
fn empty_tick_records_and_reads_open_floor() {
    let (_iq, _causal, _crs, _view, l) = make_loop();
    let r = l.tick();
    assert_eq!(r.impulses_sensed, 0);
    assert_eq!(r.floor, FloorState::Open);
    assert_eq!(l.total_ticks(), 1);
}
