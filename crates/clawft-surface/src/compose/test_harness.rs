//! Headless render helpers for integration tests (ADR-016 §8).
//!
//! Runs one egui pass against an in-memory Context with no native
//! window and no eframe glue. Suitable for asserting primitive
//! counts, affordance presence, and rendered text regexes.

use crate::substrate::OntologySnapshot;
use crate::tree::SurfaceTree;

use super::runtime::{ComposePermits, compose_with_permits};
use clawft_canon::CanonResponse;

/// Execute the composer in a headless egui frame. Returns the full
/// `Vec<CanonResponse>` for assertion.
///
/// The implementation allocates a throw-away `egui::Context`, runs
/// one `begin_frame`/`end_frame` cycle, and discards the paint
/// output. Enough for unit tests; not an egui viewport.
///
/// **M1.5.1a**: the composer now returns a [`ComposeOutcome`] with
/// both responses and pending RPC dispatches. This helper keeps the
/// historical `Vec<CanonResponse>` return shape for existing tests;
/// callers who want dispatches should use `render_headless_full`.
///
/// Uses [`ComposePermits::open`] (unrestricted non-capture verbs).
pub fn render_headless(tree: &SurfaceTree, snapshot: OntologySnapshot) -> Vec<CanonResponse> {
    render_headless_full(tree, snapshot).responses
}

/// Same as [`render_headless`] but returns the full
/// [`ComposeOutcome`] — responses + pending dispatches. Used by
/// M1.5.1a tests that assert dispatch plumbing without opening a
/// viewport.
// `Context::run` is deprecated in egui 0.34 in favour of `run_ui`,
// and `CentralPanel::show` is deprecated in favour of `show_inside`.
// The test harness's whole point is to run a one-shot frame with no
// real Ui in scope, which is exactly what the deprecated `run` was
// for. Migrating to `run_ui` would require restructuring the test
// harness around a Ui that doesn't exist outside a viewport. Allow
// the deprecation here pending a deeper rewrite.
#[allow(deprecated)]
pub fn render_headless_full(
    tree: &SurfaceTree,
    snapshot: OntologySnapshot,
) -> super::runtime::ComposeOutcome {
    render_headless_with_permits(tree, snapshot, &ComposePermits::open())
}

/// Headless compose with explicit ADR-006 rule 2 permits (WEFT-430).
#[allow(deprecated)]
pub fn render_headless_with_permits(
    tree: &SurfaceTree,
    snapshot: OntologySnapshot,
    permits: &ComposePermits,
) -> super::runtime::ComposeOutcome {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput::default();
    let mut captured: super::runtime::ComposeOutcome = super::runtime::ComposeOutcome::default();

    let _output = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            captured = compose_with_permits(tree, &snapshot, ui, permits);
        });
    });

    captured
}
