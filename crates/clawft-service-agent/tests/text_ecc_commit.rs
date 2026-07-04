//! Integration: text turns commit Frontier→Committed on the shared forest (M2).
//!
//! The text analogue of `clawft-voice-talk`'s `live_native_talk_session`, but
//! device-free and deterministic — a Mock embedder, an in-memory `ChainManager`,
//! and a directly-`tick()`ed `TalkModeLoop`. No daemon socket, no Hermes, no
//! audio. It proves the M2 thesis end-to-end at the wiring level:
//!
//!   text turn → `SessionTier::index_turn` dual-write + `EndOfUtterance` impulse
//!             → `TalkModeLoop::tick()` → Frontier→Committed on the SAME
//!               `SessionView` the tier grafts from AND on the causal node,
//!               with a `Follows` lineage edge user→assistant.
//!
//! The wiring mirrors the daemon (P4, D4): the loop's [`ViewResolver`] *is* the
//! tier (via the `Weak` cycle-breaker), so the loop commits onto the tier's own
//! per-conversation view rather than a second lifecycle copy. That is the whole
//! point of M2 — commit and graft ride one structure.

use std::sync::Arc;

use clawft_core::agent::graft::ContextGraftProvider;
use clawft_kernel::chain::ChainManager;
use clawft_kernel::context_graft::NodeState;
use clawft_kernel::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use clawft_kernel::{
    CausalEdgeType, CausalGraph, CognitiveTick, CognitiveTickConfig, CrossRefStore, TalkModeConfig,
    TalkModeLoop, ViewResolver,
};
use clawft_service_agent::SessionTier;

const DIMS: usize = 64;

/// Build a forest-joined `SessionTier` wired to a daemon-shaped `TalkModeLoop`.
///
/// The construction mirrors `clawft-weave`'s daemon boot (P4): the tier is
/// `Arc`-wrapped first, the loop is built with a `Weak` resolver pointing back
/// at the tier, then the loop is attached via `set_talk_loop`. This is the only
/// arrangement that makes the loop commit the *same* view the tier grafts from
/// (M2 D4) — a `SingleViewResolver` over a hand-made view would commit a
/// different object than `index_turn` populates and defeat the test.
fn wire_tier() -> (
    Arc<SessionTier>,
    Arc<ChainManager>,
    Arc<CausalGraph>,
    Arc<TalkModeLoop>,
) {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(DIMS));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());

    let tier = Arc::new(
        SessionTier::new(embedder, chain.clone(), None).with_forest(causal.clone(), crossrefs.clone()),
    );

    // The loop resolves views through the tier itself (D4). `Weak` back-ref so
    // there is no strong tier↔loop cycle — exactly the daemon's wiring.
    let resolver = SessionTier::weak_view_resolver(&tier);
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let talk_loop = Arc::new(TalkModeLoop::new(
        Arc::new(clawft_kernel::ImpulseQueue::new()),
        causal.clone(),
        crossrefs,
        resolver,
        tick,
        TalkModeConfig::default(),
    ));
    tier.set_talk_loop(talk_loop.clone());

    (tier, chain, causal, talk_loop)
}

/// Append a turn to the witness chain (as the substrate sink does) and index it
/// into the tier — the exact path `KernelTurnAnchor` drives in the daemon.
/// Returns the minted global `chain_seq`.
async fn append_and_index(
    tier: &SessionTier,
    chain: &ChainManager,
    conv: &str,
    role: &str,
    text: &str,
) -> u64 {
    let ev = chain.append(
        "agent",
        "agent.chat.turn",
        Some(serde_json::json!({ "conv": conv, "content": text })),
    );
    tier.index_turn(conv, ev.sequence, "agent.chat.turn", role, text)
        .await;
    ev.sequence
}

/// Find the causal node id the tier's dual-write produced for `chain_seq` by
/// scanning the graph's public API (the node id is not returned to callers).
/// The dual-write stamps `metadata.chain_seq`, so match on it.
fn node_for_seq(causal: &CausalGraph, chain_seq: u64) -> clawft_kernel::causal::NodeId {
    causal
        .node_ids()
        .into_iter()
        .find(|n| {
            causal
                .get_node(*n)
                .and_then(|node| node.metadata.get("chain_seq").and_then(|v| v.as_u64()))
                == Some(chain_seq)
        })
        .unwrap_or_else(|| panic!("no causal node for chain_seq {chain_seq}"))
}

/// The causal node's `state` attribute for `chain_seq` ("frontier"/"committed").
fn causal_state(causal: &CausalGraph, chain_seq: u64) -> String {
    let node = node_for_seq(causal, chain_seq);
    causal
        .get_node(node)
        .unwrap()
        .metadata
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>")
        .to_string()
}

/// HEADLINE (plan §2.3): a text turn commits Frontier→Committed on the shared
/// forest, the assistant reply commits Follows-linked to it, and graft still
/// returns both turns after commit.
#[tokio::test]
async fn text_turn_commits_frontier_to_committed_on_shared_forest() {
    let (tier, chain, causal, talk_loop) = wire_tier();
    let conv = "text-ecc";

    // ── Turn 1 (user) — indexed as Frontier, EOU impulse queued ───────────
    let s1 = append_and_index(&tier, &chain, conv, "user", "hello ecc").await;

    // The tier created the conversation's view; the loop resolves that SAME
    // view (D4 — one object, not a second lifecycle copy).
    let view = tier
        .view_for(conv)
        .expect("index_turn created the conversation view");

    // Before the tick: the view chunk and the causal node are both Frontier,
    // and exactly one EndOfUtterance is queued on the loop's shared queue.
    assert_eq!(
        view.state(s1),
        Some(NodeState::Frontier),
        "user turn enters the view as Frontier"
    );
    assert_eq!(
        causal_state(&causal, s1),
        "frontier",
        "user turn's causal node enters Frontier"
    );
    assert_eq!(
        talk_loop.impulses().len(),
        1,
        "index_turn emitted exactly one EndOfUtterance impulse"
    );

    // ── Tick — the sole consumer drains the EOU and commits turn 1 ─────────
    let r1 = talk_loop.tick();
    assert_eq!(r1.impulses_sensed, 1, "the one queued EOU is sensed");
    assert_eq!(r1.commits, 1, "the EOU commits exactly one turn");

    // Frontier→Committed on BOTH substrates — the shared-forest commit.
    assert_eq!(
        view.state(s1),
        Some(NodeState::Committed),
        "the loop committed the user turn in the view"
    );
    assert_eq!(
        causal_state(&causal, s1),
        "committed",
        "the loop committed the user turn's causal node"
    );
    assert!(
        talk_loop.impulses().is_empty(),
        "the EOU was drained, not left pending"
    );

    // ── Turn 2 (assistant reply) — commits Follows-linked to turn 1 ────────
    let s2 = append_and_index(&tier, &chain, conv, "assistant", "hi there, ecc online").await;
    assert!(s2 > s1, "chain_seq is globally monotone");
    assert_eq!(
        view.state(s2),
        Some(NodeState::Frontier),
        "reply enters Frontier before its tick"
    );

    let r2 = talk_loop.tick();
    assert_eq!(r2.commits, 1, "the reply's EOU commits it");
    assert_eq!(
        view.state(s2),
        Some(NodeState::Committed),
        "the reply committed in the view (the text CommittedReply)"
    );
    assert_eq!(
        causal_state(&causal, s2),
        "committed",
        "the reply committed on the causal node"
    );

    // The reply is Follows-linked to the user turn: the lineage edge runs
    // user(s1) → assistant(s2), stamped with the reply's witness seq.
    let n1 = node_for_seq(&causal, s1);
    let n2 = node_for_seq(&causal, s2);
    let follows: Vec<_> = causal
        .get_forward_edges(n1)
        .into_iter()
        .filter(|e| e.edge_type == CausalEdgeType::Follows)
        .collect();
    assert_eq!(follows.len(), 1, "exactly one Follows edge out of the user turn");
    assert_eq!(follows[0].target, n2, "Follows edge points at the reply node");
    assert_eq!(
        follows[0].chain_seq, s2,
        "the Follows edge carries the reply's witness seq"
    );

    // ── Graft still returns both turns — commit did not evict the index ────
    // Both committed chunks remain in the index (Committed ≠ removed).
    assert!(
        view.chunk(s1).is_some() && view.chunk(s2).is_some(),
        "both committed turns stay in the session index"
    );
    // A graft over the committed conversation recalls the user turn (exact
    // cosine hit under the Mock embedder) and lineage-fuses the reply forward
    // over the Follows edge — both witness sequences are cited.
    let block = tier.graft_block(conv, "hello ecc").await;
    assert_eq!(block.len(), 1, "graft returns a single recalled-context block");
    let body = &block[0].content;
    assert!(
        body.contains(&format!("chain_seq {s1}")),
        "graft cites the committed user turn: {body}"
    );
    assert!(
        body.contains(&format!("chain_seq {s2}")),
        "graft lineage-fuses the committed reply: {body}"
    );
}
