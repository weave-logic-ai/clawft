//! ADR-016 surface-description host — walks a
//! [`clawft_surface::SurfaceTree`] and drives the in-crate canon
//! primitives.
//!
//! Lives in `clawft-gui-egui` (and not `clawft-surface` where the IR +
//! parser + evaluator live) because the composer talks to concrete
//! canon widget types (`Chip`, `Gauge`, `Table`, …) defined here. A
//! future milestone may extract the canon types into their own crate,
//! at which point this module can move back into `clawft-surface`.
//!
//! Public entry points:
//! - [`compose`] — walk a `SurfaceTree` against an `OntologySnapshot`
//!   and render every frame.
//! - [`render_headless`] — run one egui pass with no viewport, for
//!   integration tests that want to assert on the composer's return
//!   signals.

mod compose;
mod test_harness;

pub use compose::{
    ComposeOutcome, ComposePermits, PendingDispatch, compose, compose_with_permits,
    honest_affordances, normalize_verb,
};
pub use test_harness::{render_headless, render_headless_full, render_headless_with_permits};
