//! `weft kernel` subcommand implementation.
//!
//! Provides kernel introspection commands:
//! - `weft kernel status` -- kernel state, uptime, process/service counts
//! - `weft kernel services` -- list registered services with health
//! - `weft kernel ps` -- list process table entries

use std::sync::Arc;

use clap::{Parser, Subcommand};
use comfy_table::{presets, Table};

use clawft_kernel::{Kernel, KernelState};
use clawft_platform::NativePlatform;

/// Kernel management subcommand.
#[derive(Parser)]
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
    /// Show kernel state, uptime, process count, service count.
    Status,

    /// List registered services with name, type, health status.
    Services,

    /// List process table entries.
    Ps,

    /// Boot the kernel (non-interactive, foreground).
    Boot {
        /// Run in foreground with log output (no REPL).
        #[arg(long)]
        foreground: bool,
    },
}

/// Run the kernel subcommand.
pub async fn run(args: KernelArgs) -> anyhow::Result<()> {
    let platform = NativePlatform::new();
    let config = super::load_config(&platform, args.config.as_deref()).await?;
    let kernel_config = config.kernel.clone();

    match args.action {
        KernelAction::Status => {
            // Boot the kernel briefly to get status
            let kernel = Kernel::boot(config, kernel_config, Arc::new(platform)).await;
            match kernel {
                Ok(kernel) => {
                    print_status(&kernel);
                }
                Err(e) => {
                    eprintln!("kernel boot failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        KernelAction::Services => {
            let kernel = Kernel::boot(config, kernel_config, Arc::new(platform)).await;
            match kernel {
                Ok(kernel) => {
                    print_services(&kernel).await;
                }
                Err(e) => {
                    eprintln!("kernel boot failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        KernelAction::Ps => {
            let kernel = Kernel::boot(config, kernel_config, Arc::new(platform)).await;
            match kernel {
                Ok(kernel) => {
                    print_ps(&kernel);
                }
                Err(e) => {
                    eprintln!("kernel boot failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        KernelAction::Boot { foreground } => {
            let kernel = Kernel::boot(config, kernel_config, Arc::new(platform)).await;
            match kernel {
                Ok(kernel) => {
                    // Print boot log
                    print!("{}", clawft_kernel::console::boot_banner());
                    print!("{}", kernel.boot_log().format_all());

                    if foreground {
                        println!("\nKernel running in foreground. Press Ctrl+C to stop.");
                        // Wait indefinitely until interrupted
                        tokio::signal::ctrl_c().await?;
                        println!("\nShutting down...");
                    }
                }
                Err(e) => {
                    eprintln!("kernel boot failed: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

/// Print kernel status summary.
fn print_status<P: clawft_platform::Platform>(kernel: &Kernel<P>) {
    let state_str = match kernel.state() {
        KernelState::Booting => "booting",
        KernelState::Running => "running",
        KernelState::ShuttingDown => "shutting down",
        KernelState::Halted => "halted",
    };

    let uptime = kernel.uptime();
    let uptime_str = if uptime.as_secs() > 3600 {
        format!(
            "{}h {}m {}s",
            uptime.as_secs() / 3600,
            (uptime.as_secs() % 3600) / 60,
            uptime.as_secs() % 60
        )
    } else if uptime.as_secs() > 60 {
        format!(
            "{}m {}s",
            uptime.as_secs() / 60,
            uptime.as_secs() % 60
        )
    } else {
        format!("{:.1}s", uptime.as_secs_f64())
    };

    println!("WeftOS Kernel Status");
    println!("--------------------");
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

/// Print services table.
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

/// Print process table.
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
        let cpu = format!("{:.1}s", entry.resource_usage.cpu_time_ms as f64 / 1000.0);
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
    fn kernel_args_parses() {
        use clap::CommandFactory;
        KernelArgs::command().debug_assert();
    }
}
