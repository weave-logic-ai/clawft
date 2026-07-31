//! `AgentChatSystemService` — kernel [`SystemService`] adapter for
//! `agent.chat` (WEFT-333).
//!
//! Why this exists
//! ===============
//!
//! Before this module the daemon's [`crate::AgentService`] ran as a
//! private `OnceLock` handle with no registry row. Operators could not
//! see last-completion time, in-flight dispatch count, or per-conv
//! lock contention via `kernel.services` / `weft status`.
//!
//! What this gives you
//! ===================
//!
//! - **Visibility**: a row in `kernel.services` (name = `"agent.chat"`,
//!   `ServiceType::Custom("agent.chat")`) so the desktop Services panel
//!   and `weft kernel services` both see it.
//! - **Metrics** (shared with [`crate::AgentService`] via
//!   [`AgentChatMetrics`]):
//!   - `last_completion_unix_ms` — wall-clock ms of the most recent
//!     finished dispatch (0 = never completed a turn)
//!   - `in_flight` — concurrent dispatches currently inside
//!     `handle_turn`
//!   - `lock_contentions` — times a dispatch had to wait on a per-conv
//!     mutex (another turn held the lock)
//! - **Lifecycle**: `service.start` / `service.stop` flip the same
//!   control flag the daemon registers as
//!   `(ControlKind::Service, "agent")`.
//! - **Health**: disabled → `Degraded("disabled")`; shutting-down →
//!   `Unhealthy(...)`; otherwise `Healthy`. Operator-facing metrics
//!   live on [`SystemService::health_detail`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use clawft_kernel::{HealthStatus, ServiceType, SystemService};

/// Stable registry name. Matches the `agent.chat` JSON-RPC method so
/// operators and `weft status` key on the same surface.
pub const AGENT_CHAT_SERVICE_NAME: &str = "agent.chat";

/// Methods this service contract exposes (governance chain audit).
pub const AGENT_CHAT_CONTRACT_METHODS: &[&str] = &[
    "agent.chat",
    "agent.chat_stream",
    "agent.chat.cancel",
    "agent.chat.end",
    "agent.chat.defer_decide",
    "agent.chat.reset_budget",
];

/// Shared runtime metrics for `agent.chat`.
///
/// Owned by [`crate::AgentService`] (writes on dispatch) and cloned
/// into [`AgentChatSystemService`] (reads on health / status). All
/// fields are atomics so probes never take the per-conv locks.
#[derive(Debug, Default)]
pub struct AgentChatMetrics {
    /// Unix epoch milliseconds of the most recent finished dispatch.
    /// `0` means no dispatch has completed yet.
    last_completion_unix_ms: AtomicU64,
    /// Concurrent dispatches currently inside `handle_turn`.
    /// Shared with [`crate::AgentService`]'s in-flight counter when
    /// constructed via [`Self::with_in_flight`].
    in_flight: Arc<AtomicUsize>,
    /// Times a dispatch observed a contended per-conv mutex (had to
    /// `await` instead of `try_lock`).
    lock_contentions: AtomicU64,
    /// Cumulative milliseconds spent waiting on per-conv locks.
    lock_wait_ms_total: AtomicU64,
}

impl AgentChatMetrics {
    /// Fresh metrics with a private in-flight counter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Share an existing in-flight counter (the one
    /// [`crate::AgentService`] already maintains for shutdown drain).
    pub fn with_in_flight(in_flight: Arc<AtomicUsize>) -> Self {
        Self {
            last_completion_unix_ms: AtomicU64::new(0),
            in_flight,
            lock_contentions: AtomicU64::new(0),
            lock_wait_ms_total: AtomicU64::new(0),
        }
    }

    /// Record that a dispatch finished (success, cancel, or loop error
    /// after the in-flight guard was taken).
    pub fn record_completion(&self) {
        let ms = system_time_unix_ms();
        self.last_completion_unix_ms.store(ms, Ordering::Release);
    }

    /// Record one contended per-conv lock acquisition and its wait.
    pub fn record_lock_contention(&self, wait: std::time::Duration) {
        self.lock_contentions.fetch_add(1, Ordering::Relaxed);
        let ms = wait.as_millis() as u64;
        if ms > 0 {
            self.lock_wait_ms_total.fetch_add(ms, Ordering::Relaxed);
        }
    }

    /// Point-in-time snapshot for health probes and `weft status`.
    pub fn snapshot(&self) -> AgentChatStatusSnapshot {
        AgentChatStatusSnapshot {
            last_completion_unix_ms: self.last_completion_unix_ms.load(Ordering::Acquire),
            in_flight: self.in_flight.load(Ordering::Acquire),
            lock_contentions: self.lock_contentions.load(Ordering::Relaxed),
            lock_wait_ms_total: self.lock_wait_ms_total.load(Ordering::Relaxed),
        }
    }

    /// Shared in-flight counter handle (for tests / AgentService wiring).
    pub fn in_flight_handle(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.in_flight)
    }
}

/// Immutable view of [`AgentChatMetrics`] for display / tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentChatStatusSnapshot {
    /// Unix ms of last finished dispatch; `0` if never.
    pub last_completion_unix_ms: u64,
    /// Concurrent in-flight dispatches.
    pub in_flight: usize,
    /// Lifetime lock-contention count.
    pub lock_contentions: u64,
    /// Lifetime lock-wait milliseconds.
    pub lock_wait_ms_total: u64,
}

impl AgentChatStatusSnapshot {
    /// Compact single-line form for `health_detail` / CLI.
    ///
    /// Example:
    /// `last_completion=never in_flight=0 lock_contentions=0 lock_wait_ms=0`
    pub fn format_detail(&self) -> String {
        let last = if self.last_completion_unix_ms == 0 {
            "never".to_string()
        } else {
            format_unix_ms(self.last_completion_unix_ms)
        };
        format!(
            "last_completion={last} in_flight={} lock_contentions={} lock_wait_ms={}",
            self.in_flight, self.lock_contentions, self.lock_wait_ms_total
        )
    }
}

/// SystemService adapter wrapping shared [`AgentChatMetrics`].
///
/// Constructed at AgentService init in the daemon; registered into
/// `k.services()` so it appears in the Services panel and
/// `weft status`.
pub struct AgentChatSystemService {
    metrics: Arc<AgentChatMetrics>,
    /// Mirror of the daemon control flag
    /// `(ControlKind::Service, "agent")`.
    enabled: Arc<AtomicBool>,
    /// Shared with [`crate::AgentService`]'s shutting-down flag so
    /// health reflects drain state without downcasting.
    shutting_down: Arc<AtomicBool>,
}

impl AgentChatSystemService {
    /// Build an adapter from shared metrics + control flags.
    pub fn new(
        metrics: Arc<AgentChatMetrics>,
        enabled: Arc<AtomicBool>,
        shutting_down: Arc<AtomicBool>,
    ) -> Self {
        Self {
            metrics,
            enabled,
            shutting_down,
        }
    }

    /// Shared metrics handle.
    pub fn metrics(&self) -> &Arc<AgentChatMetrics> {
        &self.metrics
    }
}

#[async_trait]
impl SystemService for AgentChatSystemService {
    fn name(&self) -> &str {
        AGENT_CHAT_SERVICE_NAME
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Custom("agent.chat".into())
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.enabled.store(true, Ordering::SeqCst);
        tracing::info!("agent.chat service enabled (service.start)");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.enabled.store(false, Ordering::SeqCst);
        tracing::info!("agent.chat service disabled (service.stop)");
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if !self.enabled.load(Ordering::SeqCst) {
            return HealthStatus::Degraded("disabled".into());
        }
        if self.shutting_down.load(Ordering::Acquire) {
            let detail = self.metrics.snapshot().format_detail();
            return HealthStatus::Unhealthy(format!("shutting down; {detail}"));
        }
        HealthStatus::Healthy
    }

    fn health_detail(&self) -> Option<String> {
        Some(self.metrics.snapshot().format_detail())
    }
}

fn system_time_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn format_unix_ms(ms: u64) -> String {
    // Prefer RFC3339 via chrono when available; fall back to raw ms.
    use chrono::{TimeZone, Utc};
    match Utc.timestamp_millis_opt(ms as i64) {
        chrono::LocalResult::Single(dt) => dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        _ => ms.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn name_and_type_are_stable() {
        let metrics = Arc::new(AgentChatMetrics::new());
        let svc = AgentChatSystemService::new(
            metrics,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(svc.name(), "agent.chat");
        assert_eq!(
            svc.service_type(),
            ServiceType::Custom("agent.chat".into())
        );
    }

    #[test]
    fn snapshot_defaults_are_zero() {
        let m = AgentChatMetrics::new();
        let s = m.snapshot();
        assert_eq!(s.last_completion_unix_ms, 0);
        assert_eq!(s.in_flight, 0);
        assert_eq!(s.lock_contentions, 0);
        assert_eq!(s.lock_wait_ms_total, 0);
        assert!(s.format_detail().contains("last_completion=never"));
        assert!(s.format_detail().contains("in_flight=0"));
        assert!(s.format_detail().contains("lock_contentions=0"));
    }

    #[test]
    fn record_completion_updates_timestamp() {
        let m = AgentChatMetrics::new();
        assert_eq!(m.snapshot().last_completion_unix_ms, 0);
        m.record_completion();
        let ms = m.snapshot().last_completion_unix_ms;
        assert!(ms > 0, "expected non-zero completion timestamp, got {ms}");
        assert!(!m.snapshot().format_detail().contains("never"));
    }

    #[test]
    fn record_lock_contention_increments() {
        let m = AgentChatMetrics::new();
        m.record_lock_contention(Duration::from_millis(12));
        m.record_lock_contention(Duration::from_millis(3));
        let s = m.snapshot();
        assert_eq!(s.lock_contentions, 2);
        assert_eq!(s.lock_wait_ms_total, 15);
    }

    #[test]
    fn with_in_flight_shares_counter() {
        let counter = Arc::new(AtomicUsize::new(2));
        let m = AgentChatMetrics::with_in_flight(Arc::clone(&counter));
        assert_eq!(m.snapshot().in_flight, 2);
        counter.fetch_add(1, Ordering::SeqCst);
        assert_eq!(m.snapshot().in_flight, 3);
    }

    #[tokio::test]
    async fn start_flips_flag_on() {
        let flag = Arc::new(AtomicBool::new(false));
        let svc = AgentChatSystemService::new(
            Arc::new(AgentChatMetrics::new()),
            Arc::clone(&flag),
            Arc::new(AtomicBool::new(false)),
        );
        svc.start().await.unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn stop_flips_flag_off() {
        let flag = Arc::new(AtomicBool::new(true));
        let svc = AgentChatSystemService::new(
            Arc::new(AgentChatMetrics::new()),
            Arc::clone(&flag),
            Arc::new(AtomicBool::new(false)),
        );
        svc.stop().await.unwrap();
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn health_when_disabled_is_degraded() {
        let svc = AgentChatSystemService::new(
            Arc::new(AgentChatMetrics::new()),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        );
        match svc.health_check().await {
            HealthStatus::Degraded(msg) => assert!(msg.contains("disabled")),
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn health_when_shutting_down_is_unhealthy() {
        let svc = AgentChatSystemService::new(
            Arc::new(AgentChatMetrics::new()),
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(true)),
        );
        match svc.health_check().await {
            HealthStatus::Unhealthy(msg) => {
                assert!(msg.contains("shutting down"));
                assert!(msg.contains("in_flight="));
            }
            other => panic!("expected Unhealthy, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn health_when_enabled_is_healthy() {
        let svc = AgentChatSystemService::new(
            Arc::new(AgentChatMetrics::new()),
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(false)),
        );
        assert!(matches!(svc.health_check().await, HealthStatus::Healthy));
    }

    #[test]
    fn health_detail_surfaces_metrics() {
        let metrics = Arc::new(AgentChatMetrics::new());
        metrics.record_lock_contention(Duration::from_millis(5));
        let svc = AgentChatSystemService::new(
            metrics,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(false)),
        );
        let detail = svc.health_detail().expect("detail present");
        assert!(detail.contains("lock_contentions=1"));
        assert!(detail.contains("lock_wait_ms=5"));
        assert!(detail.contains("in_flight=0"));
    }

    #[test]
    fn contract_methods_include_agent_chat() {
        assert!(AGENT_CHAT_CONTRACT_METHODS.contains(&"agent.chat"));
        assert!(AGENT_CHAT_CONTRACT_METHODS.contains(&"agent.chat.cancel"));
    }
}
