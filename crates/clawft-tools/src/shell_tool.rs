//! Shell execution tool.
//!
//! Ported from Python `nanobot/agent/tools/shell.py`. Executes shell commands
//! with timeout enforcement and dangerous command rejection.

use std::path::PathBuf;
use std::time::Instant;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use serde_json::json;
use tracing::{debug, warn};

/// Maximum allowed timeout in seconds.
const MAX_TIMEOUT_SECS: u64 = 300;

/// Default timeout in seconds when none is specified.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Patterns that indicate dangerous commands. If any pattern is found
/// (case-insensitive) in the command string, execution is rejected.
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "sudo ",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
    "chmod 777 /",
    "> /dev/sd",
    "shutdown",
    "reboot",
    "poweroff",
    "format c:",
];

/// Check whether a command string contains any dangerous patterns.
fn is_dangerous(command: &str) -> Option<&'static str> {
    let lower = command.to_lowercase();
    DANGEROUS_PATTERNS
        .iter()
        .find(|pattern| lower.contains(*pattern))
        .copied()
}

/// Execute shell commands with safety guardrails.
///
/// Commands are run with the working directory set to the configured
/// workspace. Dangerous commands are rejected before execution, and
/// a configurable timeout prevents runaway processes.
pub struct ShellExecTool {
    workspace: PathBuf,
    max_timeout: u64,
}

impl ShellExecTool {
    /// Create a new `ShellExecTool` with the given workspace directory.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            max_timeout: MAX_TIMEOUT_SECS,
        }
    }

    /// Create a new `ShellExecTool` with a custom maximum timeout.
    pub fn with_max_timeout(workspace: PathBuf, max_timeout: u64) -> Self {
        Self {
            workspace,
            max_timeout,
        }
    }
}

#[async_trait]
impl Tool for ShellExecTool {
    fn name(&self) -> &str {
        "exec_shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. Enforces timeout and rejects dangerous commands."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds (default 30, max 300)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: command".to_string()))?;

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(self.max_timeout);

        // Safety check: reject dangerous commands.
        if let Some(pattern) = is_dangerous(command) {
            warn!(command, pattern, "dangerous command rejected");
            return Err(ToolError::PermissionDenied(format!(
                "command blocked by safety guard (matched: {})",
                pattern
            )));
        }

        debug!(command, timeout_secs, "executing shell command");

        let start = Instant::now();

        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workspace)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to spawn process: {}", e)))?;

        // Take stdout/stderr handles before awaiting so we can read them
        // and still kill the child on timeout.
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let wait_result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait(),
        )
        .await;

        let status = match wait_result {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => {
                return Err(ToolError::ExecutionFailed(format!(
                    "process error: {}",
                    e
                )));
            }
            Err(_) => {
                // Attempt to kill the timed-out process.
                let _ = child.kill().await;
                return Err(ToolError::Timeout(timeout_secs));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = status.code().unwrap_or(-1);

        let stdout = if let Some(mut handle) = stdout_handle {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let _ = handle.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).into_owned()
        } else {
            String::new()
        };

        let stderr = if let Some(mut handle) = stderr_handle {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let _ = handle.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).into_owned()
        } else {
            String::new()
        };

        Ok(json!({
            "exit_code": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "duration_ms": duration_ms,
        }))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_workspace() -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_shell_test_{pid}_{id}"))
    }

    async fn setup() -> (ShellExecTool, PathBuf) {
        let ws = temp_workspace();
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let tool = ShellExecTool::new(ws.clone());
        (tool, ws)
    }

    async fn cleanup(ws: &std::path::Path) {
        let _ = tokio::fs::remove_dir_all(ws).await;
    }

    #[tokio::test]
    async fn test_echo_command() {
        let (tool, ws) = setup().await;

        let result = tool
            .execute(json!({"command": "echo hello world"}))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"].as_str().unwrap().trim(), "hello world");
        assert!(result["duration_ms"].as_u64().is_some());

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_nonzero_exit_code() {
        let (tool, ws) = setup().await;

        let result = tool
            .execute(json!({"command": "exit 42"}))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 42);

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_captures_stderr() {
        let (tool, ws) = setup().await;

        let result = tool
            .execute(json!({"command": "echo error >&2"}))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stderr"].as_str().unwrap().trim(), "error");

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_timeout() {
        let ws = temp_workspace();
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let tool = ShellExecTool::with_max_timeout(ws.clone(), 5);

        let err = tool
            .execute(json!({"command": "sleep 60", "timeout": 1}))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Timeout(1)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_timeout_clamped_to_max() {
        let ws = temp_workspace();
        tokio::fs::create_dir_all(&ws).await.unwrap();
        // max_timeout = 2, requested timeout = 100 -> clamped to 2
        let tool = ShellExecTool::with_max_timeout(ws.clone(), 2);

        let err = tool
            .execute(json!({"command": "sleep 60", "timeout": 100}))
            .await
            .unwrap_err();

        // Should timeout at 2, not 100
        assert!(matches!(err, ToolError::Timeout(2)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_dangerous_rm_rf() {
        let (tool, ws) = setup().await;

        let err = tool
            .execute(json!({"command": "rm -rf /"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));
        assert!(err.to_string().contains("safety guard"));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_dangerous_sudo() {
        let (tool, ws) = setup().await;

        let err = tool
            .execute(json!({"command": "sudo apt-get install evil"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_dangerous_mkfs() {
        let (tool, ws) = setup().await;

        let err = tool
            .execute(json!({"command": "mkfs.ext4 /dev/sda1"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_dangerous_dd() {
        let (tool, ws) = setup().await;

        let err = tool
            .execute(json!({"command": "dd if=/dev/zero of=/dev/sda"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_dangerous_fork_bomb() {
        let (tool, ws) = setup().await;

        let err = tool
            .execute(json!({"command": ":(){ :|:& };:"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PermissionDenied(_)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_safe_command_allowed() {
        let (tool, ws) = setup().await;

        // rm on a specific file (not rm -rf /) should be allowed
        let result = tool
            .execute(json!({"command": "echo safe rm -rf ./foo"}))
            .await;
        // "rm -rf /" pattern won't match "rm -rf ./foo"
        assert!(result.is_ok());

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_working_directory() {
        let (tool, ws) = setup().await;

        let result = tool.execute(json!({"command": "pwd"})).await.unwrap();

        let stdout = result["stdout"].as_str().unwrap().trim();
        // The canonical form of the workspace should match pwd output
        let canonical_ws = ws.canonicalize().unwrap();
        let canonical_stdout = PathBuf::from(stdout)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(stdout));

        assert_eq!(canonical_stdout, canonical_ws);

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_missing_command_param() {
        let (tool, ws) = setup().await;

        let err = tool.execute(json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));

        cleanup(&ws).await;
    }

    #[tokio::test]
    async fn test_default_timeout_used() {
        let (tool, ws) = setup().await;

        // A fast command should succeed within default timeout
        let result = tool
            .execute(json!({"command": "echo fast"}))
            .await
            .unwrap();
        assert_eq!(result["exit_code"], 0);

        cleanup(&ws).await;
    }

    #[test]
    fn test_is_dangerous_none_for_safe() {
        assert!(is_dangerous("ls -la").is_none());
        assert!(is_dangerous("echo hello").is_none());
        assert!(is_dangerous("cat file.txt").is_none());
        assert!(is_dangerous("rm specific_file.txt").is_none());
    }

    #[test]
    fn test_is_dangerous_detects_patterns() {
        assert!(is_dangerous("rm -rf /").is_some());
        assert!(is_dangerous("sudo something").is_some());
        assert!(is_dangerous("SUDO something").is_some()); // case insensitive
        assert!(is_dangerous("mkfs.ext4 /dev/sda").is_some());
        assert!(is_dangerous("dd if=/dev/zero").is_some());
        assert!(is_dangerous(":(){ :|:& };:").is_some());
    }
}
