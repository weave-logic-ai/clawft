# Phase VS1: Audio Foundation + STT/TTS + VoiceChannel

> **Element:** Voice Development (Workstream G)
> **Phase:** VS1
> **Timeline:** Weeks 1-3
> **Priority:** P0 (Critical Path)
> **Crates:** `clawft-plugin` (voice module), `clawft-tools` (voice tools), `clawft-types` (VoiceConfig), `clawft-cli` (CLI commands)
> **Dependencies IN:** C1 (Plugin Traits -- VoiceHandler trait, voice feature flag), C7 (ChannelAdapter trait)
> **Blocks:** VS2 (Wake Word + Platform Integration), VS3 (Advanced Features + UI Integration)
> **Status:** Planning

---

## 1. Overview

Phase VS1 delivers the complete audio foundation for ClawFT voice capabilities. This is the critical-path first phase of Workstream G, building on the `VoiceHandler` placeholder trait and `voice` feature flag established in C1 (Plugin Traits).

VS1 produces three deliverables across three weeks:

1. **VS1.1 (Week 1) -- Audio Foundation:** Microphone capture, speaker playback, voice activity detection, model management, and configuration types.
2. **VS1.2 (Week 2) -- STT + TTS:** Streaming speech-to-text and text-to-speech via sherpa-rs, exposed as agent tools and CLI commands.
3. **VS1.3 (Week 3) -- VoiceChannel + Talk Mode:** A `ChannelAdapter` implementation bridging voice I/O with the agent message bus, plus a continuous conversation loop with interruption detection.

The voice pipeline architecture:

```text
mic (cpal) -> VAD (sherpa-rs Silero) -> STT (sherpa-rs) -> Agent Loop -> TTS (sherpa-rs) -> speaker (cpal)
```

All voice dependencies are gated behind the `voice` feature flag. When disabled, the binary has zero voice-related code compiled in.

---

## 2. Current Code

### 2.1 VoiceHandler Trait (C1 -- exists)

The `VoiceHandler` trait is already defined in `crates/clawft-plugin/src/traits.rs` as a placeholder:

```rust
// crates/clawft-plugin/src/traits.rs (current)
#[async_trait]
pub trait VoiceHandler: Send + Sync {
    async fn process_audio(
        &self,
        audio_data: &[u8],
        mime_type: &str,
    ) -> Result<String, PluginError>;

    async fn synthesize(
        &self,
        text: &str,
    ) -> Result<(Vec<u8>, String), PluginError>;
}
```

VS1 implements this trait with real sherpa-rs backends and extends it with streaming capabilities.

### 2.2 Voice Feature Flag (C1 -- exists)

The `voice` feature flag is reserved in `crates/clawft-plugin/Cargo.toml`:

```toml
[features]
default = []
voice = []   # Empty no-op -- reserves the feature flag for Workstream G
```

VS1 expands this into granular sub-features with real dependencies.

### 2.3 MessagePayload::Binary (exists in clawft-types)

Binary content transport already exists in `crates/clawft-types/src/agent_bus.rs`:

```rust
// crates/clawft-types/src/agent_bus.rs (current)
pub enum MessagePayload {
    Text { content: String },
    Structured { content: Value },
    Binary {
        mime_type: String,
        #[serde(with = "base64_bytes")]
        data: Vec<u8>,
    },
}
```

The `Binary` variant with `mime_type: "audio/pcm"` is the transport format for raw audio between voice pipeline components.

### 2.4 ChannelAdapter Trait (C1 -- exists)

The `ChannelAdapter` trait in `crates/clawft-plugin/src/traits.rs` is the contract that `VoiceChannel` will implement:

```rust
// crates/clawft-plugin/src/traits.rs (current)
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn supports_threads(&self) -> bool;
    fn supports_media(&self) -> bool;
    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError>;
    async fn send(
        &self,
        target: &str,
        payload: &MessagePayload,
    ) -> Result<String, PluginError>;
}
```

### 2.5 Config Types (exists in clawft-types)

The root `Config` struct in `crates/clawft-types/src/config/mod.rs` does not yet have a `voice` field. VS1 adds `VoiceConfig` here.

### 2.6 No Voice Module

There is no `crates/clawft-plugin/src/voice/` directory today. This module tree is created from scratch in VS1.

---

## 3. Implementation Tasks

### Task VS1.1: Audio Foundation (Week 1)

#### VS1.1.1: Expand Feature Flags + Add Dependencies

Update `crates/clawft-plugin/Cargo.toml` to wire real voice dependencies behind granular feature flags:

```toml
[features]
default = []
voice = ["voice-stt", "voice-tts", "voice-vad"]
voice-stt = ["dep:sherpa-rs"]
voice-tts = ["dep:sherpa-rs"]
voice-vad = ["dep:sherpa-rs", "dep:cpal"]
voice-wake = ["dep:rustpotter", "dep:cpal"]   # Reserved for VS2

[dependencies]
# ... existing deps ...

# Voice dependencies (all optional, gated behind feature flags)
sherpa-rs = { version = "0.1", optional = true }
cpal = { version = "0.15", optional = true }
rustpotter = { version = "1.0", optional = true }
ringbuf = { version = "0.4", optional = true }
```

**Note:** Exact `sherpa-rs` version pinned after VP1 prototype validation. The `ringbuf` crate provides a lock-free ring buffer for the audio pipeline.

#### VS1.1.2: Create Voice Module Structure

```
crates/clawft-plugin/src/voice/
  mod.rs              -- module root, re-exports, feature-gated
  capture.rs          -- AudioCapture (cpal microphone input)
  playback.rs         -- AudioPlayback (cpal speaker output)
  vad.rs              -- VoiceActivityDetector (Silero VAD)
  stt.rs              -- SpeechToText (sherpa-rs streaming recognizer)
  tts.rs              -- TextToSpeech (sherpa-rs streaming synthesizer)
  models.rs           -- ModelDownloadManager (fetch + cache + integrity)
  config.rs           -- VoiceConfig re-export / voice-specific config helpers
  channel.rs          -- VoiceChannel (ChannelAdapter impl) -- VS1.3
  talk_mode.rs        -- TalkModeController -- VS1.3
```

Module root with feature gating:

```rust
// crates/clawft-plugin/src/voice/mod.rs

//! Voice pipeline components for ClawFT.
//!
//! All types are gated behind the `voice` feature flag and its sub-features.
//! When `voice` is disabled, this module is empty.

#[cfg(feature = "voice-vad")]
pub mod capture;
#[cfg(feature = "voice-vad")]
pub mod playback;
#[cfg(feature = "voice-vad")]
pub mod vad;

#[cfg(feature = "voice-stt")]
pub mod stt;

#[cfg(feature = "voice-tts")]
pub mod tts;

#[cfg(any(feature = "voice-stt", feature = "voice-tts"))]
pub mod models;

pub mod config;

#[cfg(feature = "voice")]
pub mod channel;
#[cfg(feature = "voice")]
pub mod talk_mode;

// Re-exports for convenience
#[cfg(feature = "voice-vad")]
pub use capture::AudioCapture;
#[cfg(feature = "voice-vad")]
pub use playback::AudioPlayback;
#[cfg(feature = "voice-vad")]
pub use vad::VoiceActivityDetector;

#[cfg(feature = "voice-stt")]
pub use stt::SpeechToText;

#[cfg(feature = "voice-tts")]
pub use tts::TextToSpeech;

#[cfg(any(feature = "voice-stt", feature = "voice-tts"))]
pub use models::ModelDownloadManager;

#[cfg(feature = "voice")]
pub use channel::VoiceChannel;
#[cfg(feature = "voice")]
pub use talk_mode::TalkModeController;
```

Wire into `crates/clawft-plugin/src/lib.rs`:

```rust
// Add to crates/clawft-plugin/src/lib.rs
#[cfg(any(
    feature = "voice",
    feature = "voice-stt",
    feature = "voice-tts",
    feature = "voice-vad",
))]
pub mod voice;
```

#### VS1.1.3: AudioCapture -- cpal Microphone Input Stream

```rust
// crates/clawft-plugin/src/voice/capture.rs

use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::PluginError;

/// Audio format constants for the voice pipeline.
pub const SAMPLE_RATE: u32 = 16_000;
pub const CHANNELS: u16 = 1;
pub const CHUNK_SAMPLES: usize = 480; // 30ms at 16kHz
pub const BUFFER_CHUNKS: usize = 10;  // 300ms buffer depth

/// Captures audio from the system microphone via cpal.
///
/// Produces a stream of `Vec<f32>` audio chunks (mono, 16kHz, 30ms each)
/// sent through a tokio mpsc channel. The stream runs on a dedicated
/// cpal audio thread and is cancelled by dropping the `AudioCapture`.
pub struct AudioCapture {
    /// The cpal audio stream handle. Dropping this stops capture.
    _stream: Stream,
    /// Receiver for audio chunks. Each chunk is `CHUNK_SAMPLES` f32 samples.
    pub rx: mpsc::Receiver<Vec<f32>>,
    /// Name of the input device being used.
    pub device_name: String,
}

impl AudioCapture {
    /// Start capturing from the default input device.
    ///
    /// Returns an `AudioCapture` with an `rx` channel that yields
    /// `Vec<f32>` audio chunks (mono, 16kHz, 480 samples = 30ms).
    pub fn start_default() -> Result<Self, PluginError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            PluginError::ExecutionFailed("no default audio input device found".into())
        })?;
        Self::start_device(device)
    }

    /// Start capturing from a specific input device.
    pub fn start_device(device: Device) -> Result<Self, PluginError> {
        let device_name = device.name().unwrap_or_else(|_| "unknown".into());
        info!(device = %device_name, "starting audio capture");

        let config = StreamConfig {
            channels: CHANNELS,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Fixed(CHUNK_SAMPLES as u32),
        };

        let (tx, rx) = mpsc::channel::<Vec<f32>>(BUFFER_CHUNKS);

        let supported = device.supported_input_configs().map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to query input configs: {e}"))
        })?;

        // Log supported formats for debugging
        for fmt in supported {
            debug!(
                channels = fmt.channels(),
                min_rate = fmt.min_sample_rate().0,
                max_rate = fmt.max_sample_rate().0,
                format = ?fmt.sample_format(),
                "supported input config"
            );
        }

        let err_fn = |err: cpal::StreamError| {
            error!(%err, "audio capture stream error");
        };

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    // Chunk the incoming samples into CHUNK_SAMPLES-sized pieces
                    for chunk in data.chunks(CHUNK_SAMPLES) {
                        let samples = chunk.to_vec();
                        if tx.try_send(samples).is_err() {
                            warn!("audio capture buffer full, dropping chunk");
                        }
                    }
                },
                err_fn,
                None, // No timeout
            )
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("failed to build input stream: {e}"))
            })?;

        stream.play().map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to start input stream: {e}"))
        })?;

        Ok(Self {
            _stream: stream,
            rx,
            device_name,
        })
    }

    /// List available input devices.
    pub fn list_devices() -> Result<Vec<String>, PluginError> {
        let host = cpal::default_host();
        let devices = host.input_devices().map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to enumerate input devices: {e}"))
        })?;
        Ok(devices
            .filter_map(|d| d.name().ok())
            .collect())
    }
}
```

#### VS1.1.4: AudioPlayback -- cpal Speaker Output Stream

```rust
// crates/clawft-plugin/src/voice/playback.rs

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use ringbuf::{HeapRb, Rb};
use ringbuf::traits::{Consumer, Producer, Split};
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::PluginError;
use super::capture::SAMPLE_RATE;

/// Plays audio through the system speaker via cpal.
///
/// Accepts `Vec<f32>` audio chunks pushed to the `producer` side of
/// a lock-free ring buffer. The cpal output thread pulls from the
/// consumer side. Supports cancellation via a watch channel.
pub struct AudioPlayback {
    /// The cpal output stream handle. Dropping this stops playback.
    _stream: Stream,
    /// Producer side of the ring buffer. Push f32 samples here.
    pub producer: ringbuf::HeapProd<f32>,
    /// Send `true` to signal that playback should stop (for interruption).
    pub cancel_tx: watch::Sender<bool>,
    /// Name of the output device being used.
    pub device_name: String,
}

impl AudioPlayback {
    /// Start playback on the default output device.
    ///
    /// Returns an `AudioPlayback` with a `producer` ring buffer
    /// that accepts f32 samples (mono, 16kHz).
    pub fn start_default() -> Result<Self, PluginError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            PluginError::ExecutionFailed("no default audio output device found".into())
        })?;
        Self::start_device(device)
    }

    /// Start playback on a specific output device.
    pub fn start_device(device: Device) -> Result<Self, PluginError> {
        let device_name = device.name().unwrap_or_else(|_| "unknown".into());
        info!(device = %device_name, "starting audio playback");

        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // Ring buffer: 1 second of audio at 16kHz
        let ring = HeapRb::<f32>::new(SAMPLE_RATE as usize);
        let (producer, mut consumer) = ring.split();

        let (cancel_tx, cancel_rx) = watch::channel(false);

        let err_fn = |err: cpal::StreamError| {
            error!(%err, "audio playback stream error");
        };

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    if *cancel_rx.borrow() {
                        // Fill with silence when cancelled
                        data.fill(0.0);
                        return;
                    }
                    for sample in data.iter_mut() {
                        *sample = consumer.try_pop().unwrap_or(0.0);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("failed to build output stream: {e}"))
            })?;

        stream.play().map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to start output stream: {e}"))
        })?;

        Ok(Self {
            _stream: stream,
            producer,
            cancel_tx,
            device_name,
        })
    }

    /// Push audio samples into the playback buffer.
    ///
    /// Returns the number of samples actually written (may be less
    /// than `samples.len()` if the buffer is full).
    pub fn write_samples(&mut self, samples: &[f32]) -> usize {
        let mut written = 0;
        for &s in samples {
            if self.producer.try_push(s).is_ok() {
                written += 1;
            } else {
                warn!("playback buffer full, dropping samples");
                break;
            }
        }
        written
    }

    /// Signal the playback stream to stop outputting audio.
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }

    /// List available output devices.
    pub fn list_devices() -> Result<Vec<String>, PluginError> {
        let host = cpal::default_host();
        let devices = host.output_devices().map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to enumerate output devices: {e}"))
        })?;
        Ok(devices
            .filter_map(|d| d.name().ok())
            .collect())
    }
}
```

#### VS1.1.5: VoiceActivityDetector -- sherpa-rs Silero VAD Wrapper

```rust
// crates/clawft-plugin/src/voice/vad.rs

use sherpa_rs::vad::{Vad, VadConfig};
use tracing::{debug, info};

use crate::PluginError;
use super::capture::SAMPLE_RATE;

/// Voice activity detection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    /// No speech detected. Pipeline is idle.
    Silence,
    /// Speech detected. Pipeline should capture and transcribe.
    Speech,
    /// Transition from speech to silence (end of utterance).
    SpeechEnd,
}

/// Wraps sherpa-rs Silero VAD for speech boundary detection.
///
/// Fed audio chunks from `AudioCapture`. Emits `VadState` transitions
/// that drive the STT pipeline. The VAD model (Silero V5) is loaded
/// from the model cache directory.
pub struct VoiceActivityDetector {
    vad: Vad,
    /// Current state for edge detection.
    prev_is_speech: bool,
    /// Minimum speech duration in samples before triggering Speech state.
    min_speech_samples: usize,
    /// Accumulated speech samples for debouncing.
    speech_accumulator: usize,
}

impl VoiceActivityDetector {
    /// Create a new VAD instance.
    ///
    /// `model_path` is the path to the Silero VAD ONNX model file.
    /// `threshold` is the speech probability threshold (default 0.5).
    /// `min_speech_duration_ms` is the minimum speech duration to
    /// trigger detection (default 250ms, prevents false triggers).
    pub fn new(
        model_path: &str,
        threshold: f32,
        min_speech_duration_ms: u32,
    ) -> Result<Self, PluginError> {
        info!(model = model_path, threshold, "initializing VAD");

        let config = VadConfig {
            model: model_path.into(),
            min_silence_duration: 0.5,       // 500ms silence = end of speech
            min_speech_duration: 0.25,        // 250ms minimum speech
            threshold,
            sample_rate: SAMPLE_RATE as i32,
            window_size: 512,                 // Silero V5 window
        };

        let vad = Vad::new(config, 60.0).map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to initialize VAD: {e}"))
        })?;

        let min_speech_samples =
            (min_speech_duration_ms as usize * SAMPLE_RATE as usize) / 1000;

        Ok(Self {
            vad,
            prev_is_speech: false,
            min_speech_samples,
            speech_accumulator: 0,
        })
    }

    /// Process an audio chunk and return the current VAD state.
    ///
    /// `samples` must be mono f32 audio at 16kHz.
    /// Returns `VadState::Speech` when speech is detected,
    /// `VadState::SpeechEnd` on the transition from speech to silence,
    /// and `VadState::Silence` otherwise.
    pub fn process(&mut self, samples: &[f32]) -> VadState {
        self.vad.accept_waveform(samples.to_vec());

        let is_speech = !self.vad.is_empty();

        if is_speech {
            self.speech_accumulator += samples.len();
        }

        let state = match (self.prev_is_speech, is_speech) {
            (false, true) => {
                if self.speech_accumulator >= self.min_speech_samples {
                    debug!("VAD: speech detected");
                    VadState::Speech
                } else {
                    VadState::Silence // Still accumulating
                }
            }
            (true, true) => VadState::Speech,
            (true, false) => {
                debug!("VAD: speech ended");
                self.speech_accumulator = 0;
                VadState::SpeechEnd
            }
            (false, false) => {
                self.speech_accumulator = 0;
                VadState::Silence
            }
        };

        self.prev_is_speech = is_speech;
        state
    }

    /// Reset the VAD state (e.g., after processing an utterance).
    pub fn reset(&mut self) {
        self.vad.reset();
        self.prev_is_speech = false;
        self.speech_accumulator = 0;
    }
}
```

#### VS1.1.6: ModelDownloadManager -- Fetch + Cache + SHA-256 Integrity

```rust
// crates/clawft-plugin/src/voice/models.rs

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::{info, warn};

use crate::PluginError;

/// Known voice model identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VoiceModel {
    /// Silero VAD V5 ONNX model.
    SileroVad,
    /// sherpa-onnx streaming STT (Zipformer int8).
    StreamingStt,
    /// sherpa-onnx streaming TTS (VITS).
    StreamingTts,
}

/// Model metadata for download and integrity verification.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model identifier.
    pub model: VoiceModel,
    /// Display name.
    pub name: String,
    /// Download URL.
    pub url: String,
    /// Expected SHA-256 hash of the downloaded file.
    pub sha256: String,
    /// Relative path within the model cache directory.
    pub relative_path: String,
    /// Approximate size in bytes (for progress display).
    pub size_bytes: u64,
}

/// Callback type for download progress reporting.
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send>;

/// Manages voice model downloads, caching, and integrity verification.
///
/// Models are stored in `~/.clawft/models/voice/`. Each model is
/// verified against a SHA-256 hash after download. Already-downloaded
/// models are validated on startup and re-downloaded if corrupted.
pub struct ModelDownloadManager {
    /// Base directory for model storage (e.g., `~/.clawft/models/voice/`).
    cache_dir: PathBuf,
    /// Registry of known models.
    registry: Vec<ModelInfo>,
}

impl ModelDownloadManager {
    /// Create a new manager with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        let registry = Self::default_registry();
        Self {
            cache_dir,
            registry,
        }
    }

    /// Default model cache directory: `~/.clawft/models/voice/`.
    pub fn default_cache_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".clawft")
            .join("models")
            .join("voice")
    }

    /// Check whether a model is already downloaded and valid.
    pub async fn is_model_available(&self, model: &VoiceModel) -> bool {
        let Some(info) = self.find_model(model) else {
            return false;
        };
        let path = self.cache_dir.join(&info.relative_path);
        if !path.exists() {
            return false;
        }
        // Verify integrity
        match self.verify_sha256(&path, &info.sha256).await {
            Ok(valid) => valid,
            Err(_) => false,
        }
    }

    /// Get the local path for a model. Returns None if not downloaded.
    pub fn model_path(&self, model: &VoiceModel) -> Option<PathBuf> {
        let info = self.find_model(model)?;
        let path = self.cache_dir.join(&info.relative_path);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Download a model if not already cached.
    ///
    /// `progress` is called with `(bytes_downloaded, total_bytes)`.
    /// Returns the local path to the downloaded model file.
    pub async fn ensure_model(
        &self,
        model: &VoiceModel,
        progress: Option<ProgressCallback>,
    ) -> Result<PathBuf, PluginError> {
        let info = self.find_model(model).ok_or_else(|| {
            PluginError::ExecutionFailed(format!("unknown voice model: {model:?}"))
        })?;

        let path = self.cache_dir.join(&info.relative_path);

        // Check if already valid
        if path.exists() {
            if self.verify_sha256(&path, &info.sha256).await? {
                info!(model = %info.name, "model already cached and valid");
                return Ok(path);
            }
            warn!(model = %info.name, "cached model failed integrity check, re-downloading");
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                PluginError::Io(e)
            })?;
        }

        info!(model = %info.name, url = %info.url, "downloading voice model");

        // Download with progress
        let client = reqwest::Client::new();
        let response = client.get(&info.url).send().await.map_err(|e| {
            PluginError::ExecutionFailed(format!("model download failed: {e}"))
        })?;

        let total = response.content_length().unwrap_or(info.size_bytes);
        let mut downloaded: u64 = 0;

        let bytes = response.bytes().await.map_err(|e| {
            PluginError::ExecutionFailed(format!("model download read failed: {e}"))
        })?;

        downloaded = bytes.len() as u64;
        if let Some(ref cb) = progress {
            cb(downloaded, total);
        }

        // Write to disk
        fs::write(&path, &bytes).await.map_err(|e| {
            PluginError::Io(e)
        })?;

        // Verify integrity
        if !self.verify_sha256(&path, &info.sha256).await? {
            let _ = fs::remove_file(&path).await;
            return Err(PluginError::ExecutionFailed(
                format!("model {} failed SHA-256 integrity check after download", info.name),
            ));
        }

        info!(model = %info.name, path = %path.display(), "model downloaded successfully");
        Ok(path)
    }

    /// List all known models with their availability status.
    pub async fn list_models(&self) -> Vec<(ModelInfo, bool)> {
        let mut result = Vec::new();
        for info in &self.registry {
            let available = self.is_model_available(&info.model).await;
            result.push((info.clone(), available));
        }
        result
    }

    /// Verify a file's SHA-256 hash.
    async fn verify_sha256(
        &self,
        path: &Path,
        expected: &str,
    ) -> Result<bool, PluginError> {
        let data = fs::read(path).await.map_err(PluginError::Io)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = format!("{:x}", hasher.finalize());
        Ok(hash == expected)
    }

    fn find_model(&self, model: &VoiceModel) -> Option<ModelInfo> {
        self.registry.iter().find(|m| m.model == *model).cloned()
    }

    /// Default model registry.
    ///
    /// URLs and hashes are placeholders -- updated after VP1 prototype
    /// selects specific model versions.
    fn default_registry() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                model: VoiceModel::SileroVad,
                name: "Silero VAD V5".into(),
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx".into(),
                sha256: "PLACEHOLDER_SHA256_SILERO_VAD".into(),
                relative_path: "vad/silero_vad.onnx".into(),
                size_bytes: 2_000_000,
            },
            ModelInfo {
                model: VoiceModel::StreamingStt,
                name: "Streaming STT (Zipformer int8)".into(),
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2".into(),
                sha256: "PLACEHOLDER_SHA256_STREAMING_STT".into(),
                relative_path: "stt/streaming-zipformer-en".into(),
                size_bytes: 50_000_000,
            },
            ModelInfo {
                model: VoiceModel::StreamingTts,
                name: "Streaming TTS (VITS)".into(),
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-amy-low.tar.bz2".into(),
                sha256: "PLACEHOLDER_SHA256_STREAMING_TTS".into(),
                relative_path: "tts/vits-piper-en-amy".into(),
                size_bytes: 30_000_000,
            },
        ]
    }
}
```

**Note:** SHA-256 hashes and exact URLs are placeholder values. They will be filled in after VP1 prototype validation confirms the exact model versions to use.

#### VS1.1.7: VoiceConfig Types in clawft-types

```rust
// Add to crates/clawft-types/src/config/mod.rs
// (new struct, added to the Config root)

use serde::{Deserialize, Serialize};

/// Voice pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Whether voice features are enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Audio input device name. None = system default.
    #[serde(default, alias = "inputDevice")]
    pub input_device: Option<String>,

    /// Audio output device name. None = system default.
    #[serde(default, alias = "outputDevice")]
    pub output_device: Option<String>,

    /// Voice activity detection configuration.
    #[serde(default)]
    pub vad: VadConfig,

    /// Speech-to-text configuration.
    #[serde(default)]
    pub stt: SttConfig,

    /// Text-to-speech configuration.
    #[serde(default)]
    pub tts: TtsConfig,

    /// Talk mode configuration.
    #[serde(default, alias = "talkMode")]
    pub talk_mode: TalkModeConfig,

    /// Model cache directory. Defaults to `~/.clawft/models/voice/`.
    #[serde(default = "default_model_dir", alias = "modelDir")]
    pub model_dir: String,
}

fn default_model_dir() -> String {
    "~/.clawft/models/voice".into()
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            input_device: None,
            output_device: None,
            vad: VadConfig::default(),
            stt: SttConfig::default(),
            tts: TtsConfig::default(),
            talk_mode: TalkModeConfig::default(),
            model_dir: default_model_dir(),
        }
    }
}

/// VAD-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    /// Speech probability threshold (0.0 - 1.0). Default 0.5.
    #[serde(default = "default_vad_threshold")]
    pub threshold: f32,

    /// Minimum speech duration in milliseconds before triggering. Default 250.
    #[serde(default = "default_min_speech_ms", alias = "minSpeechDurationMs")]
    pub min_speech_duration_ms: u32,

    /// Silence duration in milliseconds to end an utterance. Default 500.
    #[serde(default = "default_silence_ms", alias = "silenceDurationMs")]
    pub silence_duration_ms: u32,
}

fn default_vad_threshold() -> f32 { 0.5 }
fn default_min_speech_ms() -> u32 { 250 }
fn default_silence_ms() -> u32 { 500 }

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold: default_vad_threshold(),
            min_speech_duration_ms: default_min_speech_ms(),
            silence_duration_ms: default_silence_ms(),
        }
    }
}

/// STT-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// STT engine to use. Default "sherpa-onnx".
    #[serde(default = "default_stt_engine")]
    pub engine: String,

    /// Language code. Default "en".
    #[serde(default = "default_language")]
    pub language: String,

    /// Number of decoding threads. Default 2.
    #[serde(default = "default_num_threads", alias = "numThreads")]
    pub num_threads: u32,

    /// Whether to enable streaming partial results. Default true.
    #[serde(default = "default_true_voice", alias = "enablePartialResults")]
    pub enable_partial_results: bool,
}

fn default_stt_engine() -> String { "sherpa-onnx".into() }
fn default_language() -> String { "en".into() }
fn default_num_threads() -> u32 { 2 }
fn default_true_voice() -> bool { true }

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            engine: default_stt_engine(),
            language: default_language(),
            num_threads: default_num_threads(),
            enable_partial_results: true,
        }
    }
}

/// TTS-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// TTS engine to use. Default "sherpa-onnx".
    #[serde(default = "default_tts_engine")]
    pub engine: String,

    /// Voice name / model identifier. Default "amy".
    #[serde(default = "default_voice_name")]
    pub voice: String,

    /// Speaking rate multiplier (1.0 = normal). Default 1.0.
    #[serde(default = "default_speaking_rate", alias = "speakingRate")]
    pub speaking_rate: f32,

    /// Number of synthesis threads. Default 2.
    #[serde(default = "default_num_threads", alias = "numThreads")]
    pub num_threads: u32,
}

fn default_tts_engine() -> String { "sherpa-onnx".into() }
fn default_voice_name() -> String { "amy".into() }
fn default_speaking_rate() -> f32 { 1.0 }

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            engine: default_tts_engine(),
            voice: default_voice_name(),
            speaking_rate: default_speaking_rate(),
            num_threads: default_num_threads(),
        }
    }
}

/// Talk mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkModeConfig {
    /// Silence timeout in seconds before the system assumes the user
    /// has finished speaking. Default 2.0.
    #[serde(default = "default_silence_timeout", alias = "silenceTimeout")]
    pub silence_timeout_secs: f32,

    /// Whether to enable interruption detection (stop TTS when user
    /// starts speaking). Default true.
    #[serde(default = "default_true_voice", alias = "enableInterruption")]
    pub enable_interruption: bool,

    /// Whether to play a chime sound on state transitions. Default false.
    #[serde(default, alias = "playChime")]
    pub play_chime: bool,
}

fn default_silence_timeout() -> f32 { 2.0 }

impl Default for TalkModeConfig {
    fn default() -> Self {
        Self {
            silence_timeout_secs: default_silence_timeout(),
            enable_interruption: true,
            play_chime: false,
        }
    }
}
```

Add to the root `Config` struct:

```rust
// In crates/clawft-types/src/config/mod.rs, add to struct Config:
    /// Voice pipeline configuration.
    #[serde(default)]
    pub voice: VoiceConfig,
```

#### VS1.1.8: Feature Flag Wiring

Verify the feature flag chain compiles across crates:

```toml
# Root Cargo.toml workspace features
[workspace.features]
voice = ["clawft-plugin/voice", "clawft-cli/voice"]

# crates/clawft-cli/Cargo.toml
[features]
voice = ["clawft-plugin/voice"]

# crates/clawft-tools/Cargo.toml
[features]
voice = ["clawft-plugin/voice"]
```

---

### Task VS1.2: STT + TTS (Week 2)

#### VS1.2.1: SpeechToText -- sherpa-rs Streaming Recognizer

```rust
// crates/clawft-plugin/src/voice/stt.rs

use std::sync::Arc;

use sherpa_rs::recognizer::{
    OnlineRecognizer, OnlineRecognizerConfig, OnlineStream,
};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::PluginError;
use super::capture::SAMPLE_RATE;
use super::models::{ModelDownloadManager, VoiceModel};

/// Transcription result from the STT engine.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// The transcribed text.
    pub text: String,
    /// Whether this is a final result (utterance complete) or partial.
    pub is_final: bool,
    /// Confidence score (0.0 - 1.0), if available.
    pub confidence: Option<f32>,
}

/// Callback for partial transcription results.
pub type PartialResultCallback = Arc<dyn Fn(TranscriptionResult) + Send + Sync>;

/// Streaming speech-to-text engine backed by sherpa-rs.
///
/// Accepts audio chunks from `AudioCapture` and produces transcription
/// results. Supports streaming partial results for real-time feedback.
pub struct SpeechToText {
    recognizer: OnlineRecognizer,
    /// Optional callback invoked for each partial result.
    partial_callback: Option<PartialResultCallback>,
}

impl SpeechToText {
    /// Create a new STT engine.
    ///
    /// `model_mgr` is used to locate model files.
    /// `num_threads` controls the number of ONNX inference threads.
    pub async fn new(
        model_mgr: &ModelDownloadManager,
        num_threads: u32,
        partial_callback: Option<PartialResultCallback>,
    ) -> Result<Self, PluginError> {
        let model_path = model_mgr
            .ensure_model(&VoiceModel::StreamingStt, None)
            .await?;

        let model_dir = model_path.to_string_lossy().to_string();

        info!(model = %model_dir, threads = num_threads, "initializing STT engine");

        // Configure sherpa-rs online recognizer for streaming
        let config = OnlineRecognizerConfig {
            model_dir,
            num_threads: num_threads as i32,
            sample_rate: SAMPLE_RATE as i32,
            feature_dim: 80,
            decoding_method: "greedy_search".into(),
            ..Default::default()
        };

        let recognizer = OnlineRecognizer::new(config).map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to create STT recognizer: {e}"))
        })?;

        Ok(Self {
            recognizer,
            partial_callback,
        })
    }

    /// Create a new recognition stream for a single utterance.
    pub fn create_stream(&self) -> Result<SttStream, PluginError> {
        let stream = self.recognizer.create_stream().map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to create STT stream: {e}"))
        })?;
        Ok(SttStream {
            stream,
            recognizer: &self.recognizer,
            partial_callback: self.partial_callback.clone(),
        })
    }

    /// Transcribe a complete audio buffer (non-streaming, one-shot).
    ///
    /// Convenience method for the `voice_listen` tool.
    pub async fn transcribe_buffer(
        &self,
        samples: &[f32],
    ) -> Result<String, PluginError> {
        let mut stream = self.create_stream()?;
        stream.accept_waveform(samples);
        stream.finalize()
    }
}

/// A streaming recognition session for a single utterance.
pub struct SttStream<'a> {
    stream: OnlineStream,
    recognizer: &'a OnlineRecognizer,
    partial_callback: Option<PartialResultCallback>,
}

impl<'a> SttStream<'a> {
    /// Feed audio samples into the recognizer.
    ///
    /// Emits partial results via the callback if configured.
    pub fn accept_waveform(&mut self, samples: &[f32]) {
        self.stream.accept_waveform(SAMPLE_RATE as i32, samples);

        // Decode and emit partial result
        if self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
            let text = self.recognizer.get_result(&self.stream);
            if !text.is_empty() {
                if let Some(ref cb) = self.partial_callback {
                    cb(TranscriptionResult {
                        text: text.clone(),
                        is_final: false,
                        confidence: None,
                    });
                }
                debug!(partial = %text, "STT partial result");
            }
        }
    }

    /// Finalize the stream and return the complete transcription.
    pub fn finalize(&mut self) -> Result<String, PluginError> {
        // Signal end of audio
        self.stream.input_finished();

        // Final decode pass
        while self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);
        }

        let text = self.recognizer.get_result(&self.stream);
        let text = text.trim().to_string();

        if let Some(ref cb) = self.partial_callback {
            cb(TranscriptionResult {
                text: text.clone(),
                is_final: true,
                confidence: None,
            });
        }

        info!(text = %text, "STT final result");
        Ok(text)
    }
}
```

#### VS1.2.2: TextToSpeech -- sherpa-rs Streaming Synthesizer

```rust
// crates/clawft-plugin/src/voice/tts.rs

use std::sync::Arc;

use sherpa_rs::tts::{OfflineTts, OfflineTtsConfig};
use tokio::sync::watch;
use tracing::{debug, info};

use crate::PluginError;
use super::capture::SAMPLE_RATE;
use super::models::{ModelDownloadManager, VoiceModel};

/// Handle for cancelling an in-progress TTS synthesis.
#[derive(Clone)]
pub struct TtsAbortHandle {
    cancel_tx: watch::Sender<bool>,
}

impl TtsAbortHandle {
    /// Cancel the in-progress synthesis.
    pub fn abort(&self) {
        let _ = self.cancel_tx.send(true);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.cancel_tx.borrow()
    }
}

/// Result of a TTS synthesis operation.
#[derive(Debug, Clone)]
pub struct SynthesisResult {
    /// Raw audio samples (mono, 16kHz, f32).
    pub samples: Vec<f32>,
    /// Sample rate of the output.
    pub sample_rate: u32,
    /// Whether synthesis was cancelled before completion.
    pub was_cancelled: bool,
}

/// Text-to-speech engine backed by sherpa-rs (VITS model).
///
/// Synthesizes text into audio samples. Supports streaming output
/// (start playback before full synthesis completes) and cancellation
/// via `TtsAbortHandle` for interruption support.
pub struct TextToSpeech {
    tts: OfflineTts,
    /// Speaking rate multiplier.
    speaking_rate: f32,
}

impl TextToSpeech {
    /// Create a new TTS engine.
    ///
    /// `model_mgr` is used to locate model files.
    /// `num_threads` controls the number of synthesis threads.
    /// `speaking_rate` is a multiplier (1.0 = normal speed).
    pub async fn new(
        model_mgr: &ModelDownloadManager,
        num_threads: u32,
        speaking_rate: f32,
    ) -> Result<Self, PluginError> {
        let model_path = model_mgr
            .ensure_model(&VoiceModel::StreamingTts, None)
            .await?;

        let model_dir = model_path.to_string_lossy().to_string();

        info!(model = %model_dir, threads = num_threads, rate = speaking_rate, "initializing TTS engine");

        let config = OfflineTtsConfig {
            model_dir,
            num_threads: num_threads as i32,
            ..Default::default()
        };

        let tts = OfflineTts::new(config).map_err(|e| {
            PluginError::ExecutionFailed(format!("failed to create TTS engine: {e}"))
        })?;

        Ok(Self {
            tts,
            speaking_rate,
        })
    }

    /// Synthesize text into audio samples.
    ///
    /// Returns a `SynthesisResult` with raw f32 samples.
    pub fn synthesize(&self, text: &str) -> Result<SynthesisResult, PluginError> {
        info!(text_len = text.len(), "synthesizing speech");

        let audio = self.tts.generate(text, 0, self.speaking_rate).map_err(|e| {
            PluginError::ExecutionFailed(format!("TTS synthesis failed: {e}"))
        })?;

        Ok(SynthesisResult {
            samples: audio.samples,
            sample_rate: audio.sample_rate as u32,
            was_cancelled: false,
        })
    }

    /// Synthesize text with cancellation support.
    ///
    /// Returns a `TtsAbortHandle` that can be used to cancel synthesis,
    /// and sends audio chunks through the provided channel as they
    /// are generated.
    pub fn synthesize_streaming(
        &self,
        text: &str,
        chunk_tx: tokio::sync::mpsc::Sender<Vec<f32>>,
    ) -> Result<TtsAbortHandle, PluginError> {
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let handle = TtsAbortHandle {
            cancel_tx: cancel_tx.clone(),
        };

        // Split text into sentences for chunked synthesis
        let sentences: Vec<&str> = text
            .split_inclusive(&['.', '!', '?', ';'][..])
            .filter(|s| !s.trim().is_empty())
            .collect();

        info!(
            sentences = sentences.len(),
            "streaming TTS in sentence chunks"
        );

        for sentence in sentences {
            if *cancel_rx.borrow() {
                debug!("TTS synthesis cancelled");
                return Ok(handle);
            }

            let audio = self.tts.generate(sentence.trim(), 0, self.speaking_rate).map_err(|e| {
                PluginError::ExecutionFailed(format!("TTS synthesis failed: {e}"))
            })?;

            if chunk_tx.blocking_send(audio.samples).is_err() {
                break; // Receiver dropped
            }
        }

        Ok(handle)
    }
}
```

#### VS1.2.3 + VS1.2.4 + VS1.2.5: Partial Results, Streaming Playback, Cancellation

These capabilities are embedded in the `SpeechToText` (partial result callback) and `TextToSpeech` (streaming synthesis + `TtsAbortHandle`) structs defined above. The key integration points:

- **Partial STT results:** The `PartialResultCallback` in `SpeechToText::new()` is invoked after each `decode()` call with `is_final: false`. The `voice_listen` tool and Talk Mode both use this for real-time UI feedback.
- **Streaming TTS playback:** `TextToSpeech::synthesize_streaming()` splits text into sentences and sends audio chunks through an mpsc channel. `AudioPlayback` consumes these chunks as they arrive, starting playback before full synthesis completes.
- **TTS cancellation:** `TtsAbortHandle::abort()` sets a watch flag checked between sentence boundaries. Talk Mode's interruption detector calls this when the user starts speaking during TTS output.

#### VS1.2.6: voice_listen Tool -- On-Demand Transcription

```rust
// crates/clawft-tools/src/voice_listen.rs

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use clawft_plugin::{PluginError, Tool, ToolContext};
#[cfg(feature = "voice")]
use clawft_plugin::voice::{AudioCapture, SpeechToText, VoiceActivityDetector};
#[cfg(feature = "voice")]
use clawft_plugin::voice::models::ModelDownloadManager;
#[cfg(feature = "voice")]
use clawft_plugin::voice::vad::VadState;

/// Tool: voice_listen
///
/// Captures audio from the microphone, detects speech via VAD,
/// transcribes the utterance via STT, and returns the text.
///
/// Parameters:
/// - `timeout_secs` (optional, default 30): Maximum seconds to wait
///   for speech before returning empty.
/// - `language` (optional, default "en"): Language code for STT.
pub struct VoiceListenTool;

#[async_trait]
impl Tool for VoiceListenTool {
    fn name(&self) -> &str {
        "voice_listen"
    }

    fn description(&self) -> &str {
        "Listen to microphone, transcribe speech to text. Returns the transcribed utterance."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "timeout_secs": {
                    "type": "integer",
                    "description": "Maximum seconds to wait for speech (default 30)",
                    "default": 30
                },
                "language": {
                    "type": "string",
                    "description": "Language code for STT (default 'en')",
                    "default": "en"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: &dyn ToolContext,
    ) -> Result<Value, PluginError> {
        #[cfg(not(feature = "voice"))]
        {
            return Err(PluginError::NotImplemented(
                "voice feature is not enabled; rebuild with --features voice".into(),
            ));
        }

        #[cfg(feature = "voice")]
        {
            let timeout_secs = params["timeout_secs"].as_u64().unwrap_or(30);
            let _language = params["language"]
                .as_str()
                .unwrap_or("en")
                .to_string();

            info!(timeout = timeout_secs, "voice_listen: starting capture");

            // Initialize components
            let model_mgr = ModelDownloadManager::new(
                ModelDownloadManager::default_cache_dir(),
            );
            let mut capture = AudioCapture::start_default()?;

            let vad_path = model_mgr
                .ensure_model(
                    &clawft_plugin::voice::models::VoiceModel::SileroVad,
                    None,
                )
                .await?;
            let mut vad = VoiceActivityDetector::new(
                &vad_path.to_string_lossy(),
                0.5,
                250,
            )?;

            let stt = SpeechToText::new(&model_mgr, 2, None).await?;
            let mut stream = stt.create_stream()?;

            let deadline = tokio::time::Instant::now()
                + tokio::time::Duration::from_secs(timeout_secs);

            let mut speech_started = false;

            loop {
                if tokio::time::Instant::now() > deadline {
                    info!("voice_listen: timeout reached");
                    break;
                }

                let chunk = tokio::time::timeout(
                    tokio::time::Duration::from_millis(100),
                    capture.rx.recv(),
                )
                .await;

                let chunk = match chunk {
                    Ok(Some(c)) => c,
                    Ok(None) => break,   // Channel closed
                    Err(_) => continue,  // Timeout, try again
                };

                let state = vad.process(&chunk);

                match state {
                    VadState::Speech => {
                        speech_started = true;
                        stream.accept_waveform(&chunk);
                    }
                    VadState::SpeechEnd if speech_started => {
                        stream.accept_waveform(&chunk);
                        break;
                    }
                    _ => {}
                }
            }

            let text = if speech_started {
                stream.finalize()?
            } else {
                String::new()
            };

            Ok(json!({
                "text": text,
                "speech_detected": speech_started,
            }))
        }
    }
}
```

#### VS1.2.7: voice_speak Tool -- On-Demand TTS

```rust
// crates/clawft-tools/src/voice_speak.rs

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use clawft_plugin::{PluginError, Tool, ToolContext};
#[cfg(feature = "voice")]
use clawft_plugin::voice::{AudioPlayback, TextToSpeech};
#[cfg(feature = "voice")]
use clawft_plugin::voice::models::ModelDownloadManager;

/// Tool: voice_speak
///
/// Synthesizes text to speech and plays it through the speaker.
///
/// Parameters:
/// - `text` (required): The text to speak.
/// - `voice` (optional, default "amy"): Voice name/model.
/// - `rate` (optional, default 1.0): Speaking rate multiplier.
pub struct VoiceSpeakTool;

#[async_trait]
impl Tool for VoiceSpeakTool {
    fn name(&self) -> &str {
        "voice_speak"
    }

    fn description(&self) -> &str {
        "Speak text aloud through the system speaker using text-to-speech."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to speak aloud"
                },
                "voice": {
                    "type": "string",
                    "description": "Voice name (default 'amy')",
                    "default": "amy"
                },
                "rate": {
                    "type": "number",
                    "description": "Speaking rate multiplier (default 1.0)",
                    "default": 1.0
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: &dyn ToolContext,
    ) -> Result<Value, PluginError> {
        #[cfg(not(feature = "voice"))]
        {
            return Err(PluginError::NotImplemented(
                "voice feature is not enabled; rebuild with --features voice".into(),
            ));
        }

        #[cfg(feature = "voice")]
        {
            let text = params["text"]
                .as_str()
                .ok_or_else(|| PluginError::ExecutionFailed("'text' parameter is required".into()))?
                .to_string();
            let rate = params["rate"].as_f64().unwrap_or(1.0) as f32;

            info!(text_len = text.len(), rate, "voice_speak: synthesizing");

            let model_mgr = ModelDownloadManager::new(
                ModelDownloadManager::default_cache_dir(),
            );

            let tts = TextToSpeech::new(&model_mgr, 2, rate).await?;
            let result = tts.synthesize(&text)?;

            // Play through speaker
            let mut playback = AudioPlayback::start_default()?;
            playback.write_samples(&result.samples);

            // Wait for playback to complete (approximate based on sample count)
            let duration_ms = (result.samples.len() as f64
                / result.sample_rate as f64
                * 1000.0) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms + 200)).await;

            Ok(json!({
                "spoken": true,
                "text_length": text.len(),
                "audio_duration_ms": duration_ms,
            }))
        }
    }
}
```

#### VS1.2.8: CLI Commands

```rust
// crates/clawft-cli/src/commands/voice.rs

use clap::Subcommand;

/// Voice pipeline commands.
#[derive(Debug, Subcommand)]
pub enum VoiceCommand {
    /// Download voice models and validate audio hardware.
    Setup,

    /// Test microphone: capture 3 seconds and report audio levels.
    TestMic {
        /// Duration in seconds to capture (default 3).
        #[arg(long, default_value = "3")]
        duration: u32,
    },

    /// Test speaker: synthesize and play a test phrase.
    TestSpeak {
        /// Text to speak (default: "Hello, I am ClawFT voice assistant.").
        #[arg(long, default_value = "Hello, I am ClawFT voice assistant.")]
        text: String,
    },

    /// Start Talk Mode: continuous voice conversation loop.
    Talk {
        /// Silence timeout in seconds (default from config).
        #[arg(long)]
        silence_timeout: Option<f32>,

        /// Disable interruption detection.
        #[arg(long)]
        no_interrupt: bool,
    },
}
```

**`weft voice setup` implementation:**

```rust
/// Handler for `weft voice setup`.
///
/// 1. Check audio devices (list inputs and outputs).
/// 2. Download required models (VAD, STT, TTS) with progress bars.
/// 3. Run a quick mic test (1 second capture, report level).
/// 4. Run a quick TTS test (speak "Setup complete").
pub async fn handle_voice_setup() -> Result<(), anyhow::Error> {
    use clawft_plugin::voice::capture::AudioCapture;
    use clawft_plugin::voice::models::{ModelDownloadManager, VoiceModel};
    use clawft_plugin::voice::playback::AudioPlayback;

    println!("=== ClawFT Voice Setup ===\n");

    // 1. Check audio devices
    println!("Checking audio devices...");
    let inputs = AudioCapture::list_devices()?;
    println!("  Input devices: {}", if inputs.is_empty() {
        "NONE FOUND".to_string()
    } else {
        inputs.join(", ")
    });

    let outputs = AudioPlayback::list_devices()?;
    println!("  Output devices: {}", if outputs.is_empty() {
        "NONE FOUND".to_string()
    } else {
        outputs.join(", ")
    });

    if inputs.is_empty() {
        anyhow::bail!("No audio input devices found. Ensure a microphone is connected.");
    }

    // 2. Download models
    let mgr = ModelDownloadManager::new(ModelDownloadManager::default_cache_dir());

    for model in &[VoiceModel::SileroVad, VoiceModel::StreamingStt, VoiceModel::StreamingTts] {
        print!("  Downloading {:?}...", model);
        let progress = Box::new(|downloaded: u64, total: u64| {
            let pct = if total > 0 { downloaded * 100 / total } else { 0 };
            print!("\r  Downloading {:?}... {}%", model, pct);
        });
        mgr.ensure_model(model, Some(progress)).await?;
        println!(" OK");
    }

    // 3. Quick mic test
    println!("\nTesting microphone (1 second)...");
    let mut capture = AudioCapture::start_default()?;
    let mut max_level: f32 = 0.0;
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(1);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(chunk)) = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            capture.rx.recv(),
        ).await {
            let chunk_max = chunk.iter().copied().fold(0.0f32, f32::max);
            max_level = max_level.max(chunk_max);
        }
    }
    let db = 20.0 * max_level.max(1e-10).log10();
    println!("  Peak level: {:.1} dB ({})",
        db,
        if db > -30.0 { "good" } else { "low -- check mic" }
    );

    println!("\n=== Voice setup complete ===");
    Ok(())
}
```

**`weft voice test-mic` implementation:**

```rust
/// Handler for `weft voice test-mic`.
///
/// Captures audio for the specified duration and reports:
/// - Peak amplitude (dB)
/// - RMS level (dB)
/// - VAD speech detection events
pub async fn handle_voice_test_mic(duration: u32) -> Result<(), anyhow::Error> {
    use clawft_plugin::voice::capture::AudioCapture;
    use clawft_plugin::voice::vad::{VoiceActivityDetector, VadState};
    use clawft_plugin::voice::models::{ModelDownloadManager, VoiceModel};

    println!("Testing microphone for {} seconds...", duration);
    println!("Speak now!\n");

    let model_mgr = ModelDownloadManager::new(ModelDownloadManager::default_cache_dir());
    let vad_path = model_mgr
        .ensure_model(&VoiceModel::SileroVad, None)
        .await?;

    let mut capture = AudioCapture::start_default()?;
    let mut vad = VoiceActivityDetector::new(
        &vad_path.to_string_lossy(),
        0.5,
        250,
    )?;

    let mut peak: f32 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let mut sample_count: u64 = 0;
    let mut speech_events: u32 = 0;

    let deadline = tokio::time::Instant::now()
        + tokio::time::Duration::from_secs(duration as u64);

    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(chunk)) = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            capture.rx.recv(),
        ).await {
            for &s in &chunk {
                peak = peak.max(s.abs());
                sum_sq += (s as f64) * (s as f64);
                sample_count += 1;
            }

            let state = vad.process(&chunk);
            if state == VadState::Speech {
                speech_events += 1;
            }
        }
    }

    let rms = if sample_count > 0 {
        (sum_sq / sample_count as f64).sqrt() as f32
    } else {
        0.0
    };

    let peak_db = 20.0 * peak.max(1e-10).log10();
    let rms_db = 20.0 * rms.max(1e-10).log10();

    println!("Results:");
    println!("  Peak level:    {:.1} dB", peak_db);
    println!("  RMS level:     {:.1} dB", rms_db);
    println!("  Speech events: {}", speech_events);
    println!("  Status:        {}", if peak_db > -30.0 {
        "PASS"
    } else {
        "LOW -- check microphone"
    });

    Ok(())
}
```

**`weft voice test-speak` implementation:**

```rust
/// Handler for `weft voice test-speak`.
///
/// Synthesizes the given text and plays it through the speaker.
pub async fn handle_voice_test_speak(text: String) -> Result<(), anyhow::Error> {
    use clawft_plugin::voice::models::ModelDownloadManager;
    use clawft_plugin::voice::playback::AudioPlayback;
    use clawft_plugin::voice::tts::TextToSpeech;

    println!("Speaking: \"{}\"", text);

    let model_mgr = ModelDownloadManager::new(ModelDownloadManager::default_cache_dir());
    let tts = TextToSpeech::new(&model_mgr, 2, 1.0).await?;

    let result = tts.synthesize(&text)?;
    println!("  Synthesized {} samples ({:.1}s)",
        result.samples.len(),
        result.samples.len() as f64 / result.sample_rate as f64,
    );

    let mut playback = AudioPlayback::start_default()?;
    playback.write_samples(&result.samples);

    let duration_ms = (result.samples.len() as f64 / result.sample_rate as f64 * 1000.0) as u64;
    tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms + 500)).await;

    println!("  Done.");
    Ok(())
}
```

---

### Task VS1.3: VoiceChannel + Talk Mode (Week 3)

#### VS1.3.1: VoiceChannel Implementing ChannelAdapter

```rust
// crates/clawft-plugin/src/voice/channel.rs

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::error::PluginError;
use crate::message::MessagePayload;
use crate::traits::{ChannelAdapter, ChannelAdapterHost};
use super::capture::AudioCapture;
use super::playback::AudioPlayback;
use super::stt::SpeechToText;
use super::tts::TextToSpeech;
use super::vad::{VoiceActivityDetector, VadState};
use super::models::ModelDownloadManager;

/// Voice channel state reported via WebSocket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceStatus {
    /// Pipeline is idle, not processing audio.
    Idle,
    /// Listening for speech (VAD active, no speech detected yet).
    Listening,
    /// Speech detected, actively transcribing.
    Transcribing,
    /// Agent is processing the transcription (thinking).
    Processing,
    /// Agent response is being spoken via TTS.
    Speaking,
}

/// A ChannelAdapter implementation for the local voice pipeline.
///
/// Bridges the audio pipeline (mic -> VAD -> STT) to the agent
/// MessageBus as `InboundMessage`, and routes outbound agent
/// responses through TTS -> speaker.
pub struct VoiceChannel {
    model_mgr: Arc<ModelDownloadManager>,
    /// Sender for voice status updates (consumed by WebSocket handler).
    status_tx: mpsc::Sender<VoiceStatus>,
    /// Current voice status.
    status: Arc<Mutex<VoiceStatus>>,
}

impl VoiceChannel {
    pub fn new(
        model_mgr: Arc<ModelDownloadManager>,
        status_tx: mpsc::Sender<VoiceStatus>,
    ) -> Self {
        Self {
            model_mgr,
            status_tx,
            status: Arc::new(Mutex::new(VoiceStatus::Idle)),
        }
    }

    async fn set_status(&self, status: VoiceStatus) {
        let mut current = self.status.lock().await;
        if *current != status {
            *current = status;
            let _ = self.status_tx.send(status).await;
            debug!(status = ?status, "voice channel status changed");
        }
    }
}

#[async_trait]
impl ChannelAdapter for VoiceChannel {
    fn name(&self) -> &str {
        "voice"
    }

    fn display_name(&self) -> &str {
        "Voice (Local Microphone)"
    }

    fn supports_threads(&self) -> bool {
        false
    }

    fn supports_media(&self) -> bool {
        true // Supports Binary payloads for audio
    }

    /// Start the voice pipeline. Runs until cancelled.
    ///
    /// Loop: listen -> VAD -> STT -> deliver_inbound -> wait for response
    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        info!("voice channel starting");

        let stt = SpeechToText::new(&self.model_mgr, 2, None).await?;

        let vad_path = self.model_mgr
            .ensure_model(&super::models::VoiceModel::SileroVad, None)
            .await?;
        let mut vad = VoiceActivityDetector::new(
            &vad_path.to_string_lossy(),
            0.5,
            250,
        )?;

        let mut capture = AudioCapture::start_default()?;

        self.set_status(VoiceStatus::Listening).await;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            // Wait for audio chunk
            let chunk = tokio::select! {
                _ = cancel.cancelled() => break,
                chunk = capture.rx.recv() => {
                    match chunk {
                        Some(c) => c,
                        None => break, // Channel closed
                    }
                }
            };

            let state = vad.process(&chunk);

            match state {
                VadState::Speech => {
                    self.set_status(VoiceStatus::Transcribing).await;
                    // Accumulate speech into STT stream
                    let mut stream = stt.create_stream()?;
                    stream.accept_waveform(&chunk);

                    // Continue until speech ends
                    loop {
                        let next = tokio::select! {
                            _ = cancel.cancelled() => break,
                            c = capture.rx.recv() => match c {
                                Some(c) => c,
                                None => break,
                            }
                        };

                        let next_state = vad.process(&next);
                        stream.accept_waveform(&next);

                        if next_state == VadState::SpeechEnd {
                            break;
                        }
                    }

                    // Finalize transcription
                    let text = stream.finalize()?;
                    if text.is_empty() {
                        self.set_status(VoiceStatus::Listening).await;
                        vad.reset();
                        continue;
                    }

                    info!(text = %text, "voice channel: delivering transcription");
                    self.set_status(VoiceStatus::Processing).await;

                    // Deliver as inbound message
                    let payload = MessagePayload::text(&text);
                    let metadata = HashMap::from([
                        ("source".into(), serde_json::json!("voice")),
                        ("channel".into(), serde_json::json!("voice")),
                    ]);

                    host.deliver_inbound(
                        "voice",
                        "local-user",
                        "voice-session",
                        payload,
                        metadata,
                    )
                    .await?;

                    self.set_status(VoiceStatus::Listening).await;
                    vad.reset();
                }
                _ => {
                    // Silence or non-speech, continue listening
                }
            }
        }

        self.set_status(VoiceStatus::Idle).await;
        info!("voice channel stopped");
        Ok(())
    }

    /// Send an outbound message through TTS.
    ///
    /// Text payloads are synthesized and played through the speaker.
    /// Binary payloads with audio MIME types are played directly.
    async fn send(
        &self,
        _target: &str,
        payload: &MessagePayload,
    ) -> Result<String, PluginError> {
        self.set_status(VoiceStatus::Speaking).await;

        match payload {
            MessagePayload::Text(text) => {
                let tts = TextToSpeech::new(&self.model_mgr, 2, 1.0).await?;
                let result = tts.synthesize(text)?;

                let mut playback = AudioPlayback::start_default()?;
                playback.write_samples(&result.samples);

                let duration_ms = (result.samples.len() as f64
                    / result.sample_rate as f64
                    * 1000.0) as u64;
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    duration_ms + 100,
                ))
                .await;
            }
            MessagePayload::Binary { mime_type, data } if mime_type.starts_with("audio/") => {
                // Play raw audio directly
                // Convert bytes to f32 samples (assuming 16-bit PCM)
                let samples: Vec<f32> = data
                    .chunks_exact(2)
                    .map(|c| {
                        let sample = i16::from_le_bytes([c[0], c[1]]);
                        sample as f32 / 32768.0
                    })
                    .collect();

                let mut playback = AudioPlayback::start_default()?;
                playback.write_samples(&samples);

                let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as u64;
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    duration_ms + 100,
                ))
                .await;
            }
            _ => {
                return Err(PluginError::ExecutionFailed(
                    "VoiceChannel can only send Text or audio Binary payloads".into(),
                ));
            }
        }

        self.set_status(VoiceStatus::Idle).await;
        Ok("voice-out".into())
    }
}
```

#### VS1.3.4: TalkModeController

```rust
// crates/clawft-plugin/src/voice/talk_mode.rs

use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::PluginError;
use super::capture::AudioCapture;
use super::playback::AudioPlayback;
use super::stt::SpeechToText;
use super::tts::{TextToSpeech, TtsAbortHandle};
use super::vad::{VoiceActivityDetector, VadState};
use super::channel::VoiceStatus;
use super::models::{ModelDownloadManager, VoiceModel};

/// Callback invoked when the user finishes speaking.
/// Receives the transcribed text, returns the agent's response text.
pub type AgentCallback = Arc<dyn Fn(String) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<String, PluginError>> + Send>
> + Send + Sync>;

/// Continuous conversation controller.
///
/// Implements the listen -> transcribe -> think -> speak -> listen loop.
/// Supports interruption detection: if the user speaks during TTS
/// output, synthesis is cancelled and the new utterance is captured.
pub struct TalkModeController {
    model_mgr: Arc<ModelDownloadManager>,
    /// Silence timeout in seconds.
    silence_timeout_secs: f32,
    /// Whether interruption detection is enabled.
    enable_interruption: bool,
    /// Status sender for WebSocket events.
    status_tx: mpsc::Sender<VoiceStatus>,
}

impl TalkModeController {
    pub fn new(
        model_mgr: Arc<ModelDownloadManager>,
        silence_timeout_secs: f32,
        enable_interruption: bool,
        status_tx: mpsc::Sender<VoiceStatus>,
    ) -> Self {
        Self {
            model_mgr,
            silence_timeout_secs,
            enable_interruption,
            status_tx,
        }
    }

    async fn set_status(&self, status: VoiceStatus) {
        let _ = self.status_tx.send(status).await;
    }

    /// Run the Talk Mode loop until cancelled.
    ///
    /// `agent_fn` is called with each transcribed utterance and should
    /// return the agent's response text (which will be spoken via TTS).
    pub async fn run(
        &self,
        agent_fn: AgentCallback,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        info!("talk mode starting");

        // Initialize all pipeline components
        let stt = SpeechToText::new(&self.model_mgr, 2, None).await?;
        let tts = TextToSpeech::new(&self.model_mgr, 2, 1.0).await?;

        let vad_path = self.model_mgr
            .ensure_model(&VoiceModel::SileroVad, None)
            .await?;
        let mut vad = VoiceActivityDetector::new(
            &vad_path.to_string_lossy(),
            0.5,
            250,
        )?;

        let mut capture = AudioCapture::start_default()?;
        let mut playback = AudioPlayback::start_default()?;

        self.set_status(VoiceStatus::Listening).await;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            // ── Phase 1: Listen ──
            let mut speech_chunks: Vec<Vec<f32>> = Vec::new();
            let mut speech_detected = false;

            loop {
                if cancel.is_cancelled() {
                    break;
                }

                let chunk = tokio::select! {
                    _ = cancel.cancelled() => break,
                    c = capture.rx.recv() => match c {
                        Some(c) => c,
                        None => break,
                    }
                };

                let state = vad.process(&chunk);

                match state {
                    VadState::Speech => {
                        if !speech_detected {
                            self.set_status(VoiceStatus::Transcribing).await;
                            speech_detected = true;
                        }
                        speech_chunks.push(chunk);
                    }
                    VadState::SpeechEnd if speech_detected => {
                        speech_chunks.push(chunk);
                        break;
                    }
                    _ => {
                        if speech_detected {
                            // Extended silence after speech started
                            speech_chunks.push(chunk);
                        }
                    }
                }
            }

            if cancel.is_cancelled() || !speech_detected {
                break;
            }

            // ── Phase 2: Transcribe ──
            let all_samples: Vec<f32> = speech_chunks
                .into_iter()
                .flatten()
                .collect();

            let text = stt.transcribe_buffer(&all_samples).await?;

            if text.is_empty() {
                debug!("talk mode: empty transcription, resuming listening");
                vad.reset();
                self.set_status(VoiceStatus::Listening).await;
                continue;
            }

            info!(text = %text, "talk mode: user said");

            // ── Phase 3: Think ──
            self.set_status(VoiceStatus::Processing).await;
            let response = match agent_fn(text).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(%e, "talk mode: agent error");
                    "I'm sorry, I encountered an error processing that.".to_string()
                }
            };

            if response.is_empty() {
                vad.reset();
                self.set_status(VoiceStatus::Listening).await;
                continue;
            }

            info!(response_len = response.len(), "talk mode: agent responded");

            // ── Phase 4: Speak (with interruption detection) ──
            self.set_status(VoiceStatus::Speaking).await;

            let (chunk_tx, mut chunk_rx) = mpsc::channel::<Vec<f32>>(16);
            let abort_handle = tts.synthesize_streaming(&response, chunk_tx)?;

            // Play TTS output while monitoring mic for interruption
            let mut interrupted = false;

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,

                    audio_chunk = chunk_rx.recv() => {
                        match audio_chunk {
                            Some(samples) => {
                                playback.write_samples(&samples);
                            }
                            None => break, // TTS complete
                        }
                    }

                    mic_chunk = capture.rx.recv(), if self.enable_interruption => {
                        if let Some(mic) = mic_chunk {
                            let mic_state = vad.process(&mic);
                            if mic_state == VadState::Speech {
                                info!("talk mode: user interrupted, cancelling TTS");
                                abort_handle.abort();
                                playback.cancel();
                                interrupted = true;
                                break;
                            }
                        }
                    }
                }
            }

            vad.reset();

            if interrupted {
                // User interrupted -- go back to listening immediately
                // The captured speech chunk is already in the VAD state
                playback = AudioPlayback::start_default()?;
            }

            self.set_status(VoiceStatus::Listening).await;
        }

        self.set_status(VoiceStatus::Idle).await;
        info!("talk mode stopped");
        Ok(())
    }
}
```

#### VS1.3.5: Silence Timeout Configuration

The `TalkModeConfig::silence_timeout_secs` field defined in VS1.1.7 controls the VAD silence threshold. The `VoiceActivityDetector::new()` constructor accepts `min_speech_duration_ms` and the sherpa-rs `VadConfig::min_silence_duration` field maps to this.

The silence timeout chain:

```text
VoiceConfig.talk_mode.silence_timeout_secs  (user config, default 2.0)
  -> VadConfig.silence_duration_ms           (converted to ms)
    -> sherpa-rs VadConfig.min_silence_duration (passed to VAD engine)
```

#### VS1.3.6: Interruption Detection

Interruption detection is implemented in `TalkModeController::run()` above. During Phase 4 (Speak), the controller monitors the microphone channel in parallel with TTS playback using `tokio::select!`. When `VadState::Speech` is detected during TTS output:

1. `TtsAbortHandle::abort()` is called to stop synthesis.
2. `AudioPlayback::cancel()` signals the output stream to fill with silence.
3. The loop breaks back to Phase 1 (Listen) immediately.

The `enable_interruption` flag (from `TalkModeConfig`) allows disabling this behavior.

#### VS1.3.7: CLI `weft voice talk`

```rust
/// Handler for `weft voice talk`.
///
/// Starts a continuous Talk Mode session. Ctrl+C to stop.
pub async fn handle_voice_talk(
    silence_timeout: Option<f32>,
    no_interrupt: bool,
) -> Result<(), anyhow::Error> {
    use clawft_plugin::voice::channel::VoiceStatus;
    use clawft_plugin::voice::models::ModelDownloadManager;
    use clawft_plugin::voice::talk_mode::TalkModeController;
    use tokio_util::sync::CancellationToken;

    println!("=== ClawFT Talk Mode ===");
    println!("Speak naturally. Press Ctrl+C to stop.\n");

    let model_mgr = Arc::new(ModelDownloadManager::new(
        ModelDownloadManager::default_cache_dir(),
    ));

    let (status_tx, mut status_rx) = tokio::sync::mpsc::channel(16);

    let controller = TalkModeController::new(
        model_mgr,
        silence_timeout.unwrap_or(2.0),
        !no_interrupt,
        status_tx,
    );

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Print status changes
    tokio::spawn(async move {
        while let Some(status) = status_rx.recv().await {
            let indicator = match status {
                VoiceStatus::Idle => "[IDLE]",
                VoiceStatus::Listening => "[LISTENING...]",
                VoiceStatus::Transcribing => "[TRANSCRIBING...]",
                VoiceStatus::Processing => "[THINKING...]",
                VoiceStatus::Speaking => "[SPEAKING...]",
            };
            println!("  {}", indicator);
        }
    });

    // Handle Ctrl+C
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_for_signal.cancel();
    });

    // Agent callback -- in production this routes through the real agent loop.
    // For the CLI standalone, we use a simple echo/stub.
    let agent_fn: super::talk_mode::AgentCallback = Arc::new(|text| {
        Box::pin(async move {
            // TODO: Wire to real agent loop via MessageBus
            Ok(format!("I heard you say: {}", text))
        })
    });

    controller.run(agent_fn, cancel_clone).await?;

    println!("\nTalk Mode ended.");
    Ok(())
}
```

#### VS1.3.8: WebSocket voice:status Events

```rust
// Added to crates/clawft-services/src/ws/mod.rs (or equivalent WebSocket handler)

use serde::{Deserialize, Serialize};

/// WebSocket event types for voice status updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VoiceWsEvent {
    /// Voice pipeline status changed.
    VoiceStatus {
        status: String, // "idle" | "listening" | "transcribing" | "processing" | "speaking"
    },

    /// Partial transcription result (real-time STT feedback).
    VoicePartialTranscription {
        text: String,
        is_final: bool,
    },

    /// Agent response text (for display alongside TTS).
    VoiceResponse {
        text: String,
    },
}
```

The `VoiceChannel` and `TalkModeController` emit status changes through the `status_tx` mpsc channel. The WebSocket handler subscribes to this channel and broadcasts `VoiceWsEvent::VoiceStatus` messages to connected clients.

---

## 4. Exit Criteria

- [ ] `cargo build -p clawft-plugin --features voice` compiles cleanly
- [ ] `cargo build -p clawft-plugin --features voice-stt` compiles cleanly (STT only)
- [ ] `cargo build -p clawft-plugin --features voice-tts` compiles cleanly (TTS only)
- [ ] `cargo build -p clawft-plugin --features voice-vad` compiles cleanly (VAD + capture only)
- [ ] `cargo build -p clawft-plugin` compiles cleanly (no voice, zero voice code included)
- [ ] `AudioCapture` opens default mic and produces f32 audio chunks
- [ ] `AudioPlayback` plays f32 audio chunks through default speaker
- [ ] `VoiceActivityDetector` correctly classifies speech vs silence (< 3% false trigger rate)
- [ ] `ModelDownloadManager` downloads, caches, and SHA-256-validates voice models
- [ ] `VoiceConfig` deserializes from JSON with defaults, round-trips correctly
- [ ] `SpeechToText` transcribes audio buffer with > 90% accuracy on common English phrases
- [ ] `SpeechToText` emits partial results via callback during streaming recognition
- [ ] `TextToSpeech` synthesizes text to audio samples
- [ ] `TextToSpeech::synthesize_streaming()` sends audio chunks through mpsc channel
- [ ] `TtsAbortHandle::abort()` stops synthesis within one sentence boundary
- [ ] `voice_listen` tool captures mic -> VAD -> STT -> returns transcription JSON
- [ ] `voice_speak` tool takes text -> TTS -> speaker playback
- [ ] `weft voice setup` downloads models and tests mic/speaker
- [ ] `weft voice test-mic` reports audio levels and VAD events
- [ ] `weft voice test-speak` synthesizes and plays test phrase
- [ ] `VoiceChannel` implements `ChannelAdapter` and delivers transcriptions via `deliver_inbound`
- [ ] `VoiceChannel::send()` routes text through TTS to speaker
- [ ] `TalkModeController` runs the full listen -> transcribe -> think -> speak loop
- [ ] Interruption detection: user speech during TTS cancels synthesis within 50ms
- [ ] WebSocket `voice:status` events emitted on state transitions
- [ ] `weft voice talk` starts Talk Mode and responds to Ctrl+C
- [ ] `cargo clippy -p clawft-plugin --features voice -- -D warnings` is clean
- [ ] `cargo test -p clawft-plugin --features voice` -- all tests pass
- [ ] End-to-end latency (speech end to first TTS byte) < 3 seconds

---

## 5. Test Plan

### 5.1 Unit Tests (crates/clawft-plugin/src/voice/)

All tests use `#[cfg(test)]` modules within each file.

| Test | File | Description |
|------|------|-------------|
| `test_audio_format_constants` | `capture.rs` | Assert `SAMPLE_RATE == 16000`, `CHANNELS == 1`, `CHUNK_SAMPLES == 480`. |
| `test_list_devices_returns_vec` | `capture.rs` | `AudioCapture::list_devices()` returns `Ok(Vec)` (may be empty in CI). |
| `test_vad_state_transitions` | `vad.rs` | Feed known speech+silence WAV through VAD, assert correct `VadState` sequence. |
| `test_vad_silence_returns_silence` | `vad.rs` | Feed 1 second of zeros, assert all states are `VadState::Silence`. |
| `test_vad_reset` | `vad.rs` | After `reset()`, `prev_is_speech` is false and accumulator is zero. |
| `test_model_download_manager_default_dir` | `models.rs` | `default_cache_dir()` ends with `.clawft/models/voice`. |
| `test_model_registry_has_all_models` | `models.rs` | Registry contains `SileroVad`, `StreamingStt`, `StreamingTts`. |
| `test_model_path_returns_none_if_missing` | `models.rs` | `model_path()` returns `None` for non-existent model. |
| `test_sha256_verification` | `models.rs` | Write known bytes to temp file, verify correct hash passes and wrong hash fails. |
| `test_transcription_result_fields` | `stt.rs` | Construct `TranscriptionResult`, verify `text`, `is_final`, `confidence` fields. |
| `test_tts_abort_handle` | `tts.rs` | Create `TtsAbortHandle`, verify `is_cancelled()` is false, call `abort()`, verify true. |
| `test_synthesis_result_fields` | `tts.rs` | Construct `SynthesisResult`, verify `samples`, `sample_rate`, `was_cancelled`. |
| `test_voice_status_serde` | `channel.rs` | Serialize each `VoiceStatus` variant to JSON and deserialize back. |
| `test_voice_channel_metadata` | `channel.rs` | Assert `name() == "voice"`, `display_name()` contains "Voice", `supports_media() == true`. |

### 5.2 Config Tests (crates/clawft-types/)

| Test | Description |
|------|-------------|
| `test_voice_config_defaults` | `VoiceConfig::default()` has `enabled: false`, VAD threshold 0.5, silence 500ms, etc. |
| `test_voice_config_serde_roundtrip` | Serialize `VoiceConfig` to JSON and back, verify all fields preserved. |
| `test_voice_config_camel_case_aliases` | JSON with `"inputDevice"`, `"talkMode"`, `"modelDir"` deserializes correctly. |
| `test_voice_config_in_root_config` | Root `Config` with `"voice": {"enabled": true}` deserializes correctly. |
| `test_voice_config_absent_defaults` | Root `Config` with no `"voice"` field has `VoiceConfig::default()`. |

### 5.3 Tool Tests (crates/clawft-tools/)

| Test | Description |
|------|-------------|
| `test_voice_listen_schema` | `VoiceListenTool.parameters_schema()` has `timeout_secs` and `language` properties. |
| `test_voice_listen_name` | `VoiceListenTool.name() == "voice_listen"`. |
| `test_voice_speak_schema` | `VoiceSpeakTool.parameters_schema()` has required `text` property. |
| `test_voice_speak_name` | `VoiceSpeakTool.name() == "voice_speak"`. |
| `test_voice_listen_no_voice_feature` | Without `voice` feature, `execute()` returns `PluginError::NotImplemented`. |
| `test_voice_speak_no_voice_feature` | Without `voice` feature, `execute()` returns `PluginError::NotImplemented`. |

### 5.4 Integration Tests (tests/)

These require audio hardware or pre-recorded WAV files.

| Test | Description |
|------|-------------|
| `test_capture_to_vad_pipeline` | Feed pre-recorded WAV (known speech) through `AudioCapture` mock -> `VoiceActivityDetector`, assert speech detected. |
| `test_vad_to_stt_pipeline` | Feed speech WAV through VAD -> STT, assert transcription contains expected keywords. |
| `test_tts_to_playback_pipeline` | Synthesize "hello world" -> verify output samples are non-empty and within [-1, 1] range. |
| `test_tts_cancellation` | Start streaming synthesis, abort after first chunk, verify `was_cancelled`. |
| `test_voice_channel_deliver_inbound` | Mock `ChannelAdapterHost`, feed speech WAV through `VoiceChannel`, verify `deliver_inbound` called with transcribed text. |
| `test_talk_mode_single_turn` | Feed one speech WAV, mock agent returns "test response", verify TTS output produced. |
| `test_interruption_detection` | During TTS playback, inject speech into mic channel, verify TTS abort and loop restart. |

### 5.5 CI Strategy

Voice tests in CI cannot use real audio hardware. The test strategy:

1. **Compile tests:** `cargo build -p clawft-plugin --features voice` on all CI platforms.
2. **Unit tests:** All unit tests use mock/synthetic data and run without audio hardware.
3. **Integration tests:** Use pre-recorded WAV files and mock `cpal` streams.
4. **Platform tests:** Self-hosted runners with audio hardware (nightly, not PR-blocking).

---

## 6. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `sherpa-rs` build fails on target platform | Medium | High | VP1 prototype validates all platforms before sprint begins. Fallback: `whisper-rs` for STT. |
| `cpal` doesn't work on WSL2 | Medium | Medium | Document PulseAudio bridge setup for WSL2 users. Voice is optional (feature-gated). |
| Model download size (50+ MB) slows first run | Low | Medium | Progressive download with progress bar. `weft voice setup` as explicit trigger. |
| VAD false triggers in noisy environments | Medium | Medium | Configurable threshold via `VoiceConfig.vad.threshold`. Debounce with `min_speech_duration_ms`. |
| TTS latency > target (200ms to first byte) | Medium | Medium | Streaming synthesis splits text into sentences. If sherpa-rs is slow, fall back to shorter chunks. |
| Interruption detection races with TTS output | Low | High | `tokio::select!` ensures VAD check runs concurrently with playback. TTS abort is immediate (watch channel). |
| `ringbuf` API changes between versions | Low | Low | Pin to specific `ringbuf` version in `Cargo.toml`. |
| Memory usage high with concurrent STT + TTS | Low | Medium | STT and TTS share the sherpa-rs runtime. Models are loaded once and reused. |
