//! Train / eval scaffolding for [`KstemitModel`] on synthetic arrays.

use super::fixture::SyntheticDataset;
use super::model::{KstemitConfig, KstemitModel};

/// SGD hyper-parameters.
#[derive(Debug, Clone)]
pub struct TrainConfig {
    pub epochs: usize,
    pub learning_rate: f32,
    pub weight_decay: f32,
    /// Shuffle samples each epoch (LCG).
    pub shuffle: bool,
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            epochs: 30,
            learning_rate: 0.05,
            weight_decay: 1e-4,
            shuffle: true,
            seed: 7,
        }
    }
}

/// Result of a training run.
#[derive(Debug, Clone)]
pub struct TrainResult {
    pub model: KstemitModel,
    /// Mean CE loss per epoch.
    pub epoch_losses: Vec<f32>,
    /// Final train accuracy.
    pub train_accuracy: f32,
}

/// Classification metrics.
#[derive(Debug, Clone)]
pub struct EvalMetrics {
    pub accuracy: f32,
    pub mean_loss: f32,
    pub n: usize,
    /// Confusion matrix flattened row-major `[num_classes * num_classes]`.
    pub confusion: Vec<usize>,
    pub num_classes: usize,
}

/// Train a new model on a synthetic dataset.
pub fn train_sgd(
    dataset: &SyntheticDataset,
    model_cfg: KstemitConfig,
    train_cfg: TrainConfig,
) -> TrainResult {
    let mut model = KstemitModel::new(model_cfg);
    let mut rng = train_cfg.seed;
    let mut epoch_losses = Vec::with_capacity(train_cfg.epochs);
    let n = dataset.samples.len();
    assert!(n > 0, "empty dataset");

    for _epoch in 0..train_cfg.epochs {
        let mut order: Vec<usize> = (0..n).collect();
        if train_cfg.shuffle {
            // Fisher–Yates with LCG.
            for i in (1..n).rev() {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let j = (rng as usize) % (i + 1);
                order.swap(i, j);
            }
        }
        let mut loss_sum = 0.0f32;
        for &si in &order {
            let s = &dataset.samples[si];
            let loss = model.train_step(
                &dataset.graph,
                s.node_idx,
                s.label,
                train_cfg.learning_rate,
                train_cfg.weight_decay,
            );
            loss_sum += loss;
        }
        epoch_losses.push(loss_sum / n as f32);
    }

    let eval = evaluate(&model, dataset);
    TrainResult {
        model,
        epoch_losses,
        train_accuracy: eval.accuracy,
    }
}

/// Evaluate mean CE and accuracy on all samples.
pub fn evaluate(model: &KstemitModel, dataset: &SyntheticDataset) -> EvalMetrics {
    let num_classes = model.config.num_classes;
    let mut confusion = vec![0usize; num_classes * num_classes];
    let mut correct = 0usize;
    let mut loss_sum = 0.0f32;
    let n = dataset.samples.len();

    for s in &dataset.samples {
        let out = model.forward_node(&dataset.graph, s.node_idx);
        loss_sum += super::layers::cross_entropy(&out.probs, s.label);
        let pred = argmax(&out.logits);
        if pred == s.label {
            correct += 1;
        }
        if s.label < num_classes && pred < num_classes {
            confusion[s.label * num_classes + pred] += 1;
        }
    }

    EvalMetrics {
        accuracy: if n == 0 {
            0.0
        } else {
            correct as f32 / n as f32
        },
        mean_loss: if n == 0 { 0.0 } else { loss_sum / n as f32 },
        n,
        confusion,
        num_classes,
    }
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
    use crate::sensor_graph::fixture::{generate_synthetic_array, SyntheticConfig};
    use crate::sensor_graph::graph::PHYSICS_DIM;

    #[test]
    fn train_improves_over_chance_on_synthetic() {
        let ds = generate_synthetic_array(SyntheticConfig {
            n_buoys: 12,
            series_len: 32,
            connect_radius: 3.0,
            noise: 0.04,
            seed: 1234,
        });
        let model_cfg = KstemitConfig {
            feature_dim: PHYSICS_DIM,
            temporal_window: 16,
            hidden: 12,
            num_classes: 3,
            physics_scale: 0.25,
            seed: 3,
        };
        let train_cfg = TrainConfig {
            epochs: 40,
            learning_rate: 0.08,
            weight_decay: 1e-4,
            shuffle: true,
            seed: 9,
        };
        let result = train_sgd(&ds, model_cfg, train_cfg);
        assert!(result.epoch_losses.len() == 40);
        // Loss should trend down.
        let first = result.epoch_losses[0];
        let last = *result.epoch_losses.last().unwrap();
        assert!(
            last < first,
            "loss should drop: first={first:.4} last={last:.4}"
        );
        // Better than random (1/3) with margin — synthetic is separable.
        assert!(
            result.train_accuracy > 0.45,
            "acc={:.3} expected > 0.45",
            result.train_accuracy
        );
    }

    #[test]
    fn evaluate_shapes() {
        let ds = generate_synthetic_array(SyntheticConfig::default());
        let model = KstemitModel::new(KstemitConfig::default());
        let m = evaluate(&model, &ds);
        assert_eq!(m.n, ds.samples.len());
        assert_eq!(m.confusion.len(), 9);
    }
}
