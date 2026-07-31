//! Delegation monitoring API routes.
//!
//! Provides endpoints for viewing active delegations, managing delegation
//! rules, and browsing delegation history.
//!
//! ## Live-state status (WEFT-304)
//!
//! `FlowDelegator` was **retired** (WEFT-179): multi-agent orchestration uses
//! MCP servers + skills, not a subprocess Flow target. There is no live
//! FlowDelegator event bus or active-task registry in `clawft-services` that
//! these handlers can subscribe to.
//!
//! What *is* wired:
//! - **Rules (read)**: derived from `state.config` → `delegation.rules` when
//!   the config bridge exposes a `delegation` object (see
//!   [`ConfigBridge::get_config`](super::bridge::ConfigBridge)).
//! - **Active / history**: return empty lists (not fixtures). A future
//!   ClaudeDelegator / A2A task ledger can populate them.
//! - **Rules (write)**: upsert/delete attempt a config round-trip when the
//!   simplified config view includes a full-enough `delegation` payload;
//!   otherwise return a structured error so the UI does not silently drop
//!   edits.
//!
//! Ticket remains open until active + history have a real store and rule
//! writes persist reliably end-to-end.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch},
};
use serde::{Deserialize, Serialize};

use super::ApiState;

/// Build delegation API routes.
pub fn delegation_routes() -> Router<ApiState> {
    Router::new()
        .route("/delegation/active", get(list_active_delegations))
        .route("/delegation/rules", get(list_delegation_rules))
        .route("/delegation/rules", patch(upsert_delegation_rule))
        .route("/delegation/rules/{name}", delete(delete_delegation_rule))
        .route("/delegation/history", get(delegation_history))
}

// ── Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveDelegation {
    pub task_id: String,
    pub session_key: String,
    pub target: String,
    pub status: DelegationStatus,
    pub started_at: String,
    pub latency_ms: Option<u64>,
    pub tool_name: String,
    pub complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DelegationStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRule {
    pub name: String,
    pub pattern: String,
    pub target: String,
    pub complexity_threshold: f64,
    pub enabled: bool,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationHistoryEntry {
    pub task_id: String,
    pub session_key: String,
    pub target: String,
    pub tool_name: String,
    pub status: DelegationStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub latency_ms: Option<u64>,
    pub complexity: f64,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub session: Option<String>,
    pub target: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedHistory {
    pub items: Vec<DelegationHistoryEntry>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
    code: &'static str,
}

// ── Handlers ───────────────────────────────────────────────────

/// List in-flight delegated tasks.
///
/// WEFT-304 / WEFT-179: no FlowDelegator live state — return empty rather
/// than mock fixtures so the dashboard never invents running work.
async fn list_active_delegations(State(_state): State<ApiState>) -> Json<Vec<ActiveDelegation>> {
    // Gap: ClaudeDelegator is request-scoped and does not publish a shared
    // active-task registry to ApiState. Leave empty until that lands.
    Json(Vec::new())
}

/// List routing rules from live config (`delegation.rules`).
async fn list_delegation_rules(State(state): State<ApiState>) -> Json<Vec<DelegationRule>> {
    Json(rules_from_config(&state))
}

/// Upsert a rule into config persistence when possible.
async fn upsert_delegation_rule(
    State(state): State<ApiState>,
    Json(rule): Json<DelegationRule>,
) -> Response {
    match mutate_rules(&state, |rules| {
        if let Some(existing) = rules.iter_mut().find(|r| r.name == rule.name) {
            *existing = rule.clone();
        } else {
            rules.push(rule.clone());
        }
    }) {
        Ok(()) => Json(rule).into_response(),
        Err(msg) => (
            StatusCode::NOT_IMPLEMENTED,
            Json(ApiErrorBody {
                error: msg,
                code: "delegation_rules_persist_unavailable",
            }),
        )
            .into_response(),
    }
}

/// Delete a named rule from config persistence when possible.
async fn delete_delegation_rule(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> Response {
    match mutate_rules(&state, |rules| {
        rules.retain(|r| r.name != name);
    }) {
        Ok(()) => Json(serde_json::json!({ "deleted": name })).into_response(),
        Err(msg) => (
            StatusCode::NOT_IMPLEMENTED,
            Json(ApiErrorBody {
                error: msg,
                code: "delegation_rules_persist_unavailable",
            }),
        )
            .into_response(),
    }
}

/// Historical delegated tasks.
///
/// WEFT-304: no event-sourced delegation store on ApiState — empty page.
async fn delegation_history(
    State(_state): State<ApiState>,
    Query(params): Query<HistoryQuery>,
) -> Json<PaginatedHistory> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    Json(PaginatedHistory {
        items: Vec::new(),
        total: 0,
        limit,
        offset,
    })
}

// ── Config helpers ─────────────────────────────────────────────

/// Pull rules from the simplified config JSON exposed by [`ConfigAccess`].
fn rules_from_config(state: &ApiState) -> Vec<DelegationRule> {
    let cfg = state.config.get_config();
    let Some(rules_val) = cfg
        .get("delegation")
        .and_then(|d| d.get("rules"))
        .cloned()
    else {
        return Vec::new();
    };

    let raw: Vec<serde_json::Value> = match serde_json::from_value(rules_val) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    raw.into_iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let pattern = v.get("pattern")?.as_str()?.to_string();
            let target = match v.get("target") {
                Some(t) if t.is_string() => t.as_str()?.to_string(),
                Some(t) => t.to_string().trim_matches('"').to_string(),
                None => "auto".into(),
            };
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("rule-{}", i + 1));
            let complexity_threshold = v
                .get("complexity_threshold")
                .and_then(|c| c.as_f64())
                .unwrap_or(1.0);
            let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
            let priority = v
                .get("priority")
                .and_then(|p| p.as_u64())
                .unwrap_or(i as u64 + 1) as u32;
            Some(DelegationRule {
                name,
                pattern,
                target,
                complexity_threshold,
                enabled,
                priority,
            })
        })
        .collect()
}

/// Apply a mutation to the rules list and try to persist via `save_config`.
///
/// The simplified config view from `ConfigBridge` is **not** a full
/// `clawft_types::config::Config` document, so `save_config` usually
/// rejects it. We still attempt the path so a future bridge that returns
/// a full document (or a dedicated rules API) lights up without further
/// handler changes. On failure, surface a clear WEFT-304 message.
fn mutate_rules(state: &ApiState, mutate: impl FnOnce(&mut Vec<DelegationRule>)) -> Result<(), String> {
    let mut cfg = state.config.get_config();
    let mut rules = rules_from_config(state);
    mutate(&mut rules);

    let rules_json: Vec<serde_json::Value> = rules
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "pattern": r.pattern,
                "target": r.target,
                "complexity_threshold": r.complexity_threshold,
                "enabled": r.enabled,
                "priority": r.priority,
            })
        })
        .collect();

    // Ensure a delegation object exists, then write rules.
    if !cfg.get("delegation").map(|d| d.is_object()).unwrap_or(false) {
        cfg.as_object_mut()
            .ok_or_else(|| {
                "delegation rules write failed: config root is not an object (WEFT-304)".to_string()
            })?
            .insert("delegation".into(), serde_json::json!({}));
    }
    cfg.get_mut("delegation")
        .and_then(|d| d.as_object_mut())
        .ok_or_else(|| {
            "delegation rules write failed: delegation node is not an object (WEFT-304)".to_string()
        })?
        .insert("rules".into(), serde_json::Value::Array(rules_json));

    state.config.save_config(cfg).map_err(|e| {
        format!(
            "delegation rules persistence unavailable: {e}. \
             FlowDelegator/active-task ledger not wired (WEFT-304 / WEFT-179); \
             rule writes need a full config document or dedicated rules store."
        )
    })
}
