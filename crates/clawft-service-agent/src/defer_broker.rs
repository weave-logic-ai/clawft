//! Interactive defer broker — WEFT-331.
//!
//! Bridges the agent loop's [`DeferInteractor`](clawft_core::agent::defer::DeferInteractor)
//! to the panel RPC `agent.chat.defer_decide`:
//!
//! 1. Loop hits `GateDecision::Defer` → calls [`InteractiveDeferBroker::await_decision`].
//! 2. Broker mints a `defer_id`, stores a oneshot, publishes the prompt
//!    via the optional [`DeferPromptPublisher`].
//! 3. Panel shows the prompt and fires `agent.chat.defer_decide`.
//! 4. [`InteractiveDeferBroker::decide`] resolves the oneshot.
//! 5. Loop resumes (allow → run tool; deny/cancel/timeout → structured
//!    envelope, no tool dispatch).
//!
//! # Timeout / default-deny
//!
//! Wait budget defaults to
//! [`DEFER_DEFAULT_TIMEOUT_MS`](clawft_types::agent_chat::DEFER_DEFAULT_TIMEOUT_MS)
//! (120s). On expiry the waiter resolves as [`DeferOutcome::Timeout`]
//! (default-deny). Per-conv cancel (WEFT-323) maps to
//! [`DeferOutcome::Cancelled`].

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use clawft_core::agent::defer::{DeferInteractor, DeferOutcome, DeferRequest};
use clawft_types::agent_chat::{
    AgentChatDeferDecideResult, DEFER_DEFAULT_TIMEOUT_MS, DeferPromptEvent, DeferUserDecision,
};
use dashmap::DashMap;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Publishes a pending defer prompt for panel consumption.
///
/// Daemon implements this by writing an `awaiting_defer` stream frame
/// plus the dedicated `chat_defer_path` substrate value.
#[async_trait]
pub trait DeferPromptPublisher: Send + Sync + 'static {
    /// Surface a new pending defer to the panel.
    async fn publish_prompt(&self, prompt: &DeferPromptEvent);
    /// Clear the pending prompt after decide / timeout / cancel.
    async fn clear_prompt(&self, conv_id: &str, defer_id: &str);
}

/// No-op publisher used in unit tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopDeferPublisher;

#[async_trait]
impl DeferPromptPublisher for NoopDeferPublisher {
    async fn publish_prompt(&self, _prompt: &DeferPromptEvent) {}
    async fn clear_prompt(&self, _conv_id: &str, _defer_id: &str) {}
}

struct PendingSlot {
    conv_id: String,
    tx: oneshot::Sender<DeferUserDecision>,
}

/// Shared broker: loop waiters + panel decision RPC.
pub struct InteractiveDeferBroker {
    pending: DashMap<String, PendingSlot>,
    /// Latest defer_id per conv (for diagnostics / single-pending UI).
    by_conv: DashMap<String, String>,
    counter: AtomicU64,
    publisher: Arc<dyn DeferPromptPublisher>,
    default_timeout: Duration,
}

impl InteractiveDeferBroker {
    /// Construct a broker with the given publisher and default timeout.
    pub fn new(publisher: Arc<dyn DeferPromptPublisher>, default_timeout: Duration) -> Self {
        Self {
            pending: DashMap::new(),
            by_conv: DashMap::new(),
            counter: AtomicU64::new(1),
            publisher,
            default_timeout: if default_timeout.is_zero() {
                Duration::from_millis(DEFER_DEFAULT_TIMEOUT_MS)
            } else {
                default_timeout
            },
        }
    }

    /// Broker with no-op publisher and the wire default timeout.
    pub fn with_noop_publisher() -> Self {
        Self::new(
            Arc::new(NoopDeferPublisher),
            Duration::from_millis(DEFER_DEFAULT_TIMEOUT_MS),
        )
    }

    /// Resolve a pending defer from `agent.chat.defer_decide`.
    pub fn decide(
        &self,
        conv_id: &str,
        defer_id: &str,
        decision: DeferUserDecision,
    ) -> AgentChatDeferDecideResult {
        match self.pending.remove(defer_id) {
            Some((_, slot)) => {
                if slot.conv_id != conv_id {
                    // Put it back — wrong conv must not steal another waiter's
                    // decision (belt-and-suspenders against id guessing).
                    // But we already removed the oneshot; resolve as not
                    // accepted rather than re-inserting a broken half.
                    warn!(
                        defer_id,
                        expected = %slot.conv_id,
                        got = %conv_id,
                        "defer_decide conv_id mismatch"
                    );
                    // Drop the sender without sending → waiter sees hangover
                    // as cancelled/timeout. Better: send Cancel so the wait
                    // unblocks safely without running the tool.
                    let _ = slot.tx.send(DeferUserDecision::Cancel);
                    return AgentChatDeferDecideResult {
                        accepted: false,
                        defer_id: defer_id.into(),
                        decision: decision.as_str().into(),
                        error: Some("conv_id mismatch for defer_id".into()),
                    };
                }
                self.by_conv
                    .remove_if(conv_id, |_, id| id == defer_id);
                let dec_str = decision.as_str().to_string();
                match slot.tx.send(decision) {
                    Ok(()) => {
                        info!(conv_id, defer_id, decision = %dec_str, "defer decision accepted");
                        AgentChatDeferDecideResult {
                            accepted: true,
                            defer_id: defer_id.into(),
                            decision: dec_str,
                            error: None,
                        }
                    }
                    Err(_) => AgentChatDeferDecideResult {
                        accepted: false,
                        defer_id: defer_id.into(),
                        decision: dec_str,
                        error: Some("waiter dropped".into()),
                    },
                }
            }
            None => AgentChatDeferDecideResult {
                accepted: false,
                defer_id: defer_id.into(),
                decision: decision.as_str().into(),
                error: Some("no pending defer with that id".into()),
            },
        }
    }

    /// Peek at the active defer id for a conversation, if any.
    pub fn pending_defer_id(&self, conv_id: &str) -> Option<String> {
        self.by_conv.get(conv_id).map(|e| e.value().clone())
    }

    fn mint_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("defer-{n:016x}")
    }
}

#[async_trait]
impl DeferInteractor for InteractiveDeferBroker {
    async fn await_decision(
        &self,
        request: DeferRequest,
        cancel: &CancellationToken,
    ) -> DeferOutcome {
        let timeout = if request.timeout_ms == 0 {
            self.default_timeout
        } else {
            Duration::from_millis(request.timeout_ms)
        };
        let gate_reason = request.reason.clone();
        let defer_id = self.mint_id();
        let mut prompt = request.into_prompt_event(&defer_id);
        prompt.ts_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        );
        let conv_id = prompt.conv_id.clone();

        let (tx, rx) = oneshot::channel();
        self.pending.insert(
            defer_id.clone(),
            PendingSlot {
                conv_id: conv_id.clone(),
                tx,
            },
        );
        self.by_conv.insert(conv_id.clone(), defer_id.clone());

        self.publisher.publish_prompt(&prompt).await;
        debug!(%conv_id, %defer_id, tool = %prompt.tool, "defer waiter registered");

        let outcome = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                DeferOutcome::Cancelled
            }
            res = rx => {
                match res {
                    Ok(decision) => DeferOutcome::from_user(decision, gate_reason.clone()),
                    Err(_) => {
                        // Sender dropped without decide — treat as cancel.
                        DeferOutcome::Cancel
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                warn!(%conv_id, %defer_id, "defer timed out — default-deny");
                DeferOutcome::Timeout
            }
        };

        // Cleanup pending slot if still present (timeout / cancel paths).
        self.pending.remove(&defer_id);
        self.by_conv
            .remove_if(&conv_id, |_, id| id == &defer_id);
        self.publisher.clear_prompt(&conv_id, &defer_id).await;

        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingPublisher {
        prompts: Mutex<Vec<DeferPromptEvent>>,
        clears: Mutex<Vec<(String, String)>>,
    }

    impl RecordingPublisher {
        fn new() -> Self {
            Self {
                prompts: Mutex::new(Vec::new()),
                clears: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl DeferPromptPublisher for RecordingPublisher {
        async fn publish_prompt(&self, prompt: &DeferPromptEvent) {
            self.prompts.lock().unwrap().push(prompt.clone());
        }
        async fn clear_prompt(&self, conv_id: &str, defer_id: &str) {
            self.clears
                .lock()
                .unwrap()
                .push((conv_id.into(), defer_id.into()));
        }
    }

    #[tokio::test]
    async fn decide_allow_unblocks_waiter() {
        let pubr = Arc::new(RecordingPublisher::new());
        let broker = Arc::new(InteractiveDeferBroker::new(
            pubr.clone(),
            Duration::from_secs(30),
        ));
        let broker2 = broker.clone();
        let handle = tokio::spawn(async move {
            broker2
                .await_decision(
                    DeferRequest {
                        conv_id: "c1".into(),
                        tool: "write_file".into(),
                        reason: "review".into(),
                        arguments_preview: "{}".into(),
                        timeout_ms: 30_000,
                    },
                    &CancellationToken::new(),
                )
                .await
        });

        // Spin until the prompt is published.
        let defer_id = loop {
            if let Some(p) = pubr.prompts.lock().unwrap().last() {
                break p.defer_id.clone();
            }
            tokio::task::yield_now().await;
        };

        let result = broker.decide("c1", &defer_id, DeferUserDecision::Allow);
        assert!(result.accepted, "{result:?}");

        let outcome = handle.await.unwrap();
        assert_eq!(outcome, DeferOutcome::Allow);
        assert!(!pubr.clears.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn timeout_default_denies() {
        let broker = Arc::new(InteractiveDeferBroker::new(
            Arc::new(NoopDeferPublisher),
            Duration::from_millis(30),
        ));
        let outcome = broker
            .await_decision(
                DeferRequest {
                    conv_id: "c-timeout".into(),
                    tool: "echo".into(),
                    reason: "r".into(),
                    arguments_preview: "{}".into(),
                    timeout_ms: 30,
                },
                &CancellationToken::new(),
            )
            .await;
        assert_eq!(outcome, DeferOutcome::Timeout);
    }

    #[tokio::test]
    async fn cancel_token_aborts_wait() {
        let broker = Arc::new(InteractiveDeferBroker::with_noop_publisher());
        let cancel = CancellationToken::new();
        let c2 = cancel.clone();
        let handle = tokio::spawn(async move {
            broker
                .await_decision(
                    DeferRequest {
                        conv_id: "c-cancel".into(),
                        tool: "echo".into(),
                        reason: "r".into(),
                        arguments_preview: "{}".into(),
                        timeout_ms: 60_000,
                    },
                    &c2,
                )
                .await
        });
        tokio::task::yield_now().await;
        cancel.cancel();
        assert_eq!(handle.await.unwrap(), DeferOutcome::Cancelled);
    }

    #[test]
    fn decide_missing_id_is_not_accepted() {
        let broker = InteractiveDeferBroker::with_noop_publisher();
        let r = broker.decide("c", "missing", DeferUserDecision::Allow);
        assert!(!r.accepted);
        assert!(r.error.is_some());
    }
}
