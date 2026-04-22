//! Scenario spec — the intervention description fed to the counterfactual engine.

use crate::dimensions::Store;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub id: String,
    pub description: String,
    pub template: String,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub metro: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    /// Week index within the generated range (0-based), optional.
    #[serde(default)]
    pub week_index: Option<u32>,
    /// Day-index range [start, end) to aggregate over (e.g. a quarter).
    #[serde(default)]
    pub day_range: Option<[u32; 2]>,
    /// Multiplicative revenue factor (1.0 = no change, 0.92 = −8%).
    #[serde(default = "one")]
    pub revenue_factor: f64,
    /// Multiplicative labor-hours factor.
    #[serde(default = "one")]
    pub labor_factor: f64,
    /// Promotion label to remove (for promo_pull scenarios).
    #[serde(default)]
    pub promo_id_to_pull: Option<String>,
    /// Explicit store labels for store_closure scenarios (overrides scope match).
    #[serde(default)]
    pub closed_store_labels: Option<Vec<String>>,
}

fn one() -> f64 {
    1.0
}

impl ScenarioSpec {
    pub fn matches_store(&self, store: &Store) -> bool {
        if let Some(b) = &self.brand
            && &store.brand != b
        {
            return false;
        }
        if let Some(m) = &self.metro
            && &store.metro_code != m
        {
            return false;
        }
        if let Some(r) = &self.region
            && &store.region_code != r
        {
            return false;
        }
        true
    }

    /// Does this scenario's intervention apply on a given day index?
    pub fn matches_intervention_day(&self, day_index: u32) -> bool {
        if let Some(week) = self.week_index {
            let start = week * 7;
            let end = start + 7;
            return day_index >= start && day_index < end;
        }
        // No week scope = intervention applies across the whole day_range.
        true
    }

    /// Does this day fall within the aggregation window (e.g. the quarter
    /// we're projecting)?
    pub fn in_aggregation_window(&self, day_index: u32) -> bool {
        match self.day_range {
            Some([start, end]) => day_index >= start && day_index < end,
            None => true,
        }
    }
}

pub fn load_from_file(path: &Path) -> Result<ScenarioSpec> {
    let s = std::fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&s)?)
}
