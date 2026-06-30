//! Kokoro fast-layer TTS (ADR-062 Phase 4.3) — native ONNX, for the Speculative
//! ack. **No Python.**
//!
//! The Speculative branch needs an immediate, low-TTFA spoken ack while the slow
//! Orpheus answer renders. Kokoro is a small, high-quality, cross-platform ONNX
//! TTS (`thewh1teagle/kokoro-onnx` / sherpa-onnx export) that runs under `ort`.
//! It is the **preset** layer, so paralinguistic markup is scrubbed
//! ([`scrub_tags`]) — performing `<laugh>` is the slow expressive engine's job.
//! Chatterbox / Piper are future backends behind this same
//! [`TtsEngine`](clawft_channels::voice::tts::TtsEngine) once an ONNX export
//! exists; this engine does not block on them.
//!
//! Inference mirrors the `ort` idiom in `clawft-voice-onnx`: a `tokens.txt`
//! front-end (loaded if present), session build, positional tensor IO, and a
//! model-absent graceful fallback. Real Kokoro uses phoneme G2P (espeak-ng); the
//! pure-Rust G2P front-end is out of scope here, so the model-free tests cover
//! the tokenizer + tier + absent-model path and the real decode is
//! `#[ignore]`-gated (no audio is ever fabricated).

use std::path::{Path, PathBuf};
#[cfg(feature = "onnx")]
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use clawft_channels::voice::tts::{TtsChunk, TtsEngine, TtsTier};
#[cfg(feature = "onnx")]
use clawft_channels::voice::tts::{scrub_tags, split_sentences};
use clawft_channels::voice::types::VoiceError;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Kokoro output sample rate.
pub const KOKORO_SAMPLE_RATE: u32 = 24_000;
/// Kokoro style-vector dimensionality (per-voice embedding).
pub const STYLE_DIM: usize = 256;

/// Default bundle directory name under `.weftos/models/kokoro/`.
const BUNDLE_DIR: &str = "kokoro";
/// Default model filename inside the bundle.
#[cfg(feature = "onnx")]
const MODEL_FILE: &str = "kokoro.onnx";
/// Environment override for the bundle directory.
const MODEL_ENV: &str = "WEFTOS_KOKORO_DIR";

/// Native Kokoro fast-layer engine. Build with [`KokoroTts::new`] (auto
/// discovery) or [`KokoroTts::with_dir`]. Degrades gracefully: an absent model,
/// tokens file, or `onnx` feature leaves the runtime unavailable and
/// `synthesize_stream` returns a clear `VoiceError`.
pub struct KokoroTts {
    model_dir: PathBuf,
    /// `char → token id` (SentencePiece/IPA symbol table, loaded if present).
    tokens: Vec<(String, i64)>,
    runtime_available: bool,
    #[cfg(feature = "onnx")]
    session: Option<Arc<Mutex<ort::session::Session>>>,
}

impl KokoroTts {
    /// Build with auto discovery: `$WEFTOS_KOKORO_DIR`, then
    /// `./.weftos/models/kokoro`, then the same under `$HOME`.
    pub fn new() -> Self {
        Self::with_dir(default_model_dir())
    }

    /// Build pointing at a bundle directory (containing `kokoro.onnx` and
    /// `tokens.txt`). Never fails; missing artifacts route through the
    /// model-absent error path.
    pub fn with_dir(model_dir: impl Into<PathBuf>) -> Self {
        let model_dir = model_dir.into();
        let tokens = load_tokens(&model_dir.join("tokens.txt")).unwrap_or_default();
        #[cfg(feature = "onnx")]
        let session = Self::try_load_session(&model_dir.join(MODEL_FILE));
        #[cfg(feature = "onnx")]
        let runtime_available = session.is_some();
        #[cfg(not(feature = "onnx"))]
        let runtime_available = false;

        Self {
            model_dir,
            tokens,
            runtime_available,
            #[cfg(feature = "onnx")]
            session,
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

    /// Map text → Kokoro token ids using the loaded symbol table (char-level).
    /// Unknown chars are skipped. Real Kokoro phonemizes first (espeak-ng); this
    /// char-level path is the deterministic, model-free seam the tests exercise
    /// and the live decode refines.
    pub fn text_to_ids(&self, text: &str) -> Vec<i64> {
        let mut ids = Vec::new();
        for ch in text.chars() {
            let key = ch.to_string();
            if let Some((_, id)) = self.tokens.iter().find(|(s, _)| *s == key) {
                ids.push(*id);
            }
        }
        ids
    }

    #[cfg(feature = "onnx")]
    fn try_load_session(model_path: &Path) -> Option<Arc<Mutex<ort::session::Session>>> {
        if !model_path.exists() {
            tracing::debug!("Kokoro ONNX model not found at {}", model_path.display());
            return None;
        }
        match ort::session::Session::builder().and_then(|mut b| b.commit_from_file(model_path)) {
            Ok(session) => {
                tracing::info!("Kokoro ONNX session loaded from {}", model_path.display());
                Some(Arc::new(Mutex::new(session)))
            }
            Err(e) => {
                tracing::warn!("failed to load Kokoro ONNX session: {e}");
                None
            }
        }
    }

    /// Tokenize → ONNX → 24 kHz `s16le` PCM for one sentence. Inputs are bound
    /// positionally as `(tokens, style, speed)` (the sherpa-onnx Kokoro export
    /// order); the style vector is read from `style.bin` (raw `f32` LE) or, if
    /// absent, left zero (a real voice requires the style file — documented).
    #[cfg(feature = "onnx")]
    fn run(&self, sentence: &str) -> Result<Vec<i16>, VoiceError> {
        use ort::value::Tensor;

        let session = self
            .session
            .as_ref()
            .ok_or_else(|| model_absent_error(&self.model_dir))?;
        let ids = self.text_to_ids(sentence);
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let tokens_t = Tensor::from_array((vec![1_i64, ids.len() as i64], ids))
            .map_err(|e| VoiceError::Audio(format!("Kokoro tokens tensor: {e}")))?;
        let style = load_style(&self.model_dir.join("style.bin"));
        let style_t = Tensor::from_array((vec![1_i64, STYLE_DIM as i64], style))
            .map_err(|e| VoiceError::Audio(format!("Kokoro style tensor: {e}")))?;
        let speed_t = Tensor::from_array((vec![1_i64], vec![1.0_f32]))
            .map_err(|e| VoiceError::Audio(format!("Kokoro speed tensor: {e}")))?;

        let mut guard = session
            .lock()
            .map_err(|_| VoiceError::Audio("Kokoro session mutex poisoned".into()))?;
        let outputs = guard
            .run(ort::inputs![tokens_t, style_t, speed_t])
            .map_err(|e| VoiceError::Audio(format!("Kokoro inference: {e}")))?;
        let output = outputs
            .iter()
            .next()
            .map(|(_, v)| v)
            .ok_or_else(|| VoiceError::Audio("Kokoro: no output tensor".into()))?;
        let (_, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::Audio(format!("Kokoro extract: {e}")))?;
        Ok(data
            .iter()
            .map(|&x| (x.clamp(-1.0, 1.0) * 32_767.0).round() as i16)
            .collect())
    }
}

impl Default for KokoroTts {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TtsEngine for KokoroTts {
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        #[cfg(feature = "onnx")]
        {
            if !self.runtime_available {
                return Err(model_absent_error(&self.model_dir));
            }
            // Preset layer: scrub markup so no tag reaches audio, one chunk per
            // sentence for a low-TTFA gap-free ack.
            for sentence in split_sentences(&scrub_tags(text)) {
                if cancel.is_cancelled() {
                    break;
                }
                let pcm = self.run(&sentence)?;
                if pcm.is_empty() {
                    continue;
                }
                let chunk = TtsChunk {
                    samples: pcm,
                    sample_rate: KOKORO_SAMPLE_RATE,
                };
                if cancel.is_cancelled() || tx.send(chunk).await.is_err() {
                    break;
                }
            }
            return Ok(());
        }
        #[cfg(not(feature = "onnx"))]
        {
            let _ = (text, tx, cancel);
            Err(model_absent_error(&self.model_dir))
        }
    }

    fn tier(&self) -> TtsTier {
        TtsTier::Fast
    }
}

/// Read `tokens.txt` (`symbol id` per line) into a symbol→id table.
fn load_tokens(path: &Path) -> Option<Vec<(String, i64)>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut tokens = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(pos) = line.rfind(char::is_whitespace)
            && let Ok(id) = line[pos + 1..].trim().parse::<i64>()
        {
            tokens.push((line[..pos].to_string(), id));
        }
    }
    (!tokens.is_empty()).then_some(tokens)
}

/// Load a `STYLE_DIM`-length `f32` LE style vector from `path`, or zeros.
#[cfg(feature = "onnx")]
fn load_style(path: &Path) -> Vec<f32> {
    if let Ok(bytes) = std::fs::read(path) {
        let mut v: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .take(STYLE_DIM)
            .collect();
        v.resize(STYLE_DIM, 0.0);
        v
    } else {
        vec![0.0; STYLE_DIM]
    }
}

/// Resolve the bundle directory: env override → cwd `.weftos` → `$HOME/.weftos`.
fn default_model_dir() -> PathBuf {
    if let Ok(p) = std::env::var(MODEL_ENV) {
        return PathBuf::from(p);
    }
    let rel = PathBuf::from(".weftos/models").join(BUNDLE_DIR);
    if rel.exists() {
        return rel;
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".weftos/models").join(BUNDLE_DIR);
    }
    rel
}

/// Uniform "model unavailable" error (feature off or files missing).
fn model_absent_error(dir: &Path) -> VoiceError {
    VoiceError::Audio(format!(
        "Kokoro ONNX model unavailable (looked in {}); build with --features onnx \
         and place the kokoro export there or set {MODEL_ENV}",
        dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_never_requires_model() {
        let k = KokoroTts::with_dir("/nonexistent/kokoro");
        assert!(!k.is_runtime_available());
        assert_eq!(k.tier(), TtsTier::Fast);
        assert_eq!(k.model_dir(), Path::new("/nonexistent/kokoro"));
    }

    #[tokio::test]
    async fn synthesize_without_model_errors_gracefully() {
        let k = KokoroTts::with_dir("/nonexistent/kokoro");
        let (tx, _rx) = mpsc::channel::<TtsChunk>(4);
        let err = k
            .synthesize_stream("one sec.", tx, CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(err, VoiceError::Audio(_)));
        assert!(err.to_string().contains("Kokoro"));
    }

    #[test]
    fn tokenizer_maps_known_symbols_and_skips_unknown() {
        let dir = std::env::temp_dir().join("clawft_kokoro_toktest");
        std::fs::create_dir_all(&dir).unwrap();
        // A tiny char-level table: 'h'→1, 'i'→2.
        std::fs::write(dir.join("tokens.txt"), "h 1\ni 2\n").unwrap();
        let k = KokoroTts::with_dir(&dir);
        // "hi!" → [1, 2] ('!' unknown, skipped).
        assert_eq!(k.text_to_ids("hi!"), vec![1, 2]);
        assert!(k.text_to_ids("zzz").is_empty());
    }

    #[test]
    fn default_dir_honors_env() {
        // SAFETY: single-threaded test; restore afterwards.
        unsafe { std::env::set_var(MODEL_ENV, "/tmp/custom-kokoro") };
        assert_eq!(default_model_dir(), PathBuf::from("/tmp/custom-kokoro"));
        unsafe { std::env::remove_var(MODEL_ENV) };
    }

    #[tokio::test]
    #[ignore = "requires a Kokoro ONNX bundle in .weftos/models/kokoro + --features onnx"]
    async fn live_kokoro_synthesis() {
        let k = KokoroTts::new();
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(16);
        let h = tokio::spawn(async move {
            k.synthesize_stream("One sec.", tx, CancellationToken::new())
                .await
        });
        let mut total = 0;
        while let Some(c) = rx.recv().await {
            total += c.samples.len();
            assert_eq!(c.sample_rate, KOKORO_SAMPLE_RATE);
        }
        h.await.unwrap().unwrap();
        assert!(total > 0, "live Kokoro produced no audio");
    }
}
