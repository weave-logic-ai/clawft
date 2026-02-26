//! Process spawning abstraction and native implementation.
//!
//! Provides a platform-agnostic [`ProcessSpawner`] trait for running external
//! commands. The native implementation uses [`tokio::process`]. This capability
//! is unavailable in WASM environments, which is why [`super::Platform::process`]
//! returns `Option`.

use async_trait::async_trait;
use std::path::Path;

/// Result of running an external process.
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// Process exit code. 0 typically indicates success.
    pub exit_code: i32,
    /// Captured standard output as a UTF-8 string.
    pub stdout: String,
    /// Captured standard error as a UTF-8 string.
    pub stderr: String,
}

/// Platform-agnostic process spawner.
///
/// Implementations run external commands and capture their output.
/// This is intentionally not part of the [`super::Platform`] trait bundle
/// because it is unavailable in WASM environments.
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
pub trait ProcessSpawner: Send + Sync {
    /// Run a command with arguments and capture its output.
    ///
    /// # Arguments
    /// * `command` - The executable to run.
    /// * `args` - Command-line arguments.
    /// * `working_dir` - Optional working directory for the process.
    /// * `timeout_secs` - Optional timeout in seconds. If the process exceeds
    ///   this duration, it will be killed and an error returned.
    async fn run(
        &self,
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout_secs: Option<u64>,
    ) -> Result<ProcessOutput, Box<dyn std::error::Error + Send + Sync>>;
}

/// Native process spawner using [`tokio::process`].
#[cfg(feature = "native")]
pub struct NativeProcessSpawner;

#[cfg(feature = "native")]
#[async_trait]
impl ProcessSpawner for NativeProcessSpawner {
    async fn run(
        &self,
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout_secs: Option<u64>,
    ) -> Result<ProcessOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn()?;

        let output = if let Some(secs) = timeout_secs {
            let timeout = std::time::Duration::from_secs(secs);
            match tokio::time::timeout(timeout, child.wait_with_output()).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(format!("process '{}' timed out after {}s", command, secs).into());
                }
            }
        } else {
            child.wait_with_output().await?
        };

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok(ProcessOutput {
            exit_code,
            stdout,
            stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_echo() {
        let spawner = NativeProcessSpawner;
        let output = spawner
            .run("echo", &["hello", "world"], None, Some(10))
            .await
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.trim(), "hello world");
        assert!(output.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_run_with_working_dir() {
        let spawner = NativeProcessSpawner;
        let output = spawner
            .run("pwd", &[], Some(Path::new("/tmp")), Some(10))
            .await
            .unwrap();

        assert_eq!(output.exit_code, 0);
        // /tmp may be a symlink (e.g., to /private/tmp on macOS)
        assert!(output.stdout.trim().ends_with("tmp"));
    }

    #[tokio::test]
    async fn test_run_nonexistent_command() {
        let spawner = NativeProcessSpawner;
        let result = spawner
            .run("clawft_nonexistent_command_xyz", &[], None, Some(5))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_captures_stderr() {
        let spawner = NativeProcessSpawner;
        let output = spawner
            .run("sh", &["-c", "echo error >&2"], None, Some(10))
            .await
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stderr.trim(), "error");
    }

    #[tokio::test]
    async fn test_run_nonzero_exit_code() {
        let spawner = NativeProcessSpawner;
        let output = spawner
            .run("sh", &["-c", "exit 42"], None, Some(10))
            .await
            .unwrap();

        assert_eq!(output.exit_code, 42);
    }

    #[tokio::test]
    async fn test_run_timeout() {
        let spawner = NativeProcessSpawner;
        let result = spawner.run("sleep", &["60"], None, Some(1)).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timed out"),
            "expected timeout error, got: {err_msg}"
        );
    }
}
