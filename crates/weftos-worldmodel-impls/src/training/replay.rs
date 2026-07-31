//! Per-class importance-weighted replay buffer (WEFT-531 online surface).

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use weftos_worldmodel_core::{SensorClass, TrainingSample};

/// Default capacity for the streaming-merge replay ring.
pub const DEFAULT_REPLAY_CAPACITY: usize = 256;

/// Importance-weighted ring buffer of [`TrainingSample`]s.
///
/// Draws use roulette-wheel sampling proportional to
/// `max(importance, 0) * (1 + surprise)` so rare high-surprise events stay
/// resident longer in expectation (importance-weighted rare-event residency
/// from the LeWM training diagram).
#[derive(Debug, Clone)]
pub struct ImportanceReplayBuffer {
    buf: VecDeque<TrainingSample>,
    capacity: usize,
    /// LCG state for deterministic weighted draws (no_std RNG).
    rng: u64,
}

impl Default for ImportanceReplayBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_REPLAY_CAPACITY)
    }
}

impl ImportanceReplayBuffer {
    /// Construct with explicit capacity and default seed.
    pub fn new(capacity: usize) -> Self {
        Self::with_seed(capacity, 0xC0FF_EE42_u64)
    }

    /// Construct with capacity and RNG seed.
    pub fn with_seed(capacity: usize, seed: u64) -> Self {
        Self {
            buf: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
            rng: seed | 1, // ensure non-zero
        }
    }

    /// Number of samples currently buffered.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Push a sample, dropping the oldest when over capacity.
    pub fn push(&mut self, sample: TrainingSample) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(sample);
    }

    /// Effective draw weight for a sample.
    #[inline]
    pub fn weight(sample: &TrainingSample) -> f32 {
        let imp = if sample.importance > 0.0 {
            sample.importance
        } else {
            0.0
        };
        let surprise_boost = 1.0 + sample.surprise.max(0.0);
        imp * surprise_boost
    }

    /// Sum of weights for samples matching `class` (or all if `None`).
    pub fn total_weight(&self, class: Option<SensorClass>) -> f32 {
        self.buf
            .iter()
            .filter(|s| class.map(|c| s.sensor_class == c).unwrap_or(true))
            .map(Self::weight)
            .sum()
    }

    /// Count samples matching `class`.
    pub fn count_class(&self, class: SensorClass) -> usize {
        self.buf
            .iter()
            .filter(|s| s.sensor_class == class)
            .count()
    }

    /// Draw up to `n` samples with replacement, importance-weighted, filtered
    /// by optional `class`.
    ///
    /// Returns fewer than `n` only when the filtered set is empty.
    pub fn sample_batch(&mut self, n: usize, class: Option<SensorClass>) -> Vec<TrainingSample> {
        let indices: Vec<usize> = self
            .buf
            .iter()
            .enumerate()
            .filter(|(_, s)| class.map(|c| s.sensor_class == c).unwrap_or(true))
            .map(|(i, _)| i)
            .collect();
        if indices.is_empty() || n == 0 {
            return Vec::new();
        }

        let weights: Vec<f32> = indices
            .iter()
            .map(|&i| Self::weight(&self.buf[i]))
            .collect();
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            // Uniform fallback when all weights are zero.
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                let i = indices[self.next_usize() % indices.len()];
                out.push(self.buf[i]);
            }
            return out;
        }

        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let r = self.next_f32() * total;
            let mut acc = 0.0f32;
            let mut chosen = indices[indices.len() - 1];
            for (k, &idx) in indices.iter().enumerate() {
                acc += weights[k];
                if r <= acc {
                    chosen = idx;
                    break;
                }
            }
            out.push(self.buf[chosen]);
        }
        out
    }

    /// All samples currently buffered (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &TrainingSample> {
        self.buf.iter()
    }

    /// Samples for a single class (oldest first).
    pub fn iter_class(&self, class: SensorClass) -> impl Iterator<Item = &TrainingSample> {
        self.buf.iter().filter(move |s| s.sensor_class == class)
    }

    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes LCG
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.rng
    }

    fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    fn next_f32(&mut self) -> f32 {
        // [0, 1)
        let bits = (self.next_u64() >> 40) as u32; // 24 bits
        (bits as f32) / ((1u32 << 24) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use weftos_worldmodel_core::{zero_latent, SensorClass};

    fn sample(class: SensorClass, imp: f32, surprise: f32, tag: f32) -> TrainingSample {
        let mut z0 = zero_latent();
        let mut z1 = zero_latent();
        z0[0] = tag;
        z1[0] = tag + 1.0;
        let mut s = TrainingSample::basic(class, z0, z1, 0);
        s.importance = imp;
        s.surprise = surprise;
        s
    }

    #[test]
    fn ring_drops_oldest() {
        let mut b = ImportanceReplayBuffer::new(2);
        b.push(sample(SensorClass::Imu, 1.0, 0.0, 0.0));
        b.push(sample(SensorClass::Imu, 1.0, 0.0, 1.0));
        b.push(sample(SensorClass::Imu, 1.0, 0.0, 2.0));
        assert_eq!(b.len(), 2);
        let tags: Vec<f32> = b.iter().map(|s| s.z_t[0]).collect();
        assert_eq!(tags, vec![1.0, 2.0]);
    }

    #[test]
    fn high_importance_dominates_draws() {
        let mut b = ImportanceReplayBuffer::with_seed(16, 7);
        // One very important sample tagged 99, many weak ones tagged 0.
        b.push(sample(SensorClass::RgbD, 1000.0, 0.0, 99.0));
        for i in 0..10 {
            b.push(sample(SensorClass::RgbD, 0.001, 0.0, i as f32));
        }
        let batch = b.sample_batch(50, Some(SensorClass::RgbD));
        assert_eq!(batch.len(), 50);
        let hits = batch.iter().filter(|s| s.z_t[0] == 99.0).count();
        assert!(hits > 40, "expected rare-event residency, hits={hits}");
    }

    #[test]
    fn class_filter_excludes_other() {
        let mut b = ImportanceReplayBuffer::new(8);
        b.push(sample(SensorClass::Imu, 1.0, 0.0, 1.0));
        b.push(sample(SensorClass::Audio, 1.0, 0.0, 2.0));
        let batch = b.sample_batch(10, Some(SensorClass::Imu));
        assert!(batch.iter().all(|s| s.sensor_class == SensorClass::Imu));
        assert_eq!(b.count_class(SensorClass::Audio), 1);
    }
}
