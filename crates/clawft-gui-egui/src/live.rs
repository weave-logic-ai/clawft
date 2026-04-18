//! Live kernel state — background poller thread that reads from the
//! daemon IPC socket and republishes snapshots for the UI.
//!
//! eframe runs on the main thread with no tokio runtime, so we own a
//! dedicated std::thread that hosts a current-thread runtime. The UI
//! clones an `Arc<Live>` and reads from `snapshot()` each frame.

use std::sync::Arc;
use std::time::Duration;

use clawft_rpc::{DaemonClient, Request};
use parking_lot::RwLock;
use serde_json::Value;

/// Tick interval for polling the kernel daemon.
const POLL_INTERVAL: Duration = Duration::from_millis(1000);
/// Number of log entries to pull per tick.
const LOG_TAIL: usize = 200;
/// Queue size for commands pushed from the UI to the poller.
const CMD_QUEUE: usize = 64;

/// Shared live state readable from any thread.
///
/// The poller thread owns the write side; UI code calls `snapshot()` to
/// get a cloned, point-in-time view. Reads are cheap (an `RwLock` plus
/// a small clone).
pub struct Live {
    inner: RwLock<Snapshot>,
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
}

/// Point-in-time view of everything the poller has learned.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub connection: Connection,
    pub status: Option<Value>,
    pub processes: Option<Vec<Value>>,
    pub services: Option<Vec<Value>>,
    pub logs: Option<Vec<Value>>,
    pub last_error: Option<String>,
    /// Incremented every successful poll tick so the UI can detect freshness.
    pub tick: u64,
    /// Wall clock time of the most recent successful poll — lets the UI
    /// show "last tick Nms ago" instead of just a counter.
    pub last_tick_at: Option<std::time::Instant>,
    /// Duration of the previous successful poll round-trip.
    pub last_tick_dur: Option<std::time::Duration>,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Connecting,
    Connected,
    Disconnected,
}

/// Commands the UI pushes to the poller (e.g. from the Terminal block).
#[derive(Debug)]
pub enum Command {
    /// Fire a raw RPC call; the response is appended to the shared
    /// `Snapshot::last_error` field on failure, and dropped on success
    /// (blocks that need the result can call the method directly).
    Raw {
        method: String,
        params: Value,
        /// Channel to deliver the response back to the caller (e.g. Terminal).
        reply: Option<tokio::sync::oneshot::Sender<Result<Value, String>>>,
    },
}

impl Live {
    /// Start the poller thread and return a handle.
    pub fn spawn() -> Arc<Self> {
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<Command>(CMD_QUEUE);
        let live = Arc::new(Self {
            inner: RwLock::new(Snapshot::default()),
            cmd_tx,
        });
        let driver = Arc::clone(&live);
        std::thread::Builder::new()
            .name("clawft-gui-poller".into())
            .spawn(move || run_poller(driver, cmd_rx))
            .expect("failed to spawn poller thread");
        live
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.read().clone()
    }

    pub fn submit(&self, cmd: Command) -> bool {
        self.cmd_tx.try_send(cmd).is_ok()
    }

    fn write(&self, mut f: impl FnMut(&mut Snapshot)) {
        f(&mut self.inner.write());
    }
}

/// Worker loop: single-threaded tokio runtime that owns the DaemonClient.
///
/// On every tick we (re)establish the connection if needed, then issue
/// the four status RPCs in sequence, accumulating the results into a new
/// `Snapshot`. Any RPC error demotes us to `Disconnected` for the next
/// tick.
fn run_poller(live: Arc<Live>, mut cmd_rx: tokio::sync::mpsc::Receiver<Command>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async move {
        let mut client: Option<DaemonClient> = None;
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if client.is_none() {
                        client = DaemonClient::connect().await;
                        live.write(|s| {
                            s.connection = if client.is_some() {
                                Connection::Connected
                            } else {
                                Connection::Disconnected
                            };
                        });
                    }
                    if let Some(c) = client.as_mut() {
                        if let Err(e) = poll_once(c, &live).await {
                            live.write(|s| {
                                s.connection = Connection::Disconnected;
                                s.last_error = Some(e.clone());
                            });
                            client = None;
                        }
                    }
                }
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Command::Raw { method, params, reply } => {
                            let result = run_raw(&mut client, &method, params).await;
                            if let Some(tx) = reply {
                                let _ = tx.send(result.clone());
                            }
                            if let Err(ref e) = result {
                                live.write(|s| s.last_error = Some(e.clone()));
                            }
                        }
                    }
                }
            }
        }
    });
}

async fn poll_once(client: &mut DaemonClient, live: &Arc<Live>) -> Result<(), String> {
    let started = std::time::Instant::now();
    let status = simple(client, "kernel.status").await?;
    let ps = simple(client, "kernel.ps").await?;
    let services = simple(client, "kernel.services").await?;
    let logs = call(
        client,
        "kernel.logs",
        serde_json::json!({ "count": LOG_TAIL }),
    )
    .await?;
    let dur = started.elapsed();

    live.write(|s| {
        s.connection = Connection::Connected;
        s.status = Some(status.clone());
        s.processes = as_array(&ps);
        s.services = as_array(&services);
        s.logs = as_array(&logs);
        s.tick = s.tick.wrapping_add(1);
        s.last_tick_at = Some(std::time::Instant::now());
        s.last_tick_dur = Some(dur);
        s.last_error = None;
    });
    Ok(())
}

async fn run_raw(
    client_opt: &mut Option<DaemonClient>,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    if client_opt.is_none() {
        *client_opt = DaemonClient::connect().await;
    }
    let c = client_opt
        .as_mut()
        .ok_or_else(|| "no daemon running".to_string())?;
    call(c, method, params).await
}

async fn simple(client: &mut DaemonClient, method: &str) -> Result<Value, String> {
    let resp = client
        .simple_call(method)
        .await
        .map_err(|e| format!("{method}: {e}"))?;
    if !resp.ok {
        return Err(format!(
            "{method}: {}",
            resp.error.unwrap_or_else(|| "unknown error".into())
        ));
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

async fn call(client: &mut DaemonClient, method: &str, params: Value) -> Result<Value, String> {
    let resp = client
        .call(Request::with_params(method, params))
        .await
        .map_err(|e| format!("{method}: {e}"))?;
    if !resp.ok {
        return Err(format!(
            "{method}: {}",
            resp.error.unwrap_or_else(|| "unknown error".into())
        ));
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

fn as_array(v: &Value) -> Option<Vec<Value>> {
    v.as_array().cloned()
}
