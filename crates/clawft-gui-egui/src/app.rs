//! Top-level app: boot splash → desktop shell with tray + floating windows.

use std::sync::Arc;

use eframe::egui;

use crate::live::Live;
use crate::shell::{self, desktop::Desktop, Phase};

pub struct ClawftApp {
    phase: Phase,
    desktop: Desktop,
    live: Arc<Live>,
}

impl ClawftApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        egui_extras::install_image_loaders(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);
        Self {
            phase: Phase::boot(),
            desktop: Desktop::default(),
            live: Live::spawn(),
        }
    }
}

impl eframe::App for ClawftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let snap = self.live.snapshot();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| match &mut self.phase {
                Phase::Boot {
                    started,
                    sfx_played,
                } => {
                    let done = shell::boot::show(ui, *started, sfx_played);
                    if done {
                        self.phase = Phase::Desktop;
                        self.desktop.boot_started = std::time::Instant::now();
                    }
                }
                Phase::Desktop => {
                    shell::desktop::show(ui, &mut self.desktop, &self.live, &snap);
                }
            });

        // ~60 fps for the grid animation / fade.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    // Crisp, low-chrome look.
    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18));
    ctx.set_style(style);
}
