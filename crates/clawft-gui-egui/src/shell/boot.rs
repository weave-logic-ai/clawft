//! Boot splash: centered logo + subtle title, fade in → hold → fade out.

use eframe::egui;

use super::{audio, boot_logo_alpha, BOOT_LEN};

/// Logo bytes are embedded so the binary is self-contained. Exposed
/// `pub(crate)` so `App::new` can preload it into the image cache before
/// the first paint.
pub(crate) const LOGO_JPG: &[u8] = include_bytes!("../../assets/weftos-gold.jpg");

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

    // Paint the solid background + warm halo + subtitle using a layer
    // painter that doesn't hold a borrow on `ui`, so we can still
    // allocate the image sub-ui below. The halo is warm gold to echo
    // the logo's color.
    let painter = ui.ctx().layer_painter(egui::LayerId::background());
    painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
    let halo = egui::Color32::from_rgba_unmultiplied(80, 60, 20, (alpha * 150.0) as u8);
    painter.circle_filled(center, 280.0, halo);

    let subtitle_alpha = (alpha * 0.55 * 255.0) as u8;
    painter.text(
        center + egui::vec2(0.0, 220.0),
        egui::Align2::CENTER_CENTER,
        "weave the machine",
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
        egui::Color32::from_rgba_unmultiplied(210, 195, 160, subtitle_alpha),
    );

    // Logo image (JPEG). Aspect is roughly 1.14:1 (1151×1007).
    let logo_size = egui::vec2(360.0, 315.0);
    let logo_rect = egui::Rect::from_center_size(center, logo_size);
    let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, (alpha * 255.0) as u8);
    let img = egui::Image::from_bytes("bytes://weftos-gold.jpg", LOGO_JPG)
        .fit_to_exact_size(logo_size)
        .tint(tint);
    let mut logo_ui = ui.new_child(
        egui::UiBuilder::new().max_rect(logo_rect),
    );
    logo_ui.add(img);

    elapsed >= BOOT_LEN
}
