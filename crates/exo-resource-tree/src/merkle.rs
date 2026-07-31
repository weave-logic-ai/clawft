//! Merkle proof generation and O(changed) tree diff (WEFT-106 / K6.4).
//!
//! Cross-node tree sync compares Merkle roots, then walks only subtrees whose
//! hashes differ. Unchanged sibling subtrees contribute **witness hashes**
//! (minimal payload) so a receiver can recompute ancestor hashes without a
//! full transfer.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::model::{ResourceId, ResourceKind, ResourceNode};
use crate::scoring::NodeScoring;
use crate::tree::{shake256_256, ResourceTree};

// ── Wire / API types ───────────────────────────────────────────────────────

/// Kind of change for a node in a Merkle diff.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MerkleDiffKind {
    /// Present locally, absent on remote.
    Added,
    /// Present on both sides with different content or hash.
    Modified,
    /// Absent locally, present on remote.
    Removed,
}

/// Compact node payload for applying a diff without the full in-memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffNodePayload {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub parent: Option<ResourceId>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub scoring: NodeScoring,
    #[serde(with = "crate::model::hash_serde")]
    pub merkle_hash: [u8; 32],
}

impl DiffNodePayload {
    fn from_node(node: &ResourceNode) -> Self {
        Self {
            id: node.id.clone(),
            kind: node.kind.clone(),
            parent: node.parent.clone(),
            metadata: node.metadata.clone(),
            scoring: node.scoring,
            merkle_hash: node.merkle_hash,
        }
    }
}

/// One changed path plus optional sibling witnesses along the root path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleDiffEntry {
    pub id: ResourceId,
    pub diff_kind: MerkleDiffKind,
    /// Full node when [`MerkleDiffKind::Added`] or [`MerkleDiffKind::Modified`].
    pub node: Option<DiffNodePayload>,
    /// Unchanged sibling hashes at the **parent** of `id` (minimal witness).
    /// Empty for root or when no siblings.
    pub sibling_witnesses: Vec<SiblingWitness>,
}

/// Witness for an unchanged sibling subtree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingWitness {
    pub id: ResourceId,
    #[serde(with = "crate::model::hash_serde")]
    pub merkle_hash: [u8; 32],
}

/// Result of comparing two trees (or a tree against a remote hash map).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleDiff {
    #[serde(with = "crate::model::hash_serde")]
    pub local_root: [u8; 32],
    #[serde(with = "crate::model::hash_serde")]
    pub remote_root: [u8; 32],
    /// Only changed nodes (+ removals). Unchanged subtrees are omitted.
    pub entries: Vec<MerkleDiffEntry>,
    /// Nodes visited during the walk (for O(changed) assertions / metrics).
    pub nodes_visited: usize,
}

impl MerkleDiff {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn in_sync(&self) -> bool {
        self.local_root == self.remote_root && self.entries.is_empty()
    }

    /// Count of Added/Modified/Removed entries (excludes pure walk noise).
    pub fn changed_count(&self) -> usize {
        self.entries.len()
    }
}

/// One level of a Merkle inclusion proof (parent recompute material).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProofLevel {
    /// Parent resource path at this level.
    pub parent_id: ResourceId,
    /// All child hashes of `parent_id`, sorted (same order as hash input).
    #[serde(with = "hash_list_serde")]
    pub child_hashes_sorted: Vec<[u8; 32]>,
    /// Parent scoring binding (24 bytes).
    #[serde(with = "scoring_bytes_serde")]
    pub parent_scoring: [u8; 24],
    /// Canonical metadata bytes of the parent (same as hash input).
    pub parent_meta_bytes: Vec<u8>,
    /// Expected parent Merkle hash after recompute.
    #[serde(with = "crate::model::hash_serde")]
    pub parent_hash: [u8; 32],
}

/// Cryptographic inclusion proof from a node up to the tree root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleInclusionProof {
    pub path: ResourceId,
    #[serde(with = "crate::model::hash_serde")]
    pub node_hash: [u8; 32],
    /// Levels from the node's parent toward root (deepest first).
    pub levels: Vec<MerkleProofLevel>,
    #[serde(with = "crate::model::hash_serde")]
    pub root_hash: [u8; 32],
}

impl MerkleInclusionProof {
    /// Recompute hashes leaf→root and check they match the stored chain.
    pub fn verify(&self) -> bool {
        if self.path.0.is_empty() {
            return false;
        }
        // Root node: no levels; node_hash must equal root_hash.
        if self.levels.is_empty() {
            return self.node_hash == self.root_hash && self.path.is_root();
        }

        let mut expected_child = self.node_hash;
        for (i, level) in self.levels.iter().enumerate() {
            // The proven child's hash must appear among the child hashes.
            if !level.child_hashes_sorted.contains(&expected_child) {
                return false;
            }
            let recomputed = hash_from_parts(
                &level.child_hashes_sorted,
                &level.parent_scoring,
                &level.parent_meta_bytes,
            );
            if recomputed != level.parent_hash {
                return false;
            }
            expected_child = level.parent_hash;
            if i + 1 == self.levels.len() && recomputed != self.root_hash {
                return false;
            }
        }
        expected_child == self.root_hash
    }
}

// ── Hash helpers (match ResourceTree::recompute_merkle) ────────────────────

fn meta_bytes_from_map(metadata: &HashMap<String, serde_json::Value>) -> Vec<u8> {
    let mut keys: Vec<&String> = metadata.keys().collect();
    keys.sort();
    let mut buf = Vec::new();
    for key in keys {
        buf.extend_from_slice(key.as_bytes());
        if let Some(val) = metadata.get(key) {
            buf.extend_from_slice(val.to_string().as_bytes());
        }
    }
    buf
}

fn hash_from_parts(
    child_hashes_sorted: &[[u8; 32]],
    scoring: &[u8; 24],
    meta_bytes: &[u8],
) -> [u8; 32] {
    let mut buf = Vec::with_capacity(child_hashes_sorted.len() * 32 + 24 + meta_bytes.len());
    for h in child_hashes_sorted {
        buf.extend_from_slice(h);
    }
    buf.extend_from_slice(scoring);
    buf.extend_from_slice(meta_bytes);
    shake256_256(&buf)
}

// ── ResourceTree API ───────────────────────────────────────────────────────

impl ResourceTree {
    /// Export every path → Merkle hash (for remote digest exchange).
    pub fn hash_map(&self) -> HashMap<ResourceId, [u8; 32]> {
        self.nodes
            .iter()
            .map(|(id, n)| (id.clone(), n.merkle_hash))
            .collect()
    }

    /// Generate a Merkle inclusion proof for `id` (WEFT-106).
    ///
    /// Returns `None` if the node is missing. Call after hashes are current
    /// ([`Self::recompute_path`] / [`Self::recompute_all`]).
    pub fn merkle_proof(&self, id: &ResourceId) -> Option<MerkleInclusionProof> {
        let node = self.nodes.get(id)?;
        let node_hash = node.merkle_hash;
        let root_hash = self.root_hash();

        if id.is_root() {
            return Some(MerkleInclusionProof {
                path: id.clone(),
                node_hash,
                levels: Vec::new(),
                root_hash,
            });
        }

        let mut levels = Vec::new();
        let mut current = id.clone();
        while let Some(parent_id) = self.nodes.get(&current).and_then(|n| n.parent.clone()) {
            let parent = self.nodes.get(&parent_id)?;
            let mut child_hashes: Vec<[u8; 32]> = parent
                .children
                .iter()
                .filter_map(|cid| self.nodes.get(cid).map(|c| c.merkle_hash))
                .collect();
            child_hashes.sort();
            levels.push(MerkleProofLevel {
                parent_id: parent_id.clone(),
                child_hashes_sorted: child_hashes,
                parent_scoring: parent.scoring.to_hash_bytes(),
                parent_meta_bytes: meta_bytes_from_map(&parent.metadata),
                parent_hash: parent.merkle_hash,
            });
            current = parent_id;
        }

        Some(MerkleInclusionProof {
            path: id.clone(),
            node_hash,
            levels,
            root_hash,
        })
    }

    /// Verify an inclusion proof against this tree's current root hash.
    pub fn verify_proof(&self, proof: &MerkleInclusionProof) -> bool {
        proof.root_hash == self.root_hash() && proof.verify()
    }

    /// Diff `self` against `other` by walking only differing Merkle subtrees.
    ///
    /// Complexity is **O(changed + depth · witnesses)**, not O(tree): when a
    /// subtree root hash matches, the walk does not descend.
    pub fn merkle_diff(&self, other: &ResourceTree) -> MerkleDiff {
        let remote_hashes = other.hash_map();
        self.merkle_diff_against_hashes(&remote_hashes, Some(other))
    }

    /// Diff `self` against a remote `path → hash` map.
    ///
    /// When `remote_tree` is `Some`, full node payloads and accurate
    /// Added/Modified classification use the remote tree. When only hashes
    /// are known, nodes with missing remote paths are [`MerkleDiffKind::Added`]
    /// and hash mismatches are [`MerkleDiffKind::Modified`]; remote-only paths
    /// (Removed) are detected from the hash map keys.
    pub fn merkle_diff_against_hashes(
        &self,
        remote_hashes: &HashMap<ResourceId, [u8; 32]>,
        remote_tree: Option<&ResourceTree>,
    ) -> MerkleDiff {
        let local_root = self.root_hash();
        let remote_root = remote_hashes
            .get(&ResourceId::root())
            .copied()
            .unwrap_or([0u8; 32]);

        let mut entries = Vec::new();
        let mut visited = 0usize;

        if local_root == remote_root && remote_hashes.contains_key(&ResourceId::root()) {
            return MerkleDiff {
                local_root,
                remote_root,
                entries,
                nodes_visited: 1,
            };
        }

        let mut queue: VecDeque<ResourceId> = VecDeque::new();
        queue.push_back(ResourceId::root());
        let mut seen: HashSet<ResourceId> = HashSet::new();

        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            visited += 1;

            let local = self.nodes.get(&id);
            let remote_hash = remote_hashes.get(&id).copied();
            let remote_node = remote_tree.and_then(|t| t.nodes.get(&id));

            match (local, remote_hash) {
                (Some(lnode), Some(rh)) if lnode.merkle_hash == rh => {
                    // Subtrees match — do not descend.
                    continue;
                }
                (Some(lnode), Some(_rh)) => {
                    // Hash mismatch: content change and/or children.
                    let content_diff = match remote_node {
                        Some(rnode) => {
                            rnode.kind != lnode.kind
                                || rnode.metadata != lnode.metadata
                                || rnode.scoring.as_array() != lnode.scoring.as_array()
                        }
                        // No remote node body: treat as modified payload needed.
                        None => true,
                    };

                    // Emit payload only when this node's own content differs.
                    // Ancestors that differ solely via children are skipped
                    // (children are walked next) — keeps the delta minimal.
                    if content_diff {
                        entries.push(MerkleDiffEntry {
                            id: id.clone(),
                            diff_kind: MerkleDiffKind::Modified,
                            node: Some(DiffNodePayload::from_node(lnode)),
                            sibling_witnesses: self.sibling_witnesses_for(&id),
                        });
                    }

                    // Descend into union of children.
                    let mut child_ids: HashSet<ResourceId> =
                        lnode.children.iter().cloned().collect();
                    if let Some(rnode) = remote_node {
                        for c in &rnode.children {
                            child_ids.insert(c.clone());
                        }
                    }
                    for c in child_ids {
                        queue.push_back(c);
                    }
                }
                (Some(lnode), None) => {
                    // Local-only: emit whole local subtree as Added.
                    self.collect_subtree_added(&id, &mut entries, &mut visited);
                    let _ = lnode;
                }
                (None, Some(_)) => {
                    entries.push(MerkleDiffEntry {
                        id: id.clone(),
                        diff_kind: MerkleDiffKind::Removed,
                        node: None,
                        sibling_witnesses: Vec::new(),
                    });
                    if let Some(rnode) = remote_node {
                        for c in &rnode.children {
                            queue.push_back(c.clone());
                        }
                    }
                }
                (None, None) => {}
            }
        }

        // Remote-only paths not reached via walk (hash-map only mode, or
        // disconnected). Cover keys in remote_hashes missing locally.
        for rid in remote_hashes.keys() {
            if self.nodes.contains_key(rid) {
                continue;
            }
            if entries.iter().any(|e| e.id == *rid) {
                continue;
            }
            // Only emit if parent is shared or is root of a remote-only branch
            // we didn't walk (e.g. hash map without remote tree structure).
            entries.push(MerkleDiffEntry {
                id: rid.clone(),
                diff_kind: MerkleDiffKind::Removed,
                node: None,
                sibling_witnesses: Vec::new(),
            });
            visited += 1;
        }

        MerkleDiff {
            local_root,
            remote_root,
            entries,
            nodes_visited: visited,
        }
    }

    fn sibling_witnesses_for(&self, id: &ResourceId) -> Vec<SiblingWitness> {
        let parent_id = match self.nodes.get(id).and_then(|n| n.parent.clone()) {
            Some(p) => p,
            None => return Vec::new(),
        };
        let parent = match self.nodes.get(&parent_id) {
            Some(p) => p,
            None => return Vec::new(),
        };
        parent
            .children
            .iter()
            .filter(|c| *c != id)
            .filter_map(|cid| {
                self.nodes.get(cid).map(|n| SiblingWitness {
                    id: cid.clone(),
                    merkle_hash: n.merkle_hash,
                })
            })
            .collect()
    }

    fn collect_subtree_added(
        &self,
        id: &ResourceId,
        entries: &mut Vec<MerkleDiffEntry>,
        visited: &mut usize,
    ) {
        let mut stack = vec![id.clone()];
        while let Some(cid) = stack.pop() {
            *visited += 1;
            if let Some(node) = self.nodes.get(&cid) {
                entries.push(MerkleDiffEntry {
                    id: cid.clone(),
                    diff_kind: MerkleDiffKind::Added,
                    node: Some(DiffNodePayload::from_node(node)),
                    sibling_witnesses: self.sibling_witnesses_for(&cid),
                });
                for child in node.children.iter().rev() {
                    stack.push(child.clone());
                }
            }
        }
    }
}

// ── Serde helpers for hash lists ───────────────────────────────────────────

mod hash_list_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(hashes: &[[u8; 32]], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hexes: Vec<String> = hashes
            .iter()
            .map(|h| h.iter().map(|b| format!("{b:02x}")).collect())
            .collect();
        hexes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<[u8; 32]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hexes = Vec::<String>::deserialize(deserializer)?;
        hexes
            .into_iter()
            .map(|s| {
                let mut out = [0u8; 32];
                if s.len() != 64 {
                    return Err(serde::de::Error::custom("hash hex must be 64 chars"));
                }
                for i in 0..32 {
                    out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                        .map_err(serde::de::Error::custom)?;
                }
                Ok(out)
            })
            .collect()
    }
}

mod scoring_bytes_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 24], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 24], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut out = [0u8; 24];
        if s.len() != 48 {
            return Err(serde::de::Error::custom("scoring hex must be 48 chars"));
        }
        for i in 0..24 {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(out)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ResourceKind;

    fn build_tree() -> ResourceTree {
        let mut tree = ResourceTree::new();
        for path in ["/a", "/a/b", "/a/b/c", "/x", "/x/y"] {
            let id = ResourceId::new(path);
            let parent = id.parent().unwrap();
            tree.insert(id, ResourceKind::Namespace, parent).unwrap();
        }
        tree.recompute_all();
        tree
    }

    fn wide_tree(width: usize, depth: usize) -> ResourceTree {
        let mut tree = ResourceTree::new();
        for w in 0..width {
            let mut parent = ResourceId::root();
            for d in 0..depth {
                let id = if d == 0 {
                    ResourceId::new(format!("/b{w}"))
                } else {
                    ResourceId::new(format!("/b{w}/d{d}"))
                };
                tree.insert(id.clone(), ResourceKind::Namespace, parent)
                    .unwrap();
                parent = id;
            }
        }
        tree.recompute_all();
        tree
    }

    #[test]
    fn identical_trees_empty_diff() {
        let a = build_tree();
        let b = build_tree();
        let diff = a.merkle_diff(&b);
        assert!(diff.in_sync());
        assert!(diff.is_empty());
        assert_eq!(diff.nodes_visited, 1);
    }

    #[test]
    fn single_leaf_change_is_minimal() {
        let mut changed = build_tree();
        changed
            .get_mut(&ResourceId::new("/a/b/c"))
            .unwrap()
            .metadata
            .insert("k".into(), serde_json::json!("v"));
        changed.recompute_path(&ResourceId::new("/a/b/c"));

        let base = build_tree();
        let diff = changed.merkle_diff(&base);
        assert!(!diff.in_sync());

        // Only the content-changed node should appear as Modified payload;
        // ancestors differ only via children so they are not full payloads.
        let modified: Vec<_> = diff
            .entries
            .iter()
            .filter(|e| e.diff_kind == MerkleDiffKind::Modified)
            .collect();
        assert_eq!(
            modified.len(),
            1,
            "entries: {:?}",
            diff.entries
                .iter()
                .map(|e| e.id.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(modified[0].id, ResourceId::new("/a/b/c"));
        assert!(modified[0].node.is_some());

        // Walk must not visit the unrelated /x branch.
        assert!(
            diff.nodes_visited < base.len(),
            "visited {} of {}",
            diff.nodes_visited,
            base.len()
        );
        // /x and /x/y should not appear in entries.
        assert!(!diff.entries.iter().any(|e| e.id.0.starts_with("/x")));
    }

    #[test]
    fn diff_is_o_changed_not_o_tree() {
        let base = wide_tree(20, 5); // ~101 nodes
        let mut changed = wide_tree(20, 5);
        let leaf = ResourceId::new("/b0/d4");
        changed
            .get_mut(&leaf)
            .unwrap()
            .metadata
            .insert("only".into(), serde_json::json!(1));
        changed.recompute_path(&leaf);

        let diff = changed.merkle_diff(&base);
        let total = base.len();
        assert!(total > 50, "fixture should be large, got {total}");
        // Path length ~6 + a few siblings enqueued; far below full tree.
        assert!(
            diff.nodes_visited < total / 3,
            "expected O(changed) visit {}, tree {}",
            diff.nodes_visited,
            total
        );
        assert_eq!(
            diff.entries
                .iter()
                .filter(|e| e.diff_kind == MerkleDiffKind::Modified)
                .count(),
            1
        );
    }

    #[test]
    fn added_subtree_emits_all_new_nodes() {
        let base = build_tree();
        let mut local = build_tree();
        local
            .insert(
                ResourceId::new("/new"),
                ResourceKind::Namespace,
                ResourceId::root(),
            )
            .unwrap();
        local
            .insert(
                ResourceId::new("/new/child"),
                ResourceKind::Service,
                ResourceId::new("/new"),
            )
            .unwrap();
        local.recompute_path(&ResourceId::new("/new/child"));

        let diff = local.merkle_diff(&base);
        let added: Vec<_> = diff
            .entries
            .iter()
            .filter(|e| e.diff_kind == MerkleDiffKind::Added)
            .map(|e| e.id.to_string())
            .collect();
        assert!(added.contains(&"/new".to_string()));
        assert!(added.contains(&"/new/child".to_string()));
    }

    #[test]
    fn removed_node_detected() {
        // remote keeps /x/y; local removes it
        let remote = build_tree();
        let mut local = build_tree();
        local.remove(ResourceId::new("/x/y")).unwrap();
        local.recompute_path(&ResourceId::new("/x"));

        let diff = local.merkle_diff(&remote);
        assert!(
            diff.entries
                .iter()
                .any(|e| e.diff_kind == MerkleDiffKind::Removed && e.id == ResourceId::new("/x/y")),
            "entries: {:?}",
            diff.entries
                .iter()
                .map(|e| (e.id.to_string(), e.diff_kind))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn merkle_proof_verifies() {
        let tree = build_tree();
        let proof = tree
            .merkle_proof(&ResourceId::new("/a/b/c"))
            .expect("proof");
        assert!(proof.verify());
        assert!(tree.verify_proof(&proof));
        assert_eq!(proof.root_hash, tree.root_hash());
        assert!(!proof.levels.is_empty());
    }

    #[test]
    fn merkle_proof_root() {
        let tree = build_tree();
        let proof = tree.merkle_proof(&ResourceId::root()).unwrap();
        assert!(proof.verify());
        assert!(proof.levels.is_empty());
    }

    #[test]
    fn merkle_proof_tamper_fails() {
        let tree = build_tree();
        let mut proof = tree.merkle_proof(&ResourceId::new("/a/b/c")).unwrap();
        proof.node_hash[0] ^= 0xff;
        assert!(!proof.verify());
    }

    #[test]
    fn proof_serde_roundtrip() {
        let tree = build_tree();
        let proof = tree.merkle_proof(&ResourceId::new("/a")).unwrap();
        let json = serde_json::to_string(&proof).unwrap();
        let back: MerkleInclusionProof = serde_json::from_str(&json).unwrap();
        assert!(back.verify());
        assert_eq!(back.path, proof.path);
    }

    #[test]
    fn sibling_witnesses_on_modified() {
        let base = build_tree();
        let mut changed = build_tree();
        changed
            .get_mut(&ResourceId::new("/a/b/c"))
            .unwrap()
            .metadata
            .insert("k".into(), serde_json::json!("v"));
        changed.recompute_path(&ResourceId::new("/a/b/c"));

        let diff = changed.merkle_diff(&base);
        let entry = diff
            .entries
            .iter()
            .find(|e| e.id == ResourceId::new("/a/b/c"))
            .unwrap();
        // /a/b has only child /a/b/c — no siblings at that level.
        // Witnesses may be empty; ensure field is present and serializable.
        let _ = &entry.sibling_witnesses;
        let json = serde_json::to_string(&diff).unwrap();
        let back: MerkleDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(back.changed_count(), diff.changed_count());
    }

    #[test]
    fn hash_map_covers_all_nodes() {
        let tree = build_tree();
        let map = tree.hash_map();
        assert_eq!(map.len(), tree.len());
        assert_eq!(map.get(&ResourceId::root()), Some(&tree.root_hash()));
    }
}
