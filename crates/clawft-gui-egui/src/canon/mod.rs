//! Primitive canon — the 21-item vocabulary every renderer speaks.
//!
//! Frozen by ADR-001 (amended 2026-04-20 to add `ui://tabs` as row 21).
//! Each primitive implements [`CanonWidget`] and returns a
//! [`CanonResponse`] that carries the four return-signals
//! (topology / doppler / range / bearing) from session-5.
//!
//! Head fields (ADR-006) are exposed via the trait methods so the
//! kernel boundary can validate frames without reaching into the
//! widget's internal state.

pub mod chip;
pub mod gauge;
pub mod plot;
pub mod pressable;
pub mod response;
pub mod stack;
pub mod stream_view;
pub mod strip;
pub mod table;
pub mod tree;
pub mod types;

pub use chip::{Chip, ChipTone};
pub use gauge::{Gauge, Thresholds};
pub use plot::Plot;
pub use pressable::Pressable;
pub use response::{Bearing, CanonResponse, Doppler, Range, Topology};
pub use stack::{Stack, StackAxis};
pub use stream_view::StreamView;
pub use strip::{CellSize, Strip, StripAxis};
pub use table::{Table, TableColumn, TableOutcome};
pub use tree::{Tree, TreeNode, TreeOutcome};
pub use types::{
    ActorKind, Affordance, Confidence, ConfidenceSource, FrozenBy, IdentityUri, Modality,
    MutationAxis, Tooltip, VariantId,
};

use eframe::egui;

/// Typed state payload. Primitives return whatever shape their
/// ontology IRI demands (`ui://field.value`, `ui://gauge.value`, …).
/// The kernel serialises this via `serde_json::Value` at the frame
/// boundary; renderer-side we keep it an opaque type-erased slot so
/// primitives stay zero-cost.
///
/// This is deliberately lighter than a full trait object: the renderer
/// never reads foreign primitive state, it only reads its own.
pub trait CanonState: std::fmt::Debug {}

/// Default no-state marker for primitives whose entire state is the
/// user's interaction (e.g. a bare `Pressable` with no toggle state).
#[derive(Copy, Clone, Debug, Default)]
pub struct Unit;
impl CanonState for Unit {}

/// Every primitive in the canon implements this trait. The renderer
/// calls `show()` once per frame; the six head-getter methods exist
/// so the kernel boundary (and observation walker) can interrogate
/// the primitive without forcing it to render.
///
/// See session-5-renderer-contracts.md:238-255 for the trait shape
/// and ADR-006 for field semantics.
pub trait CanonWidget {
    /// The egui id this primitive will use — stable across frames,
    /// derived from its ontology path. Used for memory-keyed state
    /// (first-seen frame, open-sets, variant echoes).
    fn id(&self) -> egui::Id;

    /// Ontology IRI stem under `ui://` (e.g. `ui://pressable`).
    fn identity_uri(&self) -> IdentityUri;

    /// Non-empty list for any primitive the user or agent may act
    /// upon. Already intersected with governance (ADR-006 §2).
    /// Empty slice = read-only *right now*, not malformed.
    fn affordances(&self) -> &[Affordance];

    /// ADR-006 §3 — how state was produced.
    fn confidence(&self) -> Confidence;

    /// Composer-assigned id, echoed on every return-signal (ADR-007).
    fn variant_id(&self) -> VariantId;

    /// Legal GEPA mutation axes. Empty slice = no mutation legal.
    fn mutation_axes(&self) -> &[MutationAxis];

    /// Optional hover / accessibility help (ADR-006 §7).
    fn tooltip(&self) -> Option<&Tooltip> {
        None
    }

    /// Render the primitive for one frame. Consumes `self` because
    /// most primitives carry their bound state by value and we don't
    /// want the caller to hold onto them across frames.
    fn show(self, ui: &mut egui::Ui) -> CanonResponse;
}
