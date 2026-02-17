//! `weft` -- CLI binary for the clawft AI assistant framework.
//!
//! Provides the following subcommands:
//!
//! - `weft agent` -- Start an interactive agent session or send a single message.
//! - `weft gateway` -- Start channels + agent loop (Telegram, Slack, etc.).
//! - `weft status` -- Show configuration status and diagnostics.
//! - `weft channels` -- Inspect channel configuration status.
//! - `weft cron` -- Manage scheduled (cron) jobs.

use clap::{CommandFactory, Parser, Subcommand};

mod commands;
mod completions;
mod markdown;
mod mcp_tools;

/// clawft AI assistant CLI.
#[derive(Parser)]
#[command(name = "weft", about = "clawft AI assistant CLI", version)]
struct Cli {
    /// Enable verbose (debug-level) logging.
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Top-level subcommands.
#[derive(Subcommand)]
enum Commands {
    /// Start an interactive agent session or send a single message.
    Agent(commands::agent::AgentArgs),

    /// Start the gateway (channels + agent loop).
    Gateway(commands::gateway::GatewayArgs),

    /// Show configuration status.
    Status(commands::status::StatusArgs),

    /// Inspect channel configuration.
    Channels {
        #[command(subcommand)]
        action: ChannelsAction,
    },

    /// Manage scheduled (cron) jobs.
    Cron {
        #[command(subcommand)]
        action: CronAction,
    },

    /// Manage agent sessions.
    Sessions {
        #[command(subcommand)]
        action: SessionsCmd,
    },

    /// Read and search agent memory.
    Memory {
        #[command(subcommand)]
        action: MemoryCmd,
    },

    /// Show resolved configuration.
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },

    /// Generate shell completions.
    Completions {
        /// Shell to generate for (bash, zsh, fish, powershell).
        shell: String,
    },
}

/// Subcommands for `weft sessions`.
#[derive(Subcommand)]
enum SessionsCmd {
    /// List all sessions.
    List {
        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Inspect a specific session.
    Inspect {
        /// Session key to inspect.
        session_id: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Delete a specific session.
    Delete {
        /// Session key to delete.
        session_id: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },
}

/// Subcommands for `weft memory`.
#[derive(Subcommand)]
enum MemoryCmd {
    /// Display long-term memory (MEMORY.md).
    Show {
        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Display session history (HISTORY.md).
    History {
        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Search memory and history.
    Search {
        /// Search query.
        query: String,

        /// Maximum number of results.
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },
}

/// Subcommands for `weft config`.
#[derive(Subcommand)]
enum ConfigCmd {
    /// Show the full resolved configuration.
    Show {
        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Show a specific configuration section.
    Section {
        /// Section name (e.g., "agents", "gateway", "channels").
        name: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },
}

/// Subcommands for `weft channels`.
#[derive(Subcommand)]
enum ChannelsAction {
    /// Show channel status table.
    Status {
        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },
}

/// Subcommands for `weft cron`.
#[derive(Subcommand)]
enum CronAction {
    /// List all cron jobs.
    List {
        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Add a new cron job.
    Add {
        /// Human-readable job name.
        #[arg(long)]
        name: String,

        /// Cron expression (e.g. "0 9 * * Mon-Fri").
        #[arg(long)]
        schedule: String,

        /// Agent prompt to execute when the job fires.
        #[arg(long)]
        prompt: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Remove a cron job by ID.
    Remove {
        /// Job ID to remove.
        job_id: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Enable a cron job.
    Enable {
        /// Job ID to enable.
        job_id: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Disable a cron job.
    Disable {
        /// Job ID to disable.
        job_id: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Manually trigger a cron job.
    Run {
        /// Job ID to run.
        job_id: String,

        /// Config file path (overrides auto-discovery).
        #[arg(short, long)]
        config: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let default_filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .init();

    match cli.command {
        Commands::Agent(args) => commands::agent::run(args).await?,
        Commands::Gateway(args) => commands::gateway::run(args).await?,
        Commands::Status(args) => commands::status::run(args).await?,
        Commands::Channels { action } => {
            let platform = clawft_platform::NativePlatform::new();
            match action {
                ChannelsAction::Status { config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::channels::channels_status(&cfg);
                }
            }
        }
        Commands::Cron { action } => {
            let platform = clawft_platform::NativePlatform::new();
            match action {
                CronAction::List { config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::cron::cron_list(&cfg)?;
                }
                CronAction::Add { name, schedule, prompt, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::cron::cron_add(name, schedule, prompt, &cfg)?;
                }
                CronAction::Remove { job_id, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::cron::cron_remove(job_id, &cfg)?;
                }
                CronAction::Enable { job_id, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::cron::cron_enable(job_id, true, &cfg)?;
                }
                CronAction::Disable { job_id, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::cron::cron_enable(job_id, false, &cfg)?;
                }
                CronAction::Run { job_id, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::cron::cron_run(job_id, &cfg)?;
                }
            }
        }
        Commands::Sessions { action } => {
            let platform = clawft_platform::NativePlatform::new();
            match action {
                SessionsCmd::List { config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::sessions::sessions_list(&cfg).await?;
                }
                SessionsCmd::Inspect { session_id, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::sessions::sessions_inspect(session_id, &cfg).await?;
                }
                SessionsCmd::Delete { session_id, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::sessions::sessions_delete(session_id, &cfg).await?;
                }
            }
        }
        Commands::Memory { action } => {
            let platform = clawft_platform::NativePlatform::new();
            match action {
                MemoryCmd::Show { config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::memory_cmd::memory_show(&cfg).await?;
                }
                MemoryCmd::History { config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::memory_cmd::memory_history(&cfg).await?;
                }
                MemoryCmd::Search { query, limit, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::memory_cmd::memory_search(&query, limit, &cfg).await?;
                }
            }
        }
        Commands::Config { action } => {
            let platform = clawft_platform::NativePlatform::new();
            match action {
                ConfigCmd::Show { config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::config_cmd::config_show(&cfg);
                }
                ConfigCmd::Section { name, config } => {
                    let cfg = commands::load_config(&platform, config.as_deref()).await?;
                    commands::config_cmd::config_section(&cfg, &name);
                }
            }
        }
        Commands::Completions { shell } => {
            match completions::Shell::from_str(&shell) {
                Some(s) => {
                    let mut cmd = Cli::command();
                    completions::generate_completions(&s, &mut cmd);
                }
                None => {
                    eprintln!("unsupported shell: {shell}");
                    eprintln!("supported: {}", completions::Shell::all_names().join(", "));
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses_without_error() {
        // Verify the clap derive macro produces a valid command structure.
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_help_contains_binary_name() {
        let help = Cli::command().render_help().to_string();
        assert!(help.contains("weft"));
    }

    #[test]
    fn cli_has_all_subcommands() {
        let cmd = Cli::command();
        let sub_names: Vec<&str> = cmd
            .get_subcommands()
            .map(|s| s.get_name())
            .collect();
        assert!(sub_names.contains(&"agent"));
        assert!(sub_names.contains(&"gateway"));
        assert!(sub_names.contains(&"status"));
        assert!(sub_names.contains(&"channels"));
        assert!(sub_names.contains(&"cron"));
        assert!(sub_names.contains(&"sessions"));
        assert!(sub_names.contains(&"memory"));
        assert!(sub_names.contains(&"config"));
        assert!(sub_names.contains(&"completions"));
    }

    #[test]
    fn cli_verbose_flag_is_global() {
        // --verbose before subcommand should parse correctly.
        let result = Cli::try_parse_from(["weft", "--verbose", "status"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn cli_agent_subcommand_parses_message() {
        let result = Cli::try_parse_from([
            "weft", "agent", "--message", "hello world",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_agent_subcommand_parses_model() {
        let result = Cli::try_parse_from([
            "weft", "agent", "--model", "openai/gpt-4",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_gateway_subcommand_parses_config() {
        let result = Cli::try_parse_from([
            "weft", "gateway", "--config", "/tmp/config.json",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_status_detailed_flag() {
        let result = Cli::try_parse_from(["weft", "status", "--detailed"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_channels_status_parses() {
        let result = Cli::try_parse_from(["weft", "channels", "status"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_channels_status_with_config() {
        let result = Cli::try_parse_from([
            "weft", "channels", "status", "--config", "/tmp/config.json",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_cron_list_parses() {
        let result = Cli::try_parse_from(["weft", "cron", "list"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_cron_add_parses() {
        let result = Cli::try_parse_from([
            "weft", "cron", "add",
            "--name", "daily report",
            "--schedule", "0 9 * * *",
            "--prompt", "Generate report",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_cron_remove_parses() {
        let result = Cli::try_parse_from(["weft", "cron", "remove", "job-123"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_cron_enable_parses() {
        let result = Cli::try_parse_from(["weft", "cron", "enable", "job-123"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_cron_disable_parses() {
        let result = Cli::try_parse_from(["weft", "cron", "disable", "job-123"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_cron_run_parses() {
        let result = Cli::try_parse_from(["weft", "cron", "run", "job-123"]);
        assert!(result.is_ok());
    }
}
