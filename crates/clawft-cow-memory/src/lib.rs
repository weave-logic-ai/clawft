//! Branchable, checkpointable memory over `rvf_runtime::RvfStore`.
//!
//! This crate is the Rust port of [agenticow](https://github.com/ruvnet/agenticow)'s
//! ~650-line JS orchestration layer, built directly on the same
//! `rvf-runtime` crate agenticow's native binding (`@ruvector/rvf-node`)
//! wraps -- WeftOS already depends on it (`Cargo.toml:208`). See
//! `.planning/ruv/integration/agenticow-integration-plan.md` for the full
//! rationale (§0) and API mapping (§3); this crate implements that plan's
//! Phase 1: a standalone crate with no kernel wiring yet.
//!
//! # What this gives you
//!
//! [`BranchableMemory`] wraps one COW lineage of `.rvf` files and gives you:
//!
//! - `checkpoint()` / `rollback()` -- O(1) snapshot and cheap discard of a
//!   speculative turn's memory writes.
//! - `promote()` -- commit a turn's learnings into the lineage's durable
//!   `base`.
//! - `branch()` / `fork()` -- independent lineages that share read-only
//!   history but never leak writes back to their source.
//! - `ingest()` / `delete()` / `query()` -- the read/write surface, with a
//!   manual chain-walk merge on `query()` standing in for the native
//!   cross-COW-boundary read-through that only ships today inside
//!   `@ruvector/rvf-node`'s `linux-x64-gnu` binary (integration plan §6) --
//!   above the published Rust `rvf-runtime` crate this module builds on.
//! - `diff()` / `lineage()` / `status()` -- introspection.
//!
//! # Not yet in scope (see the integration plan's later phases)
//!
//! - Hermes-loop wiring (`AgentLoop::handle_turn` checkpoint/promote/rollback
//!   bracketing) -- Phase 2.
//! - `ChainManager`/exochain witness coupling -- Phase 3.
//! - Crash recovery / orphaned-file reaping across a process restart --
//!   Phase 3 risk item; [`BranchableMemory::open`] is an explicitly lossy
//!   partial reopen, not that.
//! - The `embedder_id` stamp-and-enforce discipline the plan calls for
//!   (§3, "DESIGN CONSTRAINT") -- `RvfOptions` has no field for it and this
//!   crate does not yet add a sidecar for one; see the crate's test/report
//!   notes for why this is flagged as a Phase 2 open question rather than
//!   silently skipped.

mod branchable_memory;
mod chain_walk;
mod error;
mod id_gen;
#[cfg(target_os = "macos")]
mod macos_errno_shim;
mod node;
mod normalize;
mod ops;

pub use branchable_memory::{
    BranchableMemory, LineageEntry, MemoryDiff, MemoryStatus, NodeRole, PromoteReport, VectorItem,
};
pub use chain_walk::ScoredId;
pub use error::{CowMemoryError, Result};
pub use node::CheckpointId;

// Re-exported so callers can build advanced `RvfOptions` (HNSW parameters,
// witness config, security policy) for `BranchableMemory::create_with_options`
// without taking a direct `rvf-runtime` dependency of their own.
pub use rvf_runtime::RvfOptions;
