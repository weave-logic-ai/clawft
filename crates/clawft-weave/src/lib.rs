//! `clawft-weave` — shared library surface for the `weaver` CLI.
//!
//! The CLI binary lives in [`main.rs`](../../../src/main.rs). This lib
//! target re-exports the modules used by integration tests (notably
//! [`daemon::handle_connection`] for driving a kernel listener from a
//! temp socket). Keeping these modules public under `pub` rather than
//! `pub(crate)` is the minimum change required to make integration
//! tests link against them.

pub mod capability;
/// Tracing → pending-buffer bridge for ExoChain (WEFT-597).
pub mod chain_bridge;
pub mod client;
pub mod commands;
pub mod control;
pub mod conv_postmortem;
#[cfg(unix)]
pub mod daemon;
#[cfg(unix)]
pub mod llm_service;
/// Live MCP registry RPC handlers (WEFT-494 / ADR-070). Available on all
/// platforms so unit tests can exercise add/list/remove without UDS.
pub mod mcp_rpc;
pub mod node_identity;
pub mod protocol;
/// WEFT-720: `ecc.spatial.*` RPC + process-local BvhStore scaffold.
#[cfg(unix)]
pub mod spatial_rpc;
#[cfg(feature = "rvf-rpc")]
pub mod rvf_codec;
#[cfg(feature = "rvf-rpc")]
pub mod rvf_rpc;
#[cfg(feature = "exochain")]
pub mod turn_ledger;
pub mod voice_loop;
pub mod voice_router;
pub mod voice_trace;
