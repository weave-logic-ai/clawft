//! Kernel daemon — persistent kernel process with Unix socket RPC.
//!
//! The daemon boots a [`Kernel`], then listens on a Unix domain socket
//! for JSON-RPC requests. This is the native transport layer; the
//! kernel itself is platform-agnostic and could be wrapped in
//! WebSocket, TCP, or `postMessage` for other environments.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use clawft_kernel::{Kernel, KernelState};
use clawft_platform::NativePlatform;
use clawft_types::config::{Config, KernelConfig};

use crate::protocol::{
    self, KernelStatusResult, LogEntry, LogsParams, ProcessInfo, Request, Response, ServiceInfo,
};

/// Run the kernel daemon.
///
/// Boots the kernel, binds to a Unix socket, and serves requests
/// until shutdown is requested (via `kernel.shutdown` RPC or signal).
pub async fn run(config: Config, kernel_config: KernelConfig) -> anyhow::Result<()> {
    let socket_path = protocol::socket_path();

    // Clean up stale socket file
    if socket_path.exists() {
        // Try connecting to see if a daemon is already running
        if tokio::net::UnixStream::connect(&socket_path)
            .await
            .is_ok()
        {
            anyhow::bail!(
                "daemon already running (socket exists and is accepting connections: {})",
                socket_path.display()
            );
        }
        // Stale socket — remove it
        std::fs::remove_file(&socket_path)?;
        debug!("removed stale socket file");
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Boot kernel
    let platform = NativePlatform::new();
    let kernel = Kernel::boot(config, kernel_config, Arc::new(platform)).await?;
    let kernel = Arc::new(tokio::sync::RwLock::new(kernel));

    // Print boot banner
    {
        let k = kernel.read().await;
        print!("{}", clawft_kernel::console::boot_banner());
        print!("{}", k.boot_log().format_all());
    }

    // Bind socket
    let listener = UnixListener::bind(&socket_path)?;
    info!(path = %socket_path.display(), "daemon listening");
    println!("Daemon listening on {}", socket_path.display());

    // Log daemon start to kernel event log
    {
        let k = kernel.read().await;
        k.event_log()
            .info("daemon", format!("listening on {}", socket_path.display()));
    }

    // Shutdown signal
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Accept loop — clone shutdown_tx so the outer scope can still use it for Ctrl+C
    let accept_kernel = Arc::clone(&kernel);
    let rpc_shutdown_tx = shutdown_tx.clone();
    let mut accept_handle = tokio::spawn(async move {
        let mut shutdown_rx = shutdown_rx;
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let k = Arc::clone(&accept_kernel);
                            let tx = rpc_shutdown_tx.clone();
                            tokio::spawn(handle_connection(stream, k, tx));
                        }
                        Err(e) => {
                            error!("accept error: {e}");
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("shutdown signal received, stopping accept loop");
                        break;
                    }
                }
            }
        }
    });

    // Wait for either Ctrl+C or the accept loop to finish (RPC shutdown).
    let ctrl_c_triggered = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received, shutting down daemon");
            let _ = shutdown_tx.send(true);
            true
        }
        _ = &mut accept_handle => {
            // Accept loop finished (shutdown requested via RPC)
            info!("accept loop finished (RPC shutdown)");
            false
        }
    };

    // If Ctrl+C triggered, wait for the accept loop to finish.
    // If it finished on its own (RPC shutdown), the handle is already consumed.
    if ctrl_c_triggered {
        let _ = accept_handle.await;
    }

    // Shut down kernel
    {
        let mut k = kernel.write().await;
        if let Err(e) = k.shutdown().await {
            warn!("kernel shutdown error: {e}");
        }
    }

    // Clean up socket
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    println!("Daemon stopped.");
    Ok(())
}

/// Handle a single client connection.
async fn handle_connection(
    stream: tokio::net::UnixStream,
    kernel: Arc<tokio::sync::RwLock<Kernel<NativePlatform>>>,
    shutdown_tx: watch::Sender<bool>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_owned();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                let id = req.id.clone();
                dispatch(
                    req.method,
                    req.params,
                    Arc::clone(&kernel),
                    shutdown_tx.clone(),
                )
                .await
                .with_id(id)
            }
            Err(e) => Response::error(format!("invalid request: {e}")),
        };

        let mut json = serde_json::to_string(&response).unwrap_or_else(|e| {
            serde_json::to_string(&Response::error(format!("serialize error: {e}"))).unwrap()
        });
        json.push('\n');

        if let Err(e) = writer.write_all(json.as_bytes()).await {
            debug!("write error (client disconnected?): {e}");
            break;
        }
    }
}

/// Dispatch a request to the appropriate handler.
///
/// Takes owned values to ensure the future is `Send + 'static`
/// for use inside `tokio::spawn`.
async fn dispatch(
    method: String,
    params: serde_json::Value,
    kernel: Arc<tokio::sync::RwLock<Kernel<NativePlatform>>>,
    shutdown_tx: watch::Sender<bool>,
) -> Response {
    match method.as_str() {
        "kernel.status" => {
            let k = kernel.read().await;
            let state_str = match k.state() {
                KernelState::Booting => "booting",
                KernelState::Running => "running",
                KernelState::ShuttingDown => "shutting_down",
                KernelState::Halted => "halted",
            };
            let result = KernelStatusResult {
                state: state_str.to_owned(),
                uptime_secs: k.uptime().as_secs_f64(),
                process_count: k.process_table().len(),
                service_count: k.services().len(),
                max_processes: k.kernel_config().max_processes,
                health_check_interval_secs: k.kernel_config().health_check_interval_secs,
            };
            Response::success(serde_json::to_value(result).unwrap())
        }
        "kernel.ps" => {
            let k = kernel.read().await;
            let mut entries: Vec<ProcessInfo> = k
                .process_table()
                .list()
                .iter()
                .map(|e| ProcessInfo {
                    pid: e.pid,
                    agent_id: e.agent_id.clone(),
                    state: e.state.to_string(),
                    memory_bytes: e.resource_usage.memory_bytes,
                    cpu_time_ms: e.resource_usage.cpu_time_ms,
                    parent_pid: e.parent_pid,
                })
                .collect();
            entries.sort_by_key(|e| e.pid);
            Response::success(serde_json::to_value(entries).unwrap())
        }
        "kernel.services" => {
            let k = kernel.read().await;
            let services = k.services().list();
            let infos: Vec<ServiceInfo> = services
                .iter()
                .map(|(name, stype)| ServiceInfo {
                    name: name.clone(),
                    service_type: stype.to_string(),
                    health: "registered".into(),
                })
                .collect();
            Response::success(serde_json::to_value(infos).unwrap())
        }
        "kernel.logs" => {
            let log_params: LogsParams = serde_json::from_value(params).unwrap_or(LogsParams {
                count: 50,
                level: None,
            });

            let k = kernel.read().await;
            let event_log = k.event_log();

            let events = if let Some(ref level_str) = log_params.level {
                let level = match level_str.as_str() {
                    "debug" => clawft_kernel::LogLevel::Debug,
                    "warn" | "warning" => clawft_kernel::LogLevel::Warn,
                    "error" => clawft_kernel::LogLevel::Error,
                    _ => clawft_kernel::LogLevel::Info,
                };
                event_log.filter_level(&level, log_params.count)
            } else {
                event_log.tail(log_params.count)
            };

            let entries: Vec<LogEntry> = events
                .iter()
                .map(|e| LogEntry {
                    timestamp: e.timestamp.to_rfc3339(),
                    phase: e.phase.tag().to_owned(),
                    level: format!("{:?}", e.level).to_lowercase(),
                    message: e.message.clone(),
                })
                .collect();

            Response::success(serde_json::to_value(entries).unwrap())
        }
        "kernel.shutdown" => {
            // Log the shutdown event before signaling
            {
                let k = kernel.read().await;
                k.event_log().info("daemon", "shutdown requested via RPC");
            }
            info!("shutdown requested via RPC");
            let _ = shutdown_tx.send(true);
            Response::success(serde_json::json!("shutting down"))
        }
        "ping" => Response::success(serde_json::json!("pong")),
        other => Response::error(format!("unknown method: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_resolves() {
        let path = crate::protocol::socket_path();
        assert!(path.to_string_lossy().ends_with("kernel.sock"));
    }
}
