//! Integration tests for [`AgentService`].
//!
//! These exercise only the public surface (no `super::*`-style reach
//! into private adapters) and prove the lock + cancel + shutdown
//! semantics promised by `docs/plans/agent-core-v1.md` Phase C1.
//!
//! Lives in `tests/` rather than inline so `service.rs` stays under
//! the 500-line file ceiling per CLAUDE.md. Inline unit tests for the
//! private adapter functions (`inbound_from_params`,
//! `result_from_outbound`) remain in `service.rs::tests`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use clawft_service_agent::{
    AgentChatMessage, AgentChatParams, AgentLoopHandle, AgentService, AgentServiceError,
};
use clawft_types::event::{InboundMessage, OutboundMessage};
use tokio::sync::Notify;

/// Stub loop handle. Each call to `handle_turn` awaits an external
/// `release` Notify and tracks how many concurrent calls were live
/// at peak — that's how the parallel-vs-serial tests assert lock
/// behaviour without observing internal state of [`AgentService`].
struct StubHandle {
    release: Arc<Notify>,
    in_progress: Arc<AtomicUsize>,
    peak_concurrent: Arc<AtomicUsize>,
}

impl StubHandle {
    fn new(release: Arc<Notify>) -> Self {
        Self {
            release,
            in_progress: Arc::new(AtomicUsize::new(0)),
            peak_concurrent: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// Decrements `in_progress` on drop so cancel-by-future-drop (outer
/// `select!`) and cancel-by-token-observe (inner `select!`) both leave
/// the counter accurate (WEFT-323 mid-turn test).
struct InProgressGuard<'a> {
    counter: &'a AtomicUsize,
}
impl Drop for InProgressGuard<'_> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

#[async_trait]
impl AgentLoopHandle for StubHandle {
    async fn handle_turn(
        &self,
        msg: InboundMessage,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<OutboundMessage, String> {
        let now = self.in_progress.fetch_add(1, Ordering::AcqRel) + 1;
        let _guard = InProgressGuard {
            counter: &self.in_progress,
        };
        // Peak-concurrency tracking — CAS loop so we don't lose to a
        // racing peer.
        let mut peak = self.peak_concurrent.load(Ordering::Acquire);
        while now > peak {
            match self.peak_concurrent.compare_exchange(
                peak,
                now,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }
        // WEFT-323: honor the per-conv cancel token (iteration-boundary
        // equivalent for the stub). Outer select! still races this path.
        tokio::select! {
            _ = self.release.notified() => {}
            _ = cancel.cancelled() => {
                return Err(format!(
                    "conversation `{}` was cancelled",
                    msg.chat_id
                ));
            }
        }
        Ok(OutboundMessage {
            channel: msg.channel,
            chat_id: msg.chat_id,
            content: format!("echo: {}", msg.content),
            reply_to: None,
            media: Vec::new(),
            metadata: HashMap::new(),
        })
    }
}

fn params_for(conv_id: &str, content: &str) -> AgentChatParams {
    AgentChatParams {
        messages: vec![AgentChatMessage {
            role: "user".into(),
            content: content.into(),
        }],
        temperature: None,
        max_tokens: None,
        conv_id: conv_id.into(),
        metadata: None,
    }
}

#[tokio::test]
async fn dispatch_acquires_per_conv_lock() {
    // Two concurrent dispatches with the SAME conv_id must serialise:
    // peak concurrency in the stub stays at 1 throughout.
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    let svc1 = Arc::clone(&svc);
    let svc2 = Arc::clone(&svc);
    let h1 = tokio::spawn(async move { svc1.dispatch(params_for("c1", "first")).await });
    let h2 = tokio::spawn(async move { svc2.dispatch(params_for("c1", "second")).await });

    // Yield enough times for both tasks to reach the lock acquire.
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }

    // Release the first dispatch.
    release.notify_one();
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    // Release the second.
    release.notify_one();

    let r1 = h1.await.unwrap().unwrap();
    let r2 = h2.await.unwrap().unwrap();
    assert_eq!(r1.assistant_text, "echo: first");
    assert_eq!(r2.assistant_text, "echo: second");
    assert_eq!(
        stub.peak_concurrent.load(Ordering::Acquire),
        1,
        "same-conv dispatches must never overlap"
    );
}

#[tokio::test]
async fn dispatch_parallel_for_distinct_conv_ids() {
    // Different conv_ids must overlap: peak concurrency reaches 2.
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    let svc_a = Arc::clone(&svc);
    let svc_b = Arc::clone(&svc);
    let h_a = tokio::spawn(async move { svc_a.dispatch(params_for("a", "x")).await });
    let h_b = tokio::spawn(async move { svc_b.dispatch(params_for("b", "y")).await });

    // Spin until both stubs have entered handle_turn (50ms is
    // generous in a tokio:test single-thread scheduler).
    let deadline = std::time::Instant::now() + Duration::from_millis(50);
    while stub.in_progress.load(Ordering::Acquire) < 2 {
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::task::yield_now().await;
    }

    let peak = stub.peak_concurrent.load(Ordering::Acquire);
    // Drain — `notify_waiters()` is one-shot so we may need to nudge
    // a few times.
    for _ in 0..16 {
        release.notify_waiters();
        tokio::task::yield_now().await;
    }

    h_a.await.unwrap().unwrap();
    h_b.await.unwrap().unwrap();
    assert_eq!(peak, 2, "distinct-conv dispatches must overlap");
}

#[tokio::test]
async fn cancel_aborts_in_flight_dispatch() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    let svc_clone = Arc::clone(&svc);
    let handle =
        tokio::spawn(async move { svc_clone.dispatch(params_for("c-cancel", "wait")).await });

    // Wait for the stub to actually be inside handle_turn.
    let deadline = std::time::Instant::now() + Duration::from_millis(50);
    while stub.in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            panic!("stub never entered handle_turn");
        }
        tokio::task::yield_now().await;
    }

    svc.cancel("c-cancel");

    let result = handle.await.unwrap();
    match result {
        Err(AgentServiceError::Cancelled(id)) => assert_eq!(id, "c-cancel"),
        other => panic!("expected Cancelled, got {:?}", other),
    }

    // Release the stub so its task tree drops cleanly.
    release.notify_waiters();
}

/// WEFT-323: cancel must break a multi-iteration turn at the boundary,
/// not only when the outer `select!` drops a blocked future.
///
/// The stub simulates iteration-boundary observation by racing its
/// release wait against the per-conv token. Cancel after the stub
/// has entered `handle_turn` must surface as
/// [`AgentServiceError::Cancelled`] without requiring the release
/// Notify (i.e. mid-turn break via the threaded token path).
#[tokio::test]
async fn cancel_breaks_mid_turn_via_threaded_token() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    let svc_clone = Arc::clone(&svc);
    let handle = tokio::spawn(async move {
        svc_clone
            .dispatch(params_for("c-mid-turn", "long-tool-loop"))
            .await
    });

    let deadline = std::time::Instant::now() + Duration::from_millis(100);
    while stub.in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            panic!("stub never entered handle_turn");
        }
        tokio::task::yield_now().await;
    }

    // Trip the per-conv token while the turn is "in flight". The stub
    // observes it via select! on cancel.cancelled() (standing in for
    // run_tool_loop's iteration-boundary check); dispatch maps that
    // to Cancelled and re-arms a fresh token.
    svc.cancel("c-mid-turn");

    let result = tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("dispatch must complete after cancel (mid-turn break)")
        .expect("join ok");
    match result {
        Err(AgentServiceError::Cancelled(id)) => assert_eq!(id, "c-mid-turn"),
        other => panic!("expected Cancelled mid-turn, got {:?}", other),
    }

    // Do NOT notify release — cancel alone must have broken the turn.
    // Yield so the dropped future's InProgressGuard runs before we sample.
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        stub.in_progress.load(Ordering::Acquire),
        0,
        "mid-turn cancel must drop in_progress without release Notify"
    );
}

#[tokio::test]
async fn cancel_on_idle_arms_next_dispatch() {
    // A `cancel()` issued before any dispatch should still fire on
    // the very next dispatch — matches the spike's
    // `agent.chat.cancel` racing with the next `agent.chat`.
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    svc.cancel("future-conv");

    let res = svc.dispatch(params_for("future-conv", "x")).await;
    match res {
        Err(AgentServiceError::Cancelled(id)) => assert_eq!(id, "future-conv"),
        other => panic!("expected pre-armed cancel to fire, got {:?}", other),
    }
}

#[tokio::test]
async fn shutdown_returns_true_when_idle() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = AgentService::new(Arc::clone(&stub));

    let drained = svc.shutdown(Duration::from_millis(100)).await;
    assert!(drained, "idle shutdown must drain immediately");
}

#[tokio::test]
async fn shutdown_drains_in_flight_within_deadline() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    let svc_clone = Arc::clone(&svc);
    let dispatch_handle =
        tokio::spawn(async move { svc_clone.dispatch(params_for("c", "x")).await });

    let deadline = std::time::Instant::now() + Duration::from_millis(50);
    while stub.in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            panic!("stub never entered handle_turn");
        }
        tokio::task::yield_now().await;
    }

    // Shutdown cancels every known token, the dispatch returns
    // `Cancelled`, the in-flight counter drops to zero, drain wakes.
    let drained = svc.shutdown(Duration::from_secs(2)).await;
    assert!(drained, "shutdown must drain after cancel propagates");

    // Best-effort cleanup — dispatch already returned Cancelled.
    release.notify_waiters();
    let _ = dispatch_handle.await;
}

#[tokio::test]
async fn shutdown_refuses_new_dispatches() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = AgentService::new(Arc::clone(&stub));

    // Idle drain.
    let drained = svc.shutdown(Duration::from_millis(100)).await;
    assert!(drained);

    // Post-shutdown dispatch refuses.
    let r = svc.dispatch(params_for("c", "ignored")).await;
    match r {
        Err(AgentServiceError::ShuttingDown) => {}
        other => panic!("expected ShuttingDown, got {:?}", other),
    }
}

// -- WEFT-322: agent.chat.reset_budget integration ---------------------------

#[tokio::test]
async fn reset_budget_without_budget_attached_returns_no_budget_error() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = AgentService::new(Arc::clone(&stub));

    let r = svc.reset_budget("conv-x");
    assert!(matches!(r, Err(AgentServiceError::NoBudget)));
}

#[tokio::test]
async fn reset_budget_clears_circuit_and_returns_prior_snapshot() {
    use clawft_core::agent::cost_budget::{BudgetStore, ConversationBudget, InMemoryBudgetStore};
    use clawft_types::config::CostBudgetConfig;

    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let store: Arc<dyn BudgetStore> = Arc::new(InMemoryBudgetStore::new());
    let budget = Arc::new(ConversationBudget::new(
        CostBudgetConfig {
            max_tokens_per_conv: 1_000,
            max_usd_per_conv: 1.0,
            max_iterations_per_conv: 10,
        },
        Arc::clone(&store),
    ));

    // Pre-trip the budget directly through the budget façade — the
    // service-level test doesn't need a full agent loop to exercise
    // the reset RPC contract.
    budget.record_call("conv-rb", 100, 50, 0.0).unwrap();
    budget.mark_open("conv-rb", "tokens").unwrap();
    assert!(budget.usage("conv-rb").circuit_open);

    let svc = AgentService::new(Arc::clone(&stub)).with_cost_budget(Arc::clone(&budget));

    let prev = svc.reset_budget("conv-rb").expect("reset should succeed");
    assert!(prev.circuit_open, "snapshot must reflect tripped state");
    assert_eq!(prev.input_tokens, 100);

    // Post-reset: circuit closed, accumulator zero.
    let post = budget.usage("conv-rb");
    assert!(!post.circuit_open);
    assert_eq!(post.input_tokens, 0);
}

// ── ADR-058 Phase 5 deferred step 4: conversation-end promotion ──────────
//
// Deterministic wiring test (Mock embedder + in-memory chain, no live LLM).
// Proves the `agent.chat.end` path: an attached SessionTier promotes on a
// supplied durable_fact, or drops the view when none is supplied. The live
// LLM-postmortem leg (durable_fact synthesis via Hermes) is exercised by the
// #[ignore]'d test in `clawft-weave`.

use clawft_kernel::chain::ChainManager;
use clawft_kernel::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use clawft_service_agent::SessionTier;

async fn index_one(tier: &SessionTier, chain: &ChainManager, conv: &str, text: &str) -> u64 {
    let ev = chain.append(
        "agent",
        "agent.chat.turn",
        Some(serde_json::json!({ "conv": conv, "content": text })),
    );
    tier.index_turn(conv, ev.sequence, "agent.chat.turn", "user", text, None)
        .await;
    ev.sequence
}

#[tokio::test]
async fn end_conversation_promotes_with_durable_fact() {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let tier = Arc::new(SessionTier::new(embedder, chain.clone(), None));
    let conv = "end-promote";
    let s = index_one(&tier, &chain, conv, "the api base is https://x").await;
    assert!(tier.mark_important(conv, s));

    // The digest (postmortem prompt source) carries the conversation text.
    let digest = tier.conversation_digest(conv, 4096).unwrap();
    assert!(
        digest.contains("api base"),
        "digest carries turn text: {digest}"
    );
    assert_eq!(tier.active_conversations(), vec![conv.to_string()]);

    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(release));
    let svc = AgentService::new(stub).with_session_tier(Arc::clone(&tier));

    // Supplying a durable_fact (as the daemon does after the LLM postmortem)
    // promotes to the trunk and drops the view.
    let promoted = svc
        .end_conversation(conv, Some("api base = https://x"))
        .expect("promotion event emitted");
    let ev = chain
        .tail_from(promoted - 1)
        .into_iter()
        .find(|e| e.sequence == promoted)
        .expect("memory.promote event present");
    assert_eq!(ev.kind, "memory.promote");
    // View dropped: the conversation is no longer active.
    assert!(tier.active_conversations().is_empty());
}

#[tokio::test]
async fn end_conversation_without_fact_drops_view() {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let tier = Arc::new(SessionTier::new(embedder, chain.clone(), None));
    let conv = "end-drop";
    index_one(&tier, &chain, conv, "ephemeral chatter").await;

    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(release));
    let svc = AgentService::new(stub).with_session_tier(Arc::clone(&tier));

    // No durable_fact → the ephemeral view is dropped, nothing promoted.
    assert!(svc.end_conversation(conv, None).is_none());
    assert!(tier.active_conversations().is_empty());
    // An empty fact is treated the same as None (no garbage promotion).
    let conv2 = "end-blank";
    index_one(&tier, &chain, conv2, "more chatter").await;
    assert!(svc.end_conversation(conv2, Some("   ")).is_none());
    assert!(tier.active_conversations().is_empty());
}

#[tokio::test]
async fn end_conversation_no_tier_is_noop() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(release));
    let svc = AgentService::new(stub);
    // No tier attached → end_conversation is a clean no-op.
    assert!(svc.end_conversation("whatever", Some("fact")).is_none());
}

#[tokio::test]
async fn cancel_of_in_flight_turn_does_not_kill_the_queued_resubmit() {
    // Wave 2 §W2.4 live shape: dispatch A is in flight; dispatch B (the
    // amendment resubmit) clones the conv token and queues on the conv lock;
    // cancel() kills A (which re-arms the map token on its way out). B's
    // LOCAL clone is the tripped token — the stale-clone re-read must adopt
    // the re-armed map token so the steer actually generates.
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    // A: enters handle_turn and blocks on `release`.
    let svc_a = Arc::clone(&svc);
    let a = tokio::spawn(async move { svc_a.dispatch(params_for("c-steer", "essay")).await });
    let deadline = std::time::Instant::now() + Duration::from_millis(200);
    while stub.in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            panic!("A never entered handle_turn");
        }
        tokio::task::yield_now().await;
    }

    // B: clones the (soon-to-be-tripped) token, then queues on the conv lock.
    let svc_b = Arc::clone(&svc);
    let b = tokio::spawn(async move { svc_b.dispatch(params_for("c-steer", "amendment")).await });
    // Give B a beat to pass token-clone and block on the lock.
    tokio::time::sleep(Duration::from_millis(20)).await;

    // The executor's cancel: A dies, re-arming the map token.
    svc.cancel("c-steer");
    match a.await.unwrap() {
        Err(AgentServiceError::Cancelled(id)) => assert_eq!(id, "c-steer"),
        other => panic!("expected A cancelled, got {other:?}"),
    }

    // B must RUN (not die on its stale clone). Keep releasing the stub until
    // B finishes — B enters handle_turn only after acquiring the conv lock.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !b.is_finished() {
        if std::time::Instant::now() > deadline {
            panic!("B never finished — stale token killed the resubmit?");
        }
        release.notify_waiters();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    match b.await.unwrap() {
        Ok(result) => assert!(
            result.assistant_text.contains("amendment"),
            "{result:?}"
        ),
        other => panic!("expected B to generate the steer, got {other:?}"),
    }
}

// -- WEFT-333: agent.chat metrics (last-completion / lock contention) -------

#[tokio::test]
async fn dispatch_records_last_completion_time() {
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    assert_eq!(
        svc.metrics().snapshot().last_completion_unix_ms,
        0,
        "no completions yet"
    );

    let svc_c = Arc::clone(&svc);
    let handle = tokio::spawn(async move { svc_c.dispatch(params_for("c", "hi")).await });

    // Wait until the stub is blocked on Notify so our wake is observed.
    let deadline = std::time::Instant::now() + Duration::from_millis(200);
    while stub.in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            panic!("dispatch never entered handle_turn");
        }
        tokio::task::yield_now().await;
    }
    release.notify_waiters();

    let r = handle.await.unwrap().unwrap();
    assert!(r.assistant_text.contains("hi"));

    let snap = svc.metrics().snapshot();
    assert!(
        snap.last_completion_unix_ms > 0,
        "completion must stamp last_completion"
    );
    assert_eq!(snap.in_flight, 0);
}

#[tokio::test]
async fn same_conv_contention_increments_lock_contentions() {
    // Two concurrent dispatches on the same conv_id: the second must wait
    // on the per-conv mutex and bump lock_contentions.
    let release = Arc::new(Notify::new());
    let stub = Arc::new(StubHandle::new(Arc::clone(&release)));
    let svc = Arc::new(AgentService::new(Arc::clone(&stub)));

    let svc1 = Arc::clone(&svc);
    let h1 = tokio::spawn(async move { svc1.dispatch(params_for("c-lock", "first")).await });

    // Wait until first is inside handle_turn (holds the lock).
    let deadline = std::time::Instant::now() + Duration::from_millis(200);
    while stub.in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            panic!("first dispatch never entered handle_turn");
        }
        tokio::task::yield_now().await;
    }

    let svc2 = Arc::clone(&svc);
    let h2 = tokio::spawn(async move { svc2.dispatch(params_for("c-lock", "second")).await });

    // Give second time to block on the lock (contention path).
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Drain both.
    for _ in 0..32 {
        release.notify_waiters();
        tokio::task::yield_now().await;
    }
    let _ = h1.await.unwrap().unwrap();
    let _ = h2.await.unwrap().unwrap();

    let snap = svc.metrics().snapshot();
    assert!(
        snap.lock_contentions >= 1,
        "expected at least one per-conv lock contention, got {}",
        snap.lock_contentions
    );
    assert!(snap.last_completion_unix_ms > 0);
}
