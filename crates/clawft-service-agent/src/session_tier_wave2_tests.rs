//! Voice Wave 2 §W2.1 tests: the register-early/commit-late reply path and
//! the interrupt-signal projection. Sibling file to `session_tier_tests.rs`
//! (the <500-line rule); same deterministic fixtures — Mock embedder,
//! in-memory chain, hand-held impulse queue.

use super::*;
use clawft_kernel::chain::ChainManager;
use clawft_kernel::context_graft::SessionView;
use clawft_kernel::embedding::MockEmbeddingProvider;
use clawft_kernel::{
    CausalGraph, CognitiveTick, CognitiveTickConfig, CrossRefStore, ImpulseQueue,
    SingleViewResolver, TalkModeConfig, TalkModeLoop,
};
use serde_json::json;

use crate::dialogue_act::Intent;
use crate::turn_classifier::KeywordTurnClassifier;

/// Forest-joined tier + hosted loop over a queue we hold, so register/commit
/// emissions are observable. Mirrors `index_turn_registers_and_emits_one_eou`.
fn wired(
    conv: &str,
) -> (
    SessionTier,
    Arc<ChainManager>,
    Arc<TalkModeLoop>,
    Arc<ImpulseQueue>,
) {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());
    let impulses = Arc::new(ImpulseQueue::new());
    let resolver: Arc<dyn ViewResolver> =
        Arc::new(SingleViewResolver::new(Arc::new(SessionView::new(conv, 64))));
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let talk_loop = Arc::new(TalkModeLoop::new(
        impulses.clone(),
        causal.clone(),
        crossrefs.clone(),
        resolver,
        tick,
        TalkModeConfig::default(),
    ));
    let tier = SessionTier::new(embedder, chain.clone(), None)
        .with_forest(causal, crossrefs)
        .with_talk_loop(talk_loop.clone());
    (tier, chain, talk_loop, impulses)
}

#[tokio::test]
async fn register_reply_frontier_is_busy_without_eou_and_stashes_goal() {
    let conv = "w21-c1";
    let (tier, _chain, talk_loop, impulses) = wired(conv);

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
    let (tier, _chain, _loop, _impulses) = wired(conv);

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
    let (tier, chain, _loop, _impulses) = wired(conv);
    let tier = tier.with_classifier(Arc::new(KeywordTurnClassifier::new()));

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
