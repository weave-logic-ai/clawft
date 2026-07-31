//! `WindowIntent` bus — re-export of the shared types crate surface.
//!
//! Authoritative types live in [`clawft_types::window_intent`] so MCP tools
//! (WEFT-695 / WEFT-702) and the egui shell emit the same serde-stable
//! intents via [`ExternalIntentTx::submit_mcp`].

pub use clawft_types::window_intent::*;
