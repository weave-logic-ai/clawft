//! Shared helpers for `clawft-cow-memory` integration tests. Not a test
//! binary itself (lives under `tests/common/`, not directly in `tests/`),
//! pulled in via `mod common;` in each test file.

use std::path::Path;

use clawft_cow_memory::BranchableMemory;

/// Small dimension keeps tests fast; `RvfStore::query` (crates.io 0.2.0) is
/// a brute-force linear scan (verified against `store.rs:314-368`), so
/// dimension size directly drives per-query cost.
pub fn dim() -> u16 {
    16
}

pub fn random_vector(rng: &mut impl rand::Rng, dim: usize) -> Vec<f32> {
    (0..dim).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect()
}

pub fn new_memory(dir: &Path) -> BranchableMemory {
    BranchableMemory::create(dir, dim()).expect("BranchableMemory::create")
}
