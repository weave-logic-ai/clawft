//! Ported acceptance test (integration plan §8 Phase 1 exit, §10):
//! N branches off one base, recall@10 == 100% via the chain-walk path.
//!
//! Upstream reference numbers (agenticow's own bench, plan §1/§8): ~472µs
//! COW branch, ~0.57ms p50 rollback, at 1M vectors. Our numbers below are
//! printed, not asserted against those -- different hardware, different
//! scale (one vector per checkpoint here, not 1M total), and RVF's shipped
//! `query()` is a brute-force scan (see `tests/common/mod.rs`), so this is
//! "does the chain-walk mechanism work and how fast is it on this box",
//! not a reproduction of agenticow's own benchmark.
//!
//! `N` is env-tunable (`COW_MEMORY_ACCEPTANCE_N`, default 100 for CI
//! speed); `acceptance_1000_branches` is the `#[ignore]`d full-scale run
//! matching agenticow's `bench/acceptance.js` (1,000 branches). Running it
//! opens ~1,000 simultaneous `.rvf` file handles (one per checkpoint, plus
//! base/working) -- raise the shell's `ulimit -n` if it fails with "too
//! many open files".

mod common;

use std::time::{Duration, Instant};

use clawft_cow_memory::VectorItem;

fn run_acceptance(n: usize) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = common::new_memory(dir.path());
    let mut rng = rand::thread_rng();
    let dim = common::dim() as usize;

    let mut inserted: Vec<(u64, Vec<f32>)> = Vec::with_capacity(n);
    let mut checkpoint_times: Vec<Duration> = Vec::with_capacity(n);

    for i in 0..n {
        let vector = common::random_vector(&mut rng, dim);
        let ids = mem
            .ingest(&[VectorItem::new(vector.clone())])
            .expect("ingest");
        inserted.push((ids[0], vector));

        let t0 = Instant::now();
        mem.checkpoint(format!("ckpt-{i}")).expect("checkpoint");
        checkpoint_times.push(t0.elapsed());
    }

    let total: Duration = checkpoint_times.iter().sum();
    let avg = total / checkpoint_times.len() as u32;
    let max = checkpoint_times.iter().max().copied().unwrap_or_default();
    println!(
        "acceptance(n={n}): checkpoint latency avg={avg:?} max={max:?} (upstream reference: ~472µs @1M vectors)"
    );

    assert_eq!(
        mem.status().own_checkpoint_depth,
        n,
        "one checkpoint per insert"
    );

    let mut hits = 0usize;
    let t0 = Instant::now();
    for (id, vector) in &inserted {
        let results = mem.query(vector, 10).expect("query");
        if results.iter().any(|r| r.id == *id) {
            hits += 1;
        }
    }
    let query_elapsed = t0.elapsed();
    let recall = hits as f64 / n as f64;
    println!(
        "acceptance(n={n}): recall@10 = {recall:.4} ({hits}/{n}), total query time = {query_elapsed:?}, avg/query = {:?}",
        query_elapsed / n as u32
    );

    assert_eq!(
        hits, n,
        "recall@10 must be 100% via chain-walk after {n} checkpoints"
    );
}

#[test]
fn acceptance_default() {
    let n: usize = std::env::var("COW_MEMORY_ACCEPTANCE_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    run_acceptance(n);
}

#[test]
#[ignore = "slow: opens ~1000 simultaneous .rvf handles, matches agenticow bench/acceptance.js scale"]
fn acceptance_1000_branches() {
    run_acceptance(1000);
}
