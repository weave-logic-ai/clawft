//! Capture-side voice pipeline: cleaned 16 kHz mono frames → VAD/endpoint
//! gate → turn-taking impulses + frame forwarding (ADR-062 Phase 5).
//!
//! Per ADR-062 the voice channel **does not own conversation logic — it
//! emits impulses**. [`CaptureProcessor`] is the dependency-free state
//! machine that turns echo-cancelled mic frames into [`VoiceImpulse`]s
//! (speech onset → floor claim; endpoint fires → end-of-utterance) and
//! forwards the raw frames to the Talk-Mode loop.
//!
//! The split keeps this module free of cpal and clawft-kernel so it is
//! deterministically testable:
//!
//! - the concrete cpal + WebRTC-AEC device path that *produces* the cleaned
//!   frames lives in `clawft-voice-aec` (`AecCaptureStream`);
//! - the [`ImpulseSink`] that *consumes* the impulses is wired to the kernel
//!   `ImpulseQueue` by the talk-mode binder (`clawft-voice-talk`); tests use
//!   a recording sink.
//!
//! ```text
//! cleaned 16k frame ─► CaptureProcessor::push ─┬─► EnergyVad ─► ImpulseSink
//!                                              └─► frames_tx ─► Talk-Mode loop
//! ```

use std::sync::Arc;

use tokio::sync::mpsc;

use super::vad::{EnergyVad, VadEvent};

/// A turn-taking signal emitted by the capture pipeline.
///
/// Each variant maps 1:1 onto a kernel `ImpulseType` turn code (ADR-062 D5)
/// via [`code`](VoiceImpulse::code), so the talk-mode binder can forward it
/// into the real `ImpulseQueue` without this crate depending on
/// clawft-kernel. The channel emits *signals only* — floor arbitration and
/// commit logic belong to the Talk-Mode loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceImpulse {
    /// Speech onset — the speaker claims the floor. Kernel
    /// `ImpulseType::TurnClaim` (byte code `0x50`).
    SpeechStart,
    /// End-of-utterance — the endpoint gate fired; triggers the
    /// Frontier→Committed commit. Kernel `ImpulseType::EndOfUtterance`
    /// (byte code `0x51`).
    EndOfUtterance,
}

impl VoiceImpulse {
    /// Canonical kernel byte code (ADR-062 D5). Lets the talk-mode binder
    /// forward into the ECC `ImpulseQueue` without a kernel dependency here.
    pub fn code(self) -> u8 {
        match self {
            Self::SpeechStart => 0x50,
            Self::EndOfUtterance => 0x51,
        }
    }
}

/// Sink for turn-taking impulses emitted by the capture pipeline.
///
/// The capture thread is real-time; [`emit`](ImpulseSink::emit) MUST NOT
/// block. The kernel-backed implementation (in the talk-mode binder)
/// forwards into the ECC `ImpulseQueue`; tests use a recording sink.
pub trait ImpulseSink: Send + Sync {
    /// Emit `impulse` stamped with monotonic `hlc` — the cumulative captured
    /// sample index, a deterministic clock the kernel queue orders by.
    fn emit(&self, impulse: VoiceImpulse, hlc: u64);
}

/// Capture-side state machine: cleaned 16 kHz mono `i16` frames in,
/// [`VoiceImpulse`]s + forwarded frames out.
///
/// Hold one per capture stream. Drive it by calling [`push`](Self::push)
/// with each echo-cancelled frame the AEC produces.
pub struct CaptureProcessor {
    vad: EnergyVad,
    sink: Arc<dyn ImpulseSink>,
    frames_tx: mpsc::Sender<Vec<i16>>,
    /// Whether the energy VAD's end-of-speech emits an `EndOfUtterance` impulse.
    ///
    /// `true` (default) is the standalone channel-emits-impulses path. Set
    /// `false` (via [`with_emit_eou`](Self::with_emit_eou)) when a downstream
    /// **semantic** endpointer is the authoritative EOU/commit (ADR-062 6.5):
    /// the energy onset stays the coarse early-turn `TurnClaim` tier, but the
    /// final commit is deferred to the semantic model so the two endpointers
    /// **compose** instead of both firing EOU.
    emit_eou: bool,
}

impl CaptureProcessor {
    /// Build a processor from a constructed [`EnergyVad`] (the endpoint
    /// gate), the impulse `sink`, and the `frames_tx` the Talk-Mode loop
    /// drains. Emits both onset and end-of-utterance by default.
    pub fn new(
        vad: EnergyVad,
        sink: Arc<dyn ImpulseSink>,
        frames_tx: mpsc::Sender<Vec<i16>>,
    ) -> Self {
        Self {
            vad,
            sink,
            frames_tx,
            emit_eou: true,
        }
    }

    /// Set whether end-of-speech emits an `EndOfUtterance` impulse. Pass `false`
    /// to defer the EOU/commit to a downstream semantic endpointer while still
    /// emitting the coarse onset `TurnClaim` (ADR-062 6.5 composition).
    pub fn with_emit_eou(mut self, emit_eou: bool) -> Self {
        self.emit_eou = emit_eou;
        self
    }

    /// Push one cleaned 16 kHz mono frame.
    ///
    /// Emits any turn-taking impulses the VAD/endpoint produced, then
    /// forwards the frame to the Talk-Mode loop. Real-time safe: a full or
    /// closed frame channel drops the frame (with a warning) rather than
    /// stalling capture — audio threads must never block.
    pub fn push(&mut self, frame: Vec<i16>) {
        for ev in self.vad.feed(&frame) {
            let (impulse, hlc) = match ev {
                VadEvent::SpeechStart { at_sample } => (VoiceImpulse::SpeechStart, at_sample),
                VadEvent::SpeechEnd { at_sample, .. } => (VoiceImpulse::EndOfUtterance, at_sample),
            };
            // Defer EOU to the semantic endpointer when configured (compose,
            // don't double-fire); onset always flows as the coarse claim tier.
            if impulse == VoiceImpulse::EndOfUtterance && !self.emit_eou {
                continue;
            }
            self.sink.emit(impulse, hlc);
        }
        if self.frames_tx.try_send(frame).is_err() {
            tracing::warn!("voice capture: frame channel full/closed; dropping frame");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        emitted: Mutex<Vec<(VoiceImpulse, u64)>>,
    }
    impl ImpulseSink for RecordingSink {
        fn emit(&self, impulse: VoiceImpulse, hlc: u64) {
            self.emitted.lock().unwrap().push((impulse, hlc));
        }
    }

    fn voiced(len: usize) -> Vec<i16> {
        (0..len)
            .map(|i| if i % 2 == 0 { 3_000 } else { -3_000 })
            .collect()
    }

    #[test]
    fn kernel_codes_match_adr_062() {
        assert_eq!(VoiceImpulse::SpeechStart.code(), 0x50);
        assert_eq!(VoiceImpulse::EndOfUtterance.code(), 0x51);
    }

    #[tokio::test]
    async fn silence_voiced_silence_emits_claim_then_eou_and_forwards_frames() {
        let sink = Arc::new(RecordingSink::default());
        let (tx, mut rx) = mpsc::channel::<Vec<i16>>(256);
        // 16 kHz, -45 dBFS threshold, 300 ms silence tail, 100 ms min, 10 s max.
        let vad = EnergyVad::new(16_000, -45.0, 300, 100, 10_000);
        let mut cap = CaptureProcessor::new(vad, sink.clone(), tx);

        let frame = 1_600usize; // 100 ms @ 16 kHz
        let silence = vec![0i16; frame];

        // 100 ms silence → 300 ms voiced → 500 ms silence (crosses the tail).
        cap.push(silence.clone());
        for _ in 0..3 {
            cap.push(voiced(frame));
        }
        for _ in 0..5 {
            cap.push(silence.clone());
        }

        // Closing the processor closes the frame sender so the drain ends.
        drop(cap);
        let mut frames = 0;
        while rx.recv().await.is_some() {
            frames += 1;
        }
        assert_eq!(frames, 9, "every pushed frame must be forwarded");

        let emitted = sink.emitted.lock().unwrap();
        let kinds: Vec<VoiceImpulse> = emitted.iter().map(|(i, _)| *i).collect();
        assert_eq!(
            kinds,
            vec![VoiceImpulse::SpeechStart, VoiceImpulse::EndOfUtterance],
            "onset → endpoint produces exactly one claim then one EOU"
        );
        assert!(
            emitted[0].1 < emitted[1].1,
            "HLC stamps must be monotonic (start before end)"
        );
    }

    #[tokio::test]
    async fn pure_silence_emits_nothing() {
        let sink = Arc::new(RecordingSink::default());
        let (tx, _rx) = mpsc::channel::<Vec<i16>>(256);
        let vad = EnergyVad::new(16_000, -45.0, 300, 100, 10_000);
        let mut cap = CaptureProcessor::new(vad, sink.clone(), tx);
        for _ in 0..10 {
            cap.push(vec![0i16; 1_600]);
        }
        assert!(sink.emitted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn with_emit_eou_false_emits_onset_but_defers_eou() {
        let sink = Arc::new(RecordingSink::default());
        let (tx, _rx) = mpsc::channel::<Vec<i16>>(256);
        let vad = EnergyVad::new(16_000, -45.0, 300, 100, 10_000);
        // Same onset→endpoint stimulus as the emit-both test, but EOU deferred
        // to a downstream semantic endpointer (ADR-062 6.5 composition).
        let mut cap = CaptureProcessor::new(vad, sink.clone(), tx).with_emit_eou(false);

        let frame = 1_600usize;
        cap.push(vec![0i16; frame]);
        for _ in 0..3 {
            cap.push(voiced(frame));
        }
        for _ in 0..5 {
            cap.push(vec![0i16; frame]);
        }

        let kinds: Vec<VoiceImpulse> = sink
            .emitted
            .lock()
            .unwrap()
            .iter()
            .map(|(i, _)| *i)
            .collect();
        assert_eq!(
            kinds,
            vec![VoiceImpulse::SpeechStart],
            "onset still claims the floor; EOU is deferred to the semantic endpointer"
        );
    }
}
