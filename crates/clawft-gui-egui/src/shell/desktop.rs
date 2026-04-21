//! Desktop compositor: warped grid wallpaper + floating windows + tray.

use eframe::egui;

use super::{grid, tray};
use crate::blocks;
use crate::canon_demos::{self, CanonDemoState, CanonKind};
use crate::live::Snapshot;
use crate::surface_host;
use clawft_app::registry::AppRegistry;
use clawft_surface::SurfaceTree;

/// Admin app manifest, loaded inline so the reference panel always
/// boots with at least one app installed even on a fresh workspace.
/// Path-resolved at compile time via `include_str!`.
const WEFTOS_ADMIN_MANIFEST: &str =
    include_str!("../../../clawft-app/fixtures/weftos-admin.toml");

/// Admin desktop surface description (ADR-016 §10). Loaded inline for
/// the same reason as the manifest above.
const WEFTOS_ADMIN_DESKTOP_SURFACE: &str =
    include_str!("../../../clawft-surface/fixtures/weftos-admin-desktop.toml");

/// Which section of the Blocks window is active.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PanelSection {
    Blocks,
    Canon,
    Apps,
}

pub struct Desktop {
    pub launcher_open: bool,
    pub blocks_state: blocks::DemoState,
    pub canon_state: CanonDemoState,
    pub selected_block: BlockKind,
    pub selected_canon: CanonKind,
    pub section: PanelSection,
    pub boot_started: web_time::Instant,

    /// App registry — seeded with WeftOS Admin at startup (M1.5
    /// reference app). Future apps register themselves via
    /// `registry::install`.
    pub app_registry: AppRegistry,
    /// Id of the app currently shown in the Apps panel (e.g.
    /// `app://weftos.admin`). `None` means nothing selected yet.
    pub selected_app: Option<String>,
    /// Cached parsed surface trees keyed by app id, so we don't
    /// re-parse the TOML every frame.
    pub app_surfaces: std::collections::BTreeMap<String, SurfaceTree>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Overview,
    Text,
    Button,
    Code,
    Status,
    Budget,
    Table,
    Tree,
    Tabs,
    Terminal,
    Layout,
    Oscilloscope,
}

impl BlockKind {
    pub const ALL: [(BlockKind, &'static str); 12] = [
        (BlockKind::Overview, "Overview"),
        (BlockKind::Text, "Text"),
        (BlockKind::Button, "Button"),
        (BlockKind::Code, "Code"),
        (BlockKind::Status, "Status"),
        (BlockKind::Budget, "Budget"),
        (BlockKind::Table, "Table"),
        (BlockKind::Tree, "Tree"),
        (BlockKind::Tabs, "Tabs"),
        (BlockKind::Terminal, "Terminal"),
        (BlockKind::Layout, "Layout"),
        (BlockKind::Oscilloscope, "Oscilloscope"),
    ];
}

impl Default for Desktop {
    fn default() -> Self {
        // Session-local registry path: persists to XDG-standard
        // location on native, but we never block startup on save
        // success. On wasm `default_path()` returns an err because
        // HOME is unset — we fall back to an in-memory-only path
        // under `/tmp/weftos` (the save error is ignored below).
        let registry_path = AppRegistry::default_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/weftos/apps.json"));
        let mut app_registry = AppRegistry::new(registry_path);
        // Best-effort load: missing file is not an error.
        let _ = app_registry.load();

        let mut app_surfaces = std::collections::BTreeMap::new();
        let mut selected_app = None;

        // Ensure the WeftOS Admin reference app is installed so the
        // Apps tab has content on first boot. Parse errors here are
        // programmer errors in the bundled fixture, not user input —
        // log and skip rather than panic so the desktop still comes up.
        match clawft_app::manifest::AppManifest::from_toml_str(WEFTOS_ADMIN_MANIFEST) {
            Ok(manifest) => {
                let id = manifest.id.clone();
                if app_registry.get(&id).is_none() {
                    // `install` persists to disk; if that fails (wasm,
                    // read-only fs, …) we still keep the in-memory
                    // entry so the Apps panel has something to show.
                    if let Err(e) = app_registry.install(manifest.clone()) {
                        log::warn!(
                            "couldn't persist WeftOS Admin to registry: {e} (continuing in-memory)"
                        );
                    }
                }
                selected_app = Some(id.clone());
                match clawft_surface::parse::parse_surface_toml(
                    WEFTOS_ADMIN_DESKTOP_SURFACE,
                ) {
                    Ok(tree) => {
                        app_surfaces.insert(id, tree);
                    }
                    Err(e) => {
                        log::warn!(
                            "failed to parse WeftOS Admin desktop surface: {e}"
                        );
                    }
                }
            }
            Err(e) => {
                log::warn!("failed to parse WeftOS Admin manifest: {e}");
            }
        }

        Self {
            launcher_open: false,
            blocks_state: blocks::DemoState::default(),
            canon_state: CanonDemoState::default(),
            selected_block: BlockKind::Overview,
            selected_canon: CanonKind::Pressable,
            section: PanelSection::Blocks,
            boot_started: web_time::Instant::now(),
            app_registry,
            selected_app,
            app_surfaces,
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    desk: &mut Desktop,
    live: &std::sync::Arc<crate::live::Live>,
    snap: &Snapshot,
) {
    let rect = ui.max_rect();

    // Wallpaper — warped grid.
    let t = desk.boot_started.elapsed().as_secs_f32();
    grid::paint(ui, rect, t);

    // Tray (also toggles the launcher window).
    tray::paint(ui, rect, snap, &mut desk.launcher_open);

    // Floating "Blocks" window when launcher is active.
    if desk.launcher_open {
        let mut open = true;
        egui::Window::new("Blocks")
            .collapsible(true)
            .resizable(true)
            .default_size(egui::vec2(780.0, 520.0))
            .default_pos(egui::pos2(rect.left() + 60.0, rect.top() + 60.0))
            .open(&mut open)
            .frame(window_frame())
            .show(ui.ctx(), |ui| {
                render_blocks_window(ui, desk, live, snap);
            });
        if !open {
            desk.launcher_open = false;
        }
    }
}

fn window_frame() -> egui::Frame {
    egui::Frame::window(&egui::Style::default())
        .fill(egui::Color32::from_rgba_unmultiplied(18, 18, 24, 235))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25),
        ))
        .rounding(10.0)
        .inner_margin(egui::Margin::same(8.0))
}

fn render_blocks_window(
    ui: &mut egui::Ui,
    desk: &mut Desktop,
    live: &std::sync::Arc<crate::live::Live>,
    snap: &Snapshot,
) {
    egui::SidePanel::left("blocks_nav")
        .resizable(false)
        .default_width(170.0)
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(desk.section == PanelSection::Blocks, "Blocks")
                    .clicked()
                {
                    desk.section = PanelSection::Blocks;
                }
                if ui
                    .selectable_label(desk.section == PanelSection::Canon, "Canon")
                    .clicked()
                {
                    desk.section = PanelSection::Canon;
                }
                if ui
                    .selectable_label(desk.section == PanelSection::Apps, "Apps")
                    .clicked()
                {
                    desk.section = PanelSection::Apps;
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match desk.section {
                    PanelSection::Blocks => {
                        for (kind, label) in BlockKind::ALL {
                            let selected = desk.selected_block == kind;
                            if ui.selectable_label(selected, label).clicked() {
                                desk.selected_block = kind;
                            }
                        }
                    }
                    PanelSection::Canon => {
                        for (kind, label) in CanonKind::ALL {
                            let selected = desk.selected_canon == kind;
                            if ui.selectable_label(selected, label).clicked() {
                                desk.selected_canon = kind;
                            }
                        }
                    }
                    PanelSection::Apps => {
                        // Snapshot `(id, name)` up front so we don't
                        // hold a borrow over the click handler that
                        // writes `desk.selected_app`.
                        let entries: Vec<(String, String)> = desk
                            .app_registry
                            .list()
                            .iter()
                            .map(|a| {
                                (a.manifest.id.clone(), a.manifest.name.clone())
                            })
                            .collect();
                        if entries.is_empty() {
                            ui.label(
                                egui::RichText::new("No apps installed")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                        } else {
                            for (id, name) in entries {
                                let selected = desk.selected_app.as_deref() == Some(id.as_str());
                                if ui.selectable_label(selected, name).clicked() {
                                    desk.selected_app = Some(id);
                                }
                            }
                        }
                    }
                });
        });

    egui::CentralPanel::default().show_inside(ui, |ui| match desk.section {
        PanelSection::Blocks => match desk.selected_block {
            BlockKind::Overview => blocks::overview::show(ui, snap),
            BlockKind::Text => blocks::text::show(ui),
            BlockKind::Button => blocks::button::show(ui, &mut desk.blocks_state),
            BlockKind::Code => blocks::code::show(ui, snap),
            BlockKind::Status => blocks::status::show(ui, snap),
            BlockKind::Budget => blocks::budget::show(ui),
            BlockKind::Table => blocks::table::show(ui, &mut desk.blocks_state, snap),
            BlockKind::Tree => blocks::tree::show(ui, &mut desk.blocks_state, snap),
            BlockKind::Tabs => blocks::tabs::show(ui, &mut desk.blocks_state),
            BlockKind::Terminal => blocks::terminal::show(ui, &mut desk.blocks_state, live),
            BlockKind::Layout => blocks::layout::show(ui),
            BlockKind::Oscilloscope => blocks::oscilloscope::show(ui, &mut desk.blocks_state),
        },
        PanelSection::Canon => {
            canon_demos::show(ui, desk.selected_canon, &mut desk.canon_state);
        }
        PanelSection::Apps => render_selected_app(ui, desk, live, snap),
    });
}

/// Render whichever app is currently selected in `desk.selected_app`.
/// Builds an `OntologySnapshot` from the live substrate and drives the
/// surface-description composer against the app's cached surface tree.
///
/// **M1.5.1a**: drains the composer's `PendingDispatch` list and
/// submits each one through the `Live` RPC bridge. This closes the
/// loop from "admin surface declares an affordance" → "user clicks
/// the primitive" → "daemon handler fires." Replies are
/// fire-and-forget for now; the substrate's next poll tick surfaces
/// the result (e.g. the killed PID disappears from `kernel.ps`).
fn render_selected_app(
    ui: &mut egui::Ui,
    desk: &Desktop,
    live: &std::sync::Arc<crate::live::Live>,
    snap: &crate::live::Snapshot,
) {
    let Some(app_id) = desk.selected_app.as_deref() else {
        ui.label(
            egui::RichText::new("Select an app from the list")
                .italics()
                .color(egui::Color32::GRAY),
        );
        return;
    };
    let Some(tree) = desk.app_surfaces.get(app_id) else {
        ui.colored_label(
            egui::Color32::from_rgb(220, 160, 60),
            format!("No surface description loaded for {app_id}"),
        );
        return;
    };

    if let Some(installed) = desk.app_registry.get(app_id) {
        ui.horizontal(|ui| {
            ui.heading(&installed.manifest.name);
            ui.separator();
            ui.monospace(&installed.manifest.id);
        });
    }

    // Offline banner — before the composer runs so it's always visible
    // at the top of the app pane, not buried under the 2x2 grid. The
    // admin surface binds to `substrate/kernel/*` topics; when the
    // daemon link is down every binding resolves to empty and the
    // panel would otherwise look silently broken.
    match snap.connection {
        crate::live::Connection::Connected => {}
        crate::live::Connection::Connecting => {
            ui.colored_label(
                egui::Color32::from_rgb(220, 180, 60),
                "⏳ Connecting to kernel daemon…",
            );
        }
        crate::live::Connection::Disconnected => {
            ui.colored_label(
                egui::Color32::from_rgb(240, 150, 150),
                "◉ Demo mode — kernel daemon offline. \
                 Start with:  weaver kernel start",
            );
        }
    }
    ui.separator();

    // Compose against the current substrate snapshot, then dispatch
    // any affordance activations through the RPC bridge.
    let snapshot = live.substrate_snapshot();
    let outcome = surface_host::compose(tree, &snapshot, ui);
    for d in outcome.dispatches {
        log::info!(
            "admin app affordance: {} -> {} ({:?}) from {}",
            d.affordance,
            d.verb,
            d.params,
            d.source_path
        );
        live.submit(crate::live::Command::Raw {
            method: d.verb,
            params: d.params,
            reply: None,
        });
    }
}
