//! `weft delegate` — debug and inspect task-delegation routing (WEFT-196).
//!
//! Surfaces [`DelegationEngine`] decisions for a free-text task without
//! executing the task. Useful when tuning rules or diagnosing why a task
//! went Local vs Claude.

use clap::{Args, Subcommand};
use clawft_platform::NativePlatform;

use super::load_config;

/// Arguments for the `weft delegate` subcommand.
#[derive(Args, Debug)]
pub struct DelegateArgs {
    #[command(subcommand)]
    pub action: DelegateAction,
}

/// Subcommands for `weft delegate`.
#[derive(Subcommand, Debug)]
pub enum DelegateAction {
    /// Show which target a free-text task would route to.
    ///
    /// Exercises the configured [`DelegationEngine`] (rules + complexity
    /// heuristic) and prints the resolved target, complexity score, matched
    /// rule, and availability inputs.
    Debug {
        /// Free-text task description to evaluate.
        task: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,

        /// Emit machine-readable JSON instead of human text.
        #[arg(long)]
        json: bool,

        /// Override Claude availability for the decision
        /// (`true` / `false`). Default: true when an Anthropic API key
        /// is present in env or providers config, else false.
        #[arg(long)]
        claude_available: Option<bool>,
    },
}

/// Run the `weft delegate` subcommand.
pub async fn run(args: DelegateArgs) -> anyhow::Result<()> {
    match args.action {
        DelegateAction::Debug {
            task,
            config,
            json,
            claude_available,
        } => run_debug(task, config, json, claude_available).await,
    }
}

async fn run_debug(
    task: String,
    config_path: Option<String>,
    json: bool,
    claude_available_override: Option<bool>,
) -> anyhow::Result<()> {
    #[cfg(not(feature = "delegate"))]
    {
        let _ = (task, config_path, json, claude_available_override);
        anyhow::bail!(
            "delegation support is not compiled into this binary \
             (rebuild with `--features delegate`)"
        );
    }

    #[cfg(feature = "delegate")]
    {
        use clawft_services::delegation::DelegationEngine;

        let platform = NativePlatform::new();
        let cfg = load_config(&platform, config_path.as_deref()).await?;
        let engine = DelegationEngine::new(cfg.delegation.clone());

        let claude_available = match claude_available_override {
            Some(v) => v,
            None => detect_claude_available(&cfg),
        };

        let explanation = engine.explain(&task, claude_available);

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&explanation.to_json())?
            );
            return Ok(());
        }

        println!("weft delegate debug");
        println!("===================");
        println!();
        println!("Task:              {}", explanation.task);
        println!("Target:            {:?}", explanation.target);
        println!("Complexity:        {:.3}", explanation.complexity);
        println!(
            "Claude available:  {} (input to engine)",
            explanation.claude_available
        );
        println!("Claude enabled:    {}", explanation.claude_enabled);
        println!("Claude model:      {}", explanation.claude_model);
        println!("Compiled rules:    {}", explanation.rule_count);
        match &explanation.matched_rule {
            Some(rule) => {
                println!("Matched rule:      {}", rule.pattern);
                println!("  configured as:   {:?}", rule.configured_target);
            }
            None => {
                println!("Matched rule:      (none — auto heuristic)");
            }
        }
        println!();
        println!("Reason: {}", explanation.reason);

        if !explanation.claude_enabled {
            println!();
            println!("Note: delegation.claude_enabled is false in config.");
        }
        if !explanation.claude_available {
            println!();
            println!(
                "Note: Claude was treated as unavailable \
                 (no API key, or --claude-available=false)."
            );
        }

        Ok(())
    }
}

/// Detect whether Claude is likely available for routing: non-empty
/// `ANTHROPIC_API_KEY` env var or providers.anthropic.api_key.
#[cfg(feature = "delegate")]
fn detect_claude_available(cfg: &clawft_types::config::Config) -> bool {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY")
        && !key.is_empty()
    {
        return true;
    }
    !cfg.providers.anthropic.api_key.expose().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegate_action_debug_fields() {
        // Smoke: enum construction for clap-derived types.
        let action = DelegateAction::Debug {
            task: "deploy".into(),
            config: None,
            json: true,
            claude_available: Some(false),
        };
        match action {
            DelegateAction::Debug {
                task,
                json,
                claude_available,
                ..
            } => {
                assert_eq!(task, "deploy");
                assert!(json);
                assert_eq!(claude_available, Some(false));
            }
        }
    }
}
