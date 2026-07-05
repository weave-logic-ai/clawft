//! Integration (M4): subagent lifecycle caps enforced at the spawner boundary
//! (design D5) — the concurrency cap, the depth cap, and cancellation.
//!
//! These drive the real `DaemonSubagentSpawner` + `SpawnRegistry` against a
//! blocking stub `AgentLoopHandle`, so dispatched children stay *live* and hold
//! their concurrency slots deterministically (the spawner reserves a slot
//! synchronously in `spawn()` before dispatch and releases it only when the child
//! finishes — a fast stub would race the release, which is why the in-crate unit
//! test asserts the 6th refusal only conditionally). Here the children never
//! finish until released, so the caps are exact.
//!
//! Complements the spawner's in-crate unit tests (`subagent.rs`): this file pins
//! the cancel path (untested there) and a deterministic concurrency refusal.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use clawft_core::agent::spawn::{SpawnError, SpawnSpec, SubagentSpawner, TaskStatus};
use clawft_service_agent::{
    AgentLoopHandle, AgentService, DaemonSubagentSpawner, SpawnRegistry, SubagentConfig,
};
use clawft_types::event::{InboundMessage, OutboundMessage};
use tokio::sync::Notify;

/// A loop handle whose `handle_turn` blocks on a shared `Notify` until released,
/// so a dispatched child conversation stays live (holding its concurrency slot).
struct BlockingHandle {
    release: Arc<Notify>,
    in_progress: Arc<AtomicUsize>,
}

#[async_trait]
impl AgentLoopHandle for BlockingHandle {
    async fn handle_turn(&self, msg: InboundMessage) -> Result<OutboundMessage, String> {
        self.in_progress.fetch_add(1, Ordering::AcqRel);
        self.release.notified().await;
        self.in_progress.fetch_sub(1, Ordering::AcqRel);
        Ok(OutboundMessage {
            channel: msg.channel,
            chat_id: msg.chat_id,
            content: "done".into(),
            reply_to: None,
            media: Vec::new(),
            metadata: HashMap::new(),
        })
    }
}

fn blocking_service() -> (Arc<AgentService<BlockingHandle>>, Arc<Notify>, Arc<AtomicUsize>) {
    let release = Arc::new(Notify::new());
    let in_progress = Arc::new(AtomicUsize::new(0));
    let handle = Arc::new(BlockingHandle {
        release: Arc::clone(&release),
        in_progress: Arc::clone(&in_progress),
    });
    (Arc::new(AgentService::new(handle)), release, in_progress)
}

fn detached_spec(parent: &str) -> SpawnSpec {
    let mut s = SpawnSpec::new("do work");
    s.parent_conv_id = Some(parent.into());
    s.await_completion = false;
    s
}

/// D5 concurrency cap: with five live children holding the parent's slots, the
/// sixth spawn is refused with `ConcurrencyExceeded` — deterministically, because
/// the blocking stub keeps all five children from finishing.
#[tokio::test]
async fn sixth_concurrent_spawn_is_refused() {
    let registry = Arc::new(SpawnRegistry::new());
    let config = SubagentConfig {
        max_per_conv: 5,
        ..SubagentConfig::default()
    };
    let spawner: Arc<DaemonSubagentSpawner<BlockingHandle>> =
        Arc::new(DaemonSubagentSpawner::new(Arc::clone(&registry), config));
    let (service, release, _in_progress) = blocking_service();
    spawner.set_service(Arc::downgrade(&service));

    // Five detached children reserve all five slots (reservation is synchronous
    // in spawn(); the blocking stub keeps them from releasing).
    for _ in 0..5 {
        let h = spawner.spawn(detached_spec("P")).await.expect("spawn ok");
        assert_eq!(h.status, TaskStatus::Running);
    }
    assert_eq!(registry.live_count("P"), 5, "five live children saturate the cap");

    // The sixth is refused.
    let err = spawner.spawn(detached_spec("P")).await.unwrap_err();
    assert!(
        matches!(err, SpawnError::ConcurrencyExceeded { max: 5 }),
        "sixth spawn must hit the concurrency cap, got {err:?}"
    );

    // A DIFFERENT parent conversation is unaffected (cap is per-parent, D5).
    let other = spawner.spawn(detached_spec("Q")).await.expect("distinct parent ok");
    assert_eq!(other.status, TaskStatus::Running);

    release.notify_waiters(); // let the blocked children drain on teardown
}

/// D5 depth cap: a spawn whose child depth would exceed the WEFT-180 cap is
/// refused with `DepthExceeded` before any dispatch.
#[tokio::test]
async fn depth_cap_is_enforced() {
    let registry = Arc::new(SpawnRegistry::new());
    let spawner: Arc<DaemonSubagentSpawner<BlockingHandle>> = Arc::new(DaemonSubagentSpawner::new(
        registry,
        SubagentConfig::default(), // max_depth 3
    ));
    let (service, _release, _ip) = blocking_service();
    spawner.set_service(Arc::downgrade(&service));

    let mut spec = detached_spec("P");
    spec.depth = 4; // child depth 4 > cap 3
    let err = spawner.spawn(spec).await.unwrap_err();
    assert!(
        matches!(err, SpawnError::DepthExceeded { depth: 4, max: 3 }),
        "depth 4 must be refused against cap 3, got {err:?}"
    );
}

/// D5 cancellation: a running child is cancelled to a terminal `Cancelled` state
/// (the primitive the parent-end cascade, `cascade_end_children`, is built on).
#[tokio::test]
async fn cancel_marks_the_child_cancelled() {
    let registry = Arc::new(SpawnRegistry::new());
    let spawner: Arc<DaemonSubagentSpawner<BlockingHandle>> =
        Arc::new(DaemonSubagentSpawner::new(Arc::clone(&registry), SubagentConfig::default()));
    let (service, release, in_progress) = blocking_service();
    spawner.set_service(Arc::downgrade(&service));

    let handle = spawner.spawn(detached_spec("P")).await.expect("spawn ok");
    assert_eq!(handle.status, TaskStatus::Running);

    // Wait until the child is actually inside handle_turn (dispatched + live).
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while in_progress.load(Ordering::Acquire) == 0 {
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::task::yield_now().await;
    }

    spawner.cancel(&handle.task_id).await.expect("cancel ok");

    let outcome = spawner
        .status(&handle.task_id)
        .await
        .expect("task still tracked after cancel");
    assert_eq!(
        outcome.status,
        TaskStatus::Cancelled,
        "cancel drives the child to the terminal Cancelled state"
    );

    release.notify_waiters();
}
