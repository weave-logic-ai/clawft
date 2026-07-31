//! Active long-running agent session inventory (WEFT-685 / ADR-073 Phase A).
//!
//! Pure, I/O-free helpers that turn the daemon's `agent.list` wire shape
//! into a substrate-bindable inventory under `substrate/agents/*`.
//! Agent Workspace UI (0.9+) binds these paths; stock 0.8 desktop does
//! not require freeform multi-window.
//!
//! # Substrate paths
//!
//! | Path | Shape | Semantics |
//! |------|-------|-----------|
//! | `substrate/agents/sessions` | array of session rows | full inventory (table / list UIs) |
//! | `substrate/agents/sessions/by-id/{session_id}` | single row | per-session cell (WEFT-416 style) |
//!
//! # Source of truth
//!
//! Kernel supervisor process table via weave RPC `agent.list` (excludes
//! kernel PID 0). Each row is one **long-running** agent process/session
//! suitable for later pane binding — not a dormant chat transcript key.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Topic root for the agent session inventory list.
pub const SESSIONS_ROOT: &str = "substrate/agents/sessions";

/// Per-session path prefix: `{SESSIONS_ROOT}/by-id/{session_id}`.
pub const SESSIONS_BY_ID: &str = "substrate/agents/sessions/by-id";

/// Ontology shape URI for the sessions list topic.
pub const SESSIONS_SHAPE: &str = "ontology://agent-session-list";

/// One active long-running agent session for UI / substrate binding.
///
/// Field names intentionally align with `agent.list` / `ProcessInfo`
/// while adding inventory-specific aliases (`session_id`, `name`,
/// `long_running`, `kind`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionRow {
    /// Stable unique key for list-by-id deltas (equals daemon `row_id`
    /// when present; otherwise derived from pid / agent_id).
    pub session_id: String,
    /// Agent identity string from the kernel process table.
    pub agent_id: String,
    /// Display alias for surfaces that bind `name` (defaults to `agent_id`).
    pub name: String,
    /// Kernel PID (0 is never present — `agent.list` excludes the kernel).
    pub pid: u64,
    /// Process lifecycle state (`running`, `stopped`, …).
    pub state: String,
    /// Resident memory bytes reported by the supervisor.
    pub memory_bytes: u64,
    /// Cumulative CPU time in milliseconds.
    pub cpu_time_ms: u64,
    /// Parent PID when the agent was spawned by another process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_pid: Option<u64>,
    /// Always `true` for rows sourced from the kernel process table.
    /// Reserved so future chat-only sessions can set `false`.
    pub long_running: bool,
    /// Inventory source kind — currently always `"kernel_agent"`.
    pub kind: String,
}

impl AgentSessionRow {
    /// Canonical path for this session under the inventory tree.
    pub fn substrate_path(&self) -> String {
        format!("{SESSIONS_BY_ID}/{}", sanitize_session_id(&self.session_id))
    }
}

/// Sanitize a session id so it is safe as a single path segment.
pub fn sanitize_session_id(id: &str) -> String {
    if id.is_empty() {
        return "_".into();
    }
    if id.contains('/') {
        id.replace('/', "_")
    } else {
        id.to_string()
    }
}

/// Derive a stable `session_id` from an `agent.list` row object.
///
/// Preference order mirrors process-list WEFT-416:
/// 1. explicit `row_id`
/// 2. composite `{pid}:{agent_id}`
/// 3. `pid:{pid}`
/// 4. `agent_id` alone
pub fn session_id_from_agent_list_row(row: &Value) -> Option<String> {
    if let Some(id) = row.get("row_id").and_then(|v| v.as_str())
        && !id.is_empty()
    {
        return Some(sanitize_session_id(id));
    }
    let pid = row.get("pid").and_then(|v| v.as_u64());
    let agent = row.get("agent_id").and_then(|v| v.as_str());
    match (pid, agent) {
        (Some(p), Some(a)) if !a.is_empty() => {
            Some(sanitize_session_id(&format!("{p}:{a}")))
        }
        (Some(p), _) => Some(format!("pid:{p}")),
        (None, Some(a)) if !a.is_empty() => Some(sanitize_session_id(a)),
        _ => None,
    }
}

/// Extract a session_id from an already-projected inventory row.
pub fn agent_session_row_id(row: &Value) -> Option<String> {
    if let Some(id) = row.get("session_id").and_then(|v| v.as_str())
        && !id.is_empty()
    {
        return Some(sanitize_session_id(id));
    }
    session_id_from_agent_list_row(row)
}

/// Parse one `agent.list` row into an [`AgentSessionRow`].
///
/// Returns `None` when the row lacks both a usable identity and pid
/// (cannot form a stable session key).
pub fn parse_agent_list_row(row: &Value) -> Option<AgentSessionRow> {
    let session_id = session_id_from_agent_list_row(row)?;
    let agent_id = row
        .get("agent_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = row
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if agent_id.is_empty() {
                session_id.clone()
            } else {
                agent_id.clone()
            }
        });
    let pid = row.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
    let state = row
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let memory_bytes = row
        .get("memory_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cpu_time_ms = row
        .get("cpu_time_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let parent_pid = row.get("parent_pid").and_then(|v| v.as_u64());

    Some(AgentSessionRow {
        session_id,
        agent_id,
        name,
        pid,
        state,
        memory_bytes,
        cpu_time_ms,
        parent_pid,
        long_running: true,
        kind: "kernel_agent".into(),
    })
}

/// Build inventory rows from an `agent.list` JSON array (or single object).
///
/// Unparseable rows are skipped (not fatal). Non-array / non-object
/// input yields an empty list.
pub fn inventory_from_agent_list(value: &Value) -> Vec<AgentSessionRow> {
    match value {
        Value::Array(arr) => arr.iter().filter_map(parse_agent_list_row).collect(),
        Value::Object(_) => parse_agent_list_row(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// Project `agent.list` into substrate inventory JSON (array of enriched rows).
///
/// Adds `session_id`, `name`, `long_running`, `kind` while preserving
/// raw daemon fields. Non-array inputs are passed through unchanged
/// except for a single object which is wrapped as a one-element array
/// after projection.
pub fn project_agent_session_rows(value: Value) -> Value {
    match value {
        Value::Array(arr) => {
            let projected: Vec<Value> = arr
                .iter()
                .filter_map(|row| {
                    let parsed = parse_agent_list_row(row)?;
                    serde_json::to_value(parsed).ok()
                })
                .collect();
            Value::Array(projected)
        }
        Value::Object(_) => {
            if let Some(row) = parse_agent_list_row(&value) {
                Value::Array(vec![serde_json::to_value(row).unwrap_or(json!({}))])
            } else {
                value
            }
        }
        other => other,
    }
}

/// Expand inventory rows into per-session leaf paths for flat snapshots.
pub fn explode_sessions_by_id(rows: &[AgentSessionRow]) -> Vec<(String, Value)> {
    rows.iter()
        .filter_map(|row| {
            let path = row.substrate_path();
            let value = serde_json::to_value(row).ok()?;
            Some((path, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn inventory_from_fixture_agent_list() {
        let fixture = json!([
            {
                "row_id": "pid:3",
                "pid": 3,
                "agent_id": "researcher",
                "state": "running",
                "memory_bytes": 4096,
                "cpu_time_ms": 1500,
                "parent_pid": 1
            },
            {
                "row_id": "pid:7",
                "pid": 7,
                "agent_id": "coder",
                "state": "running",
                "memory_bytes": 8192,
                "cpu_time_ms": 220,
                "parent_pid": null
            }
        ]);
        let inv = inventory_from_agent_list(&fixture);
        assert_eq!(inv.len(), 2);
        assert_eq!(inv[0].session_id, "pid:3");
        assert_eq!(inv[0].agent_id, "researcher");
        assert_eq!(inv[0].name, "researcher");
        assert!(inv[0].long_running);
        assert_eq!(inv[0].kind, "kernel_agent");
        assert_eq!(inv[0].parent_pid, Some(1));
        assert_eq!(inv[1].session_id, "pid:7");
        assert_eq!(inv[1].agent_id, "coder");
        assert_eq!(inv[1].parent_pid, None);
    }

    #[test]
    fn inventory_empty_array_is_empty() {
        let inv = inventory_from_agent_list(&json!([]));
        assert!(inv.is_empty());
    }

    #[test]
    fn inventory_skips_unkeyed_garbage() {
        let fixture = json!([
            {"state": "running"},
            {"pid": 9, "agent_id": "ok", "state": "running", "memory_bytes": 1, "cpu_time_ms": 1}
        ]);
        let inv = inventory_from_agent_list(&fixture);
        assert_eq!(inv.len(), 1);
        assert_eq!(inv[0].agent_id, "ok");
    }

    #[test]
    fn session_id_prefers_row_id() {
        let row = json!({"row_id": "pid:42", "pid": 42, "agent_id": "x"});
        assert_eq!(session_id_from_agent_list_row(&row).as_deref(), Some("pid:42"));
    }

    #[test]
    fn session_id_falls_back_to_composite() {
        let row = json!({"pid": 5, "agent_id": "mesh-worker"});
        assert_eq!(
            session_id_from_agent_list_row(&row).as_deref(),
            Some("5:mesh-worker")
        );
    }

    #[test]
    fn sanitize_session_id_strips_slashes() {
        assert_eq!(sanitize_session_id("a/b"), "a_b");
        assert_eq!(sanitize_session_id(""), "_");
        assert_eq!(sanitize_session_id("pid:1"), "pid:1");
    }

    #[test]
    fn project_adds_inventory_fields() {
        let input = json!([
            {
                "row_id": "pid:1",
                "pid": 1,
                "agent_id": "boot-agent",
                "state": "running",
                "memory_bytes": 100,
                "cpu_time_ms": 50
            }
        ]);
        let out = project_agent_session_rows(input);
        let row = &out.as_array().unwrap()[0];
        assert_eq!(row["session_id"], "pid:1");
        assert_eq!(row["name"], "boot-agent");
        assert_eq!(row["long_running"], true);
        assert_eq!(row["kind"], "kernel_agent");
        assert_eq!(row["agent_id"], "boot-agent");
        assert_eq!(row["pid"], 1);
    }

    #[test]
    fn project_passes_through_non_object_non_array() {
        let input = json!("not-a-list");
        let out = project_agent_session_rows(input.clone());
        assert_eq!(out, input);
    }

    #[test]
    fn agent_session_row_id_from_projected() {
        let row = json!({
            "session_id": "pid:3",
            "agent_id": "researcher",
            "pid": 3
        });
        assert_eq!(agent_session_row_id(&row).as_deref(), Some("pid:3"));
    }

    #[test]
    fn explode_sessions_emits_by_id_paths() {
        let rows = inventory_from_agent_list(&json!([
            {
                "row_id": "pid:3",
                "pid": 3,
                "agent_id": "researcher",
                "state": "running",
                "memory_bytes": 1,
                "cpu_time_ms": 1
            }
        ]));
        let paths: Vec<String> = explode_sessions_by_id(&rows)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        assert_eq!(paths, vec!["substrate/agents/sessions/by-id/pid:3".to_string()]);
    }

    #[test]
    fn substrate_path_constant_matches_adr_agents_prefix() {
        assert!(SESSIONS_ROOT.starts_with("substrate/agents/"));
        assert!(SESSIONS_BY_ID.starts_with("substrate/agents/"));
        assert_eq!(SESSIONS_SHAPE, "ontology://agent-session-list");
    }
}
