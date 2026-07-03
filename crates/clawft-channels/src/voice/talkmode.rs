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

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::policy::{VoiceAnswerPolicy, VoiceLlm};
use super::speaker::{SpeakerEmbedder, SpeakerId, SpeakerRegistry};
use super::stt::{SttBackend, Utterance};
use super::tts::{DualLayerTts, TtsSink};
use super::turn::{EndpointModel, SemanticEndpointer, TurnDecision};
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationEvent {
    /// A finalized user utterance committed as a `Follows` `CausalNode`.
    UserTurn {
        /// Transcript text.
        text: String,
        /// Attributed speaker id (if speaker ID is enabled and matched).
        speaker: Option<SpeakerId>,
        /// Attributed speaker name (private context, never spoken).
        speaker_name: Option<String>,
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

/// The Talk-Mode controller. Generic over the endpoint model so a smart-turn
/// ONNX model or the heuristic default both fit.
pub struct TalkModeController<M: EndpointModel> {
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
        Self {
            endpointer,
            stt,
            embedder,
            registry,
            policy,
            llm,
            tts,
            sink,
            audio,
            observer,
            config,
            noise_floor: NoiseFloor::new(VAD_NOISE_MARGIN_DB),
        }
    }

    /// Borrow the speaker registry (e.g. to persist it after a session).
    pub fn registry(&self) -> &SpeakerRegistry {
        &self.registry
    }

    /// Best-effort save of the speaker registry to the configured store
    /// (enrollments + renames survive sessions). No-op when unconfigured.
    fn persist_registry(&self) {
        if let Some(path) = &self.config.speaker_store {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = self.registry.save(path) {
                warn!(error = %e, path = %path.display(), "speaker registry save failed");
            }
        }
    }

    fn voiced(&mut self, frame: &[i16]) -> bool {
        let dbfs = EnergyVad::rms_dbfs(frame);
        let adaptive = self.noise_floor.observe(dbfs);
        dbfs >= adaptive.max(self.config.vad_threshold_dbfs)
    }

    /// Run the conversation loop until `cancel` fires or the capture channel
    /// closes. Frames are 16 kHz mono `i16` (echo-cancelled mic).
    pub async fn run(&mut self, mut frames: mpsc::Receiver<Vec<i16>>, cancel: CancellationToken) {
        let mut utt: Vec<i16> = Vec::new();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                maybe = frames.recv() => {
                    let Some(frame) = maybe else { break };
                    let voiced = self.voiced(&frame);
                    if voiced {
                        utt.extend_from_slice(&frame);
                    }
                    match self.endpointer.observe(&frame, voiced, "").await {
                        TurnDecision::Continue => {}
                        TurnDecision::Finalize => {
                            let captured = std::mem::take(&mut utt);
                            // Guard against ghost turns: noise blips and the
                            // reverb tail of our own playback can cross the
                            // gate for a few frames and finalize a sub-word
                            // "utterance" — each of which would earn an ack.
                            // Real one-word turns run well past 250 ms.
                            let min_samples =
                                (self.config.sample_rate as usize) * MIN_TURN_MS / 1_000;
                            if captured.len() >= min_samples {
                                self.handle_turn(captured, &mut frames, &cancel).await;
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
    }

    /// One full turn: STT → speaker → grounded answer → ack+answer with barge-in.
    async fn handle_turn(
        &mut self,
        samples: Vec<i16>,
        frames: &mut mpsc::Receiver<Vec<i16>>,
        cancel: &CancellationToken,
    ) {
        let sr = self.config.sample_rate;
        let utt = Utterance {
            samples,
            sample_rate: sr,
        };
        info!(
            samples = utt.samples.len(),
            ms = utt.samples.len() as u64 * 1000 / sr.max(1) as u64,
            "talk-mode end-of-turn utterance captured"
        );

        // STT (6.2).
        let text = match self.stt.transcribe(&utt).await {
            Ok(t) if !t.trim().is_empty() => t,
            Ok(_) => {
                info!("talk-mode STT returned an empty transcript; turn dropped");
                return;
            }
            Err(e) => {
                warn!(error = %e, "talk-mode STT failed");
                return;
            }
        };
        info!(transcript = %text, "talk-mode user turn");

        // Speaker attribution (6.6) → private context, never spoken.
        let (speaker_id, speaker_name, speaker_ctx) = self.attribute_speaker(&utt).await;

        // Spoken self-enrollment (voicelab parity): a voice naming itself
        // ("my name is X" / "call me X") upgrades its placeholder name on
        // the spot. Deliberately explicit phrases only — the loose "I'm X"
        // would mis-enroll chit-chat.
        if let (Some(id), Some(name)) = (&speaker_id, extract_spoken_name(&text))
            && self.registry.rename(id, name.clone())
        {
            info!(speaker = %id, %name, "talk-mode speaker self-enrolled by name");
            self.persist_registry();
        }

        self.observer.observe(ConversationEvent::UserTurn {
            text: text.clone(),
            speaker: speaker_id,
            speaker_name: speaker_name.clone(),
        });

        // Fast ack = Speculative spoken node (6.4 fast layer covers latency).
        let ack = contextual_ack(&text);
        self.observer
            .observe(ConversationEvent::SpeculativeReply { ack: ack.clone() });

        // Grounded answer, shaped short by the spoken-answer policy (6.3).
        let answer = match self
            .policy
            .answer(
                self.llm.as_ref(),
                &self.config.base_system,
                &text,
                speaker_ctx,
            )
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
        // Committed node supersedes the speculative ack.
        self.observer.observe(ConversationEvent::CommittedReply {
            answer: answer.clone(),
        });

        // Speak ack then stream the answer (6.4); monitor for barge-in.
        self.speak_with_barge_in(&ack, &answer, frames, cancel)
            .await;
    }

    async fn attribute_speaker(
        &mut self,
        utt: &Utterance,
    ) -> (Option<SpeakerId>, Option<String>, Option<String>) {
        let Some(embedder) = self.embedder.clone() else {
            return (None, None, None);
        };
        let emb = match embedder.embed(&utt.samples, utt.sample_rate).await {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "speaker embed failed; turn unattributed");
                return (None, None, None);
            }
        };
        let (id, is_new) = self
            .registry
            .identify_or_enroll(&emb, self.config.default_speaker_name.clone());
        let name = self.registry.get(&id).map(|n| n.name.clone());
        if is_new {
            self.observer.observe(ConversationEvent::SpeakerEnrolled {
                id: id.clone(),
                name: name.clone().unwrap_or_default(),
            });
            self.persist_registry();
        }
        let ctx = name
            .as_ref()
            .map(|n| format!("The current speaker is {n} (id {id}). Use this only as private context; never read it aloud."));
        (Some(id), name, ctx)
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
pub fn contextual_ack(transcript: &str) -> String {
    let words: Vec<&str> = transcript.split_whitespace().collect();
    if words.len() <= 3 {
        return "One sec.".to_string();
    }
    // Prefer echoing a capitalized subject word (a proper noun / topic).
    let subject = words
        .iter()
        .rev()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .find(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false));
    match subject {
        Some(s) if !s.is_empty() => format!("{s} — one sec."),
        _ => "Okay, one sec.".to_string(),
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
    fn ack_echoes_capitalized_subject() {
        assert_eq!(
            contextual_ack("tell me about Puyo please"),
            "Puyo — one sec."
        );
        assert_eq!(
            contextual_ack("can you explain the thing now"),
            "Okay, one sec."
        );
    }
}
