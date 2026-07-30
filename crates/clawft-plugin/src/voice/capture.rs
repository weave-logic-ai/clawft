//! Audio capture (microphone input) via cpal.
//!
//! Provides `AudioCapture` for streaming PCM audio from the default
//! input device at 16 kHz, 16-bit, mono.
//!
//! ## Privacy indicator (WEFT-207 / SC-1)
//!
//! Every start/stop transition fires the SC-1 mic privacy indicator
//! through [`crate::voice::privacy_indicator::emit_indicator`]. This
//! covers the current stub today and extends, unchanged, to the
//! 0.8.x in-process `cpal::Stream::new` path: that future code path
//! MUST funnel through `AudioCapture::start` / `AudioCapture::stop`
//! (or be invoked exclusively by code that does), so the indicator
//! cannot regress.
//!
//! ## Audio retention / zeroization (WEFT-223 / SC-2)
//!
//! Capture holds recent PCM in a [`SecureAudioRing`] and optional
//! session buffer ([`SecureAudioBuffer`]). Both zeroize on drop.
//! Disk writes of raw audio are gated by
//! [`clawft_types::config::AudioRetention`] (default **none**).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clawft_types::config::AudioRetention;
use clawft_types::{SecureAudioBuffer, SecureAudioRing};
use serde::{Deserialize, Serialize};

use super::config::{VoiceAudioConfig, VoiceCaptureSpec};
use super::privacy_indicator::{
    IndicatorPayload, IndicatorPublisher, IndicatorState, NoopIndicatorPublisher, emit_indicator,
};

/// Default pre-onset / capture ring length (ms) — bounds memory exposure
/// per the voice security review (SC-2: fixed-size ring ≤ ~400 ms).
const DEFAULT_RING_MS: u32 = 400;

/// Audio capture configuration.
///
/// **Deprecated**: prefer [`VoiceAudioConfig`] / [`VoiceCaptureSpec`].
/// Kept as a thin alias for back-compat through 0.7.x; will be removed
/// in 0.8.x once all in-tree callers migrate. Per WEFT-213 the canonical
/// audio config is [`VoiceAudioConfig`] which expresses capture +
/// playback as a single document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels (1 = mono).
    pub channels: u16,
    /// Audio chunk size in samples.
    pub chunk_size: u32,
    /// Optional capture device name; `None` = system default.
    pub device_name: Option<String>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            chunk_size: 512,
            device_name: None,
        }
    }
}

impl CaptureConfig {
    /// Promote a legacy `CaptureConfig` to a [`VoiceCaptureSpec`].
    /// Mirrors `From<CaptureConfig> for VoiceCaptureSpec`; provided as
    /// an explicit method for call sites that prefer it.
    pub fn into_spec(self) -> VoiceCaptureSpec {
        self.into()
    }
}

impl From<CaptureConfig> for VoiceCaptureSpec {
    fn from(value: CaptureConfig) -> Self {
        VoiceCaptureSpec {
            sample_rate: value.sample_rate,
            channels: value.channels,
            chunk_size: value.chunk_size,
            device_name: value.device_name,
        }
    }
}

impl From<VoiceCaptureSpec> for CaptureConfig {
    fn from(value: VoiceCaptureSpec) -> Self {
        CaptureConfig {
            sample_rate: value.sample_rate,
            channels: value.channels,
            chunk_size: value.chunk_size,
            device_name: value.device_name,
        }
    }
}

/// Microphone audio capture stream.
///
/// Wraps cpal input stream. Currently a stub -- real cpal integration
/// is deferred to the 0.8.x in-process voice backend (see ADR-053).
/// 0.7.0 ships with substrate-side capture + transcription, so this
/// scaffolding is a placeholder for the second `SttBackend` implementor.
///
/// Construction defaults the privacy-indicator publisher to a no-op
/// (see [`crate::voice::privacy_indicator`]). Production code paths
/// must call [`AudioCapture::with_indicator_publisher`] (or the full
/// [`AudioCapture::new_with_publisher`] constructor) so the SC-1
/// substrate topic actually surfaces. The tracing-target side of the
/// indicator fires regardless.
///
/// # SC-2 (WEFT-223)
///
/// - Rolling capture ring + session buffer are [`SecureAudioRing`] /
///   [`SecureAudioBuffer`] and **zeroize on drop**.
/// - [`AudioRetention::None`] (default): no session hold, no disk write.
/// - [`AudioRetention::Session`]: frames accumulate in `session_pcm`
///   until [`clear_session`](Self::clear_session) / stop / drop.
/// - [`AudioRetention::Persist`]: [`persist_session_audio`](Self::persist_session_audio)
///   may write under `{workspace}/.clawft/audio/`.
pub struct AudioCapture {
    config: CaptureConfig,
    active: bool,
    indicator: Arc<dyn IndicatorPublisher>,
    /// SC-2 retention policy (default: none).
    retention: AudioRetention,
    /// Fixed-size pre-onset / recent-frame ring (always zeroized on drop).
    ring: SecureAudioRing,
    /// Optional session-held PCM when retention is Session or Persist.
    session_pcm: SecureAudioBuffer,
}

impl AudioCapture {
    /// Create a new audio capture from a legacy [`CaptureConfig`].
    ///
    /// Defaults the indicator publisher to
    /// [`NoopIndicatorPublisher`]; the tracing target still fires.
    /// Production callers should use [`Self::new_with_publisher`].
    /// Retention defaults to [`AudioRetention::None`].
    pub fn new(config: CaptureConfig) -> Self {
        Self::new_with_publisher(config, Arc::new(NoopIndicatorPublisher))
    }

    /// Create a new audio capture with an explicit privacy-indicator
    /// publisher. The substrate-aware daemon wires the real publisher
    /// here; tests pass an
    /// [`crate::voice::privacy_indicator::InMemoryIndicatorPublisher`].
    pub fn new_with_publisher(
        config: CaptureConfig,
        indicator: Arc<dyn IndicatorPublisher>,
    ) -> Self {
        let ring_cap = ring_capacity_samples(config.sample_rate, DEFAULT_RING_MS);
        Self {
            config,
            active: false,
            indicator,
            retention: AudioRetention::None,
            ring: SecureAudioRing::with_capacity(ring_cap),
            session_pcm: SecureAudioBuffer::new(),
        }
    }

    /// Set the SC-2 audio retention policy.
    pub fn with_retention(mut self, retention: AudioRetention) -> Self {
        self.retention = retention;
        self
    }

    /// Current retention policy.
    pub fn retention(&self) -> AudioRetention {
        self.retention
    }

    /// Replace the indicator publisher on an existing handle. Useful
    /// when `from_voice` was used to construct the capture but the
    /// real publisher only becomes available later in boot.
    pub fn with_indicator_publisher(mut self, indicator: Arc<dyn IndicatorPublisher>) -> Self {
        self.indicator = indicator;
        self
    }

    /// Create a new audio capture from a unified [`VoiceAudioConfig`].
    /// Returns `None` if the config has no capture spec attached.
    pub fn from_voice(cfg: &VoiceAudioConfig) -> Option<Self> {
        cfg.capture.clone().map(|spec| Self::new(spec.into()))
    }

    /// Start capturing audio.
    ///
    /// Fires the SC-1 mic privacy indicator (`state: "capturing"`) on
    /// every successful start. Calling `start` while already active
    /// is a no-op and does NOT re-fire the indicator -- subscribers
    /// see one `capturing` per active session, not one per call.
    pub fn start(&mut self) -> Result<(), String> {
        // Stub: real cpal stream creation deferred to 0.8.x in-process
        // backend (see ADR-053). The indicator wiring below is what
        // makes this code path safe to extend with real cpal calls
        // without a separate audit -- the privacy contract is met
        // here, before any real `cpal::Stream::new` lands.
        if self.active {
            return Ok(());
        }
        self.active = true;
        tracing::info!(
            sample_rate = self.config.sample_rate,
            channels = self.config.channels,
            retention = ?self.retention,
            "Audio capture started (stub)"
        );
        let payload = IndicatorPayload::new(
            IndicatorState::Capturing,
            self.config.device_name.clone(),
            self.config.sample_rate,
            self.config.channels,
        );
        emit_indicator(self.indicator.as_ref(), &payload);
        Ok(())
    }

    /// Ingest one PCM frame (stub path / tests / future cpal callback).
    ///
    /// Always updates the zeroizing ring. When retention allows a
    /// session hold, also appends to `session_pcm`.
    pub fn push_frame(&mut self, frame: &[i16]) {
        self.ring.push_frame(frame);
        if self.retention.allows_session_hold() {
            self.session_pcm.extend_from_slice(frame);
        }
        // When retention is None we deliberately do NOT accumulate —
        // the ring alone bounds exposure to DEFAULT_RING_MS of audio.
    }

    /// Snapshot of the current ring (as a secure buffer that zeroizes
    /// on drop). Used for pre-roll / STT handoff tests.
    pub fn ring_snapshot(&self) -> SecureAudioBuffer {
        self.ring.to_secure_buffer()
    }

    /// Samples held under Session/Persist retention (empty when None).
    pub fn session_samples(&self) -> &[i16] {
        self.session_pcm.as_slice()
    }

    /// Zeroize and drop any session-held PCM. Called automatically on
    /// stop/drop; exposed for explicit end-of-session cleanup.
    pub fn clear_session(&mut self) {
        self.session_pcm.clear();
        self.ring.clear();
    }

    /// Persist session PCM under `{workspace}/.clawft/audio/{name}.pcm`
    /// when retention is [`AudioRetention::Persist`].
    ///
    /// Returns `Ok(None)` when the policy forbids disk writes or there
    /// is nothing to write. On success returns the path written.
    pub fn persist_session_audio(
        &self,
        workspace: &Path,
        name: &str,
    ) -> std::io::Result<Option<PathBuf>> {
        if !self.retention.allows_disk_write() {
            return Ok(None);
        }
        if self.session_pcm.is_empty() {
            return Ok(None);
        }
        let dir = workspace.join(".clawft").join("audio");
        std::fs::create_dir_all(&dir)?;
        // Sanitize name to a single path segment.
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = dir.join(format!("{safe}.pcm"));
        // Write little-endian i16 PCM (no header — consumers know the rate).
        let mut bytes = Vec::with_capacity(self.session_pcm.len() * 2);
        for s in self.session_pcm.as_slice() {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(&path, bytes)?;
        tracing::info!(path = %path.display(), "voice audio persisted (audio_retention=persist)");
        Ok(Some(path))
    }

    /// Stop capturing audio.
    ///
    /// Fires the SC-1 mic privacy indicator (`state: "idle"`) iff a
    /// `capturing` event was previously fired. Idempotent: stopping
    /// an inactive capture is a no-op.
    ///
    /// SC-2: when retention is [`AudioRetention::None`], clears the
    /// ring immediately. Session/Persist hold until
    /// [`clear_session`](Self::clear_session) or drop.
    pub fn stop(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        if self.retention.is_none() {
            self.clear_session();
        }
        tracing::info!("Audio capture stopped");
        let payload = IndicatorPayload::new(
            IndicatorState::Idle,
            self.config.device_name.clone(),
            self.config.sample_rate,
            self.config.channels,
        );
        emit_indicator(self.indicator.as_ref(), &payload);
    }

    /// Check if capture is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the capture configuration.
    pub fn config(&self) -> &CaptureConfig {
        &self.config
    }
}

fn ring_capacity_samples(sample_rate: u32, ring_ms: u32) -> usize {
    (sample_rate as usize).saturating_mul(ring_ms as usize) / 1_000
}

impl Drop for AudioCapture {
    /// Failsafe: if the handle is dropped while still active, fire the
    /// `idle` indicator so subscribers don't see a permanently-stuck
    /// `capturing` state. This catches panics, early-returns, and
    /// anywhere the explicit `stop` was missed.
    ///
    /// SC-2: session + ring buffers zeroize via their own `Drop` impls
    /// after this runs (field drop order).
    fn drop(&mut self) {
        if self.active {
            self.stop();
        }
        // Always clear held PCM on drop — even Session/Persist cannot
        // outlive the capture handle (persist must happen before drop).
        self.clear_session();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::privacy_indicator::InMemoryIndicatorPublisher;

    fn cfg() -> CaptureConfig {
        CaptureConfig {
            sample_rate: 16_000,
            channels: 1,
            chunk_size: 512,
            device_name: Some("test-mic".into()),
        }
    }

    #[test]
    fn start_emits_capturing_indicator() {
        let pub_ = InMemoryIndicatorPublisher::new();
        let mut cap = AudioCapture::new_with_publisher(cfg(), Arc::new(pub_.clone()));
        cap.start().unwrap();
        let snap = pub_.snapshot();
        assert_eq!(snap.len(), 1, "exactly one indicator on start");
        assert_eq!(snap[0].state, "capturing");
        assert_eq!(snap[0].device.as_deref(), Some("test-mic"));
        assert_eq!(snap[0].sample_rate, 16_000);
        assert_eq!(snap[0].channels, 1);
    }

    #[test]
    fn stop_emits_idle_indicator() {
        let pub_ = InMemoryIndicatorPublisher::new();
        let mut cap = AudioCapture::new_with_publisher(cfg(), Arc::new(pub_.clone()));
        cap.start().unwrap();
        cap.stop();
        let snap = pub_.snapshot();
        assert_eq!(snap.len(), 2, "capturing + idle");
        assert_eq!(snap[0].state, "capturing");
        assert_eq!(snap[1].state, "idle");
    }

    #[test]
    fn no_indicator_when_capture_never_started() {
        // Construct + drop without start: no spurious indicator events.
        // Audit consumers should never see an `idle` without a prior
        // `capturing`.
        let pub_ = InMemoryIndicatorPublisher::new();
        let cap = AudioCapture::new_with_publisher(cfg(), Arc::new(pub_.clone()));
        drop(cap);
        assert!(
            pub_.is_empty(),
            "no indicator events for never-started capture"
        );
    }

    #[test]
    fn double_start_emits_one_capturing() {
        let pub_ = InMemoryIndicatorPublisher::new();
        let mut cap = AudioCapture::new_with_publisher(cfg(), Arc::new(pub_.clone()));
        cap.start().unwrap();
        cap.start().unwrap();
        assert_eq!(pub_.len(), 1, "second start is a no-op");
    }

    #[test]
    fn double_stop_emits_one_idle() {
        let pub_ = InMemoryIndicatorPublisher::new();
        let mut cap = AudioCapture::new_with_publisher(cfg(), Arc::new(pub_.clone()));
        cap.start().unwrap();
        cap.stop();
        cap.stop();
        let snap = pub_.snapshot();
        assert_eq!(snap.len(), 2, "second stop is a no-op");
        assert_eq!(snap[1].state, "idle");
    }

    #[test]
    fn drop_while_active_emits_idle_failsafe() {
        // Drop without an explicit stop must still surface `idle`,
        // otherwise a panic in the surrounding code would leave the
        // mic indicator stuck on `capturing` forever from the UI's
        // perspective.
        let pub_ = InMemoryIndicatorPublisher::new();
        {
            let mut cap = AudioCapture::new_with_publisher(cfg(), Arc::new(pub_.clone()));
            cap.start().unwrap();
        }
        let snap = pub_.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[1].state, "idle");
    }

    #[test]
    fn default_publisher_does_not_panic_on_start_stop() {
        // The legacy `new` constructor uses the no-op publisher; the
        // tracing-target side still fires but the substrate-topic side
        // is silent. Calling start/stop must not panic.
        let mut cap = AudioCapture::new(cfg());
        cap.start().unwrap();
        cap.stop();
    }

    // ── SC-2 / WEFT-223 ────────────────────────────────────────────────

    #[test]
    fn default_retention_is_none() {
        let cap = AudioCapture::new(cfg());
        assert_eq!(cap.retention(), AudioRetention::None);
        assert!(!cap.retention().allows_disk_write());
    }

    #[test]
    fn retention_none_does_not_accumulate_session_pcm() {
        let mut cap = AudioCapture::new(cfg());
        cap.start().unwrap();
        cap.push_frame(&[1, 2, 3, 4]);
        cap.push_frame(&[5, 6, 7, 8]);
        // Ring holds recent samples; session buffer stays empty under None.
        assert!(cap.session_samples().is_empty());
        assert!(!cap.ring_snapshot().is_empty());
        cap.stop();
        // Stop with None clears the ring.
        assert!(cap.ring_snapshot().is_empty());
    }

    #[test]
    fn retention_session_holds_until_clear() {
        let mut cap = AudioCapture::new(cfg()).with_retention(AudioRetention::Session);
        cap.start().unwrap();
        cap.push_frame(&[10, 20, 30]);
        assert_eq!(cap.session_samples(), &[10, 20, 30]);
        // Session hold survives stop.
        cap.stop();
        assert_eq!(cap.session_samples(), &[10, 20, 30]);
        // Disk write still refused.
        let tmp = std::env::temp_dir().join("clawft_sc2_session_no_disk");
        let _ = std::fs::remove_dir_all(&tmp);
        let written = cap.persist_session_audio(&tmp, "sess").unwrap();
        assert!(written.is_none());
        cap.clear_session();
        assert!(cap.session_samples().is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn retention_persist_writes_pcm_under_workspace() {
        let mut cap = AudioCapture::new(cfg()).with_retention(AudioRetention::Persist);
        cap.start().unwrap();
        cap.push_frame(&[100, -100, 200, -200]);
        let tmp = std::env::temp_dir().join("clawft_sc2_persist_audio");
        let _ = std::fs::remove_dir_all(&tmp);
        let path = cap
            .persist_session_audio(&tmp, "utt-1")
            .unwrap()
            .expect("persist should write");
        assert!(path.ends_with(".clawft/audio/utt-1.pcm"));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 4 * 2); // 4 i16 samples
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn drop_zeroizes_session_and_ring() {
        let mut cap = AudioCapture::new(cfg()).with_retention(AudioRetention::Session);
        cap.push_frame(&[1, 2, 3, 4, 5]);
        assert!(!cap.session_samples().is_empty());
        drop(cap);
        // Drop path exercises SecureAudioBuffer/Ring Drop; no panic.
    }
}
