//! Merkle Mountain Range — append-only membership structure.

use exo_core::{blake3_hash, Blake3Hash};

/// Incremental Merkle Mountain Range over leaf digests.
///
/// Peaks form a forest; [`root`](Self::root) folds peaks left-to-right with
/// domain-separated hashing. Suitable for chain checkpoint commitments.
#[derive(Debug, Clone, Default)]
pub struct MerkleMountainRange {
    /// Complete binary trees (peaks), each entry is a node hash + leaf count power.
    peaks: Vec<(Blake3Hash, u32)>, // (hash, height) height 0 = leaf
    /// All leaves for proof generation (stub: keeps full list in memory).
    leaves: Vec<Blake3Hash>,
}

impl MerkleMountainRange {
    /// Empty MMR.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of leaves.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Append a leaf digest; returns leaf index.
    pub fn push(&mut self, leaf: Blake3Hash) -> usize {
        let idx = self.leaves.len();
        self.leaves.push(leaf);
        self.peaks.push((leaf, 0));
        self.merge_peaks();
        idx
    }

    fn merge_peaks(&mut self) {
        while self.peaks.len() >= 2 {
            let n = self.peaks.len();
            let (h1, e1) = self.peaks[n - 1];
            let (h0, e0) = self.peaks[n - 2];
            if e0 != e1 {
                break;
            }
            let parent = hash_node(h0, h1);
            self.peaks.pop();
            self.peaks.pop();
            self.peaks.push((parent, e0 + 1));
        }
    }

    /// Bag peaks into a single root (ZERO if empty).
    pub fn root(&self) -> Blake3Hash {
        if self.peaks.is_empty() {
            return Blake3Hash::ZERO;
        }
        let mut iter = self.peaks.iter().map(|(h, _)| *h);
        let mut acc = iter.next().unwrap();
        for h in iter {
            acc = hash_node(acc, h);
        }
        acc
    }

    /// Inclusion proof for leaf at `index` (sibling hashes from leaf to peak,
    /// then peak bagging path is omitted in this minimal stub — peak list is
    /// attached for verification against current root).
    pub fn proof(&self, index: usize) -> Option<MmrProof> {
        if index >= self.leaves.len() {
            return None;
        }
        // Minimal stub proof: recompute by replaying all leaves (correct but O(n)).
        // Full mountain-path proofs can replace this without API change.
        Some(MmrProof {
            leaf_index: index,
            leaf: self.leaves[index],
            all_leaves: self.leaves.clone(),
        })
    }

    /// Verify a proof against an expected root.
    pub fn verify(leaf: Blake3Hash, index: usize, proof: &MmrProof, root: Blake3Hash) -> bool {
        if proof.leaf_index != index || proof.leaf != leaf {
            return false;
        }
        if index >= proof.all_leaves.len() || proof.all_leaves[index] != leaf {
            return false;
        }
        let mut mmr = MerkleMountainRange::new();
        for l in &proof.all_leaves {
            mmr.push(*l);
        }
        mmr.root() == root
    }
}

/// Proof payload for an MMR leaf (minimal in-memory form).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmrProof {
    /// Leaf position.
    pub leaf_index: usize,
    /// Leaf digest.
    pub leaf: Blake3Hash,
    /// Full leaf list (stub). Future: mountain path + peaks only.
    pub all_leaves: Vec<Blake3Hash>,
}

fn hash_node(left: Blake3Hash, right: Blake3Hash) -> Blake3Hash {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(left.as_bytes());
    buf[32..].copy_from_slice(right.as_bytes());
    // domain tag for interior nodes
    let mut data = Vec::with_capacity(65);
    data.push(0x01);
    data.extend_from_slice(&buf);
    blake3_hash(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exo_core::blake3_hash;

    #[test]
    fn mmr_roots_change_on_append() {
        let mut mmr = MerkleMountainRange::new();
        assert_eq!(mmr.root(), Blake3Hash::ZERO);
        mmr.push(blake3_hash(b"a"));
        let r1 = mmr.root();
        mmr.push(blake3_hash(b"b"));
        let r2 = mmr.root();
        assert_ne!(r1, r2);
    }

    #[test]
    fn mmr_proof_verifies() {
        let mut mmr = MerkleMountainRange::new();
        let leaves: Vec<_> = (0..5).map(|i| blake3_hash(&[i])).collect();
        for l in &leaves {
            mmr.push(*l);
        }
        let root = mmr.root();
        for (i, l) in leaves.iter().enumerate() {
            let p = mmr.proof(i).unwrap();
            assert!(MerkleMountainRange::verify(*l, i, &p, root));
        }
    }
}
