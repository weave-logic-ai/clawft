//! Cosine-via-normalized-L2 discipline (agenticow `index.js:48`, `l2normalize`).
//!
//! `rvf-runtime` does not reliably persist a store's `metric` setting across
//! a reopen (the shipped binding agenticow wraps has the same gap, per the
//! integration plan §3). agenticow's fix -- and the one we copy verbatim --
//! is to never use `DistanceMetric::Cosine` at all: every store in a
//! `BranchableMemory` lineage is created with `DistanceMetric::L2`, and every
//! vector (ingest *and* query) is L2-normalized to unit length before it
//! touches `RvfStore`. On unit vectors, L2 order and cosine order are the
//! same permutation, so top-K is preserved whether or not the reopened
//! store "remembers" it was meant to be cosine similarity.
//!
//! This normalization is also what makes the chain-walk merge in
//! [`crate::chain_walk`] valid: distances returned by `RvfStore::query` on
//! different nodes in the lineage are only directly comparable (and
//! mergeable/re-sortable without recomputing) if every node uses the same
//! metric over the same (normalized) vector space.

/// L2-normalize a vector to unit length in place order (returns a new
/// `Vec<f32>`; never mutates the caller's slice).
///
/// A zero vector normalizes to itself (all zeros) rather than producing
/// `NaN` -- there is no meaningful direction to normalize toward, and
/// silently propagating `NaN` into a `.rvf` file would poison every
/// subsequent distance computation involving that cluster.
pub fn l2_normalize(vector: &[f32]) -> Vec<f32> {
    let norm_sq: f32 = vector.iter().map(|x| x * x).sum();
    if norm_sq <= f32::EPSILON {
        return vector.to_vec();
    }
    let norm = norm_sq.sqrt();
    vector.iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_to_unit_length() {
        let v = vec![3.0, 4.0];
        let n = l2_normalize(&v);
        let len: f32 = n.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((len - 1.0).abs() < 1e-6, "expected unit length, got {len}");
        assert!((n[0] - 0.6).abs() < 1e-6);
        assert!((n[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn zero_vector_stays_zero() {
        let v = vec![0.0, 0.0, 0.0];
        let n = l2_normalize(&v);
        assert_eq!(n, vec![0.0, 0.0, 0.0]);
    }
}
