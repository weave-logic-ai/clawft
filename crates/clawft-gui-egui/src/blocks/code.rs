use eframe::egui;

const SAMPLE: &str = r#"fn main() {
    let kernel = Kernel::boot(&config)?;
    kernel.run().await?;
}
"#;

pub fn show(ui: &mut egui::Ui) {
    ui.heading("Code");
    ui.label("Monospace block with copy-to-clipboard. Syntax highlighting TBD.");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("main.rs").weak().monospace());
        if ui.small_button("📋 copy").clicked() {
            ui.ctx().copy_text(SAMPLE.into());
        }
    });

    egui::Frame::none()
        .fill(egui::Color32::from_gray(18))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(SAMPLE).monospace())
                    .selectable(true)
                    .wrap(),
            );
        });
}
