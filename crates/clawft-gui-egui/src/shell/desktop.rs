//! Desktop compositor: warped grid wallpaper + floating windows + tray.

use eframe::egui;

use super::{grid, tray};
use crate::blocks;
use crate::live::Snapshot;

pub struct Desktop {
    pub launcher_open: bool,
    pub blocks_state: blocks::DemoState,
    pub selected_block: BlockKind,
    pub boot_started: std::time::Instant,
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
        Self {
            launcher_open: false,
            blocks_state: blocks::DemoState::default(),
            selected_block: BlockKind::Overview,
            boot_started: std::time::Instant::now(),
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
            ui.heading("Blocks");
            ui.separator();
            for (kind, label) in BlockKind::ALL {
                let selected = desk.selected_block == kind;
                if ui.selectable_label(selected, label).clicked() {
                    desk.selected_block = kind;
                }
            }
        });

    egui::CentralPanel::default().show_inside(ui, |ui| match desk.selected_block {
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
    });
}
