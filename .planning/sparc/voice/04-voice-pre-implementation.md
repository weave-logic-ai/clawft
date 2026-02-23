# Voice Pre-Implementation Validation (VP1--VP5)

> **Element:** Voice Development
> **Phase:** Week 0 -- Pre-Implementation
> **Timeline:** 1 week (5 working days)
> **Priority:** P0 (Gate -- voice sprints cannot begin until all VP tasks pass)
> **Directory:** `voice-proto/` (standalone, NOT integrated into clawft workspace)
> **Dependencies IN:** None (standalone validation, no clawft crate dependencies)
> **Blocks:** VS1.1 (Audio Foundation), VS1.2 (STT + TTS), VS1.3 (VoiceChannel), all subsequent voice sprints
> **Status:** Planning

---

## Table of Contents

1. [Overview](#1-overview)
2. [VP1: Audio Pipeline Prototype](#2-vp1-audio-pipeline-prototype)
3. [VP2: Model Hosting & Download Strategy](#3-vp2-model-hosting--download-strategy)
4. [VP3: Feature Flag Design](#4-vp3-feature-flag-design)
5. [VP4: Platform Audio Testing](#5-vp4-platform-audio-testing)
6. [VP5: Echo Cancellation Feasibility](#6-vp5-echo-cancellation-feasibility)
7. [Week 0 Schedule](#7-week-0-schedule)
8. [Combined Exit Criteria](#8-combined-exit-criteria)

---

## 1. Overview

Week 0 is a validation gate. Its purpose is to prove that the voice technology stack works before committing to a 9-week sprint. Every VP task produces a concrete artifact and has binary pass/fail exit criteria. If any VP task fails, the voice sprint plan must be revised before proceeding.

**Guiding principles:**

- All prototype code lives in `voice-proto/` with its own `Cargo.toml`. It is NOT a workspace member of the clawft monorepo.
- No clawft crate dependencies are used. The prototype validates raw crate functionality in isolation.
- Every VP task produces a written report in `voice-proto/reports/`.
- The prototype is throwaway code. Sprint VS1 rewrites everything cleanly inside `clawft-plugin`.

**Success = all 5 VP tasks pass their exit criteria.**

---

## 2. VP1: Audio Pipeline Prototype

### 2.1 Objective

Build a standalone Rust binary that validates the end-to-end audio pipeline:

```text
mic capture (cpal) -> VAD (sherpa-rs Silero V5) -> STT (sherpa-rs streaming) -> print text to stdout
```

This proves that `sherpa-rs` and `cpal` compile, link, and run correctly before any integration work begins.

### 2.2 Directory Structure

```
voice-proto/
  Cargo.toml
  src/
    main.rs              # Entry point: argument parsing, pipeline orchestration
    audio_capture.rs     # cpal microphone input stream
    vad.rs               # sherpa-rs Silero VAD wrapper
    stt.rs               # sherpa-rs streaming recognizer wrapper
  models/                # Downloaded models (gitignored)
    silero_vad.onnx
    streaming-zipformer/
  reports/
    vp1-benchmark.md     # Latency and accuracy results
  README.md              # Build + run instructions per platform
  .gitignore
```

### 2.3 Cargo.toml Specification

```toml
[package]
name = "voice-proto"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
sherpa-rs = { version = "0.1", features = ["cuda"] }  # Verify exact version at prototype time
cpal = "0.15"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
hound = "3.5"                 # WAV file reading for benchmark tests
crossbeam-channel = "0.5"     # Audio thread -> processing thread communication

[target.'cfg(target_os = "linux")'.dependencies]
# cpal pulls in ALSA bindings automatically

[target.'cfg(target_os = "windows")'.dependencies]
# cpal pulls in WASAPI bindings automatically

[target.'cfg(target_os = "macos")'.dependencies]
# cpal pulls in CoreAudio bindings automatically

[profile.release]
opt-level = 3
lto = "thin"
```

**Note:** The exact `sherpa-rs` version and feature flags must be confirmed during prototype execution. The `cuda` feature should be optional and tested separately. The baseline prototype targets CPU-only inference.

### 2.4 main.rs Pseudocode

```rust
// voice-proto/src/main.rs -- pseudocode, NOT final implementation

use clap::Parser;

#[derive(Parser)]
struct Args {
    /// Path to sherpa-onnx model directory
    #[arg(long, default_value = "models/streaming-zipformer")]
    model_dir: String,

    /// Path to Silero VAD model
    #[arg(long, default_value = "models/silero_vad.onnx")]
    vad_model: String,

    /// Run latency benchmark mode (uses WAV file instead of live mic)
    #[arg(long)]
    benchmark: Option<String>,

    /// List available audio devices and exit
    #[arg(long)]
    list_devices: bool,

    /// Select specific input device by index
    #[arg(long)]
    device: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.list_devices {
        // Enumerate cpal input devices, print name + sample rates, exit
        audio_capture::list_devices()?;
        return Ok(());
    }

    // 1. Initialize STT recognizer (sherpa-rs streaming)
    let stt = stt::StreamingRecognizer::new(&args.model_dir)?;

    // 2. Initialize VAD (sherpa-rs Silero)
    let vad = vad::VoiceActivityDetector::new(&args.vad_model)?;

    // 3. Set up audio pipeline
    //    - cpal captures 16kHz mono i16 samples in 30ms chunks (480 samples)
    //    - Chunks are sent to processing thread via crossbeam channel
    let (tx, rx) = crossbeam_channel::bounded::<Vec<i16>>(32);

    if let Some(wav_path) = args.benchmark {
        // Benchmark mode: read WAV file, feed through pipeline, measure latency
        audio_capture::feed_wav(&wav_path, tx)?;
    } else {
        // Live mode: capture from microphone
        let _stream = audio_capture::start_capture(args.device, tx)?;
        // Stream must be kept alive (RAII)
    }

    // 4. Processing loop (runs on main thread)
    let mut speech_active = false;
    let mut speech_start_time = None;

    loop {
        let chunk = rx.recv()?;

        // Feed chunk to VAD
        let is_speech = vad.process(&chunk)?;

        if is_speech && !speech_active {
            speech_active = true;
            speech_start_time = Some(std::time::Instant::now());
            eprintln!("[VAD] Speech started");
        }

        if speech_active {
            // Feed audio to STT
            stt.accept_waveform(&chunk)?;

            // Check for partial results
            if let Some(partial) = stt.partial_result()? {
                eprint!("\r[partial] {}", partial);
            }
        }

        if !is_speech && speech_active {
            speech_active = false;
            // Get final result
            let result = stt.final_result()?;
            let latency = speech_start_time
                .map(|t| t.elapsed())
                .unwrap_or_default();

            println!("[FINAL] {} (latency: {:?})", result, latency);
            stt.reset()?;
        }
    }
}
```

### 2.5 Audio Capture Module

```rust
// voice-proto/src/audio_capture.rs -- pseudocode

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;

const SAMPLE_RATE: u32 = 16_000;
const CHANNELS: u16 = 1;
const CHUNK_SAMPLES: usize = 480; // 30ms at 16kHz

pub fn list_devices() -> anyhow::Result<()> {
    let host = cpal::default_host();
    for (i, device) in host.input_devices()?.enumerate() {
        let name = device.name().unwrap_or_else(|_| "Unknown".into());
        let configs: Vec<_> = device
            .supported_input_configs()?
            .map(|c| format!("{}Hz-{}Hz", c.min_sample_rate().0, c.max_sample_rate().0))
            .collect();
        println!("[{}] {} ({})", i, name, configs.join(", "));
    }
    Ok(())
}

pub fn start_capture(
    device_index: Option<usize>,
    tx: Sender<Vec<i16>>,
) -> anyhow::Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = match device_index {
        Some(idx) => host.input_devices()?.nth(idx)
            .ok_or_else(|| anyhow::anyhow!("No device at index {}", idx))?,
        None => host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device"))?,
    };

    let config = cpal::StreamConfig {
        channels: CHANNELS,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Fixed(CHUNK_SAMPLES as u32),
    };

    // Build input stream -- i16 format
    // Accumulate samples into CHUNK_SAMPLES-sized buffers
    // Send complete chunks via tx
    // Handle sample rate conversion if device doesn't support 16kHz natively

    let stream = device.build_input_stream(
        &config,
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            // Buffer accumulation and chunking logic
            let _ = tx.send(data.to_vec());
        },
        |err| eprintln!("[audio error] {}", err),
        None,
    )?;

    stream.play()?;
    Ok(stream)
}

pub fn feed_wav(path: &str, tx: Sender<Vec<i16>>) -> anyhow::Result<()> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, SAMPLE_RATE, "WAV must be 16kHz");
    assert_eq!(spec.channels, CHANNELS, "WAV must be mono");

    let samples: Vec<i16> = reader.into_samples::<i16>().filter_map(Result::ok).collect();
    for chunk in samples.chunks(CHUNK_SAMPLES) {
        tx.send(chunk.to_vec())?;
        // Simulate real-time pacing for accurate latency measurement
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    Ok(())
}
```

### 2.6 VAD Module

```rust
// voice-proto/src/vad.rs -- pseudocode

pub struct VoiceActivityDetector {
    // sherpa-rs Silero VAD V5 instance
    // Configuration: min speech duration, min silence duration, threshold
}

impl VoiceActivityDetector {
    pub fn new(model_path: &str) -> anyhow::Result<Self> {
        // Initialize sherpa-rs VAD with:
        //   sample_rate: 16000
        //   min_speech_duration: 0.25s  (avoid single-syllable triggers)
        //   min_silence_duration: 0.5s  (detect speech end)
        //   threshold: 0.5             (default sensitivity)
        todo!()
    }

    /// Returns true if the chunk contains speech.
    /// Internally maintains state across calls (Silero VAD is stateful).
    pub fn process(&mut self, samples: &[i16]) -> anyhow::Result<bool> {
        // Convert i16 to f32 (sherpa-rs expects f32)
        // Feed to VAD
        // Return speech/silence decision
        todo!()
    }
}
```

### 2.7 STT Module

```rust
// voice-proto/src/stt.rs -- pseudocode

pub struct StreamingRecognizer {
    // sherpa-rs online/streaming recognizer instance
}

impl StreamingRecognizer {
    pub fn new(model_dir: &str) -> anyhow::Result<Self> {
        // Initialize sherpa-rs streaming recognizer with:
        //   model_type: streaming zipformer (int8 quantized)
        //   sample_rate: 16000
        //   num_threads: 2 (configurable)
        //   decoding_method: "greedy_search"
        todo!()
    }

    pub fn accept_waveform(&mut self, samples: &[i16]) -> anyhow::Result<()> {
        // Convert i16 to f32
        // Feed to streaming recognizer
        todo!()
    }

    pub fn partial_result(&self) -> anyhow::Result<Option<String>> {
        // Return partial transcription if available
        todo!()
    }

    pub fn final_result(&mut self) -> anyhow::Result<String> {
        // Finalize and return complete transcription
        todo!()
    }

    pub fn reset(&mut self) -> anyhow::Result<()> {
        // Reset recognizer state for next utterance
        todo!()
    }
}
```

### 2.8 Benchmark Protocol

Run the prototype against a standardized test corpus to collect metrics.

**Test corpus (create or download):**

| File | Content | Duration | Purpose |
|------|---------|----------|---------|
| `test_hello.wav` | "Hello, how are you?" | ~2s | Basic phrase accuracy |
| `test_numbers.wav` | "One two three four five six seven eight nine ten" | ~4s | Number recognition |
| `test_commands.wav` | "Create a new file called main dot rust in the source directory" | ~5s | Technical phrase accuracy |
| `test_silence_30s.wav` | 30 seconds of ambient room noise | 30s | VAD false positive test |
| `test_mixed.wav` | Alternating speech (2s) and silence (3s), 5 cycles | ~25s | VAD boundary detection |

**Benchmark command:**

```bash
cargo run --release -- --benchmark test_hello.wav
cargo run --release -- --benchmark test_silence_30s.wav
```

### 2.9 Benchmark Result Template

Results must be recorded in `voice-proto/reports/vp1-benchmark.md` using this format:

```markdown
# VP1 Benchmark Results

## Environment
- Date: YYYY-MM-DD
- OS: [Linux/macOS/Windows] [version]
- Audio backend: [PipeWire/PulseAudio/CoreAudio/WASAPI]
- CPU: [model]
- RAM: [amount]
- sherpa-rs version: [version]
- cpal version: [version]

## Latency Metrics

| Metric | Target | Measured | Pass? |
|--------|--------|----------|-------|
| Speech-end to text-available (p50) | < 500ms | ___ms | [ ] |
| Speech-end to text-available (p95) | < 800ms | ___ms | [ ] |
| VAD speech-start detection | < 300ms | ___ms | [ ] |
| VAD speech-end detection | < 300ms | ___ms | [ ] |
| STT partial result latency | < 200ms | ___ms | [ ] |
| Model load time (cold start) | < 5s | ___s | [ ] |

## VAD Accuracy

| Test | Target | Measured | Pass? |
|------|--------|----------|-------|
| False trigger rate on 30s silence | < 3% | ___% | [ ] |
| Speech detection rate (known speech) | > 98% | ___% | [ ] |
| Boundary accuracy (within 200ms of true boundary) | > 90% | ___% | [ ] |

## STT Accuracy

| Test File | Reference Text | Recognized Text | WER | Pass? |
|-----------|---------------|-----------------|-----|-------|
| test_hello.wav | "Hello how are you" | ___ | ___% | [ ] |
| test_numbers.wav | "One two three..." | ___ | ___% | [ ] |
| test_commands.wav | "Create a new file..." | ___ | ___% | [ ] |

WER Target: < 10% (> 90% accuracy)

## Resource Usage

| Metric | Target | Measured | Pass? |
|--------|--------|----------|-------|
| Peak RSS (runtime) | < 500 MB | ___ MB | [ ] |
| CPU usage (listening, idle speech) | < 5% | ___% | [ ] |
| CPU usage (active transcription) | < 30% | ___% | [ ] |
| Model files disk usage | < 100 MB | ___ MB | [ ] |

## Platform Results

| Platform | Builds? | Captures? | VAD? | STT? | Notes |
|----------|---------|-----------|------|------|-------|
| Linux (PipeWire) | [ ] | [ ] | [ ] | [ ] | |
| Linux (PulseAudio) | [ ] | [ ] | [ ] | [ ] | |
| macOS (CoreAudio) | [ ] | [ ] | [ ] | [ ] | |
| Windows (WASAPI) | [ ] | [ ] | [ ] | [ ] | |
| WSL2 (PulseAudio bridge) | [ ] | [ ] | [ ] | [ ] | |
```

### 2.10 VP1 Exit Criteria

- [ ] `voice-proto` compiles on Linux x86_64, macOS aarch64, and Windows x86_64
- [ ] `--list-devices` enumerates audio devices on all 3 platforms
- [ ] Live microphone capture produces audio data (verified by logging sample count)
- [ ] VAD correctly identifies speech boundaries in `test_mixed.wav`
- [ ] VAD false trigger rate on `test_silence_30s.wav` is < 3%
- [ ] STT transcribes `test_hello.wav` with > 90% accuracy
- [ ] STT transcribes `test_commands.wav` with > 90% accuracy
- [ ] End-to-end latency (speech-end to text-available) < 500ms at p50
- [ ] Benchmark report is written to `voice-proto/reports/vp1-benchmark.md`
- [ ] No unsafe code is used outside of FFI boundaries managed by sherpa-rs/cpal

---

## 3. VP2: Model Hosting & Download Strategy

### 3.1 Objective

Define and validate the model distribution infrastructure. Models are large binary files (50--100 MB total) that cannot be bundled in the clawft binary or git repository. This task designs the download, caching, integrity verification, and offline operation strategy.

### 3.2 Hosting Architecture

```text
Primary:   GitHub Releases (clawft-voice-models repo)
Mirror:    HuggingFace Hub (clawft-org/voice-models)
Fallback:  Direct URL (configurable in clawft config)

Download flow:
  1. User runs `weft voice` or `weft voice setup`
  2. Check ~/.clawft/models/voice/manifest.json for existing models
  3. If models present + checksums match -> skip download
  4. If models missing or corrupt -> download from primary
  5. If primary fails -> try mirror
  6. If mirror fails -> report error with manual download instructions
  7. Verify SHA-256 after download
  8. Write updated manifest.json
```

### 3.3 Cache Directory Layout

```
~/.clawft/
  models/
    voice/
      manifest.json              # Model inventory + checksums
      stt/
        streaming-zipformer-en/
          encoder.int8.onnx      # ~35 MB
          decoder.int8.onnx      # ~15 MB
          tokens.txt             # ~50 KB
          metadata.json          # Model info (language, version, source)
      tts/
        vits-en/
          model.onnx             # ~25 MB
          lexicon.txt            # ~2 MB
          tokens.txt             # ~50 KB
          metadata.json
      vad/
        silero-v5/
          silero_vad.onnx        # ~2 MB
          metadata.json
      wake/
        hey-weft/
          model.rpw              # ~500 KB (rustpotter format)
          metadata.json
        custom/                  # User-trained wake word models
```

### 3.4 Model Size Budget

| Model | Type | Size | Required? | Download Trigger |
|-------|------|------|-----------|-----------------|
| Streaming Zipformer (int8, English) | STT | ~50 MB | Yes (for `voice-stt`) | `weft voice setup` or first `weft voice` |
| VITS English | TTS | ~30 MB | Yes (for `voice-tts`) | `weft voice setup` or first `weft voice` |
| Silero VAD V5 | VAD | ~2 MB | Yes (always) | Bundled with STT download |
| Hey Weft | Wake word | ~500 KB | No | `weft voice wake` first run |
| Additional STT languages | STT | ~50 MB each | No | `weft voice setup --lang <code>` |
| Additional TTS voices | TTS | ~20-40 MB each | No | `weft voice setup --voice <name>` |

**Total required download:** ~82 MB (STT + TTS + VAD)
**Total with wake word:** ~82.5 MB

### 3.5 SHA-256 Manifest Format

**JSON Schema:**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClawFT Voice Model Manifest",
  "type": "object",
  "required": ["version", "models"],
  "properties": {
    "version": {
      "type": "integer",
      "description": "Manifest schema version",
      "const": 1
    },
    "generated_at": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp of manifest generation"
    },
    "models": {
      "type": "object",
      "additionalProperties": {
        "type": "object",
        "required": ["files", "total_size", "category"],
        "properties": {
          "category": {
            "type": "string",
            "enum": ["stt", "tts", "vad", "wake"]
          },
          "version": {
            "type": "string",
            "description": "Model version string"
          },
          "language": {
            "type": "string",
            "description": "ISO 639-1 language code or 'multi'"
          },
          "total_size": {
            "type": "integer",
            "description": "Total size in bytes of all files"
          },
          "description": {
            "type": "string"
          },
          "source_url": {
            "type": "string",
            "format": "uri",
            "description": "Primary download URL (GitHub Releases)"
          },
          "mirror_url": {
            "type": "string",
            "format": "uri",
            "description": "Mirror download URL (HuggingFace)"
          },
          "files": {
            "type": "object",
            "additionalProperties": {
              "type": "object",
              "required": ["sha256", "size"],
              "properties": {
                "sha256": {
                  "type": "string",
                  "pattern": "^[a-f0-9]{64}$"
                },
                "size": {
                  "type": "integer",
                  "description": "File size in bytes"
                }
              }
            }
          }
        }
      }
    }
  }
}
```

### 3.6 Example manifest.json

```json
{
  "version": 1,
  "generated_at": "2026-02-23T00:00:00Z",
  "models": {
    "streaming-zipformer-en": {
      "category": "stt",
      "version": "2024.12.1",
      "language": "en",
      "total_size": 52428800,
      "description": "Streaming Zipformer English STT model (int8 quantized)",
      "source_url": "https://github.com/clawft/voice-models/releases/download/v0.1.0/streaming-zipformer-en.tar.gz",
      "mirror_url": "https://huggingface.co/clawft/voice-models/resolve/main/streaming-zipformer-en.tar.gz",
      "files": {
        "encoder.int8.onnx": {
          "sha256": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
          "size": 36700160
        },
        "decoder.int8.onnx": {
          "sha256": "f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5",
          "size": 15728640
        },
        "tokens.txt": {
          "sha256": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
          "size": 51200
        }
      }
    },
    "vits-en": {
      "category": "tts",
      "version": "2024.11.1",
      "language": "en",
      "total_size": 28311552,
      "description": "VITS English TTS model",
      "source_url": "https://github.com/clawft/voice-models/releases/download/v0.1.0/vits-en.tar.gz",
      "mirror_url": "https://huggingface.co/clawft/voice-models/resolve/main/vits-en.tar.gz",
      "files": {
        "model.onnx": {
          "sha256": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
          "size": 26214400
        },
        "lexicon.txt": {
          "sha256": "7890abcdef1234567890abcdef1234567890abcdef1234567890abcdef123456",
          "size": 2097152
        }
      }
    },
    "silero-v5": {
      "category": "vad",
      "version": "5.0.0",
      "language": "multi",
      "total_size": 2097152,
      "description": "Silero VAD V5 voice activity detection model",
      "source_url": "https://github.com/clawft/voice-models/releases/download/v0.1.0/silero-v5.tar.gz",
      "mirror_url": "https://huggingface.co/clawft/voice-models/resolve/main/silero-v5.tar.gz",
      "files": {
        "silero_vad.onnx": {
          "sha256": "deadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef",
          "size": 2097152
        }
      }
    },
    "hey-weft": {
      "category": "wake",
      "version": "1.0.0",
      "language": "en",
      "total_size": 512000,
      "description": "Hey Weft wake word model (rustpotter format)",
      "source_url": "https://github.com/clawft/voice-models/releases/download/v0.1.0/hey-weft.rpw",
      "mirror_url": "https://huggingface.co/clawft/voice-models/resolve/main/hey-weft.rpw",
      "files": {
        "model.rpw": {
          "sha256": "cafe0000deadbeef1234567890abcdef1234567890abcdef1234567890abcdef",
          "size": 512000
        }
      }
    }
  }
}
```

### 3.7 Download Flow Specification

```text
weft voice setup
  |
  v
[1] Check ~/.clawft/models/voice/manifest.json exists?
  |-- No  --> Download remote manifest from source_url/manifest.json
  |-- Yes --> Read local manifest
  |
  v
[2] For each required model (stt, tts, vad):
  |
  [2a] Check local files exist at expected paths?
  |     |-- No  --> Queue for download
  |     |-- Yes --> Verify SHA-256
  |                   |-- Match   --> Skip (already valid)
  |                   |-- Mismatch --> Delete corrupt files, queue for download
  |
  v
[3] Download queued models:
  |
  [3a] Show progress bar: "Downloading STT model (50 MB)..."
  [3b] Try primary source_url
  |     |-- Success --> Extract archive, verify individual file SHA-256s
  |     |-- Failure --> Try mirror_url
  |                      |-- Success --> Extract, verify
  |                      |-- Failure --> Error with manual download instructions
  |
  v
[4] Write updated local manifest.json with download timestamps
  |
  v
[5] Print summary:
    "Voice models ready. STT: streaming-zipformer-en, TTS: vits-en, VAD: silero-v5"
```

### 3.8 Offline Mode

**Requirement:** Once models are downloaded, voice features must work without any internet connection.

- Model paths are resolved locally from `~/.clawft/models/voice/`
- No network calls are made during STT/TTS/VAD inference
- The manifest.json is used only for integrity verification and download management
- `weft voice setup --check` validates local models without network access
- Config option `voice.offline_only = true` prevents any download attempts

### 3.9 VP2 Exit Criteria

- [ ] Manifest JSON schema is finalized and validated with a JSON schema validator
- [ ] Example manifest.json is created with realistic model entries
- [ ] Cache directory layout is documented and matches the manifest structure
- [ ] Download flow handles: fresh install, partial download, corrupt file, offline mode
- [ ] Model size budget is confirmed (total < 100 MB for required models)
- [ ] GitHub Releases hosting approach is validated (create test release with dummy model file)
- [ ] HuggingFace mirror approach is validated (create test repository with dummy model file)
- [ ] SHA-256 verification logic is prototyped (can be a simple Rust fn or shell script)
- [ ] Report written to `voice-proto/reports/vp2-hosting.md`

---

## 4. VP3: Feature Flag Design

### 4.1 Objective

Design the Cargo feature flag tree that allows partial voice builds. Users who only need STT should not pull in TTS or wake word dependencies. The voice feature adds significant binary size and native dependencies; fine-grained feature flags keep the default binary lean.

### 4.2 Feature Tree

```text
voice (meta-feature, enables all voice)
  |
  +-- voice-stt (speech-to-text)
  |     +-- dep:sherpa-rs
  |     +-- dep:cpal
  |
  +-- voice-tts (text-to-speech)
  |     +-- dep:sherpa-rs  (shared with voice-stt)
  |     +-- dep:cpal       (shared with voice-stt)
  |
  +-- voice-wake (always-on wake word)
        +-- dep:rustpotter
        +-- dep:cpal       (shared)
        +-- implies: voice-stt (wake word needs STT to process after wake)
```

### 4.3 Cargo.toml Specification

```toml
# crates/clawft-plugin/Cargo.toml (voice features section)

[features]
default = []

# Meta-feature: enables full voice pipeline
voice = ["voice-stt", "voice-tts", "voice-wake"]

# Speech-to-text: microphone capture + VAD + streaming transcription
voice-stt = ["dep:sherpa-rs", "dep:cpal"]

# Text-to-speech: text synthesis + speaker output
voice-tts = ["dep:sherpa-rs", "dep:cpal"]

# Wake word: always-on listening with custom wake phrases
# Implies voice-stt because wake detection triggers transcription
voice-wake = ["voice-stt", "dep:rustpotter"]

[dependencies]
# Voice dependencies (all optional, gated by features)
sherpa-rs = { version = "0.1", optional = true }
cpal = { version = "0.15", optional = true }
rustpotter = { version = "1.0", optional = true }
```

```toml
# crates/clawft-cli/Cargo.toml (propagate features from plugin)

[features]
default = []
voice = ["clawft-plugin/voice"]
voice-stt = ["clawft-plugin/voice-stt"]
voice-tts = ["clawft-plugin/voice-tts"]
voice-wake = ["clawft-plugin/voice-wake"]
```

```toml
# Root Cargo.toml workspace (if applicable)

[workspace.features]
voice = ["clawft-cli/voice"]
```

### 4.4 Conditional Compilation Examples

```rust
// crates/clawft-plugin/src/voice/mod.rs

#[cfg(feature = "voice-stt")]
pub mod stt;

#[cfg(feature = "voice-tts")]
pub mod tts;

#[cfg(feature = "voice-wake")]
pub mod wake;

#[cfg(any(feature = "voice-stt", feature = "voice-tts"))]
pub mod audio;  // Shared audio capture/playback (cpal)

#[cfg(any(feature = "voice-stt", feature = "voice-tts", feature = "voice-wake"))]
pub mod models;  // Model download manager (needed by all voice features)

#[cfg(feature = "voice-stt")]
pub mod vad;  // VAD is part of STT pipeline
```

```rust
// crates/clawft-plugin/src/voice/stt.rs

#[cfg(feature = "voice-stt")]
use sherpa_rs::online_recognizer::OnlineRecognizer;

#[cfg(feature = "voice-stt")]
pub struct SpeechToText {
    recognizer: OnlineRecognizer,
    // ...
}

#[cfg(feature = "voice-stt")]
impl SpeechToText {
    pub fn new(model_dir: &str) -> Result<Self, VoiceError> {
        // ...
    }

    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String, VoiceError> {
        // ...
    }
}
```

```rust
// crates/clawft-cli/src/commands/voice.rs

#[cfg(any(feature = "voice-stt", feature = "voice-tts", feature = "voice-wake"))]
pub mod voice_cmd {
    use clap::Subcommand;

    #[derive(Subcommand)]
    pub enum VoiceCommand {
        /// Download and set up voice models
        Setup {
            #[arg(long)]
            lang: Option<String>,
        },

        #[cfg(feature = "voice-stt")]
        /// Test microphone input
        TestMic,

        #[cfg(feature = "voice-tts")]
        /// Test text-to-speech output
        TestSpeak {
            text: String,
        },

        #[cfg(feature = "voice-stt")]
        /// Start talk mode (continuous voice conversation)
        Talk,

        #[cfg(feature = "voice-wake")]
        /// Start wake word listener
        Wake,

        #[cfg(feature = "voice-wake")]
        /// Train a custom wake word
        TrainWake {
            phrase: String,
        },
    }
}

// When no voice features are enabled, provide a helpful error
#[cfg(not(any(feature = "voice-stt", feature = "voice-tts", feature = "voice-wake")))]
pub mod voice_cmd {
    pub fn not_available() {
        eprintln!("Voice features are not enabled in this build.");
        eprintln!("Rebuild with: cargo build --features voice");
        std::process::exit(1);
    }
}
```

### 4.5 Binary Size Impact

Estimate binary size impact per feature combination (to be measured during VP1):

| Feature Combination | Estimated Binary Delta | Native Deps | Notes |
|--------------------|-----------------------|-------------|-------|
| No voice features | Baseline (0) | None | Default build |
| `voice-stt` only | +3--5 MB | cpal, sherpa-rs (ONNX Runtime) | STT + VAD + mic capture |
| `voice-tts` only | +2--4 MB | cpal, sherpa-rs (ONNX Runtime) | TTS + speaker output |
| `voice-stt` + `voice-tts` | +4--6 MB | cpal, sherpa-rs (shared) | Full pipeline minus wake |
| `voice-wake` | +5--7 MB | cpal, sherpa-rs, rustpotter | Includes voice-stt |
| `voice` (all) | +5--8 MB | cpal, sherpa-rs, rustpotter | Complete voice support |

**Note:** These are binary size estimates only. Model files (50--100 MB) are downloaded separately and not included in the binary.

**ONNX Runtime consideration:** `sherpa-rs` bundles ONNX Runtime statically. This is the largest contributor to binary size. If both `voice-stt` and `voice-tts` are enabled, ONNX Runtime is linked once (shared). The delta between single-feature and dual-feature is therefore small.

### 4.6 Build Matrix Verification

The following feature combinations must compile successfully:

```bash
# No voice (default)
cargo build -p clawft-plugin

# STT only
cargo build -p clawft-plugin --features voice-stt

# TTS only
cargo build -p clawft-plugin --features voice-tts

# STT + TTS (no wake)
cargo build -p clawft-plugin --features voice-stt,voice-tts

# Wake word (implies STT)
cargo build -p clawft-plugin --features voice-wake

# Full voice
cargo build -p clawft-plugin --features voice

# Full build (all features including voice)
cargo build --all-features
```

### 4.7 VP3 Exit Criteria

- [ ] Feature tree is documented with dependency relationships
- [ ] Cargo.toml feature definitions are validated (no circular dependencies)
- [ ] All 7 feature combinations in the build matrix compile (can be validated during VP1)
- [ ] `#[cfg(feature = "...")]` gating patterns are documented with examples
- [ ] Binary size delta is measured for each feature combination (from VP1 prototype)
- [ ] Default build (no voice features) binary size is unchanged from current baseline
- [ ] `voice-wake` correctly implies `voice-stt`
- [ ] CLI shows helpful error when voice commands are used without voice features enabled
- [ ] Report written to `voice-proto/reports/vp3-features.md`

---

## 5. VP4: Platform Audio Testing

### 5.1 Objective

Validate that `cpal` audio capture and playback works on all target platforms before committing to a voice sprint. Audio hardware interaction is the most platform-dependent component and the highest-risk area for build failures and runtime issues.

### 5.2 cpal Validation Matrix

| # | Platform | Audio Backend | cpal Support | Risk Level | Priority |
|---|----------|--------------|-------------|------------|----------|
| 1 | Linux x86_64 (PipeWire) | PipeWire (via ALSA compat) | Supported | Low | P0 |
| 2 | Linux x86_64 (PulseAudio) | PulseAudio (via ALSA compat) | Supported | Low | P0 |
| 3 | macOS aarch64 (Apple Silicon) | CoreAudio | Supported | Low | P0 |
| 4 | Windows x86_64 | WASAPI | Supported | Low | P1 |
| 5 | WSL2 (Linux under Windows) | PulseAudio bridge | Partial | High | P1 |

### 5.3 Test Script Specification

Build a small Rust binary (`voice-proto/src/bin/audio_test.rs`) that validates capture and playback independently.

```rust
// voice-proto/src/bin/audio_test.rs -- specification

// Subcommands:
//   audio_test devices        -- List all input + output devices
//   audio_test capture        -- Capture 5 seconds of mic audio, save to test_capture.wav
//   audio_test playback FILE  -- Play a WAV file through default output
//   audio_test loopback       -- Capture mic -> play through speaker (real-time)
//   audio_test formats        -- List supported formats for default input/output devices

// Each subcommand prints:
//   [OK] or [FAIL] with explanation
//   Device name
//   Sample rate negotiated
//   Buffer size negotiated
//   Any format conversion applied (e.g., 48kHz -> 16kHz resampling)
```

**Test procedure per platform:**

```text
1. cargo build --bin audio_test
2. ./audio_test devices
   -> Verify at least one input and one output device listed
3. ./audio_test formats
   -> Verify 16kHz mono i16 is either natively supported or resample-able
4. ./audio_test capture
   -> Speak into mic, verify test_capture.wav is non-silent
   -> Play test_capture.wav externally to verify audio quality
5. ./audio_test playback test_capture.wav
   -> Verify audio plays through speakers
6. ./audio_test loopback
   -> Verify mic input is heard through speakers (latency < 100ms perceived)
```

### 5.4 Hardware Requirements per Tier

| Tier | CPU | RAM | Audio Hardware | Voice Features Available |
|------|-----|-----|---------------|------------------------|
| **Low** | 2-core ARM64 (RPi 4, Chromebook) | 2 GB | USB mic or built-in | STT only (no TTS, no wake word). Batch mode preferred. |
| **Medium** | 4-core x86_64/ARM64 | 4 GB | Built-in mic + speakers | Full pipeline (STT + TTS). No always-on wake word daemon. |
| **Full** | 4+ core x86_64 | 8 GB | Built-in or USB mic + speakers | Full pipeline + always-on wake word detection. |

**Minimum for VP4 validation:** Medium tier (developer workstation with built-in or USB mic).

### 5.5 Known Issues and Workarounds per Platform

#### Linux (PipeWire)

| Issue | Workaround | Status |
|-------|-----------|--------|
| cpal uses ALSA backend by default, not PipeWire native | PipeWire provides ALSA compatibility layer (`pipewire-alsa`). No code changes needed. Ensure `pipewire-alsa` is installed. | Likely works out of box |
| Exclusive mode conflicts | PipeWire handles device sharing natively. No exclusive mode issues expected. | Non-issue |
| Sample rate negotiation | PipeWire resamples automatically. Request 16kHz, get 16kHz. | Non-issue |

#### Linux (PulseAudio)

| Issue | Workaround | Status |
|-------|-----------|--------|
| cpal ALSA backend may bypass PulseAudio | Ensure `pulseaudio-alsa` bridge is installed. Alternatively, use PulseAudio cpal backend if available. | Needs validation |
| Latency with PulseAudio buffering | Set PULSE_LATENCY_MSEC=30 environment variable. Or configure PulseAudio fragment size. | Needs validation |
| Mic permissions (PulseAudio access control) | User must be in `audio` group or PulseAudio must be running as user session. | Document in README |

#### macOS (CoreAudio)

| Issue | Workaround | Status |
|-------|-----------|--------|
| Microphone permission prompt | First access triggers macOS "allow microphone" dialog. App must be signed or run from Terminal. | Expected behavior |
| Aggregate device creation | Not needed for single mic input. Only relevant if user has multiple audio interfaces. | Non-issue for prototype |
| Sample rate mismatch | CoreAudio natively supports 16kHz. If device is 44.1kHz or 48kHz, cpal handles resampling. | Needs validation |

#### Windows (WASAPI)

| Issue | Workaround | Status |
|-------|-----------|--------|
| WASAPI exclusive mode | Use shared mode (default in cpal). Exclusive mode blocks other apps from using audio. | Use shared mode |
| Privacy settings block mic access | User must enable "Allow apps to access your microphone" in Windows Settings > Privacy > Microphone. | Document in README |
| Sample rate: WASAPI may only expose 48kHz | cpal can request 16kHz; if unavailable, capture at 48kHz and resample. Implement software resampler (linear interpolation or `rubato` crate). | Needs validation |

#### WSL2 (PulseAudio Bridge)

| Issue | Workaround | Status |
|-------|-----------|--------|
| No native audio device access in WSL2 | Must bridge to Windows PulseAudio server. Requires PulseAudio on Windows side. | High risk, needs validation |
| Network latency through bridge | Adds 10--50ms round-trip. May push end-to-end latency above 500ms target. | Measure in VP1 |
| Microphone access through bridge | Depends on Windows PulseAudio module-waveout and module-wavein configuration. | High risk |

### 5.6 WSL2 PulseAudio Bridge Setup

**Prerequisites:**
- Windows: Install PulseAudio for Windows (pulseaudio-win32) or use PulseAudio from MSYS2
- WSL2: Install `pulseaudio` and `alsa-plugins-pulseaudio`

**Setup steps:**

```bash
# Windows side (PowerShell, run as administrator):
# 1. Download PulseAudio for Windows
# 2. Edit config: %APPDATA%\PulseAudio\default.pa
#    Add: load-module module-native-protocol-tcp auth-ip-acl=172.16.0.0/12;192.168.0.0/16
#    Add: load-module module-waveout
#    Add: load-module module-wavein
# 3. Edit: %APPDATA%\PulseAudio\daemon.conf
#    Set: exit-idle-time = -1
# 4. Start PulseAudio: pulseaudio.exe --start

# WSL2 side:
sudo apt install pulseaudio alsa-plugins-pulseaudio

# Get Windows host IP from WSL2
WINDOWS_HOST=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}')

# Set PulseAudio server to Windows host
export PULSE_SERVER=tcp:${WINDOWS_HOST}

# Test connection
pactl info  # Should show Windows PulseAudio server details

# Make persistent (add to ~/.bashrc):
echo "export PULSE_SERVER=tcp:\$(cat /etc/resolv.conf | grep nameserver | awk '{print \$2}')" >> ~/.bashrc

# Test microphone capture:
parecord --format=s16le --rate=16000 --channels=1 test.wav
# Speak into Windows microphone, Ctrl+C to stop
# Play back:
paplay test.wav
```

**Validation checklist for WSL2:**

- [ ] `pactl info` connects to Windows PulseAudio server
- [ ] `parecord` captures audio from Windows microphone
- [ ] `paplay` plays audio through Windows speakers
- [ ] cpal (via ALSA -> PulseAudio bridge) sees audio devices
- [ ] Round-trip latency (capture -> cpal callback) < 50ms

### 5.7 VP4 Exit Criteria

- [ ] `audio_test` binary compiles on all 5 platforms
- [ ] `audio_test devices` lists at least one input + one output on each platform
- [ ] `audio_test capture` records non-silent audio on at least 3 of 5 platforms
- [ ] `audio_test playback` plays audio on at least 3 of 5 platforms
- [ ] `audio_test loopback` achieves < 100ms perceived latency on at least 3 of 5 platforms
- [ ] 16kHz mono i16 capture is confirmed working (native or via resampling) on each platform
- [ ] Known issues table is populated with actual test results (not just predictions)
- [ ] WSL2 PulseAudio bridge is documented (setup steps + validation results)
- [ ] Hardware requirements per tier are validated against actual resource usage
- [ ] Report written to `voice-proto/reports/vp4-platform.md`

---

## 6. VP5: Echo Cancellation Feasibility

### 6.1 Objective

Determine the best approach for software echo cancellation (AEC) in Talk Mode. When the agent speaks via TTS and the user is listening on speakers (not headphones), the microphone picks up the TTS output and creates a feedback loop. AEC removes the TTS output from the mic signal so that only the user's voice is transcribed.

**This is a feasibility study, not an implementation task.** The goal is to select an approach and validate it minimally, then implement fully in VS2.2.

### 6.2 Option Analysis

#### Option A: Loopback Subtraction

**Description:** Capture the TTS audio output via cpal loopback stream (OS-level audio capture of what is being played). Subtract the known TTS signal from the mic input using spectral subtraction or time-domain subtraction.

**Implementation sketch:**

```text
TTS Output ─────┬──> Speaker (cpal output)
                |
                v
        Loopback Capture (cpal loopback) ──> Known Reference Signal
                                                      |
Mic Input ──────────────────────────────> Subtraction ──> Clean Signal ──> VAD ──> STT
```

| Aspect | Assessment |
|--------|-----------|
| **Pros** | No external dependency. Pure Rust implementation possible. Works offline. Low latency (no network). |
| **Cons** | Loopback capture is platform-dependent (WASAPI supports it natively; Linux/macOS need workaround). Time alignment is critical -- even 10ms misalignment degrades quality. Does not handle room reverb well. Spectral subtraction can introduce artifacts. |
| **Complexity** | Medium-High. Implementing correct time alignment and spectral subtraction is non-trivial. |
| **Latency impact** | +5--10ms (subtraction computation) |
| **Platform support** | Windows (WASAPI loopback): native. Linux (PulseAudio monitor source): works. macOS: no native loopback, needs BlackHole/Soundflower virtual device. |

#### Option B: WebRTC AEC Crate

**Description:** Use the `webrtc-audio-processing` crate, which provides Rust bindings to Google's WebRTC audio processing library, including acoustic echo cancellation (AEC3), noise suppression, and automatic gain control.

**Implementation sketch:**

```text
TTS Output ──────────┬──> Speaker
                     |
                     v
              WebRTC AEC Module
                     ^
                     |
Mic Input ───────────┘──> Clean Signal ──> VAD ──> STT
```

| Aspect | Assessment |
|--------|-----------|
| **Pros** | Battle-tested AEC algorithm (used in Chrome, Meet, etc.). Handles reverb, non-linear echo, double-talk. Also provides noise suppression and AGC for free. Well-documented behavior. |
| **Cons** | C++ dependency (webrtc-audio-processing is a Rust wrapper around C++ code). Adds ~5 MB to binary. Build complexity (needs cmake, C++ compiler). May not compile on all targets without effort. |
| **Complexity** | Low-Medium. The crate handles the hard DSP work. Integration is straightforward: feed reference signal + mic signal, get clean signal. |
| **Latency impact** | +10--20ms (AEC processing) |
| **Platform support** | Linux: good. macOS: good. Windows: needs MSVC toolchain. WSL2: should work (no hardware dependency). |

#### Option C: Hardware / OS-Level AEC

**Description:** Rely on the operating system or audio hardware to perform echo cancellation. macOS CoreAudio provides built-in AEC when using the voice processing audio unit. Windows can provide AEC through communication mode audio. Linux has no standard OS-level AEC.

**Implementation sketch:**

```text
Mic + Speaker ──> OS Audio Stack (AEC built-in) ──> Clean Mic Signal ──> cpal ──> VAD ──> STT
```

| Aspect | Assessment |
|--------|-----------|
| **Pros** | Zero code to write. Best possible quality (OS has access to hardware timing). No additional binary size. Works with existing cpal capture. |
| **Cons** | macOS only (reliably). Windows communication mode exists but is not universally available. Linux has no OS-level AEC. Inconsistent behavior across hardware. Not controllable by our code. |
| **Complexity** | Low. Just enable the right audio device/mode. |
| **Latency impact** | +0--5ms (handled by OS, nearly transparent) |
| **Platform support** | macOS: excellent. Windows: partial (depends on audio driver). Linux: not available. WSL2: not available. |

### 6.3 Comparison Table

| Criterion | Option A: Loopback | Option B: WebRTC AEC | Option C: Hardware AEC |
|-----------|-------------------|---------------------|----------------------|
| Echo removal quality | Medium | High | High (where available) |
| Room reverb handling | Poor | Good | Good |
| Double-talk handling | Poor | Good | Good |
| Additional latency | 5--10ms | 10--20ms | 0--5ms |
| Binary size impact | 0 | +5 MB | 0 |
| Build complexity | Low | Medium (C++ dep) | None |
| Platform: Linux | Works (PA monitor) | Works | Not available |
| Platform: macOS | Needs virtual device | Works | Native |
| Platform: Windows | Native (WASAPI) | Works | Partial |
| Platform: WSL2 | Risky (bridge) | Works | Not available |
| Maintenance burden | High (custom DSP) | Low (upstream maintained) | None |
| Offline support | Yes | Yes | Yes |
| Noise suppression bonus | No | Yes (free) | Sometimes |

### 6.4 Test Methodology

**Setup (for each option):**

1. Place speaker and microphone ~50cm apart (simulates laptop usage)
2. Play known reference audio through speaker (TTS output of "The quick brown fox jumps over the lazy dog")
3. Simultaneously speak a different phrase into the microphone ("Hello, how are you today")
4. Feed mic + reference through AEC
5. Measure:
   - **Echo suppression ratio (dB):** How much TTS audio is removed from mic signal
   - **Speech quality (PESQ/MOS):** Quality of remaining user speech
   - **STT accuracy on cleaned signal:** Does STT correctly transcribe user speech (not TTS echo)?
   - **Processing latency:** Time to process one 30ms audio chunk

**Test files needed:**

| File | Content | Purpose |
|------|---------|---------|
| `tts_reference.wav` | "The quick brown fox..." (TTS output) | Known reference signal |
| `user_speech.wav` | "Hello, how are you today" (recorded separately) | Ground truth for user speech |
| `mic_capture_echo.wav` | Both TTS echo + user speech (recorded from mic during TTS playback) | Input to AEC |

**Scoring:**

| Metric | Good | Acceptable | Fail |
|--------|------|-----------|------|
| Echo suppression | > 30 dB | 20--30 dB | < 20 dB |
| STT accuracy on cleaned signal | > 90% | 80--90% | < 80% |
| Processing latency per chunk | < 5ms | 5--20ms | > 20ms |
| MOS (speech quality) | > 3.5 | 2.5--3.5 | < 2.5 |

### 6.5 Decision Criteria

Weighted scoring (1--5 per criterion, multiply by weight):

| Criterion | Weight | Why |
|-----------|--------|-----|
| Echo removal quality | 5 | Primary purpose of AEC |
| Cross-platform support | 4 | Must work on Linux + macOS minimum |
| Implementation complexity | 3 | Week 5 budget is 30h total |
| Latency impact | 3 | Must stay within 500ms end-to-end budget |
| Binary size impact | 2 | Less critical than quality |
| Maintenance burden | 2 | Long-term cost |

**Scoring template:**

| Criterion | Weight | Option A | Option B | Option C |
|-----------|--------|---------|---------|---------|
| Echo quality | 5 | _/5 | _/5 | _/5 |
| Cross-platform | 4 | _/5 | _/5 | _/5 |
| Complexity | 3 | _/5 | _/5 | _/5 |
| Latency | 3 | _/5 | _/5 | _/5 |
| Binary size | 2 | _/5 | _/5 | _/5 |
| Maintenance | 2 | _/5 | _/5 | _/5 |
| **Total** | | _/95 | _/95 | _/95 |

### 6.6 Recommendation

**Primary recommendation: Option B (WebRTC AEC)**

Rationale:
- Best quality echo cancellation across all platforms
- Includes noise suppression and AGC as bonus features
- Proven technology (Chrome, Google Meet, Discord all use WebRTC AEC)
- Acceptable binary size trade-off (+5 MB)
- Moderate latency impact (+10--20ms, within budget)
- Low maintenance (upstream Google project maintains the DSP algorithms)

**Fallback: Option C (Hardware AEC) for macOS + Option A (Loopback) for Linux**

If Option B proves too difficult to build (C++ dependency chain issues), fall back to:
- macOS: Use CoreAudio voice processing audio unit (zero effort, excellent quality)
- Linux: Implement basic loopback subtraction (acceptable quality, no external deps)
- Windows: Use WASAPI loopback subtraction (WASAPI has native loopback support)

**Degraded mode (last resort): Disable AEC, require headphones**

If no AEC option meets quality thresholds, document that Talk Mode works best with headphones and disable echo cancellation entirely. This is acceptable for v1 and avoids shipping poor-quality AEC.

### 6.7 VP5 Exit Criteria

- [ ] All 3 options are analyzed with pros/cons documented
- [ ] At least Option B (WebRTC AEC) is prototyped minimally (compile `webrtc-audio-processing` crate, process one audio buffer)
- [ ] Test methodology is documented with concrete test files and scoring criteria
- [ ] Echo suppression is measured for at least one option (even rough estimate)
- [ ] Decision is made with documented rationale and scoring
- [ ] Fallback strategy is documented
- [ ] cpal loopback capture is tested on at least one platform (needed for all AEC options)
- [ ] Report written to `voice-proto/reports/vp5-aec.md`

---

## 7. Week 0 Schedule

| Day | Tasks | Deliverables |
|-----|-------|-------------|
| **Day 1** (Mon) | VP1: Set up `voice-proto/`, write Cargo.toml, implement `audio_capture.rs`, test `cpal` on primary dev platform | Compiling prototype, audio capture works on 1 platform |
| **Day 2** (Tue) | VP1: Implement `vad.rs` + `stt.rs`, download models, get end-to-end pipeline working | Pipeline runs: mic -> VAD -> STT -> text output |
| **Day 3** (Wed) | VP1: Run benchmarks, VP4: Test on additional platforms (macOS/Windows if available, WSL2), VP2: Create test manifest + validate hosting | VP1 benchmark report, VP4 platform results, VP2 manifest validated |
| **Day 4** (Thu) | VP3: Validate feature flag builds, measure binary sizes, VP5: Prototype WebRTC AEC crate compilation and basic usage | VP3 feature matrix validated, VP5 AEC feasibility determined |
| **Day 5** (Fri) | Write all reports, fill exit criteria checklists, compile final go/no-go assessment | All 5 VP reports in `voice-proto/reports/`, go/no-go decision |

**Parallelism:** VP2 (model hosting) and VP3 (feature flags) are primarily design/documentation tasks and can be worked on in parallel with VP1 (prototype coding). VP4 (platform testing) reuses the VP1 prototype binary. VP5 (AEC) can start as soon as VP1 has basic audio capture working.

---

## 8. Combined Exit Criteria

All five VP tasks must pass before voice sprints begin. This is the final gate checklist.

### VP1: Audio Pipeline Prototype

- [ ] Standalone `voice-proto` binary compiles on Linux, macOS, Windows
- [ ] mic -> VAD -> STT -> text pipeline works end-to-end
- [ ] Latency < 500ms (speech-end to text-available)
- [ ] VAD false trigger rate < 3% on silence
- [ ] STT accuracy > 90% on English test phrases
- [ ] Benchmark report written

### VP2: Model Hosting & Download

- [ ] Manifest JSON schema defined and validated
- [ ] Cache directory layout documented
- [ ] Download + integrity check flow designed
- [ ] Offline mode documented
- [ ] Hosting validated (GitHub Releases + HuggingFace)

### VP3: Feature Flag Design

- [ ] Feature tree documented (voice -> voice-stt, voice-tts, voice-wake)
- [ ] Cargo.toml feature specs written
- [ ] All feature combinations compile
- [ ] Binary size impact measured
- [ ] Conditional compilation patterns documented

### VP4: Platform Audio Testing

- [ ] cpal validated on 3+ platforms
- [ ] 16kHz mono capture confirmed on each
- [ ] Known issues documented per platform
- [ ] WSL2 PulseAudio bridge documented
- [ ] Hardware requirements validated

### VP5: Echo Cancellation Feasibility

- [ ] 3 options analyzed with pros/cons
- [ ] At least 1 option prototyped
- [ ] Decision made with rationale
- [ ] Fallback strategy documented

### Go/No-Go Decision

| Outcome | Criteria | Action |
|---------|----------|--------|
| **Go** | All VP1--VP5 exit criteria pass | Proceed to VS1.1 (Week 1) |
| **Conditional Go** | VP1 passes, 1--2 minor VP items remain | Proceed to VS1.1, carry remaining items as VS1.1 tasks |
| **No-Go** | VP1 fails (pipeline doesn't work) or 3+ VPs fail | Revise technology choices (consider whisper-rs, piper-rs as alternatives), re-run Week 0 |

**Report location:** `voice-proto/reports/week0-summary.md`
**Decision authority:** Project lead
