//! The `VoiceAnalysis` record — one versioned, `tier:"voice"` per-utterance
//! decomposition (Voice Phase 1, Wave 1 §W1.2).
//!
//! This is the **complete** decomposition contract: every field is present
//! with an honest confidence flag. A field may be present-but-coarse (DSP
//! arousal, a `0.0` dominance without a SER model), but the *structure* is
//! frozen so a later SER model or incremental decoder refines **values**
//! without reshaping storage or the wire. The producer lives in the voice
//! session (`TalkModeController`); this module is the pure, serde-serializable
//! record it hands off at the observer boundary (`ConversationEvent::UserTurn`)
//! for the daemon to carry over `agent.turn.record`.
//!
//! The JSON shape is flat-keyed for verbatim `conversation.graph` serving —
//! it deserializes to the schema in the plan byte-for-byte. Keys are FROZEN;
//! only values evolve.
//!
//! Pure data + the honesty enums — no model, no audio dependency; always
//! compiled and unit-tested.

use serde::{Deserialize, Serialize};

/// Schema version stamped on every record (`v`). Bump only on a
/// **structural** change; value-quality upgrades (a real SER model,
/// incremental partials) keep `v` at its current value.
pub const VOICE_ANALYSIS_VERSION: u32 = 1;

/// The `tier` discriminator: this is the voice-modality decomposition, which
/// overrides the compact cross-modality `classification` emotion axis.
pub const VOICE_TIER: &str = "voice";

/// Honest confidence flag on a coarse field. DSP arousal is `Medium`; a
/// DSP valence guess is `Low`; a real model lifts these without a reshape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// Provisional — read as a lean, not a measurement (e.g. DSP valence).
    #[default]
    Low,
    /// Robustly derivable from the available signal (e.g. DSP arousal).
    Medium,
    /// High-confidence (e.g. a real SER model's categorical label).
    High,
}

/// Which STT path produced the transcript. The native in-process TDT path
/// grows per-token data; the substrate HTTP path is honestly text-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SttPath {
    /// In-process parakeet TDT — full per-token timing/duration/confidence.
    Native,
    /// Substrate whisper/parakeet HTTP — `{"text":…}` only, `tokens: []`.
    Substrate,
}

/// What speaker attribution did for this utterance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeakerAction {
    /// Matched an enrolled speaker at/above the cosine threshold.
    Identified,
    /// No match — a fresh per-speaker node was enrolled.
    Enrolled,
    /// No embedder / attribution unavailable this turn.
    Unknown,
}

/// Where the emotion axis came from. The `source` badge on the surface reads
/// this so the user knows DSP-arousal from a real model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmotionSource {
    /// DSP prosody extractor (arousal always-present floor).
    ProsodyDsp,
    /// A speech-emotion ONNX model (refines valence + label, maybe dominance).
    SerOnnx,
}

/// Coarse paralinguistic class — the groundwork Wave 3's DuplexChannel
/// `Backchannel` verdict consumes (ADR-062 D5 / ADR-068 D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParalinguisticClass {
    /// Ordinary lexical speech.
    Speech,
    /// Short non-lexical vocalization likely to be a backchannel ("mm-hmm").
    BackchannelCandidate,
    /// Energy-burst non-lexical vocalization likely to be laughter.
    LaughterCandidate,
    /// A filled pause ("um", "uh") standing alone.
    Filler,
    /// Voiced but unclassifiable.
    Unknown,
}

/// One decoded token with its acoustic timing (native TDT path only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenAnalysis {
    /// SentencePiece surface text (word-start `▁` already turned into a space).
    pub text: String,
    /// Onset in ms from utterance start (encoder frame × subsampling stride).
    pub t_ms: u32,
    /// Duration in ms (TDT `skip` × subsampling stride).
    pub dur_ms: u32,
    /// Softmax confidence of the emitted token over the token logits.
    pub conf: f32,
}

/// STT sub-record: model, latency, path, and per-token data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SttAnalysis {
    /// Model wire name (e.g. `parakeet-tdt-0.6b`).
    pub model: String,
    /// Wall-clock transcription latency (ms).
    pub latency_ms: u64,
    /// Which path produced this.
    pub path: SttPath,
    /// Per-token timing/duration/confidence. Empty on the substrate path.
    pub tokens: Vec<TokenAnalysis>,
    /// Mean per-token confidence (`1.0` when no tokens).
    pub token_conf_mean: f32,
    /// Minimum per-token confidence (`1.0` when no tokens).
    pub token_conf_min: f32,
}

impl SttAnalysis {
    /// Build from a model name, path, latency, and the decoded tokens,
    /// computing the mean/min confidence summary.
    pub fn new(model: impl Into<String>, path: SttPath, latency_ms: u64, tokens: Vec<TokenAnalysis>) -> Self {
        let (mean, min) = if tokens.is_empty() {
            (1.0, 1.0)
        } else {
            let sum: f32 = tokens.iter().map(|t| t.conf).sum();
            let min = tokens.iter().map(|t| t.conf).fold(f32::INFINITY, f32::min);
            (sum / tokens.len() as f32, min)
        };
        Self {
            model: model.into(),
            latency_ms,
            path,
            tokens,
            token_conf_mean: mean,
            token_conf_min: min,
        }
    }
}

/// Endpoint sub-record: the smart-turn "related data" captured then normally
/// discarded after the finalize decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndpointAnalysis {
    /// Turn-completion probability that fired the finalize (or the last one
    /// computed before a max-silence ceiling finalize).
    pub completion_prob: f32,
    /// Which endpointer produced it: `smart-turn-v3` | `heuristic`.
    pub source: String,
    /// Trailing-silence length at finalize (ms).
    pub silence_tail_ms: u64,
    /// Onset→finalize wall clock (ms).
    pub latency_ms: u64,
}

/// Speaker sub-record: the ECAPA cosine attribution, normally dropped before
/// the `UserTurn`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerAnalysis {
    /// Matched/enrolled speaker id (`None` when unattributed).
    pub id: Option<String>,
    /// Speaker name (private context; `None` when unattributed).
    pub name: Option<String>,
    /// Cosine similarity to the matched centroid (`0.0` when unattributed).
    pub score: f32,
    /// Acceptance threshold in force.
    pub threshold: f32,
    /// What attribution did.
    pub action: SpeakerAction,
    /// d-vector dimensionality (ECAPA = 192; `0` when no embedder).
    pub embedding_dim: u32,
}

impl Default for SpeakerAnalysis {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            score: 0.0,
            threshold: 0.0,
            action: SpeakerAction::Unknown,
            embedding_dim: 0,
        }
    }
}

/// Audio/capture-health sub-record — all derivable from the raw i16 frames,
/// the VAD counters, and the noise-floor tracker at ~zero cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioAnalysis {
    /// `[start,end]` utterance length (ms).
    pub duration_ms: u64,
    /// Voiced-sample duration (ms).
    pub voiced_ms: u64,
    /// Silence within the utterance (ms).
    pub silence_ms: u64,
    /// Mean RMS over the utterance (dBFS).
    pub rms_dbfs_mean: f32,
    /// Peak per-frame RMS (dBFS).
    pub rms_dbfs_peak: f32,
    /// Tracked room-tone floor (dBFS).
    pub noise_floor_dbfs: f32,
    /// `rms_dbfs_mean − noise_floor_dbfs`.
    pub snr_db: f32,
    /// Fraction of samples at/near i16 full-scale (%).
    pub clip_pct: f32,
    /// Mean sample offset (DC bias) in i16 units.
    pub dc_offset: i32,
    /// Whether the noise-floor tracker had converged when this turn finalized.
    /// `false` on an early (typically first) turn of a speech-first session —
    /// the floor hasn't seen real silence yet, so `snr_db` is unreliable and
    /// should be read as provisional. The honesty flag for §W1.2 riskiest-call
    /// #3 applied to capture health.
    #[serde(default)]
    pub noise_floor_converged: bool,
    /// Frames the real-time capture thread dropped (channel full) up to this
    /// turn — cumulative for the session. Non-zero means the consume loop fell
    /// behind and audio was lost mid-stream (rate/token-conf become unreliable).
    #[serde(default)]
    pub dropped_frames: u64,
    /// High-water mark of frames queued in the capture→consume channel this
    /// session — the early warning that precedes a drop.
    #[serde(default)]
    pub channel_peak: u64,
}

impl Default for AudioAnalysis {
    fn default() -> Self {
        Self {
            duration_ms: 0,
            voiced_ms: 0,
            silence_ms: 0,
            rms_dbfs_mean: -100.0,
            rms_dbfs_peak: -100.0,
            noise_floor_dbfs: -100.0,
            snr_db: 0.0,
            clip_pct: 0.0,
            dc_offset: 0,
            noise_floor_converged: false,
            dropped_frames: 0,
            channel_peak: 0,
        }
    }
}

/// Prosody sub-record — DSP over the utterance (no model). `f0_*` are `0.0`
/// when no voiced pitch was found; `confidence` carries the honesty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProsodyAnalysis {
    /// Mean pitch over voiced frames (Hz; `0.0` if unvoiced).
    pub f0_mean_hz: f32,
    /// Minimum voiced pitch (Hz).
    pub f0_min_hz: f32,
    /// Maximum voiced pitch (Hz).
    pub f0_max_hz: f32,
    /// Pitch range in semitones (`12·log2(max/min)`).
    pub f0_range_semitones: f32,
    /// Normalized pitch slope over the utterance (rise/fall trend).
    pub f0_slope: f32,
    /// Speaking rate (tokens per voiced second).
    pub rate_tokens_per_s: f32,
    /// Intra-utterance pause count.
    pub pause_count: u32,
    /// Mean pause length (ms).
    pub pause_mean_ms: u32,
    /// Energy envelope range (dB).
    pub energy_dynamics_db: f32,
    /// Overall prosody confidence (`Medium` with voiced pitch, else `Low`).
    pub confidence: Confidence,
}

impl Default for ProsodyAnalysis {
    fn default() -> Self {
        Self {
            f0_mean_hz: 0.0,
            f0_min_hz: 0.0,
            f0_max_hz: 0.0,
            f0_range_semitones: 0.0,
            f0_slope: 0.0,
            rate_tokens_per_s: 0.0,
            pause_count: 0,
            pause_mean_ms: 0,
            energy_dynamics_db: 0.0,
            confidence: Confidence::Low,
        }
    }
}

/// Emotion sub-record — the VAD (valence/arousal/dominance) axis + a coarse
/// label. DSP is honest at `Medium` arousal / `Low` valence / `0.0`
/// dominance; a SER model overrides valence + label (and dominance if it
/// emits it) while DSP arousal stays the always-present floor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmotionAnalysis {
    /// Valence in `[-1, 1]` (negative→positive). DSP: a lean, `Low` conf.
    pub valence: f32,
    /// Arousal in `[0, 1]` (calm→activated). DSP: robust, `Medium` conf.
    pub arousal: f32,
    /// Dominance in `[0, 1]`. `0.0` without a model (not DSP-inferable).
    pub dominance: f32,
    /// Coarse categorical label (prosody-derived; a SER model refines it).
    pub label: String,
    /// Confidence on `arousal`.
    pub arousal_conf: Confidence,
    /// Confidence on `valence`.
    pub valence_conf: Confidence,
    /// Which producer set the axis.
    pub source: EmotionSource,
}

impl Default for EmotionAnalysis {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.0,
            dominance: 0.0,
            label: "neutral".into(),
            arousal_conf: Confidence::Low,
            valence_conf: Confidence::Low,
            source: EmotionSource::ProsodyDsp,
        }
    }
}

/// Paralinguistics sub-record — the coarse non-lexical classifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParalinguisticsAnalysis {
    /// Whether the utterance is non-lexical (voiced but ~empty transcript).
    pub non_lexical: bool,
    /// Coarse class.
    pub class: ParalinguisticClass,
    /// Confidence (coarse rule-based → `Low`).
    pub confidence: Confidence,
}

impl Default for ParalinguisticsAnalysis {
    fn default() -> Self {
        Self {
            non_lexical: false,
            class: ParalinguisticClass::Speech,
            confidence: Confidence::Low,
        }
    }
}

/// The complete per-utterance voice decomposition (§W1.2). Every section is
/// present; coarse fields carry their honesty in the confidence flags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceAnalysis {
    /// Schema version.
    pub v: u32,
    /// Modality tier (`"voice"`).
    pub tier: String,
    /// Recognition.
    pub stt: SttAnalysis,
    /// Endpoint related-data.
    pub endpoint: EndpointAnalysis,
    /// Speaker attribution.
    pub speaker: SpeakerAnalysis,
    /// Acoustics + capture health.
    pub audio: AudioAnalysis,
    /// Prosody.
    pub prosody: ProsodyAnalysis,
    /// Emotion (VAD + label).
    pub emotion: EmotionAnalysis,
    /// Paralinguistics.
    pub paralinguistics: ParalinguisticsAnalysis,
}

impl VoiceAnalysis {
    /// Assemble a record from its produced sections, stamping the version and
    /// tier. The controller (`TalkModeController::handle_turn`) fills each
    /// section from the extractors and calls this at the observer boundary.
    pub fn new(
        stt: SttAnalysis,
        endpoint: EndpointAnalysis,
        speaker: SpeakerAnalysis,
        audio: AudioAnalysis,
        prosody: ProsodyAnalysis,
        emotion: EmotionAnalysis,
        paralinguistics: ParalinguisticsAnalysis,
    ) -> Self {
        Self {
            v: VOICE_ANALYSIS_VERSION,
            tier: VOICE_TIER.to_string(),
            stt,
            endpoint,
            speaker,
            audio,
            prosody,
            emotion,
            paralinguistics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VoiceAnalysis {
        VoiceAnalysis::new(
            SttAnalysis::new(
                "parakeet-tdt-0.6b",
                SttPath::Native,
                48,
                vec![TokenAnalysis {
                    text: "hello".into(),
                    t_ms: 120,
                    dur_ms: 240,
                    conf: 0.98,
                }],
            ),
            EndpointAnalysis {
                completion_prob: 0.86,
                source: "smart-turn-v3".into(),
                silence_tail_ms: 320,
                latency_ms: 410,
            },
            SpeakerAnalysis {
                id: Some("spk_3".into()),
                name: Some("Mathew".into()),
                score: 0.72,
                threshold: 0.45,
                action: SpeakerAction::Identified,
                embedding_dim: 192,
            },
            AudioAnalysis {
                duration_ms: 1830,
                voiced_ms: 1510,
                silence_ms: 320,
                rms_dbfs_mean: -28.4,
                rms_dbfs_peak: -12.1,
                noise_floor_dbfs: -52.0,
                snr_db: 23.6,
                clip_pct: 0.0,
                dc_offset: 3,
                noise_floor_converged: true,
                dropped_frames: 0,
                channel_peak: 0,
            },
            ProsodyAnalysis {
                f0_mean_hz: 165.0,
                f0_min_hz: 110.0,
                f0_max_hz: 240.0,
                f0_range_semitones: 13.4,
                f0_slope: -0.8,
                rate_tokens_per_s: 4.2,
                pause_count: 2,
                pause_mean_ms: 180,
                energy_dynamics_db: 18.0,
                confidence: Confidence::Medium,
            },
            EmotionAnalysis {
                valence: 0.1,
                arousal: 0.63,
                dominance: 0.0,
                label: "agitated".into(),
                arousal_conf: Confidence::Medium,
                valence_conf: Confidence::Low,
                source: EmotionSource::ProsodyDsp,
            },
            ParalinguisticsAnalysis {
                non_lexical: false,
                class: ParalinguisticClass::Speech,
                confidence: Confidence::Low,
            },
        )
    }

    #[test]
    fn version_and_tier_stamped() {
        let a = sample();
        assert_eq!(a.v, 1);
        assert_eq!(a.tier, "voice");
    }

    #[test]
    fn serializes_to_frozen_schema_keys() {
        let v = serde_json::to_value(sample()).unwrap();
        // Top-level sections present.
        for key in [
            "v",
            "tier",
            "stt",
            "endpoint",
            "speaker",
            "audio",
            "prosody",
            "emotion",
            "paralinguistics",
        ] {
            assert!(v.get(key).is_some(), "missing top-level key {key}");
        }
        // Honesty enums serialize to the exact wire strings the plan freezes.
        assert_eq!(v["stt"]["path"], "native");
        assert_eq!(v["speaker"]["action"], "identified");
        assert_eq!(v["prosody"]["confidence"], "medium");
        assert_eq!(v["emotion"]["valence_conf"], "low");
        assert_eq!(v["emotion"]["source"], "prosody-dsp");
        assert_eq!(v["paralinguistics"]["class"], "speech");
        // Per-token shape.
        assert_eq!(v["stt"]["tokens"][0]["text"], "hello");
        assert_eq!(v["stt"]["tokens"][0]["t_ms"], 120);
    }

    #[test]
    fn round_trips_through_json() {
        let a = sample();
        let json = serde_json::to_string(&a).unwrap();
        let back: VoiceAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn kebab_and_snake_enum_wire_strings() {
        assert_eq!(
            serde_json::to_value(EmotionSource::SerOnnx).unwrap(),
            "ser-onnx"
        );
        assert_eq!(
            serde_json::to_value(ParalinguisticClass::BackchannelCandidate).unwrap(),
            "backchannel_candidate"
        );
        assert_eq!(
            serde_json::to_value(ParalinguisticClass::LaughterCandidate).unwrap(),
            "laughter_candidate"
        );
    }

    #[test]
    fn stt_conf_summary_defaults_to_one_when_empty() {
        let s = SttAnalysis::new("parakeet-tdt-0.6b", SttPath::Substrate, 12, vec![]);
        assert_eq!(s.token_conf_mean, 1.0);
        assert_eq!(s.token_conf_min, 1.0);
        assert!(s.tokens.is_empty());
    }

    #[test]
    fn stt_conf_summary_computes_mean_and_min() {
        let s = SttAnalysis::new(
            "parakeet-tdt-0.6b",
            SttPath::Native,
            20,
            vec![
                TokenAnalysis { text: "a".into(), t_ms: 0, dur_ms: 40, conf: 0.9 },
                TokenAnalysis { text: "b".into(), t_ms: 40, dur_ms: 40, conf: 0.7 },
            ],
        );
        assert!((s.token_conf_mean - 0.8).abs() < 1e-6);
        assert!((s.token_conf_min - 0.7).abs() < 1e-6);
    }
}
