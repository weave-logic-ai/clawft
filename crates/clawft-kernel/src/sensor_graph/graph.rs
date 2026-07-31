//! Sensor array graph: buoys as nodes, acoustic links as edges.
//!
//! Nodes carry physics priors + static acoustic features. Edges store
//! Euclidean distance and approximate acoustic propagation delay. Temporal
//! ring buffers hold per-node acoustic energy (or other scalar series).

use std::collections::{HashMap, HashSet};

/// Default number of ocean-physics prior channels (K-STEMIT → sonobuoy map).
///
/// Order: sound_speed (m/s, normalized), thermocline_depth_m (norm), sea_state,
/// current_velocity (norm), bottom_type (0–1 categorical proxy).
pub const PHYSICS_DIM: usize = 5;

/// A buoy node in the sensor graph.
#[derive(Debug, Clone)]
pub struct SensorNode {
    /// Unique identifier for this buoy.
    pub id: String,
    /// Position as (latitude, longitude, depth_m).
    pub position: (f64, f64, f64),
    /// Node features: physics priors first ([`PHYSICS_DIM`]), then optional
    /// extra static channels. Spatial GraphSAGE consumes the full vector;
    /// the temporal branch never sees position or static GPS identity.
    pub features: Vec<f32>,
}

impl SensorNode {
    /// Physics prior slice (first [`PHYSICS_DIM`] entries, zero-padded if short).
    pub fn physics(&self) -> [f32; PHYSICS_DIM] {
        let mut out = [0.0f32; PHYSICS_DIM];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.features.get(i).copied().unwrap_or(0.0);
        }
        out
    }
}

/// An edge connecting two buoys.
#[derive(Debug, Clone)]
pub struct SensorEdge {
    /// Index of the source node.
    pub from: usize,
    /// Index of the target node.
    pub to: usize,
    /// Euclidean distance between buoys in meters (fixture units may be
    /// synthetic lat/lon deltas treated as metres for unit tests).
    pub distance_m: f64,
    /// Estimated acoustic propagation delay in milliseconds.
    pub propagation_delay_ms: f64,
}

/// Graph-based sensor fusion substrate for distributed buoy arrays.
#[derive(Debug, Clone)]
pub struct SensorGraph {
    /// Buoy positions and node features.
    pub nodes: Vec<SensorNode>,
    /// Inter-buoy connections.
    pub edges: Vec<SensorEdge>,
    /// Temporal feature buffer per node (ring of scalar samples; typically
    /// short-time acoustic energy or a spectral band).
    pub temporal_buffers: Vec<Vec<f32>>,
}

impl Default for SensorGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorGraph {
    /// Create an empty sensor graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            temporal_buffers: Vec::new(),
        }
    }

    /// Add a buoy node. Returns its index.
    pub fn add_buoy(&mut self, node: SensorNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        self.temporal_buffers.push(Vec::new());
        idx
    }

    /// Connect two buoys; distance and delay derived from positions.
    ///
    /// Uses Euclidean distance in (lat, lon, depth) space and a constant
    /// 1500 m/s sound speed. Production deployments should swap in geodesic
    /// + SSP / BELLHOP-style propagation.
    ///
    /// # Panics
    /// Panics if `from` or `to` are out of bounds.
    pub fn connect(&mut self, from: usize, to: usize) {
        assert!(from < self.nodes.len(), "from index out of bounds");
        assert!(to < self.nodes.len(), "to index out of bounds");
        let (distance_m, propagation_delay_ms) = Self::link_metrics(
            self.nodes[from].position,
            self.nodes[to].position,
        );
        self.edges.push(SensorEdge {
            from,
            to,
            distance_m,
            propagation_delay_ms,
        });
    }

    /// Connect with caller-supplied metrics (tests / offline graph builders).
    pub fn connect_with(
        &mut self,
        from: usize,
        to: usize,
        distance_m: f64,
        propagation_delay_ms: f64,
    ) {
        assert!(from < self.nodes.len(), "from index out of bounds");
        assert!(to < self.nodes.len(), "to index out of bounds");
        self.edges.push(SensorEdge {
            from,
            to,
            distance_m,
            propagation_delay_ms,
        });
    }

    /// Build a radius graph: undirected edges when distance ≤ `radius`.
    pub fn connect_within_radius(&mut self, radius: f64) {
        let n = self.nodes.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (d, delay) = Self::link_metrics(self.nodes[i].position, self.nodes[j].position);
                if d <= radius {
                    self.edges.push(SensorEdge {
                        from: i,
                        to: j,
                        distance_m: d,
                        propagation_delay_ms: delay,
                    });
                }
            }
        }
    }

    /// Fully connected undirected graph (K-STEMIT-style small arrays).
    pub fn connect_fully(&mut self) {
        let n = self.nodes.len();
        for i in 0..n {
            for j in (i + 1)..n {
                self.connect(i, j);
            }
        }
    }

    fn link_metrics(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64) {
        let dlat = b.0 - a.0;
        let dlon = b.1 - a.1;
        let ddep = b.2 - a.2;
        let distance_m = (dlat * dlat + dlon * dlon + ddep * ddep).sqrt();
        let speed_of_sound = 1500.0_f64;
        let propagation_delay_ms = (distance_m / speed_of_sound) * 1000.0;
        (distance_m, propagation_delay_ms)
    }

    /// Append a feature snapshot to a node's temporal buffer.
    ///
    /// # Panics
    /// Panics if `node_idx` is out of bounds.
    pub fn push_temporal(&mut self, node_idx: usize, values: &[f32]) {
        assert!(
            node_idx < self.temporal_buffers.len(),
            "node index out of bounds"
        );
        self.temporal_buffers[node_idx].extend_from_slice(values);
    }

    /// Cap each temporal buffer to the last `max_len` samples.
    pub fn trim_temporal(&mut self, max_len: usize) {
        for buf in &mut self.temporal_buffers {
            if buf.len() > max_len {
                let start = buf.len() - max_len;
                *buf = buf[start..].to_vec();
            }
        }
    }

    /// Neighbor indices for a node (undirected).
    pub fn neighbor_indices(&self, node_idx: usize) -> Vec<usize> {
        let mut neighbors = HashSet::new();
        for edge in &self.edges {
            if edge.from == node_idx {
                neighbors.insert(edge.to);
            }
            if edge.to == node_idx {
                neighbors.insert(edge.from);
            }
        }
        let mut v: Vec<usize> = neighbors.into_iter().collect();
        v.sort_unstable();
        v
    }

    /// Direct-edge distance map from `node_idx` to each neighbor.
    pub fn neighbor_distances(&self, node_idx: usize) -> HashMap<usize, f64> {
        let mut m = HashMap::new();
        for e in &self.edges {
            if e.from == node_idx {
                m.insert(e.to, e.distance_m);
            } else if e.to == node_idx {
                m.insert(e.from, e.distance_m);
            }
        }
        m
    }

    /// GraphSAGE-style mean aggregation (unlearned baseline, hop-BFs).
    ///
    /// Weighted by inverse distance on direct edges; multi-hop nodes without
    /// a direct edge use weight `1.0`. Prefer [`crate::sensor_graph::GraphSageLayer`]
    /// for the learned path.
    pub fn aggregate_neighbors(&self, node_idx: usize, hop: usize) -> Vec<f32> {
        if node_idx >= self.nodes.len() || hop == 0 {
            return self
                .nodes
                .get(node_idx)
                .map(|n| n.features.clone())
                .unwrap_or_default();
        }

        let mut visited = HashSet::new();
        let mut frontier = vec![node_idx];
        visited.insert(node_idx);

        for _ in 0..hop {
            let mut next_frontier = Vec::new();
            for &n in &frontier {
                for &nb in &self.neighbor_indices(n) {
                    if visited.insert(nb) {
                        next_frontier.push(nb);
                    }
                }
            }
            frontier = next_frontier;
        }

        let neighbors: Vec<usize> = visited.into_iter().filter(|&n| n != node_idx).collect();
        if neighbors.is_empty() {
            return self.nodes[node_idx].features.clone();
        }

        let edge_distances = self.neighbor_distances(node_idx);
        let feat_dim = self.nodes[node_idx].features.len();
        let mut agg = vec![0.0f32; feat_dim];
        let mut total_weight = 0.0f32;

        for &nb in &neighbors {
            let dist = edge_distances.get(&nb).copied().unwrap_or(1.0);
            let w = 1.0 / (dist as f32 + 1e-6);
            total_weight += w;
            let feats = &self.nodes[nb].features;
            for (i, &f) in feats.iter().enumerate().take(feat_dim) {
                agg[i] += w * f;
            }
        }

        if total_weight > 0.0 {
            for v in &mut agg {
                *v /= total_weight;
            }
        }
        agg
    }

    /// Last `window` temporal samples (left zero-padded).
    pub fn temporal_features(&self, node_idx: usize, window: usize) -> Vec<f32> {
        if node_idx >= self.temporal_buffers.len() {
            return vec![0.0; window];
        }
        let buf = &self.temporal_buffers[node_idx];
        if buf.len() >= window {
            buf[buf.len() - window..].to_vec()
        } else {
            let mut result = vec![0.0f32; window - buf.len()];
            result.extend_from_slice(buf);
            result
        }
    }

    /// Baseline fused feature: own + 1-hop mean-agg + last-16 temporal.
    pub fn fused_features(&self, node_idx: usize) -> Vec<f32> {
        let own = self
            .nodes
            .get(node_idx)
            .map(|n| n.features.clone())
            .unwrap_or_default();
        let spatial = self.aggregate_neighbors(node_idx, 1);
        let temporal = self.temporal_features(node_idx, 16);
        let mut fused = Vec::with_capacity(own.len() + spatial.len() + temporal.len());
        fused.extend_from_slice(&own);
        fused.extend_from_slice(&spatial);
        fused.extend_from_slice(&temporal);
        fused
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Feature dimension of node 0 (0 if empty). Assumes homogeneous features.
    pub fn feature_dim(&self) -> usize {
        self.nodes.first().map(|n| n.features.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buoy(id: &str, lat: f64, lon: f64, depth: f64, features: Vec<f32>) -> SensorNode {
        SensorNode {
            id: id.to_string(),
            position: (lat, lon, depth),
            features,
        }
    }

    #[test]
    fn new_graph_is_empty() {
        let g = SensorGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn add_buoy_increments_count() {
        let mut g = SensorGraph::new();
        let idx = g.add_buoy(make_buoy("B1", 0.0, 0.0, 10.0, vec![1.0, 2.0]));
        assert_eq!(idx, 0);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.temporal_buffers.len(), 1);
    }

    #[test]
    fn connect_creates_edge_with_distance() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0]));
        g.add_buoy(make_buoy("B2", 3.0, 4.0, 0.0, vec![2.0]));
        g.connect(0, 1);

        assert_eq!(g.edge_count(), 1);
        let edge = &g.edges[0];
        assert!((edge.distance_m - 5.0).abs() < 1e-6);
        assert!(edge.propagation_delay_ms > 0.0);
    }

    #[test]
    fn connect_within_radius_filters() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("A", 0.0, 0.0, 0.0, vec![0.0; 5]));
        g.add_buoy(make_buoy("B", 1.0, 0.0, 0.0, vec![0.0; 5]));
        g.add_buoy(make_buoy("C", 10.0, 0.0, 0.0, vec![0.0; 5]));
        g.connect_within_radius(2.0);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edges[0].from, 0);
        assert_eq!(g.edges[0].to, 1);
    }

    #[test]
    fn connect_fully_complete() {
        let mut g = SensorGraph::new();
        for i in 0..4 {
            g.add_buoy(make_buoy(&format!("B{i}"), i as f64, 0.0, 0.0, vec![0.0; 5]));
        }
        g.connect_fully();
        // C(4,2) = 6
        assert_eq!(g.edge_count(), 6);
    }

    #[test]
    fn aggregate_neighbors_single_hop() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0, 0.0]));
        g.add_buoy(make_buoy("B2", 1.0, 0.0, 0.0, vec![0.0, 1.0]));
        g.add_buoy(make_buoy("B3", 0.0, 1.0, 0.0, vec![0.0, 1.0]));
        g.connect(0, 1);
        g.connect(0, 2);

        let agg = g.aggregate_neighbors(0, 1);
        assert_eq!(agg.len(), 2);
        assert!((agg[0] - 0.0).abs() < 0.01);
        assert!((agg[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn aggregate_neighbors_no_neighbors_returns_self() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![5.0, 3.0]));
        let agg = g.aggregate_neighbors(0, 1);
        assert_eq!(agg, vec![5.0, 3.0]);
    }

    #[test]
    fn aggregate_neighbors_hop_zero_returns_self() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![5.0]));
        g.add_buoy(make_buoy("B2", 1.0, 0.0, 0.0, vec![9.0]));
        g.connect(0, 1);
        let agg = g.aggregate_neighbors(0, 0);
        assert_eq!(agg, vec![5.0]);
    }

    #[test]
    fn temporal_features_zero_padded() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0]));
        g.push_temporal(0, &[0.5, 0.6, 0.7]);
        let tf = g.temporal_features(0, 5);
        assert_eq!(tf, vec![0.0, 0.0, 0.5, 0.6, 0.7]);
    }

    #[test]
    fn temporal_features_exact_window() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0]));
        g.push_temporal(0, &[1.0, 2.0, 3.0]);
        assert_eq!(g.temporal_features(0, 3), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn temporal_features_larger_buffer_takes_tail() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0]));
        g.push_temporal(0, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(g.temporal_features(0, 3), vec![3.0, 4.0, 5.0]);
    }

    #[test]
    fn fused_features_length() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0, 2.0, 3.0]));
        g.add_buoy(make_buoy("B2", 1.0, 0.0, 0.0, vec![4.0, 5.0, 6.0]));
        g.connect(0, 1);
        g.push_temporal(0, &[0.1; 20]);
        let fused = g.fused_features(0);
        assert_eq!(fused.len(), 22);
    }

    #[test]
    fn fused_features_isolated_node() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0, 2.0]));
        let fused = g.fused_features(0);
        assert_eq!(fused.len(), 20);
    }

    #[test]
    #[should_panic(expected = "from index out of bounds")]
    fn connect_out_of_bounds_panics() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("B1", 0.0, 0.0, 0.0, vec![1.0]));
        g.connect(5, 0);
    }

    #[test]
    fn multi_hop_aggregation() {
        let mut g = SensorGraph::new();
        g.add_buoy(make_buoy("A", 0.0, 0.0, 0.0, vec![1.0]));
        g.add_buoy(make_buoy("B", 1.0, 0.0, 0.0, vec![2.0]));
        g.add_buoy(make_buoy("C", 2.0, 0.0, 0.0, vec![3.0]));
        g.connect(0, 1);
        g.connect(1, 2);

        let agg1 = g.aggregate_neighbors(0, 1);
        assert!((agg1[0] - 2.0).abs() < 0.01);

        let agg2 = g.aggregate_neighbors(0, 2);
        assert!(agg2[0] > 1.0 && agg2[0] < 3.0);
    }

    #[test]
    fn physics_slice_padded() {
        let n = make_buoy("x", 0.0, 0.0, 0.0, vec![1.0, 2.0]);
        let p = n.physics();
        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);
        assert_eq!(p[2], 0.0);
    }
}
