//! Truth manifest — the oracle the system-under-test never sees.

use crate::config::GeneratorConfig;
use crate::dimensions::Dimensions;
use crate::events::DailyRollup;
use crate::scenarios::ScenarioSpec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthManifest {
    pub generator_version: String,
    pub seed: u64,
    pub scale_tier: String,
    pub causal_edges: Vec<TrueCausalEdge>,
    pub org_gaps: Vec<TrueOrgGap>,
    pub counterfactuals: BTreeMap<String, CounterfactualTruth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrueCausalEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub true_weight: f64,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrueOrgGap {
    pub position_label: String,
    pub gap_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualTruth {
    pub scenario_id: String,
    pub description: String,
    pub baseline_revenue: f64,
    pub intervention_revenue: f64,
    pub delta: f64,
    pub delta_pct: f64,
    pub stores_in_scope: usize,
    pub days_intervened: usize,
}

pub fn build(config: &GeneratorConfig, dims: &Dimensions) -> TruthManifest {
    let causal_edges = dims
        .promotions
        .iter()
        .map(|p| TrueCausalEdge {
            source: p.label.clone(),
            target: format!("brand:{}", p.brand),
            edge_type: "CAUSES".into(),
            true_weight: p.true_lift,
            provenance: "ground_truth".into(),
        })
        .collect();

    let org_gaps = dims
        .positions
        .iter()
        .filter(|p| p.filled_by_ref.is_none())
        .map(|p| TrueOrgGap {
            position_label: p.label.clone(),
            gap_type: if p.critical {
                "critical_vacancy".into()
            } else {
                "vacancy".into()
            },
        })
        .collect();

    TruthManifest {
        generator_version: env!("CARGO_PKG_VERSION").into(),
        seed: config.seed,
        scale_tier: format!("{:?}", config.scale_tier).to_lowercase(),
        causal_edges,
        org_gaps,
        counterfactuals: BTreeMap::new(),
    }
}

pub fn compute_counterfactual(
    scenario: &ScenarioSpec,
    events: &[DailyRollup],
    dims: &Dimensions,
) -> CounterfactualTruth {
    // Store scope: explicit list trumps scope-match.
    let stores_in_scope: Vec<&str> = match &scenario.closed_store_labels {
        Some(labels) => labels.iter().map(|s| s.as_str()).collect(),
        None => dims
            .stores
            .iter()
            .filter(|s| scenario.matches_store(s))
            .map(|s| s.label.as_str())
            .collect(),
    };

    // For promo_pull, restrict the intervention window to the promo's active
    // days (even if day_range is broader). The engine.plan() does the same
    // tightening; truth must match or scoring is self-inconsistent.
    let promo_window: Option<(u32, u32)> = match (&scenario.template, &scenario.promo_id_to_pull) {
        (t, Some(pid)) if t == "promo_pull" => dims
            .promotions
            .iter()
            .find(|p| &p.label == pid)
            .map(|p| (p.start_day, p.end_day)),
        _ => None,
    };

    let mut baseline_sum = 0.0f64;
    let mut intervention_sum = 0.0f64;
    let mut days_intervened_set = std::collections::BTreeSet::new();

    for ev in events {
        if !scenario.in_aggregation_window(ev.day_index) {
            continue;
        }
        baseline_sum += ev.revenue;

        let store_in_scope = stores_in_scope.iter().any(|&s| s == ev.store_ref);
        let day_in_scope_from_week = scenario.matches_intervention_day(ev.day_index);
        let day_in_scope_from_promo = match promo_window {
            Some((s, e)) => ev.day_index >= s && ev.day_index < e,
            None => true,
        };
        let apply = store_in_scope && day_in_scope_from_week && day_in_scope_from_promo;

        if apply {
            days_intervened_set.insert(ev.day_index);
            intervention_sum += ev.revenue * scenario.revenue_factor;
        } else {
            intervention_sum += ev.revenue;
        }
    }

    let delta = intervention_sum - baseline_sum;
    let delta_pct = if baseline_sum.abs() > f64::EPSILON {
        delta / baseline_sum
    } else {
        0.0
    };

    CounterfactualTruth {
        scenario_id: scenario.id.clone(),
        description: scenario.description.clone(),
        baseline_revenue: round2(baseline_sum),
        intervention_revenue: round2(intervention_sum),
        delta: round2(delta),
        delta_pct: (delta_pct * 10_000.0).round() / 10_000.0,
        stores_in_scope: stores_in_scope.len(),
        days_intervened: days_intervened_set.len(),
    }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
