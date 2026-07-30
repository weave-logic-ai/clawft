//! Router decision observability log (WEFT-335 / agent-core-v1.1).
//!
//! ## Why this exists
//!
//! Every [`ContextRouter::route`](super::context_router::ContextRouter::route)
//! call must append a durable decision record so the v1→v2 promotion
//! gate ("≥ 1,000 logged decisions") and the v2→v2.5 gate ("fallback
//! rate < 25% over 7 days") have real data. Without this path, router-
//! phase metrics stay best-effort (see
//! `.planning/reviews/0.7.0-release-gate/11-agent-core-v1.md` Router-
//! phasing task #13 and `docs/plans/chat-agent-v1.md:682`).
//!
//! ## Substrate path
//!
//! Mesh-canonical (grant-gated under the `agent` `DerivedWriteGrant`):
//!
//! ```text
//! substrate/_derived/agent/routing/recent/<ulid>
//! ```
//!
//! Ticket prose says `substrate/<node>/agent/routing/recent/<ulid>`;
//! production mesh writes use the `_derived/` tier (same migration as
//! soul-journal / chat turns). Downstream `weft routing trace` /
//! `replay` (WEFT-336) read this prefix.
//!
//! ## Retention
//!
//! Bounded to [`DEFAULT_ROUTING_LOG_RETENTION`] entries (last-N ring).
//! Implementations drop / tombstone the oldest when the cap is exceeded.
//! The live-observability window is "last 100" per chat-agent-v1; the
//! default cap is higher so the ≥1,000-decision promotion gate can
//! accumulate without pruning training labels.
//!
//! ## Chat-path safety
//!
//! Append failures are **non-fatal**. The agent loop logs a warn and
//! continues — a substrate hiccup must never abort `agent.chat`
//! (degrade, don't crash).

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::context_router::{ContextDecision, ContextRequest};

/// Mesh-canonical substrate prefix for router decision records
/// (no trailing slash). Topic segment under `_derived/` is `agent`;
/// the remainder is `routing/recent/<ulid>`.
pub const ROUTING_RECENT_SUBSTRATE_PREFIX: &str =
    "substrate/_derived/agent/routing/recent";

/// Default last-N retention for decision records.
///
/// Sized so the v1→v2 gate (≥ 1,000 decisions) can accumulate without
/// immediate pruning, while still bounding unbounded growth. Live
/// dashboards can still present the most-recent 100.
pub const DEFAULT_ROUTING_LOG_RETENTION: usize = 10_000;

/// One logged [`ContextRouter`](super::context_router::ContextRouter)
/// decision.
///
/// Wire shape at
/// `substrate/_derived/agent/routing/recent/<decision_id>`:
/// ```json
/// {
///   "decision_id": "<ulid>",
///   "query": "...",
///   "selected_route": { "skills": [], "archetype": null, ... },
///   "alternatives": [],
///   "confidence": 0.9,
///   "latency_ms": 12,
///   "channel": "panel",
///   "chat_id": "...",
///   "fallback_used": false,
///   "ts": "2026-07-30T..."
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterDecisionRecord {
    /// Stable entry id (ULID preferred). Empty string means "writer
    /// should mint one".
    pub decision_id: String,
    /// User turn text that was routed (the query).
    pub query: String,
    /// Selected route payload (skills, tool_subset, complexity_hint,
    /// archetype). Structured so `weft routing replay` can rehydrate.
    pub selected_route: serde_json::Value,
    /// Alternatives considered but not selected (skill ids / labels).
    /// Empty when the router does not surface candidates.
    pub alternatives: Vec<String>,
    /// Confidence in `[0, 1]` when the router reports one; `None`
    /// for v0 [`NullRouter`](super::context_router::NullRouter) /
    /// routers that omit it.
    pub confidence: Option<f32>,
    /// Wall-clock latency of the `route` call in milliseconds.
    pub latency_ms: u64,
    /// Channel the message arrived on.
    pub channel: String,
    /// Conversation / chat identifier.
    pub chat_id: String,
    /// True when a hybrid/fallback arm produced the final decision.
    pub fallback_used: bool,
    /// ISO-8601 (UTC) wall-clock. Empty → writer stamps now.
    pub ts: String,
}

impl RouterDecisionRecord {
    /// Build a record from a request + decision + measured latency.
    ///
    /// `confidence` / `alternatives` / `fallback_used` are optional
    /// enrichments the call site may know (HybridRouter fallback,
    /// EmbeddingRouter top-k). When unknown, pass defaults.
    pub fn from_route(
        request: &ContextRequest,
        decision: &ContextDecision,
        latency_ms: u64,
        alternatives: Vec<String>,
        confidence: Option<f32>,
        fallback_used: bool,
    ) -> Self {
        let selected_route = serde_json::json!({
            "skills": decision.skills,
            "tool_subset": decision.tool_subset,
            "complexity_hint": decision.complexity_hint,
            "archetype": decision.archetype,
        });
        Self {
            decision_id: String::new(),
            query: request.content.clone(),
            selected_route,
            alternatives,
            confidence,
            latency_ms,
            channel: request.channel.clone(),
            chat_id: request.chat_id.clone(),
            fallback_used,
            ts: String::new(),
        }
    }

    /// Ensure `ts` / `decision_id` are populated.
    pub fn finalized(&self) -> Self {
        let mut out = self.clone();
        if out.ts.is_empty() {
            out.ts = Utc::now().to_rfc3339();
        }
        if out.decision_id.is_empty() {
            // Compact, filesystem/substrate-safe id without pulling
            // the `ulid` crate into clawft-core. Substrate writers may
            // replace this with a true ULID before publish.
            let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
            out.decision_id = format!("rd-{nanos:x}");
        }
        out
    }

    /// Substrate JSON value (value side of the path).
    pub fn to_substrate_value(&self) -> serde_json::Value {
        let o = self.finalized();
        serde_json::json!({
            "decision_id": o.decision_id,
            "query": o.query,
            "selected_route": o.selected_route,
            "alternatives": o.alternatives,
            "confidence": o.confidence,
            "latency_ms": o.latency_ms,
            "channel": o.channel,
            "chat_id": o.chat_id,
            "fallback_used": o.fallback_used,
            "ts": o.ts,
        })
    }

    /// Substrate path for this record's id under the mesh-canonical prefix.
    pub fn substrate_path(&self) -> String {
        let id = if self.decision_id.is_empty() {
            self.finalized().decision_id
        } else {
            self.decision_id.clone()
        };
        format!("{ROUTING_RECENT_SUBSTRATE_PREFIX}/{id}")
    }
}

/// Append-only router decision log used by
/// [`super::loop_core::AgentLoop`].
///
/// Implementations:
/// - [`InMemoryRouterDecisionLog`] — tests (bounded ring).
/// - `SubstrateRouterDecisionLog` (service-agent) — grant-gated
///   publish under [`ROUTING_RECENT_SUBSTRATE_PREFIX`].
///
/// Errors return `String` (same pattern as
/// [`super::sink::ConversationSink`]); the loop logs and continues so
/// a log failure never aborts chat.
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
pub trait RouterDecisionLog: Send + Sync + 'static {
    /// Append one decision record. Must be effectively append-only
    /// (aside from bounded retention pruning of the oldest entries).
    async fn append(&self, record: RouterDecisionRecord) -> Result<(), String>;
}

/// HashMap/Vec-backed [`RouterDecisionLog`] for unit tests.
///
/// Enforces [`DEFAULT_ROUTING_LOG_RETENTION`] (or a custom cap) by
/// dropping the oldest entries when the ring is full.
#[derive(Debug)]
pub struct InMemoryRouterDecisionLog {
    entries: Mutex<VecDeque<RouterDecisionRecord>>,
    retention: usize,
}

impl InMemoryRouterDecisionLog {
    /// Empty log with the default retention cap.
    pub fn new() -> Self {
        Self::with_retention(DEFAULT_ROUTING_LOG_RETENTION)
    }

    /// Empty log with an explicit retention cap (`0` means unbounded).
    pub fn with_retention(retention: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            retention,
        }
    }

    /// Snapshot of records recorded so far (finalized copies, oldest first).
    pub fn entries(&self) -> Vec<RouterDecisionRecord> {
        self.entries
            .lock()
            .map(|g| g.iter().map(|e| e.finalized()).collect())
            .unwrap_or_default()
    }

    /// Number of retained records.
    pub fn len(&self) -> usize {
        self.entries.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for InMemoryRouterDecisionLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl RouterDecisionLog for InMemoryRouterDecisionLog {
    async fn append(&self, record: RouterDecisionRecord) -> Result<(), String> {
        let finalized = record.finalized();
        let mut guard = self
            .entries
            .lock()
            .map_err(|e| format!("routing log mutex poisoned: {e}"))?;
        guard.push_back(finalized);
        if self.retention > 0 {
            while guard.len() > self.retention {
                guard.pop_front();
            }
        }
        debug!(
            count = guard.len(),
            retention = self.retention,
            "routing log: in-memory decision appended"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::context_router::ContextDecision;

    fn sample_request(n: usize) -> ContextRequest {
        ContextRequest {
            content: format!("query-{n}"),
            channel: "panel".into(),
            chat_id: format!("c{n}"),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn append_records_query_route_latency() {
        let log = InMemoryRouterDecisionLog::new();
        let req = sample_request(0);
        let decision = ContextDecision::new(vec!["skill-a".into()], None, 0.1);
        let rec = RouterDecisionRecord::from_route(&req, &decision, 7, vec!["skill-b".into()], Some(0.85), false);
        log.append(rec).await.unwrap();

        let entries = log.entries();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.query, "query-0");
        assert_eq!(e.latency_ms, 7);
        assert_eq!(e.confidence, Some(0.85));
        assert_eq!(e.alternatives, vec!["skill-b".to_string()]);
        assert!(!e.decision_id.is_empty());
        assert!(!e.ts.is_empty());
        assert_eq!(e.selected_route["skills"][0], "skill-a");
        assert!((e.selected_route["complexity_hint"].as_f64().unwrap() - 0.1).abs() < 1e-6);
    }

    #[tokio::test]
    async fn one_hundred_routes_produce_one_hundred_entries() {
        let log = InMemoryRouterDecisionLog::with_retention(1_000);
        for i in 0..100 {
            let req = sample_request(i);
            let decision = ContextDecision::default();
            let rec = RouterDecisionRecord::from_route(&req, &decision, i as u64, vec![], None, false);
            log.append(rec).await.unwrap();
        }
        assert_eq!(log.len(), 100, "100 routes must produce 100 entries");
        let entries = log.entries();
        assert_eq!(entries[0].query, "query-0");
        assert_eq!(entries[99].query, "query-99");
    }

    #[tokio::test]
    async fn retention_drops_oldest() {
        let log = InMemoryRouterDecisionLog::with_retention(3);
        for i in 0..5 {
            let req = sample_request(i);
            let rec = RouterDecisionRecord::from_route(
                &req,
                &ContextDecision::default(),
                0,
                vec![],
                None,
                false,
            );
            log.append(rec).await.unwrap();
        }
        assert_eq!(log.len(), 3);
        let queries: Vec<_> = log.entries().into_iter().map(|e| e.query).collect();
        assert_eq!(queries, vec!["query-2", "query-3", "query-4"]);
    }

    #[test]
    fn substrate_path_uses_mesh_canonical_prefix() {
        let mut rec = RouterDecisionRecord::from_route(
            &sample_request(1),
            &ContextDecision::default(),
            1,
            vec![],
            None,
            false,
        );
        rec.decision_id = "01HXTESTULID00000000000000".into();
        assert_eq!(
            rec.substrate_path(),
            "substrate/_derived/agent/routing/recent/01HXTESTULID00000000000000"
        );
    }

    #[test]
    fn substrate_value_round_trips_required_fields() {
        let mut rec = RouterDecisionRecord::from_route(
            &sample_request(2),
            &ContextDecision::new(vec!["x".into()], Some(vec!["t".into()]), 0.0),
            42,
            vec!["alt".into()],
            Some(0.5),
            true,
        );
        rec.decision_id = "rd-test".into();
        let v = rec.to_substrate_value();
        assert_eq!(v["decision_id"], "rd-test");
        assert_eq!(v["query"], "query-2");
        assert_eq!(v["latency_ms"], 42);
        assert_eq!(v["confidence"], 0.5);
        assert_eq!(v["fallback_used"], true);
        assert_eq!(v["alternatives"][0], "alt");
        assert_eq!(v["selected_route"]["skills"][0], "x");
        assert!(!v["ts"].as_str().unwrap_or("").is_empty());
    }
}
