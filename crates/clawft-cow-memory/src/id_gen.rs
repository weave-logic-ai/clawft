//! Global monotonic auto-id (agenticow `index.js:36`, `GLOBAL_AUTO_ID`).
//!
//! A per-`BranchableMemory` counter would hand out the same id from a base
//! and from each of its forks/checkpoints -- a later `promote()` would then
//! silently overwrite one lineage's vector with another's. agenticow avoids
//! this with a single process-wide counter; a Rust process is the same unit
//! of "one JS process" that guarantee assumes, so a `static AtomicU64` here
//! is the direct, correct port. Callers that need cross-*process* uniqueness
//! (e.g. a promoted brain shared by multiple daemons) must supply their own
//! ids -- `ingest` only auto-assigns for ids left as `None`.

use std::sync::atomic::{AtomicU64, Ordering};

static GLOBAL_VECTOR_ID: AtomicU64 = AtomicU64::new(1);

/// Reserve and return the next process-wide auto-assigned vector id.
pub fn next_vector_id() -> u64 {
    GLOBAL_VECTOR_ID.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_monotone_and_unique() {
        let a = next_vector_id();
        let b = next_vector_id();
        let c = next_vector_id();
        assert!(a < b);
        assert!(b < c);
    }
}
