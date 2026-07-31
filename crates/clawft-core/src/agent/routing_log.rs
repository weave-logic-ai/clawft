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

    /// Decode a substrate (or file-dump) JSON value into a record.
    ///
    /// Skips prune tombstones (`{ "_pruned": true }`). Missing optional
    /// fields use defaults so older writers remain readable by
    /// `weft routing trace` / `replay` (WEFT-336).
    pub fn from_substrate_value(value: &serde_json::Value) -> Result<Self, String> {
        if is_routing_tombstone(value) {
            return Err("pruned routing decision tombstone".into());
        }
        // Prefer typed deserialize; fall back to field-by-field for
        // partial/legacy payloads.
        if let Ok(rec) = serde_json::from_value::<RouterDecisionRecord>(value.clone()) {
            return Ok(rec);
        }
        let obj = value
            .as_object()
            .ok_or_else(|| "routing decision value must be a JSON object".to_string())?;
        let decision_id = obj
            .get("decision_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let query = obj
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if decision_id.is_empty() && query.is_empty() {
            return Err("routing decision missing decision_id and query".into());
        }
        let selected_route = obj
            .get("selected_route")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let alternatives = obj
            .get("alternatives")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let confidence = obj.get("confidence").and_then(|v| {
            v.as_f64()
                .map(|f| f as f32)
                .or_else(|| v.as_i64().map(|i| i as f32))
        });
        let latency_ms = obj
            .get("latency_ms")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i.max(0) as u64)))
            .unwrap_or(0);
        let channel = obj
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let chat_id = obj
            .get("chat_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fallback_used = obj
            .get("fallback_used")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let ts = obj
            .get("ts")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(Self {
            decision_id,
            query,
            selected_route,
            alternatives,
            confidence,
            latency_ms,
            channel,
            chat_id,
            fallback_used,
            ts,
        })
    }

    /// Rehydrate a [`ContextRequest`](super::context_router::ContextRequest)
    /// for `weft routing replay`.
    pub fn to_context_request(&self) -> ContextRequest {
        ContextRequest {
            content: self.query.clone(),
            channel: self.channel.clone(),
            chat_id: self.chat_id.clone(),
            metadata: Default::default(),
        }
    }

    /// Skills list from `selected_route` (empty when missing/non-array).
    pub fn selected_skills(&self) -> Vec<String> {
        self.selected_route
            .get("skills")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Complexity hint from `selected_route` (0.0 when missing).
    pub fn selected_complexity_hint(&self) -> f32 {
        self.selected_route
            .get("complexity_hint")
            .and_then(|v| v.as_f64().map(|f| f as f32))
            .unwrap_or(0.0)
    }

    /// Archetype label from `selected_route` when present.
    pub fn selected_archetype(&self) -> Option<String> {
        self.selected_route
            .get("archetype")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
}

/// True when a substrate value is a retention tombstone.
pub fn is_routing_tombstone(value: &serde_json::Value) -> bool {
    value
        .get("_pruned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Extract the decision id from a substrate path under
/// [`ROUTING_RECENT_SUBSTRATE_PREFIX`].
pub fn decision_id_from_path(path: &str) -> Option<String> {
    let prefix = format!("{ROUTING_RECENT_SUBSTRATE_PREFIX}/");
    path.strip_prefix(&prefix)
        .filter(|rest| !rest.is_empty() && !rest.contains('/'))
        .map(str::to_string)
}

/// Sort records newest-first (by `ts` then `decision_id`), optionally
/// filter by conversation / chat id, then keep the first `limit` entries.
///
/// `limit == 0` means unbounded (return all after filter/sort).
pub fn select_recent_records(
    mut records: Vec<RouterDecisionRecord>,
    conversation: Option<&str>,
    limit: usize,
) -> Vec<RouterDecisionRecord> {
    if let Some(conv) = conversation {
        records.retain(|r| r.chat_id == conv);
    }
    records.sort_by(|a, b| {
        // RFC3339 strings sort lexicographically when zero-padded;
        // fall back to decision_id for stable order.
        b.ts.cmp(&a.ts)
            .then_with(|| b.decision_id.cmp(&a.decision_id))
    });
    if limit > 0 && records.len() > limit {
        records.truncate(limit);
    }
    records
}

/// Aggregate latency + fallback metrics over logged decisions
/// (WEFT-336 / promotion-gate surfaces for `weft status`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingLogMetrics {
    /// Number of live (non-tombstone) records considered.
    pub count: usize,
    /// p50 wall-clock route latency in ms (`None` when empty).
    pub p50_latency_ms: Option<u64>,
    /// p99 wall-clock route latency in ms (`None` when empty).
    pub p99_latency_ms: Option<u64>,
    /// Fraction of decisions with `fallback_used == true` in `[0, 1]`.
    pub fallback_rate: f64,
    /// Absolute fallback count.
    pub fallback_count: usize,
}

impl RoutingLogMetrics {
    /// Empty metrics (no decisions yet).
    pub fn empty() -> Self {
        Self {
            count: 0,
            p50_latency_ms: None,
            p99_latency_ms: None,
            fallback_rate: 0.0,
            fallback_count: 0,
        }
    }
}

/// Compute p50 / p99 latency and fallback rate for a set of records.
pub fn compute_metrics(records: &[RouterDecisionRecord]) -> RoutingLogMetrics {
    if records.is_empty() {
        return RoutingLogMetrics::empty();
    }
    let mut latencies: Vec<u64> = records.iter().map(|r| r.latency_ms).collect();
    latencies.sort_unstable();
    let fallback_count = records.iter().filter(|r| r.fallback_used).count();
    let count = records.len();
    RoutingLogMetrics {
        count,
        p50_latency_ms: Some(percentile_nearest_rank(&latencies, 50)),
        p99_latency_ms: Some(percentile_nearest_rank(&latencies, 99)),
        fallback_rate: fallback_count as f64 / count as f64,
        fallback_count,
    }
}

/// Nearest-rank percentile for a **sorted** non-empty slice.
///
/// `pct` is in `1..=100`. For n=1 every percentile returns the sole value.
fn percentile_nearest_rank(sorted: &[u64], pct: u8) -> u64 {
    debug_assert!(!sorted.is_empty());
    let n = sorted.len();
    // rank = ceil(pct/100 * n), 1-indexed → index = rank - 1
    let rank = ((pct as usize) * n).div_ceil(100).max(1);
    sorted[(rank - 1).min(n - 1)]
}

/// Diff a live [`ContextDecision`] against a logged `selected_route`
/// (WEFT-336 replay comparison).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingReplayDiff {
    /// Skills present in the log but not in the live decision.
    pub skills_missing: Vec<String>,
    /// Skills present in the live decision but not in the log.
    pub skills_extra: Vec<String>,
    /// Logged complexity hint.
    pub logged_complexity_hint: f32,
    /// Live complexity hint.
    pub live_complexity_hint: f32,
    /// Absolute difference of complexity hints.
    pub complexity_delta: f32,
    /// Logged fallback flag.
    pub logged_fallback_used: bool,
    /// Live fallback flag.
    pub live_fallback_used: bool,
    /// True when skills match (as sets) and complexity/fallback agree
    /// within a small epsilon.
    pub match_ok: bool,
}

/// Compare a logged record's selected route to a freshly routed decision.
pub fn diff_replay(
    logged: &RouterDecisionRecord,
    live: &ContextDecision,
) -> RoutingReplayDiff {
    let logged_skills = logged.selected_skills();
    let logged_set: std::collections::BTreeSet<&str> =
        logged_skills.iter().map(String::as_str).collect();
    let live_set: std::collections::BTreeSet<&str> =
        live.skills.iter().map(String::as_str).collect();

    let skills_missing: Vec<String> = logged_set
        .difference(&live_set)
        .map(|s| (*s).to_string())
        .collect();
    let skills_extra: Vec<String> = live_set
        .difference(&logged_set)
        .map(|s| (*s).to_string())
        .collect();

    let logged_complexity_hint = logged.selected_complexity_hint();
    let live_complexity_hint = live.complexity_hint;
    let complexity_delta = (logged_complexity_hint - live_complexity_hint).abs();
    let match_ok = skills_missing.is_empty()
        && skills_extra.is_empty()
        && complexity_delta < 1e-4
        && logged.fallback_used == live.fallback_used;

    RoutingReplayDiff {
        skills_missing,
        skills_extra,
        logged_complexity_hint,
        live_complexity_hint,
        complexity_delta,
        logged_fallback_used: logged.fallback_used,
        live_fallback_used: live.fallback_used,
        match_ok,
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

    #[test]
    fn from_substrate_value_round_trips() {
        let mut rec = RouterDecisionRecord::from_route(
            &sample_request(3),
            &ContextDecision::new(vec!["a".into(), "b".into()], None, 0.2),
            15,
            vec!["c".into()],
            Some(0.9),
            false,
        );
        rec.decision_id = "rd-round".into();
        let v = rec.to_substrate_value();
        let decoded = RouterDecisionRecord::from_substrate_value(&v).unwrap();
        assert_eq!(decoded.decision_id, "rd-round");
        assert_eq!(decoded.query, "query-3");
        assert_eq!(decoded.selected_skills(), vec!["a", "b"]);
        assert!((decoded.selected_complexity_hint() - 0.2).abs() < 1e-6);
        assert_eq!(decoded.latency_ms, 15);
        assert_eq!(decoded.confidence, Some(0.9));
    }

    #[test]
    fn from_substrate_value_rejects_tombstone() {
        let v = serde_json::json!({ "_pruned": true });
        assert!(RouterDecisionRecord::from_substrate_value(&v).is_err());
        assert!(is_routing_tombstone(&v));
    }

    #[test]
    fn select_recent_filters_conversation_and_limit() {
        let mut recs = Vec::new();
        for i in 0..5 {
            let mut r = RouterDecisionRecord::from_route(
                &sample_request(i),
                &ContextDecision::default(),
                i as u64,
                vec![],
                None,
                false,
            );
            r.decision_id = format!("id-{i}");
            r.ts = format!("2026-07-30T00:00:0{i}Z");
            r.chat_id = if i % 2 == 0 { "even".into() } else { "odd".into() };
            recs.push(r);
        }
        let filtered = select_recent_records(recs, Some("even"), 2);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.chat_id == "even"));
        // Newest first.
        assert_eq!(filtered[0].decision_id, "id-4");
        assert_eq!(filtered[1].decision_id, "id-2");
    }

    #[test]
    fn compute_metrics_p50_p99_and_fallback_rate() {
        let mut recs = Vec::new();
        // latencies: 1,2,3,4,5,6,7,8,9,100 — one fallback
        for (i, lat) in [1u64, 2, 3, 4, 5, 6, 7, 8, 9, 100].into_iter().enumerate() {
            let mut r = RouterDecisionRecord::from_route(
                &sample_request(i),
                &ContextDecision::default(),
                lat,
                vec![],
                None,
                i == 9,
            );
            r.decision_id = format!("m-{i}");
            recs.push(r);
        }
        let m = compute_metrics(&recs);
        assert_eq!(m.count, 10);
        assert_eq!(m.fallback_count, 1);
        assert!((m.fallback_rate - 0.1).abs() < 1e-9);
        assert_eq!(m.p50_latency_ms, Some(5));
        assert_eq!(m.p99_latency_ms, Some(100));
    }

    #[test]
    fn compute_metrics_empty() {
        let m = compute_metrics(&[]);
        assert_eq!(m, RoutingLogMetrics::empty());
    }

    #[test]
    fn decision_id_from_path_extracts_id() {
        assert_eq!(
            decision_id_from_path("substrate/_derived/agent/routing/recent/01HXABC"),
            Some("01HXABC".into())
        );
        assert_eq!(
            decision_id_from_path("substrate/_derived/agent/routing/recent"),
            None
        );
        assert_eq!(decision_id_from_path("other/path"), None);
    }

    #[test]
    fn diff_replay_detects_skill_mismatch() {
        let mut rec = RouterDecisionRecord::from_route(
            &sample_request(0),
            &ContextDecision::new(vec!["skill-a".into()], None, 0.1),
            1,
            vec![],
            None,
            false,
        );
        rec.decision_id = "diff-1".into();
        let live = ContextDecision::new(vec!["skill-b".into()], None, 0.1);
        let d = diff_replay(&rec, &live);
        assert!(!d.match_ok);
        assert_eq!(d.skills_missing, vec!["skill-a".to_string()]);
        assert_eq!(d.skills_extra, vec!["skill-b".to_string()]);
    }

    #[test]
    fn diff_replay_match_ok_on_identical() {
        let decision = ContextDecision::new(vec!["s".into()], None, 0.0);
        let mut rec =
            RouterDecisionRecord::from_route(&sample_request(0), &decision, 1, vec![], None, false);
        rec.decision_id = "diff-ok".into();
        let d = diff_replay(&rec, &decision);
        assert!(d.match_ok);
        assert!(d.skills_missing.is_empty());
        assert!(d.skills_extra.is_empty());
    }
}
