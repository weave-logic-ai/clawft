//! Fully-live cpal mic + speaker wiring for `weft talk` (ADR-062 Phase 6.1 /
//! Phase 5). Behind the `live-audio` feature (pulls cpal via
//! `clawft-voice-aec/device`).
//!
//! [`run_live`] is the real spoken loop, composed per the two reconciliation
//! rules:
//!
//! 1. **Endpointing composes; one authoritative EOU.** [`run_capture`]'s
//!    [`CaptureProcessor`] runs the energy VAD and emits the coarse onset as a
//!    `TurnClaim` (the early-turn tier) into the forest via [`KernelImpulseSink`],
//!    but **defers EOU** (`with_emit_eou(false)`). The single *authoritative*
//!    endpointer is the controller's smart-turn
//!    [`SemanticEndpointer`](clawft_channels::voice::turn::SemanticEndpointer)
//!    (ADR-062 6.5); its end-of-turn decision becomes the `EndOfUtterance`
//!    impulse (via the `LoopObserver` on `UserTurn`) that the **P2 Talk-Mode
//!    loop** — the one orchestrator — commits. The two endpointers never both
//!    fire EOU.
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

use clawft_channels::voice::capture::CaptureProcessor;
use clawft_channels::voice::tts::TtsSink;
use clawft_channels::voice::types::VoiceError;
use clawft_channels::voice::vad::EnergyVad;
use clawft_voice_aec::{AecProcessor, AecTtsSink, run_capture, spawn_output};
// Re-exported for `weft voice test-mic` — the probe opens the same device
// the live capture path would, so its verdict transfers directly.
pub use clawft_voice_aec::{MicProbeReport, list_input_devices, mic_probe};

use crate::audio::AecAudioControl;
use crate::forest::KernelImpulseSink;
use crate::native::native_components;
use crate::session::{TalkConfig, TalkSession};

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
    run_live_observed(config, input_device, cancel, None).await
}

/// [`run_live`] with an optional extra [`ConversationObserver`] fanned out
/// alongside the loop observer — the seam `weft voice talk` uses to mirror
/// committed turns to the daemon's `agent.turn.record` RPC (witness-chain /
/// substrate anchoring). The observer must be non-blocking.
pub async fn run_live_observed(
    config: TalkConfig,
    input_device: Option<String>,
    cancel: CancellationToken,
    extra_observer: Option<
        std::sync::Arc<dyn clawft_channels::voice::talkmode::ConversationObserver>,
    >,
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
    let session = TalkSession::assemble_observed(config.clone(), components, extra_observer);

    // Capture → AEC → CaptureProcessor: forwards cleaned frames to the
    // controller AND emits the coarse onset as a TurnClaim into the forest
    // queue; EOU is deferred to the controller's SemanticEndpointer (rule 1).
    let (frames_tx, frames_rx) = mpsc::channel::<Vec<i16>>(256);
    let impulse_sink = Arc::new(KernelImpulseSink::new(
        session.forest().impulses().clone(),
        [0u8; 32],
    ));
    let vad = EnergyVad::new(
        config.sample_rate,
        config.vad_threshold_dbfs,
        config.max_silence_ms,
        100,
        10_000,
    );
    let processor = CaptureProcessor::new(vad, impulse_sink, frames_tx).with_emit_eou(false);

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
