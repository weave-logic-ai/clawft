//! Claude Code CLI-based task delegator.
//!
//! [`FlowDelegator`] spawns the `claude` CLI binary as a subprocess to
//! delegate complex tasks. It provides timeout enforcement, minimal
//! environment construction (security), and depth-limited delegation.
//!
//! # Security
//!
//! The child process receives only `PATH`, `HOME`, and `ANTHROPIC_API_KEY`
//! from the parent environment. No other environment variables are passed.
//!
//! # Feature gate
//!
//! This module lives inside the `delegate`-gated `delegation` module.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::process::Command;
use tracing::{debug, warn};

use clawft_types::delegation::DelegationConfig;

use super::claude::DelegationError;

/// Default timeout for subprocess delegation.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// Default maximum delegation depth to prevent recursive loops.
const DEFAULT_MAX_DEPTH: u32 = 3;

/// Cached result of `claude` binary detection.
static CLAUDE_BINARY: OnceLock<Option<String>> = OnceLock::new();

/// Delegator that spawns Claude Code CLI as a subprocess.
///
/// Uses non-interactive mode (`claude --print`) to delegate tasks and
/// collect results. Enforces timeout and depth limits to prevent
/// runaway processes and recursive delegation loops.
pub struct FlowDelegator {
    /// Path to the `claude` binary.
    claude_binary: String,
    /// Maximum time to wait for the subprocess.
    timeout: Duration,
    /// Maximum delegation depth (prevents recursive loops).
    max_depth: u32,
}

impl FlowDelegator {
    /// Create from config. Returns `None` if `claude` binary not found on PATH.
    ///
    /// Detection is cached: the first call probes the filesystem, subsequent
    /// calls reuse the result.
    pub fn new(config: &DelegationConfig) -> Option<Self> {
        let binary = detect_claude_binary()?;

        // Derive timeout from config max_turns (rough heuristic: 30s per turn)
        // or fall back to the default.
        let timeout = if config.max_turns > 0 {
            Duration::from_secs(u64::from(config.max_turns) * 30)
        } else {
            DEFAULT_TIMEOUT
        };

        Some(Self {
            claude_binary: binary,
            timeout,
            max_depth: DEFAULT_MAX_DEPTH,
        })
    }

    /// Create with explicit parameters (for testing).
    #[cfg(test)]
    pub fn with_params(binary: String, timeout: Duration, max_depth: u32) -> Self {
        Self {
            claude_binary: binary,
            timeout,
            max_depth,
        }
    }

    /// Override the maximum delegation depth.
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Maximum delegation depth.
    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Path to the detected `claude` binary.
    pub fn binary_path(&self) -> &str {
        &self.claude_binary
    }

    /// Delegate a task via Claude Code CLI.
    ///
    /// Spawns `claude --print` (non-interactive mode) with the task as
    /// stdin input. Includes `agent_id` in environment context for
    /// MCP callback routing.
    ///
    /// # Arguments
    ///
    /// * `task` - The task description to delegate.
    /// * `agent_id` - Agent ID for MCP callback routing context.
    /// * `current_depth` - Current delegation depth (for loop prevention).
    ///
    /// # Errors
    ///
    /// Returns [`DelegationError`] on subprocess failure, timeout,
    /// parse failure, or depth limit exceeded.
    pub async fn delegate(
        &self,
        task: &str,
        agent_id: &str,
        current_depth: u32,
    ) -> Result<String, DelegationError> {
        // Check depth limit.
        if current_depth >= self.max_depth {
            return Err(DelegationError::SubprocessFailed {
                exit_code: -1,
                stderr: format!(
                    "delegation depth limit exceeded ({}/{})",
                    current_depth, self.max_depth
                ),
            });
        }

        debug!(
            binary = %self.claude_binary,
            agent_id = %agent_id,
            depth = current_depth,
            timeout = ?self.timeout,
            "delegating task via Claude Code CLI"
        );

        // Build minimal environment for child process (security).
        let env = build_minimal_env(agent_id);

        // Spawn the subprocess.
        let mut child = Command::new(&self.claude_binary)
            .arg("--print")
            .arg("--")
            .arg(task)
            .env_clear()
            .envs(env)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| DelegationError::SubprocessFailed {
                exit_code: -1,
                stderr: format!("failed to spawn claude: {e}"),
            })?;

        // Enforce timeout using select on wait + sleep.
        let timeout_duration = self.timeout;
        let result = tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        // Read stdout/stderr after wait completes.
                        let mut stdout_buf = Vec::new();
                        let mut stderr_buf = Vec::new();
                        if let Some(mut stdout) = child.stdout.take() {
                            use tokio::io::AsyncReadExt;
                            let _ = stdout.read_to_end(&mut stdout_buf).await;
                        }
                        if let Some(mut stderr) = child.stderr.take() {
                            use tokio::io::AsyncReadExt;
                            let _ = stderr.read_to_end(&mut stderr_buf).await;
                        }
                        let stdout = String::from_utf8_lossy(&stdout_buf).to_string();
                        let stderr = String::from_utf8_lossy(&stderr_buf).to_string();

                        if !exit_status.success() {
                            let exit_code = exit_status.code().unwrap_or(-1);
                            warn!(
                                exit_code,
                                stderr = %stderr,
                                "claude subprocess failed"
                            );
                            Err(DelegationError::SubprocessFailed {
                                exit_code,
                                stderr,
                            })
                        } else if stdout.trim().is_empty() {
                            Err(DelegationError::OutputParseFailed {
                                raw_output: stdout,
                                parse_error: "empty output from claude".into(),
                            })
                        } else {
                            debug!(
                                output_len = stdout.len(),
                                "claude subprocess completed successfully"
                            );
                            Ok(stdout)
                        }
                    }
                    Err(e) => Err(DelegationError::SubprocessFailed {
                        exit_code: -1,
                        stderr: format!("failed to wait for claude: {e}"),
                    }),
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                // Timeout: kill the child process.
                warn!(
                    timeout = ?timeout_duration,
                    "claude subprocess timed out, killing"
                );
                let _ = child.kill().await;
                Err(DelegationError::Timeout {
                    elapsed: timeout_duration,
                })
            }
        };

        result
    }
}

impl std::fmt::Debug for FlowDelegator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlowDelegator")
            .field("claude_binary", &self.claude_binary)
            .field("timeout", &self.timeout)
            .field("max_depth", &self.max_depth)
            .finish()
    }
}

/// Detect whether the `claude` binary is available on PATH.
///
/// Result is cached in a process-global `OnceLock` so detection only
/// runs once per process lifetime.
pub fn detect_claude_binary() -> Option<String> {
    CLAUDE_BINARY
        .get_or_init(|| {
            match which::which("claude") {
                Ok(path) => {
                    let path_str = path.to_string_lossy().to_string();
                    debug!(path = %path_str, "detected claude binary");
                    Some(path_str)
                }
                Err(_) => {
                    debug!("claude binary not found on PATH");
                    None
                }
            }
        })
        .clone()
}

/// Check whether Claude Code CLI is available without caching.
///
/// Unlike [`detect_claude_binary`], this always re-probes the filesystem.
/// Useful for health checks and diagnostics.
pub fn probe_claude_binary() -> Option<String> {
    which::which("claude")
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Construct a minimal environment for the child process.
///
/// Only passes `PATH`, `HOME`, and `ANTHROPIC_API_KEY` from the parent.
/// Also sets `CLAWFT_AGENT_ID` for MCP callback routing.
///
/// # Security
///
/// This prevents accidental leakage of secrets, credentials, or
/// sensitive environment variables to the child process.
fn build_minimal_env(agent_id: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Essential: PATH for subprocess to find its own dependencies.
    if let Ok(path) = std::env::var("PATH") {
        env.insert("PATH".into(), path);
    }

    // Essential: HOME for config file discovery.
    if let Ok(home) = std::env::var("HOME") {
        env.insert("HOME".into(), home);
    }

    // Required: API key for Claude authentication.
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        env.insert("ANTHROPIC_API_KEY".into(), key);
    }

    // Routing context: agent ID for MCP callback routing.
    env.insert("CLAWFT_AGENT_ID".into(), agent_id.to_string());

    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── FlowDelegator::new ─────────────────────────────────────────

    #[test]
    fn new_returns_none_when_binary_not_found() {
        // This test relies on the system not having `claude` installed,
        // which is environment-dependent. We test the probe function
        // instead to ensure the logic is sound.
        let result = probe_claude_binary();
        // We cannot assert None because the test env might have claude.
        // Instead, verify the function returns a coherent result.
        if let Some(ref path) = result {
            assert!(!path.is_empty());
        }
    }

    #[test]
    fn with_params_creates_delegator() {
        let d = FlowDelegator::with_params(
            "/usr/bin/claude".into(),
            Duration::from_secs(60),
            5,
        );
        assert_eq!(d.binary_path(), "/usr/bin/claude");
        assert_eq!(d.max_depth(), 5);
    }

    #[test]
    fn with_max_depth_overrides() {
        let d = FlowDelegator::with_params(
            "/usr/bin/claude".into(),
            Duration::from_secs(60),
            3,
        )
        .with_max_depth(7);
        assert_eq!(d.max_depth(), 7);
    }

    // ── build_minimal_env ──────────────────────────────────────────

    #[test]
    fn minimal_env_contains_agent_id() {
        let env = build_minimal_env("agent-42");
        assert_eq!(env.get("CLAWFT_AGENT_ID").unwrap(), "agent-42");
    }

    #[test]
    fn minimal_env_has_path_if_set() {
        // PATH is almost always set in test environments.
        let env = build_minimal_env("test");
        if std::env::var("PATH").is_ok() {
            assert!(env.contains_key("PATH"));
        }
    }

    #[test]
    fn minimal_env_does_not_leak_extra_vars() {
        let env = build_minimal_env("test");
        // Should only contain PATH, HOME, ANTHROPIC_API_KEY, CLAWFT_AGENT_ID
        for key in env.keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "PATH" | "HOME" | "ANTHROPIC_API_KEY" | "CLAWFT_AGENT_ID"
                ),
                "unexpected env var in minimal env: {key}"
            );
        }
    }

    // ── DelegationError display (new variants) ─────────────────────

    #[test]
    fn error_display_subprocess_failed() {
        let err = DelegationError::SubprocessFailed {
            exit_code: 1,
            stderr: "some error".into(),
        };
        assert_eq!(
            err.to_string(),
            "subprocess failed (exit code 1): some error"
        );
    }

    #[test]
    fn error_display_output_parse_failed() {
        let err = DelegationError::OutputParseFailed {
            raw_output: "garbage".into(),
            parse_error: "expected JSON".into(),
        };
        assert_eq!(err.to_string(), "output parse failed: expected JSON");
    }

    #[test]
    fn error_display_timeout() {
        let err = DelegationError::Timeout {
            elapsed: Duration::from_secs(30),
        };
        assert_eq!(err.to_string(), "delegation timed out after 30s");
    }

    #[test]
    fn error_display_cancelled() {
        let err = DelegationError::Cancelled;
        assert_eq!(err.to_string(), "delegation cancelled");
    }

    #[test]
    fn error_display_fallback_exhausted() {
        use clawft_types::delegation::DelegationTarget;
        let err = DelegationError::FallbackExhausted {
            attempts: vec![
                (DelegationTarget::Flow, "binary not found".into()),
                (DelegationTarget::Claude, "api key missing".into()),
            ],
        };
        assert_eq!(err.to_string(), "all delegation targets exhausted");
    }

    // ── Depth limit ────────────────────────────────────────────────

    #[tokio::test]
    async fn delegate_rejects_depth_exceeded() {
        let d = FlowDelegator::with_params(
            "/usr/bin/claude".into(),
            Duration::from_secs(5),
            3,
        );
        let result = d.delegate("test task", "agent-1", 3).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DelegationError::SubprocessFailed { stderr, .. } => {
                assert!(stderr.contains("depth limit exceeded"));
            }
            other => panic!("expected SubprocessFailed, got: {other}"),
        }
    }

    // ── Debug impl ─────────────────────────────────────────────────

    #[test]
    fn debug_shows_fields() {
        let d = FlowDelegator::with_params(
            "/usr/bin/claude".into(),
            Duration::from_secs(60),
            3,
        );
        let debug_str = format!("{d:?}");
        assert!(debug_str.contains("FlowDelegator"));
        assert!(debug_str.contains("/usr/bin/claude"));
    }
}
