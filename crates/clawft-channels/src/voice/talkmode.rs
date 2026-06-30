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
use tracing::{debug, warn};

use super::policy::{VoiceAnswerPolicy, VoiceLlm};
use super::speaker::{SpeakerEmbedder, SpeakerId, SpeakerRegistry};
use super::stt::{SttBackend, Utterance};
use super::tts::{DualLayerTts, TtsSink};
use super::turn::{EndpointModel, SemanticEndpointer, TurnDecision};
use super::vad::EnergyVad;

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
}

impl Default for TalkModeConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            vad_threshold_dbfs: -45.0,
            barge_in_frames: 3,
            default_speaker_name: "unknown speaker".into(),
            base_system: String::new(),
        }
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
        }
    }

    /// Borrow the speaker registry (e.g. to persist it after a session).
    pub fn registry(&self) -> &SpeakerRegistry {
        &self.registry
    }

    fn voiced(&self, frame: &[i16]) -> bool {
        EnergyVad::rms_dbfs(frame) >= self.config.vad_threshold_dbfs
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
                            if !captured.is_empty() {
                                self.handle_turn(captured, &mut frames, &cancel).await;
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

        // STT (6.2).
        let text = match self.stt.transcribe(&utt).await {
            Ok(t) if !t.trim().is_empty() => t,
            Ok(_) => return,
            Err(e) => {
                warn!(error = %e, "talk-mode STT failed");
                return;
            }
        };

        // Speaker attribution (6.6) → private context, never spoken.
        let (speaker_id, speaker_name, speaker_ctx) = self.attribute_speaker(&utt).await;

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
            Ok(_) => return,
            Err(e) => {
                warn!(error = %e, "talk-mode LLM failed");
                return;
            }
        };
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
                            if self.voiced(&frame) {
                                voiced_run += 1;
                                if voiced_run >= self.config.barge_in_frames {
                                    debug!("talk-mode barge-in detected");
                                    self.audio.flush();          // AEC render reference
                                    turn_cancel.cancel();        // TTS producer + sink flush
                                    self.observer.observe(ConversationEvent::Interrupted);
                                    let _ = (&mut speak).await;   // let it unwind
                                    break;
                                }
                            } else {
                                voiced_run = 0;
                            }
                        }
                        None => {
                            turn_cancel.cancel();
                            let _ = (&mut speak).await;
                            break;
                        }
                    }
                }
            }
        }
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
