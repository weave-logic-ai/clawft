//! Headless render helpers for integration tests (ADR-016 §8).
//!
//! Runs one egui pass against an in-memory Context with no native
//! window and no eframe glue. Suitable for asserting primitive
//! counts, affordance presence, and rendered text regexes.

use clawft_gui_egui::canon::CanonResponse;

use crate::compose::compose;
use crate::substrate::OntologySnapshot;
use crate::tree::SurfaceTree;

/// Execute the composer in a headless egui frame. Returns the full
/// `Vec<CanonResponse>` for assertion.
///
/// The implementation allocates a throw-away `egui::Context`, runs
/// one `begin_frame`/`end_frame` cycle, and discards the paint
/// output. Enough for unit tests; not an egui viewport.
pub fn render_headless(
    tree: &SurfaceTree,
    snapshot: OntologySnapshot,
) -> Vec<CanonResponse> {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput::default();
    let mut captured: Vec<CanonResponse> = Vec::new();

    let _output = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            captured = compose(tree, &snapshot, ui);
        });
    });

    captured
}
