//! Native WebRTC-AEC full-duplex audio bridge + in-process AEC library.
//!
//! This crate owns echo cancellation for the weftos voice loop. It runs the
//! WebRTC Audio Processing Module (AEC3 + NS + AGC) with the played audio as
//! the *render/reference* stream and the captured mic as the *capture*
//! stream — yielding an echo-cancelled mono 16 kHz mic. This replaces the
//! dead-end Apple VPIO path (which `-10875`s on this Mac's 9-channel built-in
//! mic): we take the raw device stream from cpal and DOWNMIX whatever channel
//! count it gives us to mono ourselves, so the 9-channel aggregate problem
//! cannot happen.
//!
//! # Two ways to consume it
//!
//! - **In-process ([`AecProcessor`]):** the library API. A consumer
//!   (`clawft-channels::voice`) owns its own capture + playback and routes
//!   PCM through [`AecProcessor::push_render`] / [`AecProcessor::process_capture`],
//!   with [`AecProcessor::flush`] for barge-in. No device I/O, no subprocess —
//!   available in the **default build** (passthrough) and with real AEC3 under
//!   `--features webrtc-aec`.
//! - **Stdio bridge (`aec-bridge` bin, [`run`]):** owns mic + speaker via cpal
//!   and speaks an int16/16k stdio protocol. Used by the `~/llm` voicelab
//!   harness. Gated behind `bridge`.
//!
//! # STDIO protocol (bridge; mirrors `voicelab/native/vpio.py`)
//!
//! - **stdin**  ← audio to PLAY: little-endian `int16` mono @ 16000 Hz
//!   (also fed to the APM as the AEC render/reference stream).
//! - **stdout** → cleaned, echo-cancelled mic: little-endian `int16` mono
//!   @ 16000 Hz (continuous).
//! - **stderr** → status / errors only (one banner line on start).
//! - **SIGUSR1** → barge-in flush: immediately drop all queued playback +
//!   render reference (David goes silent). Mirrors vpio's SIGUSR1.
//!
//! # Feature flags
//!
//! - *(default)*  — the in-process [`AecProcessor`] (passthrough core). No
//!   cpal, no libwebrtc; always builds.
//! - `webrtc-aec` — real WebRTC AEC3 echo cancellation (bundled libwebrtc
//!   APM, built via cmake) in [`AecProcessor`]. No device deps on its own.
//! - `bridge`     — cpal device I/O + the stdio `aec-bridge` bin. Combine with
//!   `webrtc-aec` for a real-AEC bridge.

/// Wire / processing constants. Public so the binary and any in-process
/// weftos consumer can refer to them.
pub mod consts {
    /// Target sample rate of the stdin/stdout wire format and the AEC API.
    pub const TARGET_SR: u32 = 16_000;
    /// 10 ms frame at 16 kHz — the WebRTC APM's native frame size at 16k.
    pub const FRAME: usize = 160;
}

mod dsp;
pub use dsp::AecProcessor;

#[cfg(feature = "bridge")]
mod bridge;

#[cfg(feature = "bridge")]
pub use bridge::{Config, run};
