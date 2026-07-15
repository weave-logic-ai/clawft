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
//! - `pause()` / `resume()` -- park a lineage in place (freeze `working`
//!   without deriving a fresh child) for AutoPause/AutoResume idle-loop
//!   parking; reads keep working through the chain-walk while parked,
//!   writes fail closed with [`CowMemoryError::Paused`] until `resume()`.
//! - `ingest()` / `delete()` / `query()` -- the read/write surface, with a
//!   manual chain-walk merge on `query()` standing in for the native
//!   cross-COW-boundary read-through that only ships today inside
//!   `@ruvector/rvf-node`'s `linux-x64-gnu` binary (integration plan §6) --
//!   above the published Rust `rvf-runtime` crate this module builds on.
//! - `diff()` / `lineage()` / `status()` -- introspection.
//! - A durable lineage manifest (`crate::manifest`), written on every
//!   topology change, that makes [`BranchableMemory::open`] restore the
//!   *full* lineage (base, checkpoints, inherited chain, tombstones, id
//!   watermarks) rather than the base-only reopen from before -- see that
//!   module's docs for exactly what is and isn't restored.
//!
//! # Not yet in scope (see the integration plan's later phases)
//!
//! - Hermes-loop wiring (`AgentLoop::handle_turn` checkpoint/promote/rollback
//!   bracketing) -- Phase 2.
//! - `ChainManager`/exochain witness coupling -- Phase 3.
//! - Crash recovery for an in-flight (not yet checkpointed/paused) turn --
//!   the manifest closes the *topology* half of this gap (see
//!   `crate::manifest`), but `working`'s uncommitted edits between
//!   checkpoints are still lost on an unclean restart, by design (a
//!   manifest write is not triggered per-`ingest`).
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
mod manifest;
mod node;
mod normalize;
mod ops;
mod types;

pub use branchable_memory::BranchableMemory;
pub use chain_walk::ScoredId;
pub use error::{CowMemoryError, Result};
pub use node::CheckpointId;
pub use types::{
    LineageEntry, MemoryDiff, MemoryStatus, NodeRole, PromoteReport, VectorItem, VectorTags,
};

// Re-exported so callers can build advanced `RvfOptions` (HNSW parameters,
// witness config, security policy) for `BranchableMemory::create_with_options`
// without taking a direct `rvf-runtime` dependency of their own.
pub use rvf_runtime::RvfOptions;
