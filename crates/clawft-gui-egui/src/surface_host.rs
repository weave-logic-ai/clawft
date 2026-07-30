//! ADR-016 surface-description host — re-exported from
//! [`clawft_surface::compose`] (WEFT-427).
//!
//! The composer runtime walks a [`clawft_surface::SurfaceTree`] and
//! drives the shared [`clawft_canon`] primitives. It lived here during
//! M1.5-D to break a crate cycle; with canon extracted it lives in
//! `clawft-surface` again. This module preserves the historical
//! `clawft_gui_egui::surface_host` import path for shell + tests.

pub use clawft_surface::compose::*;
