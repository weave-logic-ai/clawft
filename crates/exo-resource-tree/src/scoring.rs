//! Node scoring vectors for trust/performance tracking.
//!
//! Every `ResourceNode` carries a 6-dimensional scoring vector (24 bytes)
//! that flows into the Merkle hash, making scores tamper-evident.
//!
//! Dimensions: trust, performance, difficulty, reward, reliability, velocity.

use serde::{Deserialize, Serialize};

/// Number of scoring dimensions.
pub const SCORING_DIMS: usize = 6;

/// 6-dimensional scoring vector embedded on every `ResourceNode`.
///
/// Each dimension is a normalized f32 in \[0.0, 1.0\]. Default is 0.5
/// (neutral) on all axes. The vector is 24 bytes and designed for fast
/// comparison via cosine similarity or L2 distance.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NodeScoring {
    /// Trustworthiness of outcomes produced by this node.
    pub trust: f32,
    /// Speed/efficiency — actual vs estimated completion.
    pub performance: f32,
    /// Task complexity rating.
    pub difficulty: f32,
    /// Value/importance weighting.
    pub reward: f32,
    /// Consistency of results over time.
    pub reliability: f32,
    /// Throughput / rate of progress.
    pub velocity: f32,
}

impl Default for NodeScoring {
    fn default() -> Self {
        Self {
            trust: 0.5,
            performance: 0.5,
            difficulty: 0.5,
            reward: 0.5,
            reliability: 0.5,
            velocity: 0.5,
        }
    }
}

impl NodeScoring {
    /// Create a scoring vector from explicit values.
    pub fn new(
        trust: f32,
        performance: f32,
        difficulty: f32,
        reward: f32,
        reliability: f32,
        velocity: f32,
    ) -> Self {
        Self {
            trust,
            performance,
            difficulty,
            reward,
            reliability,
            velocity,
        }
    }

    /// Deterministic byte representation for Merkle hashing (24 bytes).
    ///
    /// Each f32 is little-endian. NaN values are canonicalized to 0.0.
    pub fn to_hash_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        let arr = self.as_array();
        for (i, &val) in arr.iter().enumerate() {
            let clean = if val.is_nan() { 0.0f32 } else { val };
            buf[i * 4..(i + 1) * 4].copy_from_slice(&clean.to_le_bytes());
        }
        buf
    }

    /// Exponential moving average update.
    ///
    /// `self = self * (1 - alpha) + observation * alpha`
    ///
    /// Alpha is clamped to \[0.0, 1.0\].
    pub fn blend(&mut self, observation: &NodeScoring, alpha: f32) {
        let a = alpha.clamp(0.0, 1.0);
        let inv = 1.0 - a;
        self.trust = self.trust * inv + observation.trust * a;
        self.performance = self.performance * inv + observation.performance * a;
        self.difficulty = self.difficulty * inv + observation.difficulty * a;
        self.reward = self.reward * inv + observation.reward * a;
        self.reliability = self.reliability * inv + observation.reliability * a;
        self.velocity = self.velocity * inv + observation.velocity * a;
    }

    /// Reward-weighted mean of child scoring vectors.
    ///
    /// Each child's contribution is weighted by its `reward` dimension.
    /// If all rewards are zero, falls back to uniform mean.
    pub fn aggregate(children: &[&NodeScoring]) -> NodeScoring {
        if children.is_empty() {
            return NodeScoring::default();
        }

        let total_weight: f32 = children.iter().map(|c| c.reward).sum();
        if total_weight <= f32::EPSILON {
            // Uniform mean fallback
            let n = children.len() as f32;
            return NodeScoring {
                trust: children.iter().map(|c| c.trust).sum::<f32>() / n,
                performance: children.iter().map(|c| c.performance).sum::<f32>() / n,
                difficulty: children.iter().map(|c| c.difficulty).sum::<f32>() / n,
                reward: children.iter().map(|c| c.reward).sum::<f32>() / n,
                reliability: children.iter().map(|c| c.reliability).sum::<f32>() / n,
                velocity: children.iter().map(|c| c.velocity).sum::<f32>() / n,
            };
        }

        let mut result = NodeScoring::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for child in children {
            let w = child.reward / total_weight;
            result.trust += child.trust * w;
            result.performance += child.performance * w;
            result.difficulty += child.difficulty * w;
            result.reward += child.reward * w;
            result.reliability += child.reliability * w;
            result.velocity += child.velocity * w;
        }
        result
    }

    /// Cosine similarity between two scoring vectors.
    ///
    /// Returns a value in \[-1.0, 1.0\]. Identical vectors return 1.0.
    /// Zero-magnitude vectors return 0.0.
    pub fn cosine_similarity(&self, other: &NodeScoring) -> f32 {
        let a = self.as_array();
        let b = other.as_array();

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a = self.magnitude();
        let mag_b = other.magnitude();

        if mag_a < f32::EPSILON || mag_b < f32::EPSILON {
            return 0.0;
        }
        dot / (mag_a * mag_b)
    }

    /// Euclidean (L2) distance between two scoring vectors.
    pub fn l2_distance(&self, other: &NodeScoring) -> f32 {
        let a = self.as_array();
        let b = other.as_array();
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f32>()
            .sqrt()
    }

    /// Euclidean magnitude of the scoring vector.
    pub fn magnitude(&self) -> f32 {
        let a = self.as_array();
        a.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    /// Weighted linear combination of dimensions.
    ///
    /// `weights` has the same ordering as `as_array()`:
    /// \[trust, performance, difficulty, reward, reliability, velocity\].
    pub fn weighted_score(&self, weights: &[f32; SCORING_DIMS]) -> f32 {
        let a = self.as_array();
        a.iter().zip(weights.iter()).map(|(v, w)| v * w).sum()
    }

    /// Pareto dominance: `self` dominates `other` iff every dimension of
    /// `self` is >= `other` and at least one is strictly greater.
    pub fn dominates(&self, other: &NodeScoring) -> bool {
        let a = self.as_array();
        let b = other.as_array();
        let all_geq = a.iter().zip(b.iter()).all(|(x, y)| x >= y);
        let any_gt = a.iter().zip(b.iter()).any(|(x, y)| x > y);
        all_geq && any_gt
    }

    /// Convert to a fixed-size array.
    ///
    /// Order: \[trust, performance, difficulty, reward, reliability, velocity\].
    pub fn as_array(&self) -> [f32; SCORING_DIMS] {
        [
            self.trust,
            self.performance,
            self.difficulty,
            self.reward,
            self.reliability,
            self.velocity,
        ]
    }

    /// Create from a fixed-size array.
    ///
    /// Order: \[trust, performance, difficulty, reward, reliability, velocity\].
    pub fn from_array(arr: [f32; SCORING_DIMS]) -> Self {
        Self {
            trust: arr[0],
            performance: arr[1],
            difficulty: arr[2],
            reward: arr[3],
            reliability: arr[4],
            velocity: arr[5],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_neutral() {
        let s = NodeScoring::default();
        for &v in &s.as_array() {
            assert!((v - 0.5).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn to_hash_bytes_deterministic() {
        let s = NodeScoring::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6);
        let b1 = s.to_hash_bytes();
        let b2 = s.to_hash_bytes();
        assert_eq!(b1, b2);
        assert_ne!(b1, [0u8; 24]);
    }

    #[test]
    fn to_hash_bytes_nan_canonicalization() {
        let s = NodeScoring::new(f32::NAN, 0.5, 0.5, 0.5, 0.5, 0.5);
        let bytes = s.to_hash_bytes();
        // First 4 bytes should be 0.0f32 LE
        let first_f32 = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert!((first_f32 - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn blend_alpha_zero_preserves_original() {
        let mut s = NodeScoring::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6);
        let original = s;
        let obs = NodeScoring::new(0.9, 0.8, 0.7, 0.6, 0.5, 0.4);
        s.blend(&obs, 0.0);
        for (a, b) in s.as_array().iter().zip(original.as_array().iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn blend_alpha_one_replaces_with_observation() {
        let mut s = NodeScoring::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6);
        let obs = NodeScoring::new(0.9, 0.8, 0.7, 0.6, 0.5, 0.4);
        s.blend(&obs, 1.0);
        for (a, b) in s.as_array().iter().zip(obs.as_array().iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn blend_alpha_half() {
        let mut s = NodeScoring::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let obs = NodeScoring::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        s.blend(&obs, 0.5);
        for &v in &s.as_array() {
            assert!((v - 0.5).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn aggregate_reward_weighted() {
        let a = NodeScoring::new(1.0, 1.0, 1.0, 0.8, 1.0, 1.0);
        let b = NodeScoring::new(0.0, 0.0, 0.0, 0.2, 0.0, 0.0);
        let result = NodeScoring::aggregate(&[&a, &b]);
        // a has reward=0.8, b has reward=0.2, total=1.0
        // trust = 1.0*0.8 + 0.0*0.2 = 0.8
        assert!((result.trust - 0.8).abs() < 1e-6);
        assert!((result.performance - 0.8).abs() < 1e-6);
    }

    #[test]
    fn aggregate_empty_returns_default() {
        let result = NodeScoring::aggregate(&[]);
        let def = NodeScoring::default();
        assert_eq!(result.as_array(), def.as_array());
    }

    #[test]
    fn aggregate_zero_reward_falls_back_to_uniform() {
        let a = NodeScoring::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let b = NodeScoring::new(0.0, 1.0, 0.0, 0.0, 0.0, 0.0);
        let result = NodeScoring::aggregate(&[&a, &b]);
        assert!((result.trust - 0.5).abs() < 1e-6);
        assert!((result.performance - 0.5).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_identical() {
        let s = NodeScoring::new(0.3, 0.4, 0.5, 0.6, 0.7, 0.8);
        let sim = s.cosine_similarity(&s);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let zero = NodeScoring::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let s = NodeScoring::default();
        assert!((zero.cosine_similarity(&s) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn l2_distance_identical() {
        let s = NodeScoring::new(0.3, 0.4, 0.5, 0.6, 0.7, 0.8);
        let dist = s.l2_distance(&s);
        assert!(dist.abs() < 1e-6);
    }

    #[test]
    fn l2_distance_nonzero() {
        let a = NodeScoring::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let b = NodeScoring::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let dist = a.l2_distance(&b);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn magnitude_unit_vector() {
        let s = NodeScoring::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert!((s.magnitude() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn weighted_score() {
        let s = NodeScoring::new(1.0, 0.5, 0.0, 0.0, 0.0, 0.0);
        let weights = [2.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let score = s.weighted_score(&weights);
        assert!((score - 2.5).abs() < 1e-6);
    }

    #[test]
    fn dominates_strict() {
        let a = NodeScoring::new(0.8, 0.8, 0.8, 0.8, 0.8, 0.8);
        let b = NodeScoring::new(0.5, 0.5, 0.5, 0.5, 0.5, 0.5);
        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn dominates_equal_is_false() {
        let s = NodeScoring::default();
        assert!(!s.dominates(&s));
    }

    #[test]
    fn dominates_mixed_is_false() {
        let a = NodeScoring::new(0.9, 0.1, 0.5, 0.5, 0.5, 0.5);
        let b = NodeScoring::new(0.1, 0.9, 0.5, 0.5, 0.5, 0.5);
        assert!(!a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn from_array_roundtrip() {
        let arr = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let s = NodeScoring::from_array(arr);
        assert_eq!(s.as_array(), arr);
    }

    #[test]
    fn serde_roundtrip() {
        let s = NodeScoring::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6);
        let json = serde_json::to_string(&s).unwrap();
        let back: NodeScoring = serde_json::from_str(&json).unwrap();
        assert_eq!(s.as_array(), back.as_array());
    }
}
