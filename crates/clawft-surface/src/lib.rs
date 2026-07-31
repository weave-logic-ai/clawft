//! `clawft-surface` — surface-description IR, TOML + Rust builder
//! parsers, binding expression evaluator, and composer runtime that
//! drives the shared canon primitives in [`clawft_canon`].
//!
//! This crate is the **M1.5 subset** of ADR-016 — sufficient for the
//! WeftOS Admin reference panel (session-10 §6.1). Governance /
//! GEPA affordance intersection (WEFT-277 / ADR-006 rule 2 + ADR-008)
//! ships as a compose-time policy snapshot ([`ComposeGovernance`]);
//! full Goal aggregate + chain events remain kernel-side. Still out
//! of scope here: ternary, nested lambdas, hot-path memoisation.
//! Sibling milestones M1.6+ fill these in.
//!
//! **WEFT-431**: variant-id stamping is driven from surface bindings
//! (`variant_id` / `variant` slots or attrs) into every
//! [`CanonResponse`] the composer emits (ADR-006 §4 / ADR-007).
//!
//! # M1.5 scope reductions vs ADR-016 §5
//!
//! The binding expression language implemented here is a near-total
//! subset of the grammar in ADR-016 §5. As of 0.7.0 the wired
//! combinators are `count`, `filter`, `sort`, `len`, `first`, `last`
//! plus the `fmt_*` and `exists` helpers. The following are *still*
//! out of scope and are expected to land in M1.6+:
//!
//! - Ternary `?:` operator — explicitly rejected with
//!   `ParseError::TernaryNotSupported`.
//! - Nested lambdas — a lambda body that itself contains `->` is
//!   rejected with `ParseError::NestedLambda`. Sibling lambdas
//!   (e.g. the `s -> …` inside a `count(…, s -> …)` that sits
//!   beside another `t -> …` at the same depth) are permitted.
//!
//! Already shipped (history note for downstream readers):
//! - `sort(list, key)` ordering combinator (WEFT-423).
//! - `.first` / `.last` field-access shorthand on list values
//!   (WEFT-422); the function-call form `first($xs)` / `last($xs)`
//!   continues to work.
//! - Scientific (`1e5`, `1.5e-3`) and hex (`0xff`) number literals
//!   (WEFT-424).
//! - User-defined compositions (`[compositions.*]`) — parsed and
//!   expanded at load time into canon IRIs (WEFT-425 / ADR-016 §7).
//!
//! See `.planning/symposiums/compositional-ui/adrs/adr-016-surface-description.md`
//! §5 for the full authoritative grammar.
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
pub mod tree;

/// Ontology snapshot re-export (ADR-017 §1; unified M1.5-D).
///
/// The composer runtime reads the substrate state tree through an
/// [`OntologySnapshot`]. The canonical definition lives in
/// `clawft-substrate` alongside the `Substrate` state tree; this
/// module simply re-exports it so downstream callers can keep using
/// `clawft_surface::substrate::OntologySnapshot`.
///
/// Prior to M1.5-D this crate held its own structurally-identical
/// copy to avoid a pre-merge dependency cycle. That cycle is now
/// resolved — the substrate crate owns the type, and the composer
/// builds on top of it.
///
/// **WEFT-428**: previously a 14-line `src/substrate.rs` shim; folded
/// inline here as a one-line re-export so the indirection has zero
/// surface area.
pub mod substrate {
    pub use clawft_substrate::OntologySnapshot;
}

pub use compose::{
    ComposeGovernance, ComposeOutcome, ComposePermits, EffectCeiling, PendingDispatch, compose,
    compose_with_permits, estimate_verb_effect, honest_affordances, is_gepa_mutate_verb,
    normalize_verb, render_headless, render_headless_full, render_headless_with_permits,
};
pub use substrate::OntologySnapshot;
pub use tree::{
    AffordanceDecl, AttrValue, Binding, CompositionDef, IdentityIri, Input, Invocation, Mode,
    SurfaceNode, SurfaceTree,
};

// WEFT-427: canon primitives live in `clawft-canon`; the composer
// runtime lives here again (was temporarily in
// `clawft-gui-egui::surface_host` during M1.5-D to break the cycle).
