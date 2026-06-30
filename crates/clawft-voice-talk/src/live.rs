//! Fully-live cpal mic + speaker wiring for `weft talk` (ADR-062 Phase 6.1 /
//! Phase 5). Behind the `live-audio` feature (pulls cpal via
//! `clawft-voice-aec/device`).
//!
//! [`run_live`] is the real spoken loop, composed per the two reconciliation
//! rules:
//!
//! 1. **ONE orchestrator, ONE endpointer.** [`run_capture`] is used purely as
//!    the cpal mic → AEC → 16 kHz **frame source** (its [`CaptureProcessor`]
//!    impulses are dropped). The single endpointer is the controller's
//!    [`SemanticEndpointer`](clawft_channels::voice::turn::SemanticEndpointer)
//!    (smart-turn); its end-of-turn decision becomes the `EndOfUtterance`
//!    impulse (via the `LoopObserver` on `UserTurn`) that the **P2 Talk-Mode
//!    loop** — the one orchestrator — commits. No second EOU path runs.
//! 2. **ONE shared `AecProcessor`.** A single `Arc<Mutex<AecProcessor>>` is
//!    shared across the output [`AecTtsSink`] (playback → `push_render`
//!    reference), [`run_capture`] (mic → `process_capture`), **and** the
//!    barge-in [`AecAudioControl`] (flush). So capture, playback, and the
//!    barge-in flush all cancel the same echo — the live path is runnable, not
//!    a `DiscardSink` stub.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::capture::{CaptureProcessor, ImpulseSink, VoiceImpulse};
use clawft_channels::voice::tts::TtsSink;
use clawft_channels::voice::types::VoiceError;
use clawft_channels::voice::vad::EnergyVad;
use clawft_voice_aec::{AecProcessor, AecTtsSink, run_capture, spawn_output};

use crate::audio::AecAudioControl;
use crate::native::native_components;
use crate::session::{TalkConfig, TalkSession};

/// Drops capture-side impulses: the live path's single endpointer is the
/// controller's `SemanticEndpointer`, so the P5 VAD must not emit a competing
/// `EndOfUtterance` (rule 1 above). `run_capture` still forwards the frames.
struct DropImpulses;
impl ImpulseSink for DropImpulses {
    fn emit(&self, _impulse: VoiceImpulse, _hlc: u64) {}
}

/// Run the live cpal spoken conversation for `config` until `cancel` fires.
///
/// `input_device` selects the mic by name substring (default mic if `None`).
/// Blocks for the session's lifetime; the cpal capture stream lives on a
/// dedicated blocking thread.
pub async fn run_live(
    config: TalkConfig,
    input_device: Option<String>,
    cancel: CancellationToken,
) -> Result<(), VoiceError> {
    // ONE shared AEC: played audio (render reference) ⇄ captured mic (subtract)
    // ⇄ barge-in flush. The single handle is what makes echo cancellation close.
    let aec = Arc::new(Mutex::new(AecProcessor::new()));

    // Output sink (also feeds the render reference) + its cpal stream.
    let sink = Arc::new(AecTtsSink::new(aec.clone()));
    let _out_stream = spawn_output(sink.as_ref()).map_err(VoiceError::Config)?;
    let sink_dyn: Arc<dyn TtsSink> = sink.clone();

    // Barge-in control over the SAME shared AEC (drops the render reference the
    // instant playback is silenced, alongside the sink's own flush).
    let audio = Arc::new(AecAudioControl::from_shared(aec.clone()));

    // Native component stack (Hermes brain, parakeet/smart-turn/ECAPA, Kokoro+
    // Orpheus TTS). The smart-turn SemanticEndpointer inside is THE endpointer.
    let components = native_components(&config, sink_dyn, audio)?;
    let session = TalkSession::assemble(config.clone(), components);

    // Capture → AEC → CaptureProcessor: forwards cleaned frames to the
    // controller. Impulses are dropped (single endpointer — see rule 1).
    let (frames_tx, frames_rx) = mpsc::channel::<Vec<i16>>(256);
    let vad = EnergyVad::new(
        config.sample_rate,
        config.vad_threshold_dbfs,
        config.max_silence_ms,
        100,
        10_000,
    );
    let processor = CaptureProcessor::new(vad, Arc::new(DropImpulses), frames_tx);

    // Bridge the CancellationToken to the capture loop's AtomicBool flag.
    let cap_flag = Arc::new(AtomicBool::new(false));
    {
        let cap_flag = cap_flag.clone();
        let cancel = cancel.clone();
        tokio::spawn(async move {
            cancel.cancelled().await;
            cap_flag.store(true, Ordering::SeqCst);
        });
    }
    let capture = tokio::task::spawn_blocking(move || {
        if let Err(e) = run_capture(input_device.as_deref(), aec, processor, cap_flag) {
            tracing::warn!(error = %e, "live capture stream ended");
        }
    });

    // Drive the conversation (P2 loop + render shell) until cancelled.
    session.run(frames_rx, cancel).await;
    capture.abort();
    let _ = capture.await;
    Ok(())
}
