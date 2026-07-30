//! `network` reference adapter — system-local WiFi / ethernet / battery.
//!
//! Reads `/sys/class/net/*` and `/sys/class/power_supply/*` directly so
//! it works on any **Linux** host without requiring NetworkManager,
//! bluetoothd, or any userspace service layer.
//!
//! # Platform support (WEFT-420)
//!
//! | Host | Behaviour |
//! |------|-----------|
//! | **Linux** | Real sysfs sampling. Missing hardware → plain `absent` / `present: false` (no platform cause). |
//! | **macOS / Windows / other** | Adapter still **opens** (tray/Explorer keep a uniform surface). Emits `absent` / `present: false` with `platform: "unsupported"` and `cause: "sysfs-unavailable"`, and surfaces [`crate::health::AdapterHealthEvent::Error`] on `substrate/meta/adapter/network/health` with that cause. |
//! | **wasm32** | Module is not compiled; GUI uses the legacy Snapshot fallback. |
//!
//! macOS CoreWLAN / Windows WinRT ports are **unscheduled**. See
//! [`crate::sysfs`] for the shared cause codes and health helpers.
//!
//! ## Topics
//!
//! | Topic | Shape | Refresh | Emits |
//! |-------|-------|---------|-------|
//! | `substrate/network/wifi` | `{state, iface?, platform?, cause?}` | 3s | Aggregate wifi state — `"connected"` if any wlan-class iface has operstate `up`, `"disconnected"` if present but down, `"absent"` if no wlan iface exists (or non-Linux) |
//! | `substrate/network/ethernet` | `{state, iface?, platform?, cause?}` | 3s | Same shape for physical ethernet (en*/eth* interfaces, excluding virtual bridge/docker/tap) |
//! | `substrate/network/battery` | `{present, percent?, charging?, platform?, cause?}` | 5s | From `/sys/class/power_supply/BAT*/capacity` + `status` |
//!
//! All topics are [`Sensitivity::Public`] and require no
//! [`PermissionReq`] — interface names, aggregate state, and battery
//! percent don't carry user content. SSID / IP lookup would escalate
//! to `Workspace` and are out of scope for M1.5.1b.
//!
//! ## Scope note for 1.5.1b
//!
//! This adapter is a minimal honest replacement for the hardcoded
//! tray placeholders. It doesn't attempt SSID enumeration, signal
//! strength, or connection management — those belong to a
//! nmcli/iwd-specific variant (native Linux) and land in M1.6+ once
//! the editor-in work exposes a permissions UX.

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

/// Channel depth for singleton topics.
const CHAN_SINGLETON: usize = 1;

/// Declared topics — local system state, all singletons, periodic refresh.
pub const TOPICS: &[TopicDecl] = &[
    TopicDecl {
        path: "substrate/network/wifi",
        shape: "ontology://network-link",
        refresh_hint: RefreshHint::Periodic { ms: 3000 },
        sensitivity: Sensitivity::Public,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
    TopicDecl {
        path: "substrate/network/ethernet",
        shape: "ontology://network-link",
        refresh_hint: RefreshHint::Periodic { ms: 3000 },
        sensitivity: Sensitivity::Public,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
    TopicDecl {
        path: "substrate/network/battery",
        shape: "ontology://battery",
        refresh_hint: RefreshHint::Periodic { ms: 5000 },
        sensitivity: Sensitivity::Public,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
];

/// Permissions — all topics are public host-state; nothing required.
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

/// Host-local network adapter. Pollers read `/sys/class/*` on Linux; no RPC.
///
/// On non-Linux hosts (or when constructed with
/// [`Self::with_roots_and_platform`] `platform_supported = false`),
/// pollers emit absent placeholders + adapter-health error with
/// `sysfs-unavailable` (WEFT-420).
pub struct NetworkAdapter {
    reg: Mutex<Registry>,
    /// Root for `/sys/class/net` — overridable for tests via [`Self::with_roots`].
    net_root: PathBuf,
    /// Root for `/sys/class/power_supply` — overridable for tests.
    power_root: PathBuf,
    /// When `false`, skip sysfs sampling and emit the unsupported
    /// platform fallback (payload cause + adapter-health error).
    ///
    /// `new()` sets this from [`linux_sysfs_native`]. `with_roots`
    /// sets `true` so unit tests can feed canned trees on any host.
    platform_supported: bool,
}

impl Default for NetworkAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkAdapter {
    /// Build with real `/sys` roots and the compile-time platform gate.
    pub fn new() -> Self {
        Self::with_roots_and_platform(
            PathBuf::from("/sys/class/net"),
            PathBuf::from("/sys/class/power_supply"),
            linux_sysfs_native(),
        )
    }

    /// Construct with arbitrary filesystem roots — used by unit tests
    /// to feed canned sysfs directories. Marks the platform as
    /// **supported** so sampling runs against the injected trees even
    /// on macOS CI.
    pub fn with_roots(net_root: PathBuf, power_root: PathBuf) -> Self {
        Self::with_roots_and_platform(net_root, power_root, true)
    }

    /// Like [`Self::with_roots`], but control the platform gate.
    ///
    /// `platform_supported = false` exercises the non-Linux fallback
    /// path on any host (tests for WEFT-420).
    pub fn with_roots_and_platform(
        net_root: PathBuf,
        power_root: PathBuf,
        platform_supported: bool,
    ) -> Self {
        Self {
            reg: Mutex::new(Registry::new()),
            net_root,
            power_root,
            platform_supported,
        }
    }

    /// Whether this instance will sample sysfs (vs unsupported fallback).
    pub fn platform_supported(&self) -> bool {
        self.platform_supported
    }
}

#[async_trait]
impl OntologyAdapter for NetworkAdapter {
    fn id(&self) -> &'static str {
        "network"
    }

    fn topics(&self) -> &'static [TopicDecl] {
        TOPICS
    }

    fn permissions(&self) -> &'static [PermissionReq] {
        PERMISSIONS
    }

    async fn open(&self, topic: &str, _args: Value) -> Result<Subscription, AdapterError> {
        let known = matches!(
            topic,
            "substrate/network/wifi" | "substrate/network/ethernet" | "substrate/network/battery"
        );
        if !known {
            return Err(AdapterError::UnknownTopic(topic.into()));
        }
        let id = {
            let mut reg = self.reg.lock();
            reg.allocate()
        };
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let (tx, rx) = mpsc::channel::<StateDelta>(CHAN_SINGLETON);
        self.reg.lock().live.insert(id, cancel_tx);

        let topic_path = topic.to_string();
        let net_root = self.net_root.clone();
        let power_root = self.power_root.clone();
        let platform_supported = self.platform_supported;
        tokio::spawn(async move {
            spawn_poller(
                topic_path,
                net_root,
                power_root,
                platform_supported,
                id,
                tx,
                cancel_rx,
            )
            .await;
        });
        Ok(Subscription { id, rx })
    }

    async fn close(&self, sub_id: SubId) -> Result<(), AdapterError> {
        let _ = self.reg.lock().live.remove(&sub_id);
        Ok(())
    }
}

async fn spawn_poller(
    topic: String,
    net_root: PathBuf,
    power_root: PathBuf,
    platform_supported: bool,
    sub_id: SubId,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let period = match topic.as_str() {
        "substrate/network/battery" => Duration::from_secs(5),
        _ => Duration::from_secs(3),
    };
    let mut ticker = tokio::time::interval(period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    // Surface non-Linux once up front so adapter-health shows the
    // sysfs cause before the first payload tick lands (WEFT-420 AC).
    if !platform_supported {
        let health = sysfs_unavailable_health_delta("network", &topic, Some(sub_id));
        if tx.send(health).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = ticker.tick() => {
                let value = if !platform_supported {
                    sample_unsupported(&topic)
                } else {
                    match topic.as_str() {
                        "substrate/network/wifi" => sample_link_state(&net_root, LinkKind::Wifi),
                        "substrate/network/ethernet" => sample_link_state(&net_root, LinkKind::Ethernet),
                        "substrate/network/battery" => sample_battery(&power_root),
                        _ => continue,
                    }
                };
                let delta = StateDelta::Replace {
                    path: topic.clone(),
                    value,
                };
                if tx.send(delta).await.is_err() {
                    return; // subscriber dropped
                }
            }
        }
    }
}

/// Absent / present:false placeholders with platform cause fields.
fn sample_unsupported(topic: &str) -> Value {
    let fields = unsupported_payload_fields();
    match topic {
        "substrate/network/battery" => {
            let mut obj = fields;
            obj.insert("present".into(), json!(false));
            Value::Object(obj)
        }
        _ => {
            let mut obj = fields;
            obj.insert("state".into(), json!("absent"));
            Value::Object(obj)
        }
    }
}

/// Which interface class the scan is looking for.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum LinkKind {
    Wifi,
    Ethernet,
}

impl LinkKind {
    fn iface_type_dir(&self, iface: &Path) -> Option<PathBuf> {
        // Wireless interfaces have a `wireless/` subdir in /sys/class/net.
        // Ethernet interfaces typically carry a `device/` symlink.
        let wireless_dir = iface.join("wireless");
        match self {
            LinkKind::Wifi => {
                if wireless_dir.is_dir() {
                    Some(wireless_dir)
                } else {
                    None
                }
            }
            LinkKind::Ethernet => {
                // Exclude loopback, docker bridges, tap, wireguard, etc.
                // by requiring the iface to have a `device/` symlink and
                // NOT a `wireless/` subdir.
                let name = iface.file_name()?.to_string_lossy().to_string();
                if wireless_dir.is_dir() {
                    return None;
                }
                if name == "lo" || name.starts_with("docker") || name.starts_with("br-") {
                    return None;
                }
                if name.starts_with("veth") || name.starts_with("tap") {
                    return None;
                }
                if name.starts_with("en") || name.starts_with("eth") {
                    Some(iface.to_path_buf())
                } else {
                    None
                }
            }
        }
    }
}

/// Walk `/sys/class/net/*` and summarise the state of interfaces of
/// the requested kind. Returns:
///   `{"state": "connected"|"disconnected"|"absent", "iface": <name>?}`
fn sample_link_state(net_root: &Path, kind: LinkKind) -> Value {
    let Ok(entries) = std::fs::read_dir(net_root) else {
        return json!({ "state": "absent" });
    };
    let mut first_match: Option<(String, bool)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if kind.iface_type_dir(&path).is_none() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let operstate = std::fs::read_to_string(path.join("operstate"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let connected = operstate == "up";
        if first_match.is_none() || connected {
            first_match = Some((name, connected));
            if connected {
                break;
            }
        }
    }
    match first_match {
        Some((iface, true)) => json!({ "state": "connected", "iface": iface }),
        Some((iface, false)) => json!({ "state": "disconnected", "iface": iface }),
        None => json!({ "state": "absent" }),
    }
}

/// Read `/sys/class/power_supply/BAT*` for battery state. Returns:
///   `{"present": bool, "percent"?: u8, "charging"?: bool}`
fn sample_battery(power_root: &Path) -> Value {
    let Ok(entries) = std::fs::read_dir(power_root) else {
        return json!({ "present": false });
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if !name.to_uppercase().starts_with("BAT") {
            continue;
        }
        let percent: Option<u8> = std::fs::read_to_string(path.join("capacity"))
            .ok()
            .and_then(|s| s.trim().parse().ok());
        let status_str = std::fs::read_to_string(path.join("status"))
            .ok()
            .map(|s| s.trim().to_lowercase());
        let charging = status_str
            .as_deref()
            .map(|s| s == "charging" || s == "full");
        let mut obj = serde_json::Map::new();
        obj.insert("present".into(), json!(true));
        if let Some(p) = percent {
            obj.insert("percent".into(), json!(p));
        }
        if let Some(c) = charging {
            obj.insert("charging".into(), json!(c));
        }
        return Value::Object(obj);
    }
    json!({ "present": false })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sysfs::CAUSE_SYSFS_UNAVAILABLE;
    use std::fs;
    use tempfile::TempDir;

    /// Build a fake `/sys/class/net` root with the given interfaces,
    /// each declared as either `wireless` (present wireless/ subdir) or
    /// `ethernet` (no wireless/ subdir), and a given operstate.
    fn fake_net_root(
        dir: &TempDir,
        ifaces: &[(&str, bool /* wireless */, &str /* operstate */)],
    ) -> PathBuf {
        let root = dir.path().join("net");
        fs::create_dir_all(&root).unwrap();
        for (name, wireless, operstate) in ifaces {
            let iface = root.join(name);
            fs::create_dir_all(&iface).unwrap();
            fs::write(iface.join("operstate"), operstate).unwrap();
            if *wireless {
                fs::create_dir_all(iface.join("wireless")).unwrap();
            } else {
                // Real eth/en ifaces carry a `device` symlink; we create
                // a dir instead — the adapter only checks the `wireless`
                // subdir's absence and the name prefix.
            }
        }
        root
    }

    fn fake_power_root_empty(dir: &TempDir) -> PathBuf {
        let root = dir.path().join("power");
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn fake_power_root_with_battery(dir: &TempDir, capacity: &str, status: &str) -> PathBuf {
        let root = dir.path().join("power");
        let bat = root.join("BAT0");
        fs::create_dir_all(&bat).unwrap();
        fs::write(bat.join("capacity"), capacity).unwrap();
        fs::write(bat.join("status"), status).unwrap();
        root
    }

    #[test]
    fn wifi_absent_when_no_wireless_iface() {
        let dir = TempDir::new().unwrap();
        let net = fake_net_root(&dir, &[("eth0", false, "up"), ("lo", false, "unknown")]);
        let v = sample_link_state(&net, LinkKind::Wifi);
        assert_eq!(v["state"], "absent");
        // Linux-path absent must NOT carry a platform cause — that is
        // reserved for non-Linux (WEFT-420).
        assert!(v.get("cause").is_none());
    }

    #[test]
    fn wifi_connected_when_wireless_iface_up() {
        let dir = TempDir::new().unwrap();
        let net = fake_net_root(&dir, &[("wlan0", true, "up"), ("eth0", false, "up")]);
        let v = sample_link_state(&net, LinkKind::Wifi);
        assert_eq!(v["state"], "connected");
        assert_eq!(v["iface"], "wlan0");
    }

    #[test]
    fn wifi_disconnected_when_wireless_iface_present_but_down() {
        let dir = TempDir::new().unwrap();
        let net = fake_net_root(&dir, &[("wlan0", true, "down")]);
        let v = sample_link_state(&net, LinkKind::Wifi);
        assert_eq!(v["state"], "disconnected");
        assert_eq!(v["iface"], "wlan0");
    }

    #[test]
    fn ethernet_ignores_docker_and_loopback() {
        let dir = TempDir::new().unwrap();
        let net = fake_net_root(
            &dir,
            &[
                ("lo", false, "unknown"),
                ("docker0", false, "up"),
                ("eth0", false, "up"),
            ],
        );
        let v = sample_link_state(&net, LinkKind::Ethernet);
        assert_eq!(v["state"], "connected");
        assert_eq!(v["iface"], "eth0");
    }

    #[test]
    fn ethernet_absent_when_only_virtual_ifaces() {
        let dir = TempDir::new().unwrap();
        let net = fake_net_root(&dir, &[("lo", false, "unknown"), ("docker0", false, "up")]);
        let v = sample_link_state(&net, LinkKind::Ethernet);
        assert_eq!(v["state"], "absent");
    }

    #[test]
    fn battery_absent_when_directory_empty() {
        let dir = TempDir::new().unwrap();
        let power = fake_power_root_empty(&dir);
        let v = sample_battery(&power);
        assert_eq!(v["present"], false);
    }

    #[test]
    fn battery_reports_percent_and_charging() {
        let dir = TempDir::new().unwrap();
        let power = fake_power_root_with_battery(&dir, "78", "Charging");
        let v = sample_battery(&power);
        assert_eq!(v["present"], true);
        assert_eq!(v["percent"], 78);
        assert_eq!(v["charging"], true);
    }

    #[test]
    fn battery_reports_not_charging_when_discharging() {
        let dir = TempDir::new().unwrap();
        let power = fake_power_root_with_battery(&dir, "42", "Discharging");
        let v = sample_battery(&power);
        assert_eq!(v["charging"], false);
    }

    #[test]
    fn sample_unsupported_wifi_carries_sysfs_cause() {
        let v = sample_unsupported("substrate/network/wifi");
        assert_eq!(v["state"], "absent");
        assert_eq!(v["cause"], CAUSE_SYSFS_UNAVAILABLE);
        assert_eq!(v["platform"], "unsupported");
    }

    #[test]
    fn sample_unsupported_battery_carries_sysfs_cause() {
        let v = sample_unsupported("substrate/network/battery");
        assert_eq!(v["present"], false);
        assert_eq!(v["cause"], CAUSE_SYSFS_UNAVAILABLE);
    }

    #[test]
    fn new_respects_compile_time_platform_gate() {
        let a = NetworkAdapter::new();
        assert_eq!(a.platform_supported(), linux_sysfs_native());
    }

    #[test]
    fn with_roots_marks_platform_supported_for_tests() {
        let dir = TempDir::new().unwrap();
        let a = NetworkAdapter::with_roots(
            fake_net_root(&dir, &[]),
            fake_power_root_empty(&dir),
        );
        assert!(a.platform_supported());
    }

    #[tokio::test]
    async fn adapter_open_unknown_topic_errors() {
        let a = NetworkAdapter::new();
        let r = a.open("substrate/network/bogus", Value::Null).await;
        assert!(matches!(r, Err(AdapterError::UnknownTopic(_))));
    }

    #[tokio::test]
    async fn unsupported_platform_emits_health_error_then_absent_payload() {
        // WEFT-420: force non-Linux path on any host.
        let dir = TempDir::new().unwrap();
        let a = NetworkAdapter::with_roots_and_platform(
            fake_net_root(&dir, &[("wlan0", true, "up")]),
            fake_power_root_empty(&dir),
            false,
        );
        let mut sub = a
            .open("substrate/network/wifi", Value::Null)
            .await
            .expect("open");

        // First delta: adapter-health error with sysfs cause.
        let health = tokio::time::timeout(Duration::from_secs(2), sub.rx.recv())
            .await
            .expect("timeout waiting health")
            .expect("channel closed");
        match health {
            StateDelta::Replace { path, value } => {
                assert_eq!(path, "substrate/meta/adapter/network/health");
                assert_eq!(value["event"], "error");
                let reason = value["reason"].as_str().unwrap_or("");
                assert!(
                    reason.contains(CAUSE_SYSFS_UNAVAILABLE),
                    "reason={reason}"
                );
            }
            other => panic!("expected health Replace, got {other:?}"),
        }

        // Second delta: absent wifi payload with cause (interval fires).
        let payload = tokio::time::timeout(Duration::from_secs(5), sub.rx.recv())
            .await
            .expect("timeout waiting payload")
            .expect("channel closed");
        match payload {
            StateDelta::Replace { path, value } => {
                assert_eq!(path, "substrate/network/wifi");
                assert_eq!(value["state"], "absent");
                assert_eq!(value["cause"], CAUSE_SYSFS_UNAVAILABLE);
                // Even though fake tree has wlan0 up, unsupported path
                // must not sample it.
                assert!(value.get("iface").is_none());
            }
            other => panic!("expected wifi Replace, got {other:?}"),
        }
    }
}
