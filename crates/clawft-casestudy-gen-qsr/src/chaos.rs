//! Phase 4 — chaos injection.
//!
//! Deterministic mutations over a `DailyRollup` event stream to exercise the
//! ingest pipeline's resilience guarantees:
//!
//! - Drop: remove a percentage of events (in an optional day-range window)
//! - Duplicate: emit a percentage of events twice (tests dedupe)
//! - Reorder: locally shuffle events within a sliding window (tests HLC
//!   causal ordering)
//! - Clock skew: shift `business_date` for a subset of stores by ±N days
//!   (tests late-arrival rewriting)
//!
//! All injections are seeded so chaos runs are reproducible.

use crate::events::DailyRollup;
use crate::rng::subseed;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosConfig {
    pub seed: u64,
    /// Probability of dropping any single event (in the drop window if set).
    #[serde(default)]
    pub drop_rate: f64,
    /// Day-index window `[start, end)` to restrict drops to. `None` = all days.
    #[serde(default)]
    pub drop_window: Option<[u32; 2]>,
    /// Probability of emitting an event twice.
    #[serde(default)]
    pub duplicate_rate: f64,
    /// Max swap distance for local reordering. 0 = no reorder.
    #[serde(default)]
    pub reorder_window: usize,
    /// Probability a given store will have its timestamps skewed.
    #[serde(default)]
    pub clock_skew_store_prob: f64,
    /// Days to skew business_date by (can be negative via `clock_skew_backwards`).
    #[serde(default)]
    pub clock_skew_days: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChaosReport {
    pub total_input: usize,
    pub kept: usize,
    pub dropped: usize,
    pub duplicated: usize,
    pub reordered_pairs: usize,
    pub skewed_stores: usize,
}

impl ChaosConfig {
    pub fn none() -> Self {
        Self {
            seed: 0,
            drop_rate: 0.0,
            drop_window: None,
            duplicate_rate: 0.0,
            reorder_window: 0,
            clock_skew_store_prob: 0.0,
            clock_skew_days: 0,
        }
    }
}

pub fn apply(config: &ChaosConfig, events: &[DailyRollup]) -> (Vec<DailyRollup>, ChaosReport) {
    let mut report = ChaosReport {
        total_input: events.len(),
        ..Default::default()
    };

    // Step 1: pick the set of stores to clock-skew.
    let mut skew_rng = subseed(config.seed, "chaos:skew-store", 0);
    let skewed_stores: std::collections::HashSet<String> = {
        let mut seen = std::collections::BTreeSet::new();
        for ev in events {
            seen.insert(ev.store_ref.clone());
        }
        seen.into_iter()
            .filter(|_| skew_rng.gen_bool(config.clock_skew_store_prob.clamp(0.0, 1.0)))
            .collect()
    };
    report.skewed_stores = skewed_stores.len();

    // Step 2: walk events applying drop / dup / skew.
    let mut out: Vec<DailyRollup> = Vec::with_capacity(events.len());
    for (idx, ev) in events.iter().enumerate() {
        let mut rng = subseed(config.seed, "chaos:event", idx as u64);

        let in_drop_window = match config.drop_window {
            Some([s, e]) => ev.day_index >= s && ev.day_index < e,
            None => true,
        };
        if in_drop_window && rng.gen_bool(config.drop_rate.clamp(0.0, 1.0)) {
            report.dropped += 1;
            continue;
        }

        let mut mutated = ev.clone();
        if skewed_stores.contains(&mutated.store_ref) && config.clock_skew_days != 0 {
            mutated.business_date = shift_date(&mutated.business_date, config.clock_skew_days);
        }
        out.push(mutated);
        report.kept += 1;

        if rng.gen_bool(config.duplicate_rate.clamp(0.0, 1.0)) {
            let dup = out.last().cloned().unwrap();
            out.push(dup);
            report.duplicated += 1;
        }
    }

    // Step 3: local reordering via swap-within-window.
    if config.reorder_window > 0 && out.len() > 1 {
        let mut rng = subseed(config.seed, "chaos:reorder", 0);
        let w = config.reorder_window.min(out.len() - 1);
        let swaps = out.len() / 4;
        for _ in 0..swaps {
            let i = rng.gen_range(0..out.len());
            let lo = i.saturating_sub(w);
            let hi = (i + w).min(out.len() - 1);
            if lo == hi {
                continue;
            }
            let j = rng.gen_range(lo..=hi);
            if i != j {
                out.swap(i, j);
                report.reordered_pairs += 1;
            }
        }
    }

    (out, report)
}

fn shift_date(date: &str, days: i32) -> String {
    let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return date.to_string();
    };
    let shifted = d + chrono::Duration::days(days as i64);
    shifted.to_string()
}
