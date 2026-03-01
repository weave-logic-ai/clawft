//! `weaver kernel` subcommand implementation.
//!
//! Provides kernel lifecycle and introspection commands:
//! - `weaver kernel start`    -- start the kernel daemon (persistent)
//! - `weaver kernel stop`     -- stop a running daemon
//! - `weaver kernel status`   -- kernel state, uptime, process/service counts
//! - `weaver kernel services` -- list registered services with health
//! - `weaver kernel ps`       -- list process table entries
//! - `weaver kernel logs`     -- show kernel event log
//!
//! Query commands (`status`, `services`, `ps`) connect to a running daemon
//! first. If no daemon is running, they boot an ephemeral kernel, display
//! results, and exit.

use std::sync::Arc;

use clap::{Parser, Subcommand};
use comfy_table::{presets, Table};

use clawft_kernel::{Kernel, KernelState};
use clawft_platform::NativePlatform;

use crate::client::DaemonClient;
use crate::protocol;

/// Kernel management subcommand.
#[derive(Parser)]
#[command(about = "WeftOS kernel management (start, stop, status, services, processes)")]
pub struct KernelArgs {
    /// Kernel subcommand.
    #[command(subcommand)]
    pub action: KernelAction,

    /// Config file path (overrides auto-discovery).
    #[arg(short, long, global = true)]
    pub config: Option<String>,
}

/// Kernel subcommands.
#[derive(Subcommand)]
pub enum KernelAction {
    /// Start the kernel daemon (persistent, runs in foreground).
    Start,

    /// Stop a running kernel daemon.
    Stop,

    /// Show kernel state, uptime, process count, service count.
    Status,

    /// List registered services with name, type, health status.
    Services,

    /// List process table entries.
    Ps,

    /// Show kernel event log.
    Logs {
        /// Number of recent entries to show (default: 50, 0 = all).
        #[arg(short = 'n', long, default_value = "50")]
        count: usize,

        /// Minimum log level: debug, info, warn, error.
        #[arg(short, long)]
        level: Option<String>,
    },
}

/// Run the kernel subcommand.
pub async fn run(args: KernelArgs) -> anyhow::Result<()> {
    match args.action {
        KernelAction::Start => {
            let platform = NativePlatform::new();
            let config = super::load_config(&platform, args.config.as_deref()).await?;
            let kernel_config = config.kernel.clone();
            crate::daemon::run(config, kernel_config).await?;
        }
        KernelAction::Stop => {
            let mut client = DaemonClient::connect()
                .await
                .ok_or_else(|| anyhow::anyhow!("no daemon running"))?;
            let resp = client.simple_call("kernel.shutdown").await?;
            if resp.ok {
                println!("Daemon shutdown initiated.");
            } else {
                let msg = resp.error.unwrap_or_else(|| "unknown error".into());
                anyhow::bail!("shutdown failed: {msg}");
            }
        }
        KernelAction::Status => {
            if let Some(mut client) = DaemonClient::connect().await {
                let resp = client.simple_call("kernel.status").await?;
                if resp.ok {
                    let result: protocol::KernelStatusResult =
                        serde_json::from_value(resp.result.unwrap())?;
                    print_daemon_status(&result);
                } else {
                    let msg = resp.error.unwrap_or_else(|| "unknown error".into());
                    eprintln!("daemon error: {msg}");
                }
            } else {
                eprintln!("(no daemon running — booting ephemeral kernel)\n");
                let platform = NativePlatform::new();
                let config = super::load_config(&platform, args.config.as_deref()).await?;
                let kernel = boot_or_exit(config.clone(), config.kernel.clone(), platform).await;
                print_status(&kernel);
            }
        }
        KernelAction::Services => {
            if let Some(mut client) = DaemonClient::connect().await {
                let resp = client.simple_call("kernel.services").await?;
                if resp.ok {
                    let infos: Vec<protocol::ServiceInfo> =
                        serde_json::from_value(resp.result.unwrap())?;
                    print_daemon_services(&infos);
                } else {
                    let msg = resp.error.unwrap_or_else(|| "unknown error".into());
                    eprintln!("daemon error: {msg}");
                }
            } else {
                eprintln!("(no daemon running — booting ephemeral kernel)\n");
                let platform = NativePlatform::new();
                let config = super::load_config(&platform, args.config.as_deref()).await?;
                let kernel = boot_or_exit(config.clone(), config.kernel.clone(), platform).await;
                print_services(&kernel).await;
            }
        }
        KernelAction::Ps => {
            if let Some(mut client) = DaemonClient::connect().await {
                let resp = client.simple_call("kernel.ps").await?;
                if resp.ok {
                    let entries: Vec<protocol::ProcessInfo> =
                        serde_json::from_value(resp.result.unwrap())?;
                    print_daemon_ps(&entries);
                } else {
                    let msg = resp.error.unwrap_or_else(|| "unknown error".into());
                    eprintln!("daemon error: {msg}");
                }
            } else {
                eprintln!("(no daemon running — booting ephemeral kernel)\n");
                let platform = NativePlatform::new();
                let config = super::load_config(&platform, args.config.as_deref()).await?;
                let kernel = boot_or_exit(config.clone(), config.kernel.clone(), platform).await;
                print_ps(&kernel);
            }
        }
        KernelAction::Logs { count, level } => {
            if let Some(mut client) = DaemonClient::connect().await {
                let params = protocol::LogsParams {
                    count,
                    level: level.clone(),
                };
                let req = protocol::Request::with_params(
                    "kernel.logs",
                    serde_json::to_value(params)?,
                );
                let resp = client.call(req).await?;
                if resp.ok {
                    let entries: Vec<protocol::LogEntry> =
                        serde_json::from_value(resp.result.unwrap())?;
                    print_daemon_logs(&entries);
                } else {
                    let msg = resp.error.unwrap_or_else(|| "unknown error".into());
                    eprintln!("daemon error: {msg}");
                }
            } else {
                eprintln!("(no daemon running — booting ephemeral kernel)\n");
                let platform = NativePlatform::new();
                let config = super::load_config(&platform, args.config.as_deref()).await?;
                let kernel = boot_or_exit(config.clone(), config.kernel.clone(), platform).await;
                print_event_log(&kernel, count, level.as_deref());
            }
        }
    }

    Ok(())
}

// ── Daemon-mode display (from protocol types) ─────────────────────

/// Print kernel status from a daemon response.
fn print_daemon_status(result: &protocol::KernelStatusResult) {
    let uptime_str = format_uptime(result.uptime_secs);

    println!("WeftOS Kernel Status (daemon)");
    println!("-----------------------------");
    println!("State:      {}", result.state);
    println!("Uptime:     {uptime_str}");
    println!("Processes:  {}", result.process_count);
    println!("Services:   {}", result.service_count);
    println!("Max procs:  {}", result.max_processes);
    println!("Health chk: {}s", result.health_check_interval_secs);
}

/// Print services from a daemon response.
fn print_daemon_services(infos: &[protocol::ServiceInfo]) {
    if infos.is_empty() {
        println!("No services registered.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    table.set_header(vec!["Name", "Type", "Health"]);

    for info in infos {
        table.add_row(vec![&info.name, &info.service_type, &info.health]);
    }

    println!("{table}");
}

/// Print process table from a daemon response.
fn print_daemon_ps(entries: &[protocol::ProcessInfo]) {
    if entries.is_empty() {
        println!("No agents running.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    table.set_header(vec!["PID", "Agent", "State", "Mem", "CPU", "Parent"]);

    for entry in entries {
        let mem = format_bytes(entry.memory_bytes);
        let cpu = format!("{:.1}s", entry.cpu_time_ms as f64 / 1000.0);
        let parent = entry
            .parent_pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".into());

        table.add_row(vec![
            &entry.pid.to_string(),
            &entry.agent_id,
            &entry.state,
            &mem,
            &cpu,
            &parent,
        ]);
    }

    println!("{table}");
}

/// Print log entries from a daemon response.
fn print_daemon_logs(entries: &[protocol::LogEntry]) {
    if entries.is_empty() {
        println!("No log entries.");
        return;
    }

    for entry in entries {
        let ts = &entry.timestamp[11..19]; // HH:MM:SS from ISO timestamp
        let level_tag = match entry.level.as_str() {
            "error" => "ERR ",
            "warn" => "WARN",
            "debug" => "DBG ",
            _ => "INFO",
        };
        println!("{ts} [{level_tag}] {}", entry.message);
    }
    println!("({} entries)", entries.len());
}

// ── Ephemeral-mode display (from Kernel<P>) ───────────────────────

/// Boot the kernel or exit with an error message.
async fn boot_or_exit(
    config: clawft_types::config::Config,
    kernel_config: clawft_types::config::KernelConfig,
    platform: NativePlatform,
) -> Kernel<NativePlatform> {
    match Kernel::boot(config, kernel_config, Arc::new(platform)).await {
        Ok(kernel) => kernel,
        Err(e) => {
            eprintln!("kernel boot failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Print kernel status from an ephemeral kernel.
fn print_status<P: clawft_platform::Platform>(kernel: &Kernel<P>) {
    let state_str = match kernel.state() {
        KernelState::Booting => "booting",
        KernelState::Running => "running",
        KernelState::ShuttingDown => "shutting down",
        KernelState::Halted => "halted",
    };

    let uptime_str = format_uptime(kernel.uptime().as_secs_f64());

    println!("WeftOS Kernel Status (ephemeral)");
    println!("--------------------------------");
    println!("State:      {state_str}");
    println!("Uptime:     {uptime_str}");
    println!("Processes:  {}", kernel.process_table().len());
    println!("Services:   {}", kernel.services().len());
    println!(
        "Max procs:  {}",
        kernel.kernel_config().max_processes
    );
    println!(
        "Health chk: {}s",
        kernel.kernel_config().health_check_interval_secs
    );
}

/// Print services table from an ephemeral kernel.
async fn print_services<P: clawft_platform::Platform>(kernel: &Kernel<P>) {
    let services = kernel.services().list();
    if services.is_empty() {
        println!("No services registered.");
        return;
    }

    let health_results = kernel.services().health_all().await;

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    table.set_header(vec!["Name", "Type", "Health"]);

    for (name, stype) in &services {
        let health = health_results
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| h.to_string())
            .unwrap_or_else(|| "unknown".into());

        table.add_row(vec![name.as_str(), &stype.to_string(), &health]);
    }

    println!("{table}");
}

/// Print process table from an ephemeral kernel.
fn print_ps<P: clawft_platform::Platform>(kernel: &Kernel<P>) {
    let entries = kernel.process_table().list();
    if entries.is_empty() {
        println!("No agents running.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    table.set_header(vec!["PID", "Agent", "State", "Mem", "CPU", "Parent"]);

    let mut entries = entries;
    entries.sort_by_key(|e| e.pid);

    for entry in &entries {
        let mem = format_bytes(entry.resource_usage.memory_bytes);
        let cpu = format!(
            "{:.1}s",
            entry.resource_usage.cpu_time_ms as f64 / 1000.0
        );
        let parent = entry
            .parent_pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".into());

        table.add_row(vec![
            &entry.pid.to_string(),
            &entry.agent_id,
            &entry.state.to_string(),
            &mem,
            &cpu,
            &parent,
        ]);
    }

    println!("{table}");
}

/// Print event log from an ephemeral kernel.
fn print_event_log<P: clawft_platform::Platform>(
    kernel: &Kernel<P>,
    count: usize,
    level: Option<&str>,
) {
    let event_log = kernel.event_log();

    let events = if let Some(level_str) = level {
        let min_level = match level_str {
            "debug" => clawft_kernel::LogLevel::Debug,
            "warn" | "warning" => clawft_kernel::LogLevel::Warn,
            "error" => clawft_kernel::LogLevel::Error,
            _ => clawft_kernel::LogLevel::Info,
        };
        event_log.filter_level(&min_level, count)
    } else {
        event_log.tail(count)
    };

    if events.is_empty() {
        println!("No log entries.");
        return;
    }

    for event in &events {
        let ts = &event.timestamp.format("%H:%M:%S").to_string();
        let level_tag = match event.level {
            clawft_kernel::LogLevel::Error => "ERR ",
            clawft_kernel::LogLevel::Warn => "WARN",
            clawft_kernel::LogLevel::Debug => "DBG ",
            clawft_kernel::LogLevel::Info => "INFO",
        };
        println!("{ts} [{level_tag}] {}", event.message);
    }
    println!("({} entries)", events.len());
}

// ── Shared helpers ────────────────────────────────────────────────

/// Format an uptime in seconds as a human-readable string.
fn format_uptime(secs: f64) -> String {
    let total_secs = secs as u64;
    if total_secs > 3600 {
        format!(
            "{}h {}m {}s",
            total_secs / 3600,
            (total_secs % 3600) / 60,
            total_secs % 60
        )
    } else if total_secs > 60 {
        format!("{}m {}s", total_secs / 60, total_secs % 60)
    } else {
        format!("{:.1}s", secs)
    }
}

/// Format a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes}B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_units() {
        assert_eq!(format_bytes(0), "0B");
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(1024), "1.0KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0GB");
    }

    #[test]
    fn format_uptime_units() {
        assert_eq!(format_uptime(0.5), "0.5s");
        assert_eq!(format_uptime(42.0), "42.0s");
        assert_eq!(format_uptime(90.0), "1m 30s");
        assert_eq!(format_uptime(3661.0), "1h 1m 1s");
    }

    #[test]
    fn kernel_args_parses() {
        use clap::CommandFactory;
        KernelArgs::command().debug_assert();
    }
}
