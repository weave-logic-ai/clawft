//! WEFT-34 / CONS-002: contention bench for `RwLock<HashMap>` vs `DashMap`.
//!
//! Models the hot paths of `CostTracker` and `RateLimiter`:
//! - **Cost-style**: get-or-insert per sender, mutate small numeric state under
//!   a write (reserve / reconcile). Occasional read of totals.
//! - **Rate-style**: get-or-insert per sender, prune + push a timestamp on
//!   every check (always write on the map entry).
//!
//! Target deployment is 1–10 concurrent users. Prefer keeping std
//! `RwLock<HashMap>` unless DashMap clearly wins at realistic load.
//!
//! Run:
//! ```text
//! cargo bench -p clawft-core --bench map_contention_bench -- --sample-size 20
//! ```
//!
//! Decision recorded in `.planning/development_notes/01-tiered-router/consensus-log.md`
//! (CONS-002) and summarized in `docs/benchmarks/results.md`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dashmap::DashMap;

// ── Minimal stand-ins (same shape as production hot paths) ───────────────

#[derive(Clone, Default)]
struct CostEntry {
    daily_spent: f64,
    daily_reserved: f64,
}

#[derive(Clone)]
struct RateEntry {
    timestamps: Vec<u64>,
    last_access: u64,
}

impl Default for RateEntry {
    fn default() -> Self {
        Self {
            timestamps: Vec::with_capacity(16),
            last_access: 0,
        }
    }
}

// ── Workloads ────────────────────────────────────────────────────────────

/// CostTracker-like: write-heavy reserve + occasional get_spend-style read.
fn cost_rw_lock(
    map: &RwLock<HashMap<String, CostEntry>>,
    keys: &[String],
    ops: usize,
    thread_id: usize,
) {
    for i in 0..ops {
        let key = &keys[(thread_id.wrapping_mul(31) + i) % keys.len()];
        if i % 5 == 0 {
            // Read path (~20%): get_spend
            let g = map.read().expect("poisoned");
            let _ = g.get(key).map(|e| e.daily_spent + e.daily_reserved);
        } else {
            // Write path: reserve_budget-style
            let mut g = map.write().expect("poisoned");
            let e = g.entry(key.clone()).or_default();
            e.daily_reserved += 0.01;
            e.daily_spent += 0.001;
        }
    }
}

fn cost_dashmap(map: &DashMap<String, CostEntry>, keys: &[String], ops: usize, thread_id: usize) {
    for i in 0..ops {
        let key = &keys[(thread_id.wrapping_mul(31) + i) % keys.len()];
        if i % 5 == 0 {
            let _ = map.get(key).map(|e| e.daily_spent + e.daily_reserved);
        } else {
            let mut e = map.entry(key.clone()).or_default();
            e.daily_reserved += 0.01;
            e.daily_spent += 0.001;
        }
    }
}

/// RateLimiter-like: every op takes a write on the entry (check + record).
fn rate_rw_lock(
    map: &RwLock<HashMap<String, RateEntry>>,
    keys: &[String],
    ops: usize,
    thread_id: usize,
    clock: &AtomicU64,
    window: u64,
    limit: usize,
) {
    for i in 0..ops {
        let key = &keys[(thread_id.wrapping_mul(31) + i) % keys.len()];
        let now = clock.fetch_add(1, Ordering::Relaxed);
        let mut g = map.write().expect("poisoned");
        let e = g.entry(key.clone()).or_default();
        e.last_access = now;
        e.timestamps.retain(|ts| now.saturating_sub(*ts) < window);
        if e.timestamps.len() < limit {
            e.timestamps.push(now);
        }
    }
}

fn rate_dashmap(
    map: &DashMap<String, RateEntry>,
    keys: &[String],
    ops: usize,
    thread_id: usize,
    clock: &AtomicU64,
    window: u64,
    limit: usize,
) {
    for i in 0..ops {
        let key = &keys[(thread_id.wrapping_mul(31) + i) % keys.len()];
        let now = clock.fetch_add(1, Ordering::Relaxed);
        let mut e = map.entry(key.clone()).or_default();
        e.last_access = now;
        e.timestamps.retain(|ts| now.saturating_sub(*ts) < window);
        if e.timestamps.len() < limit {
            e.timestamps.push(now);
        }
    }
}

// ── Multi-thread harness ─────────────────────────────────────────────────

fn run_parallel<F>(threads: usize, body: F) -> Duration
where
    F: FnMut(usize) + Send + Clone + 'static,
{
    let barrier = Arc::new(Barrier::new(threads + 1));
    let mut handles = Vec::with_capacity(threads);

    for tid in 0..threads {
        let barrier = Arc::clone(&barrier);
        let mut body = body.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            body(tid);
        }));
    }

    barrier.wait();
    let start = Instant::now();
    for h in handles {
        h.join().expect("worker panicked");
    }
    start.elapsed()
}

fn make_keys(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("user_{i}")).collect()
}

// ── Criterion groups ─────────────────────────────────────────────────────

fn bench_cost_style(c: &mut Criterion) {
    let mut group = c.benchmark_group("cons002_cost_style");
    // Ops per thread per sample iteration — keeps wall time short.
    let ops_per_thread = 2_000usize;

    // (threads, num_keys) — cover single-user, small-team, and wide fan-out.
    let configs: &[(usize, usize)] = &[
        (1, 1),
        (4, 1),  // high contention, single user
        (4, 8),  // target: small team
        (8, 8),
        (8, 100), // wide fan-out (where DashMap sharding helps most)
        (16, 100),
    ];

    for &(threads, num_keys) in configs {
        let total_ops = (threads * ops_per_thread) as u64;
        group.throughput(Throughput::Elements(total_ops));

        let id = format!("rwlock_t{threads}_k{num_keys}");
        group.bench_function(BenchmarkId::from_parameter(&id), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let map = Arc::new(RwLock::new(HashMap::<String, CostEntry>::new()));
                    let keys = Arc::new(make_keys(num_keys));
                    let elapsed = run_parallel(threads, {
                        let map = Arc::clone(&map);
                        let keys = Arc::clone(&keys);
                        move |tid| cost_rw_lock(&map, &keys, ops_per_thread, tid)
                    });
                    total += elapsed;
                }
                total
            });
        });

        let id = format!("dashmap_t{threads}_k{num_keys}");
        group.bench_function(BenchmarkId::from_parameter(&id), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let map = Arc::new(DashMap::<String, CostEntry>::new());
                    let keys = Arc::new(make_keys(num_keys));
                    let elapsed = run_parallel(threads, {
                        let map = Arc::clone(&map);
                        let keys = Arc::clone(&keys);
                        move |tid| cost_dashmap(&map, &keys, ops_per_thread, tid)
                    });
                    total += elapsed;
                }
                total
            });
        });
    }

    group.finish();
}

fn bench_rate_style(c: &mut Criterion) {
    let mut group = c.benchmark_group("cons002_rate_style");
    let ops_per_thread = 2_000usize;
    let window = 10_000u64;
    let limit = 1_000usize;

    let configs: &[(usize, usize)] = &[
        (1, 1),
        (4, 1),
        (4, 8),
        (8, 8),
        (8, 100),
        (16, 100),
    ];

    for &(threads, num_keys) in configs {
        let total_ops = (threads * ops_per_thread) as u64;
        group.throughput(Throughput::Elements(total_ops));

        let id = format!("rwlock_t{threads}_k{num_keys}");
        group.bench_function(BenchmarkId::from_parameter(&id), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let map = Arc::new(RwLock::new(HashMap::<String, RateEntry>::new()));
                    let keys = Arc::new(make_keys(num_keys));
                    let clock = Arc::new(AtomicU64::new(1));
                    let elapsed = run_parallel(threads, {
                        let map = Arc::clone(&map);
                        let keys = Arc::clone(&keys);
                        let clock = Arc::clone(&clock);
                        move |tid| {
                            rate_rw_lock(&map, &keys, ops_per_thread, tid, &clock, window, limit)
                        }
                    });
                    total += elapsed;
                }
                total
            });
        });

        let id = format!("dashmap_t{threads}_k{num_keys}");
        group.bench_function(BenchmarkId::from_parameter(&id), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let map = Arc::new(DashMap::<String, RateEntry>::new());
                    let keys = Arc::new(make_keys(num_keys));
                    let clock = Arc::new(AtomicU64::new(1));
                    let elapsed = run_parallel(threads, {
                        let map = Arc::clone(&map);
                        let keys = Arc::clone(&keys);
                        let clock = Arc::clone(&clock);
                        move |tid| {
                            rate_dashmap(&map, &keys, ops_per_thread, tid, &clock, window, limit)
                        }
                    });
                    total += elapsed;
                }
                total
            });
        });
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(25).warm_up_time(Duration::from_millis(300));
    targets = bench_cost_style, bench_rate_style
);
criterion_main!(benches);
