//! Bounded, drop-oldest job queue for the Phase-B async classification
//! enrichment tier (classification-design §D4, plan B3).
//!
//! `index_turn` (the witness path) enqueues one [`EnrichJob`] per committed turn
//! when `[kernel.agent.classification] mode = full`; a single daemon drain task
//! ([`recv`](EnrichQueue::recv)) pulls jobs, runs the cheap-model four-axis
//! enrichment off the turn path, and patches the node blob (`tier: "llm"`).
//!
//! The queue exists to decouple that LLM latency from the turn. Two properties
//! are load-bearing (design §D1, §7 risk 1):
//!
//! - **Enqueue never blocks or awaits.** [`enqueue`](EnrichQueue::enqueue) takes
//!   the mutex, mutates a `VecDeque`, and returns — safe to call synchronously on
//!   the anchor path. A stalled enrichment backend can never stall `index_turn`.
//! - **Drop-oldest overflow.** When the drain falls behind and the queue is at
//!   its bound, the *oldest* pending job is dropped (logged) to make room for the
//!   new one. Best-effort and honest: a refinement is a quality upgrade, never a
//!   correctness requirement, so shedding the stalest backlog is the right call.

use std::collections::VecDeque;
use std::sync::Mutex;

use tokio::sync::Notify;
use tracing::warn;

use clawft_kernel::causal::NodeId;

/// One unit of deferred enrichment work: refine the classification blob of an
/// already-committed turn node. Carries exactly what the drain task needs to
/// locate the node and prompt the model (design §D4 job shape).
#[derive(Debug, Clone)]
pub struct EnrichJob {
    /// Conversation the turn belongs to (for logging / routing).
    pub conv_id: String,
    /// Causal node whose `classification` blob will be patched.
    pub node_id: NodeId,
    /// Witness-chain sequence of the turn (its universal key).
    pub chain_seq: u64,
    /// Turn text to classify.
    pub text: String,
}

/// A bounded MPSC-style queue with **drop-oldest** overflow. Producers
/// ([`enqueue`](Self::enqueue)) never block or await; the single consumer
/// ([`recv`](Self::recv)) awaits when the queue is empty.
///
/// Backed by a plain `Mutex<VecDeque>` + a `Notify` rather than
/// `tokio::sync::mpsc` because tokio's bounded channel drops the *newest* item
/// on a full `try_send` — the opposite of the design's drop-oldest requirement —
/// and its unbounded channel has no bound at all.
pub struct EnrichQueue {
    inner: Mutex<VecDeque<EnrichJob>>,
    notify: Notify,
    bound: usize,
}

impl EnrichQueue {
    /// Create a queue holding at most `bound` pending jobs. `bound` is clamped to
    /// at least 1 so a misconfigured `queue_bound = 0` still makes progress
    /// (holding exactly one in-flight job) rather than dropping everything.
    pub fn new(bound: usize) -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            notify: Notify::new(),
            bound: bound.max(1),
        }
    }

    /// Enqueue a job. Non-blocking and never awaits — safe on the turn path. If
    /// the queue is already at its bound, the OLDEST pending job(s) are dropped
    /// (each logged at WARN) to make room, so a slow drain sheds its stalest
    /// backlog instead of applying backpressure to `index_turn` (design §D1).
    pub fn enqueue(&self, job: EnrichJob) {
        {
            let mut q = self.inner.lock().expect("enrich queue mutex poisoned");
            while q.len() >= self.bound {
                if let Some(dropped) = q.pop_front() {
                    warn!(
                        conv_id = %dropped.conv_id,
                        node_id = dropped.node_id,
                        bound = self.bound,
                        "enrich queue full — dropped oldest enrichment job"
                    );
                }
            }
            q.push_back(job);
        }
        // notify_one stores a permit if no consumer is currently parked, so a
        // notify that races ahead of `recv`'s park is not lost; the next
        // `notified().await` consumes it. One permit is enough because `recv`
        // always drains via `pop_front` before it ever awaits again.
        self.notify.notify_one();
    }

    /// Await and return the next job, oldest-first. Suspends when the queue is
    /// empty; woken by [`enqueue`](Self::enqueue). Cancel-safe: a job is removed
    /// only once this returns, so dropping the future mid-wait (e.g. the drain
    /// task losing a `select!` race to its cancel token) loses nothing.
    pub async fn recv(&self) -> EnrichJob {
        loop {
            if let Some(job) = self
                .inner
                .lock()
                .expect("enrich queue mutex poisoned")
                .pop_front()
            {
                return job;
            }
            self.notify.notified().await;
        }
    }

    /// Current pending-job count (test / observability).
    pub fn len(&self) -> usize {
        self.inner.lock().expect("enrich queue mutex poisoned").len()
    }

    /// Whether the queue currently holds no pending jobs.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(node_id: NodeId, text: &str) -> EnrichJob {
        EnrichJob {
            conv_id: "conv-1".to_string(),
            node_id,
            chain_seq: node_id,
            text: text.to_string(),
        }
    }

    #[test]
    fn enqueue_is_fifo_and_non_blocking() {
        let q = EnrichQueue::new(8);
        q.enqueue(job(1, "a"));
        q.enqueue(job(2, "b"));
        q.enqueue(job(3, "c"));
        assert_eq!(q.len(), 3);
    }

    #[test]
    fn enqueue_drops_oldest_when_full_without_blocking() {
        // Bound 2: the third enqueue must drop node 1 (oldest), never block.
        let q = EnrichQueue::new(2);
        q.enqueue(job(1, "oldest"));
        q.enqueue(job(2, "mid"));
        q.enqueue(job(3, "newest"));
        assert_eq!(q.len(), 2, "queue never exceeds its bound");

        // Remaining jobs are the two NEWEST, still oldest-first (2 then 3).
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            assert_eq!(q.recv().await.node_id, 2);
            assert_eq!(q.recv().await.node_id, 3);
        });
    }

    #[test]
    fn zero_bound_is_clamped_to_one() {
        let q = EnrichQueue::new(0);
        q.enqueue(job(1, "a"));
        q.enqueue(job(2, "b"));
        assert_eq!(q.len(), 1, "bound 0 clamps to 1 rather than dropping all");
    }

    #[tokio::test]
    async fn recv_awaits_then_wakes_on_enqueue() {
        use std::sync::Arc;
        let q = Arc::new(EnrichQueue::new(4));

        // Spawn a consumer that parks on the empty queue.
        let qc = Arc::clone(&q);
        let consumer = tokio::spawn(async move { qc.recv().await.node_id });

        // Give the consumer a moment to park, then enqueue — it must wake.
        tokio::task::yield_now().await;
        q.enqueue(job(42, "late"));

        let got = tokio::time::timeout(std::time::Duration::from_secs(1), consumer)
            .await
            .expect("consumer woke within timeout")
            .expect("consumer task ok");
        assert_eq!(got, 42);
    }

    #[tokio::test]
    async fn recv_drains_all_pending_before_parking_again() {
        // Multiple enqueues collapse to a single stored notify permit; recv must
        // still yield every pending job (it re-checks the deque before awaiting).
        let q = EnrichQueue::new(8);
        q.enqueue(job(1, "a"));
        q.enqueue(job(2, "b"));
        q.enqueue(job(3, "c"));
        assert_eq!(q.recv().await.node_id, 1);
        assert_eq!(q.recv().await.node_id, 2);
        assert_eq!(q.recv().await.node_id, 3);
        assert!(q.is_empty());
    }
}
