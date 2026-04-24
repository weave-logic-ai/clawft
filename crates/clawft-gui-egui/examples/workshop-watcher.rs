//! Workshop TOML watcher — the second-most important half of Track 5.
//!
//! Watches a local TOML file; on change, parses it into the Workshop
//! JSON shape described in
//! `crates/clawft-gui-egui/src/explorer/workshop.rs` and publishes the
//! result to `substrate/ui/workshop/<name>` via the running daemon's
//! Unix-socket RPC (`substrate.publish`).
//!
//! This is what makes the hot-reload loop actually *feel* live: edit
//! a TOML on disk, save, and the Explorer's Workshop pane reconfigures
//! within ~1s. No daemon restart, no GUI rebuild.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example workshop-watcher -- \
//!     --toml examples/example-workshop.toml \
//!     --workshop mic-diagnostic
//! ```
//!
//! With `--once`, publishes a single snapshot and exits (useful for
//! scripting / CI / initial-seeding).
//!
//! ## Debounce
//!
//! Filesystem events for a single save often arrive in bursts (editors
//! write-then-rename; some emit MODIFY + CLOSE_WRITE pairs). A 100ms
//! debounce collapses them into one publish — the `notify` crate's
//! recommended watcher gives us precise events but not coalescence.
//!
//! ## Graceful failure modes
//!
//! * Daemon not running — retries the next tick; log once per state
//!   transition so we don't spam.
//! * TOML parse error — log the message, keep the old value on the
//!   daemon (no destructive publish of a broken shape).
//! * File missing — wait for it to appear.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clap::Parser;
use clawft_rpc::{DaemonClient, Request};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;

/// File-change debounce window. Saves from editors like vim
/// (`:w` → rename + write) often emit two events within ~40ms; 100ms
/// is the smallest window that reliably collapses them to one publish
/// on my machine without feeling laggy when triggered by hand.
const DEBOUNCE: Duration = Duration::from_millis(100);

/// Retry cadence when publishing fails (daemon not running, socket
/// gone, …). We don't want to hammer the socket on every tick.
const RETRY_BACKOFF: Duration = Duration::from_secs(2);

#[derive(Parser, Debug)]
#[command(
    name = "workshop-watcher",
    about = "Watch a TOML file; publish it as a Workshop value to substrate."
)]
struct Args {
    /// Path to the TOML file describing the Workshop.
    #[arg(long)]
    toml: PathBuf,

    /// Workshop name — appended to `substrate/ui/workshop/` to form
    /// the publish path.
    #[arg(long)]
    workshop: String,

    /// Publish once and exit. Useful for scripts / CI / seeding.
    #[arg(long, default_value_t = false)]
    once: bool,

    /// Override the substrate publish path entirely. When set,
    /// `--workshop` is only used for log decoration.
    #[arg(long)]
    path: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let publish_path = args
        .path
        .clone()
        .unwrap_or_else(|| format!("substrate/ui/workshop/{}", args.workshop));

    eprintln!(
        "[workshop-watcher] toml={} path={} mode={}",
        args.toml.display(),
        publish_path,
        if args.once { "once" } else { "watch" },
    );

    // Build the tokio runtime once; publishes reuse it.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Initial publish — the --once short-circuit.
    match read_and_convert(&args.toml) {
        Ok(value) => {
            if let Err(e) = rt.block_on(publish(&publish_path, value)) {
                eprintln!("[workshop-watcher] initial publish failed: {e}");
                if args.once {
                    std::process::exit(1);
                }
            } else {
                eprintln!("[workshop-watcher] initial publish ok");
            }
        }
        Err(e) => {
            eprintln!("[workshop-watcher] initial read failed: {e}");
            if args.once {
                std::process::exit(1);
            }
        }
    }

    if args.once {
        return Ok(());
    }

    // Set up the file watcher. `RecommendedWatcher` picks the best
    // backend per-OS (inotify on Linux, FSEvents on macOS, etc.).
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(tx)?;

    // Watch the parent directory rather than the file itself — many
    // editors replace the file (rename-over) on save, which detaches a
    // file-level watch. Directory watches survive that.
    let watch_dir = args
        .toml
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

    eprintln!("[workshop-watcher] watching {}", watch_dir.display());

    let canonical_target = args
        .toml
        .canonicalize()
        .unwrap_or_else(|_| args.toml.clone());

    // Debounce + retry loop. `last_event_at` tracks when we last saw
    // a relevant change; we publish at most once per debounce window.
    let mut pending: Option<Instant> = None;
    loop {
        // Wait for either a filesystem event or the debounce timer
        // expiring.
        let timeout = pending
            .map(|when| {
                let elapsed = when.elapsed();
                if elapsed >= DEBOUNCE {
                    Duration::ZERO
                } else {
                    DEBOUNCE - elapsed
                }
            })
            .unwrap_or(Duration::from_millis(500));

        match rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                if is_relevant(&event, &canonical_target) {
                    pending = Some(Instant::now());
                }
            }
            Ok(Err(e)) => {
                eprintln!("[workshop-watcher] watch error: {e}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Fall through; below we check if a pending publish
                // is now past its debounce deadline.
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("watcher channel disconnected");
            }
        }

        if let Some(when) = pending
            && when.elapsed() >= DEBOUNCE
        {
            pending = None;
            match read_and_convert(&args.toml) {
                Ok(value) => match rt.block_on(publish(&publish_path, value)) {
                    Ok(tick) => {
                        eprintln!("[workshop-watcher] publish ok tick={tick}");
                    }
                    Err(e) => {
                        eprintln!("[workshop-watcher] publish failed: {e}");
                        std::thread::sleep(RETRY_BACKOFF);
                    }
                },
                Err(e) => {
                    eprintln!("[workshop-watcher] parse failed: {e}");
                }
            }
        }
    }
}

/// Is this event relevant to our target file? Filters out unrelated
/// sibling files in the watched directory and noise events (access,
/// metadata-only).
fn is_relevant(event: &Event, target: &Path) -> bool {
    match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {}
        _ => return false,
    }
    // Any path in the event that resolves to the target matches. We
    // canonicalize both sides so `./foo.toml` and `/abs/foo.toml`
    // compare equal.
    event.paths.iter().any(|p| {
        let canonical = p.canonicalize().unwrap_or_else(|_| p.clone());
        canonical == target || p == target
    })
}

/// Read the TOML file and convert it to the Workshop JSON shape.
/// Accepts either a bare TOML document (matching the Workshop schema)
/// or a nested `[workshop]` table — both are common conventions.
fn read_and_convert(toml_path: &Path) -> anyhow::Result<Value> {
    let text = std::fs::read_to_string(toml_path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", toml_path.display()))?;
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|e| anyhow::anyhow!("toml parse: {e}"))?;

    // If the document has a `[workshop]` table, unwrap it; otherwise
    // treat the whole document as the Workshop.
    let workshop_value: toml::Value = match parsed {
        toml::Value::Table(mut table) => {
            if let Some(w) = table.remove("workshop") {
                w
            } else {
                toml::Value::Table(table)
            }
        }
        other => other,
    };

    // Serialize through serde_json to land in the wire shape the
    // daemon expects.
    let json_value: Value = serde_json::to_value(workshop_value)
        .map_err(|e| anyhow::anyhow!("toml→json: {e}"))?;

    // Sanity: must be an object. The parsed Workshop side will
    // reject deeper mistakes; here we short-circuit on the most
    // obvious one so the publish log is actionable.
    if !json_value.is_object() {
        anyhow::bail!("workshop TOML must produce a JSON object at top level");
    }
    Ok(json_value)
}

/// Publish a Workshop value via the running daemon. Returns the new
/// tick on success.
///
/// **Known issue (post node-identity gate):** the daemon's
/// `substrate.publish` now requires a registered `node_id` +
/// signed `node_publish_payload(...)`. This example currently
/// sends an unsigned publish, so it will fail at runtime with
/// "node_id required" until migrated. Migration plan: call
/// `node.register` once at startup with a persistent local
/// keypair, then sign each publish over the canonical payload.
/// Tracked in the gate-slice handoff; not a blocker for the
/// gate flip itself since this is a developer tool.
async fn publish(path: &str, value: Value) -> anyhow::Result<u64> {
    let mut client = DaemonClient::connect()
        .await
        .ok_or_else(|| anyhow::anyhow!("no daemon running (is `weft daemon` up?)"))?;
    let params = serde_json::json!({
        "path": path,
        "value": value,
    });
    let resp = client
        .call(Request::with_params("substrate.publish", params))
        .await?;
    if !resp.ok {
        anyhow::bail!(
            "substrate.publish: {}",
            resp.error.unwrap_or_else(|| "unknown error".into())
        );
    }
    let result = resp.result.unwrap_or(Value::Null);
    let tick = result.get("tick").and_then(Value::as_u64).unwrap_or(0);
    Ok(tick)
}
