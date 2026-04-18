//! Top-level app: side-navigation of block demos.

use std::sync::Arc;

use eframe::egui;

use crate::blocks;
use crate::live::{Connection, Live};

#[derive(Copy, Clone, PartialEq, Eq)]
enum Demo {
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

impl Demo {
    const ALL: [(Demo, &'static str); 12] = [
        (Demo::Overview, "Overview"),
        (Demo::Text, "Text"),
        (Demo::Button, "Button"),
        (Demo::Code, "Code"),
        (Demo::Status, "Status"),
        (Demo::Budget, "Budget"),
        (Demo::Table, "Table"),
        (Demo::Tree, "Tree"),
        (Demo::Tabs, "Tabs"),
        (Demo::Terminal, "Terminal"),
        (Demo::Layout, "Row / Column / Grid"),
        (Demo::Oscilloscope, "Oscilloscope"),
    ];
}

pub struct ClawftApp {
    selected: Demo,
    state: blocks::DemoState,
    live: Arc<Live>,
}

impl ClawftApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            selected: Demo::Overview,
            state: blocks::DemoState::default(),
            live: Live::spawn(),
        }
    }

    fn status_pill(&self, ui: &mut egui::Ui, conn: Connection) {
        let (color, label) = match conn {
            Connection::Connecting => (egui::Color32::from_rgb(180, 160, 40), "connecting"),
            Connection::Connected => (egui::Color32::from_rgb(60, 160, 90), "connected"),
            Connection::Disconnected => (egui::Color32::from_rgb(200, 80, 80), "offline"),
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(10.0, 10.0),
                egui::Sense::hover(),
            );
            ui.painter().circle_filled(rect.center(), 5.0, color);
            ui.label(egui::RichText::new(label).small());
        });
    }
}

impl eframe::App for ClawftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let snap = self.live.snapshot();

        egui::SidePanel::left("nav")
            .resizable(false)
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("ClawFT blocks");
                ui.add_space(4.0);
                ui.label(egui::RichText::new("egui spike").weak().small());
                self.status_pill(ui, snap.connection);
                ui.separator();
                for (demo, label) in Demo::ALL {
                    let selected = self.selected == demo;
                    if ui.selectable_label(selected, label).clicked() {
                        self.selected = demo;
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected {
            Demo::Overview => blocks::overview::show(ui, &snap),
            Demo::Text => blocks::text::show(ui),
            Demo::Button => blocks::button::show(ui, &mut self.state),
            Demo::Code => blocks::code::show(ui, &snap),
            Demo::Status => blocks::status::show(ui, &snap),
            Demo::Budget => blocks::budget::show(ui),
            Demo::Table => blocks::table::show(ui, &mut self.state, &snap),
            Demo::Tree => blocks::tree::show(ui, &mut self.state, &snap),
            Demo::Tabs => blocks::tabs::show(ui, &mut self.state),
            Demo::Terminal => blocks::terminal::show(ui, &mut self.state, &self.live),
            Demo::Layout => blocks::layout::show(ui),
            Demo::Oscilloscope => blocks::oscilloscope::show(ui, &mut self.state),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}
