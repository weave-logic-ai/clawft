//! Voice Wave 2 §W2.1 tests: the register-early/commit-late reply path and
//! the interrupt-signal projection. Sibling file to `session_tier_tests.rs`
//! (the <500-line rule); same deterministic fixtures — Mock embedder,
//! in-memory chain, hand-held impulse queue.

use super::*;
use clawft_kernel::chain::ChainManager;
use clawft_kernel::embedding::MockEmbeddingProvider;
use clawft_kernel::{
    CausalGraph, CognitiveTick, CognitiveTickConfig, CrossRefStore, CrossRefType, ImpulseQueue,
    TalkModeConfig, TalkModeLoop,
};
use serde_json::json;

use crate::dialogue_act::Intent;
use crate::turn_classifier::KeywordTurnClassifier;

/// Forest-joined tier + hosted loop over a queue we hold, so register/commit
/// emissions are observable. Daemon shape (M2 D4): the loop resolves the
/// TIER'S OWN view through the Weak resolver — a detached view would let a
/// silently no-oping commit/prune pass (the exact blind spot that hid the
/// register-without-view-chunk bug).
type Wired = (
    Arc<SessionTier>,
    Arc<ChainManager>,
    Arc<TalkModeLoop>,
    Arc<ImpulseQueue>,
    Arc<CrossRefStore>,
);

fn wired(_conv: &str) -> Wired {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());
    let impulses = Arc::new(ImpulseQueue::new());
    let tier = Arc::new(
        SessionTier::new(embedder, chain.clone(), None)
            .with_forest(causal.clone(), crossrefs.clone()),
    );
    let resolver = SessionTier::weak_view_resolver(&tier);
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let talk_loop = Arc::new(TalkModeLoop::new(
        impulses.clone(),
        causal,
        crossrefs.clone(),
        resolver,
        tick,
        TalkModeConfig::default(),
    ));
    tier.set_talk_loop(talk_loop.clone());
    (tier, chain, talk_loop, impulses, crossrefs)
}

#[tokio::test]
async fn register_reply_frontier_is_busy_without_eou_and_stashes_goal() {
    let conv = "w21-c1";
    let (tier, _chain, talk_loop, impulses, _crossrefs) = wired(conv);

    let seq = tier
        .register_reply_frontier(conv, "sort the list")
        .await
        .expect("forest + loop attached");

    // The attempt is the conversation's in-flight turn — the durable busy
    // state the router keys on.
    assert_eq!(talk_loop.current_turn(conv), Some(seq));
    // Register-early does NOT commit: no EndOfUtterance until finalize.
    assert!(
        impulses.drain_ready().is_empty(),
        "register must not emit the commit EOU"
    );
    // The original goal is on the node for the Refine reconstruction.
    assert_eq!(tier.goal_for(conv, seq).as_deref(), Some("sort the list"));

    // Commit-late: finalize emits exactly the index_turn-shaped EOU.
    tier.commit_reply_frontier(conv, seq);
    let drained = impulses.drain_ready();
    assert_eq!(drained.len(), 1, "one commit EOU per finalize");
    let imp = &drained[0];
    assert_eq!(imp.impulse_type, ImpulseType::EndOfUtterance);
    assert_eq!(imp.payload.get("chain_seq").and_then(|v| v.as_u64()), Some(seq));
    assert_eq!(
        imp.payload.get("conv_id").and_then(|v| v.as_str()),
        Some(conv)
    );
    assert_eq!(imp.hlc_timestamp, seq);
}

#[tokio::test]
async fn register_reply_frontier_none_without_forest_or_loop() {
    // No forest, no loop: nothing to register against — the caller just
    // dispatches (degrade, don't fabricate busy state).
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let tier = SessionTier::new(embedder, chain, None);
    assert!(tier.register_reply_frontier("c", "goal").await.is_none());
}

#[tokio::test]
async fn project_signals_intent_backchannel_short_and_busy() {
    let conv = "w21-c2";
    let (tier, _chain, _loop, _impulses, _crossrefs) = wired(conv);

    // Correction cue → Intent::Correction (the Refine trigger).
    let s = tier.project_interrupt_signals(conv, "actually make it blue", None, true);
    assert_eq!(s.intent, Intent::Correction);
    assert!(s.busy);
    assert!(!s.is_backchannel);

    // Wave-1 wire decomposition drives is_backchannel.
    let va = json!({ "paralinguistics": { "class": "backchannel_candidate" } });
    let s = tier.project_interrupt_signals(conv, "mm-hmm", Some(&va), true);
    assert!(s.is_backchannel);
    assert!(s.is_short, "≤3 words is short");

    // Plain speech class is not a backchannel; busy passes through.
    let va = json!({ "paralinguistics": { "class": "speech" } });
    let s = tier.project_interrupt_signals(
        conv,
        "write a function that sorts a list of numbers",
        Some(&va),
        false,
    );
    assert!(!s.is_backchannel);
    assert!(!s.is_short);
    assert!(!s.busy);
}

#[tokio::test]
async fn project_signals_topic_continuity_follows_the_classifier_carry() {
    let conv = "w21-c3";
    // Classifier attaches before Arc-wrapping (builder by-value); no loop
    // needed — topic carry is index-side only.
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());
    let tier = SessionTier::new(embedder, chain.clone(), None)
        .with_forest(causal, crossrefs)
        .with_classifier(Arc::new(KeywordTurnClassifier::new()));

    // Seed the conversation's topic via a normal indexed turn.
    let ev = chain.append("agent", "agent.chat.turn", Some(json!({})));
    tier.index_turn(
        conv,
        ev.sequence,
        "agent.chat.turn",
        "user",
        "write a function that sorts a list of numbers",
        None,
    )
    .await;

    // A continuation that re-mentions the carried topic token ("write", the
    // seed turn's top token) classifies to the same topic → continuous.
    let s = tier.project_interrupt_signals(conv, "also write the duplicates last", None, true);
    assert!(
        s.topically_continuous,
        "continuation should carry the conversation topic"
    );

    // Without a prior topic there is nothing to be continuous WITH.
    let s = tier.project_interrupt_signals("fresh-conv", "also make it blue", None, true);
    assert!(!s.topically_continuous);
}

#[tokio::test]
async fn emit_backchannel_draws_a_continuer_never_a_turn() {
    // WEFT-650: a backchannel ("mm-hmm") while busy must land as a Continuer
    // cross-ref to the in-flight turn — never a new turn node.
    let conv = "w22-bc1";
    let (tier, _chain, talk_loop, _impulses, crossrefs) = wired(conv);

    let in_flight = tier
        .register_reply_frontier(conv, "sort the list")
        .await
        .expect("forest + loop attached");
    assert_eq!(talk_loop.current_turn(conv), Some(in_flight));

    assert!(tier.emit_backchannel(conv, Some(in_flight)), "talk loop is attached");
    let result = talk_loop.tick();
    assert_eq!(result.backchannels, 1, "exactly one Continuer this tick");
    assert_eq!(result.commits, 0, "a backchannel commits no turn");

    // The in-flight attempt is still busy — a backchannel never displaces it.
    assert_eq!(talk_loop.current_turn(conv), Some(in_flight));

    // The Continuer cross-ref landed on the store, targeting the in-flight
    // turn's uid (never minting a turn node of its own).
    let continuers: Vec<_> = crossrefs
        .all()
        .into_iter()
        .filter(|cr| cr.ref_type == CrossRefType::Continuer)
        .collect();
    assert_eq!(continuers.len(), 1, "one Continuer landed");
}

#[tokio::test]
async fn backchannel_targets_the_attempt_even_after_current_turn_moves() {
    // The live w26-probe bug shape: a mid-flight recorded turn (the "mm-hmm"
    // utterance itself) overwrites `current_turn` and its commit-tick CLEARS
    // it — so a Continuer relying on `current_turn` at drain time targets the
    // wrong node or drops. The explicit `turn_seq` payload must pin the
    // in-flight attempt regardless.
    let conv = "w22-bc2";
    let (tier, chain, talk_loop, _impulses, crossrefs) = wired(conv);

    let attempt = tier
        .register_reply_frontier(conv, "write the essay")
        .await
        .expect("attempt registered");

    // The backchannel utterance is recorded like any turn: it registers
    // (overwriting current_turn) and commits on the next tick (clearing it).
    let ev = chain.append("agent", "agent.chat.turn", Some(json!({})));
    tier.index_turn(conv, ev.sequence, "agent.chat.turn", "user", "mm-hmm", None)
        .await;
    talk_loop.tick();
    assert_eq!(
        talk_loop.current_turn(conv),
        None,
        "the recorded turn's commit cleared current_turn — the bug precondition"
    );

    // Explicit target still lands the Continuer on the attempt.
    assert!(tier.emit_backchannel(conv, Some(attempt)));
    let result = talk_loop.tick();
    assert_eq!(result.backchannels, 1, "Continuer landed via explicit turn_seq");
    let attempt_uid = crate::session_forest::turn_universal_id(conv, attempt, "write the essay");
    let continuer_hits_attempt = crossrefs
        .all()
        .into_iter()
        .filter(|cr| cr.ref_type == CrossRefType::Continuer)
        .any(|cr| cr.target == attempt_uid && cr.chain_seq == attempt);
    assert!(continuer_hits_attempt, "Continuer targets the in-flight attempt");
}

#[tokio::test]
async fn emit_backchannel_is_a_noop_without_a_talk_loop() {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let tier = SessionTier::new(embedder, chain, None);
    assert!(!tier.emit_backchannel("c", None));
}

#[tokio::test]
async fn live_interrupt_phrase_routes_to_refine() {
    // The exact live-smoke phrase that (2026-07-06) pruned WITHOUT resubmitting
    // — pin the projection + routing end to end.
    let conv = "w21-c4";
    let (tier, _chain, _loop, _impulses, _crossrefs) = wired(conv);
    let text = "hold on, actually just give me one plain-text sentence about rivers instead";
    let s = tier.project_interrupt_signals(conv, text, None, true);
    let action = crate::interrupt_router::InterruptRouter::new().route(&s);
    assert!(
        matches!(action, crate::interrupt_router::InterruptAction::Refine { .. }),
        "expected Refine, got {action:?} (intent={:?})",
        s.intent
    );
}
