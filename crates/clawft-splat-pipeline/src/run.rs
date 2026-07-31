//! Subprocess runner — every pipeline stage shells out through here.
//!
//! One place owns: spawning, log capture (`logs/<stage>.log`), timing,
//! and exit-status checking. Mirrors the whisper probe's HTTP-as-stage
//! discipline: external tools keep their own lifecycle; we only observe.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncWriteExt;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("failed to spawn `{tool}`: {source} (is it installed and on PATH?)")]
    Spawn { tool: String, source: std::io::Error },
    #[error("`{tool}` exited with {code:?} — see {log}")]
    Failed { tool: String, code: Option<i32>, log: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Outcome of one tool invocation.
pub struct RunOutput {
    pub elapsed_ms: u64,
    /// Combined stdout+stderr, also appended to the stage log.
    pub output: String,
}

/// Run `tool args…` with cwd `dir`, appending combined output to `log_path`.
pub async fn run_tool(
    tool: &str,
    args: &[String],
    dir: &Path,
    log_path: &Path,
) -> Result<RunOutput, RunError> {
    let started = Instant::now();

    let mut log = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await?;
    log.write_all(format!("\n$ {} {}\n", tool, args.join(" ")).as_bytes())
        .await?;

    let out = tokio::process::Command::new(tool)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|source| RunError::Spawn { tool: tool.to_string(), source })?;

    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    log.write_all(combined.as_bytes()).await?;
    log.flush().await?;

    if !out.status.success() {
        return Err(RunError::Failed {
            tool: tool.to_string(),
            code: out.status.code(),
            log: log_path.display().to_string(),
        });
    }

    Ok(RunOutput { elapsed_ms: started.elapsed().as_millis() as u64, output: combined })
}

/// Resolve a stage's log file path.
pub fn stage_log(logs_dir: &Path, stage: &str) -> PathBuf {
    logs_dir.join(format!("{stage}.log"))
}
