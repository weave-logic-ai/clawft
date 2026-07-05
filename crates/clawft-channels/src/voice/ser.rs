//! Speech-emotion recognition (SER) seam (Voice Phase 1, Wave 1 §W1.2).
//!
//! Wave 1 **wires the seam and ships the DSP producer**; it does not stage a
//! model. A real SER model (a wav2vec2/HuBERT-SER int8 export, or a small
//! prosody→emotion MLP) drops into the **exact smart-turn / ECAPA pattern**:
//! it lives in `clawft-voice-onnx`, discovers `~/.weftos/models/ser/…` with a
//! `WEFTOS_SER_MODEL` override, its construction never fails, and it degrades
//! gracefully to the DSP prosody extractor when absent — identical to
//! [`SmartTurnEndpoint`](../../../../clawft_voice_onnx/smart_turn) falling back
//! to the heuristic.
//!
//! This module is the **channels-side trait** ([`SerModel`]) that such a model
//! satisfies, plus [`DspSer`] — the always-present degraded stand-in that
//! emits no prediction, so the DSP arousal floor stands. When a model is
//! present it **overrides `valence` + `label`** (and `dominance` if it emits
//! it) while **DSP arousal stays the always-present floor**, and the record's
//! `emotion.source` flips to `ser-onnx`. [`refine_emotion`] is the merge point.

use super::analysis::{Confidence, EmotionAnalysis, EmotionSource};

/// What a SER model predicts. Every axis is optional: a model may emit only a
/// categorical label, or only valence, etc. Absent axes leave the DSP value in
/// place (arousal is never overridden — it is the DSP floor).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SerPrediction {
    /// Valence in `[-1, 1]` (overrides the DSP lean when present).
    pub valence: Option<f32>,
    /// Dominance in `[0, 1]` (fills the `0.0` DSP placeholder when present).
    pub dominance: Option<f32>,
    /// Categorical label (overrides the coarse DSP label when present).
    pub label: Option<String>,
    /// Confidence the model assigns to `valence`/`label`.
    pub confidence: Confidence,
}

/// A speech-emotion model over a finalized utterance. Implemented by a future
/// ONNX model in `clawft-voice-onnx`; the trait lives here so the record
/// producer stays model-free and the seam is testable with a mock.
pub trait SerModel: Send + Sync {
    /// Predict emotion for 16 kHz mono `s16le` PCM, or `None` to defer to the
    /// DSP floor (model unavailable, low confidence, or out-of-domain input).
    fn predict(&self, samples: &[i16], sample_rate: u32) -> Option<SerPrediction>;

    /// Whether a real model is loaded (vs the DSP-degraded stand-in). Mirrors
    /// `is_runtime_available()` on the other native models.
    fn is_available(&self) -> bool {
        false
    }
}

/// The always-present degraded stand-in: no model, no prediction — the DSP
/// prosody emotion stands. This is what the controller holds until a real SER
/// ONNX is staged, so the seam is wired and the surface never reshapes.
#[derive(Debug, Default, Clone, Copy)]
pub struct DspSer;

impl SerModel for DspSer {
    fn predict(&self, _samples: &[i16], _sample_rate: u32) -> Option<SerPrediction> {
        None
    }
}

/// Merge a SER prediction onto the DSP emotion floor: **arousal always stays
/// the DSP value** (the honest, always-present signal); a present prediction
/// overrides `valence` + `label` (and `dominance` if it emits it) and flips
/// `source` to `ser-onnx` with the model's confidence. `None` leaves the DSP
/// emotion untouched.
pub fn refine_emotion(dsp: EmotionAnalysis, prediction: Option<SerPrediction>) -> EmotionAnalysis {
    let Some(p) = prediction else {
        return dsp;
    };
    let mut out = dsp;
    if let Some(v) = p.valence {
        out.valence = v.clamp(-1.0, 1.0);
        out.valence_conf = p.confidence;
    }
    if let Some(d) = p.dominance {
        out.dominance = d.clamp(0.0, 1.0);
    }
    if let Some(label) = p.label {
        out.label = label;
    }
    out.source = EmotionSource::SerOnnx;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dsp_floor() -> EmotionAnalysis {
        EmotionAnalysis {
            valence: 0.05,
            arousal: 0.62,
            dominance: 0.0,
            label: "activated".into(),
            arousal_conf: Confidence::Medium,
            valence_conf: Confidence::Low,
            source: EmotionSource::ProsodyDsp,
        }
    }

    #[test]
    fn dsp_ser_defers_to_floor() {
        assert!(DspSer.predict(&[0i16; 1_600], 16_000).is_none());
        assert!(!DspSer.is_available());
        // Absent prediction leaves the DSP emotion exactly as-is.
        let floor = dsp_floor();
        let out = refine_emotion(floor.clone(), DspSer.predict(&[], 16_000));
        assert_eq!(out, floor);
        assert_eq!(out.source, EmotionSource::ProsodyDsp);
    }

    #[test]
    fn present_model_overrides_valence_label_keeps_arousal() {
        let floor = dsp_floor();
        let pred = SerPrediction {
            valence: Some(-0.7),
            dominance: Some(0.4),
            label: Some("angry".into()),
            confidence: Confidence::High,
        };
        let out = refine_emotion(floor.clone(), Some(pred));
        // Arousal is the DSP floor — never overridden.
        assert_eq!(out.arousal, floor.arousal);
        assert_eq!(out.arousal_conf, Confidence::Medium);
        // Valence, dominance, label, and source come from the model.
        assert!((out.valence + 0.7).abs() < 1e-6);
        assert_eq!(out.valence_conf, Confidence::High);
        assert!((out.dominance - 0.4).abs() < 1e-6);
        assert_eq!(out.label, "angry");
        assert_eq!(out.source, EmotionSource::SerOnnx);
    }

    #[test]
    fn partial_prediction_only_touches_present_axes() {
        // A label-only model: valence/dominance stay DSP, but source flips.
        let floor = dsp_floor();
        let pred = SerPrediction {
            label: Some("sad".into()),
            confidence: Confidence::Medium,
            ..SerPrediction::default()
        };
        let out = refine_emotion(floor.clone(), Some(pred));
        assert_eq!(out.valence, floor.valence);
        assert_eq!(out.dominance, floor.dominance);
        assert_eq!(out.label, "sad");
        assert_eq!(out.source, EmotionSource::SerOnnx);
    }
}
