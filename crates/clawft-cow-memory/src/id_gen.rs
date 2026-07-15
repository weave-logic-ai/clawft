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

/// Peek the next id this process would hand out, without allocating it --
/// used to stamp a watermark into the lineage manifest (`crate::manifest`)
/// so a restored lineage's ids are known even before this process ever
/// calls `next_vector_id()` again.
pub(crate) fn watermark() -> u64 {
    GLOBAL_VECTOR_ID.load(Ordering::Relaxed)
}

/// Bump the process-wide counter so it never hands out an id already used
/// by a lineage being restored from a manifest -- a no-op if `min` is not
/// past the current value. `fetch_max` rather than a plain store: several
/// lineages can be `open()`ed in the same process, each with its own
/// watermark, and the counter must end up past all of them.
pub(crate) fn bump_watermark(min: u64) {
    GLOBAL_VECTOR_ID.fetch_max(min, Ordering::Relaxed);
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
