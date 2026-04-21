//! Bottom tray — frosted-glass bar with service status chips.

use eframe::egui;

use crate::live::{Connection, Snapshot};

pub const TRAY_HEIGHT: f32 = 42.0;

/// Abstract status of a tray service.
#[derive(Copy, Clone)]
pub enum Ok { On, Warn, Off }

impl Ok {
    fn color(self) -> egui::Color32 {
        match self {
            Ok::On => egui::Color32::from_rgb(110, 210, 160),
            Ok::Warn => egui::Color32::from_rgb(255, 205, 90),
            Ok::Off => egui::Color32::from_rgb(140, 140, 150),
        }
    }
}

/// Render the tray at the bottom of `rect`. Left cluster is the launcher
/// and clock; right cluster is the service chips.
pub fn paint(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    snap: &Snapshot,
    launcher_active: &mut bool,
) {
    let tray_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.bottom() - TRAY_HEIGHT),
        egui::vec2(rect.width(), TRAY_HEIGHT),
    );
    let painter = ui.painter_at(tray_rect);

    // Frosted glass: dark semi-transparent fill + 1px highlight at top.
    painter.rect_filled(
        tray_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(14, 14, 20, 215),
    );
    painter.line_segment(
        [tray_rect.left_top(), tray_rect.right_top()],
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
    );

    let inner_rect = tray_rect.shrink2(egui::vec2(12.0, 6.0));
    let mut tray_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
    tray_ui.horizontal_centered(|ui| {
            // ── Left: launcher ────────────────────────────────
            let btn_text = egui::RichText::new("⏾ Blocks").monospace();
            if ui.selectable_label(*launcher_active, btn_text).clicked() {
                *launcher_active = !*launcher_active;
            }

            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("WeftOS")
                    .small()
                    .color(egui::Color32::from_rgba_unmultiplied(200, 200, 220, 180)),
            );

            // ── Right: service chips + clock ─────────────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(clock_text())
                        .monospace()
                        .color(egui::Color32::from_rgba_unmultiplied(220, 220, 235, 200)),
                );
                ui.add_space(12.0);

                // Services (right-to-left, so list them in reverse read order)
                for (label, glyph, status) in services(snap).iter().rev() {
                    chip(ui, glyph, label, *status);
                }
            });
        });
}

fn chip(ui: &mut egui::Ui, glyph: &str, tip: &str, status: Ok) {
    let resp = egui::Frame::none()
        .fill(egui::Color32::from_rgba_unmultiplied(28, 28, 38, 180))
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, status.color());
                ui.label(
                    egui::RichText::new(glyph)
                        .monospace()
                        .color(egui::Color32::from_rgba_unmultiplied(230, 230, 240, 220)),
                );
            });
        })
        .response;
    resp.on_hover_text(tip);
    ui.add_space(6.0);
}

/// Determine service statuses from the live snapshot.
///
/// As of M1.5.1 (α) every chip is backed by a real adapter or a real
/// daemon read:
/// - **Kernel** ← `snap.connection` (daemon-socket reachability)
/// - **Mesh** ← `substrate/mesh/status` (cluster.status RPC) via
///   M1.5.1d MeshAdapter. Grey when daemon unreachable or no peers.
/// - **ExoChain** ← `substrate/chain/status` (chain.status RPC) via
///   M1.5.1d ChainAdapter. Grey when the daemon lacks the `exochain`
///   feature or is unreachable.
/// - **Wi-Fi** ← `substrate/network/wifi` via M1.5.1b NetworkAdapter.
/// - **Bluetooth** ← `substrate/bluetooth` via M1.5.1c BluetoothAdapter.
///
/// The M1.5 placeholder **DeFi** chip has been removed — no DeFi
/// subsystem exists in the workspace today. It comes back once an
/// ADR-scoped DeFi module ships with its own adapter.
fn services(snap: &Snapshot) -> Vec<(&'static str, &'static str, Ok)> {
    let kernel = match snap.connection {
        Connection::Connected => Ok::On,
        Connection::Connecting => Ok::Warn,
        Connection::Disconnected => Ok::Off,
    };

    let mesh = mesh_state_to_ok(&snap.mesh_status);
    let exochain = chain_state_to_ok(&snap.chain_status);
    let wifi = link_state_to_ok(&snap.network_wifi);
    let bluetooth = bluetooth_state_to_ok(&snap.bluetooth);

    vec![
        ("Kernel", "◉ kernel", kernel),
        ("Mesh", "⌖ mesh", mesh),
        ("ExoChain", "⛓ chain", exochain),
        ("Wi-Fi", "≋ wifi", wifi),
        ("Bluetooth", "∗ bt", bluetooth),
    ]
}

/// Map a `substrate/network/{wifi,ethernet}` Replace value to a tray
/// chip status. `absent` → grey (Off). `disconnected` → amber (Warn).
/// `connected` → green (On). `None` (no adapter tick landed yet, or
/// we're on wasm before the substrate-bridge ships) → grey.
fn link_state_to_ok(v: &Option<serde_json::Value>) -> Ok {
    let Some(state) = v.as_ref().and_then(|x| x.get("state")).and_then(|s| s.as_str()) else {
        return Ok::Off;
    };
    match state {
        "connected" => Ok::On,
        "disconnected" => Ok::Warn,
        _ => Ok::Off,
    }
}

/// Map a `substrate/bluetooth` Replace value to a tray chip status.
/// `enabled=true` → green. `present=true` but `enabled=false` (rfkill
/// soft-blocked) → amber. Otherwise → grey.
fn bluetooth_state_to_ok(v: &Option<serde_json::Value>) -> Ok {
    let Some(obj) = v.as_ref() else {
        return Ok::Off;
    };
    let enabled = obj.get("enabled").and_then(|b| b.as_bool()).unwrap_or(false);
    let present = obj.get("present").and_then(|b| b.as_bool()).unwrap_or(false);
    match (present, enabled) {
        (_, true) => Ok::On,
        (true, false) => Ok::Warn,
        _ => Ok::Off,
    }
}

/// Map a `substrate/mesh/status` Replace value to a tray chip status.
/// `total_nodes > 0` with `healthy_nodes == total_nodes` → green;
/// some nodes degraded → amber; `available: false` or no data → grey.
fn mesh_state_to_ok(v: &Option<serde_json::Value>) -> Ok {
    let Some(obj) = v.as_ref() else {
        return Ok::Off;
    };
    if obj
        .get("available")
        .and_then(|b| b.as_bool())
        == Some(false)
    {
        return Ok::Off;
    }
    let total = obj.get("total_nodes").and_then(|n| n.as_u64()).unwrap_or(0);
    let healthy = obj.get("healthy_nodes").and_then(|n| n.as_u64()).unwrap_or(0);
    match (total, healthy) {
        (0, _) => Ok::Off,
        (t, h) if h == t => Ok::On,
        _ => Ok::Warn,
    }
}

/// Map a `substrate/chain/status` Replace value to a tray chip status.
/// `available: true` → green; `available: false` (including missing
/// `exochain` feature) → grey.
fn chain_state_to_ok(v: &Option<serde_json::Value>) -> Ok {
    let Some(obj) = v.as_ref() else {
        return Ok::Off;
    };
    // On the success path ChainAdapter injects `available: true`.
    // On the failure path it emits `{available: false, reason}`.
    match obj.get("available").and_then(|b| b.as_bool()) {
        Some(true) => Ok::On,
        _ => Ok::Off,
    }
}

fn clock_text() -> String {
    let now = chrono_utc_local_ish();
    format!("{:02}:{:02}", now.0, now.1)
}

/// Crude local clock via std::time. We don't want a chrono dep bump here;
/// seconds-since-UTC-midnight is close enough for a status bar mock.
fn chrono_utc_local_ish() -> (u32, u32) {
    use web_time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let day = secs % 86_400;
    let hour = (day / 3600) as u32;
    let minute = ((day % 3600) / 60) as u32;
    (hour, minute)
}
