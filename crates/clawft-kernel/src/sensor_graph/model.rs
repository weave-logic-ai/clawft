//! K-STEMIT dual-branch model: temporal GLU + spatial GraphSAGE + physics + head.

use super::graph::{SensorGraph, PHYSICS_DIM};
use super::layers::{
    cross_entropy, softmax, softmax_ce_grad, GraphSageLayer, Linear, TemporalGlu,
};

const DEFAULT_SEED: u64 = 0x00c5_7e71;

/// Hyperparameters for [`KstemitModel`].
#[derive(Debug, Clone)]
pub struct KstemitConfig {
    /// Node feature dimension (includes physics priors).
    pub feature_dim: usize,
    /// Temporal window length.
    pub temporal_window: usize,
    /// Shared hidden size for both branches.
    pub hidden: usize,
    /// Number of classification labels.
    pub num_classes: usize,
    /// Scale on the physics residual branch (γ).
    pub physics_scale: f32,
    /// RNG seed for weight init.
    pub seed: u64,
}

impl Default for KstemitConfig {
    fn default() -> Self {
        Self {
            feature_dim: PHYSICS_DIM,
            temporal_window: 16,
            hidden: 8,
            num_classes: 3,
            physics_scale: 0.25,
            seed: DEFAULT_SEED,
        }
    }
}

/// Outputs of a single-node forward pass.
#[derive(Debug, Clone)]
pub struct ForwardOutput {
    /// Spatial branch embedding.
    pub h_spatial: Vec<f32>,
    /// Temporal branch embedding.
    pub h_temporal: Vec<f32>,
    /// Physics prior embedding.
    pub h_physics: Vec<f32>,
    /// Fused hidden: `α·h_s + (1-α)·h_t + γ·h_p`.
    pub h_fused: Vec<f32>,
    /// Classification logits.
    pub logits: Vec<f32>,
    /// Softmax probabilities.
    pub probs: Vec<f32>,
    /// Adaptive fusion weight α = σ(α_logit).
    pub alpha: f32,
}

/// K-STEMIT-style spatio-temporal GNN for sonobuoy arrays.
///
/// ```text
/// temporal window ──► TemporalGlu ──► h_t ──┐
///                                           │ α
/// node feats + AGG ─► GraphSAGE ───► h_s ───┼──► fuse ──► Linear head ──► logits
///                                           │
/// physics priors ───► Linear ──────► h_p ───┘ (γ residual)
/// ```
#[derive(Debug, Clone)]
pub struct KstemitModel {
    pub config: KstemitConfig,
    pub temporal: TemporalGlu,
    pub spatial: GraphSageLayer,
    pub physics_proj: Linear,
    /// Unconstrained logit; α = sigmoid(alpha_logit).
    pub alpha_logit: f32,
    pub head: Linear,
}

impl KstemitModel {
    /// Construct a freshly initialized model.
    pub fn new(config: KstemitConfig) -> Self {
        assert!(config.feature_dim > 0);
        assert!(config.temporal_window > 0);
        assert!(config.hidden > 0);
        assert!(config.num_classes > 0);
        let s = config.seed;
        Self {
            temporal: TemporalGlu::new(config.temporal_window, config.hidden, s),
            spatial: GraphSageLayer::new(config.feature_dim, config.hidden, s.wrapping_add(1)),
            physics_proj: Linear::xavier(
                PHYSICS_DIM.min(config.feature_dim).max(1),
                config.hidden,
                s.wrapping_add(2),
            ),
            alpha_logit: 0.0, // α ≈ 0.5 at init
            head: Linear::xavier(config.hidden, config.num_classes, s.wrapping_add(3)),
            config,
        }
    }

    /// Current fusion weight α ∈ (0, 1).
    pub fn alpha(&self) -> f32 {
        sigmoid(self.alpha_logit)
    }

    /// Forward for one node index on a [`SensorGraph`].
    pub fn forward_node(&self, graph: &SensorGraph, node_idx: usize) -> ForwardOutput {
        let (x_self, x_agg, series, physics) = self.inputs_for(graph, node_idx);
        self.forward_tensors(&x_self, &x_agg, &series, &physics)
    }

    /// Build model inputs for a node (public for training caches).
    pub fn inputs_for(
        &self,
        graph: &SensorGraph,
        node_idx: usize,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
        let feat_dim = self.config.feature_dim;
        let x_self = pad_or_trunc(
            graph
                .nodes
                .get(node_idx)
                .map(|n| n.features.as_slice())
                .unwrap_or(&[]),
            feat_dim,
        );
        let x_agg = pad_or_trunc(&graph.aggregate_neighbors(node_idx, 1), feat_dim);
        let series = graph.temporal_features(node_idx, self.config.temporal_window);
        let physics = {
            let raw = graph
                .nodes
                .get(node_idx)
                .map(|n| n.physics())
                .unwrap_or([0.0; PHYSICS_DIM]);
            let need = self.physics_proj.in_dim;
            pad_or_trunc(&raw, need)
        };
        (x_self, x_agg, series, physics)
    }

    /// Forward from pre-built tensors.
    pub fn forward_tensors(
        &self,
        x_self: &[f32],
        x_agg: &[f32],
        series: &[f32],
        physics: &[f32],
    ) -> ForwardOutput {
        let h_spatial = self.spatial.forward(x_self, x_agg);
        let h_temporal = self.temporal.forward(series);
        let h_physics = self.physics_proj.forward(physics);
        let alpha = self.alpha();
        let gamma = self.config.physics_scale;
        let hidden = self.config.hidden;
        let mut h_fused = vec![0.0f32; hidden];
        for i in 0..hidden {
            h_fused[i] =
                alpha * h_spatial[i] + (1.0 - alpha) * h_temporal[i] + gamma * h_physics[i];
        }
        let logits = self.head.forward(&h_fused);
        let probs = softmax(&logits);
        ForwardOutput {
            h_spatial,
            h_temporal,
            h_physics,
            h_fused,
            logits,
            probs,
            alpha,
        }
    }

    /// Predicted class (argmax).
    pub fn predict(&self, graph: &SensorGraph, node_idx: usize) -> usize {
        let out = self.forward_node(graph, node_idx);
        argmax(&out.logits)
    }

    /// Loss for one labelled node.
    pub fn loss_node(&self, graph: &SensorGraph, node_idx: usize, label: usize) -> f32 {
        let out = self.forward_node(graph, node_idx);
        cross_entropy(&out.probs, label)
    }

    /// One SGD step on a single (graph, node, label) sample.
    ///
    /// Analytical gradients through GLU, GraphSAGE, fusion α, and head.
    pub fn train_step(
        &mut self,
        graph: &SensorGraph,
        node_idx: usize,
        label: usize,
        lr: f32,
        weight_decay: f32,
    ) -> f32 {
        let (x_self, x_agg, series, physics) = self.inputs_for(graph, node_idx);
        let out = self.forward_tensors(&x_self, &x_agg, &series, &physics);
        let loss = cross_entropy(&out.probs, label);

        // dL/dlogits
        let dlogits = softmax_ce_grad(&out.probs, label);

        // head backward
        let mut dhead_w = vec![0.0; self.head.w.len()];
        let mut dhead_b = vec![0.0; self.head.b.len()];
        let dh_fused = self
            .head
            .backward(&out.h_fused, &dlogits, &mut dhead_w, &mut dhead_b);

        let alpha = out.alpha;
        let gamma = self.config.physics_scale;
        let hidden = self.config.hidden;

        // fusion: h = α h_s + (1-α) h_t + γ h_p
        let mut dh_s = vec![0.0f32; hidden];
        let mut dh_t = vec![0.0f32; hidden];
        let mut dh_p = vec![0.0f32; hidden];
        let mut d_alpha = 0.0f32;
        for i in 0..hidden {
            dh_s[i] = alpha * dh_fused[i];
            dh_t[i] = (1.0 - alpha) * dh_fused[i];
            dh_p[i] = gamma * dh_fused[i];
            d_alpha += dh_fused[i] * (out.h_spatial[i] - out.h_temporal[i]);
        }
        // α = σ(logit) ⇒ dlogit = dα · α · (1-α)
        let d_alpha_logit = d_alpha * alpha * (1.0 - alpha);

        // temporal GLU
        let mut dproj_w = vec![0.0; self.temporal.proj.w.len()];
        let mut dproj_b = vec![0.0; self.temporal.proj.b.len()];
        let _dseries = self
            .temporal
            .backward(&series, &dh_t, &mut dproj_w, &mut dproj_b);

        // spatial GraphSAGE
        let mut dws_w = vec![0.0; self.spatial.w_self.w.len()];
        let mut dws_b = vec![0.0; self.spatial.w_self.b.len()];
        let mut dwn_w = vec![0.0; self.spatial.w_neigh.w.len()];
        let mut dwn_b = vec![0.0; self.spatial.w_neigh.b.len()];
        let _ = self.spatial.backward(
            &x_self,
            &x_agg,
            &dh_s,
            &mut dws_w,
            &mut dws_b,
            &mut dwn_w,
            &mut dwn_b,
        );

        // physics proj
        let mut dphys_w = vec![0.0; self.physics_proj.w.len()];
        let mut dphys_b = vec![0.0; self.physics_proj.b.len()];
        let _ = self
            .physics_proj
            .backward(&physics, &dh_p, &mut dphys_w, &mut dphys_b);

        // apply updates
        self.head.sgd_step(&dhead_w, &dhead_b, lr, weight_decay);
        self.temporal
            .proj
            .sgd_step(&dproj_w, &dproj_b, lr, weight_decay);
        self.spatial
            .w_self
            .sgd_step(&dws_w, &dws_b, lr, weight_decay);
        self.spatial
            .w_neigh
            .sgd_step(&dwn_w, &dwn_b, lr, weight_decay);
        self.physics_proj
            .sgd_step(&dphys_w, &dphys_b, lr, weight_decay);
        self.alpha_logit -= lr * d_alpha_logit;

        loss
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn pad_or_trunc(v: &[f32], dim: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; dim];
    let n = v.len().min(dim);
    out[..n].copy_from_slice(&v[..n]);
    out
}

fn argmax(v: &[f32]) -> usize {
    let mut best_i = 0;
    let mut best_v = f32::NEG_INFINITY;
    for (i, &x) in v.iter().enumerate() {
        if x > best_v {
            best_v = x;
            best_i = i;
        }
    }
    best_i
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor_graph::graph::SensorNode;

    fn tiny_graph() -> SensorGraph {
        let mut g = SensorGraph::new();
        g.add_buoy(SensorNode {
            id: "A".into(),
            position: (0.0, 0.0, 10.0),
            features: vec![0.5, 0.2, 0.1, 0.0, 0.3],
        });
        g.add_buoy(SensorNode {
            id: "B".into(),
            position: (1.0, 0.0, 10.0),
            features: vec![0.4, 0.2, 0.2, 0.1, 0.3],
        });
        g.connect(0, 1);
        g.push_temporal(0, &[0.1, 0.2, 0.8, 0.9, 0.7, 0.3, 0.1, 0.0]);
        g.push_temporal(0, &[0.0; 8]);
        g.push_temporal(1, &[0.05; 16]);
        g
    }

    #[test]
    fn forward_shapes() {
        let cfg = KstemitConfig {
            feature_dim: 5,
            temporal_window: 16,
            hidden: 4,
            num_classes: 3,
            physics_scale: 0.25,
            seed: 1,
        };
        let model = KstemitModel::new(cfg);
        let g = tiny_graph();
        let out = model.forward_node(&g, 0);
        assert_eq!(out.h_spatial.len(), 4);
        assert_eq!(out.h_temporal.len(), 4);
        assert_eq!(out.h_physics.len(), 4);
        assert_eq!(out.h_fused.len(), 4);
        assert_eq!(out.logits.len(), 3);
        assert_eq!(out.probs.len(), 3);
        let s: f32 = out.probs.iter().sum();
        assert!((s - 1.0).abs() < 1e-5);
        assert!(out.alpha > 0.0 && out.alpha < 1.0);
    }

    #[test]
    fn train_step_decreases_loss_on_easy_label() {
        let cfg = KstemitConfig {
            feature_dim: 5,
            temporal_window: 16,
            hidden: 8,
            num_classes: 3,
            physics_scale: 0.25,
            seed: 99,
        };
        let mut model = KstemitModel::new(cfg);
        let g = tiny_graph();
        let label = 1usize;
        let loss0 = model.loss_node(&g, 0, label);
        for _ in 0..40 {
            let _ = model.train_step(&g, 0, label, 0.05, 1e-4);
        }
        let loss1 = model.loss_node(&g, 0, label);
        assert!(
            loss1 < loss0,
            "expected loss drop: {loss0:.4} → {loss1:.4}"
        );
    }

    #[test]
    fn predict_returns_valid_class() {
        let model = KstemitModel::new(KstemitConfig::default());
        let g = tiny_graph();
        let c = model.predict(&g, 0);
        assert!(c < model.config.num_classes);
    }
}
