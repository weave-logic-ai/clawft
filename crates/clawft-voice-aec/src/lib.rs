//! Native WebRTC-AEC full-duplex audio bridge.
//!
//! This crate owns BOTH microphone capture and speaker playback itself
//! (via cpal / CoreAudio), then runs the WebRTC Audio Processing Module
//! (AEC3 + NS + AGC) with the played audio as the *render/reference*
//! stream and the captured mic as the *capture* stream — yielding an
//! echo-cancelled mono 16 kHz mic. This replaces the dead-end Apple VPIO
//! path (which `-10875`s on this Mac's 9-channel built-in mic) because we
//! take the raw device stream from cpal and DOWNMIX whatever channel
//! count it gives us to mono ourselves — the 9-channel aggregate problem
//! cannot happen.
//!
//! # STDIO protocol (mirrors `voicelab/native/vpio.py`)
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
//! - `bridge`     — cpal device I/O + stdio protocol + passthrough AEC.
//!   Always runnable; proves the connect/round-trip without the heavy dep.
//! - `webrtc-aec` — real WebRTC AEC3 echo cancellation (bundled libwebrtc
//!   APM, built via cmake). Implies `bridge`.

#[cfg(feature = "bridge")]
mod bridge;

#[cfg(feature = "bridge")]
pub use bridge::{run, Config};

/// Wire / processing constants. Public so the binary (and any future
/// in-process weftos consumer) can refer to them.
#[cfg(feature = "bridge")]
pub mod consts {
    /// Target sample rate of the stdin/stdout wire format and the APM.
    pub const TARGET_SR: u32 = 16_000;
    /// 10 ms frame at 16 kHz — the WebRTC APM's native frame size.
    pub const FRAME: usize = 160;
}
