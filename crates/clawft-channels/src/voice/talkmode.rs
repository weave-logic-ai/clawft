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
    AudioAnalysis, EndpointAnalysis, ParalinguisticClass, SpeakerAction, SpeakerAnalysis,
    SttAnalysis, SttPath, TokenAnalysis, VoiceAnalysis,
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
use super::voiceness::{SpectralVoiceness, Voiceness};

/// How far above the tracked room-tone floor a frame must sit to count as
/// voice. 10 dB, above a quiet room's own dynamic swing: the floor tracks the
/// quiet-p10 (~-50 dBFS at sane gain) but room tone fluctuates ~7 dB up to its
/// peaks (~-43), so a tighter gate lets the room cross its own onset and the
/// utterance never ends (Wall 5). The round-8 relaxation to 4 dB chased a "5 dB
/// SNR corner" that was an ARTIFACT of a hot input gain (floor -37); at sane
/// gain real speech runs 25–35 dB over the floor (peaks -25..-13), so 10 dB
/// keeps room peaks out while speech clears by 15+ dB. Voiceness stays as the
/// AND-gate for broadband bursts that DO cross. Sustain (`margin/2`) + 400 ms
/// hangover unchanged.
const VAD_NOISE_MARGIN_DB: f32 = 10.0;

/// Minimum spectral speech-band ratio a frame must show to START an utterance.
/// Broadband room tone scores ~its bandwidth fraction (~0.4); voiced speech
/// scores ~0.8–0.95, and stays above this even mixed with noise a few dB below
/// it. Applied only at onset — once voiced, the energy hysteresis/hangover
/// sustains through unvoiced consonants (fricatives are broadband and would
/// score low), and pre-roll recovers a fricative that preceded the onset.
const VOICENESS_MIN: f32 = 0.5;

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

/// Default inter-sentence listening window (ms, WEFT-615 interim) — see
/// [`TalkModeConfig::sentence_window_ms`]. No grace period is needed here
/// (unlike [`BARGE_IN_GRACE_MS`]): nothing plays during the window, so
/// there is no echo to converge past.
const DEFAULT_SENTENCE_WINDOW_MS: u64 = 350;

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
    /// A contextual filler spoken when the grounded answer hasn't landed
    /// within the filler gate (~1.5 s after the ack) — buys the (slow) brain
    /// thinking time and opens a natural interruption window (WEFT-658).
    /// Dynamic, subject-derived text rendered on the fly through the FAST
    /// tier, so unlike [`SpeculativeReply`](Self::SpeculativeReply) it is
    /// never pre-rendered/cached. Spoken at most once per turn; surface-only
    /// like the other §W1.4 process events — not a durable graph node.
    SpeculativeFiller {
        /// Filler text spoken by the fast TTS layer.
        text: String,
    },
    /// A turn was gated before the reply slot ran — no ack, no filler, no
    /// `VoiceLlm::complete`, no spoken reply (WEFT-659). The turn still
    /// finalizes and records/observes as [`UserTurn`](Self::UserTurn)
    /// normally (the decomposition on the graph is wanted either way); this
    /// is a companion, surface-only §W1.4 process event explaining *why* no
    /// reply followed. Grouped with [`SpeculativeFiller`](Self::SpeculativeFiller)
    /// wherever observers treat process events as surface-only.
    TurnGated {
        /// Why the reply slot was skipped: `"non_lexical"` (paralinguistics
        /// classified the turn as backchannel/laughter/filler), `"unclear"`
        /// ([`utterance_clarity`] failed — a spoken clarification request
        /// was the only audio emitted), or `"playback_stop"` (WEFT-615: an
        /// inter-sentence listening window caught a bare STOP phrase — the
        /// user just wanted silence, not a new turn).
        reason: String,
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
    /// Inter-sentence listening window (ms, WEFT-615 interim): the spoken
    /// answer is split into sentences ([`super::tts::split_sentences`]),
    /// each played to completion; between sentences the sink is drained to
    /// physical silence and this window listens for a sustained voiced
    /// onset against the SAME frozen barge gate the continuous monitor uses
    /// ([`BARGE_IN_MARGIN_DB`] over the noise floor, not learning). Because
    /// nothing is playing during the window, it is echo-free BY PHYSICS —
    /// no AEC/ERL tuning required — making it the SPEAKER-SAFE default,
    /// unlike [`barge_in_enabled`](Self::barge_in_enabled) (continuous
    /// monitoring DURING playback; open speakers echo the bot's own voice
    /// back into the gate, so that stays headphones-only and off by
    /// default). `0` disables windows entirely — the answer plays as one
    /// single-shot call, byte-identical to the pre-WEFT-615 path. A single-
    /// sentence reply never opens a window (nothing to interrupt between).
    pub sentence_window_ms: u64,
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
            sentence_window_ms: DEFAULT_SENTENCE_WINDOW_MS,
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
    /// Whether this utterance's onset was the captured tail of a WEFT-615
    /// inter-sentence playback-window interrupt (vs an ordinary idle-mic
    /// onset) — `talk_turn` scopes the bare-stop swallow
    /// ([`is_bare_stop_phrase`]) to window interrupts only: "stop" said
    /// while idle is an ordinary word (mirrors interrupt_router.rs's timing
    /// axis — see [`WINDOW_STOP_PHRASES`]).
    from_window_interrupt: bool,
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
    /// [`SpeakerAnalysis`] plus the private LLM context. A confident match is
    /// `Identified`; otherwise the turn is `Unknown` and NO profile is written —
    /// the only enrollment path is a spoken self-ID ("my name is X"), and only
    /// in talk mode. This closes the registry-pollution leak at its source:
    /// neither mode ever persists a placeholder "unknown speaker".
    async fn attribute_speaker(
        &self,
        utt: &Utterance,
        text: &str,
    ) -> (SpeakerAnalysis, Option<String>) {
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
        let spoken_name = extract_spoken_name(text);
        // Lock once for the read + optional attribute/enroll/rename; release
        // before the observer event / persist so those never run under the lock.
        let (analysis, persist, enrolled) = {
            let mut reg = self.registry.lock().expect("speaker registry poisoned");
            let threshold = reg.threshold();
            let best = reg.best_match(&emb);
            let near_score = best.as_ref().map(|(_, s)| *s).unwrap_or(0.0);
            if let Some((id, score)) = best.filter(|(_, s)| *s >= threshold) {
                // Confident match — fold in the embedding; a spoken self-ID
                // renames this existing speaker (not a new profile).
                reg.attribute(&id, &emb);
                let renamed = spoken_name.as_ref().is_some_and(|n| reg.rename(&id, n.clone()));
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
                    renamed,
                    None,
                )
            } else if !self.config.listen_only && spoken_name.is_some() {
                // Talk mode + a spoken self-ID + no match → a DELIBERATE
                // enrollment under the real name (never a placeholder).
                let name = spoken_name.clone().unwrap();
                let id = reg.enroll(name.clone(), &emb);
                (
                    SpeakerAnalysis {
                        id: Some(id.clone()),
                        name: Some(name.clone()),
                        score: near_score,
                        threshold,
                        action: SpeakerAction::Enrolled,
                        embedding_dim,
                    },
                    true,
                    Some((id, name)),
                )
            } else {
                // No confident match and no self-ID → Unknown, persist nothing.
                (
                    SpeakerAnalysis {
                        id: None,
                        name: None,
                        score: near_score,
                        threshold,
                        action: SpeakerAction::Unknown,
                        embedding_dim,
                    },
                    false,
                    None,
                )
            }
        };
        if let Some((id, name)) = enrolled {
            info!(speaker = %id, %name, "talk-mode speaker enrolled by spoken name");
            self.observer
                .observe(ConversationEvent::SpeakerEnrolled { id, name });
        }
        if persist {
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
    /// `UserTurn`. Returns the [`DecodedTurn`] (transcript, private speaker
    /// context, and the produced `VoiceAnalysis` — threaded back so the
    /// talk-mode reply slot can gate on it per WEFT-659 without re-deriving
    /// what this method already computed), or `None` when the turn is
    /// dropped (empty transcript / error).
    async fn decode_and_emit(&self, turn: FinalizedTurn) -> Option<DecodedTurn> {
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

        // Attribution handles identify / spoken-self-ID enroll / rename inline;
        // it never writes a placeholder profile.
        let (speaker, speaker_ctx) = self.attribute_speaker(&utt, &text).await;
        let speaker_id = speaker.id.clone();
        let speaker_name = speaker.name.clone();

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
            voice_analysis: Some(Box::new(voice_analysis.clone())),
        });
        Some(DecodedTurn {
            text,
            speaker_ctx,
            analysis: voice_analysis,
        })
    }
}

/// The result of [`Decoder::decode_and_emit`]: the transcript, private
/// speaker context for the LLM system prompt, and the full per-utterance
/// [`VoiceAnalysis`] threaded back to the caller (not just emitted on the
/// observer) so `talk_turn` can gate the reply slot on it (WEFT-659).
struct DecodedTurn {
    text: String,
    speaker_ctx: Option<String>,
    analysis: VoiceAnalysis,
}

// ── Reply-slot gate (WEFT-659): non-lexical + unclear ───────────────────
//
// Two independent reasons `talk_turn` may skip the ENTIRE reply slot (ack,
// filler, `VoiceLlm::complete`, spoken answer) for an otherwise-finalized
// turn. The turn still finalizes/records/observes as `UserTurn` — only the
// reply slot is skipped, and a `ConversationEvent::TurnGated` explains why.

/// Whether an utterance's transcript is trustworthy enough to hand to the
/// grounded LLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clarity {
    /// Confident enough to answer.
    Clear,
    /// Confidence/SNR combination says the transcript is likely wrong; ask
    /// the user to repeat rather than answer garbage.
    Unclear,
}

/// Utterances at/above this word count with mean confidence below
/// [`CLARITY_LONG_CONF_MIN`] are unclear — a long transcript is far more
/// likely to contain a substantive misrecognition than a one-word answer.
const CLARITY_LONG_WORD_COUNT: usize = 4;
/// Mean-confidence floor for the long-utterance rule.
const CLARITY_LONG_CONF_MIN: f32 = 0.55;
/// Below this mean confidence, ANY utterance (regardless of length) is
/// unclear. Live-trace calibration: "No." (1 word, conf 0.44) must stay
/// CLEAR, so this floor sits well below that, not at it.
const CLARITY_ABSOLUTE_CONF_MIN: f32 = 0.30;
/// A converged noise floor with SNR below this corroborates a borderline
/// confidence score (the mic really was struggling): combined with
/// [`CLARITY_LOW_SNR_CONF_MIN`] this catches turns the two floors above
/// individually let through.
const CLARITY_LOW_SNR_DB: f32 = 5.0;
/// Confidence floor for the low-SNR rule (higher than
/// [`CLARITY_LONG_CONF_MIN`] — a bad mic environment needs more headroom
/// before its transcript is trusted).
const CLARITY_LOW_SNR_CONF_MIN: f32 = 0.70;

/// Rate `analysis`'s transcript-worthiness. Deliberately expressible from
/// wire-level [`VoiceAnalysis`] fields only (`stt.token_conf_mean`,
/// `audio.snr_db`, `audio.noise_floor_converged`) plus the transcript's word
/// count, so the daemon side can mirror this exact rule from the JSON
/// record without re-deriving it from raw audio. `transcript` supplies the
/// word count (not itself a `VoiceAnalysis` field).
///
/// Calibrated against live hot-mic traces (WEFT-659):
/// - "No." (1 word, conf 0.44, SNR 6.4 dB) → Clear (short answers are
///   allowed low confidence).
/// - "What question did you hear?" (conf 0.98, SNR 16.3 dB) → Clear.
/// - A 12-word utterance at conf ≈0.45 → Unclear (long + shaky = likely
///   garbled).
pub fn utterance_clarity(analysis: &VoiceAnalysis, transcript: &str) -> Clarity {
    let word_count = transcript.split_whitespace().count();
    let conf_mean = analysis.stt.token_conf_mean;
    let snr_db = analysis.audio.snr_db;

    if word_count >= CLARITY_LONG_WORD_COUNT && conf_mean < CLARITY_LONG_CONF_MIN {
        return Clarity::Unclear;
    }
    if conf_mean < CLARITY_ABSOLUTE_CONF_MIN {
        return Clarity::Unclear;
    }
    if snr_db < CLARITY_LOW_SNR_DB
        && analysis.audio.noise_floor_converged
        && conf_mean < CLARITY_LOW_SNR_CONF_MIN
    {
        return Clarity::Unclear;
    }
    Clarity::Clear
}

/// Whether a finalized turn should engage the grounded brain (ack + filler +
/// LLM + spoken answer), or be gated before any of that runs. Pure and
/// unit-tested; [`TalkModeController::talk_turn`] consumes the verdict and
/// owns the actual async speak/skip behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engage {
    /// Run the full reply slot.
    Engage,
    /// Skip the reply slot: paralinguistics classified this as a
    /// non-lexical vocalization (backchannel/laughter/filler), not speech
    /// addressed to the assistant.
    GateNonLexical,
    /// Skip the reply slot: [`utterance_clarity`] says the transcript isn't
    /// trustworthy; speak a clarification request instead.
    GateUnclear,
}

/// The gate `talk_turn` consults before running the reply slot. Non-lexical
/// paralinguistics wins over the clarity check — a backchannel is never
/// "unclear", it simply isn't addressed to us — so it must be checked FIRST,
/// before the (expensive to be wrong about) unclear path even runs.
pub fn should_engage_brain(analysis: &VoiceAnalysis, transcript: &str) -> Engage {
    match analysis.paralinguistics.class {
        ParalinguisticClass::BackchannelCandidate
        | ParalinguisticClass::LaughterCandidate
        | ParalinguisticClass::Filler => return Engage::GateNonLexical,
        ParalinguisticClass::Speech | ParalinguisticClass::Unknown => {}
    }
    match utterance_clarity(analysis, transcript) {
        Clarity::Unclear => Engage::GateUnclear,
        Clarity::Clear => Engage::Engage,
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
    /// Spectral voiceness gate — the "voice vs broadband noise" decision the
    /// energy floor can't make at ~5 dB SNR. AND-ed with the energy gate at
    /// utterance onset (see [`level`](Self::level)).
    voiceness: Arc<dyn Voiceness>,
    /// Whether we're mid-voiced-run: at onset both energy AND voiceness must
    /// agree; while sustaining, the energy hysteresis/hangover alone holds the
    /// gate so unvoiced consonants aren't punched out.
    voiced_run: bool,
    /// One-shot latch: log the armed floor + onset the first frame after
    /// startup calibration completes, so every session's report shows what the
    /// gate armed at.
    armed_logged: bool,
    /// Time throttle for the gate-input level probe.
    last_probe: Option<Instant>,
    /// Turn counter driving [`pick_ack`]'s pool rotation — advances once per
    /// spoken turn so back-to-back acks never repeat (WEFT-658).
    ack_counter: u32,
    /// Turn counter driving [`pick_filler`]'s pool rotation, independent of
    /// `ack_counter` (a turn speaks at most one filler, but not every turn
    /// speaks one, so the two counters drift apart over a session).
    filler_counter: u32,
    /// Turn counter driving [`pick_clarification`]'s pool rotation
    /// (WEFT-659 unclear gate), independent of the other two — a
    /// clarification is spoken only on `Engage::GateUnclear` turns.
    clarification_counter: u32,
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
        let config_sr = config.sample_rate;
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
            noise_floor: NoiseFloor::new(VAD_NOISE_MARGIN_DB, config_sr),
            voiceness: Arc::new(SpectralVoiceness::new()),
            voiced_run: false,
            armed_logged: false,
            last_probe: None,
            ack_counter: 0,
            filler_counter: 0,
            clarification_counter: 0,
        }
    }

    /// Inject a voiceness backend (the [`Voiceness`] seam). Defaults to the
    /// model-free [`SpectralVoiceness`]; a Silero-VAD ONNX backend overrides it
    /// once the model is staged, without touching the energy floor/watchdog.
    pub fn with_voiceness(mut self, voiceness: Arc<dyn Voiceness>) -> Self {
        self.voiceness = voiceness;
        self
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
        // Schmitt-trigger + hangover + floor-freeze gate. Purely floor-relative
        // (no fixed dBFS floor) so a quiet mic still voices — the frozen floor
        // stays honest instead of chasing quiet speech up and gating it out.
        let energy_voiced = self.noise_floor.classify(rms_dbfs, frame.len() as u64);
        // Score voiceness on EVERY frame so its rolling analysis buffer stays
        // warm: the live path delivers tiny (~80–160 sample) frames, and a cold
        // or stale buffer at onset would misjudge. The score is only USED to
        // gate the onset — require spectral voiceness (not just energy at the
        // relaxed 4 dB margin) to START a run so broadband room tone stays out;
        // once voiced, sustain on the energy hysteresis alone so unvoiced
        // consonants aren't punched out (they score low, and pre-roll recovers a
        // fricative that led the onset).
        let vscore = self.voiceness.score(frame, self.config.sample_rate);
        let voiced = if self.voiced_run {
            energy_voiced
        } else {
            energy_voiced && vscore >= VOICENESS_MIN
        };
        self.voiced_run = voiced;
        // Gate-input level probe (throttled ~2 s) — the third of the three level
        // probes (entry/resampled/cleaned live in run_capture). Logs the dbfs
        // NoiseFloor.classify actually receives, the tracked floor, and the
        // voiceness verdict, so the listen path can be compared against
        // test-mic's raw levels and any signal loss between capture and the gate
        // is convicted. Logged even during silence — a gate input far below
        // test-mic's speech level while the user is talking IS the smoking gun.
        if self
            .last_probe
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(1))
        {
            self.last_probe = Some(Instant::now());
            let voiced_run_ms = self.noise_floor.voiced_run_samples() * 1_000
                / u64::from(self.config.sample_rate.max(1));
            info!(
                classify_dbfs = rms_dbfs,
                floor_dbfs = self.noise_floor.floor_dbfs(),
                gate_floor_dbfs = self.noise_floor.gate_floor_dbfs(),
                voiceness = vscore,
                voiceness_min = VOICENESS_MIN,
                energy_voiced,
                voiced,
                voiced_run_ms,
                watchdog_recals = self.noise_floor.recalibrations(),
                "voice gate input probe"
            );
        }
        // One-shot: announce the armed gate the moment calibration completes, so
        // every session's log shows the floor + onset the VAD is running with.
        if !self.armed_logged && self.noise_floor.calibrated() {
            self.armed_logged = true;
            info!(
                floor_dbfs = self.noise_floor.floor_dbfs(),
                gate_floor_dbfs = self.noise_floor.gate_floor_dbfs(),
                onset_dbfs = self.noise_floor.gate_floor_dbfs() + VAD_NOISE_MARGIN_DB,
                voiceness_min = VOICENESS_MIN,
                "voice gate armed"
            );
        }
        (voiced, rms_dbfs, self.noise_floor.floor_dbfs())
    }

    /// Run the conversation loop until `cancel` fires or the capture channel
    /// closes. Frames are 16 kHz mono `i16` (echo-cancelled mic).
    pub async fn run(&mut self, mut frames: mpsc::Receiver<Vec<i16>>, cancel: CancellationToken) {
        // Pre-render the fixed ack set through the slow tier (off-thread) so
        // acks speak in the answer's own voice from the first turn that the
        // warm beats; fast-tier fallback covers the race. Skipped in listen-only
        // mode — that path never speaks, so it does zero synthesis.
        if !self.config.listen_only {
            let warm: Vec<String> = ACK_SHORT_POOL
                .iter()
                .chain(ACK_LONG_POOL.iter())
                .chain(CLARIFICATION_POOL.iter())
                .map(|s| s.to_string())
                .collect();
            self.tts.spawn_warm_acks(warm);
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
        // WEFT-615: set when `utt` was seeded from a playback-window
        // interrupt's captured onset frames (see the `talk_turn` handoff
        // below) rather than an ordinary idle-mic onset. Threaded onto the
        // next `FinalizedTurn` so `talk_turn` can scope the bare-stop
        // swallow to it; reset (taken) at every finalize.
        let mut from_window_interrupt = false;
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
                            let captured_from_window = std::mem::take(&mut from_window_interrupt);
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
                                    from_window_interrupt: captured_from_window,
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
                                    // Talk mode: decode inline, then reply. A
                                    // non-empty return is the captured onset of a
                                    // WEFT-615 playback-window interrupt (see
                                    // `speak_answer_windowed`/`listen_window`):
                                    // seed it straight into `utt` so the NEXT
                                    // `frames.recv()` iteration continues this
                                    // utterance exactly like an ordinary in-
                                    // progress capture — no preroll needed, this
                                    // audio already IS the onset.
                                    let pending = self.talk_turn(turn, &mut frames, &cancel).await;
                                    if !pending.is_empty() {
                                        voiced_samples = pending.len();
                                        // The endpointer never saw these frames
                                        // (`listen_window` consumed them directly,
                                        // bypassing `self.endpointer.observe`), so
                                        // without this catch-up call its `active`
                                        // flag stays false and the silence that
                                        // follows would never finalize the turn —
                                        // `SemanticEndpointer::observe` treats a
                                        // silence frame as a no-op while no turn is
                                        // in progress. One voiced replay call opens
                                        // it, exactly as if these frames had arrived
                                        // through the normal capture path.
                                        let _ = self.endpointer.observe(&pending, true, "").await;
                                        utt = pending;
                                        from_window_interrupt = true;
                                    }
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
    /// [`Decoder`]), then ack → (race the grounded answer against the filler
    /// gate) → speak with barge-in / inter-sentence windows. Talk mode
    /// decodes inline (it must produce the reply anyway); listen-only runs
    /// the same decode off-loop in the worker.
    ///
    /// Returns the captured onset frames of a WEFT-615 playback-window
    /// interrupt (see [`speak_answer_windowed`](Self::speak_answer_windowed)),
    /// for `run()` to seed straight into the next in-progress utterance;
    /// empty in every other case (normal completion, continuous barge-in,
    /// any of the existing gate/error early-outs).
    async fn talk_turn(
        &mut self,
        turn: FinalizedTurn,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) -> Vec<i16> {
        let from_window_interrupt = turn.from_window_interrupt;
        let Some(decoded) = self.decoder.decode_and_emit(turn).await else {
            return Vec::new(); // empty transcript / STT error — turn dropped (UserTurn not emitted)
        };
        let DecodedTurn {
            text,
            speaker_ctx,
            analysis,
        } = decoded;

        // WEFT-615: an utterance whose ONSET was captured by a playback-
        // window interrupt gets one extra gate before the usual non-lexical/
        // unclear checks — a bare STOP phrase means the user just wanted
        // silence, not a new turn: no ack, no LLM call, no spoken reply, and
        // (unlike the gates below) no `TurnGated` follow-up speech either.
        // Scoped strictly to `from_window_interrupt` turns — the same words
        // said while idle are an ordinary turn (mirrors interrupt_router.rs's
        // timing-axis rule; this crate cannot depend on that crate, so
        // `WINDOW_STOP_PHRASES` is a local, explicitly-synced mirror of its
        // `STOP_PHRASES`).
        if from_window_interrupt && is_bare_stop_phrase(&text) {
            debug!(transcript = %text, "talk-mode window interrupt: bare stop — swallowed");
            self.observer.observe(ConversationEvent::TurnGated {
                reason: "playback_stop".into(),
            });
            return Vec::new();
        }

        // The non-lexical / unclear gate (WEFT-659): decide BEFORE any of the
        // reply slot runs — no ack, no filler, no `VoiceLlm::complete`, no
        // spoken reply. The turn already finalized/recorded above regardless.
        match should_engage_brain(&analysis, &text) {
            Engage::Engage => {}
            Engage::GateNonLexical => {
                debug!(transcript = %text, "talk-mode turn gated: non-lexical");
                self.observer.observe(ConversationEvent::TurnGated {
                    reason: "non_lexical".into(),
                });
                return Vec::new();
            }
            Engage::GateUnclear => {
                debug!(transcript = %text, "talk-mode turn gated: unclear");
                self.observer.observe(ConversationEvent::TurnGated {
                    reason: "unclear".into(),
                });
                let clarification = pick_clarification(self.clarification_counter);
                self.clarification_counter = self.clarification_counter.wrapping_add(1);
                speak_while_draining(
                    self.tts.speak_ack(&clarification, self.sink.clone(), cancel.clone()),
                    frames,
                    cancel,
                )
                .await;
                return Vec::new();
            }
        }

        // Fast ack = Speculative spoken node (6.4 fast layer covers latency).
        // Rotated through the pool (WEFT-658 hot-mic feedback: talk mode said
        // the same "Okay, one sec." on every turn) — the counter advances
        // once per turn, so back-to-back picks from the same pool never
        // repeat verbatim.
        let ack = pick_ack(&text, self.ack_counter);
        self.ack_counter = self.ack_counter.wrapping_add(1);
        self.observer
            .observe(ConversationEvent::SpeculativeReply { ack: ack.clone() });

        // Speak the ack RIGHT AWAY instead of waiting for the LLM, so it
        // actually covers the generation latency rather than playing
        // back-to-back with the answer once both happen to be ready.
        speak_while_draining(
            self.tts.speak_ack(&ack, self.sink.clone(), cancel.clone()),
            frames,
            cancel,
        )
        .await;
        if cancel.is_cancelled() {
            return Vec::new();
        }

        // Grounded answer, shaped short by the spoken-answer policy (6.3) —
        // cloned inputs so the future owns everything and doesn't hold a
        // `self` borrow across the loop below (which also mutates
        // `self.filler_counter` and reads `self.observer`/`self.tts`).
        let policy = self.policy.clone();
        let llm = self.llm.clone();
        let base_system = self.config.base_system.clone();
        let transcript = text.clone();
        let answer_fut =
            async move { policy.answer(llm.as_ref(), &base_system, &transcript, speaker_ctx).await };
        tokio::pin!(answer_fut);

        // Race the answer against the filler gate: if the LLM hasn't
        // returned within FILLER_GATE_DELAY, speak one contextual filler
        // (fast tier, dynamic text) so the wait doesn't read as dead air —
        // this also opens a natural interruption window (WEFT-658). `biased`
        // polls the answer branch first on every wakeup, so a reply that
        // lands at (or just before) the gate is taken over the filler —
        // "skip it if the reply arrives while queued/being decided".
        let mut filler_spoken = false;
        let gate_start = std::time::Instant::now();
        let answer_result = loop {
            tokio::select! {
                biased;
                res = &mut answer_fut => break res,
                _ = tokio::time::sleep(FILLER_GATE_DELAY), if !filler_spoken => {
                    filler_spoken = true;
                    if should_speak_filler(gate_start.elapsed(), false) {
                        let filler = pick_filler(&text, self.filler_counter);
                        self.filler_counter = self.filler_counter.wrapping_add(1);
                        self.observer.observe(ConversationEvent::SpeculativeFiller {
                            text: filler.clone(),
                        });
                        speak_while_draining(
                            self.tts.speak_filler(&filler, self.sink.clone(), cancel.clone()),
                            frames,
                            cancel,
                        )
                        .await;
                    }
                }
            }
        };
        if cancel.is_cancelled() {
            return Vec::new();
        }

        let answer = match answer_result {
            Ok(a) if !a.trim().is_empty() => a,
            Ok(_) => {
                warn!("talk-mode LLM returned an empty answer; turn dropped");
                return Vec::new();
            }
            Err(e) => {
                warn!(error = %e, "talk-mode LLM failed");
                return Vec::new();
            }
        };
        info!(answer = %answer, "talk-mode committed reply");
        self.observer.observe(ConversationEvent::CommittedReply {
            answer: answer.clone(),
        });

        // Stream the answer (6.4) sentence-by-sentence with inter-sentence
        // listening windows (WEFT-615) plus continuous barge-in monitoring
        // per sentence (`barge_in_enabled`, headphones). The ack (and any
        // filler) have already been spoken by now.
        self.speak_answer_windowed(&answer, frames, cancel).await
    }

    /// Stream the grounded answer (WEFT-615): split into sentences
    /// ([`super::tts::split_sentences`]) and speak them one at a time via
    /// [`speak_answer_with_barge_in`](Self::speak_answer_with_barge_in), so
    /// each sentence keeps that call's exact tier-selection/fallback
    /// semantics (SLOW-tier prebuffer-then-play, re-render through FAST on
    /// zero audio) — just applied per sentence instead of to the whole
    /// answer. Between sentences (once the sink is confirmed drained to
    /// silence by that call) an inter-sentence [`listen_window`](Self::listen_window)
    /// opens for `sentence_window_ms`. `0` or a single-sentence answer skips
    /// all of this and calls [`speak_answer_with_barge_in`](Self::speak_answer_with_barge_in)
    /// once with the whole text — byte-identical to the pre-WEFT-615 path
    /// (nothing to interrupt between one sentence, and disabling windows
    /// should disable every seam this feature adds).
    ///
    /// Returns the captured onset frames of a window interrupt for `run()`
    /// to seed into the next in-progress utterance (see the `talk_turn`
    /// caller); empty otherwise (normal completion, continuous barge-in, or
    /// session cancel — all of which already left `Interrupted`/nothing
    /// further to say).
    async fn speak_answer_windowed(
        &self,
        answer: &str,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) -> Vec<i16> {
        let sentences = super::tts::split_sentences(answer);
        if self.config.sentence_window_ms == 0 || sentences.len() <= 1 {
            self.speak_answer_with_barge_in(answer, frames, cancel).await;
            return Vec::new();
        }
        let last = sentences.len() - 1;
        for (i, sentence) in sentences.iter().enumerate() {
            if cancel.is_cancelled() {
                return Vec::new();
            }
            let interrupted = self
                .speak_answer_with_barge_in(sentence, frames, cancel)
                .await;
            if interrupted || cancel.is_cancelled() || i == last {
                return Vec::new();
            }
            if let Some(onset) = self.listen_window(frames, cancel).await {
                debug!("talk-mode inter-sentence window onset — stopping remaining sentences");
                self.audio.flush(); // AEC render reference, mirrors the continuous barge path
                self.observer.observe(ConversationEvent::Interrupted);
                return onset;
            }
        }
        Vec::new()
    }

    /// Open one inter-sentence listening window (WEFT-615): the sink is
    /// already silent (the preceding [`speak_answer_with_barge_in`](Self::speak_answer_with_barge_in)
    /// call drained it), so this is echo-free BY PHYSICS — no AEC and no
    /// [`BARGE_IN_GRACE_MS`] convergence period needed (nothing is playing
    /// to converge past). Consumes mic frames against the SAME frozen barge
    /// gate the continuous monitor uses (the noise floor is NOT advanced
    /// here — same freeze rationale as [`speak_answer_with_barge_in`](Self::speak_answer_with_barge_in)).
    ///
    /// Returns the captured frames of a sustained voiced onset
    /// ([`TalkModeConfig::barge_in_frames`] consecutive frames over the
    /// gate) — the caller hands these to `run()`'s normal capture machinery
    /// so the interrupting utterance is captured from its true onset, not
    /// re-detected from scratch. `None` if `sentence_window_ms` elapses (or
    /// `cancel` fires, or the capture channel closes) with no sustained
    /// onset — the caller proceeds to the next sentence.
    async fn listen_window(
        &self,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) -> Option<Vec<i16>> {
        let barge_gate = (self.noise_floor.floor_dbfs() + BARGE_IN_MARGIN_DB)
            .max(self.config.vad_threshold_dbfs);
        let deadline = tokio::time::sleep(Duration::from_millis(self.config.sentence_window_ms));
        tokio::pin!(deadline);
        let mut voiced_run: u32 = 0;
        let mut captured: Vec<i16> = Vec::new();
        loop {
            tokio::select! {
                _ = &mut deadline => return None,
                _ = cancel.cancelled() => return None,
                maybe = frames.recv() => {
                    match maybe {
                        Some(frame) => {
                            if EnergyVad::rms_dbfs(&frame) >= barge_gate {
                                voiced_run += 1;
                                captured.extend_from_slice(&frame);
                                if voiced_run >= self.config.barge_in_frames {
                                    return Some(captured);
                                }
                            } else {
                                voiced_run = 0;
                                captured.clear();
                            }
                        }
                        None => return None, // capture channel closed
                    }
                }
            }
        }
    }

    /// Stream one sentence (or, when windows are disabled/inapplicable, the
    /// whole answer text) of the grounded answer, monitoring for CONTINUOUS
    /// barge-in: a sustained voiced run during playback is a barge-in →
    /// cancel TTS (flushes the sink) + flush the AEC render reference + emit
    /// `Interrupted`. Gated behind [`TalkModeConfig::barge_in_enabled`]
    /// (headphones only — see [`TalkModeConfig::sentence_window_ms`] for the
    /// speaker-safe default). See [`speak_while_draining`] and WEFT-658's
    /// "no new interrupt machinery" constraint.
    ///
    /// Returns `true` if this call's playback was interrupted (continuous
    /// barge-in, session cancel, or the capture channel closing) — the
    /// [`speak_answer_windowed`](Self::speak_answer_windowed) caller stops
    /// there rather than starting the next sentence; `false` on normal
    /// completion.
    async fn speak_answer_with_barge_in(
        &self,
        answer: &str,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) -> bool {
        // Snapshot the voiced gate for the playback window: the noise-floor
        // tracker must NOT learn from frames while the bot is speaking (its
        // own echo would lift the floor), so barge-in uses a frozen, steeper
        // threshold instead of `self.voiced` (see BARGE_IN_MARGIN_DB).
        let barge_gate = (self.noise_floor.floor_dbfs() + BARGE_IN_MARGIN_DB)
            .max(self.config.vad_threshold_dbfs);
        let speak_started = std::time::Instant::now();
        let turn_cancel = cancel.child_token();
        let speak = self
            .tts
            .speak_answer(answer, self.sink.clone(), turn_cancel.clone());
        tokio::pin!(speak);

        let mut voiced_run: u32 = 0;
        let mut interrupted = false;
        loop {
            tokio::select! {
                res = &mut speak => {
                    if let Err(e) = res {
                        warn!(error = %e, "talk-mode TTS error");
                    }
                    break; // finished speaking, uninterrupted
                }
                _ = cancel.cancelled() => {
                    interrupted = true;
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
                                    return true;
                                }
                            } else {
                                voiced_run = 0;
                            }
                        }
                        None => {
                            turn_cancel.cancel();
                            let _ = (&mut speak).await;
                            return true;
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
        interrupted
    }
}

/// Local mirror of `clawft_service_agent::interrupt_router::STOP_PHRASES`
/// (crates/clawft-service-agent/src/interrupt_router.rs) — this crate
/// (clawft-channels) cannot depend on clawft-service-agent, so the bare-stop
/// lexicon used to swallow a WEFT-615 playback-window interrupt is
/// duplicated here. **KEEP IN SYNC**: a change to the source list should be
/// mirrored here (and vice versa); see the matching note on
/// `interrupt_router::STOP_PHRASES`.
const WINDOW_STOP_PHRASES: &[&str] = &[
    "stop",
    "hold on",
    "hold up",
    "wait",
    "wait wait",
    "cancel",
    "cancel that",
    "never mind",
    "nevermind",
    "forget it",
    "scratch that",
    "abort",
    "halt",
    "belay that",
    "quit",
    "no wait",
    "stop stop",
];

/// Whether `text`, once normalized (lowercased, whitespace-collapsed,
/// trailing sentence punctuation stripped — mirrors
/// `interrupt_router::normalize`), IS (in full) one of
/// [`WINDOW_STOP_PHRASES`]. Used only to swallow a WEFT-615 playback-window
/// interrupt's finalized utterance. Unlike the router's `stop_remainder`,
/// there is no "stop that continues into a correction/request" branch here —
/// a window interrupt is either a bare stop (swallowed) or an ordinary turn
/// (proceeds); there is no Refine path in a playback interrupt.
fn is_bare_stop_phrase(text: &str) -> bool {
    let lowered = text.trim().to_lowercase();
    let collapsed = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = collapsed.trim_end_matches(['.', '!', '?']).trim();
    WINDOW_STOP_PHRASES.contains(&normalized)
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

/// Short-ask ack pool (WEFT-658: talk mode said "One sec." on every turn —
/// this rotates it). `ACK_SHORT` is kept as the first entry so existing
/// callers of the single-string [`contextual_ack`] see no change; the live
/// talk loop uses [`pick_ack`] instead, which rotates through this pool.
/// Closed set (no transcript interpolation) so [`DualLayerTts`] can
/// pre-render every entry through the SLOW tier at session start.
pub const ACK_SHORT_POOL: &[&str] = &[ACK_SHORT, "Mm-hm.", "Okay.", "Sure — sec.", "Got it."];
/// Long-ask ack pool — same closed-set / pre-render contract as
/// [`ACK_SHORT_POOL`].
pub const ACK_LONG_POOL: &[&str] = &[
    ACK_LONG,
    "Okay, let me think.",
    "Good question — one moment.",
    "Alright, let me look.",
];

/// Pick an ack for `transcript`, rotating through the short/long pool (the
/// short-vs-long heuristic is unchanged from [`contextual_ack`]) so
/// consecutive turns don't repeat the same line. `counter` is a
/// monotonically incrementing per-controller turn count (see
/// [`TalkModeController::talk_turn`]) — since it advances by exactly one
/// between calls, indexing `pool[counter % pool.len()]` guarantees the
/// immediately-previous pick from either pool is never repeated verbatim
/// (deterministic, no RNG, easy to test).
pub fn pick_ack(transcript: &str, counter: u32) -> String {
    let words = transcript.split_whitespace().count();
    let pool: &[&str] = if words <= 3 { ACK_SHORT_POOL } else { ACK_LONG_POOL };
    pool[counter as usize % pool.len()].to_string()
}

/// Await `speak` (an ack or filler TTS render) while draining mic frames
/// concurrently, so the capture channel doesn't back up during the short
/// playback (mirrors the answer path's frame consumption — unconsumed
/// frames would otherwise flood "frame channel full" warnings). No barge-in
/// monitoring here: these are brief utterances, and barge-in coverage stays
/// on the long answer path (see
/// [`TalkModeController::speak_answer_with_barge_in`]) per WEFT-658's "no
/// new interrupt machinery" constraint.
async fn speak_while_draining<F>(
    speak: F,
    frames: &mut mpsc::Receiver<Vec<i16>>,
    cancel: &CancellationToken,
) where
    F: std::future::Future<Output = Result<(), super::types::VoiceError>>,
{
    tokio::pin!(speak);
    loop {
        tokio::select! {
            res = &mut speak => {
                if let Err(e) = res {
                    warn!(error = %e, "talk-mode TTS error");
                }
                break;
            }
            _ = cancel.cancelled() => break,
            maybe = frames.recv() => {
                if maybe.is_none() {
                    break; // capture channel closed
                }
            }
        }
    }
}

/// Stopwords/question-scaffolding filtered out when extracting a short
/// subject phrase from a transcript for the contextual filler. Deliberately
/// small and keyword-simple (not NLP) — just enough to drop "what is" /
/// "can you tell me about" / greetings and leave the content words.
const SUBJECT_STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "am", "be", "been", "what", "who", "when",
    "where", "why", "how", "which", "can", "could", "would", "should", "will", "do", "does",
    "did", "you", "me", "please", "tell", "explain", "about", "know", "think", "i", "to", "of",
    "for", "on", "in", "at", "with", "and", "now", "hi", "hello", "hey", "yo", "thanks", "thank",
    "ok", "okay",
];

/// Extract a short subject phrase (≤4 content words) from a transcript for
/// the contextual filler ("Let me look at {subject}."). Strips
/// stopwords/question scaffolding and keeps the first content words.
/// Returns `None` when nothing survives (greeting-only / all-stopword
/// utterances) — the filler falls back to [`FILLER_FALLBACK_POOL`].
pub fn extract_subject(transcript: &str) -> Option<String> {
    let content: Vec<&str> = transcript
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\''))
        .filter(|w| !w.is_empty() && !SUBJECT_STOPWORDS.contains(&w.to_lowercase().as_str()))
        .take(4)
        .collect();
    if content.is_empty() {
        None
    } else {
        Some(content.join(" "))
    }
}

/// Filler templates spoken when the grounded answer hasn't landed within
/// [`FILLER_GATE_DELAY`] — dynamic text (the extracted subject is spliced
/// in), so unlike the closed ack pools it is always rendered on the fly
/// through the FAST tier rather than pre-rendered/cached.
const FILLER_TEMPLATES: &[&str] = &[
    "Let me look at {subject} for a second.",
    "Okay — checking {subject}.",
    "Thinking about {subject}…",
];
/// Fallback filler pool for utterances with no extractable subject (short
/// greetings, all-stopword asks).
const FILLER_FALLBACK_POOL: &[&str] = &["Give me a moment.", "Almost there.", "Still with you."];

/// Clarification pool spoken when [`Engage::GateUnclear`] fires (WEFT-659):
/// the STT confidence/SNR combination says the transcript is unreliable, so
/// rather than answer garbage the turn asks the user to repeat themselves.
/// Closed set (no transcript interpolation), same pre-render contract as
/// [`ACK_SHORT_POOL`] — added to the warm set in
/// [`TalkModeController::run`].
pub const CLARIFICATION_POOL: &[&str] = &[
    "Sorry, I didn't catch that — say it again?",
    "I lost you there, one more time?",
    "Didn't quite hear that — could you repeat it?",
];

/// Pick a clarification for the unclear gate, rotating like [`pick_ack`] so
/// consecutive gated turns don't repeat the same line.
pub fn pick_clarification(counter: u32) -> String {
    CLARIFICATION_POOL[counter as usize % CLARIFICATION_POOL.len()].to_string()
}

/// How long to wait after the ack for the grounded answer before speaking a
/// filler (WEFT-658: "buys the (slow) brain thinking time and opens natural
/// interruption windows").
const FILLER_GATE_DELAY: Duration = Duration::from_millis(1_500);

/// Pick a contextual filler for `transcript`, rotating like [`pick_ack`].
/// Splices the extracted subject into a template, or falls back to a
/// subject-less variant when nothing survives extraction.
pub fn pick_filler(transcript: &str, counter: u32) -> String {
    match extract_subject(transcript) {
        Some(subject) => {
            let idx = counter as usize % FILLER_TEMPLATES.len();
            FILLER_TEMPLATES[idx].replace("{subject}", &subject)
        }
        None => {
            let idx = counter as usize % FILLER_FALLBACK_POOL.len();
            FILLER_FALLBACK_POOL[idx].to_string()
        }
    }
}

/// The pure filler gate decision: speak a filler only once the grounded
/// answer's wait has crossed [`FILLER_GATE_DELAY`] AND it still hasn't
/// arrived. Kept as a standalone, synchronously-testable function per
/// WEFT-658 — the timing itself (the actual sleep/select race) lives in
/// [`TalkModeController::talk_turn`]; this is the rule it enforces, so it
/// can be unit-tested without spinning up the async path.
pub fn should_speak_filler(elapsed: Duration, reply_ready: bool) -> bool {
    !reply_ready && elapsed >= FILLER_GATE_DELAY
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_bare_stop_phrase (WEFT-615 window-interrupt swallow lexicon) ──

    #[test]
    fn bare_stop_phrases_match_canonical_entries() {
        for text in ["stop", "wait", "cancel", "never mind", "hold on", "abort"] {
            assert!(is_bare_stop_phrase(text), "{text} must match");
        }
    }

    #[test]
    fn bare_stop_match_is_case_and_punctuation_insensitive() {
        assert!(is_bare_stop_phrase("Stop."));
        assert!(is_bare_stop_phrase("  WAIT!  "));
        assert!(is_bare_stop_phrase("Never Mind?"));
    }

    #[test]
    fn content_utterances_do_not_match() {
        for text in [
            "what's the weather today",
            "stop the car please", // continues past the bare phrase — not a match
            "stopwatch feature", // must not match on a mere prefix
            "",
        ] {
            assert!(!is_bare_stop_phrase(text), "{text} must not match");
        }
    }

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

    // ── pick_ack (WEFT-658 pool rotation) ──────────────────────────────

    #[test]
    fn pick_ack_preserves_the_short_long_heuristic() {
        assert!(ACK_SHORT_POOL.contains(&pick_ack("hi", 0).as_str()));
        assert!(ACK_SHORT_POOL.contains(&pick_ack("what time", 3).as_str()));
        assert!(ACK_SHORT_POOL.contains(&pick_ack("quick question", 7).as_str()));
        assert!(ACK_LONG_POOL.contains(&pick_ack("tell me about Puyo please", 0).as_str()));
        assert!(ACK_LONG_POOL.contains(&pick_ack("can you explain the thing now", 2).as_str()));
    }

    #[test]
    fn pick_ack_never_repeats_the_immediately_previous_pick() {
        // Consecutive turns (counter advancing by 1 each time) — regardless
        // of which pool a given turn lands in, back-to-back picks must
        // differ (the pools are disjoint strings, and same-pool neighbors
        // land on different indices since the counter always advances by 1).
        let mut prev: Option<String> = None;
        for i in 0..20 {
            let text = if i % 3 == 0 { "hi" } else { "tell me a long story please" };
            let ack = pick_ack(text, i);
            if let Some(p) = &prev {
                assert_ne!(*p, ack, "immediate repeat at counter {i}");
            }
            prev = Some(ack);
        }
    }

    #[test]
    fn pick_ack_cycles_through_every_pool_entry() {
        let picks: std::collections::HashSet<String> = (0..ACK_LONG_POOL.len() as u32)
            .map(|i| pick_ack("tell me about the weather today please", i))
            .collect();
        assert_eq!(picks.len(), ACK_LONG_POOL.len(), "a full cycle visits every entry once");
    }

    // ── extract_subject / pick_filler ──────────────────────────────────

    #[test]
    fn extract_subject_strips_question_scaffolding() {
        assert_eq!(extract_subject("tell me about Puyo please"), Some("Puyo".to_string()));
        assert_eq!(
            extract_subject("what is the capital of France"),
            Some("capital France".to_string())
        );
        assert_eq!(
            extract_subject("can you explain the thing now"),
            Some("thing".to_string())
        );
    }

    #[test]
    fn extract_subject_none_for_greetings_and_all_stopwords() {
        assert_eq!(extract_subject("hi"), None);
        assert_eq!(extract_subject("thanks"), None);
        assert_eq!(extract_subject("can you please"), None);
    }

    #[test]
    fn pick_filler_uses_subject_template_when_available() {
        let f = pick_filler("tell me about Puyo please", 0);
        assert_eq!(f, "Let me look at Puyo for a second.");
        let f2 = pick_filler("tell me about Puyo please", 1);
        assert_eq!(f2, "Okay — checking Puyo.");
    }

    #[test]
    fn pick_filler_falls_back_when_no_subject() {
        let f = pick_filler("hi", 0);
        assert!(FILLER_FALLBACK_POOL.contains(&f.as_str()));
    }

    #[test]
    fn pick_filler_rotates_without_immediate_repeat() {
        let mut prev: Option<String> = None;
        for i in 0..10 {
            let f = pick_filler("tell me about Puyo please", i);
            if let Some(p) = &prev {
                assert_ne!(*p, f, "immediate repeat at counter {i}");
            }
            prev = Some(f);
        }
    }

    // ── should_speak_filler gate ────────────────────────────────────────

    #[test]
    fn should_speak_filler_waits_for_the_gate() {
        assert!(!should_speak_filler(Duration::from_millis(500), false), "too early");
        assert!(!should_speak_filler(Duration::from_millis(1_499), false), "still too early");
        assert!(
            should_speak_filler(Duration::from_millis(1_500), false),
            "gate elapsed, reply not ready"
        );
        assert!(
            should_speak_filler(Duration::from_millis(3_000), false),
            "well past the gate, reply not ready"
        );
    }

    #[test]
    fn should_speak_filler_skips_when_the_reply_already_arrived() {
        // "if the reply arrives while the filler is queued/being decided,
        // skip it" (WEFT-658) — modeled by reply_ready=true regardless of
        // elapsed time.
        assert!(!should_speak_filler(Duration::from_millis(1_500), true));
        assert!(!should_speak_filler(Duration::from_millis(5_000), true));
    }

    // ── pick_clarification (WEFT-659 unclear-gate pool rotation) ────────

    #[test]
    fn pick_clarification_rotates_without_immediate_repeat() {
        let mut prev: Option<String> = None;
        for i in 0..10 {
            let c = pick_clarification(i);
            if let Some(p) = &prev {
                assert_ne!(*p, c, "immediate repeat at counter {i}");
            }
            prev = Some(c);
        }
    }

    #[test]
    fn pick_clarification_cycles_through_every_entry() {
        let picks: std::collections::HashSet<String> =
            (0..CLARIFICATION_POOL.len() as u32).map(pick_clarification).collect();
        assert_eq!(picks.len(), CLARIFICATION_POOL.len(), "a full cycle visits every entry once");
    }
}

/// WEFT-659: the reply-slot gate — `utterance_clarity` calibration and the
/// `should_engage_brain` decision tree it feeds.
#[cfg(test)]
mod gate_tests {
    use super::*;
    use super::super::analysis::{
        AudioAnalysis, EmotionAnalysis, EndpointAnalysis, ParalinguisticClass,
        ParalinguisticsAnalysis, ProsodyAnalysis, SpeakerAnalysis, SttAnalysis, SttPath,
        VoiceAnalysis,
    };

    /// Build a `VoiceAnalysis` exercising only the fields `utterance_clarity`
    /// / `should_engage_brain` read; everything else defaulted.
    fn analysis_with(
        conf_mean: f32,
        snr_db: f32,
        noise_floor_converged: bool,
        class: ParalinguisticClass,
    ) -> VoiceAnalysis {
        VoiceAnalysis::new(
            SttAnalysis {
                model: "test".into(),
                latency_ms: 0,
                path: SttPath::Native,
                tokens: vec![],
                token_conf_mean: conf_mean,
                token_conf_min: conf_mean,
            },
            EndpointAnalysis {
                completion_prob: 1.0,
                source: "heuristic".into(),
                silence_tail_ms: 0,
                latency_ms: 0,
            },
            SpeakerAnalysis::default(),
            AudioAnalysis {
                snr_db,
                noise_floor_converged,
                ..AudioAnalysis::default()
            },
            ProsodyAnalysis::default(),
            EmotionAnalysis::default(),
            ParalinguisticsAnalysis {
                class,
                ..ParalinguisticsAnalysis::default()
            },
        )
    }

    // ── utterance_clarity calibration (WEFT-659 live-trace cases) ───────

    #[test]
    fn short_low_confidence_answer_is_clear() {
        // "No." — 1 word, conf 0.44, SNR 6.4 dB. Must stay CLEAR.
        let a = analysis_with(0.44, 6.4, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "No."), Clarity::Clear);
    }

    #[test]
    fn high_confidence_question_is_clear() {
        let a = analysis_with(0.98, 16.3, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "What question did you hear?"), Clarity::Clear);
    }

    #[test]
    fn long_low_confidence_utterance_is_unclear() {
        let a = analysis_with(0.45, 10.0, true, ParalinguisticClass::Speech);
        let transcript =
            "this is a twelve word utterance spoken with shaky recognition confidence today";
        assert_eq!(utterance_clarity(&a, transcript), Clarity::Unclear);
    }

    #[test]
    fn live_trace_garbage_stt_is_unclear() {
        // The hot-mic feedback trace: engaged the full brain despite
        // conf_mean 0.44 / conf_min 0.34 / SNR 6.4 dB. Must gate now.
        let a = analysis_with(0.44, 6.4, true, ParalinguisticClass::Speech);
        let transcript = "If that shoes is a some sort of weird gap in those yellow tree store";
        assert_eq!(utterance_clarity(&a, transcript), Clarity::Unclear);
        assert_eq!(should_engage_brain(&a, transcript), Engage::GateUnclear);
    }

    // ── boundaries ────────────────────────────────────────────────────

    #[test]
    fn long_word_count_boundary_below_conf_floor_is_unclear() {
        let a = analysis_with(0.54, 10.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "one two three four"), Clarity::Unclear);
    }

    #[test]
    fn long_word_count_boundary_at_conf_floor_is_clear() {
        let a = analysis_with(0.55, 10.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "one two three four"), Clarity::Clear);
    }

    #[test]
    fn below_long_word_count_threshold_the_long_rule_does_not_apply() {
        // 3 words, low confidence: the long-utterance rule needs >= 4 words,
        // and this clears the absolute + SNR floors, so it stays Clear.
        let a = analysis_with(0.40, 10.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "one two three"), Clarity::Clear);
    }

    #[test]
    fn absolute_confidence_floor_is_unclear_regardless_of_length() {
        let a = analysis_with(0.29, 20.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "hi"), Clarity::Unclear);
    }

    #[test]
    fn absolute_confidence_floor_boundary_is_clear() {
        let a = analysis_with(0.30, 20.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "hi"), Clarity::Clear);
    }

    #[test]
    fn low_snr_converged_low_confidence_is_unclear() {
        let a = analysis_with(0.60, 3.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "can you hear me now"), Clarity::Unclear);
    }

    #[test]
    fn low_snr_unconverged_floor_does_not_apply() {
        // Same confidence/SNR as above, but the noise floor hasn't converged
        // — SNR is unreliable, so this rule must not fire.
        let a = analysis_with(0.60, 3.0, false, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "can you hear me now"), Clarity::Clear);
    }

    #[test]
    fn low_snr_high_confidence_is_clear() {
        let a = analysis_with(0.75, 3.0, true, ParalinguisticClass::Speech);
        assert_eq!(utterance_clarity(&a, "can you hear me now"), Clarity::Clear);
    }

    // ── should_engage_brain gate ─────────────────────────────────────────

    #[test]
    fn clear_speech_engages() {
        let a = analysis_with(0.9, 15.0, true, ParalinguisticClass::Speech);
        assert_eq!(should_engage_brain(&a, "what time is it"), Engage::Engage);
    }

    #[test]
    fn unknown_class_still_uses_the_clarity_rule() {
        let a = analysis_with(0.9, 15.0, true, ParalinguisticClass::Unknown);
        assert_eq!(should_engage_brain(&a, "hello there"), Engage::Engage);
    }

    #[test]
    fn backchannel_gates_non_lexical_even_when_confidently_clear() {
        // Non-lexical must win over clarity — a confidently-transcribed
        // "mm-hmm" is still not a question addressed to the assistant.
        let a = analysis_with(0.95, 15.0, true, ParalinguisticClass::BackchannelCandidate);
        assert_eq!(should_engage_brain(&a, "mm"), Engage::GateNonLexical);
    }

    #[test]
    fn laughter_gates_non_lexical() {
        let a = analysis_with(0.95, 15.0, true, ParalinguisticClass::LaughterCandidate);
        assert_eq!(should_engage_brain(&a, ""), Engage::GateNonLexical);
    }

    #[test]
    fn filler_gates_non_lexical() {
        // The live feedback: "Mm." was tagged Filler yet still got a brain
        // reply. Must gate now.
        let a = analysis_with(0.95, 15.0, true, ParalinguisticClass::Filler);
        assert_eq!(should_engage_brain(&a, "Mm."), Engage::GateNonLexical);
    }

    #[test]
    fn unclear_speech_gates_unclear() {
        let a = analysis_with(0.2, 15.0, true, ParalinguisticClass::Speech);
        assert_eq!(should_engage_brain(&a, "garbled nonsense here"), Engage::GateUnclear);
    }
}
