//! Live STT/TTS backends for `voice_listen` / `voice_speak` (WEFT-214).
//!
//! Targets the **live** voice stack ([`clawft_channels::voice`]), not the
//! deprecated `clawft-plugin` scaffold (WEFT-671). Local substrate HTTP is
//! primary; optional OpenAI cloud clients provide fallback when local fails
//! or STT confidence is below threshold.
//!
//! All I/O is behind traits so unit tests inject mocks without hardware or
//! network.

use std::env;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use clawft_channels::voice::types::{
    DEFAULT_SYNTHESIZE_PATH, DEFAULT_TRANSCRIBE_PATH, DEFAULT_TTS_ENDPOINT, DEFAULT_WHISPER_ENDPOINT,
};
use clawft_channels::voice::wav::{pcm_s16le_to_wav, wav_to_pcm_s16le};
use clawft_channels::voice::{
    SttBackend, SttModel, SubstrateStt, SubstrateTts, TtsChunk, TtsEngine, TtsTier, Utterance,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Default confidence for text-only substrate STT (no per-token scores).
const DEFAULT_LOCAL_CONFIDENCE: f32 = 0.85;
/// OpenAI Whisper does not return confidence; treat successful cloud as high.
const DEFAULT_CLOUD_CONFIDENCE: f32 = 0.95;
/// Below this, local STT tries cloud when configured.
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.60;

// ── Shared result types ─────────────────────────────────────────────────────

/// Mono PCM buffer (typically 16 kHz s16le).
#[derive(Debug, Clone)]
pub struct PcmAudio {
    /// Interleaved mono samples.
    pub samples: Vec<i16>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
}

impl PcmAudio {
    /// Duration of the buffer in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.samples.is_empty() {
            return 0;
        }
        (self.samples.len() as u64 * 1000) / u64::from(self.sample_rate)
    }
}

/// STT result returned to the tool layer.
#[derive(Debug, Clone)]
pub struct Transcript {
    /// Transcribed text (trimmed; may be empty).
    pub text: String,
    /// Aggregate confidence in \[0.0, 1.0\].
    pub confidence: f32,
    /// Language hint or detected code.
    pub language: String,
    /// Audio duration used for transcription.
    pub duration_ms: u64,
    /// `"local"` or `"cloud:<provider>"`.
    pub source: String,
}

/// TTS synthesis result before/after playback.
#[derive(Debug, Clone)]
pub struct Synthesis {
    /// Rendered PCM samples.
    pub samples: Vec<i16>,
    /// Sample rate of `samples`.
    pub sample_rate: u32,
    /// Estimated audio duration.
    pub duration_ms: u64,
    /// `"local"` or `"cloud:<provider>"`.
    pub source: String,
}

// ── Traits (injectable) ─────────────────────────────────────────────────────

/// Microphone / utterance capture seam.
#[async_trait]
pub trait MicCapture: Send + Sync {
    /// Capture one utterance (or timeout), returning PCM.
    async fn capture(&self, timeout_seconds: f64) -> Result<PcmAudio, String>;
}

/// Speaker playback seam.
#[async_trait]
pub trait SpeakerPlayback: Send + Sync {
    /// Play PCM and wait until drained (or error).
    async fn play(&self, samples: &[i16], sample_rate: u32) -> Result<(), String>;
}

/// Speech-to-text seam (local or cloud).
#[async_trait]
pub trait SttService: Send + Sync {
    /// Transcribe PCM to text + confidence.
    async fn transcribe(&self, pcm: &PcmAudio, language: &str) -> Result<Transcript, String>;
}

/// Text-to-speech seam (local or cloud).
#[async_trait]
pub trait TtsService: Send + Sync {
    /// Synthesize `text` to PCM. `voice` / `speed` are hints (cloud may use them).
    async fn synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
    ) -> Result<Synthesis, String>;
}

// ── Capture / playback helpers ──────────────────────────────────────────────

/// Capture that always fails — used when no mic is wired (headless / CI).
pub struct UnavailableMicCapture {
    reason: String,
}

impl UnavailableMicCapture {
    /// Build with a human-readable reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl MicCapture for UnavailableMicCapture {
    async fn capture(&self, _timeout_seconds: f64) -> Result<PcmAudio, String> {
        Err(self.reason.clone())
    }
}

/// In-memory capture for tests: returns fixed PCM once.
pub struct FixedMicCapture {
    pcm: PcmAudio,
}

impl FixedMicCapture {
    /// Build a capture that always returns `pcm`.
    pub fn new(pcm: PcmAudio) -> Self {
        Self { pcm }
    }
}

#[async_trait]
impl MicCapture for FixedMicCapture {
    async fn capture(&self, _timeout_seconds: f64) -> Result<PcmAudio, String> {
        Ok(self.pcm.clone())
    }
}

/// Playback sink that records the last played buffer (tests) or discards.
#[derive(Default)]
pub struct RecordingPlayback {
    /// Last played samples (if any).
    pub last: std::sync::Mutex<Option<PcmAudio>>,
    /// When true, `play` returns an error.
    pub fail: bool,
}

impl RecordingPlayback {
    /// Successful recording sink.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SpeakerPlayback for RecordingPlayback {
    async fn play(&self, samples: &[i16], sample_rate: u32) -> Result<(), String> {
        if self.fail {
            return Err("playback failed (mock)".into());
        }
        let mut guard = self
            .last
            .lock()
            .map_err(|e| format!("playback lock: {e}"))?;
        *guard = Some(PcmAudio {
            samples: samples.to_vec(),
            sample_rate,
        });
        Ok(())
    }
}

/// No-op playback: synthesis still succeeds; tool reports `synthesized`.
pub struct NoopPlayback;

#[async_trait]
impl SpeakerPlayback for NoopPlayback {
    async fn play(&self, _samples: &[i16], _sample_rate: u32) -> Result<(), String> {
        Ok(())
    }
}

// ── Local substrate STT ─────────────────────────────────────────────────────

/// Substrate whisper/parakeet STT via [`SubstrateStt`].
pub struct LocalSubstrateStt {
    url: String,
    model: SttModel,
    timeout_s: u64,
}

impl LocalSubstrateStt {
    /// Point at a fully-qualified transcribe URL (e.g. `http://localhost:8112/inference`).
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            model: SttModel::ParakeetEnglish,
            timeout_s: 30,
        }
    }

    /// Override model routing hint.
    pub fn with_model(mut self, model: SttModel) -> Self {
        self.model = model;
        self
    }
}

#[async_trait]
impl SttService for LocalSubstrateStt {
    async fn transcribe(&self, pcm: &PcmAudio, language: &str) -> Result<Transcript, String> {
        let stt = SubstrateStt::new(
            self.url.clone(),
            self.model,
            language.to_string(),
            self.timeout_s,
        )
        .map_err(|e| e.to_string())?;
        let utt = Utterance {
            samples: pcm.samples.clone(),
            sample_rate: pcm.sample_rate,
        };
        let detailed = stt
            .transcribe_detailed(&utt)
            .await
            .map_err(|e| e.to_string())?;
        let confidence = if detailed.tokens.is_empty() {
            if detailed.text.is_empty() {
                0.0
            } else {
                DEFAULT_LOCAL_CONFIDENCE
            }
        } else {
            let sum: f32 = detailed.tokens.iter().map(|t| t.confidence).sum();
            (sum / detailed.tokens.len() as f32).clamp(0.0, 1.0)
        };
        Ok(Transcript {
            text: detailed.text,
            confidence,
            language: language.to_string(),
            duration_ms: pcm.duration_ms(),
            source: "local".into(),
        })
    }
}

// ── Cloud STT (OpenAI Whisper) ──────────────────────────────────────────────

/// OpenAI Whisper HTTP STT (cloud fallback). Lives in tools so we do **not**
/// depend on deprecated plugin `CloudSttProvider`.
pub struct CloudWhisperStt {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl CloudWhisperStt {
    /// Build from API key; optional custom base (tests / Azure OpenAI).
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "whisper-1".into(),
            base_url: "https://api.openai.com/v1".into(),
            client: reqwest::Client::new(),
        }
    }

    /// Override API base URL (wiremock tests).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = base.into();
        self
    }

    /// Override model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl SttService for CloudWhisperStt {
    async fn transcribe(&self, pcm: &PcmAudio, language: &str) -> Result<Transcript, String> {
        let wav = pcm_s16le_to_wav(&pcm.samples, pcm.sample_rate);
        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| format!("MIME error: {e}"))?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json");
        if !language.is_empty() {
            form = form.text("language", language.to_string());
        }
        let url = format!("{}/audio/transcriptions", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Whisper request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Whisper API {status}: {body}"));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Whisper parse error: {e}"))?;
        let text = body["text"].as_str().unwrap_or("").trim().to_string();
        let confidence = if text.is_empty() {
            0.0
        } else {
            DEFAULT_CLOUD_CONFIDENCE
        };
        let lang = body["language"]
            .as_str()
            .unwrap_or(if language.is_empty() { "en" } else { language })
            .to_string();
        let duration_ms = body["duration"]
            .as_f64()
            .map(|d| (d * 1000.0) as u64)
            .unwrap_or_else(|| pcm.duration_ms());
        Ok(Transcript {
            text,
            confidence,
            language: lang,
            duration_ms,
            source: "cloud:openai-whisper".into(),
        })
    }
}

// ── Local substrate TTS ─────────────────────────────────────────────────────

/// Substrate TTS via [`SubstrateTts`] (collect full stream into one buffer).
pub struct LocalSubstrateTts {
    url: String,
    timeout_s: u64,
}

impl LocalSubstrateTts {
    /// Fully-qualified synthesize URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            timeout_s: 30,
        }
    }
}

#[async_trait]
impl TtsService for LocalSubstrateTts {
    async fn synthesize(
        &self,
        text: &str,
        _voice: &str,
        _speed: f64,
    ) -> Result<Synthesis, String> {
        let engine = SubstrateTts::new(self.url.clone(), TtsTier::Fast, true, self.timeout_s)
            .map_err(|e| e.to_string())?;
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(8);
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let text_owned = text.to_string();
        let join = tokio::spawn(async move {
            engine
                .synthesize_stream(&text_owned, tx, cancel_clone)
                .await
                .map_err(|e| e.to_string())
        });
        let mut samples = Vec::new();
        let mut sample_rate = 16_000u32;
        while let Some(chunk) = rx.recv().await {
            sample_rate = chunk.sample_rate;
            samples.extend_from_slice(&chunk.samples);
        }
        join.await.map_err(|e| format!("tts join: {e}"))??;
        if samples.is_empty() {
            return Err("local TTS returned empty audio".into());
        }
        let duration_ms = (samples.len() as u64 * 1000) / u64::from(sample_rate.max(1));
        Ok(Synthesis {
            samples,
            sample_rate,
            duration_ms,
            source: "local".into(),
        })
    }
}

// ── Cloud TTS (OpenAI) ──────────────────────────────────────────────────────

/// OpenAI TTS HTTP client returning WAV PCM for playback.
pub struct CloudOpenAiTts {
    api_key: String,
    model: String,
    base_url: String,
    default_voice: String,
    client: reqwest::Client,
}

impl CloudOpenAiTts {
    /// Build from API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "tts-1".into(),
            base_url: "https://api.openai.com/v1".into(),
            default_voice: "alloy".into(),
            client: reqwest::Client::new(),
        }
    }

    /// Override API base (wiremock).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = base.into();
        self
    }

    /// Override default voice when tool passes empty voice.
    pub fn with_default_voice(mut self, voice: impl Into<String>) -> Self {
        self.default_voice = voice.into();
        self
    }
}

#[async_trait]
impl TtsService for CloudOpenAiTts {
    async fn synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
    ) -> Result<Synthesis, String> {
        let voice = if voice.is_empty() {
            self.default_voice.as_str()
        } else {
            voice
        };
        let speed = speed.clamp(0.25, 4.0);
        let url = format!("{}/audio/speech", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": voice,
            "speed": speed,
            "response_format": "wav",
        });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("OpenAI TTS request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI TTS {status}: {err_body}"));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("TTS body read: {e}"))?;
        let (samples, sample_rate) =
            wav_to_pcm_s16le(&bytes).map_err(|e| format!("TTS wav decode: {e}"))?;
        if samples.is_empty() {
            return Err("cloud TTS returned empty audio".into());
        }
        let duration_ms = (samples.len() as u64 * 1000) / u64::from(sample_rate.max(1));
        Ok(Synthesis {
            samples,
            sample_rate,
            duration_ms,
            source: "cloud:openai-tts".into(),
        })
    }
}

// ── Fallback chains ─────────────────────────────────────────────────────────

/// Local-first STT with optional cloud fallback on error / low confidence.
pub struct FallbackStt {
    local: Arc<dyn SttService>,
    cloud: Option<Arc<dyn SttService>>,
    confidence_threshold: f32,
}

impl FallbackStt {
    /// Local-only chain.
    pub fn new(local: Arc<dyn SttService>) -> Self {
        Self {
            local,
            cloud: None,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        }
    }

    /// Add cloud fallback.
    pub fn with_cloud(mut self, cloud: Arc<dyn SttService>) -> Self {
        self.cloud = Some(cloud);
        self
    }

    /// Override confidence threshold (default 0.60).
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }
}

#[async_trait]
impl SttService for FallbackStt {
    async fn transcribe(&self, pcm: &PcmAudio, language: &str) -> Result<Transcript, String> {
        match self.local.transcribe(pcm, language).await {
            Ok(local) if local.confidence >= self.confidence_threshold => {
                debug!(source = %local.source, conf = local.confidence, "STT local accepted");
                Ok(local)
            }
            Ok(low) => {
                if let Some(cloud) = &self.cloud {
                    info!(
                        conf = low.confidence,
                        threshold = self.confidence_threshold,
                        "STT local low confidence — trying cloud"
                    );
                    match cloud.transcribe(pcm, language).await {
                        Ok(cloud_res) if cloud_res.confidence > low.confidence => {
                            info!(source = %cloud_res.source, "STT cloud fallback won");
                            Ok(cloud_res)
                        }
                        Ok(_) => Ok(low),
                        Err(e) => {
                            warn!(error = %e, "STT cloud fallback failed; keeping local");
                            Ok(low)
                        }
                    }
                } else {
                    Ok(low)
                }
            }
            Err(local_err) => {
                if let Some(cloud) = &self.cloud {
                    warn!(error = %local_err, "STT local failed — trying cloud");
                    let cloud_res = cloud.transcribe(pcm, language).await?;
                    info!(source = %cloud_res.source, "STT cloud fallback after local error");
                    Ok(cloud_res)
                } else {
                    Err(local_err)
                }
            }
        }
    }
}

/// Local-first TTS with optional cloud fallback on error.
pub struct FallbackTts {
    local: Arc<dyn TtsService>,
    cloud: Option<Arc<dyn TtsService>>,
}

impl FallbackTts {
    /// Local-only chain.
    pub fn new(local: Arc<dyn TtsService>) -> Self {
        Self { local, cloud: None }
    }

    /// Add cloud fallback.
    pub fn with_cloud(mut self, cloud: Arc<dyn TtsService>) -> Self {
        self.cloud = Some(cloud);
        self
    }
}

#[async_trait]
impl TtsService for FallbackTts {
    async fn synthesize(
        &self,
        text: &str,
        voice: &str,
        speed: f64,
    ) -> Result<Synthesis, String> {
        match self.local.synthesize(text, voice, speed).await {
            Ok(s) => {
                debug!(source = %s.source, "TTS local ok");
                Ok(s)
            }
            Err(local_err) => {
                if let Some(cloud) = &self.cloud {
                    warn!(error = %local_err, "TTS local failed — trying cloud");
                    let s = cloud.synthesize(text, voice, speed).await?;
                    info!(source = %s.source, "TTS cloud fallback");
                    Ok(s)
                } else {
                    Err(local_err)
                }
            }
        }
    }
}

// ── Default wiring from env ─────────────────────────────────────────────────

/// Resolve substrate STT URL from env or live defaults.
pub fn default_stt_url() -> String {
    env::var("WEFT_WHISPER_URL")
        .unwrap_or_else(|_| format!("{DEFAULT_WHISPER_ENDPOINT}{DEFAULT_TRANSCRIBE_PATH}"))
}

/// Resolve substrate TTS URL from env or live defaults.
pub fn default_tts_url() -> String {
    env::var("WEFT_TTS_URL")
        .unwrap_or_else(|_| format!("{DEFAULT_TTS_ENDPOINT}{DEFAULT_SYNTHESIZE_PATH}"))
}

/// Optional OpenAI key for cloud fallback.
pub fn openai_api_key() -> Option<String> {
    env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
}

/// Build the production STT chain (substrate local + optional OpenAI cloud).
pub fn build_default_stt() -> Arc<dyn SttService> {
    let local: Arc<dyn SttService> = Arc::new(LocalSubstrateStt::new(default_stt_url()));
    let mut chain = FallbackStt::new(local);
    if let Some(key) = openai_api_key() {
        chain = chain.with_cloud(Arc::new(CloudWhisperStt::new(key)));
    }
    Arc::new(chain)
}

/// Build the production TTS chain (substrate local + optional OpenAI cloud).
pub fn build_default_tts() -> Arc<dyn TtsService> {
    let local: Arc<dyn TtsService> = Arc::new(LocalSubstrateTts::new(default_tts_url()));
    let mut chain = FallbackTts::new(local);
    if let Some(key) = openai_api_key() {
        chain = chain.with_cloud(Arc::new(CloudOpenAiTts::new(key)));
    }
    Arc::new(chain)
}

/// Default mic: unavailable until a real `MicCapture` is injected (no cpal in tools).
pub fn build_default_capture() -> Arc<dyn MicCapture> {
    Arc::new(UnavailableMicCapture::new(
        "microphone capture not configured; inject MicCapture (or enable voice-real-audio path)",
    ))
}

/// Default playback: no-op (synthesis still runs; status = synthesized).
pub fn build_default_playback() -> Arc<dyn SpeakerPlayback> {
    Arc::new(NoopPlayback)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockStt {
        text: String,
        confidence: f32,
        fail: bool,
        source: String,
    }

    #[async_trait]
    impl SttService for MockStt {
        async fn transcribe(
            &self,
            pcm: &PcmAudio,
            language: &str,
        ) -> Result<Transcript, String> {
            if self.fail {
                return Err("mock stt fail".into());
            }
            Ok(Transcript {
                text: self.text.clone(),
                confidence: self.confidence,
                language: language.to_string(),
                duration_ms: pcm.duration_ms(),
                source: self.source.clone(),
            })
        }
    }

    struct MockTts {
        fail: bool,
        source: String,
    }

    #[async_trait]
    impl TtsService for MockTts {
        async fn synthesize(
            &self,
            _text: &str,
            _voice: &str,
            _speed: f64,
        ) -> Result<Synthesis, String> {
            if self.fail {
                return Err("mock tts fail".into());
            }
            Ok(Synthesis {
                samples: vec![100, -100, 200],
                sample_rate: 16_000,
                duration_ms: 1,
                source: self.source.clone(),
            })
        }
    }

    fn pcm_ms(ms: u32) -> PcmAudio {
        let n = 16 * ms; // 16 kHz
        PcmAudio {
            samples: vec![500i16; n as usize],
            sample_rate: 16_000,
        }
    }

    #[tokio::test]
    async fn fallback_stt_accepts_high_confidence_local() {
        let local = Arc::new(MockStt {
            text: "hello".into(),
            confidence: 0.9,
            fail: false,
            source: "local".into(),
        });
        let cloud = Arc::new(MockStt {
            text: "cloud".into(),
            confidence: 0.99,
            fail: false,
            source: "cloud:x".into(),
        });
        let stt = FallbackStt::new(local).with_cloud(cloud);
        let r = stt.transcribe(&pcm_ms(100), "en").await.unwrap();
        assert_eq!(r.text, "hello");
        assert_eq!(r.source, "local");
    }

    #[tokio::test]
    async fn fallback_stt_uses_cloud_on_low_confidence() {
        let local = Arc::new(MockStt {
            text: "maybe".into(),
            confidence: 0.2,
            fail: false,
            source: "local".into(),
        });
        let cloud = Arc::new(MockStt {
            text: "certain".into(),
            confidence: 0.95,
            fail: false,
            source: "cloud:openai-whisper".into(),
        });
        let stt = FallbackStt::new(local).with_cloud(cloud);
        let r = stt.transcribe(&pcm_ms(50), "en").await.unwrap();
        assert_eq!(r.text, "certain");
        assert_eq!(r.source, "cloud:openai-whisper");
    }

    #[tokio::test]
    async fn fallback_stt_cloud_on_local_error() {
        let local = Arc::new(MockStt {
            text: String::new(),
            confidence: 0.0,
            fail: true,
            source: "local".into(),
        });
        let cloud = Arc::new(MockStt {
            text: "recovered".into(),
            confidence: 0.95,
            fail: false,
            source: "cloud:openai-whisper".into(),
        });
        let stt = FallbackStt::new(local).with_cloud(cloud);
        let r = stt.transcribe(&pcm_ms(50), "en").await.unwrap();
        assert_eq!(r.text, "recovered");
    }

    #[tokio::test]
    async fn fallback_stt_propagates_when_no_cloud() {
        let local = Arc::new(MockStt {
            text: String::new(),
            confidence: 0.0,
            fail: true,
            source: "local".into(),
        });
        let stt = FallbackStt::new(local);
        let err = stt.transcribe(&pcm_ms(10), "en").await.unwrap_err();
        assert!(err.contains("mock stt fail"));
    }

    #[tokio::test]
    async fn fallback_tts_cloud_on_local_error() {
        let local = Arc::new(MockTts {
            fail: true,
            source: "local".into(),
        });
        let cloud = Arc::new(MockTts {
            fail: false,
            source: "cloud:openai-tts".into(),
        });
        let tts = FallbackTts::new(local).with_cloud(cloud);
        let s = tts.synthesize("hi", "", 1.0).await.unwrap();
        assert_eq!(s.source, "cloud:openai-tts");
        assert!(!s.samples.is_empty());
    }

    #[tokio::test]
    async fn pcm_duration_ms() {
        let p = pcm_ms(500);
        assert_eq!(p.duration_ms(), 500);
    }

    #[tokio::test]
    async fn unavailable_mic_errors() {
        let mic = UnavailableMicCapture::new("no mic");
        assert!(mic.capture(1.0).await.is_err());
    }

    #[tokio::test]
    async fn recording_playback_stores_samples() {
        let pb = RecordingPlayback::new();
        pb.play(&[1, 2, 3], 16_000).await.unwrap();
        let last = pb.last.lock().unwrap().clone().unwrap();
        assert_eq!(last.samples, vec![1, 2, 3]);
    }
}
