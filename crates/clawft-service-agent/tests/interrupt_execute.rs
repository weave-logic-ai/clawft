//! Wave 2 §W2.3/§W2.4 end-to-end: `AgentService::execute_interrupt` against a
//! REAL forest-joined `SessionTier` and a wired `ReplySubmitter` — the full
//! Refine closure (cancel → resubmit goal+amendment → prune old → Contradicts
//! → witness) that the unit tests only cover piecewise.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use clawft_service_agent::interrupt_router::{InterruptAction, InterruptCtx};
use clawft_service_agent::{
    AgentLoopHandle, AgentService, ReplySubmitter, SessionTier,
};
use clawft_types::event::{InboundMessage, OutboundMessage};

use clawft_kernel::chain::ChainManager;
use clawft_kernel::context_graft::SessionView;
use clawft_kernel::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use clawft_kernel::{
    CausalGraph, CognitiveTick, CognitiveTickConfig, CrossRefStore, ImpulseQueue,
    SingleViewResolver, TalkModeConfig, TalkModeLoop, ViewResolver,
};

/// Loop handle stub — `execute_interrupt` never reaches it (the Refine arm
/// dispatches through the submitter, which this test stubs too).
struct NoopHandle;

#[async_trait]
impl AgentLoopHandle for NoopHandle {
    async fn handle_turn(&self, msg: InboundMessage) -> Result<OutboundMessage, String> {
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

/// Register-only submitter: the daemon submitter minus the off-path
/// `agent.chat` dispatch — registration is the part the executor's forest
/// closure depends on (the amendment seq for the Contradicts edge).
struct RegisterOnlySubmitter {
    tier: Arc<SessionTier>,
}

#[async_trait]
impl ReplySubmitter for RegisterOnlySubmitter {
    async fn submit_reply(&self, conv_id: &str, goal_text: &str) -> Option<u64> {
        self.tier.register_reply_frontier(conv_id, goal_text)
    }
}

fn wired(conv: &str) -> (Arc<SessionTier>, Arc<ChainManager>, Arc<TalkModeLoop>) {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());
    let impulses = Arc::new(ImpulseQueue::new());
    let resolver: Arc<dyn ViewResolver> =
        Arc::new(SingleViewResolver::new(Arc::new(SessionView::new(conv, 64))));
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let talk_loop = Arc::new(TalkModeLoop::new(
        impulses,
        causal.clone(),
        crossrefs.clone(),
        resolver,
        tick,
        TalkModeConfig::default(),
    ));
    let tier = Arc::new(
        SessionTier::new(embedder, chain.clone(), None)
            .with_forest(causal, crossrefs)
            .with_talk_loop(talk_loop.clone()),
    );
    (tier, chain, talk_loop)
}

#[tokio::test]
async fn refine_resubmits_prunes_old_and_witnesses() {
    let conv = "refine-e2e";
    let (tier, chain, talk_loop) = wired(conv);

    let service = AgentService::new(Arc::new(NoopHandle)).with_session_tier(tier.clone());
    service.set_reply_submitter(Arc::new(RegisterOnlySubmitter { tier: tier.clone() }));

    // An in-flight reply attempt (what §W2.1 registers when the turn starts).
    let old = tier
        .register_reply_frontier(conv, "sort the list")
        .expect("attempt registered");
    assert_eq!(talk_loop.current_turn(conv), Some(old));

    let outcome = service
        .execute_interrupt(
            InterruptAction::Refine {
                amendment: "actually make it descending".into(),
            },
            InterruptCtx {
                conv_id: conv.into(),
                in_flight_seq: Some(old),
            },
        )
        .await;

    // The amendment attempt was registered and is now the in-flight turn.
    let new = outcome.amendment_seq.expect("amendment registered");
    assert_ne!(new, old);
    assert_eq!(talk_loop.current_turn(conv), Some(new));
    // The old attempt is the prune target; Contradicts is drawn by the loop's
    // TurnClaim handler off the emitted impulse.
    assert_eq!(outcome.pruned_seq, Some(old));
    assert!(outcome.contradicts);
    assert!(outcome.witnessed);

    // The combined goal landed on the amendment node — a later Refine of the
    // amendment reconstructs the full lineage-free goal.
    let combined = tier.goal_for(conv, new).expect("goal stashed");
    assert!(combined.contains("sort the list"), "{combined}");
    assert!(combined.contains("[amendment] actually make it descending"), "{combined}");

    // The turn-level cancel marker is on the witness chain.
    let cancel_witnessed = chain
        .tail(32)
        .iter()
        .any(|e| e.kind == "agent.turn.cancel");
    assert!(cancel_witnessed, "cancel marker witnessed on the chain");
}

#[tokio::test]
async fn stop_prunes_and_witnesses_without_resubmit() {
    let conv = "stop-e2e";
    let (tier, chain, talk_loop) = wired(conv);

    let service = AgentService::new(Arc::new(NoopHandle)).with_session_tier(tier.clone());
    service.set_reply_submitter(Arc::new(RegisterOnlySubmitter { tier: tier.clone() }));

    let old = tier
        .register_reply_frontier(conv, "write a poem")
        .expect("attempt registered");

    let outcome = service
        .execute_interrupt(
            InterruptAction::Stop,
            InterruptCtx {
                conv_id: conv.into(),
                in_flight_seq: Some(old),
            },
        )
        .await;

    assert_eq!(outcome.pruned_seq, Some(old));
    assert!(outcome.witnessed);
    assert_eq!(outcome.amendment_seq, None, "a bare STOP never resubmits");
    assert!(!outcome.contradicts);
    assert_eq!(
        talk_loop.current_turn(conv),
        Some(old),
        "STOP leaves the pruned attempt as the claim-less prune target (the \
         loop's TurnClaim tick tombstones it; no new turn claims the floor)"
    );
    let cancel_witnessed = chain
        .tail(32)
        .iter()
        .any(|e| e.kind == "agent.turn.cancel");
    assert!(cancel_witnessed);
}
