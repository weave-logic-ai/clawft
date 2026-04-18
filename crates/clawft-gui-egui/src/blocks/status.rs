use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.heading("Status");
    ui.label("Threshold-colored metric cards.");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        metric_card(ui, "Kernel RSS", "184", "MB", Some(200.0), Some(300.0), 184.0);
        metric_card(ui, "CPU load", "62", "%", Some(70.0), Some(90.0), 62.0);
        metric_card(ui, "Mesh peers", "3", "", None, None, 3.0);
        metric_card(ui, "HNSW search", "0.9", "ms", Some(5.0), Some(20.0), 0.9);
        metric_card(ui, "Error rate", "3", "/min", Some(1.0), Some(5.0), 3.0);
    });
}

fn metric_card(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    unit: &str,
    warn: Option<f32>,
    crit: Option<f32>,
    current: f32,
) {
    let (border, text) = color_for(current, warn, crit);
    egui::Frame::none()
        .fill(egui::Color32::from_gray(22))
        .stroke(egui::Stroke::new(1.0, border))
        .rounding(6.0)
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.set_min_width(140.0);
            ui.label(egui::RichText::new(label).small().weak());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(value)
                        .monospace()
                        .size(22.0)
                        .color(text),
                );
                if !unit.is_empty() {
                    ui.label(egui::RichText::new(unit).small().weak());
                }
            });
        });
}

fn color_for(v: f32, warn: Option<f32>, crit: Option<f32>) -> (egui::Color32, egui::Color32) {
    if let Some(c) = crit {
        if v >= c {
            return (egui::Color32::from_rgb(220, 70, 70), egui::Color32::from_rgb(255, 140, 140));
        }
    }
    if let Some(w) = warn {
        if v >= w {
            return (egui::Color32::from_rgb(220, 160, 40), egui::Color32::from_rgb(255, 205, 90));
        }
    }
    (egui::Color32::from_rgb(60, 160, 90), egui::Color32::from_rgb(110, 210, 140))
}
