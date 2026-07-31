//! # exo-dag — K6 content-addressed DAG substrate (ADR-043 / WEFT-619)
//!
//! In-tree stubs mapping the exo-dag contract used by SPARC / ADR-043:
//!
//! | Component | Role |
//! |-----------|------|
//! | [`DagStore`] | Append-only, content-addressed event DAG (parents + payload) |
//! | [`MerkleMountainRange`] | Incremental append-only membership proofs (chain checkpoints) |
//! | [`SparseMerkleTree`] | Key/value commitments for resource-tree / state digests |
//!
//! **No postgres** (or any external store). Persistence adapters land when the
//! full K6 cutover replaces `ChainManager` SHAKE-256 with this BLAKE3+HLC path.
//!
//! ## ChainManager seam mapping
//!
//! | Legacy (`clawft-kernel::chain`) | K6 substrate |
//! |--------------------------------|--------------|
//! | `append(source, kind, payload)` | [`DagStore::append`] with HLC stamp |
//! | linear `prev_hash` | multi-parent DAG edges + MMR peak |
//! | checkpoint hash | [`MerkleMountainRange::root`] / SMT root |
//!
//! Pure Rust — compiles for native and `wasm32` (mutex via `parking_lot`).

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

mod dag;
mod mmr;
mod smt;

pub use dag::{
    load_checkpoint, Checkpoint, DagError, DagEvent, DagStore, K6_CHAIN_SEAM_VERSION,
};
pub use mmr::MerkleMountainRange;
pub use smt::SparseMerkleTree;

/// Re-export core hash / clock types so callers need one import path.
pub use exo_core::{blake3_hash, Blake3Hash, EventId, HybridLogicalClock, Hlc, HLC};

#[cfg(test)]
mod contract_tests {
    use super::*;
    use exo_core::blake3_hash_with_domain;

    /// End-to-end: HLC-stamped DAG append + MMR membership + SMT state root.
    #[test]
    fn k6_substrate_contract_map() {
        let clock = HybridLogicalClock::new();
        let store = DagStore::new(clock);

        let e1 = store
            .append("kernel", "boot.init", b"{}", &[])
            .expect("append genesis");
        let e2 = store
            .append("kernel", "boot.ready", b"{\"ok\":true}", &[e1.id])
            .expect("append child");

        assert_eq!(store.len(), 2);
        assert!(store.get(&e2.id).is_some());
        assert_eq!(store.get(&e2.id).unwrap().parents, vec![e1.id]);

        // MMR tracks append order for checkpoint proofs
        let mut mmr = MerkleMountainRange::new();
        mmr.push(e1.id.0);
        mmr.push(e2.id.0);
        let root = mmr.root();
        assert_ne!(root, Blake3Hash::ZERO);
        let proof = mmr.proof(0).expect("proof leaf 0");
        assert!(MerkleMountainRange::verify(e1.id.0, 0, &proof, root));

        // SMT commits key/value state (resource tree digest seam)
        let mut smt = SparseMerkleTree::new();
        let key = blake3_hash_with_domain(b"WEFTOS-RESOURCE-PATH-v1", b"/kernel");
        let val = blake3_hash(b"node-body");
        smt.update(key, val);
        assert_eq!(smt.get(&key), Some(val));
        let smt_root = smt.root();
        assert_ne!(smt_root, Blake3Hash::ZERO);

        let cp = load_checkpoint(&store, &e2.id).expect("checkpoint from tip");
        assert_eq!(cp.events_in_order().len(), 2);
        assert_eq!(cp.tip(), e2.id);
        assert_eq!(K6_CHAIN_SEAM_VERSION, 1);
    }

    #[test]
    fn no_postgres_in_memory_only() {
        // Compile-time: crate has no sqlx/postgres features. Runtime: empty store works.
        let store = DagStore::default();
        assert_eq!(store.len(), 0);
    }
}
