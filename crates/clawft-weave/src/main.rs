//! `weaver` — WeftOS operator CLI.
//!
//! The human-facing CLI for kernel management, agent orchestration,
//! and system administration. Complement to `weft` (the agent CLI).
//!
//! # Commands
//!
//! - `weaver kernel` — Boot, status, process table, services.
//! - `weaver agent` — Spawn, stop, restart, inspect agents (planned).
//! - `weaver app` — Install, start, stop applications (planned).
//! - `weaver ipc` — Send messages, manage topics (planned).

use clap::{Parser, Subcommand};

mod client;
mod commands;
mod daemon;
mod protocol;
#[cfg(feature = "rvf-rpc")]
mod rvf_codec;
#[cfg(feature = "rvf-rpc")]
mod rvf_rpc;

/// WeftOS operator CLI.
#[derive(Parser)]
#[command(
    name = "weaver",
    about = "WeftOS operator CLI — kernel, agents, and system management",
    version,
    disable_help_subcommand = true
)]
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
    /// Kernel management (boot, status, services, processes).
    Kernel(commands::kernel_cmd::KernelArgs),

    /// Agent lifecycle management (spawn, stop, restart, inspect).
    Agent(commands::agent_cmd::AgentArgs),

    /// Application management (install, start, stop, list).
    App(commands::app_cmd::AppArgs),

    /// Cluster management (nodes, shards, health).
    Cluster(commands::cluster_cmd::ClusterArgs),

    /// Chain management (status, events, checkpoints).
    Chain(commands::chain_cmd::ChainArgs),

    /// Resource tree management (tree, inspect, stats).
    Resource(commands::resource_cmd::ResourceArgs),

    /// Cron job management (add, list, remove).
    Cron(commands::cron_cmd::CronArgs),

    /// IPC management (topics, subscribe, publish).
    Ipc(commands::ipc_cmd::IpcArgs),

    /// Interactive kernel console (boot + REPL, or attach to running kernel).
    Console(commands::console_cmd::ConsoleArgs),

    /// ECC cognitive substrate management (status, calibrate, search).
    Ecc(commands::ecc_cmd::EccArgs),

    /// Show version and build info.
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let default_filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .init();

    match cli.command {
        Commands::Kernel(args) => commands::kernel_cmd::run(args).await?,
        Commands::Agent(args) => commands::agent_cmd::run(args).await?,
        Commands::App(args) => commands::app_cmd::run(args).await?,
        Commands::Cluster(args) => commands::cluster_cmd::run(args).await?,
        Commands::Chain(args) => commands::chain_cmd::run(args).await?,
        Commands::Resource(args) => commands::resource_cmd::run(args).await?,
        Commands::Cron(args) => commands::cron_cmd::run(args).await?,
        Commands::Ipc(args) => commands::ipc_cmd::run(args).await?,
        Commands::Console(args) => commands::console_cmd::run(args).await?,
        Commands::Ecc(args) => commands::ecc_cmd::run(args).await?,
        Commands::Version => {
            println!("weaver {} (WeftOS)", env!("CARGO_PKG_VERSION"));
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
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_help_contains_binary_name() {
        let help = Cli::command().render_help().to_string();
        assert!(help.contains("weaver"));
    }
}
