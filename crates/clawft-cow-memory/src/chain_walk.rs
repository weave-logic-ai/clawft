//! The cross-platform correctness fallback for reading a COW lineage.
//!
//! `RvfStore::query` (crates.io 0.2.0) does not read through the COW
//! boundary -- it only scans its own `vectors` (verified against
//! `store.rs:314-368`: the loop is over `self.vectors.ids()`, nothing else).
//! The native dual-graph HNSW merge that *would* read through the boundary
//! ships only inside `@ruvector/rvf-node`'s `linux-x64-gnu` binary, which is
//! above the published Rust crate entirely (integration plan §6). So on
//! every platform we support today, a lineage-aware query means: query every
//! node in the chain ourselves, newest first, and merge.
//!
//! This is exactly agenticow's own default path (`index.js:301-324`), not a
//! degraded fallback we're stuck with -- it's correct on every platform
//! because it never assumes the native fast path exists.

use rvf_runtime::QueryOptions;

use crate::error::Result;
use crate::node::MemoryNode;

/// A single chain-walk result: a vector id and its distance from the query
/// vector, comparable across every node in the lineage because the whole
/// lineage shares one metric (`DistanceMetric::L2`) over one normalized
/// vector space (see `crate::normalize`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoredId {
    pub id: u64,
    pub distance: f32,
}

/// Per-node overscan factor. agenticow overscans `k*4` per node
/// (`index.js:304`) before merging and re-ranking, so that a node whose
/// nearest-by-its-own-metric candidates get masked by a newer node's
/// tombstones still leaves enough headroom to fill out `k` results.
const OVERSCAN_FACTOR: usize = 4;

/// Query every node in `chain` (already in priority order: nearest/newest
/// first, oldest/base-and-inherited last) and merge into the top `k`.
///
/// `chain[0]` (the topmost / newest node, i.e. `working`) wins ties over
/// everything below it: for a given id, the first node in the walk that
/// returns it is authoritative, and every node's tombstones mask that id
/// from surfacing there *and* from every node still older. This is
/// agenticow's "child wins on id collision" contract (plan §3), implemented
/// explicitly here because RVF's own COW read-through does not exist on
/// this platform.
///
/// A node's tombstones are checked against its *own* results too (not only
/// propagated to older nodes) -- deliberately: `BranchableMemory::delete`
/// never calls `RvfStore::delete`, precisely so this crate-level tombstone
/// set is the single source of truth for "is this id hidden here," at
/// every level including same-node re-ingest-after-delete. See the doc
/// comment on `BranchableMemory::delete` for why relying on RVF's own
/// `deletion_bitmap` for that would be actively wrong (its delete bit
/// outlives a later re-ingest of the same id, and outlives `compact()` too).
pub(crate) fn query_chain(
    chain: &[&MemoryNode],
    query_vector: &[f32],
    k: usize,
    query_opts: &QueryOptions,
) -> Result<Vec<ScoredId>> {
    let overscan_k = k.saturating_mul(OVERSCAN_FACTOR).max(k).max(1);

    let mut masked: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut merged: Vec<ScoredId> = Vec::new();

    for node in chain {
        let results = node.store.query(query_vector, overscan_k, query_opts)?;
        for r in results {
            if masked.contains(&r.id) || node.tombstones.contains(&r.id) || seen.contains(&r.id) {
                continue;
            }
            seen.insert(r.id);
            merged.push(ScoredId {
                id: r.id,
                distance: r.distance,
            });
        }
        // Shadow every *older* node from here on. Applied after scanning
        // this node's own results (which are checked against
        // `node.tombstones` directly, above) so a newer node's tombstones
        // never mask something a still-newer node already returned.
        masked.extend(node.tombstones.iter().copied());
    }

    merged.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    merged.truncate(k);
    Ok(merged)
}

#[cfg(test)]
mod tests {
    // Exercised end-to-end via `tests/tombstone.rs` and `tests/acceptance.rs`
    // against real `RvfStore` instances -- chain-walk merge logic isn't
    // meaningfully unit-testable without a real store to query.
}
