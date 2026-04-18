use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.heading("Native GUI spike — egui / eframe");
    ui.add_space(4.0);
    ui.label(
        "Ports the 12 core UI blocks from gui/src/blocks/*.tsx into Rust-native \
         egui widgets. Pick a block on the left to see it in action.",
    );
    ui.add_space(12.0);

    ui.group(|ui| {
        ui.label(egui::RichText::new("Block status").strong());
        for (name, note) in [
            ("Text", "RichText + wrapping, mock heading/body"),
            ("Button", "variants (primary/secondary/ghost) + counter"),
            ("Code", "monospace, syntax-free, copy button"),
            ("Status", "threshold-colored metric card"),
            ("Budget", "per-agent cost + budget bars"),
            ("Table", "sortable columns via egui_extras::TableBuilder"),
            ("Tree", "collapsing headers with file-tree mock"),
            ("Tabs", "egui::TopBottomPanel mock tab strip"),
            ("Terminal", "scrollback + input line, `help/clear/echo`"),
            ("Layout", "Row / Column / Grid primitives side-by-side"),
            ("Oscilloscope", "egui_plot Line, sliding window sine"),
            ("Overview", "you are here"),
        ] {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{name:>12}")).monospace());
                ui.label(" — ");
                ui.label(egui::RichText::new(note).weak());
            });
        }
    });
}
