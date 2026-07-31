//! Synthetic sonobuoy-like arrays for train / eval scaffolding.
//!
//! Generates a small fleet of buoys with physics priors, radius-graph edges,
//! and temporal energy series that embed class-conditional acoustic events:
//!
//! | Class | Name        | Temporal signature              |
//! |-------|-------------|---------------------------------|
//! | 0     | ambient     | low-level noise only            |
//! | 1     | vessel      | mid-band periodic bumps         |
//! | 2     | mammal_call | sparse high-amplitude impulses  |

use super::graph::{SensorGraph, SensorNode, PHYSICS_DIM};

/// Labels used by the synthetic fixture (matches default `num_classes = 3`).
pub const CLASS_AMBIENT: usize = 0;
pub const CLASS_VESSEL: usize = 1;
pub const CLASS_MAMMAL: usize = 2;

/// Human-readable class name.
pub fn class_name(label: usize) -> &'static str {
    match label {
        CLASS_AMBIENT => "ambient",
        CLASS_VESSEL => "vessel",
        CLASS_MAMMAL => "mammal_call",
        _ => "unknown",
    }
}

/// Parameters for [`generate_synthetic_array`].
#[derive(Debug, Clone)]
pub struct SyntheticConfig {
    /// Number of buoys.
    pub n_buoys: usize,
    /// Temporal series length per buoy.
    pub series_len: usize,
    /// Radius for undirected edges (position units).
    pub connect_radius: f64,
    /// Ambient noise amplitude.
    pub noise: f32,
    /// RNG seed.
    pub seed: u64,
}

impl Default for SyntheticConfig {
    fn default() -> Self {
        Self {
            n_buoys: 6,
            series_len: 32,
            connect_radius: 3.5,
            noise: 0.05,
            seed: 0x50b007,
        }
    }
}

/// One labelled observation: which node, and ground-truth class.
#[derive(Debug, Clone)]
pub struct SyntheticSample {
    pub node_idx: usize,
    pub label: usize,
}

/// Full synthetic dataset: shared array graph + per-node labels.
#[derive(Debug, Clone)]
pub struct SyntheticDataset {
    pub graph: SensorGraph,
    pub samples: Vec<SyntheticSample>,
    pub config: SyntheticConfig,
}

/// Generate a synthetic sonobuoy array with class-conditional temporal events.
///
/// Buoys are placed on a rough grid. Roughly one third of nodes are labelled
/// ambient / vessel / mammal. Vessel and mammal events are injected into the
/// temporal buffer; neighbors share a weak bleed of the same event to make
/// spatial aggregation informative.
pub fn generate_synthetic_array(cfg: SyntheticConfig) -> SyntheticDataset {
    let mut rng = cfg.seed;
    let mut graph = SensorGraph::new();

    // Grid placement: ~sqrt(N) columns.
    let cols = (cfg.n_buoys as f64).sqrt().ceil() as usize;
    let mut labels = Vec::with_capacity(cfg.n_buoys);

    for i in 0..cfg.n_buoys {
        let row = i / cols;
        let col = i % cols;
        // Slight jitter so distances are not perfectly uniform.
        let jx = f64::from(unit(&mut rng)) * 0.3;
        let jy = f64::from(unit(&mut rng)) * 0.3;
        let lat = row as f64 + jx;
        let lon = col as f64 + jy;
        let depth = 8.0 + f64::from(unit(&mut rng)) * 4.0;

        // Physics priors (normalized-ish): SSP, thermocline, sea_state, current, bottom.
        let features = vec![
            0.4 + 0.2 * unit(&mut rng), // sound speed proxy
            0.3 + 0.3 * unit(&mut rng), // thermocline
            0.1 + 0.4 * unit(&mut rng), // sea state
            unit(&mut rng) * 0.5,       // current
            if unit(&mut rng) > 0.5 { 0.8 } else { 0.2 }, // bottom type
        ];
        debug_assert_eq!(features.len(), PHYSICS_DIM);

        graph.add_buoy(SensorNode {
            id: format!("buoy-{i:02}"),
            position: (lat, lon, depth),
            features,
        });

        // Round-robin class assignment with a bit of randomness.
        let label = match i % 3 {
            0 => CLASS_AMBIENT,
            1 => CLASS_VESSEL,
            _ => CLASS_MAMMAL,
        };
        labels.push(label);
    }

    graph.connect_within_radius(cfg.connect_radius);
    // Ensure connectivity for tiny fleets: if isolated, fall back to full mesh.
    if graph.edge_count() == 0 && cfg.n_buoys > 1 {
        graph.connect_fully();
    }

    // Temporal series per class.
    for (i, &label) in labels.iter().enumerate() {
        let series = match label {
            CLASS_VESSEL => vessel_series(cfg.series_len, cfg.noise, &mut rng),
            CLASS_MAMMAL => mammal_series(cfg.series_len, cfg.noise, &mut rng),
            _ => ambient_series(cfg.series_len, cfg.noise, &mut rng),
        };
        graph.push_temporal(i, &series);
    }

    // Spatial bleed: add 20% of each event node's series to 1-hop neighbors
    // (simulates shared propagation of the same acoustic source).
    let bleed: Vec<(usize, Vec<f32>)> = (0..cfg.n_buoys)
        .filter(|&i| labels[i] != CLASS_AMBIENT)
        .map(|i| {
            let src = graph.temporal_buffers[i].clone();
            (i, src)
        })
        .collect();
    for (src_idx, src_series) in bleed {
        for nb in graph.neighbor_indices(src_idx) {
            let buf = &mut graph.temporal_buffers[nb];
            let n = buf.len().min(src_series.len());
            for t in 0..n {
                buf[t] += 0.2 * src_series[t];
            }
        }
    }

    let samples = labels
        .into_iter()
        .enumerate()
        .map(|(node_idx, label)| SyntheticSample { node_idx, label })
        .collect();

    SyntheticDataset {
        graph,
        samples,
        config: cfg,
    }
}

fn ambient_series(len: usize, noise: f32, rng: &mut u64) -> Vec<f32> {
    (0..len).map(|_| (unit(rng) - 0.5) * 2.0 * noise).collect()
}

fn vessel_series(len: usize, noise: f32, rng: &mut u64) -> Vec<f32> {
    // Periodic mid-amplitude bumps (propeller-like cadence).
    let period = 6.0f32;
    (0..len)
        .map(|t| {
            let phase = (t as f32) * std::f32::consts::TAU / period;
            let sig = 0.35 * (phase.sin().max(0.0));
            sig + (unit(rng) - 0.5) * 2.0 * noise
        })
        .collect()
}

fn mammal_series(len: usize, noise: f32, rng: &mut u64) -> Vec<f32> {
    // Sparse high impulses (call-like).
    let mut s: Vec<f32> = (0..len)
        .map(|_| (unit(rng) - 0.5) * 2.0 * noise)
        .collect();
    let n_clicks = 2 + (*rng as usize % 3);
    for _ in 0..n_clicks {
        let t = (*rng as usize) % len;
        *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        s[t] += 0.9;
        if t + 1 < len {
            s[t + 1] += 0.4;
        }
    }
    s
}

/// LCG unit float in [0, 1).
fn unit(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*rng >> 33) as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_expected_counts() {
        let ds = generate_synthetic_array(SyntheticConfig {
            n_buoys: 9,
            series_len: 24,
            connect_radius: 2.5,
            noise: 0.05,
            seed: 1,
        });
        assert_eq!(ds.graph.node_count(), 9);
        assert_eq!(ds.samples.len(), 9);
        assert!(ds.graph.edge_count() > 0);
        for s in &ds.samples {
            assert!(s.label < 3);
            assert_eq!(
                ds.graph.temporal_buffers[s.node_idx].len(),
                24
            );
        }
    }

    #[test]
    fn class_names() {
        assert_eq!(class_name(0), "ambient");
        assert_eq!(class_name(1), "vessel");
        assert_eq!(class_name(2), "mammal_call");
    }

    #[test]
    fn vessel_nodes_have_higher_energy_than_ambient() {
        let ds = generate_synthetic_array(SyntheticConfig {
            n_buoys: 6,
            series_len: 32,
            connect_radius: 5.0,
            noise: 0.02,
            seed: 42,
        });
        let mut amb_e = 0.0f32;
        let mut amb_n = 0usize;
        let mut ves_e = 0.0f32;
        let mut ves_n = 0usize;
        for s in &ds.samples {
            let e: f32 = ds.graph.temporal_buffers[s.node_idx]
                .iter()
                .map(|v| v.abs())
                .sum();
            if s.label == CLASS_AMBIENT {
                amb_e += e;
                amb_n += 1;
            } else if s.label == CLASS_VESSEL {
                ves_e += e;
                ves_n += 1;
            }
        }
        assert!(amb_n > 0 && ves_n > 0);
        assert!(ves_e / ves_n as f32 > amb_e / amb_n as f32);
    }
}
