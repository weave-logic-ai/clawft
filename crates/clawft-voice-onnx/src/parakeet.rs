//! Native-Rust parakeet-tdt-0.6b STT (NeMo FastConformer + TDT transducer via
//! ONNX Runtime).
//!
//! Implements [`clawft_channels::voice::stt::SttBackend`] over the sherpa-onnx
//! NeMo export (`encoder` / `decoder` / `joiner` int8 ONNX + `tokens.txt`). The
//! front-end is the in-crate [`NemoMel`]; the encoder + greedy
//! Token-and-Duration Transducer decode live in [`crate::parakeet_tdt`].
//! Inference mirrors the `ort` idiom in [`crate::ecapa`].
//!
//! Weights are out-of-band (~470 MB int8 bundle, not in the repo). Construction
//! never fails and never requires the model: without the `onnx` feature or the
//! model directory, [`transcribe`](SttBackend::transcribe) returns a clear
//! `VoiceError` rather than panicking, so the default build and unit tests need
//! no weights.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clawft_channels::voice::stt::{SttBackend, SttModel, Utterance};
#[cfg(feature = "onnx")]
use clawft_channels::voice::stt::{TranscriptResult, TranscriptToken};
use clawft_channels::voice::types::VoiceError;

use crate::nemo_mel::NemoMel;
#[cfg(feature = "onnx")]
use crate::nemo_mel::SAMPLE_RATE;
#[cfg(feature = "onnx")]
use crate::parakeet_tdt::TdtEngine;

/// Default bundle directory name (the sherpa-onnx v3 int8 export).
const BUNDLE_DIR: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
/// Environment override for the model directory.
const MODEL_ENV: &str = "WEFTOS_PARAKEET_DIR";

/// Native parakeet-tdt STT backend. Build with [`ParakeetStt::new`] (auto
/// discovery) or [`ParakeetStt::with_dir`].
pub struct ParakeetStt {
    /// Front-end; only read on the inference path (gated by `onnx`).
    #[cfg_attr(not(feature = "onnx"), allow(dead_code))]
    mel: NemoMel,
    model_dir: PathBuf,
    runtime_available: bool,
    #[cfg(feature = "onnx")]
    engine: Option<TdtEngine>,
}

impl ParakeetStt {
    /// Build with auto discovery: `$WEFTOS_PARAKEET_DIR`, then
    /// `./.weftos/models/parakeet/<bundle>`, then the same under `$HOME`.
    pub fn new() -> Self {
        Self::with_dir(default_model_dir())
    }

    /// Build pointing at a bundle directory (containing `encoder.int8.onnx`,
    /// `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`). Degrades
    /// gracefully when the directory, files, or `onnx` feature are absent.
    pub fn with_dir(model_dir: impl Into<PathBuf>) -> Self {
        let model_dir = model_dir.into();
        #[cfg(feature = "onnx")]
        {
            let tokens = load_tokens(&model_dir.join("tokens.txt")).unwrap_or_default();
            let engine = TdtEngine::load(&model_dir, tokens);
            Self {
                mel: NemoMel::new(),
                model_dir,
                runtime_available: engine.is_some(),
                engine,
            }
        }
        #[cfg(not(feature = "onnx"))]
        {
            Self {
                mel: NemoMel::new(),
                model_dir,
                runtime_available: false,
            }
        }
    }

    /// Whether real ONNX inference is active (vs. the model-absent error path).
    pub fn is_runtime_available(&self) -> bool {
        self.runtime_available
    }

    /// The resolved bundle directory (may not exist).
    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// Front-end + encoder + TDT decode for 16 kHz mono PCM.
    #[cfg(feature = "onnx")]
    fn run(&self, samples_16k: &[f32]) -> Result<String, VoiceError> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| absent(&self.model_dir))?;
        let (frames, feats) = self.mel.compute(samples_16k);
        if frames == 0 {
            return Ok(String::new());
        }
        engine.decode(&feats, frames)
    }

    /// [`run`](Self::run) with per-token detail (native path). Converts the
    /// TDT decode's encoder-frame timing to milliseconds via the mel hop and
    /// the FastConformer subsampling factor.
    #[cfg(feature = "onnx")]
    fn run_detailed(&self, samples_16k: &[f32]) -> Result<TranscriptResult, VoiceError> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| absent(&self.model_dir))?;
        let (frames, feats) = self.mel.compute(samples_16k);
        if frames == 0 {
            return Ok(TranscriptResult::default());
        }
        let (text, tokens) = engine.decode_detailed(&feats, frames)?;
        let stride = frame_stride_ms();
        let tokens = tokens
            .into_iter()
            .map(|t| TranscriptToken {
                text: engine.token_text(t.id),
                start_ms: (t.t_frame as u32) * stride,
                duration_ms: (t.dur_frames as u32) * stride,
                confidence: t.conf,
            })
            .collect();
        Ok(TranscriptResult { text, tokens })
    }
}

/// Milliseconds per encoder frame: `HOP · subsampling / sample_rate`. For
/// parakeet-tdt-0.6b (160-sample hop, 8× subsampling, 16 kHz) this is 80 ms.
#[cfg(feature = "onnx")]
fn frame_stride_ms() -> u32 {
    use crate::nemo_mel::HOP;
    (HOP as u32 * crate::parakeet_tdt::SUBSAMPLING_FACTOR as u32 * 1_000) / SAMPLE_RATE
}

impl Default for ParakeetStt {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttBackend for ParakeetStt {
    async fn warm(&self) -> Result<(), VoiceError> {
        #[cfg(feature = "onnx")]
        {
            if self.runtime_available {
                // Half a second of silence drives the full graph without
                // producing a meaningful transcript.
                let _ = self.run(&vec![0.0f32; SAMPLE_RATE as usize / 2])?;
                return Ok(());
            }
        }
        Err(absent(&self.model_dir))
    }

    async fn transcribe(&self, utt: &Utterance) -> Result<String, VoiceError> {
        #[cfg(feature = "onnx")]
        {
            if self.runtime_available {
                let mono: Vec<f32> = utt.samples.iter().map(|&s| s as f32 / 32_768.0).collect();
                let samples = resample_linear(&mono, utt.sample_rate, SAMPLE_RATE);
                return self.run(&samples);
            }
        }
        #[cfg(not(feature = "onnx"))]
        let _ = utt;
        Err(absent(&self.model_dir))
    }

    async fn transcribe_detailed(
        &self,
        utt: &Utterance,
    ) -> Result<clawft_channels::voice::stt::TranscriptResult, VoiceError> {
        #[cfg(feature = "onnx")]
        {
            if self.runtime_available {
                let mono: Vec<f32> = utt.samples.iter().map(|&s| s as f32 / 32_768.0).collect();
                let samples = resample_linear(&mono, utt.sample_rate, SAMPLE_RATE);
                return self.run_detailed(&samples);
            }
        }
        #[cfg(not(feature = "onnx"))]
        let _ = utt;
        Err(absent(&self.model_dir))
    }

    fn model(&self) -> SttModel {
        SttModel::ParakeetEnglish
    }
}

/// Read `tokens.txt` (`symbol id` per line) into an id-indexed table.
#[cfg_attr(not(feature = "onnx"), allow(dead_code))]
fn load_tokens(path: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut tokens: Vec<(usize, String)> = Vec::new();
    for line in text.lines() {
        // Split on the LAST whitespace: the symbol may itself be a space.
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(pos) = line.rfind(char::is_whitespace) {
            let sym = &line[..pos];
            if let Ok(id) = line[pos + 1..].trim().parse::<usize>() {
                tokens.push((id, sym.to_string()));
            }
        }
    }
    if tokens.is_empty() {
        return None;
    }
    tokens.sort_by_key(|(id, _)| *id);
    Some(tokens.into_iter().map(|(_, s)| s).collect())
}

/// Resolve the bundle directory: env override → cwd `.weftos` → `$HOME/.weftos`.
fn default_model_dir() -> PathBuf {
    if let Ok(p) = std::env::var(MODEL_ENV) {
        return PathBuf::from(p);
    }
    let rel = PathBuf::from(".weftos/models/parakeet").join(BUNDLE_DIR);
    if rel.exists() {
        return rel;
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".weftos/models/parakeet")
            .join(BUNDLE_DIR);
    }
    rel
}

fn absent(dir: &Path) -> VoiceError {
    VoiceError::Audio(format!(
        "parakeet ONNX model unavailable (looked in {}); build with --features onnx \
         and place the sherpa-onnx bundle there or set {MODEL_ENV}",
        dir.display()
    ))
}

/// Linear-interpolation resampler to 16 kHz (fixtures are already 16 kHz; this
/// only engages for off-rate input).
#[cfg(feature = "onnx")]
fn resample_linear(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = ((samples.len() as f64) * ratio).round() as usize;
    (0..out_len)
        .map(|i| {
            let src = i as f64 / ratio;
            let i0 = src.floor() as usize;
            let frac = (src - i0 as f64) as f32;
            let a = samples.get(i0).copied().unwrap_or(0.0);
            let b = samples.get(i0 + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn transcribe_without_model_errors_gracefully() {
        let stt = ParakeetStt::with_dir("/nonexistent/parakeet");
        assert!(!stt.is_runtime_available());
        let utt = Utterance {
            samples: vec![0i16; 16_000],
            sample_rate: 16_000,
        };
        let err = stt.transcribe(&utt).await.unwrap_err();
        assert!(matches!(err, VoiceError::Audio(_)));
        assert!(err.to_string().contains("parakeet"));
    }

    #[tokio::test]
    async fn warm_without_model_errors() {
        let stt = ParakeetStt::with_dir("/nonexistent/parakeet");
        assert!(stt.warm().await.is_err());
        assert_eq!(stt.model(), SttModel::ParakeetEnglish);
    }

    #[test]
    fn load_tokens_parses_id_indexed_table() {
        let dir = std::env::temp_dir().join("clawft_parakeet_toktest");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("tokens.txt");
        // Out-of-order + a space symbol to exercise rfind-on-last-whitespace.
        std::fs::write(&f, "b 1\n\u{2581}a 0\n<blk> 2\n").unwrap();
        let toks = load_tokens(&f).unwrap();
        assert_eq!(
            toks,
            vec!["\u{2581}a".to_string(), "b".into(), "<blk>".into()]
        );
    }

    #[test]
    fn default_dir_honors_env() {
        // SAFETY: single-threaded test; restore afterwards.
        unsafe { std::env::set_var(MODEL_ENV, "/tmp/custom-parakeet") };
        assert_eq!(default_model_dir(), PathBuf::from("/tmp/custom-parakeet"));
        unsafe { std::env::remove_var(MODEL_ENV) };
    }
}
