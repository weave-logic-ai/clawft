//! Composer runtime — walk a [`crate::SurfaceTree`] and drive the
//! primitive canon in [`clawft_canon`].
//!
//! **WEFT-427**: extracted from `clawft-gui-egui::surface_host` now that
//! the canon types live in a shared crate (no surface ↔ gui-egui cycle).
//!
//! **WEFT-277**: [`ComposeGovernance`] is the goal-aggregate / GEPA half
//! of ADR-006 rule 2 (`raw ∩ permit ∩ governance`).
//!
//! Public entry points:
//! - [`compose`] / [`compose_with_permits`] — per-frame walk
//! - [`render_headless`] / [`render_headless_full`] — integration tests

mod governance;
mod runtime;
mod test_harness;

pub use governance::{
    ComposeGovernance, EffectCeiling, estimate_verb_effect, is_gepa_mutate_verb, normalize_verb,
};
pub use runtime::{
    ComposeOutcome, ComposePermits, PendingDispatch, compose, compose_with_permits,
    honest_affordances,
};
pub use test_harness::{render_headless, render_headless_full, render_headless_with_permits};
