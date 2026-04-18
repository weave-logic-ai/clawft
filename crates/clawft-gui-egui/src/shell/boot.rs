//! Boot splash: centered logo + subtle title, fade in → hold → fade out.

use eframe::egui;

use super::{audio, boot_logo_alpha, BOOT_LEN};

/// Logo bytes are embedded so the binary is self-contained.
const LOGO_SVG: &[u8] = include_bytes!("../../assets/weft-logo.svg");

/// Render the boot splash. Returns `true` when the boot timeline has
/// elapsed and the caller should transition to the desktop.
pub fn show(
    ui: &mut egui::Ui,
    started: std::time::Instant,
    sfx_played: &mut bool,
) -> bool {
    let elapsed = started.elapsed().as_secs_f32();
    let alpha = boot_logo_alpha(elapsed);

    // Fire scuttle sound once, early in the fade-in.
    if !*sfx_played && elapsed > 0.05 {
        audio::play_scuttle();
        *sfx_played = true;
    }

    let rect = ui.max_rect();
    let center = rect.center();

    // Paint the solid background + halo + title text using a layer painter
    // that doesn't hold a borrow on `ui`, so we can still allocate the
    // image sub-ui below.
    let painter = ui.ctx().layer_painter(egui::LayerId::background());
    painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
    let halo = egui::Color32::from_rgba_unmultiplied(30, 18, 60, (alpha * 180.0) as u8);
    painter.circle_filled(center, 220.0, halo);

    let title_alpha = (alpha * 255.0) as u8;
    let subtitle_alpha = (alpha * 0.55 * 255.0) as u8;
    painter.text(
        center + egui::vec2(0.0, 150.0),
        egui::Align2::CENTER_CENTER,
        "WeftOS",
        egui::FontId::new(36.0, egui::FontFamily::Proportional),
        egui::Color32::from_rgba_unmultiplied(230, 225, 245, title_alpha),
    );
    painter.text(
        center + egui::vec2(0.0, 185.0),
        egui::Align2::CENTER_CENTER,
        "weave the machine",
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
        egui::Color32::from_rgba_unmultiplied(180, 175, 200, subtitle_alpha),
    );

    // Logo image (SVG). Uses egui_extras' SVG loader installed in App::new.
    let logo_size = egui::vec2(220.0, 220.0);
    let logo_rect = egui::Rect::from_center_size(center, logo_size);
    let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, (alpha * 255.0) as u8);
    let img = egui::Image::from_bytes("bytes://weft-logo.svg", LOGO_SVG)
        .fit_to_exact_size(logo_size)
        .tint(tint);
    let mut logo_ui = ui.new_child(
        egui::UiBuilder::new().max_rect(logo_rect),
    );
    logo_ui.add(img);

    elapsed >= BOOT_LEN
}
