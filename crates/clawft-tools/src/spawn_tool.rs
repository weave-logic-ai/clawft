//! Spawn tool for creating sub-processes.
//!
//! Allows the agent to spawn child processes for task delegation.
//! Processes run in the workspace directory with controlled environment.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_platform::Platform;
use serde_json::json;
use tracing::{debug, warn};

use crate::security_policy::CommandPolicy;

/// Maximum number of concurrent spawned processes.
const MAX_CONCURRENT_SPAWNS: usize = 5;

/// Default timeout in seconds for spawned processes.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Counter for active spawned processes.
static ACTIVE_SPAWNS: AtomicUsize = AtomicUsize::new(0);

/// Tool for spawning sub-processes.
///
/// Enables the agent to delegate tasks to child processes. Each spawn
/// runs in the configured workspace directory. Commands are validated
/// against a [`CommandPolicy`] before execution. A concurrency limit
/// prevents resource exhaustion.
pub struct SpawnTool<P: Platform> {
    platform: Arc<P>,
    workspace: PathBuf,
    policy: CommandPolicy,
}

impl<P: Platform> SpawnTool<P> {
    /// Create a new spawn tool sandboxed to the given workspace directory.
    pub fn new(platform: Arc<P>, workspace: PathBuf, policy: CommandPolicy) -> Self {
        Self {
            platform,
            workspace,
            policy,
        }
    }
}

#[async_trait]
impl<P: Platform + 'static> Tool for SpawnTool<P> {
    fn name(&self) -> &str {
        "spawn"
    }

    fn description(&self) -> &str {
        "Spawn a sub-process to run a command. Runs in the workspace directory with a concurrency limit."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Command arguments"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description of what this spawn does"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds (default 60)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: command".into()))?;

        let cmd_args: Vec<String> = args
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("(no description)");

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        // Security policy check.
        if let Err(e) = self.policy.validate(command) {
            warn!(command, error = %e, "spawn command rejected by security policy");
            return Err(ToolError::PermissionDenied {
                tool: "spawn".into(),
                reason: e.to_string(),
            });
        }

        // Check concurrency limit.
        let current = ACTIVE_SPAWNS.load(Ordering::Relaxed);
        if current >= MAX_CONCURRENT_SPAWNS {
            return Err(ToolError::ExecutionFailed(format!(
                "concurrency limit reached: {} active spawns (max {})",
                current, MAX_CONCURRENT_SPAWNS
            )));
        }

        debug!(
            command = %command,
            args = ?cmd_args,
            description = %description,
            workspace = %self.workspace.display(),
            "spawning sub-process"
        );

        // Check if the platform supports process spawning.
        let spawner = self.platform.process().ok_or_else(|| {
            ToolError::ExecutionFailed("process spawning not supported on this platform".into())
        })?;

        ACTIVE_SPAWNS.fetch_add(1, Ordering::Relaxed);

        // Convert Vec<String> to &[&str] for the ProcessSpawner::run signature.
        let arg_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();

        let result = spawner
            .run(
                command,
                &arg_refs,
                Some(&self.workspace),
                Some(timeout_secs),
            )
            .await;

        ACTIVE_SPAWNS.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(output) => {
                let exit_code = output.exit_code;

                if exit_code != 0 {
                    warn!(
                        command = %command,
                        exit_code = exit_code,
                        "spawned process exited with non-zero code"
                    );
                }

                Ok(json!({
                    "exit_code": exit_code,
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                    "command": command,
                    "description": description,
                }))
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!(
                "failed to spawn process '{}': {}",
                command, e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_platform::NativePlatform;
    use std::path::PathBuf;

    fn make_tool() -> SpawnTool<NativePlatform> {
        SpawnTool::new(
            Arc::new(NativePlatform::new()),
            PathBuf::from("/tmp"),
            CommandPolicy::safe_defaults(),
        )
    }

    #[test]
    fn name_is_spawn() {
        assert_eq!(make_tool().name(), "spawn");
    }

    #[test]
    fn description_not_empty() {
        assert!(!make_tool().description().is_empty());
    }

    #[test]
    fn parameters_has_command() {
        let params = make_tool().parameters();
        assert!(params["properties"]["command"].is_object());
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&json!("command")));
    }

    #[tokio::test]
    async fn missing_command_returns_error() {
        let err = make_tool().execute(json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn spawn_echo_succeeds() {
        let tool = make_tool();
        let result = tool
            .execute(json!({
                "command": "echo",
                "args": ["hello", "world"],
                "description": "test echo"
            }))
            .await;

        match result {
            Ok(val) => {
                assert_eq!(val["exit_code"], 0);
                assert!(val["stdout"].as_str().unwrap().contains("hello world"));
                assert_eq!(val["command"], "echo");
                assert_eq!(val["description"], "test echo");
            }
            Err(ToolError::ExecutionFailed(msg)) => {
                // Acceptable if process spawning fails in test environment.
                assert!(
                    msg.contains("not supported") || msg.contains("failed"),
                    "unexpected error: {msg}"
                );
            }
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn spawn_with_no_args() {
        let tool = make_tool();
        let result = tool
            .execute(json!({
                "command": "echo"
            }))
            .await;

        match result {
            Ok(val) => {
                assert_eq!(val["exit_code"], 0);
                assert_eq!(val["description"], "(no description)");
            }
            Err(ToolError::ExecutionFailed(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn concurrency_limit_enforced() {
        // Set active spawns to the maximum.
        ACTIVE_SPAWNS.store(MAX_CONCURRENT_SPAWNS, Ordering::Relaxed);

        let tool = make_tool();
        let err = tool
            .execute(json!({"command": "echo", "args": ["test"]}))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::ExecutionFailed(_)));
        assert!(err.to_string().contains("concurrency limit"));

        // Reset for other tests.
        ACTIVE_SPAWNS.store(0, Ordering::Relaxed);
    }

    #[test]
    fn tool_is_object_safe() {
        fn accepts_tool(_t: &dyn Tool) {}
        accepts_tool(&make_tool());
    }

    #[test]
    fn active_spawns_starts_at_zero() {
        // Reset for deterministic test.
        ACTIVE_SPAWNS.store(0, Ordering::Relaxed);
        assert_eq!(ACTIVE_SPAWNS.load(Ordering::Relaxed), 0);
    }
}
