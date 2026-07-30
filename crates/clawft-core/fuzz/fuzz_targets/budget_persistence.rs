//! Fuzz target: `CostTracker` persistence load/save (HMAC integrity).
//!
//! Property (security-review §2.4 / WEFT-28):
//! - Loading arbitrary file bytes never panics (returns Err, fresh
//!   tracker on bad HMAC, or a parsed snapshot).
//! - Round-trip persist → load never panics.
//!
//! Run: `cargo +nightly fuzz run budget_persistence -- -max_total_time=60`

#![no_main]

use std::io::Write;

use clawft_core::pipeline::cost_tracker::CostTracker;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(_) => return, // env issue — not a fuzz finding
    };
    let path = dir.path().join("cost-tracker.json");

    // ── 1. Load arbitrary bytes as if they were a persistence file ──
    if let Ok(mut f) = std::fs::File::create(&path) {
        let _ = f.write_all(data);
        let _ = f.sync_all();
    }
    // load() must not panic: Ok(fresh/parsed) or Err(_).
    match CostTracker::load(&path) {
        Ok(tracker) => {
            // Touch public API on whatever came back.
            let _ = tracker.get_spend("fuzz");
            let _ = tracker.check_budget("fuzz", 0.01, 1.0, 10.0);
            // Re-persist (may re-sign legacy/unsigned bodies).
            let _ = tracker.persist();
        }
        Err(_) => {
            // Corrupt / missing body — acceptable.
        }
    }

    // ── 2. Clean round-trip with spends derived from input ──────────
    let path2 = dir.path().join("roundtrip.json");
    let tracker = CostTracker::new(data.first().copied().unwrap_or(0) % 24)
        .with_persistence(path2.clone());

    let n = (data.get(1).copied().unwrap_or(1) as usize % 8) + 1;
    for i in 0..n {
        let sid = format!("user-{}", data.get(2 + i).copied().unwrap_or(i as u8));
        let cost = f64::from(data.get(2 + i).copied().unwrap_or(1)) * 0.001;
        let _ = tracker.reserve_budget(&sid, cost, 100.0, 1000.0);
        tracker.reconcile_actual(&sid, cost, cost);
    }

    if tracker.persist().is_ok() {
        if let Ok(loaded) = CostTracker::load(&path2) {
            for i in 0..n {
                let sid = format!("user-{}", data.get(2 + i).copied().unwrap_or(i as u8));
                let (d, m) = loaded.get_spend(&sid);
                let _ = (d.is_finite(), m.is_finite());
            }
            // Tamper with the signed file and ensure no panic on reload.
            if let Ok(mut body) = std::fs::read_to_string(&path2) {
                body.push_str("TAMPER");
                let _ = std::fs::write(&path2, body);
                let _ = CostTracker::load(&path2);
            }
        }
    }
});
