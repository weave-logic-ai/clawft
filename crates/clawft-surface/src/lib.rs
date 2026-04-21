//! `clawft-surface` — surface-description IR, TOML + Rust builder
//! parsers, binding expression evaluator, and composer runtime that
//! drives the canon primitives in `clawft-gui-egui`.
//!
//! This crate is the **M1.5 subset** of ADR-016 — sufficient for the
//! WeftOS Admin reference panel (session-10 §6.1). Out of scope for
//! M1.5: ternary, nested lambdas, user-defined compositions, real
//! governance intersection, variant-id stamping, hot-path
//! memoisation. Sibling milestones M1.6+ fill these in.
//!
//! # Authoring paths
//!
//! ```no_run
//! use clawft_surface::builder::{grid, chip, stack, Surface};
//! use clawft_surface::tree::{Mode, Input};
//!
//! let tree = Surface::new("weftos-admin/desktop")
//!     .modes(&[Mode::Desktop, Mode::Ide])
//!     .inputs(&[Input::Pointer, Input::Hybrid])
//!     .title("WeftOS Admin")
//!     .subscribe("substrate/kernel/status")
//!     .root(
//!         grid("/root")
//!             .attr("columns", clawft_surface::tree::AttrValue::Int(2))
//!             .child(
//!                 stack("/root/overview")
//!                     .attr("axis", clawft_surface::tree::AttrValue::Str("horizontal".into()))
//!                     .child(
//!                         chip("/root/overview/status")
//!                             .bind("label", "$substrate/kernel/status.state")
//!                             .bind("tone", "$substrate/kernel/status.state"),
//!                     ),
//!             ),
//!     )
//!     .build();
//! ```
//!
//! See the integration tests (`tests/`) for the TOML authoring path
//! and a full admin-panel fixture.

pub mod builder;
pub mod compose;
pub mod eval;
pub mod parse;
pub mod substrate;
pub mod test_harness;
pub mod tree;

pub use compose::compose;
pub use substrate::OntologySnapshot;
pub use test_harness::render_headless;
pub use tree::{
    AffordanceDecl, AttrValue, Binding, IdentityIri, Input, Invocation, Mode, SurfaceNode,
    SurfaceTree,
};
