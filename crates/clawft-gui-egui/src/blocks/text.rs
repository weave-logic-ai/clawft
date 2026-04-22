use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.heading("Text");
    ui.label("Text block — heading / body / small / weak variants.");
    ui.separator();

    ui.heading("The kernel is the substrate.");
    ui.label(
        "Standard body text wraps naturally. Lorem ipsum dolor sit amet, \
         consectetur adipiscing elit. Cras venenatis euismod malesuada.",
    );
    ui.add_space(8.0);
    ui.label(egui::RichText::new("strong").strong());
    ui.label(egui::RichText::new("weak secondary copy").weak());
    ui.label(egui::RichText::new("small footnote").small());
    ui.label(
        egui::RichText::new("monospace code-ish text")
            .monospace()
            .color(egui::Color32::LIGHT_BLUE),
    );
    ui.add_space(8.0);
    ui.hyperlink_to("weavelogic.ai", "https://weavelogic.ai");
}
