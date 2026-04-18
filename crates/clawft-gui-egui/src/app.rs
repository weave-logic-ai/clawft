//! Top-level app: side-navigation of block demos.

use eframe::egui;

use crate::blocks;

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
}

impl ClawftApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            selected: Demo::Overview,
            state: blocks::DemoState::default(),
        }
    }
}

impl eframe::App for ClawftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("nav")
            .resizable(false)
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("ClawFT blocks");
                ui.add_space(4.0);
                ui.label(egui::RichText::new("egui spike").weak().small());
                ui.separator();
                for (demo, label) in Demo::ALL {
                    let selected = self.selected == demo;
                    if ui.selectable_label(selected, label).clicked() {
                        self.selected = demo;
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected {
            Demo::Overview => blocks::overview::show(ui),
            Demo::Text => blocks::text::show(ui),
            Demo::Button => blocks::button::show(ui, &mut self.state),
            Demo::Code => blocks::code::show(ui),
            Demo::Status => blocks::status::show(ui),
            Demo::Budget => blocks::budget::show(ui),
            Demo::Table => blocks::table::show(ui, &mut self.state),
            Demo::Tree => blocks::tree::show(ui, &mut self.state),
            Demo::Tabs => blocks::tabs::show(ui, &mut self.state),
            Demo::Terminal => blocks::terminal::show(ui, &mut self.state),
            Demo::Layout => blocks::layout::show(ui),
            Demo::Oscilloscope => blocks::oscilloscope::show(ui, &mut self.state),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}
