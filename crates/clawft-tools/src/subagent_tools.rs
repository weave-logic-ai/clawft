//! Agent-initiated work tools (M4 — design D4).
//!
//! Four LLM tools that let the conversational agent kick off work from inside
//! a turn and pull the result back in as first-class ECC state:
//!
//! - [`AgentSpawnTool`] (`agent_spawn`) — spawn a subagent for a goal.
//! - [`TaskStatusTool`] (`task_status`) — poll one task or a swarm group.
//! - [`TaskResultTool`] (`task_result`) — fetch a task's result, optionally
//!   blocking up to `wait_ms` for it to finish.
//! - [`AgentMessageTool`] (`agent_message`) — inject an A2A message into a
//!   running child conversation.
//!
//! Each tool holds an `Arc<dyn SubagentSpawner>` — the injected daemon seam
//! (design D2). The concrete `DaemonSubagentSpawner` lives in
//! `clawft-service-agent`; these tools never name it. They are registered by
//! [`register_all`](crate::register_all) only when a spawner is injected (the
//! daemon path), so the in-process CLI never exposes them.
//!
//! Feature-gated behind `subagent` (implied by `native`); off for browser/WASM
//! builds, which have no daemon to spawn into.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use clawft_core::agent::spawn::{SpawnBudget, SpawnError, SpawnSpec, SubagentSpawner, TaskOutcome};
use clawft_core::agent::spawn_context;
use clawft_core::tools::registry::{Tool, ToolError};
use serde_json::{Value, json};
use tracing::{debug, info};

/// Upper bound on the `goal` string a single `agent_spawn` call may carry
/// (design D4: "goal length bound"). Keeps a runaway prompt from being smuggled
/// through the spawn seam.
const MAX_GOAL_LEN: usize = 8192;

// ── SpawnError → tool-result mapping ────────────────────────────────────────
//
// Governance *refusals* (gate denial, depth cap) surface as a `{denied,
// reason}` value — distinct from a runtime `{error}`, matching how
// `execute_tool_with_guards` reports a gate `Deny` (design D6, failure table).
// Every other `SpawnError` is a runtime failure and maps to a `ToolError`,
// which the guard layer renders as `{error: …}`.
fn spawn_err_to_result(err: SpawnError) -> Result<Value, ToolError> {
    match err {
        SpawnError::Denied { reason } => Ok(json!({ "denied": true, "reason": reason })),
        SpawnError::DepthExceeded { depth, max } => Ok(json!({
            "denied": true,
            "reason": format!("spawn depth {depth} exceeds max {max}"),
        })),
        other => Err(ToolError::ExecutionFailed(other.to_string())),
    }
}

/// Render a [`TaskOutcome`] as the compact `task_status` row shape (design D4).
fn status_row(o: &TaskOutcome) -> Value {
    let mut v = json!({
        "task_id": o.task_id,
        "status": o.status.as_str(),
        "child_conv_id": o.child_conv_id,
        "iterations": o.iterations,
    });
    if let Some(g) = &o.group_id {
        v["group_id"] = json!(g);
    }
    v
}

/// Render a [`TaskOutcome`] as the fuller `task_result` shape (design D4).
fn result_view(o: &TaskOutcome) -> Value {
    let mut v = json!({ "status": o.status.as_str() });
    if let Some(r) = &o.result {
        v["result"] = json!(r);
    }
    if let Some(f) = &o.finish_reason {
        v["finish_reason"] = json!(f);
    }
    if let Some(e) = &o.error {
        v["error"] = json!(e);
    }
    v
}

fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::InvalidArgs(format!("missing required field: {key}")))
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn optional_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

// ── agent_spawn ─────────────────────────────────────────────────────────────

/// Spawn a local subagent for a goal (design D4). The MVP path is
/// `await: true`, which dispatches the child inline and returns its result.
pub struct AgentSpawnTool {
    spawner: Arc<dyn SubagentSpawner>,
}

impl AgentSpawnTool {
    /// Create the tool over the injected spawner seam.
    pub fn new(spawner: Arc<dyn SubagentSpawner>) -> Self {
        Self { spawner }
    }

    /// Build a [`SpawnSpec`] from the LLM's arguments plus the ambient parent
    /// context. The `parent_*` fields and `depth` come from the execution
    /// context ([`spawn_context::current`]), never from the LLM (design D3/D5).
    fn build_spec(&self, args: &Value) -> Result<SpawnSpec, ToolError> {
        let goal = required_str(args, "goal")?;
        if goal.len() > MAX_GOAL_LEN {
            return Err(ToolError::InvalidArgs(format!(
                "goal exceeds {MAX_GOAL_LEN} chars ({} given)",
                goal.len()
            )));
        }

        let mut spec = SpawnSpec::new(goal);
        spec.persona = optional_string(args, "persona");
        spec.skills = args
            .get("skills")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        spec.await_completion = optional_bool(args, "await");
        spec.group_id = optional_string(args, "group_id");
        spec.notify_on_complete = optional_bool(args, "notify_on_complete");
        spec.budget = args
            .get("budget")
            .filter(|b| !b.is_null())
            .map(|b| serde_json::from_value::<SpawnBudget>(b.clone()))
            .transpose()
            .map_err(|e| ToolError::InvalidArgs(format!("invalid budget: {e}")))?;

        // Parent context — from the execution seam, not the model.
        if let Some(ctx) = spawn_context::current() {
            spec.parent_conv_id = ctx.conv_id;
            spec.parent_agent_id = ctx.agent_id;
            spec.parent_turn_uid = ctx.turn_uid;
            spec.depth = ctx.depth.saturating_add(1);
        } else {
            // No loop context (CLI / tests): a root-level spawn at depth 1.
            spec.depth = 1;
        }

        Ok(spec)
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl Tool for AgentSpawnTool {
    fn name(&self) -> &str {
        "agent_spawn"
    }

    fn description(&self) -> &str {
        "Spawn a local WeftOS subagent to accomplish a goal on the shared local \
         model (no API key). Set await=true to run it inline and get its result \
         back in this turn; await=false returns a task_id to poll with \
         task_status / task_result. Use group_id to fan out a swarm."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "goal": { "type": "string", "description": "The task the subagent should accomplish (its first user turn)." },
                "persona": { "type": "string", "description": "Optional persona/identity for the child (validated against known personas)." },
                "skills": { "type": "array", "items": { "type": "string" }, "description": "Optional skills to grant the child." },
                "await": { "type": "boolean", "description": "Run the child inline and return its result now (default false).", "default": false },
                "budget": {
                    "type": "object",
                    "description": "Optional per-child budget overrides.",
                    "properties": {
                        "tokens": { "type": "integer" },
                        "usd": { "type": "number" },
                        "iterations": { "type": "integer" }
                    }
                },
                "group_id": { "type": "string", "description": "Swarm group id; siblings sharing it aggregate under task_status(group_id)." },
                "notify_on_complete": { "type": "boolean", "description": "Opt-in proactive completion notification (flag-gated).", "default": false }
            },
            "required": ["goal"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let spec = self.build_spec(&args)?;
        debug!(
            goal_len = spec.goal.len(),
            await_completion = spec.await_completion,
            depth = spec.depth,
            group_id = ?spec.group_id,
            "agent_spawn invoked"
        );

        match self.spawner.spawn(spec).await {
            Ok(handle) => {
                info!(
                    task_id = %handle.task_id,
                    child = %handle.child_conv_id,
                    status = %handle.status.as_str(),
                    "subagent spawned"
                );
                let mut out = json!({
                    "task_id": handle.task_id,
                    "child_conv_id": handle.child_conv_id,
                    "status": handle.status.as_str(),
                });
                if let Some(o) = handle.outcome {
                    if let Some(r) = o.result {
                        out["result"] = json!(r);
                    }
                    if let Some(f) = o.finish_reason {
                        out["finish_reason"] = json!(f);
                    }
                    if let Some(e) = o.error {
                        out["error"] = json!(e);
                    }
                }
                Ok(out)
            }
            Err(e) => spawn_err_to_result(e),
        }
    }
}

// ── task_status ─────────────────────────────────────────────────────────────

/// Poll one task (`task_id`) or every task in a swarm group (`group_id`).
pub struct TaskStatusTool {
    spawner: Arc<dyn SubagentSpawner>,
}

impl TaskStatusTool {
    /// Create the tool over the injected spawner seam.
    pub fn new(spawner: Arc<dyn SubagentSpawner>) -> Self {
        Self { spawner }
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl Tool for TaskStatusTool {
    fn name(&self) -> &str {
        "task_status"
    }

    fn description(&self) -> &str {
        "Check the status of a spawned subagent task by task_id, or aggregate \
         every task in a swarm by group_id. Exactly one of the two is required."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "A single task to check." },
                "group_id": { "type": "string", "description": "A swarm group to aggregate." }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let task_id = optional_string(&args, "task_id");
        let group_id = optional_string(&args, "group_id");

        let rows: Vec<Value> = match (task_id, group_id) {
            (Some(_), Some(_)) => {
                return Err(ToolError::InvalidArgs(
                    "provide exactly one of task_id or group_id, not both".into(),
                ));
            }
            (Some(tid), None) => self
                .spawner
                .status(&tid)
                .await
                .iter()
                .map(status_row)
                .collect(),
            (None, Some(gid)) => self
                .spawner
                .group_status(&gid)
                .await
                .iter()
                .map(status_row)
                .collect(),
            (None, None) => {
                return Err(ToolError::InvalidArgs(
                    "one of task_id or group_id is required".into(),
                ));
            }
        };

        Ok(json!({ "tasks": rows }))
    }
}

// ── task_result ─────────────────────────────────────────────────────────────

/// Fetch a task's result, optionally blocking up to `wait_ms` for a running
/// task to finish (design D3, poll-primary).
pub struct TaskResultTool {
    spawner: Arc<dyn SubagentSpawner>,
}

impl TaskResultTool {
    /// Create the tool over the injected spawner seam.
    pub fn new(spawner: Arc<dyn SubagentSpawner>) -> Self {
        Self { spawner }
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl Tool for TaskResultTool {
    fn name(&self) -> &str {
        "task_result"
    }

    fn description(&self) -> &str {
        "Fetch a spawned subagent's result by task_id. Optionally pass wait_ms \
         to block up to that many milliseconds for a running task to finish; on \
         timeout the current (running) status is returned."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "The task to fetch." },
                "wait_ms": { "type": "integer", "description": "Max milliseconds to block for completion (default 0)." }
            },
            "required": ["task_id"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let task_id = required_str(&args, "task_id")?;
        let wait = args
            .get("wait_ms")
            .and_then(Value::as_u64)
            .filter(|ms| *ms > 0)
            .map(Duration::from_millis);

        match self.spawner.result(task_id, wait).await {
            Ok(outcome) => Ok(result_view(&outcome)),
            Err(e) => spawn_err_to_result(e),
        }
    }
}

// ── agent_message ───────────────────────────────────────────────────────────

/// Inject an A2A message turn into a running child conversation (design D4/D7).
pub struct AgentMessageTool {
    spawner: Arc<dyn SubagentSpawner>,
}

impl AgentMessageTool {
    /// Create the tool over the injected spawner seam.
    pub fn new(spawner: Arc<dyn SubagentSpawner>) -> Self {
        Self { spawner }
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl Tool for AgentMessageTool {
    fn name(&self) -> &str {
        "agent_message"
    }

    fn description(&self) -> &str {
        "Send a message to a running spawned subagent (agent-to-agent). The \
         message is injected as a turn into the child conversation."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "The target subagent task." },
                "message": { "type": "string", "description": "The message to deliver into the child conversation." }
            },
            "required": ["task_id", "message"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let task_id = required_str(&args, "task_id")?;
        let message = required_str(&args, "message")?;

        match self.spawner.message(task_id, message).await {
            Ok(()) => Ok(json!({ "delivered": true })),
            Err(e) => spawn_err_to_result(e),
        }
    }
}
#[cfg(test)]
#[path = "subagent_tools_tests.rs"]
mod tests;
