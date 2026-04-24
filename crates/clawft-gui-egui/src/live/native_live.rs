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
use clawft_substrate::bluetooth::{BluetoothAdapter, TOPICS as BLUETOOTH_TOPICS};
use clawft_substrate::chain::{ChainAdapter, TOPICS as CHAIN_TOPICS};
use clawft_substrate::kernel::{KernelAdapter, TOPICS as KERNEL_TOPICS};
use clawft_substrate::mesh::{MeshAdapter, TOPICS as MESH_TOPICS};
use clawft_substrate::mic::{MicrophoneAdapter, TOPICS as MIC_TOPICS};
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

        // M1.5.1c — host-local bluetooth adapter. Same pattern as
        // the network adapter; reads /sys/class/bluetooth and
        // /sys/class/rfkill.
        let bluetooth_adapter: Arc<dyn OntologyAdapter> = Arc::new(BluetoothAdapter::new());
        for topic in BLUETOOTH_TOPICS {
            match substrate
                .subscribe_adapter(Arc::clone(&bluetooth_adapter), topic.path, Value::Null)
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

        // M1.5.1d — mesh + chain adapters that poll existing daemon
        // RPC verbs (cluster.status, cluster.nodes, chain.status) and
        // project into substrate/{mesh,chain}/*. Replaces the tray's
        // `service_present(snap, [...])` heuristic with real topology
        // data.
        let mesh_adapter: Arc<dyn OntologyAdapter> = Arc::new(MeshAdapter::new());
        for topic in MESH_TOPICS {
            match substrate
                .subscribe_adapter(Arc::clone(&mesh_adapter), topic.path, Value::Null)
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
        let chain_adapter: Arc<dyn OntologyAdapter> = Arc::new(ChainAdapter::new());
        for topic in CHAIN_TOPICS {
            match substrate
                .subscribe_adapter(Arc::clone(&chain_adapter), topic.path, Value::Null)
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

        // Audio input adapter — file-backed RMS/peak metering into
        // `substrate/sensor/mic`. Source path defaults to
        // `/tmp/weftos/mic/stream.raw`; until a CPAL bridge lands that
        // file will be absent on a fresh machine, in which case the
        // adapter emits `{available: false, reason: "source-missing"}`
        // and the Audio chip's detail window renders an "unavailable"
        // hint instead of a lie.
        let mic_adapter: Arc<dyn OntologyAdapter> = Arc::new(MicrophoneAdapter::new());
        for topic in MIC_TOPICS {
            match substrate
                .subscribe_adapter(Arc::clone(&mic_adapter), topic.path, Value::Null)
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

        // Relay poller: for paths that are *published into the daemon's
        // SubstrateService from outside* (e.g. a Windows-side bridge
        // calling `substrate.publish`), no local OntologyAdapter is
        // running. Poll the daemon's `substrate.read` for those paths
        // and inject a Replace into the local substrate so the legacy
        // `Snapshot` shape sees them. 250ms cadence matches SNAPSHOT_MS.
        let mut relay_client: Option<DaemonClient> = None;
        const RELAYED_PATHS: &[&str] = &[
            "substrate/sensor/tof",
            "substrate/sensor/mic",
        ];

        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(SNAPSHOT_MS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    relay_external_paths(
                        &substrate,
                        &mut relay_client,
                        RELAYED_PATHS,
                    )
                    .await;
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
    let bluetooth = snap.get("substrate/bluetooth").cloned();
    let mesh_status = snap.get("substrate/mesh/status").cloned();
    let chain_status = snap.get("substrate/chain/status").cloned();
    let audio_mic = snap.get("substrate/sensor/mic").cloned();
    let tof_depth = snap.get("substrate/sensor/tof").cloned();

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
        s.bluetooth = bluetooth.clone();
        s.mesh_status = mesh_status.clone();
        s.chain_status = chain_status.clone();
        s.audio_mic = audio_mic.clone();
        s.tof_depth = tof_depth.clone();
        s.tick = s.tick.wrapping_add(1);
        s.last_tick_at_ms = Some(now_ms());
        s.last_tick_dur_ms = Some(SNAPSHOT_MS as f64);
        // Keep last_error as-is; the adapter pollers are silent on
        // transient failures by design (they just reconnect).
    });
}

/// Poll the daemon's `substrate.read` for each listed path and inject
/// the current value into the local substrate as a `StateDelta::Replace`.
///
/// Purpose: bridge daemon-side publishes (via `substrate.publish` RPC
/// from external producers like a Windows-side ESP32 bridge) into the
/// local substrate the GUI reads from. Local OntologyAdapters don't
/// exist for these paths; this relay is how externally-produced data
/// reaches the panels.
///
/// Failures are silent — we treat them like any other adapter hiccup.
/// If the daemon isn't running or the path has no value yet, we skip
/// and retry next tick.
async fn relay_external_paths(
    substrate: &Arc<clawft_substrate::Substrate>,
    client_opt: &mut Option<DaemonClient>,
    paths: &[&str],
) {
    if client_opt.is_none() {
        *client_opt = DaemonClient::connect().await;
    }
    let Some(client) = client_opt.as_mut() else {
        return;
    };
    for path in paths {
        let params = serde_json::json!({ "path": path });
        let resp = match client
            .call(Request::with_params("substrate.read", params))
            .await
        {
            Ok(r) => r,
            Err(_) => {
                // Connection dropped — null it so next tick reconnects.
                *client_opt = None;
                return;
            }
        };
        if !resp.ok {
            continue;
        }
        let result = resp.result.unwrap_or(Value::Null);
        // Expected shape from substrate.read:
        // `{value: Option<Value>, tick: u64, sensitivity: String}`.
        if let Some(value) = result.get("value").cloned()
            && !value.is_null() {
                substrate.apply(clawft_substrate::StateDelta::Replace {
                    path: path.to_string(),
                    value,
                });
            }
    }
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

