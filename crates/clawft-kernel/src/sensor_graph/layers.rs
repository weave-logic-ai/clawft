//! Pure-Rust linear / GraphSAGE / GLU layers for K-STEMIT forward + SGD.
//!
//! No external BLAS / ndarray. Matrices are row-major `Vec<f32>`.

/// Dense affine map `y = Wx + b` with analytical gradients for SGD.
#[derive(Debug, Clone)]
pub struct Linear {
    /// Weight matrix, row-major, shape `[out_dim, in_dim]`.
    pub w: Vec<f32>,
    /// Bias, length `out_dim`.
    pub b: Vec<f32>,
    pub in_dim: usize,
    pub out_dim: usize,
}

impl Linear {
    /// Zero-initialized layer.
    pub fn zeros(in_dim: usize, out_dim: usize) -> Self {
        Self {
            w: vec![0.0; out_dim * in_dim],
            b: vec![0.0; out_dim],
            in_dim,
            out_dim,
        }
    }

    /// Xavier/Glorot uniform-ish init from a simple LCG seed.
    pub fn xavier(in_dim: usize, out_dim: usize, seed: u64) -> Self {
        let mut rng = seed;
        let limit = (6.0f32 / (in_dim + out_dim) as f32).sqrt();
        let mut w = vec![0.0f32; out_dim * in_dim];
        for slot in &mut w {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = (rng >> 33) as f32 / (u32::MAX as f32);
            *slot = (u * 2.0 - 1.0) * limit;
        }
        Self {
            w,
            b: vec![0.0; out_dim],
            in_dim,
            out_dim,
        }
    }

    /// Forward: `y = Wx + b`.
    pub fn forward(&self, x: &[f32]) -> Vec<f32> {
        debug_assert_eq!(x.len(), self.in_dim);
        let mut y = self.b.clone();
        for o in 0..self.out_dim {
            let row = o * self.in_dim;
            let mut acc = y[o];
            for i in 0..self.in_dim {
                acc += self.w[row + i] * x[i];
            }
            y[o] = acc;
        }
        y
    }

    /// Backward given `dy` and cached input `x`.
    ///
    /// Returns `dx` and accumulates into `dw` / `db` buffers (same layout as `w` / `b`).
    pub fn backward(
        &self,
        x: &[f32],
        dy: &[f32],
        dw: &mut [f32],
        db: &mut [f32],
    ) -> Vec<f32> {
        debug_assert_eq!(x.len(), self.in_dim);
        debug_assert_eq!(dy.len(), self.out_dim);
        debug_assert_eq!(dw.len(), self.w.len());
        debug_assert_eq!(db.len(), self.b.len());

        for o in 0..self.out_dim {
            db[o] += dy[o];
            let row = o * self.in_dim;
            for i in 0..self.in_dim {
                dw[row + i] += dy[o] * x[i];
            }
        }

        let mut dx = vec![0.0f32; self.in_dim];
        for o in 0..self.out_dim {
            let row = o * self.in_dim;
            for i in 0..self.in_dim {
                dx[i] += self.w[row + i] * dy[o];
            }
        }
        dx
    }

    /// SGD step with optional L2 weight decay on `w`.
    pub fn sgd_step(&mut self, dw: &[f32], db: &[f32], lr: f32, weight_decay: f32) {
        for i in 0..self.w.len() {
            self.w[i] -= lr * (dw[i] + weight_decay * self.w[i]);
        }
        for i in 0..self.b.len() {
            self.b[i] -= lr * db[i];
        }
    }
}

/// Gated linear unit: `y = a ⊙ σ(g)` where `[a;g] = W x + b`.
#[derive(Debug, Clone)]
pub struct TemporalGlu {
    /// Projects flattened temporal window → `2 * hidden`.
    pub proj: Linear,
    pub window: usize,
    pub hidden: usize,
}

impl TemporalGlu {
    pub fn new(window: usize, hidden: usize, seed: u64) -> Self {
        Self {
            proj: Linear::xavier(window, 2 * hidden, seed),
            window,
            hidden,
        }
    }

    /// Forward over a length-`window` series (already padded/cropped).
    pub fn forward(&self, series: &[f32]) -> Vec<f32> {
        assert_eq!(series.len(), self.window);
        let z = self.proj.forward(series);
        glu_activate(&z, self.hidden)
    }

    /// Backward through GLU + projection.
    pub fn backward(
        &self,
        series: &[f32],
        dy: &[f32],
        dproj_w: &mut [f32],
        dproj_b: &mut [f32],
    ) -> Vec<f32> {
        let z = self.proj.forward(series);
        let dz = glu_backward(&z, dy, self.hidden);
        self.proj.backward(series, &dz, dproj_w, dproj_b)
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn glu_activate(z: &[f32], hidden: usize) -> Vec<f32> {
    debug_assert_eq!(z.len(), 2 * hidden);
    let mut y = vec![0.0f32; hidden];
    for i in 0..hidden {
        y[i] = z[i] * sigmoid(z[hidden + i]);
    }
    y
}

fn glu_backward(z: &[f32], dy: &[f32], hidden: usize) -> Vec<f32> {
    debug_assert_eq!(z.len(), 2 * hidden);
    debug_assert_eq!(dy.len(), hidden);
    let mut dz = vec![0.0f32; 2 * hidden];
    for i in 0..hidden {
        let a = z[i];
        let s = sigmoid(z[hidden + i]);
        // y = a * s
        dz[i] = dy[i] * s;
        dz[hidden + i] = dy[i] * a * s * (1.0 - s);
    }
    dz
}

/// One GraphSAGE hop: `h = σ(W_self x + W_neigh AGG(N(x)))`.
///
/// `AGG` is inverse-distance weighted mean supplied by the caller.
/// Activation is ReLU.
#[derive(Debug, Clone)]
pub struct GraphSageLayer {
    pub w_self: Linear,
    pub w_neigh: Linear,
    pub in_dim: usize,
    pub hidden: usize,
}

impl GraphSageLayer {
    pub fn new(in_dim: usize, hidden: usize, seed: u64) -> Self {
        Self {
            w_self: Linear::xavier(in_dim, hidden, seed),
            w_neigh: Linear::xavier(in_dim, hidden, seed.wrapping_add(17)),
            in_dim,
            hidden,
        }
    }

    /// Forward one node given self features and pre-aggregated neighbor features.
    pub fn forward(&self, x_self: &[f32], x_agg: &[f32]) -> Vec<f32> {
        let a = self.w_self.forward(x_self);
        let b = self.w_neigh.forward(x_agg);
        let mut h = vec![0.0f32; self.hidden];
        for i in 0..self.hidden {
            h[i] = relu(a[i] + b[i]);
        }
        h
    }

    /// Backward. Returns `(dx_self, dx_agg)`.
    pub fn backward(
        &self,
        x_self: &[f32],
        x_agg: &[f32],
        dy: &[f32],
        dw_self_w: &mut [f32],
        dw_self_b: &mut [f32],
        dw_neigh_w: &mut [f32],
        dw_neigh_b: &mut [f32],
    ) -> (Vec<f32>, Vec<f32>) {
        let a = self.w_self.forward(x_self);
        let b = self.w_neigh.forward(x_agg);
        // d(relu)/d(pre)
        let mut dpre = vec![0.0f32; self.hidden];
        for i in 0..self.hidden {
            let pre = a[i] + b[i];
            dpre[i] = if pre > 0.0 { dy[i] } else { 0.0 };
        }
        let dx_self = self
            .w_self
            .backward(x_self, &dpre, dw_self_w, dw_self_b);
        let dx_agg = self
            .w_neigh
            .backward(x_agg, &dpre, dw_neigh_w, dw_neigh_b);
        (dx_self, dx_agg)
    }
}

fn relu(x: f32) -> f32 {
    if x > 0.0 {
        x
    } else {
        0.0
    }
}

/// Softmax over logits (numerically stable).
pub fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut exps: Vec<f32> = logits.iter().map(|v| (v - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    if sum > 0.0 {
        for e in &mut exps {
            *e /= sum;
        }
    }
    exps
}

/// Cross-entropy loss for a single sample: `-log p[label]`.
pub fn cross_entropy(probs: &[f32], label: usize) -> f32 {
    let p = probs.get(label).copied().unwrap_or(0.0).clamp(1e-8, 1.0);
    -p.ln()
}

/// Softmax + CE gradient w.r.t. logits: `p - one_hot(label)`.
pub fn softmax_ce_grad(probs: &[f32], label: usize) -> Vec<f32> {
    let mut g = probs.to_vec();
    if label < g.len() {
        g[label] -= 1.0;
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_forward_identity_bias() {
        let mut layer = Linear::zeros(2, 2);
        // identity
        layer.w = vec![1.0, 0.0, 0.0, 1.0];
        layer.b = vec![1.0, -1.0];
        let y = layer.forward(&[3.0, 4.0]);
        assert!((y[0] - 4.0).abs() < 1e-5);
        assert!((y[1] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn linear_backward_matches_finite_diff() {
        let layer = Linear::xavier(3, 2, 42);
        let x = vec![0.5f32, -0.2, 1.0];
        let dy = vec![1.0f32, -0.5];
        let mut dw = vec![0.0; layer.w.len()];
        let mut db = vec![0.0; layer.b.len()];
        let _dx = layer.backward(&x, &dy, &mut dw, &mut db);

        // Finite-diff check on one weight.
        let eps = 1e-3f32;
        let mut layer_p = layer.clone();
        layer_p.w[0] += eps;
        let mut layer_m = layer.clone();
        layer_m.w[0] -= eps;
        let loss = |l: &Linear| {
            let y = l.forward(&x);
            y[0] * dy[0] + y[1] * dy[1]
        };
        let fd = (loss(&layer_p) - loss(&layer_m)) / (2.0 * eps);
        assert!((dw[0] - fd).abs() < 1e-2, "dw[0]={:.4} fd={:.4}", dw[0], fd);
    }

    #[test]
    fn glu_forward_gate_zero() {
        let mut glu = TemporalGlu::new(4, 2, 1);
        // Force proj to zeros → y = 0
        glu.proj = Linear::zeros(4, 4);
        let y = glu.forward(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(y.len(), 2);
        assert!((y[0]).abs() < 1e-6);
        assert!((y[1]).abs() < 1e-6);
    }

    #[test]
    fn graphsage_relu_positive_path() {
        let mut sage = GraphSageLayer::new(2, 2, 7);
        sage.w_self = Linear::zeros(2, 2);
        sage.w_self.w = vec![1.0, 0.0, 0.0, 1.0];
        sage.w_neigh = Linear::zeros(2, 2);
        // self [1,2] + neigh 0 → ReLU keeps positives
        let h = sage.forward(&[1.0, 2.0], &[0.0, 0.0]);
        assert!((h[0] - 1.0).abs() < 1e-5);
        assert!((h[1] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn softmax_sums_to_one() {
        let p = softmax(&[1.0, 2.0, 3.0]);
        let s: f32 = p.iter().sum();
        assert!((s - 1.0).abs() < 1e-5);
        assert!(p[2] > p[1] && p[1] > p[0]);
    }
}
