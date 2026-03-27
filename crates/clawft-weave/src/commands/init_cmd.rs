//! `weaver init` subcommand implementation.
//!
//! Initializes the clawft+WeftOS development environment by copying
//! agent/skill definitions from `agents/` to `.claude/skills/` and
//! verifying build tooling.
//!
//! This follows the ruflo pattern: the repo (`agents/`) is the source
//! of truth, `.claude/skills/` is the runtime copy.

use clap::Parser;

/// Initialize clawft+WeftOS development environment.
#[derive(Parser)]
#[command(about = "Initialize development environment (install skills, verify tools)")]
pub struct InitArgs {
    /// Overwrite existing skills in .claude/skills/.
    #[arg(short, long)]
    pub force: bool,

    /// Only install skills (skip build tool checks).
    #[arg(long)]
    pub skills: bool,

    /// Run initial ECC analysis after setup.
    #[arg(long)]
    pub analyze: bool,
}

/// Run the init command by delegating to `scripts/weave-init.sh`.
pub async fn run(args: InitArgs) -> anyhow::Result<()> {
    // Locate the project root by finding scripts/weave-init.sh relative
    // to the current working directory or the binary's location.
    let cwd = std::env::current_dir()?;
    let script = cwd.join("scripts/weave-init.sh");

    if !script.exists() {
        anyhow::bail!(
            "scripts/weave-init.sh not found in {}. \
             Run `weaver init` from the project root.",
            cwd.display()
        );
    }

    let mut cmd = tokio::process::Command::new(&script);

    if args.force {
        cmd.arg("--force");
    }
    if args.skills {
        cmd.arg("--skills");
    }
    if args.analyze {
        cmd.arg("--analyze");
    }

    let status = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("weave-init.sh exited with status {}", status);
    }

    Ok(())
}
