//! Native poller — tokio current-thread runtime on a dedicated std::thread,
//! connects to the daemon over `clawft_rpc::DaemonClient`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use clawft_rpc::{DaemonClient, Request};
use parking_lot::RwLock;
use serde_json::Value;

use super::{now_ms, Command, Connection, Live, Snapshot};

const POLL_INTERVAL: Duration = Duration::from_millis(1000);
const LOG_TAIL: usize = 200;
const CMD_QUEUE: usize = 64;

pub(super) fn spawn() -> Arc<Live> {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<Command>(CMD_QUEUE);
    let live = Arc::new(Live {
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
                    // clippy::collapsible_if would collapse this with a
                    // `let-chains` rewrite we can't emit on stable; allow
                    // the nested shape.
                    #[allow(clippy::collapsible_if)]
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
    let started = Instant::now();
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
        s.last_tick_at_ms = Some(now_ms());
        s.last_tick_dur_ms = Some(dur.as_secs_f64() * 1000.0);
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
