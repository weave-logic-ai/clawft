//! Sparse Merkle Tree over a 256-bit keyspace (in-memory stub).

use std::collections::BTreeMap;

use exo_core::{blake3_hash, Blake3Hash};

/// In-memory sparse Merkle tree: maps `Blake3Hash` keys → value digests.
///
/// Root is a domain-separated fold of sorted `(key, value)` pairs. This is a
/// **contract stub** for resource-tree / state digests — not a full 256-level
/// SMT. The API surface (`update` / `get` / `root` / `proof`) matches the K6
/// seam so a denser implementation can drop in later.
#[derive(Debug, Clone, Default)]
pub struct SparseMerkleTree {
    entries: BTreeMap<Blake3Hash, Blake3Hash>,
}

impl SparseMerkleTree {
    /// Empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a key → value digest.
    pub fn update(&mut self, key: Blake3Hash, value: Blake3Hash) {
        self.entries.insert(key, value);
    }

    /// Remove a key (no-op if absent).
    pub fn delete(&mut self, key: &Blake3Hash) {
        self.entries.remove(key);
    }

    /// Lookup value digest.
    pub fn get(&self, key: &Blake3Hash) -> Option<Blake3Hash> {
        self.entries.get(key).copied()
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Commitment root over sorted entries (ZERO if empty).
    pub fn root(&self) -> Blake3Hash {
        if self.entries.is_empty() {
            return Blake3Hash::ZERO;
        }
        let mut hasher_input = Vec::new();
        hasher_input.push(0x02); // SMT domain
        for (k, v) in &self.entries {
            hasher_input.extend_from_slice(k.as_bytes());
            hasher_input.extend_from_slice(v.as_bytes());
        }
        blake3_hash(&hasher_input)
    }

    /// Membership proof: sorted sibling key/value pairs excluding `key`
    /// (stub form — verifier recomputes root with claimed leaf).
    pub fn proof(&self, key: &Blake3Hash) -> Option<SmtProof> {
        let value = self.get(key)?;
        let siblings: Vec<(Blake3Hash, Blake3Hash)> = self
            .entries
            .iter()
            .filter(|(k, _)| *k != key)
            .map(|(k, v)| (*k, *v))
            .collect();
        Some(SmtProof {
            key: *key,
            value,
            siblings,
        })
    }

    /// Verify membership against a root.
    pub fn verify(proof: &SmtProof, root: Blake3Hash) -> bool {
        let mut tree = SparseMerkleTree::new();
        tree.update(proof.key, proof.value);
        for (k, v) in &proof.siblings {
            tree.update(*k, *v);
        }
        tree.root() == root
    }
}

/// SMT membership proof (minimal stub encoding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtProof {
    /// Claimed key.
    pub key: Blake3Hash,
    /// Claimed value digest.
    pub value: Blake3Hash,
    /// Other leaves needed to recompute the root.
    pub siblings: Vec<(Blake3Hash, Blake3Hash)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use exo_core::blake3_hash;

    #[test]
    fn smt_update_get_root() {
        let mut t = SparseMerkleTree::new();
        let k = blake3_hash(b"key");
        let v = blake3_hash(b"val");
        t.update(k, v);
        assert_eq!(t.get(&k), Some(v));
        let r1 = t.root();
        t.update(blake3_hash(b"key2"), blake3_hash(b"val2"));
        assert_ne!(t.root(), r1);
    }

    #[test]
    fn smt_proof_roundtrip() {
        let mut t = SparseMerkleTree::new();
        for i in 0..4u8 {
            t.update(blake3_hash(&[i]), blake3_hash(&[i + 10]));
        }
        let root = t.root();
        let key = blake3_hash(&[1]);
        let proof = t.proof(&key).unwrap();
        assert!(SparseMerkleTree::verify(&proof, root));
        // wrong root fails
        assert!(!SparseMerkleTree::verify(&proof, Blake3Hash::ZERO));
    }
}
