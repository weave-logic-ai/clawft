//! Fuzz target: concurrent `CostTracker` budget reservation.
//!
//! Property (security-review §6.1 / PEN-02): concurrent
//! `reserve_budget` / `reconcile_actual` never panics; budget results
//! are always valid enum variants (no poisoned locks under normal use).
//!
//! Run: `cargo +nightly fuzz run cost_tracker_concurrent -- -max_total_time=60`

#![no_main]

use std::sync::Arc;
use std::thread;

use clawft_core::pipeline::cost_tracker::CostTracker;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let reset_hour = data[0] % 24;
    let tracker = Arc::new(CostTracker::new(reset_hour));

    // Bound concurrency and work so a single fuzz iteration stays short.
    let n_threads = (data.get(1).copied().unwrap_or(1) as usize % 4) + 1;
    let ops_per = (data.get(2).copied().unwrap_or(1) as usize % 8) + 1;

    // Pre-bake a small set of sender ids from the remaining bytes.
    let tail = data.get(3..).unwrap_or(&[]);
    let senders: Vec<String> = tail
        .chunks(4)
        .take(8)
        .map(|c| {
            format!(
                "u{}",
                u32::from_le_bytes([
                    c.first().copied().unwrap_or(0),
                    c.get(1).copied().unwrap_or(0),
                    c.get(2).copied().unwrap_or(0),
                    c.get(3).copied().unwrap_or(0),
                ])
            )
        })
        .collect();
    let senders = if senders.is_empty() {
        vec!["anon".into()]
    } else {
        senders
    };

    let mut handles = Vec::with_capacity(n_threads);
    for t in 0..n_threads {
        let tracker = Arc::clone(&tracker);
        let senders = senders.clone();
        let seed = data.get(3 + t).copied().unwrap_or(0);
        handles.push(thread::spawn(move || {
            for i in 0..ops_per {
                let sid = &senders[(t + i) % senders.len()];
                let cost = f64::from(seed.wrapping_add(i as u8)) * 0.01;
                let daily = f64::from(seed % 50) + 0.5;
                let monthly = daily * 10.0;
                let result = tracker.reserve_budget(sid, cost, daily, monthly);
                // Drop the result after matching — forces enum validity.
                match result {
                    clawft_core::pipeline::traits::BudgetResult::Approved => {
                        tracker.reconcile_actual(sid, cost, cost * 0.9);
                    }
                    clawft_core::pipeline::traits::BudgetResult::DailyLimitExceeded {
                        spent,
                        limit,
                    } => {
                        let _ = (spent.is_finite(), limit.is_finite());
                    }
                    clawft_core::pipeline::traits::BudgetResult::MonthlyLimitExceeded {
                        spent,
                        limit,
                    } => {
                        let _ = (spent.is_finite(), limit.is_finite());
                    }
                    // BudgetResult is #[non_exhaustive]
                    _ => {}
                }
                let _ = tracker.check_budget(sid, cost, daily, monthly);
                let _ = tracker.get_spend(sid);
            }
        }));
    }

    for h in handles {
        // Join errors mean a worker panicked — re-raise so the fuzzer
        // records a crash.
        h.join().expect("cost_tracker worker panicked");
    }
});
