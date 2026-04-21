//! Native live driver — M1.5-C refactor.
//!
//! Historically this file hosted a polling loop that called four kernel
//! RPCs every second and wrote the result into a shared `RwLock<Snapshot>`.
//! Per ADR-017 the kernel is now an [`OntologyAdapter`] emitting
//! [`StateDelta`]s into a [`Substrate`]; this module binds that
//! substrate to the legacy [`Snapshot`] shape that the egui shell still
//! reads.
//!
//! The public [`Live`] API (`snapshot`, `submit`) is unchanged — the
//! `shell/*` and `blocks/*` modules see exactly the same types they
//! did before the refactor.

use std::sync::Arc;

use clawft_rpc::{DaemonClient, Request};
use clawft_substrate::kernel::{KernelAdapter, TOPICS as KERNEL_TOPICS};
use clawft_substrate::network::{NetworkAdapter, TOPICS as NETWORK_TOPICS};
use clawft_substrate::{OntologyAdapter, Substrate};
use parking_lot::RwLock;
use serde_json::Value;

use super::{now_ms, Command, Connection, Live, Snapshot};

const CMD_QUEUE: usize = 64;
/// Cadence for reading the substrate into the legacy `Snapshot` shape
/// and surfacing to the UI. Independent of the adapter's own poll
/// rates — those are declared by each topic's `RefreshHint`.
const SNAPSHOT_MS: u64 = 250;

pub(super) fn spawn() -> Arc<Live> {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<Command>(CMD_QUEUE);
    let live = Arc::new(Live {
        inner: RwLock::new(Snapshot::default()),
        cmd_tx,
        substrate: parking_lot::Mutex::new(None),
    });
    let driver = Arc::clone(&live);
    std::thread::Builder::new()
        .name("clawft-gui-poller".into())
        .spawn(move || run_driver(driver, cmd_rx))
        .expect("failed to spawn poller thread");
    live
}

fn run_driver(live: Arc<Live>, mut cmd_rx: tokio::sync::mpsc::Receiver<Command>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async move {
        // Build the substrate + subscribe the kernel adapter to every
        // declared topic. Subscriptions outlive this function — the
        // per-topic poller tasks run until the process exits OR until
        // `Live::drop` calls `substrate.close_all()` (see live.rs).
        let substrate = Arc::new(Substrate::new());
        // Publish the substrate handle on the Live so the Drop impl
        // can tombstone subscriptions on shutdown.
        *live.substrate.lock() = Some(Arc::clone(&substrate));

        let kernel_adapter: Arc<dyn OntologyAdapter> = Arc::new(KernelAdapter::new());
        for topic in KERNEL_TOPICS {
            match substrate
                .subscribe_adapter(Arc::clone(&kernel_adapter), topic.path, Value::Null)
                .await
            {
                Ok(_id) => { /* subscription id tracked by Substrate */ }
                Err(e) => {
                    live.write(|s| {
                        s.last_error = Some(format!("subscribe {}: {e}", topic.path));
                    });
                }
            }
        }

        // M1.5.1b — host-local network adapter (wifi / ethernet /
        // battery). Independent of the daemon; reads /sys directly.
        let network_adapter: Arc<dyn OntologyAdapter> = Arc::new(NetworkAdapter::new());
        for topic in NETWORK_TOPICS {
            match substrate
                .subscribe_adapter(Arc::clone(&network_adapter), topic.path, Value::Null)
                .await
            {
                Ok(_id) => {}
                Err(e) => {
                    live.write(|s| {
                        s.last_error = Some(format!("subscribe {}: {e}", topic.path));
                    });
                }
            }
        }

        // Separate channel for raw UI commands (ADR-011 passthrough for
        // `blocks::terminal`). Keeps its own `DaemonClient` so the
        // substrate pollers aren't serialised behind ad-hoc calls.
        let mut raw_client: Option<DaemonClient> = None;

        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(SNAPSHOT_MS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    refresh_snapshot(&substrate, &live).await;
                }
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Command::Raw { method, params, reply } => {
                            let result = run_raw(&mut raw_client, &method, params).await;
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

/// Map the substrate's current state onto the legacy [`Snapshot`]
/// shape. This is a translation layer — once the surface composer
/// (M1.5-B) lands, shell and blocks modules will read
/// [`clawft_substrate::OntologySnapshot`] directly and this function
/// goes away.
async fn refresh_snapshot(substrate: &Arc<Substrate>, live: &Arc<Live>) {
    let snap = substrate.snapshot();

    let status = snap.get("substrate/kernel/status").cloned();
    let processes = snap
        .get("substrate/kernel/processes")
        .and_then(|v| v.as_array().cloned());
    let services = snap
        .get("substrate/kernel/services")
        .and_then(|v| v.as_array().cloned());
    let logs = snap
        .get("substrate/kernel/logs")
        .and_then(|v| v.as_array().cloned());

    // M1.5.1b — NetworkAdapter emits whole-object Replaces on each
    // periodic tick. Copy those into the legacy Snapshot so the tray
    // (which still reads `Snapshot` not `OntologySnapshot`) can render
    // real chips. Migration to tray-reads-substrate lands alongside
    // the M1.6+ workspace adapter work.
    let network_wifi = snap.get("substrate/network/wifi").cloned();
    let network_ethernet = snap.get("substrate/network/ethernet").cloned();
    let network_battery = snap.get("substrate/network/battery").cloned();

    // Heuristic: if any real data from the adapter has landed in the
    // substrate we treat the connection as Connected; otherwise the
    // daemon is either still starting up or unreachable.
    let connection = if status.is_some()
        || processes.is_some()
        || services.is_some()
        || logs.is_some()
    {
        Connection::Connected
    } else {
        Connection::Connecting
    };

    live.write(|s| {
        s.connection = connection;
        s.status = status.clone();
        s.processes = processes.clone();
        s.services = services.clone();
        s.logs = logs.clone();
        s.network_wifi = network_wifi.clone();
        s.network_ethernet = network_ethernet.clone();
        s.network_battery = network_battery.clone();
        s.tick = s.tick.wrapping_add(1);
        s.last_tick_at_ms = Some(now_ms());
        s.last_tick_dur_ms = Some(SNAPSHOT_MS as f64);
        // Keep last_error as-is; the adapter pollers are silent on
        // transient failures by design (they just reconnect).
    });
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

