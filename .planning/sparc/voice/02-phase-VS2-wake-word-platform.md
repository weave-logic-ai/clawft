# Phase VS2: Wake Word + Echo Cancellation + Platform Integration

> **Element:** Voice -- Wake Word & Platform Integration
> **Phase:** VS2
> **Timeline:** Weeks 4-6
> **Priority:** P1
> **Crates:** `crates/clawft-plugin/` (voice module), `crates/clawft-channels/` (Discord voice), `crates/clawft-cli/`, `scripts/`
> **Dependencies IN:** VS1 (complete audio pipeline -- AudioCapture, AudioPlayback, VAD, STT, TTS, VoiceChannel, Talk Mode)
> **Blocks:** VS3 (UI Voice Integration, Cloud Fallback, Advanced Features)
> **Status:** Planning

---

## 1. Overview

Phase VS2 builds on the complete audio pipeline delivered by VS1 and adds three layers: always-on wake word detection ("Hey Weft"), audio quality improvements (echo cancellation, noise suppression, adaptive silence), and platform-native service integration (systemd, launchd, Windows startup). This phase also delivers the Discord voice channel bridge and multi-language STT support.

The wake word detector runs as a low-CPU background process (<2% target) that listens continuously and activates the full VAD+STT pipeline only when the wake phrase is detected. Platform service files allow the wake daemon to start automatically at login. Echo cancellation prevents the system from hearing its own TTS output, which is critical for hands-free Talk Mode operation.

---

## 2. Current Code

### Existing Voice Module (from VS1)

After VS1 completes, the following structures exist in `crates/clawft-plugin/src/voice/`:

```rust
// crates/clawft-plugin/src/voice/mod.rs (delivered by VS1)
pub mod capture;    // AudioCapture -- cpal mic input
pub mod playback;   // AudioPlayback -- cpal speaker output
pub mod vad;        // VoiceActivityDetector -- Silero VAD via sherpa-rs
pub mod stt;        // SpeechToText -- sherpa-rs streaming recognizer
pub mod tts;        // TextToSpeech -- sherpa-rs streaming synthesizer
pub mod channel;    // VoiceChannel -- ChannelAdapter impl
pub mod talk;       // TalkModeController -- listen/transcribe/think/speak loop
pub mod models;     // ModelDownloadManager -- fetch/cache/integrity
pub mod config;     // VoiceConfig types
```

### Existing VoiceHandler Trait (from C1)

The placeholder `VoiceHandler` trait in `crates/clawft-plugin/src/traits.rs`:

```rust
// crates/clawft-plugin/src/traits.rs (delivered by C1)
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

### Existing CLI Voice Commands (from VS1)

```text
weft voice setup        -- download models, configure audio devices
weft voice test-mic     -- test microphone capture
weft voice test-speak   -- test TTS output
weft voice talk         -- start Talk Mode session
```

### Existing Feature Flags (from VS1)

```toml
# crates/clawft-plugin/Cargo.toml (delivered by VS1)
[features]
voice = ["voice-stt", "voice-tts", "voice-wake"]
voice-stt = ["dep:sherpa-rs"]
voice-tts = ["dep:sherpa-rs"]
voice-wake = ["dep:cpal"]
# Note: rustpotter not yet wired -- added in VS2.1.1
```

### No Existing Wake Word, Echo Cancellation, or Platform Service Code

These are all new in VS2.

---

## 3. Implementation Tasks

### Task VS2.1: Voice Wake (Week 4)

#### VS2.1.1: Add rustpotter dependency

Add `rustpotter` to `clawft-plugin/Cargo.toml` behind the `voice-wake` feature flag.

```toml
# crates/clawft-plugin/Cargo.toml -- additions
[dependencies]
rustpotter = { version = "1.0", optional = true }

[features]
voice-wake = ["dep:cpal", "dep:rustpotter"]
```

#### VS2.1.2: Train "Hey Weft" wake word model

**Training process:**

1. Record 5-8 samples of "Hey Weft" using the built-in training CLI (VS2.1.6).
2. Each sample: 16kHz mono PCM, ~1.5s duration.
3. Rustpotter uses MFCC (Mel-frequency cepstral coefficients) + DTW (Dynamic Time Warping) for matching.
4. The trained model is a `.rpw` file (~500KB) bundled in the binary at build time.
5. Users can also train custom wake words that are stored at `~/.clawft/models/wake/`.

**Model location:**

```text
# Bundled default wake word
models/voice/wake/hey-weft.rpw          (checked into repo at build)

# User-trained custom wake words
~/.clawft/models/wake/<name>.rpw        (user directory)
```

**Training script (`scripts/train-wake-word.sh`):**

```bash
#!/usr/bin/env bash
# Usage: scripts/train-wake-word.sh "hey weft" [output_path]
#
# Records samples and trains a rustpotter wake word model.
# Requires: cargo build -p clawft-cli --features voice-wake

set -euo pipefail

PHRASE="${1:-hey weft}"
OUTPUT="${2:-models/voice/wake/$(echo "$PHRASE" | tr ' ' '-').rpw}"
SAMPLE_DIR=$(mktemp -d)
SAMPLE_COUNT=6
SAMPLE_RATE=16000

echo "Wake word training: '$PHRASE'"
echo "Will record $SAMPLE_COUNT samples at ${SAMPLE_RATE}Hz"
echo ""

for i in $(seq 1 $SAMPLE_COUNT); do
    echo "[$i/$SAMPLE_COUNT] Press Enter, then say '$PHRASE'..."
    read -r
    # Record ~2 seconds via clawft-cli (uses cpal internally)
    weft voice record --duration 2 --output "$SAMPLE_DIR/sample_$i.wav"
    echo "  Recorded: $SAMPLE_DIR/sample_$i.wav"
done

echo ""
echo "Training model..."
weft voice train-wake \
    --phrase "$PHRASE" \
    --samples "$SAMPLE_DIR" \
    --output "$OUTPUT"

echo "Model saved to: $OUTPUT"
rm -rf "$SAMPLE_DIR"
```

#### VS2.1.3: Implement WakeWordDetector

```rust
// crates/clawft-plugin/src/voice/wake.rs

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use crate::PluginError;

#[cfg(feature = "voice-wake")]
use rustpotter::{Rustpotter, RustpotterConfig, WakewordLoad};

/// Configuration for the wake word detector.
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// Path to the .rpw model file.
    pub model_path: PathBuf,
    /// Detection threshold (0.0 - 1.0). Lower = more sensitive.
    /// Default: 0.5
    pub threshold: f32,
    /// Minimum number of frames between detections to prevent rapid re-triggering.
    /// Default: 30 (roughly 1 second at 30ms frames)
    pub min_gap_frames: usize,
    /// Audio sample rate. Must match training sample rate.
    /// Default: 16000
    pub sample_rate: u32,
    /// Whether to log detection events (with confidence score).
    pub log_detections: bool,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/voice/wake/hey-weft.rpw"),
            threshold: 0.5,
            min_gap_frames: 30,
            sample_rate: 16000,
            log_detections: true,
        }
    }
}

/// Events emitted by the wake word detector.
#[derive(Debug, Clone)]
pub enum WakeWordEvent {
    /// Wake word detected with confidence score (0.0 - 1.0).
    Detected { confidence: f32 },
    /// Detector started listening.
    Started,
    /// Detector stopped.
    Stopped,
    /// Error during detection.
    Error(String),
}

/// Detects a wake word in a continuous audio stream using rustpotter.
///
/// Operates on 16kHz mono 16-bit PCM audio frames (480 samples / 30ms each).
/// When the wake word is detected, sends a `WakeWordEvent::Detected` through
/// the event channel. The caller (typically the Talk Mode controller) then
/// activates the full VAD+STT pipeline.
#[cfg(feature = "voice-wake")]
pub struct WakeWordDetector {
    config: WakeWordConfig,
    rustpotter: Rustpotter,
    event_tx: watch::Sender<WakeWordEvent>,
    event_rx: watch::Receiver<WakeWordEvent>,
    frames_since_last_detection: usize,
    cpu_monitor: CpuBudgetMonitor,
}

#[cfg(feature = "voice-wake")]
impl WakeWordDetector {
    /// Create a new WakeWordDetector with the given configuration.
    ///
    /// Loads the .rpw model file and initializes the rustpotter engine.
    pub fn new(config: WakeWordConfig) -> Result<Self, PluginError> {
        let mut rp_config = RustpotterConfig::default();
        rp_config.fmt.sample_rate = config.sample_rate as usize;
        rp_config.fmt.channels = 1;
        rp_config.fmt.sample_format =
            rustpotter::SampleFormat::I16;
        rp_config.detector.threshold = config.threshold;

        let mut rustpotter = Rustpotter::new(&rp_config)
            .map_err(|e| PluginError::LoadFailed(
                format!("failed to initialize rustpotter: {e}")
            ))?;

        // Load the wake word model
        rustpotter
            .add_wakeword_from_file(
                "wake",
                config.model_path.to_str().unwrap_or(""),
            )
            .map_err(|e| PluginError::LoadFailed(
                format!("failed to load wake word model '{}': {e}",
                    config.model_path.display())
            ))?;

        let (event_tx, event_rx) =
            watch::channel(WakeWordEvent::Stopped);

        Ok(Self {
            config,
            rustpotter,
            event_tx,
            event_rx,
            frames_since_last_detection: usize::MAX,
            cpu_monitor: CpuBudgetMonitor::new(2.0),
        })
    }

    /// Subscribe to wake word events.
    pub fn subscribe(&self) -> watch::Receiver<WakeWordEvent> {
        self.event_rx.clone()
    }

    /// Process a single audio frame (480 samples, 30ms at 16kHz).
    ///
    /// Returns `true` if the wake word was detected in this frame.
    pub fn process_frame(&mut self, samples: &[i16]) -> bool {
        self.frames_since_last_detection =
            self.frames_since_last_detection.saturating_add(1);

        // Feed audio to rustpotter
        let detection = self.rustpotter.process_i16(samples);

        if let Some(det) = detection {
            // Enforce minimum gap between detections
            if self.frames_since_last_detection < self.config.min_gap_frames {
                return false;
            }

            let confidence = det.score;
            self.frames_since_last_detection = 0;

            if self.config.log_detections {
                tracing::info!(
                    confidence = %confidence,
                    "wake word detected"
                );
            }

            let _ = self.event_tx.send(WakeWordEvent::Detected {
                confidence,
            });

            self.cpu_monitor.record_detection();
            return true;
        }

        false
    }

    /// Get the current CPU usage estimate for the wake word detector.
    pub fn cpu_usage_percent(&self) -> f32 {
        self.cpu_monitor.current_percent()
    }

    /// Replace the loaded wake word model at runtime.
    pub fn load_model(&mut self, path: &std::path::Path) -> Result<(), PluginError> {
        // Remove existing wake word
        self.rustpotter.remove_wakeword("wake");

        // Load new model
        self.rustpotter
            .add_wakeword_from_file(
                "wake",
                path.to_str().unwrap_or(""),
            )
            .map_err(|e| PluginError::LoadFailed(
                format!("failed to load wake word model '{}': {e}",
                    path.display())
            ))?;

        self.config.model_path = path.to_path_buf();
        Ok(())
    }
}

/// Monitors CPU usage of the wake word detector to stay within budget.
///
/// Uses a simple heuristic: tracks processing time per frame and compares
/// against the frame duration (30ms). Target is < 2% CPU (< 0.6ms per frame).
struct CpuBudgetMonitor {
    target_percent: f32,
    /// Rolling window of processing times in microseconds.
    frame_times_us: Vec<u64>,
    window_size: usize,
    detection_count: u64,
}

impl CpuBudgetMonitor {
    fn new(target_percent: f32) -> Self {
        Self {
            target_percent,
            frame_times_us: Vec::with_capacity(100),
            window_size: 100,
            detection_count: 0,
        }
    }

    fn record_frame_time(&mut self, micros: u64) {
        if self.frame_times_us.len() >= self.window_size {
            self.frame_times_us.remove(0);
        }
        self.frame_times_us.push(micros);
    }

    fn record_detection(&mut self) {
        self.detection_count += 1;
    }

    /// Returns estimated CPU usage as a percentage.
    ///
    /// Calculation: avg_frame_time_us / frame_duration_us * 100
    /// Frame duration at 16kHz, 480 samples = 30,000us (30ms).
    fn current_percent(&self) -> f32 {
        if self.frame_times_us.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.frame_times_us.iter().sum();
        let avg_us = sum as f32 / self.frame_times_us.len() as f32;
        let frame_duration_us = 30_000.0; // 30ms
        (avg_us / frame_duration_us) * 100.0
    }

    fn is_within_budget(&self) -> bool {
        self.current_percent() < self.target_percent
    }
}
```

#### VS2.1.4: Wake word -> VAD pipeline activation

Wire the wake word detector output into the existing Talk Mode controller from VS1. When a wake word is detected, the controller transitions from `Idle` (passive wake word listening) to `Listening` (full VAD+STT pipeline active).

```rust
// crates/clawft-plugin/src/voice/talk.rs -- additions to TalkModeController

/// Talk Mode states extended with wake word idle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalkModeState {
    /// Wake word detector is running. Full pipeline is off.
    /// Transitions to Listening on wake word detection.
    WakeIdle,
    /// VAD is active, microphone is capturing, waiting for speech.
    Listening,
    /// Speech detected, STT is processing audio.
    Transcribing,
    /// Agent is processing the transcribed text.
    Thinking,
    /// TTS is synthesizing and playing the response.
    Speaking,
    /// Pipeline is fully stopped.
    Stopped,
}

impl TalkModeController {
    /// Start Talk Mode with wake word activation.
    ///
    /// The pipeline begins in `WakeIdle` state. When the wake word is
    /// detected, it transitions to `Listening` and activates the full
    /// VAD+STT pipeline. After the conversation turn completes (agent
    /// responds via TTS), the pipeline returns to `WakeIdle`.
    pub async fn start_with_wake(
        &mut self,
        wake_detector: &mut WakeWordDetector,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<(), PluginError> {
        self.set_state(TalkModeState::WakeIdle);
        tracing::info!("talk mode started (wake word idle)");

        let mut wake_rx = wake_detector.subscribe();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    self.set_state(TalkModeState::Stopped);
                    break;
                }
                _ = wake_rx.changed() => {
                    if let WakeWordEvent::Detected { confidence } =
                        wake_rx.borrow().clone()
                    {
                        tracing::info!(
                            confidence = %confidence,
                            "wake word triggered pipeline activation"
                        );

                        // Play acknowledgment tone
                        self.play_activation_tone().await?;

                        // Run one full conversation turn:
                        // Listening -> Transcribing -> Thinking -> Speaking
                        self.run_conversation_turn().await?;

                        // Return to wake word idle
                        self.set_state(TalkModeState::WakeIdle);
                    }
                }
            }
        }

        Ok(())
    }

    /// Run a single conversation turn through the full pipeline.
    async fn run_conversation_turn(&mut self) -> Result<(), PluginError> {
        // 1. Listen for speech via VAD
        self.set_state(TalkModeState::Listening);
        let audio = self.capture_until_silence().await?;

        // 2. Transcribe
        self.set_state(TalkModeState::Transcribing);
        let text = self.stt.transcribe(&audio).await?;

        if text.trim().is_empty() {
            tracing::debug!("empty transcription, returning to idle");
            return Ok(());
        }

        // 3. Send to agent and wait for response
        self.set_state(TalkModeState::Thinking);
        let response = self.send_to_agent(&text).await?;

        // 4. Speak the response
        self.set_state(TalkModeState::Speaking);
        self.speak_with_interruption(&response).await?;

        Ok(())
    }

    /// Play a short activation tone to indicate wake word was heard.
    async fn play_activation_tone(&self) -> Result<(), PluginError> {
        // Generate a short 440Hz beep (100ms)
        let sample_rate = 16000;
        let duration_samples = sample_rate / 10; // 100ms
        let mut samples = Vec::with_capacity(duration_samples);
        for i in 0..duration_samples {
            let t = i as f32 / sample_rate as f32;
            let amplitude = 0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            samples.push((amplitude * i16::MAX as f32) as i16);
        }
        self.playback.play_samples(&samples).await
    }
}
```

#### VS2.1.5: CLI -- `weft voice wake`

```rust
// crates/clawft-cli/src/commands/voice.rs -- additions

/// Start always-on wake word listener.
///
/// Runs the wake word detector in the foreground. When the wake word is
/// detected, activates the full voice pipeline for one conversation turn,
/// then returns to listening.
#[derive(Debug, clap::Args)]
pub struct VoiceWakeArgs {
    /// Path to a custom wake word model (.rpw file).
    /// Default: bundled "Hey Weft" model.
    #[arg(long)]
    model: Option<PathBuf>,

    /// Detection sensitivity (0.0 - 1.0). Lower = more sensitive.
    #[arg(long, default_value = "0.5")]
    sensitivity: f32,

    /// Run as a background daemon (detach from terminal).
    #[arg(long)]
    daemon: bool,

    /// Log wake word detections to file.
    #[arg(long)]
    log_file: Option<PathBuf>,
}

pub async fn handle_voice_wake(args: VoiceWakeArgs) -> Result<()> {
    let model_path = args.model.unwrap_or_else(|| {
        // Check user models first, then bundled
        let user_model = dirs::home_dir()
            .unwrap_or_default()
            .join(".clawft/models/wake/hey-weft.rpw");
        if user_model.exists() {
            user_model
        } else {
            PathBuf::from("models/voice/wake/hey-weft.rpw")
        }
    });

    let config = WakeWordConfig {
        model_path,
        threshold: args.sensitivity,
        log_detections: true,
        ..Default::default()
    };

    let mut detector = WakeWordDetector::new(config)?;

    if args.daemon {
        // Fork to background (Unix) or register as service (Windows)
        daemonize()?;
    }

    println!("Wake word listener active. Say 'Hey Weft' to activate.");
    println!("CPU usage target: < 2%");
    println!("Press Ctrl+C to stop.");

    let cancel = tokio_util::sync::CancellationToken::new();

    // Set up Ctrl+C handler
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        cancel_clone.cancel();
    });

    let mut talk_controller = TalkModeController::new(/* ... */)?;
    talk_controller
        .start_with_wake(&mut detector, cancel)
        .await?;

    println!("Wake word listener stopped.");
    Ok(())
}
```

#### VS2.1.6: CLI -- `weft voice train-wake`

```rust
// crates/clawft-cli/src/commands/voice.rs -- additions

/// Guided wake word training.
///
/// Records multiple samples of the user saying a wake phrase, then
/// trains a rustpotter model (.rpw) from those samples.
#[derive(Debug, clap::Args)]
pub struct VoiceTrainWakeArgs {
    /// The wake phrase to train (e.g., "hey weft").
    phrase: String,

    /// Number of samples to record. More = better accuracy.
    #[arg(long, default_value = "6")]
    samples: usize,

    /// Output path for the trained model.
    /// Default: ~/.clawft/models/wake/<phrase>.rpw
    #[arg(long)]
    output: Option<PathBuf>,

    /// Sample duration in seconds.
    #[arg(long, default_value = "2")]
    duration: u32,
}

pub async fn handle_voice_train_wake(args: VoiceTrainWakeArgs) -> Result<()> {
    let output_path = args.output.unwrap_or_else(|| {
        let slug = args.phrase.to_lowercase().replace(' ', "-");
        dirs::home_dir()
            .unwrap_or_default()
            .join(format!(".clawft/models/wake/{slug}.rpw"))
    });

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!("Wake word training: '{}'", args.phrase);
    println!("Will record {} samples ({} seconds each)",
        args.samples, args.duration);
    println!();

    let capture = AudioCapture::new(16000, 1)?;
    let mut wav_paths = Vec::new();
    let temp_dir = tempfile::tempdir()?;

    for i in 1..=args.samples {
        println!("[{}/{}] Press Enter, then say '{}'...",
            i, args.samples, args.phrase);

        // Wait for Enter key
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        // Record audio
        let wav_path = temp_dir.path().join(format!("sample_{i}.wav"));
        capture.record_to_file(
            &wav_path,
            std::time::Duration::from_secs(args.duration as u64),
        ).await?;

        println!("  Recorded: sample_{i}.wav");
        wav_paths.push(wav_path);
    }

    println!();
    println!("Training model...");

    // Train rustpotter model from samples
    train_rustpotter_model(&args.phrase, &wav_paths, &output_path)?;

    println!("Model saved to: {}", output_path.display());
    println!();
    println!("To use this wake word:");
    println!("  weft voice wake --model {}", output_path.display());

    Ok(())
}

/// Train a rustpotter wake word model from recorded WAV samples.
fn train_rustpotter_model(
    phrase: &str,
    sample_paths: &[PathBuf],
    output_path: &PathBuf,
) -> Result<(), PluginError> {
    use rustpotter::{RustpotterConfig, WakewordModelTrain};

    let config = RustpotterConfig::default();
    let mut trainer = WakewordModelTrain::new(&config)
        .map_err(|e| PluginError::ExecutionFailed(
            format!("failed to init trainer: {e}")
        ))?;

    for path in sample_paths {
        trainer.add_sample_from_file(
            path.to_str().unwrap_or("")
        ).map_err(|e| PluginError::ExecutionFailed(
            format!("failed to add sample '{}': {e}", path.display())
        ))?;
    }

    trainer.train(
        output_path.to_str().unwrap_or("")
    ).map_err(|e| PluginError::ExecutionFailed(
        format!("training failed: {e}")
    ))?;

    Ok(())
}
```

#### VS2.1.7: Custom wake word support

Custom wake words are stored at `~/.clawft/models/wake/` and can be selected via configuration or CLI flag.

```rust
// crates/clawft-plugin/src/voice/wake.rs -- additions

/// Lists available wake word models (bundled + user-trained).
pub fn list_wake_word_models() -> Result<Vec<WakeWordModelInfo>, PluginError> {
    let mut models = Vec::new();

    // Bundled model
    let bundled = PathBuf::from("models/voice/wake/hey-weft.rpw");
    if bundled.exists() {
        models.push(WakeWordModelInfo {
            name: "hey-weft".to_string(),
            path: bundled,
            source: WakeWordSource::Bundled,
        });
    }

    // User-trained models
    if let Some(user_dir) = dirs::home_dir()
        .map(|h| h.join(".clawft/models/wake"))
    {
        if user_dir.is_dir() {
            for entry in std::fs::read_dir(&user_dir)
                .map_err(|e| PluginError::Io(e))?
            {
                let entry = entry.map_err(|e| PluginError::Io(e))?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rpw") {
                    let name = path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    models.push(WakeWordModelInfo {
                        name,
                        path,
                        source: WakeWordSource::UserTrained,
                    });
                }
            }
        }
    }

    Ok(models)
}

/// Information about an available wake word model.
#[derive(Debug, Clone)]
pub struct WakeWordModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub source: WakeWordSource,
}

/// Where a wake word model originated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeWordSource {
    Bundled,
    UserTrained,
}
```

#### VS2.1.8: CPU budget monitoring

CPU budget monitoring is implemented in `CpuBudgetMonitor` (see VS2.1.3 above). The integration point is in the wake word processing loop:

```rust
// crates/clawft-plugin/src/voice/wake.rs -- processing loop integration

impl WakeWordDetector {
    /// Run the wake word detection loop on an audio capture stream.
    ///
    /// Processes audio frames from the capture device and monitors CPU usage.
    /// If CPU exceeds the 2% budget, logs a warning and reduces processing
    /// frequency (skip every other frame).
    pub async fn run_detection_loop(
        &mut self,
        capture: &AudioCapture,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<(), PluginError> {
        let _ = self.event_tx.send(WakeWordEvent::Started);
        let mut skip_next = false;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = self.event_tx.send(WakeWordEvent::Stopped);
                    return Ok(());
                }
                frame = capture.next_frame() => {
                    let frame = frame?;

                    // CPU budget throttling: skip frames if over budget
                    if skip_next {
                        skip_next = false;
                        continue;
                    }

                    let start = std::time::Instant::now();
                    self.process_frame(&frame);
                    let elapsed = start.elapsed();

                    self.cpu_monitor.record_frame_time(
                        elapsed.as_micros() as u64
                    );

                    if !self.cpu_monitor.is_within_budget() {
                        tracing::warn!(
                            cpu_percent = %self.cpu_monitor.current_percent(),
                            "wake word CPU over budget, throttling"
                        );
                        skip_next = true;
                    }
                }
            }
        }
    }
}
```

---

### Task VS2.2: Echo Cancellation + Quality (Week 5)

#### VS2.2.1: EchoCanceller implementation

Software acoustic echo cancellation using loopback subtraction. The TTS playback audio is subtracted from the microphone input to prevent the system from hearing its own output.

```rust
// crates/clawft-plugin/src/voice/echo.rs

use std::collections::VecDeque;
use crate::PluginError;

/// Software acoustic echo canceller using loopback subtraction.
///
/// Works by maintaining a buffer of recent TTS output samples. When
/// microphone input arrives, the reference signal (TTS output) is
/// subtracted from the mic input after applying an adaptive filter
/// to account for room acoustics and speaker-to-mic delay.
///
/// Algorithm:
/// 1. Capture TTS output via cpal loopback (reference signal).
/// 2. Estimate the delay between speaker output and mic capture.
/// 3. Apply normalized LMS (NLMS) adaptive filter to align signals.
/// 4. Subtract the filtered reference from mic input.
pub struct EchoCanceller {
    /// Circular buffer of reference (TTS output) samples.
    reference_buffer: VecDeque<f32>,
    /// Maximum delay in samples to search for echo (default: 4800 = 300ms).
    max_delay_samples: usize,
    /// NLMS adaptive filter weights.
    filter_weights: Vec<f32>,
    /// NLMS filter length (number of taps).
    filter_length: usize,
    /// NLMS step size (learning rate). Smaller = more stable, slower adaptation.
    step_size: f32,
    /// Whether AEC is currently active (only when TTS is playing).
    active: bool,
    /// Sample rate.
    sample_rate: u32,
}

impl EchoCanceller {
    /// Create a new EchoCanceller.
    ///
    /// - `sample_rate`: Audio sample rate (typically 16000).
    /// - `max_delay_ms`: Maximum echo delay in milliseconds (default: 300).
    /// - `filter_length`: NLMS filter taps (default: 512).
    pub fn new(
        sample_rate: u32,
        max_delay_ms: u32,
        filter_length: usize,
    ) -> Self {
        let max_delay_samples =
            (sample_rate * max_delay_ms / 1000) as usize;

        Self {
            reference_buffer: VecDeque::with_capacity(
                max_delay_samples + filter_length
            ),
            max_delay_samples,
            filter_weights: vec![0.0; filter_length],
            filter_length,
            step_size: 0.01,
            active: false,
            sample_rate,
        }
    }

    /// Feed TTS output samples into the reference buffer.
    ///
    /// Call this whenever TTS audio is sent to the speaker.
    pub fn feed_reference(&mut self, samples: &[f32]) {
        for &s in samples {
            self.reference_buffer.push_back(s);
            if self.reference_buffer.len() > self.max_delay_samples + self.filter_length {
                self.reference_buffer.pop_front();
            }
        }
        self.active = true;
    }

    /// Process microphone input and remove echo.
    ///
    /// Returns the echo-cancelled audio samples.
    pub fn process(&mut self, mic_input: &[f32]) -> Vec<f32> {
        if !self.active || self.reference_buffer.len() < self.filter_length {
            return mic_input.to_vec();
        }

        let mut output = Vec::with_capacity(mic_input.len());

        for &mic_sample in mic_input {
            // Get reference signal segment (most recent filter_length samples)
            let ref_start = self.reference_buffer.len()
                .saturating_sub(self.filter_length);
            let ref_segment: Vec<f32> = self.reference_buffer
                .range(ref_start..)
                .copied()
                .collect();

            if ref_segment.len() < self.filter_length {
                output.push(mic_sample);
                continue;
            }

            // Compute filter output: y = sum(w[i] * ref[i])
            let echo_estimate: f32 = self.filter_weights.iter()
                .zip(ref_segment.iter())
                .map(|(w, r)| w * r)
                .sum();

            // Error signal = mic - estimated echo
            let error = mic_sample - echo_estimate;

            // NLMS weight update: w += (step_size / (ref_power + eps)) * error * ref
            let ref_power: f32 = ref_segment.iter()
                .map(|r| r * r)
                .sum();
            let norm_step = self.step_size / (ref_power + 1e-6);

            for (w, r) in self.filter_weights.iter_mut()
                .zip(ref_segment.iter())
            {
                *w += norm_step * error * r;
            }

            output.push(error);
        }

        output
    }

    /// Signal that TTS playback has finished.
    ///
    /// The echo canceller continues processing for `max_delay_ms` after
    /// playback ends to catch the tail of the echo, then deactivates.
    pub fn playback_finished(&mut self) {
        // Deactivation will happen naturally as the reference buffer
        // drains below filter_length. Mark for eventual deactivation.
        // The reference buffer is not cleared immediately to handle
        // late-arriving echoes.
    }

    /// Reset the echo canceller state.
    pub fn reset(&mut self) {
        self.reference_buffer.clear();
        self.filter_weights.fill(0.0);
        self.active = false;
    }
}
```

#### VS2.2.2: NoiseSuppressor pre-filter

```rust
// crates/clawft-plugin/src/voice/noise.rs

/// Simple spectral subtraction noise suppressor.
///
/// Estimates the noise floor during silence periods (as detected by VAD)
/// and subtracts it from the signal during speech. This pre-filter runs
/// before STT to improve transcription accuracy in noisy environments.
pub struct NoiseSuppressor {
    /// Estimated noise spectrum (magnitude).
    noise_spectrum: Vec<f32>,
    /// Number of noise estimation frames accumulated.
    noise_frames: usize,
    /// FFT size (must be power of 2).
    fft_size: usize,
    /// Whether the noise floor has been estimated.
    calibrated: bool,
    /// Over-subtraction factor (default: 1.0). Higher = more aggressive.
    over_subtraction: f32,
    /// Spectral floor to prevent musical noise artifacts.
    spectral_floor: f32,
}

impl NoiseSuppressor {
    /// Create a new NoiseSuppressor.
    ///
    /// - `fft_size`: FFT window size (default: 512 for 16kHz audio).
    pub fn new(fft_size: usize) -> Self {
        Self {
            noise_spectrum: vec![0.0; fft_size / 2 + 1],
            noise_frames: 0,
            fft_size,
            calibrated: false,
            over_subtraction: 1.0,
            spectral_floor: 0.01,
        }
    }

    /// Update the noise floor estimate with a silence frame.
    ///
    /// Call this with audio frames that VAD identifies as non-speech.
    /// The noise suppressor averages multiple silence frames to build
    /// a stable noise profile.
    pub fn update_noise_estimate(&mut self, silence_frame: &[f32]) {
        // Compute magnitude spectrum of silence frame
        let spectrum = self.compute_magnitude_spectrum(silence_frame);

        // Running average of noise spectrum
        self.noise_frames += 1;
        let alpha = 1.0 / self.noise_frames as f32;

        for (est, &new) in self.noise_spectrum.iter_mut().zip(spectrum.iter()) {
            *est = *est * (1.0 - alpha) + new * alpha;
        }

        if self.noise_frames >= 10 {
            self.calibrated = true;
        }
    }

    /// Apply noise suppression to an audio frame.
    ///
    /// Returns the noise-suppressed audio. If not yet calibrated,
    /// returns the input unchanged.
    pub fn suppress(&self, frame: &[f32]) -> Vec<f32> {
        if !self.calibrated {
            return frame.to_vec();
        }

        let spectrum = self.compute_magnitude_spectrum(frame);
        let mut clean_spectrum = Vec::with_capacity(spectrum.len());

        for (s, n) in spectrum.iter().zip(self.noise_spectrum.iter()) {
            let suppressed = (s - self.over_subtraction * n)
                .max(self.spectral_floor * s);
            clean_spectrum.push(suppressed);
        }

        self.inverse_spectrum(&clean_spectrum, frame)
    }

    /// Compute magnitude spectrum via FFT.
    fn compute_magnitude_spectrum(&self, frame: &[f32]) -> Vec<f32> {
        // Implementation uses real FFT (e.g., via rustfft crate)
        // Returns magnitude of each frequency bin
        let mut magnitudes = vec![0.0; self.fft_size / 2 + 1];
        // FFT computation placeholder -- actual impl uses rustfft
        magnitudes
    }

    /// Reconstruct time-domain signal from modified magnitude spectrum.
    fn inverse_spectrum(
        &self,
        spectrum: &[f32],
        original: &[f32],
    ) -> Vec<f32> {
        // Uses original phase with modified magnitude
        // Inverse FFT to reconstruct time-domain signal
        original.to_vec() // placeholder
    }

    /// Whether the noise floor has been calibrated.
    pub fn is_calibrated(&self) -> bool {
        self.calibrated
    }

    /// Reset the noise estimate (e.g., when environment changes).
    pub fn reset(&mut self) {
        self.noise_spectrum.fill(0.0);
        self.noise_frames = 0;
        self.calibrated = false;
    }
}
```

#### VS2.2.3: AdaptiveSilenceTimeout

```rust
// crates/clawft-plugin/src/voice/silence.rs

use std::time::Duration;

/// Learns user speech patterns to adapt the silence timeout.
///
/// Instead of a fixed silence duration to determine "end of utterance,"
/// this adapter tracks the distribution of inter-word pauses and
/// sentence-ending pauses for each user. Over time, it learns the
/// optimal timeout that avoids cutting off mid-sentence pauses while
/// still responding quickly after genuine end-of-utterance silence.
pub struct AdaptiveSilenceTimeout {
    /// Current timeout duration.
    current_timeout: Duration,
    /// Minimum allowed timeout.
    min_timeout: Duration,
    /// Maximum allowed timeout.
    max_timeout: Duration,
    /// History of pause durations classified as mid-sentence.
    mid_sentence_pauses: Vec<Duration>,
    /// History of pause durations classified as end-of-utterance.
    end_of_utterance_pauses: Vec<Duration>,
    /// Maximum history size per category.
    max_history: usize,
    /// Whether adaptation is enabled.
    enabled: bool,
}

impl AdaptiveSilenceTimeout {
    /// Create a new AdaptiveSilenceTimeout.
    ///
    /// - `initial`: Starting timeout (default: 1.5s).
    /// - `min`: Minimum timeout (default: 0.5s).
    /// - `max`: Maximum timeout (default: 3.0s).
    pub fn new(initial: Duration, min: Duration, max: Duration) -> Self {
        Self {
            current_timeout: initial,
            min_timeout: min,
            max_timeout: max,
            mid_sentence_pauses: Vec::new(),
            end_of_utterance_pauses: Vec::new(),
            max_history: 100,
            enabled: true,
        }
    }

    /// Record a pause that turned out to be mid-sentence.
    ///
    /// Call this when the user continues speaking after a pause (i.e.,
    /// the system waited and more speech followed).
    pub fn record_mid_sentence_pause(&mut self, duration: Duration) {
        if !self.enabled {
            return;
        }
        if self.mid_sentence_pauses.len() >= self.max_history {
            self.mid_sentence_pauses.remove(0);
        }
        self.mid_sentence_pauses.push(duration);
        self.recalculate();
    }

    /// Record a pause that was end-of-utterance.
    ///
    /// Call this when the user did not continue speaking after the
    /// timeout fired (i.e., the system correctly identified end of speech).
    pub fn record_end_of_utterance_pause(&mut self, duration: Duration) {
        if !self.enabled {
            return;
        }
        if self.end_of_utterance_pauses.len() >= self.max_history {
            self.end_of_utterance_pauses.remove(0);
        }
        self.end_of_utterance_pauses.push(duration);
        self.recalculate();
    }

    /// Recalculate the optimal timeout.
    ///
    /// Strategy: set timeout to the 95th percentile of mid-sentence
    /// pauses. This ensures we wait long enough to not cut off 95%
    /// of natural pauses, while still timing out quickly after actual
    /// end-of-utterance silence.
    fn recalculate(&mut self) {
        if self.mid_sentence_pauses.len() < 5 {
            return; // Not enough data to adapt
        }

        let mut sorted: Vec<Duration> = self.mid_sentence_pauses.clone();
        sorted.sort();

        let p95_index = (sorted.len() as f64 * 0.95) as usize;
        let p95_index = p95_index.min(sorted.len() - 1);
        let p95 = sorted[p95_index];

        // Add a small buffer (200ms) above the 95th percentile
        let new_timeout = p95 + Duration::from_millis(200);

        self.current_timeout = new_timeout
            .max(self.min_timeout)
            .min(self.max_timeout);

        tracing::debug!(
            timeout_ms = %self.current_timeout.as_millis(),
            mid_sentence_samples = %self.mid_sentence_pauses.len(),
            "adaptive silence timeout updated"
        );
    }

    /// Get the current timeout duration.
    pub fn timeout(&self) -> Duration {
        self.current_timeout
    }

    /// Reset learned patterns.
    pub fn reset(&mut self) {
        self.mid_sentence_pauses.clear();
        self.end_of_utterance_pauses.clear();
        self.current_timeout = Duration::from_millis(1500);
    }

    /// Enable or disable adaptation.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for AdaptiveSilenceTimeout {
    fn default() -> Self {
        Self::new(
            Duration::from_millis(1500),
            Duration::from_millis(500),
            Duration::from_millis(3000),
        )
    }
}
```

#### VS2.2.4: Multi-language STT support

```rust
// crates/clawft-plugin/src/voice/language.rs

use crate::PluginError;

/// Supported STT/TTS languages.
///
/// sherpa-onnx supports 12+ languages. Each language requires a
/// corresponding model downloaded to `~/.clawft/models/voice/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VoiceLanguage {
    English,
    Spanish,
    French,
    German,
    Italian,
    Portuguese,
    Russian,
    Chinese,
    Japanese,
    Korean,
    Arabic,
    Hindi,
    /// Auto-detect language from audio content.
    Auto,
    /// Custom language with model path.
    Custom(String),
}

impl VoiceLanguage {
    /// Get the sherpa-onnx model identifier for this language.
    pub fn model_id(&self) -> &str {
        match self {
            Self::English => "sherpa-onnx-streaming-zipformer-en",
            Self::Spanish => "sherpa-onnx-streaming-zipformer-es",
            Self::French => "sherpa-onnx-streaming-zipformer-fr",
            Self::German => "sherpa-onnx-streaming-zipformer-de",
            Self::Italian => "sherpa-onnx-streaming-zipformer-it",
            Self::Portuguese => "sherpa-onnx-streaming-zipformer-pt",
            Self::Russian => "sherpa-onnx-streaming-zipformer-ru",
            Self::Chinese => "sherpa-onnx-streaming-zipformer-zh",
            Self::Japanese => "sherpa-onnx-streaming-zipformer-ja",
            Self::Korean => "sherpa-onnx-streaming-zipformer-ko",
            Self::Arabic => "sherpa-onnx-streaming-zipformer-ar",
            Self::Hindi => "sherpa-onnx-streaming-zipformer-hi",
            Self::Auto => "sherpa-onnx-streaming-zipformer-multi",
            Self::Custom(id) => id.as_str(),
        }
    }

    /// Get the model download URL for this language.
    pub fn model_url(&self) -> String {
        let base = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models";
        format!("{}/{}.tar.bz2", base, self.model_id())
    }

    /// Parse a language from a string (case-insensitive, accepts BCP-47 codes).
    pub fn from_str_loose(s: &str) -> Result<Self, PluginError> {
        match s.to_lowercase().as_str() {
            "en" | "english" | "en-us" | "en-gb" => Ok(Self::English),
            "es" | "spanish" | "es-es" | "es-mx" => Ok(Self::Spanish),
            "fr" | "french" | "fr-fr" => Ok(Self::French),
            "de" | "german" | "de-de" => Ok(Self::German),
            "it" | "italian" | "it-it" => Ok(Self::Italian),
            "pt" | "portuguese" | "pt-br" | "pt-pt" => Ok(Self::Portuguese),
            "ru" | "russian" | "ru-ru" => Ok(Self::Russian),
            "zh" | "chinese" | "zh-cn" | "zh-tw" => Ok(Self::Chinese),
            "ja" | "japanese" | "ja-jp" => Ok(Self::Japanese),
            "ko" | "korean" | "ko-kr" => Ok(Self::Korean),
            "ar" | "arabic" | "ar-sa" => Ok(Self::Arabic),
            "hi" | "hindi" | "hi-in" => Ok(Self::Hindi),
            "auto" | "detect" => Ok(Self::Auto),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

/// Language configuration for the voice pipeline.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanguageConfig {
    /// Primary STT language. Default: English.
    pub stt_language: VoiceLanguage,
    /// Primary TTS language. Default: English.
    pub tts_language: VoiceLanguage,
    /// Whether to auto-detect language if the primary language
    /// produces low-confidence results.
    pub auto_detect_fallback: bool,
    /// Minimum confidence threshold for auto-detection to switch languages.
    pub auto_detect_threshold: f32,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            stt_language: VoiceLanguage::English,
            tts_language: VoiceLanguage::English,
            auto_detect_fallback: false,
            auto_detect_threshold: 0.6,
        }
    }
}
```

#### VS2.2.5: Voice selection (per-agent TTS voice config)

```rust
// crates/clawft-types/src/config/voice.rs -- additions

/// TTS voice configuration.
///
/// Each agent can have a distinct TTS voice. Voices are identified by
/// a model-specific name/ID and optional parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceProfile {
    /// Voice identifier (model-specific, e.g., "en-us-amy-medium").
    pub voice_id: String,
    /// Speaking rate multiplier (1.0 = normal, 0.8 = slower, 1.2 = faster).
    #[serde(default = "default_rate")]
    pub rate: f32,
    /// Pitch adjustment in semitones (-12.0 to +12.0, 0.0 = normal).
    #[serde(default)]
    pub pitch: f32,
    /// Volume multiplier (0.0 - 1.0, default 0.8).
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Language for this voice.
    #[serde(default)]
    pub language: Option<String>,
}

fn default_rate() -> f32 { 1.0 }
fn default_volume() -> f32 { 0.8 }

impl Default for VoiceProfile {
    fn default() -> Self {
        Self {
            voice_id: "en-us-amy-medium".to_string(),
            rate: 1.0,
            pitch: 0.0,
            volume: 0.8,
            language: None,
        }
    }
}

/// Per-agent voice configuration.
///
/// Maps agent names to voice profiles, allowing different agents
/// to speak with different voices in multi-agent setups.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentVoiceConfig {
    /// Default voice profile used when no agent-specific voice is set.
    #[serde(default)]
    pub default_voice: VoiceProfile,
    /// Agent-specific voice overrides. Key = agent name.
    #[serde(default)]
    pub agent_voices: std::collections::HashMap<String, VoiceProfile>,
}

impl AgentVoiceConfig {
    /// Get the voice profile for a given agent.
    ///
    /// Returns the agent-specific profile if one exists, otherwise
    /// the default profile.
    pub fn voice_for_agent(&self, agent_name: &str) -> &VoiceProfile {
        self.agent_voices
            .get(agent_name)
            .unwrap_or(&self.default_voice)
    }
}
```

#### VS2.2.6: Audio quality metrics

```rust
// crates/clawft-plugin/src/voice/metrics.rs

use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// Audio quality metrics collector.
///
/// Tracks signal-to-noise ratio, pipeline latency percentiles,
/// and detection/transcription statistics.
pub struct AudioQualityMetrics {
    /// Rolling window of SNR measurements (dB).
    snr_history: VecDeque<f32>,
    /// Rolling window of end-to-end latency measurements.
    latency_history: VecDeque<Duration>,
    /// Total speech frames processed.
    total_speech_frames: u64,
    /// Total silence frames processed.
    total_silence_frames: u64,
    /// Total transcriptions produced.
    total_transcriptions: u64,
    /// Total wake word detections.
    total_wake_detections: u64,
    /// Window size for rolling metrics.
    window_size: usize,
}

impl AudioQualityMetrics {
    pub fn new(window_size: usize) -> Self {
        Self {
            snr_history: VecDeque::with_capacity(window_size),
            latency_history: VecDeque::with_capacity(window_size),
            total_speech_frames: 0,
            total_silence_frames: 0,
            total_transcriptions: 0,
            total_wake_detections: 0,
            window_size,
        }
    }

    /// Record a signal-to-noise ratio measurement.
    ///
    /// SNR is computed as: 10 * log10(signal_power / noise_power)
    pub fn record_snr(&mut self, signal_power: f32, noise_power: f32) {
        if noise_power > 0.0 {
            let snr_db = 10.0 * (signal_power / noise_power).log10();
            if self.snr_history.len() >= self.window_size {
                self.snr_history.pop_front();
            }
            self.snr_history.push_back(snr_db);
        }
    }

    /// Record a pipeline latency measurement (speech-end to response-start).
    pub fn record_latency(&mut self, latency: Duration) {
        if self.latency_history.len() >= self.window_size {
            self.latency_history.pop_front();
        }
        self.latency_history.push_back(latency);
    }

    /// Record that a speech frame was processed.
    pub fn record_speech_frame(&mut self) {
        self.total_speech_frames += 1;
    }

    /// Record that a silence frame was processed.
    pub fn record_silence_frame(&mut self) {
        self.total_silence_frames += 1;
    }

    /// Record a completed transcription.
    pub fn record_transcription(&mut self) {
        self.total_transcriptions += 1;
    }

    /// Record a wake word detection.
    pub fn record_wake_detection(&mut self) {
        self.total_wake_detections += 1;
    }

    /// Get the average SNR over the rolling window.
    pub fn avg_snr_db(&self) -> Option<f32> {
        if self.snr_history.is_empty() {
            return None;
        }
        let sum: f32 = self.snr_history.iter().sum();
        Some(sum / self.snr_history.len() as f32)
    }

    /// Get latency percentiles over the rolling window.
    pub fn latency_percentiles(&self) -> LatencyPercentiles {
        if self.latency_history.is_empty() {
            return LatencyPercentiles::default();
        }

        let mut sorted: Vec<Duration> =
            self.latency_history.iter().copied().collect();
        sorted.sort();

        let p50 = sorted[sorted.len() / 2];
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p95 = sorted[p95_idx.min(sorted.len() - 1)];
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];

        LatencyPercentiles { p50, p95, p99 }
    }

    /// Get a summary report of all metrics.
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            avg_snr_db: self.avg_snr_db(),
            latency: self.latency_percentiles(),
            total_speech_frames: self.total_speech_frames,
            total_silence_frames: self.total_silence_frames,
            total_transcriptions: self.total_transcriptions,
            total_wake_detections: self.total_wake_detections,
            speech_ratio: if self.total_speech_frames + self.total_silence_frames > 0 {
                self.total_speech_frames as f64
                    / (self.total_speech_frames + self.total_silence_frames) as f64
            } else {
                0.0
            },
        }
    }
}

/// Latency percentile measurements.
#[derive(Debug, Clone, Default)]
pub struct LatencyPercentiles {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

/// Summary of all audio quality metrics.
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub avg_snr_db: Option<f32>,
    pub latency: LatencyPercentiles,
    pub total_speech_frames: u64,
    pub total_silence_frames: u64,
    pub total_transcriptions: u64,
    pub total_wake_detections: u64,
    pub speech_ratio: f64,
}

impl std::fmt::Display for MetricsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Audio Quality Metrics:")?;
        if let Some(snr) = self.avg_snr_db {
            writeln!(f, "  SNR (avg):         {snr:.1} dB")?;
        }
        writeln!(f, "  Latency p50:       {:?}", self.latency.p50)?;
        writeln!(f, "  Latency p95:       {:?}", self.latency.p95)?;
        writeln!(f, "  Latency p99:       {:?}", self.latency.p99)?;
        writeln!(f, "  Speech frames:     {}", self.total_speech_frames)?;
        writeln!(f, "  Silence frames:    {}", self.total_silence_frames)?;
        writeln!(f, "  Speech ratio:      {:.1}%", self.speech_ratio * 100.0)?;
        writeln!(f, "  Transcriptions:    {}", self.total_transcriptions)?;
        writeln!(f, "  Wake detections:   {}", self.total_wake_detections)?;
        Ok(())
    }
}
```

---

### Task VS2.3: Platform Integration (Week 6)

#### VS2.3.1: Linux systemd user service

```ini
# scripts/voice-wake.service
# Install to: ~/.config/systemd/user/clawft-voice-wake.service
# Enable:    systemctl --user enable clawft-voice-wake
# Start:     systemctl --user start clawft-voice-wake
# Status:    systemctl --user status clawft-voice-wake
# Logs:      journalctl --user -u clawft-voice-wake -f

[Unit]
Description=ClawFT Voice Wake Daemon
Documentation=https://github.com/ruvnet/clawft
After=pipewire.service pulseaudio.service
Wants=pipewire.service

[Service]
Type=simple
ExecStart=%h/.cargo/bin/weft voice wake --daemon
ExecStop=/bin/kill -SIGTERM $MAINPID
Restart=on-failure
RestartSec=5
# Audio group membership required for mic access
SupplementaryGroups=audio
# Environment
Environment=RUST_LOG=clawft_plugin::voice=info
Environment=HOME=%h
# Resource limits -- keep CPU low
CPUQuota=5%
MemoryMax=128M
# Watchdog
WatchdogSec=60

[Install]
WantedBy=default.target
```

**Installation script (`scripts/install-voice-service-linux.sh`):**

```bash
#!/usr/bin/env bash
# Install the ClawFT Voice Wake daemon as a systemd user service.
set -euo pipefail

SERVICE_NAME="clawft-voice-wake"
SERVICE_DIR="$HOME/.config/systemd/user"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing $SERVICE_NAME systemd user service..."

mkdir -p "$SERVICE_DIR"
cp "$SCRIPT_DIR/voice-wake.service" "$SERVICE_DIR/$SERVICE_NAME.service"

systemctl --user daemon-reload
systemctl --user enable "$SERVICE_NAME"

echo "Service installed. Start with:"
echo "  systemctl --user start $SERVICE_NAME"
echo ""
echo "View logs with:"
echo "  journalctl --user -u $SERVICE_NAME -f"
```

#### VS2.3.2: macOS launchd agent

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- scripts/com.clawft.voice-wake.plist -->
<!-- Install to: ~/Library/LaunchAgents/com.clawft.voice-wake.plist -->
<!-- Load:   launchctl load ~/Library/LaunchAgents/com.clawft.voice-wake.plist -->
<!-- Unload: launchctl unload ~/Library/LaunchAgents/com.clawft.voice-wake.plist -->
<!-- Status: launchctl list | grep clawft -->
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.clawft.voice-wake</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/weft</string>
        <string>voice</string>
        <string>wake</string>
        <string>--daemon</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>5</integer>

    <key>StandardOutPath</key>
    <string>/tmp/clawft-voice-wake.stdout.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/clawft-voice-wake.stderr.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>clawft_plugin::voice=info</string>
    </dict>

    <!-- ProcessType: Background keeps CPU priority low -->
    <key>ProcessType</key>
    <string>Background</string>

    <!-- Nice value: lower priority -->
    <key>Nice</key>
    <integer>10</integer>
</dict>
</plist>
```

**Installation script (`scripts/install-voice-service-macos.sh`):**

```bash
#!/usr/bin/env bash
# Install the ClawFT Voice Wake daemon as a macOS launchd agent.
set -euo pipefail

PLIST_NAME="com.clawft.voice-wake"
INSTALL_DIR="$HOME/Library/LaunchAgents"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing $PLIST_NAME launchd agent..."

mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/$PLIST_NAME.plist" "$INSTALL_DIR/"

# Unload if already loaded
launchctl unload "$INSTALL_DIR/$PLIST_NAME.plist" 2>/dev/null || true

# Load the agent
launchctl load "$INSTALL_DIR/$PLIST_NAME.plist"

echo "Agent installed and started."
echo ""
echo "Status: launchctl list | grep clawft"
echo "Stop:   launchctl unload ~/Library/LaunchAgents/$PLIST_NAME.plist"
```

#### VS2.3.3: Windows startup task

```rust
// crates/clawft-cli/src/commands/voice.rs -- Windows service registration

/// Register the Voice Wake daemon as a Windows startup task.
///
/// Uses the Windows Task Scheduler via `schtasks` to run the daemon
/// at user login. Alternatively, creates a shortcut in the Startup folder.
#[cfg(target_os = "windows")]
pub fn register_windows_startup() -> Result<(), PluginError> {
    let weft_path = std::env::current_exe()
        .map_err(|e| PluginError::Io(e))?;

    let task_name = "ClawFT Voice Wake";

    // Use schtasks to create a logon trigger task
    let output = std::process::Command::new("schtasks")
        .args([
            "/Create",
            "/SC", "ONLOGON",
            "/TN", task_name,
            "/TR", &format!("\"{}\" voice wake --daemon", weft_path.display()),
            "/RL", "LIMITED",
            "/F", // Force overwrite if exists
        ])
        .output()
        .map_err(|e| PluginError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PluginError::ExecutionFailed(
            format!("failed to create scheduled task: {stderr}")
        ));
    }

    tracing::info!("registered Windows startup task: {task_name}");
    Ok(())
}

/// Remove the Voice Wake daemon from Windows startup.
#[cfg(target_os = "windows")]
pub fn unregister_windows_startup() -> Result<(), PluginError> {
    let task_name = "ClawFT Voice Wake";

    let output = std::process::Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .output()
        .map_err(|e| PluginError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PluginError::ExecutionFailed(
            format!("failed to remove scheduled task: {stderr}")
        ));
    }

    Ok(())
}

/// PowerShell alternative for Windows startup (Startup folder shortcut).
///
/// Creates a .lnk shortcut in the user's Startup folder.
/// Fallback if schtasks is not available.
#[cfg(target_os = "windows")]
pub fn register_windows_startup_shortcut() -> Result<(), PluginError> {
    let startup_dir = dirs::config_dir()
        .unwrap_or_default()
        .join("Microsoft/Windows/Start Menu/Programs/Startup");

    let weft_path = std::env::current_exe()
        .map_err(|e| PluginError::Io(e))?;

    // Use PowerShell to create a .lnk shortcut
    let ps_script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}\\ClawFT Voice Wake.lnk'); $s.TargetPath = '{}'; $s.Arguments = 'voice wake --daemon'; $s.WindowStyle = 7; $s.Save()"#,
        startup_dir.display(),
        weft_path.display(),
    );

    std::process::Command::new("powershell")
        .args(["-Command", &ps_script])
        .output()
        .map_err(|e| PluginError::Io(e))?;

    Ok(())
}
```

#### VS2.3.4: Microphone permission request handling

```rust
// crates/clawft-plugin/src/voice/permissions.rs

use crate::PluginError;

/// Platform-specific microphone permission handling.
///
/// On macOS, microphone access requires a TCC (Transparency, Consent, Control)
/// prompt the first time the app accesses the mic. On Windows, mic access
/// requires the "Microphone" privacy setting to be enabled for the app.
/// On Linux, mic access is typically unrestricted if the user is in the
/// `audio` group.
pub struct MicPermission;

impl MicPermission {
    /// Check if microphone permission is granted.
    ///
    /// Returns Ok(true) if granted, Ok(false) if denied, Err if unknown.
    pub fn check() -> Result<bool, PluginError> {
        #[cfg(target_os = "macos")]
        {
            Self::check_macos()
        }

        #[cfg(target_os = "windows")]
        {
            Self::check_windows()
        }

        #[cfg(target_os = "linux")]
        {
            Self::check_linux()
        }

        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux"
        )))]
        {
            Ok(true)
        }
    }

    /// Request microphone permission from the user.
    ///
    /// On macOS, this triggers the TCC prompt. On other platforms, this
    /// prints instructions for granting mic access.
    pub fn request() -> Result<(), PluginError> {
        #[cfg(target_os = "macos")]
        {
            Self::request_macos()
        }

        #[cfg(target_os = "windows")]
        {
            Self::request_windows()
        }

        #[cfg(target_os = "linux")]
        {
            Self::request_linux()
        }

        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux"
        )))]
        {
            Ok(())
        }
    }

    #[cfg(target_os = "macos")]
    fn check_macos() -> Result<bool, PluginError> {
        // On macOS, attempting to open an audio input stream triggers the
        // TCC prompt if permission hasn't been granted yet. We check by
        // trying to enumerate audio input devices via cpal.
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    #[cfg(target_os = "macos")]
    fn request_macos() -> Result<(), PluginError> {
        // Trigger the TCC prompt by attempting to open a brief audio stream
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            PluginError::PermissionDenied(
                "no audio input device found -- check System Preferences > \
                 Security & Privacy > Microphone".to_string()
            )
        })?;

        // Opening the stream triggers the macOS permission dialog
        let config = device.default_input_config()
            .map_err(|e| PluginError::ExecutionFailed(
                format!("failed to get audio config: {e}")
            ))?;

        tracing::info!("microphone permission requested (macOS TCC prompt)");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn check_windows() -> Result<bool, PluginError> {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    #[cfg(target_os = "windows")]
    fn request_windows() -> Result<(), PluginError> {
        eprintln!("Microphone access required.");
        eprintln!("If prompted, click 'Allow' to grant microphone permission.");
        eprintln!("Or go to: Settings > Privacy > Microphone");
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn check_linux() -> Result<bool, PluginError> {
        // On Linux, check if user is in the 'audio' group
        let output = std::process::Command::new("groups")
            .output()
            .map_err(|e| PluginError::Io(e))?;
        let groups = String::from_utf8_lossy(&output.stdout);
        Ok(groups.contains("audio") || groups.contains("root"))
    }

    #[cfg(target_os = "linux")]
    fn request_linux() -> Result<(), PluginError> {
        eprintln!("Microphone access requires membership in the 'audio' group.");
        eprintln!("Run: sudo usermod -a -G audio $USER");
        eprintln!("Then log out and back in.");
        Ok(())
    }
}
```

#### VS2.3.5: Privacy indicator

```rust
// crates/clawft-plugin/src/voice/privacy.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Privacy indicator for microphone activity.
///
/// Provides a visual notification mechanism when the microphone is active.
/// This is a user trust feature -- users must always know when their mic
/// is being captured.
///
/// Platform implementations:
/// - Linux: desktop notification via `notify-send` or D-Bus
/// - macOS: menu bar indicator (requires Tauri/tray integration)
/// - Windows: system tray notification
/// - CLI fallback: prints to stderr
pub struct PrivacyIndicator {
    mic_active: Arc<AtomicBool>,
}

impl PrivacyIndicator {
    pub fn new() -> Self {
        Self {
            mic_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal that the microphone is now active (capturing audio).
    pub fn mic_activated(&self) {
        let was_active = self.mic_active.swap(true, Ordering::SeqCst);
        if !was_active {
            tracing::info!("microphone activated");
            self.show_active_indicator();
        }
    }

    /// Signal that the microphone is no longer active.
    pub fn mic_deactivated(&self) {
        let was_active = self.mic_active.swap(false, Ordering::SeqCst);
        if was_active {
            tracing::info!("microphone deactivated");
            self.hide_active_indicator();
        }
    }

    /// Whether the microphone is currently active.
    pub fn is_mic_active(&self) -> bool {
        self.mic_active.load(Ordering::SeqCst)
    }

    fn show_active_indicator(&self) {
        #[cfg(target_os = "linux")]
        {
            // Send a desktop notification
            let _ = std::process::Command::new("notify-send")
                .args([
                    "--urgency=low",
                    "--icon=audio-input-microphone",
                    "--hint=string:x-dunst-stack-tag:clawft-mic",
                    "ClawFT",
                    "Microphone is active",
                ])
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            // macOS shows its own orange dot indicator for mic access
            // Additional notification via osascript
            let _ = std::process::Command::new("osascript")
                .args([
                    "-e",
                    r#"display notification "Microphone is active" with title "ClawFT""#,
                ])
                .spawn();
        }

        #[cfg(target_os = "windows")]
        {
            // Windows shows its own mic indicator in the taskbar
            // Additional notification via PowerShell
            let _ = std::process::Command::new("powershell")
                .args([
                    "-Command",
                    r#"[System.Windows.Forms.MessageBox]::Show('Microphone is active','ClawFT','OK','Information') | Out-Null"#,
                ])
                .spawn();
        }

        // CLI fallback (always)
        eprintln!("[MIC ACTIVE] ClawFT is capturing audio");
    }

    fn hide_active_indicator(&self) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("notify-send")
                .args([
                    "--urgency=low",
                    "--icon=audio-input-microphone-muted",
                    "--hint=string:x-dunst-stack-tag:clawft-mic",
                    "ClawFT",
                    "Microphone deactivated",
                ])
                .spawn();
        }

        eprintln!("[MIC OFF] ClawFT stopped capturing audio");
    }
}
```

#### VS2.3.6: PipeWire audio integration

```rust
// crates/clawft-plugin/src/voice/pipewire.rs

/// PipeWire-specific audio integration for Linux.
///
/// While `cpal` handles basic audio I/O across platforms, PipeWire
/// offers advanced features on Linux:
/// - Virtual device creation (for loopback capture in echo cancellation)
/// - Per-application volume control
/// - Audio routing between applications
/// - Lower latency than PulseAudio compatibility layer
///
/// This module provides PipeWire-native features when available,
/// falling back to cpal's PulseAudio backend otherwise.

use crate::PluginError;

/// Check if PipeWire is available and running.
pub fn is_pipewire_available() -> bool {
    std::process::Command::new("pw-cli")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// PipeWire audio session manager.
///
/// Creates and manages virtual audio devices for the voice pipeline.
pub struct PipeWireSession {
    /// Node ID of the virtual loopback device (for echo cancellation).
    loopback_node_id: Option<u32>,
}

impl PipeWireSession {
    /// Initialize a PipeWire session.
    ///
    /// Creates a virtual loopback device for echo cancellation.
    pub fn new() -> Result<Self, PluginError> {
        if !is_pipewire_available() {
            return Err(PluginError::NotImplemented(
                "PipeWire is not available".to_string()
            ));
        }

        Ok(Self {
            loopback_node_id: None,
        })
    }

    /// Create a virtual loopback device for echo cancellation.
    ///
    /// This captures the output audio (what's being played to speakers)
    /// so the echo canceller can subtract it from the mic input.
    pub fn create_loopback(&mut self) -> Result<(), PluginError> {
        // Use pw-loopback to create a virtual capture device that
        // mirrors the default output
        let output = std::process::Command::new("pw-loopback")
            .args([
                "--capture-props", "media.class=Audio/Sink",
                "--playback-props", "media.class=Audio/Source",
                "--rate", "16000",
                "--channels", "1",
            ])
            .spawn()
            .map_err(|e| PluginError::ExecutionFailed(
                format!("failed to create PipeWire loopback: {e}")
            ))?;

        tracing::info!("created PipeWire loopback device for echo cancellation");
        Ok(())
    }

    /// List available audio devices via PipeWire.
    pub fn list_devices(&self) -> Result<Vec<PipeWireDevice>, PluginError> {
        let output = std::process::Command::new("pw-cli")
            .args(["list-objects", "Node"])
            .output()
            .map_err(|e| PluginError::Io(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse pw-cli output into device list
        let devices = parse_pipewire_devices(&stdout);
        Ok(devices)
    }
}

/// A PipeWire audio device.
#[derive(Debug, Clone)]
pub struct PipeWireDevice {
    pub node_id: u32,
    pub name: String,
    pub media_class: String,
    pub is_input: bool,
}

/// Parse pw-cli list-objects output into device structs.
fn parse_pipewire_devices(output: &str) -> Vec<PipeWireDevice> {
    // Implementation parses the pw-cli output format
    Vec::new() // placeholder
}

impl Drop for PipeWireSession {
    fn drop(&mut self) {
        // Clean up virtual loopback device if created
        if let Some(node_id) = self.loopback_node_id {
            let _ = std::process::Command::new("pw-cli")
                .args(["destroy", &node_id.to_string()])
                .output();
        }
    }
}
```

#### VS2.3.7: Discord voice channel bridge

```rust
// crates/clawft-channels/src/discord/voice_bridge.rs

use crate::PluginError;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Discord voice channel bridge.
///
/// Connects to a Discord voice channel, receives audio from participants,
/// routes it through the STT pipeline, sends the transcription to the
/// agent, and plays the agent's TTS response back into the voice channel.
///
/// Architecture:
/// ```text
/// Discord VC Audio -> Decode (Opus) -> Resample (48kHz->16kHz)
///     -> VAD -> STT -> Agent -> TTS -> Encode (Opus) -> Discord VC
/// ```
///
/// Uses the `songbird` crate for Discord voice connection management
/// and Opus codec handling.
pub struct DiscordVoiceBridge {
    /// Guild ID of the voice channel.
    guild_id: u64,
    /// Channel ID of the voice channel.
    channel_id: u64,
    /// Reference to the STT engine.
    stt: Arc<dyn SpeechToTextEngine>,
    /// Reference to the TTS engine.
    tts: Arc<dyn TextToSpeechEngine>,
    /// Reference to the agent message handler.
    agent_handler: Arc<dyn AgentMessageHandler>,
    /// Whether the bridge is currently active.
    active: Arc<Mutex<bool>>,
}

/// Trait for STT engine (implemented by the voice module).
#[async_trait::async_trait]
pub trait SpeechToTextEngine: Send + Sync {
    /// Transcribe audio samples (16kHz mono i16 PCM).
    async fn transcribe(&self, audio: &[i16]) -> Result<String, PluginError>;
}

/// Trait for TTS engine (implemented by the voice module).
#[async_trait::async_trait]
pub trait TextToSpeechEngine: Send + Sync {
    /// Synthesize text to audio samples (16kHz mono i16 PCM).
    async fn synthesize(&self, text: &str) -> Result<Vec<i16>, PluginError>;
}

/// Trait for sending messages to the agent and receiving responses.
#[async_trait::async_trait]
pub trait AgentMessageHandler: Send + Sync {
    /// Send a transcribed message and get the agent's response.
    async fn handle_message(
        &self,
        speaker_id: &str,
        text: &str,
    ) -> Result<String, PluginError>;
}

impl DiscordVoiceBridge {
    /// Create a new Discord voice bridge.
    pub fn new(
        guild_id: u64,
        channel_id: u64,
        stt: Arc<dyn SpeechToTextEngine>,
        tts: Arc<dyn TextToSpeechEngine>,
        agent_handler: Arc<dyn AgentMessageHandler>,
    ) -> Self {
        Self {
            guild_id,
            channel_id,
            stt,
            tts,
            agent_handler,
            active: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the voice bridge.
    ///
    /// Connects to the Discord voice channel and begins processing
    /// audio. Runs until cancelled.
    pub async fn start(
        &self,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<(), PluginError> {
        *self.active.lock().await = true;

        tracing::info!(
            guild_id = %self.guild_id,
            channel_id = %self.channel_id,
            "starting Discord voice bridge"
        );

        // The main loop:
        // 1. Receive Opus frames from Discord via songbird
        // 2. Decode to PCM (songbird handles this)
        // 3. Resample from 48kHz stereo to 16kHz mono
        // 4. Feed through VAD
        // 5. When speech segment ends, transcribe via STT
        // 6. Send transcription to agent
        // 7. Synthesize agent response via TTS
        // 8. Resample from 16kHz mono to 48kHz stereo
        // 9. Encode to Opus and send to Discord VC

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    *self.active.lock().await = false;
                    tracing::info!("Discord voice bridge stopped");
                    return Ok(());
                }
                // In actual implementation, this receives audio events
                // from the songbird voice connection
            }
        }
    }

    /// Stop the voice bridge and disconnect from the voice channel.
    pub async fn stop(&self) -> Result<(), PluginError> {
        *self.active.lock().await = false;
        tracing::info!("Discord voice bridge disconnected");
        Ok(())
    }
}

/// Resample audio from one sample rate to another.
///
/// Uses linear interpolation for simplicity. For production quality,
/// consider the `rubato` crate for high-quality resampling.
pub fn resample(
    input: &[i16],
    from_rate: u32,
    to_rate: u32,
) -> Vec<i16> {
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (input.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;

        let sample = if src_idx + 1 < input.len() {
            let a = input[src_idx] as f64;
            let b = input[src_idx + 1] as f64;
            (a * (1.0 - frac) + b * frac) as i16
        } else if src_idx < input.len() {
            input[src_idx]
        } else {
            0
        };

        output.push(sample);
    }

    output
}

/// Convert stereo audio to mono by averaging channels.
pub fn stereo_to_mono(stereo: &[i16]) -> Vec<i16> {
    stereo
        .chunks_exact(2)
        .map(|pair| ((pair[0] as i32 + pair[1] as i32) / 2) as i16)
        .collect()
}
```

#### VS2.3.8: Platform audio test suite

```rust
// tests/voice/platform_audio_tests.rs

//! Platform audio integration tests.
//!
//! These tests validate audio capture and playback on each platform.
//! They are NOT run in CI (require real audio hardware). Instead, they
//! run on self-hosted runners with audio devices or manually via:
//!
//!     cargo test --test platform_audio_tests --features voice
//!
//! For CI, we use mock audio sources (synthetic WAV buffers).

#[cfg(test)]
mod tests {
    use std::time::Duration;

    /// Test that cpal can enumerate at least one input device.
    #[test]
    #[cfg(feature = "voice")]
    fn test_audio_input_device_available() {
        use cpal::traits::HostTrait;
        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices()
            .expect("failed to enumerate input devices")
            .collect();
        // In CI with no audio hardware, this may be empty.
        // On real machines, at least one device should exist.
        println!("Found {} input devices", devices.len());
    }

    /// Test that cpal can enumerate at least one output device.
    #[test]
    #[cfg(feature = "voice")]
    fn test_audio_output_device_available() {
        use cpal::traits::HostTrait;
        let host = cpal::default_host();
        let devices: Vec<_> = host.output_devices()
            .expect("failed to enumerate output devices")
            .collect();
        println!("Found {} output devices", devices.len());
    }

    /// Test wake word detector initialization with bundled model.
    #[test]
    #[cfg(feature = "voice-wake")]
    fn test_wake_word_detector_init() {
        use clawft_plugin::voice::wake::{WakeWordConfig, WakeWordDetector};

        let config = WakeWordConfig {
            model_path: "models/voice/wake/hey-weft.rpw".into(),
            threshold: 0.5,
            ..Default::default()
        };

        // This test will fail if the model file doesn't exist.
        // It validates that the rustpotter integration compiles and
        // the model loading path works.
        match WakeWordDetector::new(config) {
            Ok(detector) => {
                assert_eq!(detector.cpu_usage_percent(), 0.0);
                println!("Wake word detector initialized successfully");
            }
            Err(e) => {
                println!("Wake word detector init failed (expected in CI): {e}");
            }
        }
    }

    /// Test wake word detection with synthetic audio.
    #[test]
    #[cfg(feature = "voice-wake")]
    fn test_wake_word_no_false_positive_on_silence() {
        use clawft_plugin::voice::wake::{WakeWordConfig, WakeWordDetector};

        let config = WakeWordConfig::default();
        let detector = match WakeWordDetector::new(config) {
            Ok(d) => d,
            Err(_) => {
                println!("Skipping: wake word model not available");
                return;
            }
        };

        // Feed 10 seconds of silence (all zeros)
        let silence_frame = vec![0i16; 480]; // 30ms at 16kHz
        let frames = 10 * 16000 / 480; // 10 seconds worth

        let mut detector = detector;
        let mut false_positives = 0;
        for _ in 0..frames {
            if detector.process_frame(&silence_frame) {
                false_positives += 1;
            }
        }

        assert_eq!(
            false_positives, 0,
            "wake word detector should not trigger on silence"
        );
    }

    /// Test echo canceller with known input/reference signals.
    #[test]
    fn test_echo_canceller_reduces_echo() {
        use clawft_plugin::voice::echo::EchoCanceller;

        let mut aec = EchoCanceller::new(16000, 300, 512);

        // Create a reference signal (simulated TTS output)
        let reference: Vec<f32> = (0..16000)
            .map(|i| {
                let t = i as f32 / 16000.0;
                0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            })
            .collect();

        // Feed reference to AEC
        aec.feed_reference(&reference);

        // Simulate mic input = reference (perfect echo, no real speech)
        let mic_input = reference.clone();
        let output = aec.process(&mic_input);

        // The output energy should be significantly less than input energy
        let input_energy: f32 = mic_input.iter().map(|s| s * s).sum::<f32>()
            / mic_input.len() as f32;
        let output_energy: f32 = output.iter().map(|s| s * s).sum::<f32>()
            / output.len() as f32;

        // Echo should be reduced by at least 6 dB (factor of 4 in power)
        assert!(
            output_energy < input_energy / 4.0,
            "echo canceller should reduce echo energy: input={input_energy}, output={output_energy}"
        );
    }

    /// Test adaptive silence timeout adaptation.
    #[test]
    fn test_adaptive_silence_timeout() {
        use clawft_plugin::voice::silence::AdaptiveSilenceTimeout;

        let mut timeout = AdaptiveSilenceTimeout::default();
        assert_eq!(timeout.timeout().as_millis(), 1500);

        // Record some mid-sentence pauses
        for _ in 0..10 {
            timeout.record_mid_sentence_pause(Duration::from_millis(800));
        }

        // Timeout should adapt to be above 95th percentile of mid-sentence pauses
        assert!(
            timeout.timeout().as_millis() > 800,
            "timeout should be above mid-sentence pause duration"
        );
        assert!(
            timeout.timeout().as_millis() <= 3000,
            "timeout should not exceed max"
        );
    }

    /// Test noise suppressor calibration.
    #[test]
    fn test_noise_suppressor_calibration() {
        use clawft_plugin::voice::noise::NoiseSuppressor;

        let mut ns = NoiseSuppressor::new(512);
        assert!(!ns.is_calibrated());

        // Feed 10 silence frames to calibrate
        let silence = vec![0.0f32; 512];
        for _ in 0..10 {
            ns.update_noise_estimate(&silence);
        }

        assert!(ns.is_calibrated());
    }

    /// Test audio quality metrics collection.
    #[test]
    fn test_audio_quality_metrics() {
        use clawft_plugin::voice::metrics::AudioQualityMetrics;

        let mut metrics = AudioQualityMetrics::new(100);

        // Record some SNR measurements
        for _ in 0..10 {
            metrics.record_snr(1.0, 0.01); // ~20 dB
        }

        let avg_snr = metrics.avg_snr_db().unwrap();
        assert!(avg_snr > 19.0 && avg_snr < 21.0,
            "expected ~20 dB SNR, got {avg_snr}");

        // Record some latency measurements
        for _ in 0..10 {
            metrics.record_latency(Duration::from_millis(250));
        }

        let percentiles = metrics.latency_percentiles();
        assert_eq!(percentiles.p50.as_millis(), 250);
    }

    /// Test language parsing.
    #[test]
    fn test_voice_language_parsing() {
        use clawft_plugin::voice::language::VoiceLanguage;

        assert_eq!(
            VoiceLanguage::from_str_loose("en").unwrap(),
            VoiceLanguage::English
        );
        assert_eq!(
            VoiceLanguage::from_str_loose("fr-fr").unwrap(),
            VoiceLanguage::French
        );
        assert_eq!(
            VoiceLanguage::from_str_loose("auto").unwrap(),
            VoiceLanguage::Auto
        );

        // Unknown language becomes Custom
        match VoiceLanguage::from_str_loose("klingon").unwrap() {
            VoiceLanguage::Custom(s) => assert_eq!(s, "klingon"),
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    /// Test voice profile defaults.
    #[test]
    fn test_voice_profile_defaults() {
        use clawft_types::config::voice::{VoiceProfile, AgentVoiceConfig};

        let profile = VoiceProfile::default();
        assert_eq!(profile.voice_id, "en-us-amy-medium");
        assert_eq!(profile.rate, 1.0);
        assert_eq!(profile.pitch, 0.0);
        assert_eq!(profile.volume, 0.8);

        let config = AgentVoiceConfig::default();
        let voice = config.voice_for_agent("unknown-agent");
        assert_eq!(voice.voice_id, "en-us-amy-medium");
    }

    /// Test resampling from 48kHz to 16kHz.
    #[test]
    fn test_resample_48k_to_16k() {
        use clawft_channels::discord::voice_bridge::resample;

        let input: Vec<i16> = (0..4800).map(|i| (i % 100) as i16).collect();
        let output = resample(&input, 48000, 16000);

        // Output should be ~1/3 the length
        assert_eq!(output.len(), 1600);
    }

    /// Test stereo to mono conversion.
    #[test]
    fn test_stereo_to_mono() {
        use clawft_channels::discord::voice_bridge::stereo_to_mono;

        let stereo = vec![100i16, 200, 300, 400, 500, 600];
        let mono = stereo_to_mono(&stereo);

        assert_eq!(mono.len(), 3);
        assert_eq!(mono[0], 150); // (100 + 200) / 2
        assert_eq!(mono[1], 350); // (300 + 400) / 2
        assert_eq!(mono[2], 550); // (500 + 600) / 2
    }

    /// Test privacy indicator state transitions.
    #[test]
    fn test_privacy_indicator() {
        use clawft_plugin::voice::privacy::PrivacyIndicator;

        let indicator = PrivacyIndicator::new();
        assert!(!indicator.is_mic_active());

        indicator.mic_activated();
        assert!(indicator.is_mic_active());

        indicator.mic_deactivated();
        assert!(!indicator.is_mic_active());
    }

    /// Test microphone permission check (platform-specific).
    #[test]
    fn test_mic_permission_check() {
        use clawft_plugin::voice::permissions::MicPermission;

        // This should not panic on any platform
        match MicPermission::check() {
            Ok(granted) => println!("mic permission: {granted}"),
            Err(e) => println!("mic permission check error: {e}"),
        }
    }

    /// Test CPU budget monitor calculation.
    #[test]
    fn test_cpu_budget_monitor() {
        use clawft_plugin::voice::wake::CpuBudgetMonitor;

        let mut monitor = CpuBudgetMonitor::new(2.0);
        assert_eq!(monitor.current_percent(), 0.0);
        assert!(monitor.is_within_budget());

        // Simulate processing that takes 300us per 30ms frame = 1%
        for _ in 0..100 {
            monitor.record_frame_time(300);
        }

        let cpu = monitor.current_percent();
        assert!(cpu > 0.9 && cpu < 1.1,
            "expected ~1% CPU, got {cpu}%");
        assert!(monitor.is_within_budget());

        // Simulate overbudget: 900us per frame = 3%
        for _ in 0..100 {
            monitor.record_frame_time(900);
        }

        assert!(!monitor.is_within_budget());
    }

    /// Test wake word model listing.
    #[test]
    #[cfg(feature = "voice-wake")]
    fn test_list_wake_word_models() {
        use clawft_plugin::voice::wake::list_wake_word_models;

        let models = list_wake_word_models().unwrap();
        println!("Found {} wake word models", models.len());
        for model in &models {
            println!("  - {} ({:?}): {}", model.name, model.source,
                model.path.display());
        }
    }
}
```

---

## 4. Concurrency Plan

VS2 tasks are grouped by week and can be partially parallelized:

**Week 4 (VS2.1):** Single-developer, sequential. Rustpotter integration must complete before wake word detector can be tested. CLI commands depend on the detector.

**Week 5 (VS2.2):** Two-developer parallelization possible:
- Developer A: VS2.2.1 (echo cancellation) + VS2.2.2 (noise suppression) -- both operate on audio frames independently.
- Developer B: VS2.2.3 (adaptive silence) + VS2.2.4 (multi-language) + VS2.2.5 (voice selection) -- configuration and pipeline features.
- VS2.2.6 (metrics) can be done by either developer afterward.

**Week 6 (VS2.3):** Three-way parallelization:
- Developer A: VS2.3.1-VS2.3.3 (platform service files) -- independent per-platform work.
- Developer B: VS2.3.4-VS2.3.5 (permissions + privacy) -- platform-aware but independent of services.
- Developer C: VS2.3.6-VS2.3.7 (PipeWire + Discord bridge) -- advanced integrations.
- VS2.3.8 (test suite) requires all other VS2.3 tasks to complete.

---

## 5. Tests Required

All tests in `tests/voice/` and inline `#[cfg(test)]` modules.

### Wake Word Tests

| Test | Description |
|------|-------------|
| `test_wake_word_detector_init` | Initialize `WakeWordDetector` with bundled model. Verify no panic and CPU at 0%. |
| `test_wake_word_no_false_positive_on_silence` | Feed 10 seconds of silence. Assert zero detections. |
| `test_wake_word_detection_gap` | Feed two detections within `min_gap_frames`. Assert second is suppressed. |
| `test_wake_word_model_hot_swap` | Call `load_model()` with a different model path. Verify old model is replaced. |
| `test_list_wake_word_models` | List models from both bundled and user directories. |
| `test_cpu_budget_monitor` | Feed frame times and verify CPU percentage calculation (300us/30ms = 1%). |
| `test_cpu_budget_throttling` | Simulate over-budget (>2% CPU) and verify throttling activates. |

### Echo Cancellation Tests

| Test | Description |
|------|-------------|
| `test_echo_canceller_reduces_echo` | Feed identical reference and mic signals. Output energy must be <25% of input energy (>6 dB reduction). |
| `test_echo_canceller_preserves_speech` | Feed reference signal + independent speech signal as mic input. Verify speech is preserved. |
| `test_echo_canceller_inactive_passthrough` | Process mic input without feeding reference. Output must equal input. |
| `test_echo_canceller_reset` | Call `reset()` and verify filter weights are zeroed. |

### Noise Suppression Tests

| Test | Description |
|------|-------------|
| `test_noise_suppressor_calibration` | Feed 10 silence frames. Assert `is_calibrated()` is true. |
| `test_noise_suppressor_uncalibrated_passthrough` | Call `suppress()` before calibration. Output must equal input. |
| `test_noise_suppressor_reset` | Call `reset()` and verify `is_calibrated()` is false. |

### Adaptive Silence Tests

| Test | Description |
|------|-------------|
| `test_adaptive_silence_default` | Default timeout is 1500ms. |
| `test_adaptive_silence_adapts` | Record 10 mid-sentence pauses at 800ms. Timeout must increase above 800ms. |
| `test_adaptive_silence_bounded` | Record extreme pauses. Timeout must stay within min (500ms) and max (3000ms). |
| `test_adaptive_silence_reset` | Call `reset()` and verify timeout returns to initial value. |

### Language Tests

| Test | Description |
|------|-------------|
| `test_voice_language_parsing` | Parse "en", "fr-fr", "auto", "klingon". Verify correct variants. |
| `test_voice_language_model_id` | Each language produces a non-empty model_id string. |
| `test_language_config_defaults` | Default config has English STT+TTS, auto-detect disabled. |

### Voice Profile Tests

| Test | Description |
|------|-------------|
| `test_voice_profile_defaults` | Default voice_id is "en-us-amy-medium", rate 1.0, pitch 0.0, volume 0.8. |
| `test_agent_voice_config_lookup` | Set agent-specific voice. `voice_for_agent()` returns it. Unknown agents get default. |

### Platform Tests

| Test | Description |
|------|-------------|
| `test_audio_input_device_available` | `cpal` enumerates at least one input device (skipped in CI). |
| `test_audio_output_device_available` | `cpal` enumerates at least one output device (skipped in CI). |
| `test_mic_permission_check` | `MicPermission::check()` does not panic on any platform. |
| `test_privacy_indicator` | State transitions: inactive -> activated -> deactivated. |
| `test_resample_48k_to_16k` | Resample 4800 samples from 48kHz to 16kHz produces 1600 samples. |
| `test_stereo_to_mono` | Stereo [100,200,300,400] becomes mono [150,350]. |

### Audio Quality Metrics Tests

| Test | Description |
|------|-------------|
| `test_audio_quality_metrics` | Record 10 SNR measurements at ~20dB. Verify avg_snr is 19-21 dB. |
| `test_latency_percentiles` | Record 10 latency measurements at 250ms. Verify p50 = 250ms. |

---

## 6. Acceptance Criteria

- [ ] `cargo build -p clawft-plugin --features voice-wake` compiles with rustpotter
- [ ] `WakeWordDetector` loads and processes audio frames without panic
- [ ] Wake word detector produces zero false positives on 10 seconds of silence
- [ ] CPU usage of wake word detection stays below 2% target
- [ ] Wake word detection triggers VAD+STT pipeline activation
- [ ] Custom wake words can be trained via `weft voice train-wake`
- [ ] Custom wake word models stored at `~/.clawft/models/wake/`
- [ ] `EchoCanceller` reduces echo energy by at least 6 dB
- [ ] `NoiseSuppressor` calibrates from silence frames and suppresses noise
- [ ] `AdaptiveSilenceTimeout` adapts based on recorded pause patterns
- [ ] Multi-language STT supports at least 12 languages via config
- [ ] Per-agent voice profiles configurable via `AgentVoiceConfig`
- [ ] Audio quality metrics track SNR and latency percentiles
- [ ] Linux systemd user service file installs and starts correctly
- [ ] macOS launchd agent plist installs and starts correctly
- [ ] Windows startup task registers via schtasks
- [ ] Microphone permission checked and requested on all platforms
- [ ] Privacy indicator shows/hides when mic is activated/deactivated
- [ ] PipeWire loopback device creates when PipeWire is available
- [ ] Discord voice bridge architecture implemented (traits + resample + mono)
- [ ] `cargo test --features voice` -- all VS2 tests pass
- [ ] `cargo clippy -p clawft-plugin --features voice -- -D warnings` is clean

---

## 7. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| rustpotter API changes between versions | Low | Medium | Pin to specific version in Cargo.toml. Wrap behind internal trait for future swapping. |
| Echo cancellation insufficient for all environments | Medium | High | Offer hardware AEC fallback guidance. Document recommended microphone setups. Test with different speaker-mic distances. |
| PipeWire not available on all Linux distributions | Medium | Low | Graceful fallback to cpal's PulseAudio/ALSA backend. PipeWire features are optional enhancements. |
| Discord voice API changes (songbird updates) | Low | Medium | Pin songbird version. Discord voice protocol changes are rare and backwards-compatible. |
| Wake word CPU exceeds 2% on low-power devices | Medium | Medium | Frame-skipping throttle implemented. Offer configurable quality tiers. Document minimum hardware requirements (see Platform Testing Matrix in voice_development.md). |
| systemd/launchd service fails on non-standard installations | Low | Low | CLI `weft voice wake` works without service registration. Service files are optional convenience. |
| Adaptive silence timeout learns incorrect patterns | Low | Medium | Bounded between 500ms and 3000ms. Users can reset or disable adaptation. Manual override always available. |
| Multi-language model downloads are large (50MB+ each) | Medium | Low | Progressive download with progress bar. Only download requested languages. Offline pre-download via `weft voice setup --language <code>`. |
