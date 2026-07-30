//! Composer runtime — walk a [`crate::SurfaceTree`] and drive the
//! primitive canon in [`clawft_canon`].
//!
//! **WEFT-427**: extracted from `clawft-gui-egui::surface_host` now that
//! the canon types live in a shared crate (no surface ↔ gui-egui cycle).
//!
//! Public entry points:
//! - [`compose`] / [`compose_with_permits`] — per-frame walk
//! - [`render_headless`] / [`render_headless_full`] — integration tests

mod runtime;
mod test_harness;

pub use runtime::{
    ComposeOutcome, ComposePermits, PendingDispatch, compose, compose_with_permits,
    honest_affordances, normalize_verb,
};
pub use test_harness::{render_headless, render_headless_full, render_headless_with_permits};
