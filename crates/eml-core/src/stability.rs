//! Numerical-stability scaffolding for nested EML exp/ln (WEFT-548).
//!
//! # Catalogue of loci (CIFAR-10 / deep-tree blow-ups)
//!
//! | Locus | Mechanism | Failure mode | Mitigation |
//! |-------|-----------|--------------|------------|
//! | `eml` raw `exp(x)` | `x ≳ 88` → `+∞` | NaN/Inf cascade | [`EXP_CLAMP`] via [`eml_safe`] / [`stable_eml`] |
//! | `eml` raw `ln(y)` | `y ≤ 0` → NaN/`-∞` | domain error | `y` floor at [`LN_ARG_FLOOR`] |
//! | Nested EML tree depth ≥ 4 | intermediate magnitudes compound | training divergence | per-level clamp + [`normalize_features`] |
//! | Softmax logits | large positive max → overflow | all-NaN weights | log-sum-exp / max-subtract ([`log_sum_exp`], [`softmax3`]) |
//! | Coordinate-descent steps | unbounded param drift | non-finite loss | [`clamp_params`] + [`AdaScaleState`] |
//!
//! This module is the **framework-level stability layer** for `eml-core`.
//! Evaluation trees already call [`crate::operator::eml_safe`]; training
//! loops and experimental attention should prefer the helpers here so
//! clamp bounds stay single-sourced.

use crate::operator::{eml_safe, softmax3};

/// Upper bound on the exp argument (≈ exp(20) ≈ 4.85e8).
pub const EXP_CLAMP: f64 = 20.0;

/// Lower bound on the exp argument.
pub const EXP_CLAMP_LO: f64 = -20.0;

/// Floor for `ln` arguments (avoids domain error without going to −∞).
pub const LN_ARG_FLOOR: f64 = f64::MIN_POSITIVE;

/// Default per-feature clamp after normalization to ~[0, 1] ± headroom.
pub const FEATURE_CLAMP: f64 = 10.0;

/// Default parameter clamp used during coordinate descent.
pub const PARAM_CLAMP: f64 = 5.0;

/// Stable EML using the module-wide clamp constants.
///
/// Equivalent to [`eml_safe`] but documented as the stability-layer entry
/// point so call sites can migrate without chasing operator internals.
#[inline]
pub fn stable_eml(x: f64, y: f64) -> f64 {
    eml_safe(x, y)
}

/// Numerically stable log-sum-exp: `log(Σ exp(x_i))`.
///
/// Uses the classic max-subtraction trick so large logits do not overflow.
pub fn log_sum_exp(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return f64::NEG_INFINITY;
    }
    let mut max = f64::NEG_INFINITY;
    for &x in xs {
        if x > max {
            max = x;
        }
    }
    if !max.is_finite() {
        return max;
    }
    let mut sum = 0.0;
    for &x in xs {
        sum += (x - max).exp();
    }
    max + sum.ln()
}

/// Clamp a scalar into `[lo, hi]`, preserving non-finite → midpoint.
#[inline]
pub fn clamp_finite(x: f64, lo: f64, hi: f64) -> f64 {
    if x.is_finite() {
        x.clamp(lo, hi)
    } else {
        0.5 * (lo + hi)
    }
}

/// Clamp a parameter vector into `[-PARAM_CLAMP, PARAM_CLAMP]`.
pub fn clamp_params(params: &mut [f64]) {
    for p in params.iter_mut() {
        *p = clamp_finite(*p, -PARAM_CLAMP, PARAM_CLAMP);
    }
}

/// L2-normalize a feature vector then scale into `[-FEATURE_CLAMP, FEATURE_CLAMP]`.
///
/// Empty input is a no-op. Zero-norm vectors become zeros.
pub fn normalize_features(features: &mut [f64]) {
    if features.is_empty() {
        return;
    }
    let mut sum_sq = 0.0;
    for &x in features.iter() {
        if x.is_finite() {
            sum_sq += x * x;
        }
    }
    let norm = sum_sq.sqrt();
    if norm < 1e-12 {
        for x in features.iter_mut() {
            *x = 0.0;
        }
        return;
    }
    for x in features.iter_mut() {
        let v = if x.is_finite() { *x / norm } else { 0.0 };
        *x = v.clamp(-FEATURE_CLAMP, FEATURE_CLAMP);
    }
}

/// AdaScale-style running gain for nested exp/ln chains.
///
/// Tracks an exponential moving average of `|activation|` and produces a
/// multiplicative gain that keeps typical activations near `target_rms`.
/// This is a lightweight stand-in for full AdaScale (Liu et al.) — enough
/// to stop CIFAR-class depth-5 blow-ups without a second optimizer.
#[derive(Debug, Clone)]
pub struct AdaScaleState {
    /// EMA of absolute activation magnitude.
    pub ema_abs: f64,
    /// Target RMS-ish magnitude for activations.
    pub target_rms: f64,
    /// EMA decay in `(0, 1)`.
    pub decay: f64,
    /// Gain clamp floor / ceiling.
    pub gain_lo: f64,
    /// Gain clamp ceiling.
    pub gain_hi: f64,
}

impl Default for AdaScaleState {
    fn default() -> Self {
        Self {
            ema_abs: 1.0,
            target_rms: 1.0,
            decay: 0.99,
            gain_lo: 0.05,
            gain_hi: 20.0,
        }
    }
}

impl AdaScaleState {
    /// Observe a batch of activations and update the EMA.
    pub fn observe(&mut self, activations: &[f64]) {
        if activations.is_empty() {
            return;
        }
        let mut sum = 0.0;
        let mut n = 0usize;
        for &a in activations {
            if a.is_finite() {
                sum += a.abs();
                n += 1;
            }
        }
        if n == 0 {
            return;
        }
        let mean_abs = sum / n as f64;
        self.ema_abs = self.decay * self.ema_abs + (1.0 - self.decay) * mean_abs;
    }

    /// Multiplicative gain: `target_rms / max(ema_abs, ε)`.
    pub fn gain(&self) -> f64 {
        let g = self.target_rms / self.ema_abs.max(1e-8);
        g.clamp(self.gain_lo, self.gain_hi)
    }

    /// Scale activations in place by the current gain.
    pub fn apply(&self, activations: &mut [f64]) {
        let g = self.gain();
        for a in activations.iter_mut() {
            if a.is_finite() {
                *a *= g;
            } else {
                *a = 0.0;
            }
        }
    }
}

/// Evaluate a shallow nested EML chain with stability wrappers.
///
/// Used to demonstrate finite outputs on deep-ish compositions that would
/// blow up with raw [`crate::operator::eml`].
pub fn stable_nested_eml(seed: f64, depth: usize) -> f64 {
    let mut x = clamp_finite(seed, EXP_CLAMP_LO, EXP_CLAMP);
    let mut y = 1.0;
    for i in 0..depth {
        // Mix via stable softmax so branch weights stay in (0,1).
        let (a, b, c) = softmax3(x, y, seed + i as f64 * 0.01);
        let left = a * x + b * y + c;
        let right = (a * y + b + 1e-3).max(LN_ARG_FLOOR);
        x = stable_eml(left, right);
        y = (x.abs() + 1.0).max(LN_ARG_FLOOR);
        x = clamp_finite(x, EXP_CLAMP_LO, EXP_CLAMP);
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::eml;

    #[test]
    fn log_sum_exp_matches_naive_for_small() {
        let xs = [0.1_f64, 0.2, 0.3];
        let naive = xs.iter().map(|x| x.exp()).sum::<f64>().ln();
        let stable = log_sum_exp(&xs);
        assert!((naive - stable).abs() < 1e-12);
    }

    #[test]
    fn log_sum_exp_handles_large_logits() {
        let xs = [1000.0, 1001.0, 999.0];
        let v = log_sum_exp(&xs);
        assert!(v.is_finite(), "got {v}");
        // Dominated by 1001: log(e^1001 * (e^-1 + 1 + e^-2)) ≈ 1001 + log(1+1/e+1/e²)
        let expected = 1001.0 + (1.0 + (-1.0_f64).exp() + (-2.0_f64).exp()).ln();
        assert!((v - expected).abs() < 1e-9);
    }

    #[test]
    fn raw_eml_blows_up_stable_does_not() {
        let raw = eml(100.0, 1e-300);
        assert!(!raw.is_finite() || raw.abs() > 1e20 || raw.is_nan());
        let s = stable_eml(100.0, 1e-300);
        assert!(s.is_finite(), "stable_eml must be finite, got {s}");
    }

    #[test]
    fn nested_chain_stays_finite() {
        // Depth comparable to CIFAR-scale trees that previously diverged.
        for seed in [-5.0, 0.0, 1.5, 8.0, 50.0] {
            let v = stable_nested_eml(seed, 12);
            assert!(
                v.is_finite(),
                "stable_nested_eml(seed={seed}, depth=12) = {v}"
            );
        }
    }

    #[test]
    fn normalize_and_clamp_params() {
        let mut f = [3.0, 4.0, 0.0];
        normalize_features(&mut f);
        let norm = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt();
        assert!((norm - 1.0).abs() < 1e-12);

        let mut p = [100.0, f64::NAN, -100.0];
        clamp_params(&mut p);
        assert!(p[0] <= PARAM_CLAMP && p[0] >= -PARAM_CLAMP);
        assert!(p[1].is_finite());
        assert!(p[2] >= -PARAM_CLAMP);
    }

    #[test]
    fn adascale_reduces_blowup() {
        let mut state = AdaScaleState::default();
        let mut acts = [100.0, 200.0, 50.0];
        state.observe(&acts);
        let g = state.gain();
        assert!(g < 1.0, "large activations should produce gain < 1, got {g}");
        state.apply(&mut acts);
        for a in &acts {
            assert!(a.is_finite());
            assert!(*a < 100.0);
        }
    }
}
