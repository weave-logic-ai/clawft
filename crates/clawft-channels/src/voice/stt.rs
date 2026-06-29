//! Speech-to-text backend abstraction with start-up pre-warm (ADR-061 §2,
//! ADR-053).
//!
//! ADR-053 makes **substrate-side `clawft-service-whisper`** the canonical
//! STT path; ADR-061 adds the conversational requirement that the backend is
//! **pre-warmed at startup** so the first utterance isn't a cold load (the
//! measured "slow first turn" was cold-load, not steady-state — warm
//! parakeet is ~0.05 s). This module defines the [`SttBackend`] trait (the
//! extension point ADR-053 calls out), a model selector
//! ([`SttModel`] — parakeet English-fast / whisper-turbo multilingual), and
//! the [`SubstrateStt`] HTTP backend that talks to the whisper daemon.
//!
//! The Talk-Mode controller (task 6.7) owns one [`SttBackend`], calls
//! [`SttBackend::warm`] once at start, then [`SttBackend::transcribe`] per
//! finalized utterance.

use async_trait::async_trait;
use reqwest::multipart::{Form, Part};

use super::types::VoiceError;
use super::wav::pcm_s16le_to_wav;

/// STT model family. The substrate selects the concrete weights; this is the
/// routing hint the voice front sends ("which model to serve this turn").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SttModel {
    /// Parakeet-TDT — English, lowest latency (~0.05 s warm). Default.
    #[default]
    ParakeetEnglish,
    /// whisper-large-v3-turbo — multilingual, slightly slower.
    WhisperTurbo,
}

impl SttModel {
    /// Wire name sent to the substrate as the `model` form field.
    pub fn wire_name(self) -> &'static str {
        match self {
            SttModel::ParakeetEnglish => "parakeet-tdt-0.6b",
            SttModel::WhisperTurbo => "whisper-large-v3-turbo",
        }
    }
}

/// One finalized utterance ready to transcribe: 16 kHz mono `s16le` PCM.
#[derive(Debug, Clone)]
pub struct Utterance {
    /// Interleaved 16 kHz mono `s16le` samples.
    pub samples: Vec<i16>,
    /// Sample rate of `samples`.
    pub sample_rate: u32,
}

/// Speech-to-text backend. ADR-053's documented extension point: the
/// substrate HTTP backend ships, but an in-process backend (sherpa-rs,
/// parakeet-mlx) can implement the same trait without touching the caller.
#[async_trait]
pub trait SttBackend: Send + Sync {
    /// Pre-warm the model so the first real utterance isn't a cold load.
    /// Implementations should drive a tiny throwaway transcription through
    /// the full path. Idempotent; called once at startup.
    async fn warm(&self) -> Result<(), VoiceError>;

    /// Transcribe a finalized utterance to text (trimmed; may be empty).
    async fn transcribe(&self, utt: &Utterance) -> Result<String, VoiceError>;

    /// Which model this backend serves.
    fn model(&self) -> SttModel;
}

/// Substrate-side whisper/parakeet HTTP backend (ADR-053 canonical path).
///
/// POSTs a WAV `multipart/form-data` `file` part to
/// `{endpoint}{transcribe_path}` and reads `{"text": "..."}` back — the same
/// wire contract the 0.7.0 [`VoiceChannelAdapter`](super::channel::VoiceChannelAdapter)
/// uses, factored here as a reusable building block for Talk-Mode.
pub struct SubstrateStt {
    http: reqwest::Client,
    url: String,
    model: SttModel,
    language: String,
}

impl SubstrateStt {
    /// Build a backend pointed at a fully-qualified transcribe URL.
    pub fn new(
        url: impl Into<String>,
        model: SttModel,
        language: impl Into<String>,
        timeout_s: u64,
    ) -> Result<Self, VoiceError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_s.max(1)))
            .build()
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        Ok(Self {
            http,
            url: url.into(),
            model,
            language: language.into(),
        })
    }

    async fn post(&self, utt: &Utterance) -> Result<String, VoiceError> {
        let wav = pcm_s16le_to_wav(&utt.samples, utt.sample_rate);
        let part = Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        let mut form = Form::new()
            .part("file", part)
            .text("model", self.model.wire_name());
        if !self.language.is_empty() {
            form = form.text("language", self.language.clone());
        }
        let resp = self
            .http
            .post(&self.url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        if !status.is_success() {
            return Err(VoiceError::Server {
                status: status.as_u16(),
                body: truncate(&body, 4096),
            });
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| VoiceError::Malformed(format!("{e}: {}", truncate(&body, 256))))?;
        let text = v
            .get("text")
            .and_then(|t| t.as_str())
            .ok_or_else(|| VoiceError::Malformed("response missing `text`".into()))?
            .trim()
            .to_string();
        Ok(text)
    }
}

#[async_trait]
impl SttBackend for SubstrateStt {
    async fn warm(&self) -> Result<(), VoiceError> {
        // 100 ms of silence forces the model graph to load on the substrate
        // without producing a meaningful transcript.
        let warmup = Utterance {
            samples: vec![0i16; 1_600],
            sample_rate: 16_000,
        };
        // A warm failure is non-fatal to startup in production, but the
        // caller decides — surface it.
        self.post(&warmup).await.map(|_| ())
    }

    async fn transcribe(&self, utt: &Utterance) -> Result<String, VoiceError> {
        self.post(utt).await
    }

    fn model(&self) -> SttModel {
        self.model
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn model_wire_names() {
        assert_eq!(SttModel::default(), SttModel::ParakeetEnglish);
        assert_eq!(SttModel::ParakeetEnglish.wire_name(), "parakeet-tdt-0.6b");
        assert_eq!(SttModel::WhisperTurbo.wire_name(), "whisper-large-v3-turbo");
    }

    #[tokio::test]
    async fn warm_then_transcribe_hits_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/inference"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": " hello world "
            })))
            .mount(&server)
            .await;

        let stt = SubstrateStt::new(
            format!("{}/inference", server.uri()),
            SttModel::ParakeetEnglish,
            "en",
            5,
        )
        .unwrap();

        // Pre-warm must succeed against the live endpoint.
        stt.warm().await.unwrap();

        let utt = Utterance {
            samples: vec![1_000i16; 1_600],
            sample_rate: 16_000,
        };
        let text = stt.transcribe(&utt).await.unwrap();
        assert_eq!(text, "hello world");
        assert_eq!(stt.model(), SttModel::ParakeetEnglish);
    }

    #[tokio::test]
    async fn transcribe_5xx_surfaces_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/inference"))
            .respond_with(ResponseTemplate::new(503).set_body_string("loading"))
            .mount(&server)
            .await;
        let stt = SubstrateStt::new(
            format!("{}/inference", server.uri()),
            SttModel::WhisperTurbo,
            "auto",
            5,
        )
        .unwrap();
        let err = stt.warm().await.unwrap_err();
        assert!(matches!(err, VoiceError::Server { status: 503, .. }));
    }

    /// Live parakeet/whisper latency gate (warm transcribe < 0.2 s). Requires
    /// a running substrate whisper daemon; ignored by default.
    #[tokio::test]
    #[ignore = "requires live substrate whisper daemon (ADR-053)"]
    async fn live_warm_transcribe_under_200ms() {
        let url = std::env::var("WEFT_WHISPER_URL")
            .unwrap_or_else(|_| "http://localhost:8112/inference".into());
        let stt = SubstrateStt::new(url, SttModel::ParakeetEnglish, "en", 30).unwrap();
        stt.warm().await.unwrap();
        let utt = Utterance {
            samples: vec![0i16; 16_000],
            sample_rate: 16_000,
        };
        let t = std::time::Instant::now();
        let _ = stt.transcribe(&utt).await.unwrap();
        assert!(
            t.elapsed().as_millis() < 200,
            "warm transcribe took {:?}",
            t.elapsed()
        );
    }
}
