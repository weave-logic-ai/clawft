//! Ontology snapshot re-export (ADR-017 §1; unified M1.5-D).
//!
//! The composer runtime reads the substrate state tree through an
//! [`OntologySnapshot`]. The canonical definition lives in
//! `clawft-substrate` alongside the `Substrate` state tree; this
//! module simply re-exports it so downstream callers can keep using
//! `clawft_surface::substrate::OntologySnapshot`.
//!
//! Prior to M1.5-D this crate held its own structurally-identical
//! copy to avoid a pre-merge dependency cycle. That cycle is now
//! resolved — the substrate crate owns the type, and the composer
//! builds on top of it.

pub use clawft_substrate::OntologySnapshot;
