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

/// Determine service statuses from the live snapshot. For now, only the
/// kernel link is real — the rest are mocked off/on based on whether the
/// kernel services list contains something matching.
fn services(snap: &Snapshot) -> Vec<(&'static str, &'static str, Ok)> {
    let kernel = match snap.connection {
        Connection::Connected => Ok::On,
        Connection::Connecting => Ok::Warn,
        Connection::Disconnected => Ok::Off,
    };

    let exochain = service_present(snap, &["chain", "exochain"]);
    let mesh = service_present(snap, &["mesh"]);
    let defi = service_present(snap, &["defi", "bond"]);
    let wifi = Ok::On; // placeholder
    let bluetooth = Ok::Off; // placeholder

    vec![
        ("Kernel", "◉ kernel", kernel),
        ("Mesh", "⌖ mesh", mesh),
        ("ExoChain", "⛓ chain", exochain),
        ("DeFi", "◇ defi", defi),
        ("Wi-Fi", "≋ wifi", wifi),
        ("Bluetooth", "∗ bt", bluetooth),
    ]
}

fn service_present(snap: &Snapshot, needles: &[&str]) -> Ok {
    let Some(services) = &snap.services else {
        return Ok::Off;
    };
    for svc in services {
        let Some(name) = svc.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(stype) = svc.get("service_type").and_then(|v| v.as_str()) else {
            continue;
        };
        let hay = format!("{name} {stype}").to_lowercase();
        if needles.iter().any(|n| hay.contains(n)) {
            return Ok::On;
        }
    }
    Ok::Off
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
