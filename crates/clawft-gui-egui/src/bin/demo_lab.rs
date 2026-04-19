//! Theming lab — hosts the full `egui_demo_lib::DemoWindows` suite with
//! WeftOS theming applied globally. A local, faster iteration copy of
//! https://www.egui.rs/#demo so we can tune design tokens against every
//! stock egui widget in one place.

use clawft_gui_egui::theming;
use eframe::egui;
use egui_demo_lib::DemoWindows;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 520.0])
            .with_title("WeftOS — egui demo lab (themed)"),
        ..Default::default()
    };
    eframe::run_native(
        "WeftOS egui demo lab",
        options,
        Box::new(|cc| Ok(Box::new(DemoLab::new(cc)))),
    )
}

struct DemoLab {
    demo: DemoWindows,
}

impl DemoLab {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theming::apply(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            demo: DemoWindows::default(),
        }
    }
}

impl eframe::App for DemoLab {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Match the theming bg_app token (8,8,10).
        [8.0 / 255.0, 8.0 / 255.0, 10.0 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top bar with a short hint so this isn't mistaken for the main shell.
        egui::TopBottomPanel::top("demo_lab_header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("WeftOS").strong());
                ui.label(egui::RichText::new("egui demo lab").weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("theming tokens live in crates/clawft-gui-egui/src/theming.rs")
                            .small()
                            .weak(),
                    );
                });
            });
        });

        self.demo.ui(ctx);
    }
}
