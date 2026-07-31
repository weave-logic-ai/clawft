//! Monitoring API routes.
//!
//! Provides endpoints for token usage tracking, cost breakdowns,
//! pipeline run telemetry, (WEFT-40) admin routing-decision history,
//! and (WEFT-48/49) rate-limiter metrics + LRU flush.

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use clawft_core::pipeline::decision_history::{
    DecisionHistoryFilter, RoutingAggregateStats, RoutingDecisionEntry,
};
use clawft_core::pipeline::rate_limiter::RateLimiterMetrics;
use serde::{Deserialize, Serialize};

use super::ApiState;

/// Build monitoring API routes.
pub fn monitoring_routes() -> Router<ApiState> {
    Router::new()
        .route("/monitoring/token-usage", get(token_usage))
        .route("/monitoring/costs", get(cost_breakdown))
        .route("/monitoring/pipeline-runs", get(pipeline_runs))
        // WEFT-40: admin surface for recent routing decisions.
        .route("/admin/routing/decisions", get(routing_decisions))
        .route("/admin/routing/stats", get(routing_stats))
        // WEFT-48/49: rate-limiter metrics + manual LRU flush.
        .route("/admin/rate-limiter", get(rate_limiter_metrics))
        .route("/admin/rate-limiter/flush", post(rate_limiter_flush))
}

// ── Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub total_input: u64,
    pub total_output: u64,
    pub total_requests: u64,
    pub by_provider: Vec<TokenUsage>,
    pub by_session: Vec<SessionTokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokenUsage {
    pub session_key: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub total_cost_usd: f64,
    pub by_provider: Vec<ProviderCost>,
    pub by_tier: Vec<TierCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCost {
    pub provider: String,
    pub model: String,
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierCost {
    pub tier: u32,
    pub label: String,
    pub request_count: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: String,
    pub session_key: String,
    pub model: String,
    pub complexity: f64,
    pub latency_ms: u64,
    pub status: PipelineRunStatus,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineRunStatus {
    Success,
    Error,
}

// ── Handlers ───────────────────────────────────────────────────
//
// WEFT-305: Prefer live `ApiState.routing_history` (WEFT-40 in-process
// ring) over hardcoded fixtures. Token counts are not yet recorded on
// `RoutingDecisionEntry` (D5/D6 token meter still open) — `token_usage`
// therefore returns request counts with zero tokens rather than fake
// 245k-style totals. Upgrade path: record prompt/completion tokens on
// the ring (or a dedicated metrics store) when the agent loop emits
// usage, then map them here.

async fn token_usage(State(state): State<ApiState>) -> Json<TokenUsageSummary> {
    use std::collections::HashMap;

    let entries = state.routing_history.recent(state.routing_history.capacity());

    // Aggregate request counts by (provider, model). Tokens stay 0 until
    // a real usage recorder lands (D5/D6).
    let mut by_key: HashMap<(String, String), TokenUsage> = HashMap::new();
    let mut by_session_map: HashMap<String, SessionTokenUsage> = HashMap::new();

    for e in entries.iter() {
        let key = (e.provider.clone(), e.model.clone());
        let slot = by_key.entry(key).or_insert_with(|| TokenUsage {
            provider: e.provider.clone(),
            model: e.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            request_count: 0,
        });
        slot.request_count = slot.request_count.saturating_add(1);

        // principal ≈ sender_id (D6 attribution surface when present).
        let session_key = e
            .principal
            .clone()
            .unwrap_or_else(|| "anonymous".into());
        let sess = by_session_map
            .entry(session_key.clone())
            .or_insert_with(|| SessionTokenUsage {
                session_key,
                input_tokens: 0,
                output_tokens: 0,
                request_count: 0,
            });
        sess.request_count = sess.request_count.saturating_add(1);
    }

    let by_provider: Vec<TokenUsage> = by_key.into_values().collect();
    let by_session: Vec<SessionTokenUsage> = by_session_map.into_values().collect();
    let total_requests = by_provider.iter().map(|p| p.request_count).sum();

    Json(TokenUsageSummary {
        total_input: 0,
        total_output: 0,
        total_requests,
        by_provider,
        by_session,
    })
}

async fn cost_breakdown(State(state): State<ApiState>) -> Json<CostBreakdown> {
    use std::collections::HashMap;

    let entries = state.routing_history.recent(state.routing_history.capacity());

    let mut by_key: HashMap<(String, String), ProviderCost> = HashMap::new();
    let mut by_tier_map: HashMap<String, TierCost> = HashMap::new();

    for e in &entries {
        let cost = e.cost_estimate_usd.unwrap_or(0.0);
        let key = (e.provider.clone(), e.model.clone());
        let slot = by_key.entry(key).or_insert_with(|| ProviderCost {
            provider: e.provider.clone(),
            model: e.model.clone(),
            input_cost_usd: 0.0,
            output_cost_usd: 0.0,
            total_cost_usd: 0.0,
        });
        // Routing history stores a single cost estimate (not split by
        // input/output). Attribute the whole amount to total; leave the
        // input/output split at 0 until a usage meter records both.
        slot.total_cost_usd += cost;

        let tier_label = e.tier.clone().unwrap_or_else(|| "(none)".into());
        let tier_num = tier_label_to_number(&tier_label);
        let tier_slot = by_tier_map
            .entry(tier_label.clone())
            .or_insert_with(|| TierCost {
                tier: tier_num,
                label: tier_label,
                request_count: 0,
                total_cost_usd: 0.0,
            });
        tier_slot.request_count = tier_slot.request_count.saturating_add(1);
        tier_slot.total_cost_usd += cost;
    }

    let by_provider: Vec<ProviderCost> = by_key.into_values().collect();
    let mut by_tier: Vec<TierCost> = by_tier_map.into_values().collect();
    by_tier.sort_by_key(|t| t.tier);
    let total_cost_usd = by_provider.iter().map(|p| p.total_cost_usd).sum();

    Json(CostBreakdown {
        total_cost_usd,
        by_provider,
        by_tier,
    })
}

async fn pipeline_runs(State(state): State<ApiState>) -> Json<Vec<PipelineRun>> {
    // Map routing decisions → pipeline run rows. Latency and complexity
    // are not yet on `RoutingDecisionEntry` (D5) — report 0 rather than
    // inventing values. Status is Success; fallback/escalation is still
    // visible via `/admin/routing/decisions`.
    let entries = state.routing_history.recent(200);
    let runs: Vec<PipelineRun> = entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| PipelineRun {
            id: format!("route-{}", entries_id_suffix(&e.ts, i)),
            session_key: e.principal.unwrap_or_else(|| "anonymous".into()),
            model: e.model,
            // D5: per-session latency/complexity not on RoutingDecisionEntry yet.
            complexity: 0.0,
            latency_ms: 0,
            status: PipelineRunStatus::Success,
            timestamp: e.ts,
        })
        .collect();
    Json(runs)
}

/// Stable-ish id fragment from timestamp + index (no extra deps).
fn entries_id_suffix(ts: &str, idx: usize) -> String {
    // Prefer compact alphanumeric; fall back to index alone.
    let compact: String = ts
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(16)
        .collect();
    if compact.is_empty() {
        format!("{idx}")
    } else {
        format!("{compact}-{idx}")
    }
}

/// Best-effort map of tier name → numeric tier for the UI.
fn tier_label_to_number(label: &str) -> u32 {
    match label.to_ascii_lowercase().as_str() {
        "booster" | "agent-booster" | "agent_booster" | "1" => 1,
        "haiku" | "fast" | "2" => 2,
        "sonnet" | "opus" | "standard" | "3" => 3,
        "(none)" | "" => 0,
        _ => 0,
    }
}

// ── WEFT-40: routing decision history ─────────────────────────────────

/// Query params for `GET /api/admin/routing/decisions`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RoutingDecisionsQuery {
    /// Max entries (newest first). Default 100, capped at ring capacity.
    pub limit: Option<usize>,
    /// Filter by principal / sender_id.
    pub user: Option<String>,
    /// Filter by tier name.
    pub tier: Option<String>,
    /// Inclusive lower bound on entry `ts` (RFC3339).
    pub since: Option<String>,
}

/// Response body for `GET /api/admin/routing/decisions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecisionsResponse {
    /// Matching decisions, newest first.
    pub decisions: Vec<RoutingDecisionEntry>,
    /// Aggregate stats over the *entire* ring (not just the filtered page).
    pub stats: RoutingAggregateStats,
    /// Number of entries returned in `decisions`.
    pub returned: usize,
}

/// Default page size when `limit` is omitted.
const DEFAULT_DECISIONS_PAGE: usize = 100;

/// `GET /api/admin/routing/decisions` — last N routing decisions with
/// optional filters (WEFT-40). Auth-gated via the `/api` nest.
///
/// Privacy: entries use redacted reason categories only (no free-text
/// `RoutingDecision.reason`). See `decision_history` module docs.
async fn routing_decisions(
    State(state): State<ApiState>,
    Query(q): Query<RoutingDecisionsQuery>,
) -> Json<RoutingDecisionsResponse> {
    let capacity = state.routing_history.capacity();
    let limit = q
        .limit
        .unwrap_or(DEFAULT_DECISIONS_PAGE)
        .min(if capacity == 0 { DEFAULT_DECISIONS_PAGE } else { capacity })
        .max(1);
    let filter = DecisionHistoryFilter {
        limit: Some(limit),
        principal: q.user,
        tier: q.tier,
        since: q.since,
    };
    let decisions = state.routing_history.query(filter);
    let returned = decisions.len();
    let stats = state.routing_history.aggregate_stats();
    Json(RoutingDecisionsResponse {
        decisions,
        stats,
        returned,
    })
}

/// `GET /api/admin/routing/stats` — aggregate counters only (WEFT-40).
async fn routing_stats(State(state): State<ApiState>) -> Json<RoutingAggregateStats> {
    Json(state.routing_history.aggregate_stats())
}

// ── WEFT-48/49: rate-limiter admin surface ───────────────────────────────

/// Response body for `POST /api/admin/rate-limiter/flush`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterFlushResponse {
    /// Always true on success.
    pub flushed: bool,
    /// Metrics after the flush (should show zero tracked senders / global count).
    pub metrics: RateLimiterMetrics,
}

/// `GET /api/admin/rate-limiter` — live rate-limiter metrics (WEFT-48).
///
/// Auth-gated via the `/api` nest (admin Bearer token). Returns window
/// size, global cap/count, tracked sender count, and utilization.
async fn rate_limiter_metrics(State(state): State<ApiState>) -> Json<RateLimiterMetrics> {
    Json(state.rate_limiter.metrics())
}

/// `POST /api/admin/rate-limiter/flush` — manual LRU map flush (WEFT-49).
///
/// Clears all tracked senders and resets the global counter. Auth-gated
/// via the `/api` nest (admin Bearer token).
async fn rate_limiter_flush(State(state): State<ApiState>) -> Json<RateLimiterFlushResponse> {
    state.rate_limiter.flush_lru();
    Json(RateLimiterFlushResponse {
        flushed: true,
        metrics: state.rate_limiter.metrics(),
    })
}
