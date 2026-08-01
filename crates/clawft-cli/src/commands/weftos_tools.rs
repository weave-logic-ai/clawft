//! `weftos` MCP tool provider — bridges tool calls to the kernel daemon.
//!
//! Exposes a curated, voice-agent-friendly tool surface (jobs, schedules,
//! sensors, kernel status) over the daemon RPC socket. Registered into
//! the gateway's `/mcp` composite provider so a remote MCP client — the
//! xAI Grok voice agent in particular — can submit jobs, manage jobs,
//! ask about jobs, and read sensors on a running WeftOS node.
//!
//! Every call opens a fresh [`DaemonClient`] connection to the kernel
//! socket (`.weftos/runtime/kernel.sock`), which is cheap on a local
//! UDS and keeps this provider stateless. When no daemon is running
//! (or on non-Unix hosts, where the daemon transport is unimplemented)
//! calls return an `is_error` tool result with a human-readable
//! explanation rather than failing the MCP request — the voice agent
//! can relay that to the user.

use async_trait::async_trait;
use serde_json::{Value, json};

use clawft_rpc::{DaemonClient, Request};
use clawft_services::mcp::ToolDefinition;
use clawft_services::mcp::provider::{CallToolResult, ToolError, ToolProvider};

/// Default per-call timeout. `run_task` runs a full tool-using agent
/// loop and gets a longer default (see [`RUN_TASK_TIMEOUT_SECS`]).
const CALL_TIMEOUT_SECS: u64 = 15;
/// `agent.chat` drives an LLM agent loop; allow it time to think.
const RUN_TASK_TIMEOUT_SECS: u64 = 120;

/// Actor id attached to substrate reads when the caller does not
/// supply one; shows up in substrate ACL / audit trails.
const DEFAULT_ACTOR_ID: &str = "voice-agent";

/// MCP tool provider that forwards to the kernel daemon RPC.
pub struct DaemonToolProvider;

#[async_trait]
impl ToolProvider for DaemonToolProvider {
    fn namespace(&self) -> &str {
        "weftos"
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        tool_definitions()
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        let (method, params, timeout_secs) = match build_request(name, &args) {
            Ok(t) => t,
            Err(ToolError::NotFound(n)) => return Err(ToolError::NotFound(n)),
            Err(e) => return Ok(CallToolResult::error(e.to_string())),
        };

        let Some(mut client) = DaemonClient::connect().await else {
            return Ok(CallToolResult::error(
                "The WeftOS kernel daemon is not running (no socket at the runtime \
                 path). Start it with `weaver up` on the node and try again.",
            ));
        };

        let call = client.call(Request::with_params(&method, params));
        let response = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            call,
        )
        .await
        {
            Err(_) => {
                return Ok(CallToolResult::error(format!(
                    "daemon call `{method}` timed out after {timeout_secs}s"
                )));
            }
            Ok(Err(e)) => {
                return Ok(CallToolResult::error(format!(
                    "daemon call `{method}` failed: {e}"
                )));
            }
            Ok(Ok(r)) => r,
        };

        if response.ok {
            let result = response.result.unwrap_or(Value::Null);
            Ok(CallToolResult::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
            ))
        } else {
            Ok(CallToolResult::error(
                response
                    .error
                    .unwrap_or_else(|| format!("daemon call `{method}` failed")),
            ))
        }
    }
}

/// Map a tool name + JSON args to a daemon RPC (method, params, timeout).
///
/// Param shapes mirror `clawft-weave::protocol` (`AgentSpawnParams`,
/// `CronAddParams`, `SubstrateListParams`, …) — constructed as JSON here
/// so this crate does not need a dependency on the daemon crate.
fn build_request(name: &str, args: &Value) -> Result<(String, Value, u64), ToolError> {
    let str_arg = |key: &str| -> Option<String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let u64_arg = |key: &str| -> Option<u64> { args.get(key).and_then(|v| v.as_u64()) };
    let require_str = |key: &str| -> Result<String, ToolError> {
        str_arg(key).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("missing required string argument `{key}`"))
        })
    };
    let require_u64 = |key: &str| -> Result<u64, ToolError> {
        u64_arg(key).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("missing required integer argument `{key}`"))
        })
    };

    let t = match name {
        "kernel_status" => ("kernel.status".into(), Value::Null, CALL_TIMEOUT_SECS),
        "list_services" => ("kernel.services".into(), Value::Null, CALL_TIMEOUT_SECS),
        "run_task" => {
            let instruction = require_str("instruction")?;
            let mut params = json!({
                "messages": [{ "role": "user", "content": instruction }],
            });
            if let Some(conv_id) = str_arg("conversation_id") {
                params["conv_id"] = Value::String(conv_id);
            }
            ("agent.chat".into(), params, RUN_TASK_TIMEOUT_SECS)
        }
        "list_jobs" => ("kernel.ps".into(), Value::Null, CALL_TIMEOUT_SECS),
        "job_status" => {
            let pid = require_u64("pid")?;
            ("agent.inspect".into(), json!({ "pid": pid }), CALL_TIMEOUT_SECS)
        }
        "stop_job" => {
            let pid = require_u64("pid")?;
            let graceful = args
                .get("graceful")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            (
                "agent.stop".into(),
                json!({ "pid": pid, "graceful": graceful }),
                CALL_TIMEOUT_SECS,
            )
        }
        "spawn_agent" => {
            let agent_id = require_str("agent_id")?;
            (
                "agent.spawn".into(),
                json!({ "agent_id": agent_id }),
                CALL_TIMEOUT_SECS,
            )
        }
        "schedule_job" => {
            let name = require_str("name")?;
            let interval_secs = require_u64("interval_secs")?;
            let command = require_str("command")?;
            let mut params = json!({
                "name": name,
                "interval_secs": interval_secs,
                "command": command,
            });
            if let Some(pid) = u64_arg("target_pid") {
                params["target_pid"] = json!(pid);
            }
            ("cron.add".into(), params, CALL_TIMEOUT_SECS)
        }
        "list_schedules" => ("cron.list".into(), Value::Null, CALL_TIMEOUT_SECS),
        "remove_schedule" => {
            let id = require_str("id")?;
            ("cron.remove".into(), json!({ "id": id }), CALL_TIMEOUT_SECS)
        }
        "list_sensors" => {
            let prefix = str_arg("prefix").unwrap_or_else(|| "substrate/sensor".into());
            let depth = u64_arg("depth").unwrap_or(2);
            let actor_id = str_arg("actor_id").unwrap_or_else(|| DEFAULT_ACTOR_ID.into());
            (
                "substrate.list".into(),
                json!({ "prefix": prefix, "depth": depth, "actor_id": actor_id }),
                CALL_TIMEOUT_SECS,
            )
        }
        "read_sensor" => {
            let path = require_str("path")?;
            let actor_id = str_arg("actor_id").unwrap_or_else(|| DEFAULT_ACTOR_ID.into());
            (
                "substrate.read".into(),
                json!({ "path": path, "actor_id": actor_id }),
                CALL_TIMEOUT_SECS,
            )
        }
        other => return Err(ToolError::NotFound(other.to_string())),
    };
    Ok(t)
}

/// Static tool definitions with JSON-Schema parameters, phrased for a
/// voice agent: short names, spoken-language descriptions.
fn tool_definitions() -> Vec<ToolDefinition> {
    fn def(name: &str, description: &str, schema: Value) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: schema,
        }
    }
    let no_args = json!({ "type": "object", "properties": {} });

    vec![
        def(
            "kernel_status",
            "Get the WeftOS kernel status: uptime, process counts, health.",
            no_args.clone(),
        ),
        def(
            "list_services",
            "List kernel services with their state and health.",
            no_args.clone(),
        ),
        def(
            "run_task",
            "Run a task on WeftOS by describing it in natural language. The \
             concierge agent executes it with its tools and returns the outcome. \
             Use this for one-shot requests like checking something, running a \
             command, or summarising state.",
            json!({
                "type": "object",
                "properties": {
                    "instruction": {
                        "type": "string",
                        "description": "What to do, in plain language."
                    },
                    "conversation_id": {
                        "type": "string",
                        "description": "Stable id to continue a prior task conversation."
                    }
                },
                "required": ["instruction"]
            }),
        ),
        def(
            "list_jobs",
            "List running jobs (kernel processes/agents) with pid, state, and resource use.",
            no_args.clone(),
        ),
        def(
            "job_status",
            "Inspect one job by pid: state, resource usage, capabilities.",
            json!({
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Job pid from list_jobs." }
                },
                "required": ["pid"]
            }),
        ),
        def(
            "stop_job",
            "Stop a running job by pid.",
            json!({
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Job pid from list_jobs." },
                    "graceful": { "type": "boolean", "description": "Graceful stop (default true)." }
                },
                "required": ["pid"]
            }),
        ),
        def(
            "spawn_agent",
            "Spawn a registered agent as a new background job. Returns the new pid.",
            json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Registered agent identifier." }
                },
                "required": ["agent_id"]
            }),
        ),
        def(
            "schedule_job",
            "Schedule a recurring job that fires every interval_secs seconds.",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Human-readable job name." },
                    "interval_secs": { "type": "integer", "description": "Fire every N seconds." },
                    "command": { "type": "string", "description": "Command payload to send." },
                    "target_pid": { "type": "integer", "description": "Optional target agent pid." }
                },
                "required": ["name", "interval_secs", "command"]
            }),
        ),
        def(
            "list_schedules",
            "List scheduled (cron) jobs with id, interval, and fire counts.",
            no_args.clone(),
        ),
        def(
            "remove_schedule",
            "Remove a scheduled job by its id (from list_schedules).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Schedule id from list_schedules." }
                },
                "required": ["id"]
            }),
        ),
        def(
            "list_sensors",
            "List sensors on the substrate tree. Defaults to the substrate/sensor \
             prefix; pass a different prefix to browse other substrate paths.",
            json!({
                "type": "object",
                "properties": {
                    "prefix": { "type": "string", "description": "Substrate path prefix (default substrate/sensor)." },
                    "depth": { "type": "integer", "description": "Levels to enumerate (default 2)." },
                    "actor_id": { "type": "string", "description": "Caller actor id for ACL-gated paths." }
                }
            }),
        ),
        def(
            "read_sensor",
            "Read the current value of a sensor (or any substrate path).",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Full substrate path, e.g. substrate/sensor/mic." },
                    "actor_id": { "type": "string", "description": "Caller actor id for ACL-gated paths." }
                },
                "required": ["path"]
            }),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_have_valid_schemas() {
        let defs = tool_definitions();
        assert_eq!(defs.len(), 12);
        for d in &defs {
            assert!(!d.name.is_empty());
            assert!(!d.description.is_empty());
            assert_eq!(d.input_schema["type"], "object");
        }
    }

    #[test]
    fn build_request_maps_all_defined_tools() {
        // Every advertised tool must map to a daemon method.
        let cases: Vec<(&str, Value)> = vec![
            ("kernel_status", json!({})),
            ("list_services", json!({})),
            ("run_task", json!({ "instruction": "check disk space" })),
            ("list_jobs", json!({})),
            ("job_status", json!({ "pid": 3 })),
            ("stop_job", json!({ "pid": 3 })),
            ("spawn_agent", json!({ "agent_id": "concierge-bot" })),
            (
                "schedule_job",
                json!({ "name": "n", "interval_secs": 60, "command": "c" }),
            ),
            ("list_schedules", json!({})),
            ("remove_schedule", json!({ "id": "abc" })),
            ("list_sensors", json!({})),
            ("read_sensor", json!({ "path": "substrate/sensor/mic" })),
        ];
        assert_eq!(cases.len(), tool_definitions().len());
        for (name, args) in cases {
            let (method, _, _) = build_request(name, &args)
                .unwrap_or_else(|e| panic!("tool {name} failed to map: {e}"));
            assert!(method.contains('.'), "{name} mapped to bad method {method}");
        }
    }

    #[test]
    fn build_request_missing_arg_is_execution_error() {
        let err = build_request("run_task", &json!({})).unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    #[test]
    fn build_request_unknown_tool_is_not_found() {
        let err = build_request("bogus", &json!({})).unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[test]
    fn substrate_reads_get_default_actor_id() {
        let (_, params, _) = build_request("read_sensor", &json!({ "path": "substrate/sensor/mic" }))
            .unwrap();
        assert_eq!(params["actor_id"], DEFAULT_ACTOR_ID);
        let (_, params, _) = build_request("list_sensors", &json!({})).unwrap();
        assert_eq!(params["prefix"], "substrate/sensor");
        assert_eq!(params["actor_id"], DEFAULT_ACTOR_ID);
    }
}
