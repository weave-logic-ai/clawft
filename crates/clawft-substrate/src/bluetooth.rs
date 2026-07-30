//! `bluetooth` reference adapter — host-local bluetooth controller state.
//!
//! Reads `/sys/class/bluetooth/*` (controller presence) and
//! `/sys/class/rfkill/*` (soft-block state) directly. No bluetoothd,
//! no bluez DBus, no external binaries. Same scope shape as
//! [`crate::network`] — minimum honest replacement for the hardcoded
//! tray placeholder.
//!
//! # Platform support (WEFT-420)
//!
//! | Host | Behaviour |
//! |------|-----------|
//! | **Linux** | Real sysfs sampling. No controller → `present: false` (no platform cause). |
//! | **macOS / Windows / other** | Adapter still **opens**. Emits `present: false, enabled: false` with `platform: "unsupported"` and `cause: "sysfs-unavailable"`, and surfaces [`crate::health::AdapterHealthEvent::Error`] on `substrate/meta/adapter/bluetooth/health`. |
//! | **wasm32** | Module is not compiled. |
//!
//! IOBluetooth / WinRT ports are **unscheduled**. See [`crate::sysfs`].
//!
//! ## Topic
//!
//! | Topic | Shape | Refresh | Emits |
//! |-------|-------|---------|-------|
//! | `substrate/bluetooth` | `{present, enabled, controller?, platform?, cause?}` | 5s | `present` = any `/sys/class/bluetooth/hci*` exists; `enabled` = controller present AND no rfkill soft-block for type `bluetooth` |
//!
//! [`Sensitivity::Public`]; no [`PermissionReq`]. Scanning / paired
//! device enumeration carries user content and is deferred to an
//! explicit follow-up past M1.5.1c.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::adapter::{
    AdapterError, BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
    Subscription, TopicDecl,
};
use crate::delta::StateDelta;
use crate::sysfs::{
    linux_sysfs_native, sysfs_unavailable_health_delta, unsupported_payload_fields,
};

const CHAN_SINGLETON: usize = 1;

/// Declared topic — single `substrate/bluetooth` singleton.
pub const TOPICS: &[TopicDecl] = &[TopicDecl {
    path: "substrate/bluetooth",
    shape: "ontology://bluetooth",
    refresh_hint: RefreshHint::Periodic { ms: 5000 },
    sensitivity: Sensitivity::Public,
    buffer_policy: BufferPolicy::Refuse,
    max_len: None,
}];

/// Permissions — adapter reads only kernel-exposed sysfs entries;
/// no install-time grant required.
pub const PERMISSIONS: &[PermissionReq] = &[];

type CancelTx = oneshot::Sender<()>;

struct Registry {
    next_id: u64,
    live: HashMap<SubId, CancelTx>,
}

impl Registry {
    fn new() -> Self {
        Self {
            next_id: 1,
            live: HashMap::new(),
        }
    }

    fn allocate(&mut self) -> SubId {
        let id = SubId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

/// Host-local bluetooth adapter.
///
/// On non-Linux hosts (or when constructed with
/// [`Self::with_roots_and_platform`] `platform_supported = false`),
/// pollers emit disabled placeholders + adapter-health error with
/// `sysfs-unavailable` (WEFT-420).
pub struct BluetoothAdapter {
    reg: Mutex<Registry>,
    bt_root: PathBuf,
    rfkill_root: PathBuf,
    /// When `false`, skip sysfs sampling and emit the unsupported
    /// platform fallback. See [`NetworkAdapter`](crate::network::NetworkAdapter).
    platform_supported: bool,
}

impl Default for BluetoothAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothAdapter {
    /// Build with real `/sys` roots and the compile-time platform gate.
    pub fn new() -> Self {
        Self::with_roots_and_platform(
            PathBuf::from("/sys/class/bluetooth"),
            PathBuf::from("/sys/class/rfkill"),
            linux_sysfs_native(),
        )
    }

    /// Construct with arbitrary filesystem roots — used by unit tests
    /// to feed canned sysfs directories. Marks platform **supported**.
    pub fn with_roots(bt_root: PathBuf, rfkill_root: PathBuf) -> Self {
        Self::with_roots_and_platform(bt_root, rfkill_root, true)
    }

    /// Like [`Self::with_roots`], but control the platform gate
    /// (`false` → non-Linux fallback for WEFT-420 tests).
    pub fn with_roots_and_platform(
        bt_root: PathBuf,
        rfkill_root: PathBuf,
        platform_supported: bool,
    ) -> Self {
        Self {
            reg: Mutex::new(Registry::new()),
            bt_root,
            rfkill_root,
            platform_supported,
        }
    }

    /// Whether this instance will sample sysfs (vs unsupported fallback).
    pub fn platform_supported(&self) -> bool {
        self.platform_supported
    }
}

#[async_trait]
impl OntologyAdapter for BluetoothAdapter {
    fn id(&self) -> &'static str {
        "bluetooth"
    }

    fn topics(&self) -> &'static [TopicDecl] {
        TOPICS
    }

    fn permissions(&self) -> &'static [PermissionReq] {
        PERMISSIONS
    }

    async fn open(&self, topic: &str, _args: Value) -> Result<Subscription, AdapterError> {
        if topic != "substrate/bluetooth" {
            return Err(AdapterError::UnknownTopic(topic.into()));
        }
        let id = {
            let mut reg = self.reg.lock();
            reg.allocate()
        };
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let (tx, rx) = mpsc::channel::<StateDelta>(CHAN_SINGLETON);
        self.reg.lock().live.insert(id, cancel_tx);

        let bt_root = self.bt_root.clone();
        let rfkill_root = self.rfkill_root.clone();
        let platform_supported = self.platform_supported;
        tokio::spawn(async move {
            poll_bluetooth(bt_root, rfkill_root, platform_supported, id, tx, cancel_rx).await;
        });
        Ok(Subscription { id, rx })
    }

    async fn close(&self, sub_id: SubId) -> Result<(), AdapterError> {
        let _ = self.reg.lock().live.remove(&sub_id);
        Ok(())
    }
}

async fn poll_bluetooth(
    bt_root: PathBuf,
    rfkill_root: PathBuf,
    platform_supported: bool,
    sub_id: SubId,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    // Surface non-Linux once up front on adapter-health (WEFT-420 AC).
    if !platform_supported {
        let health =
            sysfs_unavailable_health_delta("bluetooth", "substrate/bluetooth", Some(sub_id));
        if tx.send(health).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = ticker.tick() => {
                let value = if !platform_supported {
                    sample_unsupported()
                } else {
                    sample_bluetooth(&bt_root, &rfkill_root)
                };
                let delta = StateDelta::Replace {
                    path: "substrate/bluetooth".to_string(),
                    value,
                };
                if tx.send(delta).await.is_err() {
                    return;
                }
            }
        }
    }
}

/// Disabled placeholders with platform cause fields (non-Linux).
fn sample_unsupported() -> Value {
    let mut obj = unsupported_payload_fields();
    obj.insert("present".into(), json!(false));
    obj.insert("enabled".into(), json!(false));
    Value::Object(obj)
}

/// Build the bluetooth state object. Returns:
///   `{"present": bool, "enabled": bool, "controller"?: "hci0"}`
fn sample_bluetooth(bt_root: &Path, rfkill_root: &Path) -> Value {
    let controller = first_controller(bt_root);
    let present = controller.is_some();
    let soft_blocked = bluetooth_rfkill_blocked(rfkill_root);
    // `enabled` means: a controller exists AND the rfkill soft-block
    // (if any exists for bluetooth) is cleared. If there's no rfkill
    // entry for bluetooth we can't tell, so we trust the presence of
    // the controller (kernel would not expose `hci*` if the stack is
    // hard-blocked).
    let enabled = present && !soft_blocked;
    let mut obj = serde_json::Map::new();
    obj.insert("present".into(), json!(present));
    obj.insert("enabled".into(), json!(enabled));
    if let Some(c) = controller {
        obj.insert("controller".into(), json!(c));
    }
    Value::Object(obj)
}

fn first_controller(bt_root: &Path) -> Option<String> {
    let entries = std::fs::read_dir(bt_root).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("hci") {
            return Some(name);
        }
    }
    None
}

/// True when any `/sys/class/rfkill/rfkill*` of type `bluetooth` has
/// `soft=1`. False when rfkill is absent or no bluetooth entry exists.
fn bluetooth_rfkill_blocked(rfkill_root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(rfkill_root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let type_ok = std::fs::read_to_string(path.join("type"))
            .map(|s| s.trim() == "bluetooth")
            .unwrap_or(false);
        if !type_ok {
            continue;
        }
        let soft = std::fs::read_to_string(path.join("soft"))
            .map(|s| s.trim() == "1")
            .unwrap_or(false);
        if soft {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sysfs::CAUSE_SYSFS_UNAVAILABLE;
    use std::fs;
    use tempfile::TempDir;

    fn fake_bt_root_with_hci(dir: &TempDir, hci: &str) -> PathBuf {
        let root = dir.path().join("bluetooth");
        fs::create_dir_all(root.join(hci)).unwrap();
        root
    }

    fn fake_bt_root_empty(dir: &TempDir) -> PathBuf {
        let root = dir.path().join("bluetooth");
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn fake_rfkill_root_with_bt_soft(dir: &TempDir, soft: &str) -> PathBuf {
        let root = dir.path().join("rfkill");
        let entry = root.join("rfkill0");
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("type"), "bluetooth").unwrap();
        fs::write(entry.join("soft"), soft).unwrap();
        root
    }

    fn fake_rfkill_root_empty(dir: &TempDir) -> PathBuf {
        let root = dir.path().join("rfkill");
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn bluetooth_absent_when_no_controller() {
        let dir = TempDir::new().unwrap();
        let bt = fake_bt_root_empty(&dir);
        let rf = fake_rfkill_root_empty(&dir);
        let v = sample_bluetooth(&bt, &rf);
        assert_eq!(v["present"], false);
        assert_eq!(v["enabled"], false);
        assert!(v.get("controller").is_none());
        assert!(v.get("cause").is_none());
    }

    #[test]
    fn bluetooth_present_and_enabled_with_hci0_and_no_rfkill() {
        let dir = TempDir::new().unwrap();
        let bt = fake_bt_root_with_hci(&dir, "hci0");
        let rf = fake_rfkill_root_empty(&dir);
        let v = sample_bluetooth(&bt, &rf);
        assert_eq!(v["present"], true);
        assert_eq!(v["enabled"], true);
        assert_eq!(v["controller"], "hci0");
    }

    #[test]
    fn bluetooth_soft_blocked_sets_enabled_false() {
        let dir = TempDir::new().unwrap();
        let bt = fake_bt_root_with_hci(&dir, "hci0");
        let rf = fake_rfkill_root_with_bt_soft(&dir, "1");
        let v = sample_bluetooth(&bt, &rf);
        assert_eq!(v["present"], true);
        assert_eq!(v["enabled"], false);
    }

    #[test]
    fn bluetooth_soft_cleared_remains_enabled() {
        let dir = TempDir::new().unwrap();
        let bt = fake_bt_root_with_hci(&dir, "hci0");
        let rf = fake_rfkill_root_with_bt_soft(&dir, "0");
        let v = sample_bluetooth(&bt, &rf);
        assert_eq!(v["enabled"], true);
    }

    #[test]
    fn sample_unsupported_carries_sysfs_cause() {
        let v = sample_unsupported();
        assert_eq!(v["present"], false);
        assert_eq!(v["enabled"], false);
        assert_eq!(v["cause"], CAUSE_SYSFS_UNAVAILABLE);
        assert_eq!(v["platform"], "unsupported");
        assert!(v.get("controller").is_none());
    }

    #[test]
    fn new_respects_compile_time_platform_gate() {
        let a = BluetoothAdapter::new();
        assert_eq!(a.platform_supported(), linux_sysfs_native());
    }

    #[tokio::test]
    async fn adapter_open_unknown_topic_errors() {
        let a = BluetoothAdapter::new();
        let r = a.open("substrate/bogus", Value::Null).await;
        assert!(matches!(r, Err(AdapterError::UnknownTopic(_))));
    }

    #[tokio::test]
    async fn unsupported_platform_emits_health_error_then_disabled_payload() {
        // WEFT-420: force non-Linux path even with hci0 present in tree.
        let dir = TempDir::new().unwrap();
        let a = BluetoothAdapter::with_roots_and_platform(
            fake_bt_root_with_hci(&dir, "hci0"),
            fake_rfkill_root_empty(&dir),
            false,
        );
        let mut sub = a
            .open("substrate/bluetooth", Value::Null)
            .await
            .expect("open");

        let health = tokio::time::timeout(Duration::from_secs(2), sub.rx.recv())
            .await
            .expect("timeout waiting health")
            .expect("channel closed");
        match health {
            StateDelta::Replace { path, value } => {
                assert_eq!(path, "substrate/meta/adapter/bluetooth/health");
                assert_eq!(value["event"], "error");
                let reason = value["reason"].as_str().unwrap_or("");
                assert!(
                    reason.contains(CAUSE_SYSFS_UNAVAILABLE),
                    "reason={reason}"
                );
            }
            other => panic!("expected health Replace, got {other:?}"),
        }

        let payload = tokio::time::timeout(Duration::from_secs(6), sub.rx.recv())
            .await
            .expect("timeout waiting payload")
            .expect("channel closed");
        match payload {
            StateDelta::Replace { path, value } => {
                assert_eq!(path, "substrate/bluetooth");
                assert_eq!(value["present"], false);
                assert_eq!(value["enabled"], false);
                assert_eq!(value["cause"], CAUSE_SYSFS_UNAVAILABLE);
                assert!(value.get("controller").is_none());
            }
            other => panic!("expected bluetooth Replace, got {other:?}"),
        }
    }
}
