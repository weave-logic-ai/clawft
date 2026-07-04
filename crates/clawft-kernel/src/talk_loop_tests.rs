//! Tests for the Talk-Mode loop (ADR-062 Phase 2.1; M2 multiplex). Deterministic,
//! scripted impulses only — no audio, no LLM, no embedder.

use super::*;
use crate::causal::CausalGraph;
use crate::cognitive_tick::{CognitiveTick, CognitiveTickConfig};
use crate::coherence::CoherenceSignals;
use crate::context_graft::{ChunkMeta, NodeState, SessionView, content_hash};
use crate::crossref::{CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};
use crate::impulse::{ImpulseQueue, ImpulseType};
use crate::view_resolver::ViewResolver;
use serde_json::json;

const DIMS: usize = 8;

/// A test resolver backed by a map of conversation views. Views can be added
/// after construction (multiplex) or removed (conversation-reaped no-op).
#[derive(Default)]
struct MapResolver {
    views: DashMap<String, Arc<SessionView>>,
}

impl MapResolver {
    fn with(conv_id: &str, view: Arc<SessionView>) -> Arc<Self> {
        let r = Arc::new(Self::default());
        r.views.insert(conv_id.to_string(), view);
        r
    }
    fn insert(&self, conv_id: &str, view: Arc<SessionView>) {
        self.views.insert(conv_id.to_string(), view);
    }
}

impl ViewResolver for MapResolver {
    fn view_for(&self, conv_id: &str) -> Option<Arc<SessionView>> {
        self.views.get(conv_id).map(|v| v.clone())
    }
}

fn new_loop(views: Arc<dyn ViewResolver>) -> (Arc<ImpulseQueue>, Arc<CausalGraph>, Arc<CrossRefStore>, TalkModeLoop) {
    let iq = Arc::new(ImpulseQueue::new());
    let causal = Arc::new(CausalGraph::new());
    let crs = Arc::new(CrossRefStore::new());
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let l = TalkModeLoop::new(
        iq.clone(),
        causal.clone(),
        crs.clone(),
        views,
        tick,
        TalkModeConfig::default(),
    );
    (iq, causal, crs, l)
}

/// Build a fully wired single-conversation ("conv-1") loop over a fresh forest.
fn make_loop() -> (
    Arc<ImpulseQueue>,
    Arc<CausalGraph>,
    Arc<CrossRefStore>,
    Arc<SessionView>,
    TalkModeLoop,
) {
    let view = Arc::new(SessionView::new("conv-1", DIMS));
    let resolver = MapResolver::with("conv-1", view.clone());
    let (iq, causal, crs, l) = new_loop(resolver);
    (iq, causal, crs, view, l)
}

/// Seed a frontier turn node on both substrates and register it with the loop
/// under `conv_id` (mimicking `session_forest::dual_write_turn` + `register_turn`).
/// Returns the causal node id and the turn's universal id.
fn seed_turn_in(
    view: &SessionView,
    causal: &CausalGraph,
    l: &TalkModeLoop,
    conv_id: &str,
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
        conv_id.as_bytes(),
        seq,
        text.as_bytes(),
        b"turn",
    );
    let node = causal.add_node(
        format!("turn:{conv_id}:{seq}"),
        json!({ "chain_seq": seq, "state": "frontier" }),
    );
    l.register_turn(seq, node, uid.clone(), conv_id);
    (node, uid)
}

/// `conv-1` convenience over [`seed_turn_in`].
fn seed_turn(
    view: &SessionView,
    causal: &CausalGraph,
    l: &TalkModeLoop,
    seq: u64,
    text: &str,
) -> (NodeId, UniversalNodeId) {
    seed_turn_in(view, causal, l, "conv-1", seq, text)
}

/// THE full SENSE→FLOOR→MUTATE path: a backchannel never becomes a node, and
/// an EndOfUtterance commits the in-flight turn (ADR-062 D5 done-when).
#[test]
fn backchannel_never_a_node_and_eou_commits() {
    let (iq, causal, crs, view, l) = make_loop();

    let (n100, _u100) = seed_turn(&view, &causal, &l, 100, "tell me about Puyo please");
    assert_eq!(l.current_turn("conv-1"), Some(100));
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
    assert_eq!(l.current_turn("conv-1"), None);
}

/// A TurnClaim (barge-in) prunes the in-flight node and draws a Contradicts
/// edge from the claiming turn (ADR-062 D5 done-when).
#[test]
fn turn_claim_prunes_and_contradicts() {
    let (iq, causal, _crs, view, l) = make_loop();

    // Register the claiming turn first, then the in-flight turn, so current=200.
    let (n300, _u300) = seed_turn(&view, &causal, &l, 300, "actually wait a moment");
    let (n200, _u200) = seed_turn(&view, &causal, &l, 200, "turn two here right now");
    assert_eq!(l.current_turn("conv-1"), Some(200));

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
    assert_eq!(l.current_turn("conv-1"), Some(300));
}

/// A TurnShift opens the floor (drops the in-flight turn) without committing.
#[test]
fn turn_shift_hands_off_the_floor() {
    let (iq, causal, _crs, view, l) = make_loop();
    seed_turn(&view, &causal, &l, 100, "a turn that will hand off");
    assert_eq!(l.current_turn("conv-1"), Some(100));

    iq.emit(0, [0u8; 32], 0, ImpulseType::TurnShift, json!({}), 50);
    let r = l.tick();
    assert_eq!(r.handoffs, 1);
    // Floor opened: no in-flight turn, and the turn was NOT committed.
    assert_eq!(l.current_turn("conv-1"), None);
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

// ── M2 multiplex tests (plan §2.1) ───────────────────────────────────────────

/// One loop multiplexes two conversations: each EOU commits its own seq on its
/// own view, and neither touches the other (M2 D1/D4).
#[test]
fn multiplex_commits_two_convs_independently() {
    let view_a = Arc::new(SessionView::new("conv-a", DIMS));
    let view_b = Arc::new(SessionView::new("conv-b", DIMS));
    let resolver = MapResolver::with("conv-a", view_a.clone());
    resolver.insert("conv-b", view_b.clone());
    let (iq, causal, _crs, l) = new_loop(resolver);

    let (na, _) = seed_turn_in(&view_a, &causal, &l, "conv-a", 1, "hello from a");
    let (nb, _) = seed_turn_in(&view_b, &causal, &l, "conv-b", 2, "hello from b");

    // EOU for each conversation, carrying both chain_seq and conv_id.
    iq.emit(0, [0u8; 32], 0, ImpulseType::EndOfUtterance, json!({ "chain_seq": 1, "conv_id": "conv-a" }), 10);
    iq.emit(0, [0u8; 32], 0, ImpulseType::EndOfUtterance, json!({ "chain_seq": 2, "conv_id": "conv-b" }), 11);

    let r = l.tick();
    assert_eq!(r.commits, 2);

    // Each conversation committed its own seq on its own view — and only its own.
    assert_eq!(view_a.state(1), Some(NodeState::Committed));
    assert_eq!(view_a.state(2), None);
    assert_eq!(view_b.state(2), Some(NodeState::Committed));
    assert_eq!(view_b.state(1), None);
    assert_eq!(causal.get_node(na).unwrap().metadata["state"], "committed");
    assert_eq!(causal.get_node(nb).unwrap().metadata["state"], "committed");
    assert_eq!(l.current_turn("conv-a"), None);
    assert_eq!(l.current_turn("conv-b"), None);
}

/// An EOU with an explicit `chain_seq` commits that seq regardless of which turn
/// is currently in-flight for the conversation.
#[test]
fn eou_with_explicit_chain_seq_commits_named_seq() {
    let (iq, causal, _crs, view, l) = make_loop();
    let (n10, _) = seed_turn(&view, &causal, &l, 10, "first turn");
    seed_turn(&view, &causal, &l, 11, "second turn"); // current_turn now 11
    assert_eq!(l.current_turn("conv-1"), Some(11));

    // Commit the NAMED (older) seq 10, not the in-flight 11.
    iq.emit(0, [0u8; 32], 0, ImpulseType::EndOfUtterance, json!({ "chain_seq": 10 }), 5);
    let r = l.tick();
    assert_eq!(r.commits, 1);

    assert_eq!(view.state(10), Some(NodeState::Committed));
    assert_eq!(causal.get_node(n10).unwrap().metadata["state"], "committed");
    // The in-flight turn is untouched (11 was not the named seq).
    assert_eq!(view.state(11), Some(NodeState::Frontier));
    assert_eq!(l.current_turn("conv-1"), Some(11));
}

/// After `end_conversation`, the conversation's lineage/current_turn are evicted,
/// so a later EOU for an evicted seq is a no-op (M2 D6).
#[test]
fn end_conversation_evicts_lineage() {
    let (iq, causal, _crs, view, l) = make_loop();
    let (n100, _) = seed_turn(&view, &causal, &l, 100, "a turn to evict");
    assert_eq!(l.current_turn("conv-1"), Some(100));

    l.end_conversation("conv-1");
    assert_eq!(l.current_turn("conv-1"), None);

    // A late EOU for the evicted seq commits nothing — lineage is gone.
    iq.emit(0, [0u8; 32], 0, ImpulseType::EndOfUtterance, json!({ "chain_seq": 100 }), 5);
    let r = l.tick();
    assert_eq!(r.commits, 0);
    assert_eq!(view.state(100), Some(NodeState::Frontier));
    assert_eq!(causal.get_node(n100).unwrap().metadata["state"], "frontier");
}

/// The drain-overflow fix (M2 D8): a flood of `max + N` EOUs commits every seq
/// across successive ticks — none silently dropped.
#[test]
fn drain_overflow_not_dropped() {
    let view = Arc::new(SessionView::new("conv-1", DIMS));
    let resolver = MapResolver::with("conv-1", view.clone());
    let (iq, causal, _crs, l) = new_loop(resolver);

    let max = TalkModeConfig::default().max_impulses_per_tick;
    let total = max + 10;
    for seq in 0..total as u64 {
        seed_turn(&view, &causal, &l, seq, &format!("turn {seq}"));
        iq.emit(0, [0u8; 32], 0, ImpulseType::EndOfUtterance, json!({ "chain_seq": seq }), seq);
    }

    // Tick until the queue is fully drained (never more than `max` per tick).
    let mut ticks = 0;
    let mut committed = 0;
    while !iq.is_empty() {
        let r = l.tick();
        assert!(r.impulses_sensed <= max, "drained {} > max {}", r.impulses_sensed, max);
        committed += r.commits;
        ticks += 1;
        assert!(ticks < 10, "did not drain within a bounded number of ticks");
    }
    assert!(ticks >= 2, "a max+10 flood must span at least two ticks");
    assert_eq!(committed, total, "every EOU must commit — none dropped");

    // Every seq is Committed on the view.
    for seq in 0..total as u64 {
        assert_eq!(view.state(seq), Some(NodeState::Committed), "seq {seq} not committed");
    }
}
