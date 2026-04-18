//! ClawFT native GUI spike (egui/eframe).
//!
//! Hosts the 12 core UI blocks ported from `gui/src/blocks/*.tsx` so we can
//! evaluate egui as the native front-end target. The blocks are deliberately
//! small (<50 lines each) and use only mock/demo data — no kernel IPC wired
//! up yet. A future pass can swap the mock state for a real daemon client.

pub mod app;
pub mod blocks;
pub mod live;

pub use app::ClawftApp;

/// Entry point used by both the bin and any embedder (e.g. a sidecar process
/// launched from an editor extension).
pub fn run() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_title("ClawFT — egui spike"),
        ..Default::default()
    };
    eframe::run_native(
        "ClawFT egui spike",
        native_options,
        Box::new(|cc| Ok(Box::new(ClawftApp::new(cc)))),
    )
}
