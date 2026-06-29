//! Fully-live cpal mic + speaker wiring for `weft talk` (ADR-062 Phase 6.1 /
//! Phase 5). Behind the `live-audio` feature (pulls cpal via
//! `clawft-voice-aec/device`).
//!
//! [`run_live`] is the real spoken loop: it shares one [`AecProcessor`] between
//! a cpal output [`AecTtsSink`] (which doubles as the AEC render reference) and
//! the cpal capture path ([`run_capture`]), wires capture's
//! [`CaptureProcessor`] to emit floor impulses into the forest via
//! [`KernelImpulseSink`], builds the native component stack, and drives the
//! assembled [`TalkSession`] (P2 loop orchestrator + render shell) until
//! `cancel` fires.
//!
//! Barge-in: the controller cancels the in-flight TTS, whose
//! [`TtsSink::flush`](clawft_channels::voice::tts::TtsSink::flush) drops the
//! queued playback **and** the shared AEC render reference together — so the
//! mic stops cancelling the user's onset. The separate `AudioControl` is
//! therefore a no-op here (the sink owns the shared AEC).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::capture::CaptureProcessor;
use clawft_channels::voice::talkmode::NoopAudioControl;
use clawft_channels::voice::tts::TtsSink;
use clawft_channels::voice::types::VoiceError;
use clawft_channels::voice::vad::EnergyVad;
use clawft_voice_aec::{AecProcessor, AecTtsSink, run_capture, spawn_output};

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
    // Shared AEC: played audio (render reference) ⇄ captured mic (subtract).
    let aec = Arc::new(Mutex::new(AecProcessor::new()));

    // Output sink (also feeds the render reference) + its cpal stream.
    let sink = Arc::new(AecTtsSink::new(aec.clone()));
    let _out_stream = spawn_output(sink.as_ref()).map_err(VoiceError::Config)?;
    let sink_dyn: Arc<dyn TtsSink> = sink.clone();

    // Native component stack (Hermes brain, parakeet/smart-turn/ECAPA, Kokoro+
    // Orpheus TTS). AudioControl is a no-op — the sink flush owns the AEC.
    let components = native_components(&config, sink_dyn, Arc::new(NoopAudioControl))?;
    let session = TalkSession::assemble(config.clone(), components);

    // Capture → AEC → CaptureProcessor: emits floor impulses into the forest's
    // queue (KernelImpulseSink) and forwards cleaned frames to the controller.
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
    let processor = CaptureProcessor::new(vad, impulse_sink, frames_tx);

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
