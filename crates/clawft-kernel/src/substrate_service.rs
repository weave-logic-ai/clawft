//! Kernel-side substrate service.
//!
//! Provides a minimal, daemon-hosted implementation of the four
//! substrate RPCs (`substrate.read`, `substrate.publish`,
//! `substrate.subscribe`, `substrate.notify`). Each path carries:
//!
//! - a most-recent `serde_json::Value` (the state),
//! - a monotonic tick that advances on every write,
//! - a declared [`crate::topic::SubscriberSink`] set for fan-out,
//! - a declared [`Sensitivity`] (Public / Workspace / Private /
//!   Capture) consulted by the policy gate.
//!
//! The service is intentionally in-memory. It replaces the ad-hoc
//! markdown + file-backed adapter hacks described in weftos-0.7 by
//! giving external clients a real pub/sub+read/write surface to the
//! kernel's substrate.
//!
//! # Egress gating
//!
//! All reads/subscribes pass through [`SubstrateService::egress_check`]
//! — the single seam where a future governance policy will gate
//! `Capture`-tier topics on per-caller capability grants. For M1.5
//! bring-up this is a log-but-allow stub so adapters can land before
//! the policy layer is live.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::topic::{SubscriberId, SubscriberSink};

/// Privacy sensitivity of a substrate path.
///
/// Mirrors `clawft_substrate::Sensitivity` intentionally so adapter
/// code can share the classification vocabulary. Kept local here to
/// avoid a kernel → substrate crate dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    /// Safe to display anywhere; no prompt required.
    #[default]
    Public,
    /// Scoped to the current workspace/project.
    Workspace,
    /// Personal data beyond the workspace.
    Private,
    /// Derived from ambient capture (camera/mic/screen).
    Capture,
}

impl Sensitivity {
    /// Whether this level requires an authenticated caller for
    /// read / subscribe operations. `Capture` (and higher, if ever
    /// added) always requires; the rest allow anonymous reads for
    /// bring-up.
    pub fn requires_caller_identity(self) -> bool {
        matches!(self, Sensitivity::Capture)
    }

    /// Short text label, for log lines and error messages.
    pub fn as_str(self) -> &'static str {
        match self {
            Sensitivity::Public => "public",
            Sensitivity::Workspace => "workspace",
            Sensitivity::Private => "private",
            Sensitivity::Capture => "capture",
        }
    }
}

/// One substrate path's state.
struct Entry {
    value: Option<Value>,
    tick: u64,
    sensitivity: Sensitivity,
    sinks: Vec<(SubscriberId, SubscriberSink)>,
}

impl Entry {
    fn new(sensitivity: Sensitivity) -> Self {
        Self {
            value: None,
            tick: 0,
            sensitivity,
            sinks: Vec::new(),
        }
    }
}

/// Snapshot of a substrate read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateReadSnapshot {
    /// Current value at the path; `None` if never written.
    pub value: Option<Value>,
    /// Monotonic tick for the path.
    pub tick: u64,
    /// Declared sensitivity.
    pub sensitivity: Sensitivity,
}

/// Reason an egress check denied access.
#[derive(Debug, Clone)]
pub struct EgressDenied {
    /// Short failure reason.
    pub reason: String,
}

impl std::fmt::Display for EgressDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "egress denied: {}", self.reason)
    }
}

/// Substrate RPC service.
#[derive(Clone)]
pub struct SubstrateService {
    inner: Arc<SubstrateInner>,
}

struct SubstrateInner {
    entries: DashMap<String, Entry>,
    global_tick: AtomicU64,
}

impl Default for SubstrateService {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstrateService {
    /// Create a new, empty service.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SubstrateInner {
                entries: DashMap::new(),
                global_tick: AtomicU64::new(0),
            }),
        }
    }

    /// Declare (or re-declare) the sensitivity level for a path.
    ///
    /// If the path already exists, the declaration is updated in
    /// place without clearing the value. Paths not declared default
    /// to [`Sensitivity::Workspace`] on first touch.
    pub fn declare(&self, path: &str, sensitivity: Sensitivity) {
        let mut entry = self
            .inner
            .entries
            .entry(path.to_string())
            .or_insert_with(|| Entry::new(sensitivity));
        entry.sensitivity = sensitivity;
    }

    /// Egress gate stub.
    ///
    /// For bring-up: log every `Capture` read/subscribe but allow
    /// it. A future governance commit wires this to the
    /// capability-grant layer. This is intentionally the *one* seam
    /// the policy will gate, so callers need not change.
    pub fn egress_check(
        &self,
        caller: Option<&str>,
        path: &str,
        op: &str,
    ) -> Result<(), EgressDenied> {
        let sensitivity = self
            .inner
            .entries
            .get(path)
            .map(|e| e.sensitivity)
            .unwrap_or(Sensitivity::Workspace);

        if sensitivity.requires_caller_identity() && caller.is_none() {
            return Err(EgressDenied {
                reason: format!(
                    "{op} on {path} (sensitivity={}) requires authenticated caller",
                    sensitivity.as_str()
                ),
            });
        }

        if sensitivity == Sensitivity::Capture {
            warn!(
                path,
                caller = ?caller,
                op,
                "substrate egress: capture-tier path accessed (allow-all stub)"
            );
        } else {
            debug!(path, caller = ?caller, op, sensitivity = sensitivity.as_str(), "substrate egress ok");
        }
        Ok(())
    }

    /// Read the current value + metadata at a path.
    pub fn read(&self, caller: Option<&str>, path: &str) -> Result<SubstrateReadSnapshot, EgressDenied> {
        self.egress_check(caller, path, "read")?;
        let snapshot = match self.inner.entries.get(path) {
            Some(e) => SubstrateReadSnapshot {
                value: e.value.clone(),
                tick: e.tick,
                sensitivity: e.sensitivity,
            },
            None => SubstrateReadSnapshot {
                value: None,
                tick: 0,
                sensitivity: Sensitivity::Workspace,
            },
        };
        Ok(snapshot)
    }

    /// Publish a Replace delta at a path. Returns the new tick.
    ///
    /// Fans out to all external-stream subscribers registered via
    /// [`Self::subscribe`]. For bring-up, the owner-check is trusted
    /// — the daemon layer should gate on its own agent role policy.
    pub fn publish(&self, caller: Option<&str>, path: &str, value: Value) -> u64 {
        let new_tick = self.inner.global_tick.fetch_add(1, Ordering::Relaxed) + 1;
        let line = build_update_line(path, Some(&value), new_tick, "publish", caller);

        let mut entry = self
            .inner
            .entries
            .entry(path.to_string())
            .or_insert_with(|| Entry::new(Sensitivity::Workspace));
        entry.value = Some(value);
        entry.tick = new_tick;
        fanout(&mut entry.sinks, &line);
        new_tick
    }

    /// Emit a notify-only signal (no payload change) on a path.
    /// Returns the current tick (unchanged if no write has happened).
    pub fn notify(&self, caller: Option<&str>, path: &str) -> u64 {
        let new_tick = self.inner.global_tick.fetch_add(1, Ordering::Relaxed) + 1;
        let line = build_update_line(path, None, new_tick, "notify", caller);

        let mut entry = self
            .inner
            .entries
            .entry(path.to_string())
            .or_insert_with(|| Entry::new(Sensitivity::Workspace));
        entry.tick = new_tick;
        fanout(&mut entry.sinks, &line);
        new_tick
    }

    /// Subscribe an external streaming sink to updates on a path.
    ///
    /// Returns the subscriber id and a channel receiver. The caller
    /// pipes the JSON lines the sink receives into their socket.
    pub fn subscribe(
        &self,
        caller: Option<&str>,
        path: &str,
    ) -> Result<(SubscriberId, mpsc::Receiver<Vec<u8>>), EgressDenied> {
        self.egress_check(caller, path, "subscribe")?;
        let (tx, rx) = mpsc::channel::<Vec<u8>>(256);
        let mut entry = self
            .inner
            .entries
            .entry(path.to_string())
            .or_insert_with(|| Entry::new(Sensitivity::Workspace));
        let id = SubscriberId::next();
        entry.sinks.push((id, SubscriberSink::ExternalStream(tx)));
        Ok((id, rx))
    }

    /// Remove a specific subscription. Safe if the id is unknown.
    pub fn unsubscribe(&self, path: &str, id: SubscriberId) {
        if let Some(mut entry) = self.inner.entries.get_mut(path) {
            entry.sinks.retain(|(existing, _)| *existing != id);
        }
    }

    /// Number of declared paths (for metrics / tests).
    pub fn path_count(&self) -> usize {
        self.inner.entries.len()
    }
}

fn fanout(sinks: &mut Vec<(SubscriberId, SubscriberSink)>, line: &[u8]) {
    let mut dead = Vec::new();
    for (id, sink) in sinks.iter() {
        match sink {
            SubscriberSink::ExternalStream(tx) => {
                if tx.try_send(line.to_vec()).is_err() {
                    // closed or full — prune closed ones
                    if tx.is_closed() {
                        dead.push(*id);
                    }
                }
            }
            SubscriberSink::PidInbox(_) => {
                // PID delivery isn't wired for substrate today.
                // The kernel a2a router handles in-process fanout.
            }
        }
    }
    if !dead.is_empty() {
        sinks.retain(|(id, _)| !dead.contains(id));
    }
}

fn build_update_line(
    path: &str,
    value: Option<&Value>,
    tick: u64,
    kind: &str,
    caller: Option<&str>,
) -> Vec<u8> {
    let body = serde_json::json!({
        "path": path,
        "tick": tick,
        "kind": kind,
        "value": value,
        "actor_id": caller,
    });
    let mut bytes = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    bytes.push(b'\n');
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_then_read_roundtrip() {
        let svc = SubstrateService::new();
        let t = svc.publish(None, "substrate/test/ping", serde_json::json!(42));
        let snap = svc.read(None, "substrate/test/ping").unwrap();
        assert_eq!(snap.value, Some(serde_json::json!(42)));
        assert_eq!(snap.tick, t);
    }

    #[test]
    fn read_unknown_path_returns_empty() {
        let svc = SubstrateService::new();
        let snap = svc.read(None, "nope").unwrap();
        assert!(snap.value.is_none());
        assert_eq!(snap.tick, 0);
    }

    #[tokio::test]
    async fn subscribe_receives_publish_and_notify() {
        let svc = SubstrateService::new();
        let (_id, mut rx) = svc.subscribe(None, "substrate/test/ping").unwrap();
        svc.publish(None, "substrate/test/ping", serde_json::json!("hi"));
        svc.notify(None, "substrate/test/ping");

        let line1 = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let v1: serde_json::Value = serde_json::from_slice(&line1[..line1.len() - 1]).unwrap();
        assert_eq!(v1["kind"], "publish");

        let line2 = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let v2: serde_json::Value = serde_json::from_slice(&line2[..line2.len() - 1]).unwrap();
        assert_eq!(v2["kind"], "notify");
        assert!(v2["value"].is_null());
    }

    #[test]
    fn egress_check_denies_capture_anonymous() {
        let svc = SubstrateService::new();
        svc.declare("substrate/mic/frames", Sensitivity::Capture);
        let err = svc.read(None, "substrate/mic/frames").unwrap_err();
        assert!(err.reason.contains("requires authenticated"));
    }

    #[test]
    fn egress_check_allows_capture_with_identity() {
        let svc = SubstrateService::new();
        svc.declare("substrate/mic/frames", Sensitivity::Capture);
        assert!(svc.read(Some("aid-1"), "substrate/mic/frames").is_ok());
    }

    #[test]
    fn tick_monotonic_across_ops() {
        let svc = SubstrateService::new();
        let a = svc.publish(None, "p", serde_json::json!(1));
        let b = svc.notify(None, "p");
        let c = svc.publish(None, "p", serde_json::json!(2));
        assert!(b > a);
        assert!(c > b);
    }
}
