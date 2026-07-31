//! Pure peak / RMS metering for mic samples.
//!
//! Unit-testable without a real device: feed synthetic PCM and assert levels.

/// Peak absolute amplitude of mono `f32` samples in `[-1.0, 1.0]`, clamped to `[0, 1]`.
pub fn peak_level_f32(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut peak = 0.0f32;
    for &s in samples {
        let a = s.abs();
        if a > peak {
            peak = a;
        }
    }
    peak.clamp(0.0, 1.0)
}

/// Peak absolute amplitude of mono `i16` samples, normalized to `[0, 1]`.
#[allow(dead_code)] // public metering helper; used by unit tests + future poll paths
pub fn peak_level_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut peak = 0i16;
    for &s in samples {
        let a = s.saturating_abs();
        if a > peak {
            peak = a;
        }
    }
    (f32::from(peak) / 32767.0).clamp(0.0, 1.0)
}

/// RMS level of mono `f32` samples, clamped to `[0, 1]`.
#[allow(dead_code)] // public metering helper; covered by unit tests
pub fn rms_level_f32(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}

/// Downmix interleaved multi-channel `f32` frames to mono (mean of channels).
pub fn downmix_f32(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels == 0 {
        return Vec::new();
    }
    if channels == 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}

/// Convert mono `f32` in `[-1, 1]` to mono `i16`.
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32_767.0) as i16)
        .collect()
}

/// Convert mono `i16` to mono `f32` in approximately `[-1, 1]`.
#[allow(dead_code)] // public conversion helper; covered by unit tests
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| f32::from(s) / 32_768.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_empty_is_zero() {
        assert_eq!(peak_level_f32(&[]), 0.0);
        assert_eq!(peak_level_i16(&[]), 0.0);
        assert_eq!(rms_level_f32(&[]), 0.0);
    }

    #[test]
    fn peak_silence() {
        assert_eq!(peak_level_f32(&[0.0, 0.0, 0.0]), 0.0);
        assert_eq!(peak_level_i16(&[0, 0, 0]), 0.0);
    }

    #[test]
    fn peak_f32_full_scale() {
        assert!((peak_level_f32(&[0.0, 0.5, -1.0, 0.25]) - 1.0).abs() < 1e-6);
        assert!((peak_level_f32(&[0.25, -0.4]) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn peak_i16_half_scale() {
        // 16384 / 32767 ≈ 0.5
        let level = peak_level_i16(&[0, 16_384, -100]);
        assert!((level - 0.5).abs() < 0.01);
    }

    #[test]
    fn peak_clamps_above_one() {
        // Pathological input outside [-1,1] still reports ≤ 1.
        assert_eq!(peak_level_f32(&[2.0, -3.0]), 1.0);
    }

    #[test]
    fn rms_known_value() {
        // RMS of [1, -1] is 1.
        assert!((rms_level_f32(&[1.0, -1.0]) - 1.0).abs() < 1e-6);
        // RMS of silence is 0.
        assert_eq!(rms_level_f32(&[0.0, 0.0]), 0.0);
    }

    #[test]
    fn downmix_stereo_mean() {
        // L=1.0, R=0.0 → mono 0.5
        let mono = downmix_f32(&[1.0, 0.0, 0.0, 1.0], 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.5).abs() < 1e-6);
        assert!((mono[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn downmix_mono_identity() {
        let mono = downmix_f32(&[0.1, 0.2], 1);
        assert_eq!(mono, vec![0.1, 0.2]);
    }

    #[test]
    fn downmix_zero_channels() {
        assert!(downmix_f32(&[1.0], 0).is_empty());
    }

    #[test]
    fn f32_i16_roundtrip_near() {
        let orig = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let i16s = f32_to_i16(&orig);
        let back = i16_to_f32(&i16s);
        for (a, b) in orig.iter().zip(back.iter()) {
            assert!((a - b).abs() < 0.001, "{a} vs {b}");
        }
    }
}
