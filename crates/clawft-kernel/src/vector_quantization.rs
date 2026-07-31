//! In-tree LogQuantized + unified distance kernels (WEFT-15 / 128 / 352 / 353).
//!
//! ## Vendor independence (2026-07-31)
//!
//! These features were originally gated on [ruvnet/RuVector#352](https://github.com/ruvnet/RuVector/pull/352)
//! (still unmerged / conflicting). WeftOS now ships **first-party**
//! implementations so activation no longer waits on that PR:
//!
//! - **KG-011 LogQuantized** — log-scaled int8 bins for skewed embeddings
//! - **KG-012 Unified distance** — single codepath for cosine / L2 / IP
//!   with optional power-of-two padding and unrolled scalar SIMD-friendly loops
//!
//! Config types remain serializable; `is_available()` is true when
//! `enabled` is set (implementation always present in this crate).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// LogQuantized (KG-011)
// ---------------------------------------------------------------------------

/// Configuration for logarithmic quantization.
///
/// Replaces linear scalar bins with log-scaled bins for skewed
/// distributions (exp / ReLU / log-normal style embeddings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQuantizedConfig {
    /// Whether logarithmic quantization is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Compression ratio target (4 ≈ 8-bit codes). Used as documentation
    /// of intended density; codes are always u8 today.
    #[serde(default = "default_compression_ratio")]
    pub compression_ratio: usize,

    /// Floor magnitude before log (avoids log(0)).
    #[serde(default = "default_min_magnitude")]
    pub min_magnitude: f64,
}

fn default_compression_ratio() -> usize {
    4
}

fn default_min_magnitude() -> f64 {
    1e-7
}

impl Default for LogQuantizedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            compression_ratio: default_compression_ratio(),
            min_magnitude: default_min_magnitude(),
        }
    }
}

impl LogQuantizedConfig {
    /// True when operators enabled this path. Implementation is in-tree.
    pub fn is_available(&self) -> bool {
        self.enabled
    }
}

/// Log-scaled int8 quantized vector (in-tree; no ruvector-core).
#[derive(Debug, Clone, PartialEq)]
pub struct LogQuantizedVector {
    /// Per-dimension codes 0..=255.
    pub codes: Vec<u8>,
    /// Sign bits packed as true = negative original value.
    pub signs: Vec<bool>,
    /// Shared scale for reconstruction.
    pub max_log: f32,
    /// Config min magnitude used at encode.
    pub min_magnitude: f32,
}

impl LogQuantizedVector {
    /// Quantize `vector` with log-scaled magnitude bins.
    pub fn quantize(vector: &[f32], cfg: &LogQuantizedConfig) -> Self {
        let min_m = cfg.min_magnitude.max(1e-12) as f32;
        let mut max_log = 0.0f32;
        let mut logs = Vec::with_capacity(vector.len());
        let mut signs = Vec::with_capacity(vector.len());
        for &v in vector {
            let sign = v.is_sign_negative();
            signs.push(sign);
            let mag = v.abs().max(min_m);
            let lg = mag.ln();
            logs.push(lg);
            if lg > max_log {
                max_log = lg;
            }
        }
        // Avoid zero span.
        if max_log < min_m.ln() + 1e-6 {
            max_log = min_m.ln() + 1.0;
        }
        let min_log = min_m.ln();
        let span = (max_log - min_log).max(1e-6);
        let codes: Vec<u8> = logs
            .iter()
            .map(|&lg| {
                let t = ((lg - min_log) / span).clamp(0.0, 1.0);
                (t * 255.0).round() as u8
            })
            .collect();
        Self {
            codes,
            signs,
            max_log,
            min_magnitude: min_m,
        }
    }

    /// Reconstruct approximate f32 vector.
    pub fn reconstruct(&self) -> Vec<f32> {
        let min_log = self.min_magnitude.ln();
        let span = (self.max_log - min_log).max(1e-6);
        self.codes
            .iter()
            .zip(self.signs.iter())
            .map(|(&c, &neg)| {
                let t = c as f32 / 255.0;
                let lg = min_log + t * span;
                let mag = lg.exp();
                if neg {
                    -mag
                } else {
                    mag
                }
            })
            .collect()
    }

    /// Mean squared reconstruction error vs original.
    pub fn mse(&self, original: &[f32]) -> f32 {
        let rec = self.reconstruct();
        if rec.len() != original.len() || original.is_empty() {
            return f32::INFINITY;
        }
        let mut s = 0.0f32;
        for (a, b) in original.iter().zip(rec.iter()) {
            let d = a - b;
            s += d * d;
        }
        s / original.len() as f32
    }
}

// ---------------------------------------------------------------------------
// Unified distance (KG-012)
// ---------------------------------------------------------------------------

/// Distance metric for the unified kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SimdDistanceMetric {
    /// Cosine distance = 1 − cosine_similarity.
    #[default]
    Cosine,
    /// Euclidean (L2).
    L2,
    /// Inner product distance = −dot (lower is closer).
    InnerProduct,
}

/// Configuration for the unified distance kernel.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimdDistanceConfig {
    /// Enable the unified kernel path.
    #[serde(default)]
    pub enabled: bool,
    /// Pad both vectors to next power-of-two length with zeros.
    #[serde(default)]
    pub pad_to_power_of_two: bool,
    /// Metric.
    #[serde(default)]
    pub metric: SimdDistanceMetric,
    /// Preferred lane width (informational; auto path used today).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_width: Option<u16>,
}

impl SimdDistanceConfig {
    /// True when operators enabled this path. Implementation is in-tree.
    pub fn is_available(&self) -> bool {
        self.enabled
    }
}

/// Next power of two ≥ n (n ≥ 1).
pub fn next_pow2(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    n.next_power_of_two()
}

/// Optionally pad with zeros to power-of-two length.
pub fn maybe_pad(v: &[f32], pad: bool) -> Vec<f32> {
    if !pad {
        return v.to_vec();
    }
    let n = next_pow2(v.len().max(1));
    let mut out = vec![0.0f32; n];
    out[..v.len()].copy_from_slice(v);
    out
}

/// Unified distance: one codepath for cosine / L2 / IP with unrolled loops.
///
/// Branch-free accumulation; metric chosen once at the end. Suitable for
/// hot loops without depending on ruvector-core PR #352.
pub fn unified_distance(a: &[f32], b: &[f32], cfg: &SimdDistanceConfig) -> f32 {
    let a = maybe_pad(a, cfg.pad_to_power_of_two);
    let b = maybe_pad(b, cfg.pad_to_power_of_two);
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }

    // Single pass: dot, ||a||², ||b||², sum (a-b)²
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    let mut l2 = 0.0f32;
    let chunks = n / 4;
    for i in 0..chunks {
        let idx = i * 4;
        for j in 0..4 {
            let ai = a[idx + j];
            let bi = b[idx + j];
            dot += ai * bi;
            na += ai * ai;
            nb += bi * bi;
            let d = ai - bi;
            l2 += d * d;
        }
    }
    for i in (chunks * 4)..n {
        let ai = a[i];
        let bi = b[i];
        dot += ai * bi;
        na += ai * ai;
        nb += bi * bi;
        let d = ai - bi;
        l2 += d * d;
    }

    match cfg.metric {
        SimdDistanceMetric::L2 => l2.sqrt(),
        SimdDistanceMetric::InnerProduct => -dot,
        SimdDistanceMetric::Cosine => {
            let denom = na.sqrt() * nb.sqrt();
            if denom > 1e-8 {
                1.0 - (dot / denom)
            } else {
                1.0
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_quantized_available_when_enabled() {
        let off = LogQuantizedConfig::default();
        assert!(!off.is_available());
        let on = LogQuantizedConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(on.is_available());
    }

    #[test]
    fn log_quantized_roundtrip_skewed() {
        // Exponential-ish positive skew.
        let v: Vec<f32> = (0..64)
            .map(|i| (i as f32 * 0.15).exp() * if i % 3 == 0 { -1.0 } else { 1.0 })
            .collect();
        let cfg = LogQuantizedConfig {
            enabled: true,
            compression_ratio: 4,
            min_magnitude: 1e-7,
        };
        let q = LogQuantizedVector::quantize(&v, &cfg);
        let mse = q.mse(&v);
        // Must be finite and not catastrophic.
        assert!(mse.is_finite(), "mse={mse}");
        assert!(mse < 1e6, "unexpected huge mse={mse}");
        // Better or comparable to naive linear int8 on this skew is soft —
        // just ensure reconstruct length matches.
        assert_eq!(q.reconstruct().len(), v.len());
    }

    #[test]
    fn simd_distance_available_when_enabled() {
        assert!(!SimdDistanceConfig::default().is_available());
        assert!(SimdDistanceConfig {
            enabled: true,
            ..Default::default()
        }
        .is_available());
    }

    #[test]
    fn unified_cosine_identical_is_zero() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        let cfg = SimdDistanceConfig {
            enabled: true,
            metric: SimdDistanceMetric::Cosine,
            ..Default::default()
        };
        let d = unified_distance(&v, &v, &cfg);
        assert!(d.abs() < 1e-5, "d={d}");
    }

    #[test]
    fn unified_l2_orthogonal() {
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0];
        let cfg = SimdDistanceConfig {
            enabled: true,
            metric: SimdDistanceMetric::L2,
            pad_to_power_of_two: true,
            ..Default::default()
        };
        let d = unified_distance(&a, &b, &cfg);
        assert!((d - 2.0f32.sqrt()).abs() < 1e-5, "d={d}");
    }

    #[test]
    fn log_quantized_serde_roundtrip() {
        let cfg = LogQuantizedConfig {
            enabled: true,
            compression_ratio: 8,
            min_magnitude: 1e-5,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: LogQuantizedConfig = serde_json::from_str(&json).unwrap();
        assert!(restored.enabled);
        assert_eq!(restored.compression_ratio, 8);
    }

    #[test]
    fn simd_distance_serde_roundtrip() {
        let cfg = SimdDistanceConfig {
            enabled: true,
            pad_to_power_of_two: true,
            metric: SimdDistanceMetric::L2,
            lane_width: Some(256),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: SimdDistanceConfig = serde_json::from_str(&json).unwrap();
        assert!(restored.is_available());
        assert_eq!(restored.metric, SimdDistanceMetric::L2);
    }

    #[test]
    fn next_pow2_basic() {
        assert_eq!(next_pow2(1), 1);
        assert_eq!(next_pow2(3), 4);
        assert_eq!(next_pow2(8), 8);
        assert_eq!(next_pow2(9), 16);
    }
}
