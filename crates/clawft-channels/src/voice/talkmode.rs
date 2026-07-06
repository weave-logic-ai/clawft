//! Talk-Mode controller (ADR-061 §7 assembly) — the conversational loop.
//!
//! Assembles the Phase-6 building blocks over a full-duplex audio channel:
//!
//! ```text
//! capture ─▶ VAD + endpoint(6.5) ─▶ STT(6.2) ─▶ speaker-attribute(6.6)
//!         ─▶ grounded agent(6.3 policy → VoiceLlm) ─▶ fast ack(6.4)
//!         ─▶ streamed expressive answer(6.4) ;  barge-in ─▶ AEC flush + TTS cancel
//! ```
//!
//! **The speculative→committed ECC handoff** (the heart of this task): the fast
//! ack is emitted as a [`ConversationEvent::SpeculativeReply`] (a *Speculative*
//! spoken node); the grounded answer is a [`ConversationEvent::CommittedReply`]
//! (the *Committed* node that supersedes it); a barge-in emits
//! [`ConversationEvent::Interrupted`] (the in-flight reply is *Contradicted* /
//! pruned). This maps 1:1 onto the kernel ECC `NodeState` lifecycle
//! {Speculative, Frontier, Committed, Stale, Pruned} — the controller does NOT
//! build a parallel mechanism; it emits the lifecycle events and the bridge's
//! [`ConversationObserver`] writes them onto the real `CausalGraph`.
//!
//! The controller is provider/model/device-agnostic — every external system is
//! a trait (`SttBackend`, `VoiceLlm`, `TtsEngine`/`TtsSink`, `SpeakerEmbedder`,
//! `EndpointModel`, plus [`AudioControl`] and [`ConversationObserver`] defined
//! here). The concrete LocalProvider / WebRTC-AEC / kernel-ECC bindings live in
//! the `clawft-voice-talk` bridge crate, so this stays unit-testable with mocks
//! and never pulls clawft-llm / clawft-kernel / cpal into clawft-channels.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Notify};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::analysis::{
    AudioAnalysis, EndpointAnalysis, SpeakerAction, SpeakerAnalysis, SttAnalysis, SttPath,
    TokenAnalysis, VoiceAnalysis,
};
use super::capture::CaptureMetrics;
use super::paralinguistics::{classify_paralinguistics, ParalinguisticInput};
use super::policy::{VoiceAnswerPolicy, VoiceLlm};
use super::prosody::{
    analyze_prosody, capture_health, emotion_from_prosody_baseline, ProsodyInput, SessionBaseline,
};
use super::ser::{refine_emotion, DspSer, SerModel};
use super::speaker::{SpeakerEmbedder, SpeakerId, SpeakerRegistry};
use super::stt::{SttBackend, Utterance};
use super::tts::{DualLayerTts, TtsSink};
use super::turn::{EndpointModel, EndpointSnapshot, SemanticEndpointer, TurnDecision};
use super::vad::{EnergyVad, NoiseFloor};

/// How far above the tracked room-tone floor a frame must sit to count as
/// voice. 8 dB keeps conversational speech (typically 15–25 dB above room
/// tone) comfortably inside while HVAC/fan drift stays out.
const VAD_NOISE_MARGIN_DB: f32 = 8.0;

/// Minimum voiced audio (ms) for a finalized turn to reach STT. Shorter
/// captures are noise blips or playback reverb tails, not words.
const MIN_TURN_MS: usize = 250;

/// Barge-in margin over the tracked room floor while the bot is speaking.
/// Steeper than [`VAD_NOISE_MARGIN_DB`]: the AEC only partially cancels
/// speaker→mic echo (observed live: the bot barged in on its own ack 300 ms
/// after starting to speak, cancelling every answer), while a human
/// genuinely interrupting is much louder at the mic than echo residue.
const BARGE_IN_MARGIN_DB: f32 = 15.0;

/// Ignore barge-in candidates during the first moments of playback while
/// the echo canceller converges on the new render signal.
const BARGE_IN_GRACE_MS: u64 = 400;

/// Minimum spacing between live `CaptureLevel` events (~10 Hz). The capture
/// loop computes RMS + floor every frame, but the surface only needs a smooth
/// meter — this caps observer traffic regardless of the device frame cadence.
const LEVEL_METER_INTERVAL: Duration = Duration::from_millis(100);

/// Pre-onset ring buffer length (ms). The energy gate only flips voiced once a
/// word's attack clears the threshold, so the quiet onset (leading consonant,
/// the ramp into the first vowel) is below-gate and would be lost. Retain this
/// much raw audio continuously and prepend it at onset so sentence starts
/// aren't clipped — standard dictation-stack pre-roll.
const PREROLL_MS: usize = 400;

/// How many finalized utterances the listen-only decode worker may queue before
/// it drops the OLDEST (with a warning). Bounds memory if decode falls behind
/// sustained speech; whole utterances are dropped, never frames mid-utterance.
const DECODE_QUEUE_CAP: usize = 8;

/// Extract a self-given name from a transcript ("my name is X" / "call me
/// X" …). Mirrors voicelab's deliberately explicit phrase set — the loose
/// "I'm X" would mis-enroll ordinary chit-chat.
fn extract_spoken_name(text: &str) -> Option<String> {
    const PHRASES: [&str; 5] = [
        "my name is ",
        "you can call me ",
        "call me ",
        "i'm called ",
        "name's ",
    ];
    let lower = text.to_lowercase();
    for phrase in PHRASES {
        if let Some(pos) = lower.find(phrase) {
            let rest = &text[pos + phrase.len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphabetic() || *c == '\'' || *c == '-')
                .collect();
            if (2..=30).contains(&name.chars().count()) {
                return Some(name);
            }
        }
    }
    None
}

/// Barge-in control over the audio substrate: drop the AEC render reference the
/// instant playback is silenced so stale frames stop cancelling the user's
/// onset. Implemented over `clawft_voice_aec::AecProcessor` in the bridge.
pub trait AudioControl: Send + Sync {
    /// Flush queued render reference (barge-in).
    fn flush(&self);
}

/// No-op audio control (tests / no-AEC deployments).
pub struct NoopAudioControl;
impl AudioControl for NoopAudioControl {
    fn flush(&self) {}
}

/// One step of the conversation's ECC lifecycle. The bridge's observer turns
/// these into `CausalGraph` nodes / `CrossRef`s; tests assert the sequence.
///
/// Not `Eq`: [`ConversationEvent::UserTurn`] carries the `VoiceAnalysis`
/// record, whose acoustic/prosodic fields are `f32` (no total order). Equality
/// comparisons in tests use `PartialEq`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversationEvent {
    /// A finalized user utterance committed as a `Follows` `CausalNode`.
    UserTurn {
        /// Transcript text.
        text: String,
        /// Attributed speaker id (if speaker ID is enabled and matched).
        speaker: Option<SpeakerId>,
        /// Attributed speaker name (private context, never spoken).
        speaker_name: Option<String>,
        /// The complete per-utterance voice decomposition (§W1.2), produced
        /// client-side in the session and handed off here at the observer
        /// boundary. The recorder observer carries it over `agent.turn.record`
        /// so `index_turn` stores it and merges its emotion axis. `None` when
        /// the record could not be produced (no STT text). Boxed to keep the
        /// enum small for the common non-user variants.
        voice_analysis: Option<Box<VoiceAnalysis>>,
    },
    /// A new speaker was enrolled (ECC per-speaker node created).
    SpeakerEnrolled {
        /// New speaker id.
        id: SpeakerId,
        /// New speaker name.
        name: String,
    },
    /// The fast ack — a **Speculative** spoken node (covers latency).
    SpeculativeReply {
        /// Ack text spoken by the fast TTS layer.
        ack: String,
    },
    /// The grounded answer — the **Committed** node that supersedes the ack.
    CommittedReply {
        /// Answer text (already shaped by the spoken-answer policy).
        answer: String,
    },
    /// Barge-in: the in-flight reply is **Contradicted** / pruned.
    Interrupted,
    /// The endpointer fired — the turn is finalizing (§W1.4 process event).
    /// Surfaces the smart-turn completion probability + its source + the
    /// trailing silence live, before they are discarded. Not a durable graph
    /// node — the surface renders it; the committed `UserTurn` follows.
    EndpointFired {
        /// Completion probability that fired the finalize (or the last computed
        /// before a max-silence ceiling finalize).
        completion_prob: f32,
        /// Which endpointer produced it: `smart-turn-v3` | `heuristic`.
        source: String,
        /// Trailing-silence length at finalize (ms).
        silence_ms: u64,
    },
    /// A running (non-final) transcript while the user is still speaking
    /// (§W1.4 process event). Deferrable (riskiest-call #4): the controller
    /// does not emit this until incremental decode is wired — the surface
    /// falls back to the level meter + finalized line. Defined now so the
    /// surface never reshapes when partials land.
    PartialTranscript {
        /// Best-so-far transcript (may be revised by the next partial).
        text: String,
    },
    /// Live capture level (§W1.4 process event) — the per-frame RMS against
    /// the tracked room-tone floor, throttled to ~10 Hz. Drives the watch
    /// surface's level meter so the user sees input react **before** they
    /// finish speaking (matters for the acting-test feel). Surface-only; not
    /// a committed graph node.
    CaptureLevel {
        /// Frame RMS energy (dBFS).
        rms_dbfs: f32,
        /// Tracked noise-floor voice threshold (dBFS).
        floor_dbfs: f32,
    },
}

/// Observer of the conversation's ECC lifecycle. The bridge implements this
/// over the kernel; the controller calls it at each handoff point.
pub trait ConversationObserver: Send + Sync {
    /// Record one lifecycle event.
    fn observe(&self, event: ConversationEvent);
}

/// Observer that drops everything (tests that don't assert events / no-ECC).
pub struct NoopObserver;
impl ConversationObserver for NoopObserver {
    fn observe(&self, _event: ConversationEvent) {}
}

/// Talk-Mode configuration.
#[derive(Debug, Clone)]
pub struct TalkModeConfig {
    /// Capture sample rate (Hz).
    pub sample_rate: u32,
    /// VAD energy threshold (dBFS) for the voiced/silence decision.
    pub vad_threshold_dbfs: f32,
    /// Consecutive voiced frames during playback that count as a barge-in
    /// (debounces residual echo). e.g. ~150–250 ms worth of frames.
    pub barge_in_frames: u32,
    /// Default name assigned to a freshly enrolled speaker.
    pub default_speaker_name: String,
    /// Base agent system persona (the spoken-answer policy is appended).
    pub base_system: String,
    /// Where to persist the speaker registry (enrollments + spoken
    /// self-naming survive across sessions). `None` keeps it in-memory.
    pub speaker_store: Option<std::path::PathBuf>,
    /// Barge-in grace (ms) at playback start while the echo canceller
    /// converges on the new render signal. 0 disables (tests).
    pub barge_in_grace_ms: u64,
    /// Whether talking over the bot cancels its reply. OFF by default:
    /// the AEC's echo return loss is not yet verified on real hardware —
    /// live sessions showed the bot's own playback tripping the gate
    /// ~400 ms in and cancelling every answer. Enable once AEC residual
    /// is tuned (tracked in Plane).
    pub barge_in_enabled: bool,
    /// Listen-only mode (Wave 1 §W1.4): record + classify + store every turn
    /// (the recorder observer still fires `UserTurn` with the full
    /// `VoiceAnalysis`), but **skip the brain** — no ack, no LLM answer, no
    /// audio out. The surface shows ingestion + decomposition without a reply.
    /// OFF by default (full conversational loop).
    pub listen_only: bool,
}

impl Default for TalkModeConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            vad_threshold_dbfs: -45.0,
            barge_in_frames: 3,
            default_speaker_name: "unknown speaker".into(),
            base_system: String::new(),
            speaker_store: None,
            barge_in_grace_ms: BARGE_IN_GRACE_MS,
            barge_in_enabled: false,
            listen_only: false,
        }
    }
}

#[cfg(test)]
mod name_tests {
    use super::extract_spoken_name;

    #[test]
    fn explicit_phrases_extract_names() {
        assert_eq!(
            extract_spoken_name("Hi, my name is Mathew."),
            Some("Mathew".into())
        );
        assert_eq!(
            extract_spoken_name("you can call me Jean-Luc please"),
            Some("Jean-Luc".into())
        );
        assert_eq!(extract_spoken_name("Name's O'Brien"), Some("O'Brien".into()));
    }

    #[test]
    fn loose_or_absent_phrases_do_not_enroll() {
        assert_eq!(extract_spoken_name("I'm tired today"), None);
        assert_eq!(extract_spoken_name("what is seventeen times three"), None);
        // Over-long / single-char captures rejected.
        assert_eq!(extract_spoken_name("call me X"), None);
    }
}

/// A finalized utterance handed from the capture loop to decode. It carries the
/// endpoint + noise-floor readings sampled *at finalize* so decode can run off
/// the capture loop without touching the (still-advancing) endpointer / floor.
struct FinalizedTurn {
    samples: Vec<i16>,
    voiced_samples: usize,
    endpoint: Option<EndpointSnapshot>,
    noise_floor_dbfs: f32,
    noise_floor_converged: bool,
}

/// The STT → speaker → record → emit pipeline for one finalized utterance,
/// factored out of the controller so it runs either inline (talk mode, which
/// then speaks) or on a spawned worker (listen-only, so the capture loop never
/// blocks during the slow native decode — the fix for the round-3 "restart is
/// wrong" speech loss). All state is `Arc`, so it is cheap to clone into the
/// worker.
#[derive(Clone)]
struct Decoder {
    stt: Arc<dyn SttBackend>,
    embedder: Option<Arc<dyn SpeakerEmbedder>>,
    registry: Arc<Mutex<SpeakerRegistry>>,
    ser: Arc<dyn SerModel>,
    observer: Arc<dyn ConversationObserver>,
    config: TalkModeConfig,
    /// Session-relative f0/energy baseline (§round-4). Shared across turns (the
    /// listen worker holds a clone of the same `Arc`), so arousal is scored on
    /// deviation from the speaker's running baseline, not just absolutes.
    baseline: Arc<Mutex<SessionBaseline>>,
    /// Capture-path drop/high-water counters, shared with the capture thread so
    /// the record reports whether frames were lost (§round-5 instrumentation).
    capture_metrics: Arc<CaptureMetrics>,
}

impl Decoder {
    /// Best-effort save of the speaker registry to the configured store.
    fn persist_registry(&self) {
        if let Some(path) = &self.config.speaker_store {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let reg = self.registry.lock().expect("speaker registry poisoned");
            if let Err(e) = reg.save(path) {
                warn!(error = %e, path = %path.display(), "speaker registry save failed");
            }
        }
    }

    /// Attribute the utterance to a speaker, returning the full
    /// [`SpeakerAnalysis`] (id / name / near-miss cosine / action / dim) plus
    /// the private LLM context. Listen-only never auto-enrolls (no registry
    /// pollution); talk mode enrolls a session speaker on a non-match.
    async fn attribute_speaker(&self, utt: &Utterance) -> (SpeakerAnalysis, Option<String>) {
        let Some(embedder) = self.embedder.clone() else {
            return (SpeakerAnalysis::default(), None);
        };
        let emb = match embedder.embed(&utt.samples, utt.sample_rate).await {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "speaker embed failed; turn unattributed");
                return (SpeakerAnalysis::default(), None);
            }
        };
        let embedding_dim = embedder.dim() as u32;
        // Lock once for the read + optional enroll; release before the observer
        // event / persist so those never run under the lock.
        let (analysis, enrolled) = {
            let mut reg = self.registry.lock().expect("speaker registry poisoned");
            let threshold = reg.threshold();
            let best = reg.best_match(&emb);
            let near_score = best.as_ref().map(|(_, s)| *s).unwrap_or(0.0);
            if let Some((id, score)) = best.filter(|(_, s)| *s >= threshold) {
                reg.attribute(&id, &emb);
                let name = reg.get(&id).map(|n| n.name.clone());
                (
                    SpeakerAnalysis {
                        id: Some(id),
                        name,
                        score,
                        threshold,
                        action: SpeakerAction::Identified,
                        embedding_dim,
                    },
                    None,
                )
            } else if self.config.listen_only {
                (
                    SpeakerAnalysis {
                        id: None,
                        name: None,
                        score: near_score,
                        threshold,
                        action: SpeakerAction::Unknown,
                        embedding_dim,
                    },
                    None,
                )
            } else {
                let id = reg.enroll(self.config.default_speaker_name.clone(), &emb);
                let name = reg.get(&id).map(|n| n.name.clone());
                (
                    SpeakerAnalysis {
                        id: Some(id.clone()),
                        name: name.clone(),
                        score: near_score,
                        threshold,
                        action: SpeakerAction::Enrolled,
                        embedding_dim,
                    },
                    Some((id, name.unwrap_or_default())),
                )
            }
        };
        if let Some((id, name)) = enrolled {
            self.observer
                .observe(ConversationEvent::SpeakerEnrolled { id, name });
            self.persist_registry();
        }
        let ctx = analysis.id.as_ref().zip(analysis.name.as_ref()).map(|(id, n)| {
            format!("The current speaker is {n} (id {id}). Use this only as private context; never read it aloud.")
        });
        (analysis, ctx)
    }

    /// Assemble the complete `VoiceAnalysis` record (§W1.2) — endpoint + floor
    /// come from the [`FinalizedTurn`] sampled at finalize, the rest from the
    /// DSP extractors over the utterance.
    #[allow(clippy::too_many_arguments)]
    fn build_voice_analysis(
        &self,
        utt: &Utterance,
        detail: &super::stt::TranscriptResult,
        speaker: SpeakerAnalysis,
        stt_latency_ms: u64,
        voiced_samples: usize,
        endpoint: Option<EndpointSnapshot>,
        noise_floor_dbfs: f32,
        noise_floor_converged: bool,
    ) -> VoiceAnalysis {
        let sr = utt.sample_rate.max(1);
        // True voiced duration (excludes pre-roll padding in `utt.samples`).
        let voiced_ms = voiced_samples as u64 * 1_000 / sr as u64;

        let tokens: Vec<TokenAnalysis> = detail
            .tokens
            .iter()
            .map(|t| TokenAnalysis {
                text: t.text.clone(),
                t_ms: t.start_ms,
                dur_ms: t.duration_ms,
                conf: t.confidence,
            })
            .collect();
        let path = if tokens.is_empty() {
            SttPath::Substrate
        } else {
            SttPath::Native
        };
        let stt = SttAnalysis::new(self.stt.model().wire_name(), path, stt_latency_ms, tokens);

        let silence_tail_ms = endpoint.as_ref().map(|s| s.silence_tail_ms).unwrap_or(0);
        let duration_ms = voiced_ms + silence_tail_ms;
        let endpoint = EndpointAnalysis {
            completion_prob: endpoint.as_ref().map(|s| s.completion_prob).unwrap_or(0.0),
            source: endpoint
                .as_ref()
                .map(|s| s.source.clone())
                .unwrap_or_else(|| "unknown".into()),
            silence_tail_ms,
            latency_ms: duration_ms,
        };

        let health = capture_health(&utt.samples, sr);
        let audio = AudioAnalysis {
            duration_ms,
            voiced_ms,
            silence_ms: silence_tail_ms,
            rms_dbfs_mean: health.rms_dbfs_mean,
            rms_dbfs_peak: health.rms_dbfs_peak,
            noise_floor_dbfs,
            snr_db: health.rms_dbfs_mean - noise_floor_dbfs,
            clip_pct: health.clip_pct,
            dc_offset: health.dc_offset,
            noise_floor_converged,
            dropped_frames: self.capture_metrics.dropped(),
            channel_peak: self.capture_metrics.peak(),
        };

        let prosody = analyze_prosody(&ProsodyInput {
            samples: &utt.samples,
            sample_rate: sr,
            voiced_ms,
            tokens: &detail.tokens,
        });
        // Score arousal against the session baseline (deviation), then fold this
        // turn in. Zero deviation on the first turn (cold start = absolute).
        let deviation = self
            .baseline
            .lock()
            .expect("session baseline poisoned")
            .observe(prosody.f0_mean_hz, health.rms_dbfs_mean);
        let emotion = refine_emotion(
            emotion_from_prosody_baseline(&prosody, deviation),
            self.ser.predict(&utt.samples, sr),
        );
        let paralinguistics = classify_paralinguistics(&ParalinguisticInput {
            transcript: &detail.text,
            voiced_ms,
            energy_dynamics_db: prosody.energy_dynamics_db,
            has_f0: prosody.f0_mean_hz > 0.0,
        });

        VoiceAnalysis::new(stt, endpoint, speaker, audio, prosody, emotion, paralinguistics)
    }

    /// Full decode of one finalized utterance: STT → speaker → record → emit
    /// `UserTurn`. Returns `(transcript, private speaker context)` for talk-mode
    /// follow-up, or `None` when the turn is dropped (empty transcript / error).
    async fn decode_and_emit(&self, turn: FinalizedTurn) -> Option<(String, Option<String>)> {
        let sr = self.config.sample_rate;
        let utt = Utterance {
            samples: turn.samples,
            sample_rate: sr,
        };
        info!(
            samples = utt.samples.len(),
            ms = utt.samples.len() as u64 * 1000 / sr.max(1) as u64,
            "talk-mode end-of-turn utterance captured"
        );

        let stt_start = Instant::now();
        let detail = match self.stt.transcribe_detailed(&utt).await {
            Ok(d) if !d.text.trim().is_empty() => d,
            Ok(_) => {
                info!("talk-mode STT returned an empty transcript; turn dropped");
                return None;
            }
            Err(e) => {
                warn!(error = %e, "talk-mode STT failed");
                return None;
            }
        };
        let stt_latency_ms = stt_start.elapsed().as_millis() as u64;
        let text = detail.text.clone();
        info!(transcript = %text, "talk-mode user turn");

        let (speaker, speaker_ctx) = self.attribute_speaker(&utt).await;
        let speaker_id = speaker.id.clone();
        let speaker_name = speaker.name.clone();

        // Spoken self-enrollment: a voice naming itself upgrades its placeholder.
        if let (Some(id), Some(name)) = (&speaker_id, extract_spoken_name(&text)) {
            let renamed = self
                .registry
                .lock()
                .expect("speaker registry poisoned")
                .rename(id, name.clone());
            if renamed {
                info!(speaker = %id, %name, "talk-mode speaker self-enrolled by name");
                self.persist_registry();
            }
        }

        let voice_analysis = self.build_voice_analysis(
            &utt,
            &detail,
            speaker,
            stt_latency_ms,
            turn.voiced_samples,
            turn.endpoint,
            turn.noise_floor_dbfs,
            turn.noise_floor_converged,
        );

        self.observer.observe(ConversationEvent::UserTurn {
            text: text.clone(),
            speaker: speaker_id,
            speaker_name,
            voice_analysis: Some(Box::new(voice_analysis)),
        });
        Some((text, speaker_ctx))
    }
}

/// The Talk-Mode controller. Generic over the endpoint model so a smart-turn
/// ONNX model or the heuristic default both fit.
pub struct TalkModeController<M: EndpointModel> {
    endpointer: SemanticEndpointer<M>,
    policy: VoiceAnswerPolicy,
    llm: Arc<dyn VoiceLlm>,
    tts: DualLayerTts,
    sink: Arc<dyn TtsSink>,
    audio: Arc<dyn AudioControl>,
    observer: Arc<dyn ConversationObserver>,
    config: TalkModeConfig,
    /// STT → speaker → record → emit pipeline (shared with the listen worker).
    decoder: Decoder,
    /// Adaptive room-tone tracker. The voiced gate is
    /// `max(config.vad_threshold_dbfs, floor + margin)` so a loud room
    /// (fans, HVAC) cannot read as permanent speech and starve the
    /// silence-based endpointer (observed live: -37 dBFS room vs the
    /// -45 dBFS fixed default — turns never finalized).
    noise_floor: NoiseFloor,
}

impl<M: EndpointModel> TalkModeController<M> {
    /// Assemble a controller from its parts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpointer: SemanticEndpointer<M>,
        stt: Arc<dyn SttBackend>,
        embedder: Option<Arc<dyn SpeakerEmbedder>>,
        registry: SpeakerRegistry,
        policy: VoiceAnswerPolicy,
        llm: Arc<dyn VoiceLlm>,
        tts: DualLayerTts,
        sink: Arc<dyn TtsSink>,
        audio: Arc<dyn AudioControl>,
        observer: Arc<dyn ConversationObserver>,
        config: TalkModeConfig,
    ) -> Self {
        let decoder = Decoder {
            stt,
            embedder,
            registry: Arc::new(Mutex::new(registry)),
            ser: Arc::new(DspSer),
            observer: observer.clone(),
            config: config.clone(),
            baseline: Arc::new(Mutex::new(SessionBaseline::new())),
            capture_metrics: Arc::new(CaptureMetrics::default()),
        };
        Self {
            endpointer,
            policy,
            llm,
            tts,
            sink,
            audio,
            observer,
            config,
            decoder,
            noise_floor: NoiseFloor::new(VAD_NOISE_MARGIN_DB),
        }
    }

    /// Inject a speech-emotion model (§W1.2 SER seam). By default the
    /// controller uses [`DspSer`] (no model → DSP arousal floor); a real SER
    /// ONNX from `clawft-voice-onnx` overrides `emotion.valence` + `label`
    /// (and `dominance`) while DSP arousal stays the floor.
    pub fn with_ser(mut self, ser: Arc<dyn SerModel>) -> Self {
        self.decoder.ser = ser;
        self
    }

    /// The shared capture-path metrics — pass to the `CaptureProcessor`
    /// (`with_metrics`) so drops it records surface in this controller's records.
    pub fn capture_metrics(&self) -> Arc<CaptureMetrics> {
        self.decoder.capture_metrics.clone()
    }

    /// Per-frame level: the voiced decision plus the `CaptureLevel` inputs
    /// (frame RMS + the tracked floor threshold). Advances the adaptive
    /// noise-floor tracker, so call exactly once per captured frame.
    fn level(&mut self, frame: &[i16]) -> (bool, f32, f32) {
        let rms_dbfs = EnergyVad::rms_dbfs(frame);
        let adaptive = self.noise_floor.observe(rms_dbfs);
        let voiced = rms_dbfs >= adaptive.max(self.config.vad_threshold_dbfs);
        (voiced, rms_dbfs, adaptive)
    }

    /// Run the conversation loop until `cancel` fires or the capture channel
    /// closes. Frames are 16 kHz mono `i16` (echo-cancelled mic).
    pub async fn run(&mut self, mut frames: mpsc::Receiver<Vec<i16>>, cancel: CancellationToken) {
        // Pre-render the fixed ack set through the slow tier (off-thread) so
        // acks speak in the answer's own voice from the first turn that the
        // warm beats; fast-tier fallback covers the race. Skipped in listen-only
        // mode — that path never speaks, so it does zero synthesis.
        if !self.config.listen_only {
            self.tts
                .spawn_warm_acks(vec![ACK_SHORT.to_string(), ACK_LONG.to_string()]);
        }

        // Listen-only decode worker: the capture loop hands finalized utterances
        // to this off-loop task so it NEVER blocks on the slow native decode
        // (the round-3 fix — no drain, no during-decode speech loss). A drop-
        // oldest queue bounds memory if decode falls behind sustained speech.
        let (decode_queue, decode_notify, decode_worker) = if self.config.listen_only {
            let queue: Arc<Mutex<VecDeque<FinalizedTurn>>> = Arc::new(Mutex::new(VecDeque::new()));
            let notify = Arc::new(Notify::new());
            let decoder = self.decoder.clone();
            let wq = queue.clone();
            let wn = notify.clone();
            let wc = cancel.clone();
            let handle = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = wc.cancelled() => break,
                        _ = wn.notified() => {
                            loop {
                                let next = wq.lock().expect("decode queue poisoned").pop_front();
                                match next {
                                    Some(turn) => {
                                        decoder.decode_and_emit(turn).await;
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                }
            });
            (Some(queue), Some(notify), Some(handle))
        } else {
            (None, None, None)
        };

        let mut utt: Vec<i16> = Vec::new();
        // Voiced-sample count for the current utterance — the min-turn guard
        // gates on this, not `utt.len()`, since pre-roll prepends (mostly quiet)
        // pre-onset audio that would otherwise let a noise blip clear the gate.
        let mut voiced_samples: usize = 0;
        let mut last_level: Option<Instant> = None;
        // Pre-onset ring: the last PREROLL_MS of raw frames, prepended to a new
        // utterance at onset so the below-gate word attack isn't clipped.
        let preroll_cap = (self.config.sample_rate as usize) * PREROLL_MS / 1_000;
        let mut preroll: VecDeque<i16> = VecDeque::with_capacity(preroll_cap + 1);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                maybe = frames.recv() => {
                    let Some(frame) = maybe else { break };
                    let (voiced, rms_dbfs, floor_dbfs) = self.level(&frame);
                    // Live level meter (§W1.4), throttled to ~10 Hz so the
                    // observer channel isn't flooded at the device frame rate.
                    if last_level.is_none_or(|t| t.elapsed() >= LEVEL_METER_INTERVAL) {
                        last_level = Some(Instant::now());
                        self.observer.observe(ConversationEvent::CaptureLevel {
                            rms_dbfs,
                            floor_dbfs,
                        });
                    }
                    // Onset (first voiced frame of a fresh utterance): prepend the
                    // pre-onset ring so the clipped attack is recovered.
                    if voiced && utt.is_empty() {
                        utt.extend(preroll.iter().copied());
                    }
                    // Maintain the ring with this frame AFTER the onset check, so
                    // the prepended audio is strictly pre-onset (the current voiced
                    // frame is added to `utt` below, not duplicated from the ring).
                    preroll.extend(frame.iter().copied());
                    while preroll.len() > preroll_cap {
                        preroll.pop_front();
                    }
                    if voiced {
                        utt.extend_from_slice(&frame);
                        voiced_samples += frame.len();
                    }
                    match self.endpointer.observe(&frame, voiced, "").await {
                        TurnDecision::Continue => {}
                        TurnDecision::Finalize => {
                            let captured = std::mem::take(&mut utt);
                            let captured_voiced = std::mem::take(&mut voiced_samples);
                            // Guard against ghost turns: noise blips and the
                            // reverb tail of our own playback can cross the
                            // gate for a few frames and finalize a sub-word
                            // "utterance" — each of which would earn an ack.
                            // Gate on VOICED samples (pre-roll padding excluded).
                            let min_samples =
                                (self.config.sample_rate as usize) * MIN_TURN_MS / 1_000;
                            if captured_voiced >= min_samples {
                                // Surface the endpoint decision (§W1.4) just
                                // before the turn is processed — prob + source +
                                // silence tail, captured by the endpointer.
                                let snap = self.endpointer.last_endpoint().cloned();
                                if let Some(s) = &snap {
                                    self.observer.observe(ConversationEvent::EndpointFired {
                                        completion_prob: s.completion_prob,
                                        source: s.source.clone(),
                                        silence_ms: s.silence_tail_ms,
                                    });
                                }
                                let turn = FinalizedTurn {
                                    samples: captured,
                                    voiced_samples: captured_voiced,
                                    endpoint: snap,
                                    noise_floor_dbfs: self.noise_floor.floor_dbfs(),
                                    noise_floor_converged: self.noise_floor.converged(),
                                };
                                if let (Some(q), Some(n)) = (&decode_queue, &decode_notify) {
                                    // Listen-only: hand off to the worker and keep
                                    // consuming — the loop never blocks on decode.
                                    {
                                        let mut ql = q.lock().expect("decode queue poisoned");
                                        if ql.len() >= DECODE_QUEUE_CAP {
                                            ql.pop_front();
                                            warn!("decode backlog full; dropping oldest utterance");
                                        }
                                        ql.push_back(turn);
                                    }
                                    n.notify_one();
                                } else {
                                    // Talk mode: decode inline, then reply.
                                    self.talk_turn(turn, &mut frames, &cancel).await;
                                }
                            } else if !captured.is_empty() {
                                debug!(
                                    ms = captured.len() * 1_000
                                        / self.config.sample_rate.max(1) as usize,
                                    "talk-mode dropping sub-minimum utterance blip"
                                );
                            }
                        }
                    }
                }
            }
        }
        // Stop the decode worker (cancel usually already did; abort covers the
        // capture-channel-closed path where `cancel` never fired).
        if let Some(handle) = decode_worker {
            handle.abort();
        }
    }

    /// One talk-mode turn: decode + emit the `UserTurn` (via the shared
    /// [`Decoder`]), then ack → grounded answer → speak with barge-in. Talk mode
    /// decodes inline (it must produce the reply anyway); listen-only runs the
    /// same decode off-loop in the worker.
    async fn talk_turn(
        &mut self,
        turn: FinalizedTurn,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) {
        let Some((text, speaker_ctx)) = self.decoder.decode_and_emit(turn).await else {
            return; // empty transcript / STT error — turn dropped (UserTurn not emitted)
        };

        // Fast ack = Speculative spoken node (6.4 fast layer covers latency).
        let ack = contextual_ack(&text);
        self.observer
            .observe(ConversationEvent::SpeculativeReply { ack: ack.clone() });

        // Grounded answer, shaped short by the spoken-answer policy (6.3).
        let answer = match self
            .policy
            .answer(self.llm.as_ref(), &self.config.base_system, &text, speaker_ctx)
            .await
        {
            Ok(a) if !a.trim().is_empty() => a,
            Ok(_) => {
                warn!("talk-mode LLM returned an empty answer; turn dropped");
                return;
            }
            Err(e) => {
                warn!(error = %e, "talk-mode LLM failed");
                return;
            }
        };
        info!(answer = %answer, "talk-mode committed reply");
        self.observer.observe(ConversationEvent::CommittedReply {
            answer: answer.clone(),
        });

        // Speak ack then stream the answer (6.4); monitor for barge-in.
        self.speak_with_barge_in(&ack, &answer, frames, cancel).await;
    }

    /// Play the fast ack then stream the answer; a sustained voiced run during
    /// playback is a barge-in → cancel TTS (flushes the sink) + flush the AEC
    /// render reference + emit `Interrupted`.
    async fn speak_with_barge_in(
        &self,
        ack: &str,
        answer: &str,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) {
        // Snapshot the voiced gate for the playback window: the noise-floor
        // tracker must NOT learn from frames while the bot is speaking (its
        // own echo would lift the floor), so barge-in uses a frozen, steeper
        // threshold instead of `self.voiced` (see BARGE_IN_MARGIN_DB).
        let barge_gate = (self.noise_floor.floor_dbfs() + BARGE_IN_MARGIN_DB)
            .max(self.config.vad_threshold_dbfs);
        let speak_started = std::time::Instant::now();
        let turn_cancel = cancel.child_token();
        let ack_owned = ack.to_string();
        let speak = self.tts.speak(
            Some(ack_owned.as_str()),
            answer,
            self.sink.clone(),
            turn_cancel.clone(),
        );
        tokio::pin!(speak);

        let mut voiced_run: u32 = 0;
        loop {
            tokio::select! {
                res = &mut speak => {
                    if let Err(e) = res {
                        warn!(error = %e, "talk-mode TTS error");
                    }
                    break; // finished speaking, uninterrupted
                }
                _ = cancel.cancelled() => {
                    turn_cancel.cancel();
                    let _ = (&mut speak).await;
                    break;
                }
                maybe = frames.recv() => {
                    match maybe {
                        Some(frame) => {
                            let in_grace = speak_started.elapsed()
                                < std::time::Duration::from_millis(self.config.barge_in_grace_ms);
                            if self.config.barge_in_enabled
                                && !in_grace
                                && EnergyVad::rms_dbfs(&frame) >= barge_gate
                            {
                                voiced_run += 1;
                                if voiced_run >= self.config.barge_in_frames {
                                    debug!("talk-mode barge-in detected");
                                    self.audio.flush();          // AEC render reference
                                    turn_cancel.cancel();        // TTS producer + sink flush
                                    self.observer.observe(ConversationEvent::Interrupted);
                                    let _ = (&mut speak).await;   // let it unwind
                                    return;
                                }
                            } else {
                                voiced_run = 0;
                            }
                        }
                        None => {
                            turn_cancel.cancel();
                            let _ = (&mut speak).await;
                            return;
                        }
                    }
                }
            }
        }
        // Synthesis finished, but `play_chunk` only QUEUES audio — seconds of
        // our own speech may still be leaving the speaker. Hold here until
        // the sink drains — KEEP CONSUMING mic frames while waiting (they
        // contain our own voice; unconsumed they fill the capture channel
        // and the capture thread floods "frame channel full" warnings) —
        // then discard any stragglers so capture resumes clean.
        let drained = self.sink.wait_drained();
        tokio::pin!(drained);
        loop {
            tokio::select! {
                _ = &mut drained => break,
                maybe = frames.recv() => {
                    if maybe.is_none() {
                        break; // capture channel closed
                    }
                }
            }
        }
        while frames.try_recv().is_ok() {}
    }
}

/// Build a short *contextual* acknowledgment from the transcript (echo the
/// subject so the latency reads as "thinking", not lag — ADR-061). A fast-LLM
/// ack is the future upgrade; this deterministic version needs no model.
/// The fixed ack set. Kept to CLOSED strings (no transcript interpolation)
/// so [`DualLayerTts`] can pre-render every ack through the SLOW tier at
/// session start and play from cache — the ack then speaks in the SAME
/// voice as the answer (single-voice consistency, the david-profile
/// property) with zero latency. The subject-echo flourish
/// ("{Subject} — one sec.") returns when the fast tier is a clone of the
/// slow tier's voice (WEFT-613).
pub const ACK_SHORT: &str = "One sec.";
/// Ack for longer questions.
pub const ACK_LONG: &str = "Okay, one sec.";

pub fn contextual_ack(transcript: &str) -> String {
    let words: Vec<&str> = transcript.split_whitespace().collect();
    if words.len() <= 3 {
        ACK_SHORT.to_string()
    } else {
        ACK_LONG.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_is_short_for_short_input() {
        assert_eq!(contextual_ack("hi"), "One sec.");
        assert_eq!(contextual_ack("what time"), "One sec.");
    }

    #[test]
    fn ack_stays_in_the_fixed_cacheable_set() {
        // Acks are a CLOSED set so the slow tier can pre-render them all
        // (single-voice consistency). Subject echo returns with WEFT-613.
        assert_eq!(contextual_ack("tell me about Puyo please"), ACK_LONG);
        assert_eq!(contextual_ack("can you explain the thing now"), ACK_LONG);
        assert_eq!(contextual_ack("quick question"), ACK_SHORT);
    }
}
