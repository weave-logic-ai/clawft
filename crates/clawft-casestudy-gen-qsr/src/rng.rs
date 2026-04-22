//! Deterministic sub-seeded RNG helpers.
//!
//! Every entity in the corpus gets a per-entity RNG derived from
//! BLAKE3(master_seed || kind || index). This keeps the generator reproducible
//! while letting different entity streams evolve independently.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn subseed(master: u64, kind: &str, idx: u64) -> ChaCha8Rng {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&master.to_le_bytes());
    hasher.update(b"::");
    hasher.update(kind.as_bytes());
    hasher.update(b"::");
    hasher.update(&idx.to_le_bytes());
    let digest = hasher.finalize();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(digest.as_bytes());
    ChaCha8Rng::from_seed(seed)
}

pub fn stable_hash_hex(input: &str) -> String {
    let digest = blake3::hash(input.as_bytes());
    digest.to_hex()[..16].to_string()
}
