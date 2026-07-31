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
    DiarizationResult, DiarizationSegment, SttBackend, SttModel, SubstrateStt, SubstrateTts,
    TtsChunk, TtsEngine, TtsTier, Utterance,
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
    /// Multi-speaker diarization segments (WEFT-227). Empty when diarization
    /// was not run; labels are `spk-0` / `spk-1` / … unless remapped to
    /// enrolled speaker ids.
    pub speakers: Vec<DiarizationSegment>,
}

impl Transcript {
    /// Attach diarization segments (multi-speaker labels) to this transcript.
    pub fn with_diarization(mut self, diarization: DiarizationResult) -> Self {
        self.speakers = diarization.segments;
        self
    }
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
            speakers: Vec::new(),
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
            speakers: Vec::new(),
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

// ── Cloud-fallback transparency (WEFT-224 / SC-3) ───────────────────────────

/// Tracing target for SC-3 cloud-fallback transparency events.
///
/// Filter: `RUST_LOG=voice.cloud_fallback=warn` (or include in a broader filter).
pub const CLOUD_FALLBACK_TARGET: &str = "voice.cloud_fallback";

/// Human-readable OpenAI Whisper provider label (never an API key / secret).
pub const PROVIDER_OPENAI_WHISPER: &str = "OpenAI Whisper API";

/// Human-readable OpenAI TTS provider label (never an API key / secret).
pub const PROVIDER_OPENAI_TTS: &str = "OpenAI TTS API";

/// Why cloud fallback was dispatched (logged; no audio / no keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudFallbackReason {
    /// Local engine returned an error.
    LocalError,
    /// Local STT confidence was below the configured threshold.
    LowConfidence,
}

impl CloudFallbackReason {
    /// Stable snake_case tag for structured log fields.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalError => "local_error",
            Self::LowConfidence => "low_confidence",
        }
    }
}

/// SC-3 transparency message body. Contains only the provider display name —
/// never audio bytes, transcripts, synthesis text, or API keys.
pub fn cloud_fallback_transparency_message(provider: &str) -> String {
    format!("Cloud fallback active: sending audio to {provider}")
}

/// Emit a WARN transparency line when the fallback chain is about to send
/// data to a cloud provider (WEFT-224 / SC-3).
///
/// # Privacy
/// Logs **only** `provider`, `reason`, and `modality`. Callers must not pass
/// PCM samples, raw audio, transcripts, synthesis text, or credentials into
/// this function.
pub fn emit_cloud_fallback_transparency(
    provider: &str,
    reason: CloudFallbackReason,
    modality: &str,
) {
    // Format string references the `provider` field — do not interpolate
    // audio, transcripts, synthesis text, or credentials here.
    warn!(
        target: CLOUD_FALLBACK_TARGET,
        event = "voice.cloud_fallback",
        provider = %provider,
        reason = reason.as_str(),
        modality = %modality,
        "Cloud fallback active: sending audio to {provider}"
    );
}

// ── Fallback chains ─────────────────────────────────────────────────────────

/// Local-first STT with optional cloud fallback on error / low confidence.
pub struct FallbackStt {
    local: Arc<dyn SttService>,
    cloud: Option<Arc<dyn SttService>>,
    /// Display name for SC-3 transparency (never secrets / never audio).
    cloud_provider: Option<String>,
    confidence_threshold: f32,
}

impl FallbackStt {
    /// Local-only chain.
    pub fn new(local: Arc<dyn SttService>) -> Self {
        Self {
            local,
            cloud: None,
            cloud_provider: None,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        }
    }

    /// Add cloud fallback with a generic provider label.
    ///
    /// Prefer [`Self::with_cloud_provider`] so SC-3 logs name a real provider.
    pub fn with_cloud(self, cloud: Arc<dyn SttService>) -> Self {
        self.with_cloud_provider(cloud, "cloud STT provider")
    }

    /// Add cloud fallback and set the SC-3 transparency provider label.
    pub fn with_cloud_provider(
        mut self,
        cloud: Arc<dyn SttService>,
        provider: impl Into<String>,
    ) -> Self {
        self.cloud = Some(cloud);
        self.cloud_provider = Some(provider.into());
        self
    }

    /// Override confidence threshold (default 0.60).
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    fn cloud_provider_label(&self) -> &str {
        self.cloud_provider
            .as_deref()
            .unwrap_or("cloud STT provider")
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
                    // SC-3: warn before any audio leaves the machine.
                    emit_cloud_fallback_transparency(
                        self.cloud_provider_label(),
                        CloudFallbackReason::LowConfidence,
                        "stt",
                    );
                    debug!(
                        conf = low.confidence,
                        threshold = self.confidence_threshold,
                        "STT local low confidence — dispatching cloud"
                    );
                    match cloud.transcribe(pcm, language).await {
                        Ok(cloud_res) if cloud_res.confidence > low.confidence => {
                            info!(source = %cloud_res.source, "STT cloud fallback won");
                            Ok(cloud_res)
                        }
                        Ok(_) => Ok(low),
                        Err(e) => {
                            // Error string may include HTTP status text; never
                            // re-log request bodies or keys.
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
                    // SC-3: warn before any audio leaves the machine.
                    emit_cloud_fallback_transparency(
                        self.cloud_provider_label(),
                        CloudFallbackReason::LocalError,
                        "stt",
                    );
                    debug!(error = %local_err, "STT local failed — dispatching cloud");
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
    /// Display name for SC-3 transparency (never secrets).
    cloud_provider: Option<String>,
}

impl FallbackTts {
    /// Local-only chain.
    pub fn new(local: Arc<dyn TtsService>) -> Self {
        Self {
            local,
            cloud: None,
            cloud_provider: None,
        }
    }

    /// Add cloud fallback with a generic provider label.
    ///
    /// Prefer [`Self::with_cloud_provider`] so SC-3 logs name a real provider.
    pub fn with_cloud(self, cloud: Arc<dyn TtsService>) -> Self {
        self.with_cloud_provider(cloud, "cloud TTS provider")
    }

    /// Add cloud fallback and set the SC-3 transparency provider label.
    pub fn with_cloud_provider(
        mut self,
        cloud: Arc<dyn TtsService>,
        provider: impl Into<String>,
    ) -> Self {
        self.cloud = Some(cloud);
        self.cloud_provider = Some(provider.into());
        self
    }

    fn cloud_provider_label(&self) -> &str {
        self.cloud_provider
            .as_deref()
            .unwrap_or("cloud TTS provider")
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
                    // SC-3: warn before synthesis request leaves the machine.
                    // Message uses the same SC-3 phrasing; we never log `text`.
                    emit_cloud_fallback_transparency(
                        self.cloud_provider_label(),
                        CloudFallbackReason::LocalError,
                        "tts",
                    );
                    debug!(error = %local_err, "TTS local failed — dispatching cloud");
                    let s = cloud.synthesize(text, voice, speed).await?;
                    info!(source = %s.source, "TTS cloud fallback completed");
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

/// Env-driven cloud fallback config (WEFT-239).
///
/// - `WEFT_VOICE_CLOUD_FALLBACK=1|true|yes` → enabled
/// - `WEFT_VOICE_STT_PROVIDER` / `WEFT_VOICE_TTS_PROVIDER` → provider strings
///
/// When env is unset, providers default empty (router returns `None` unless
/// `enabled` and defaults apply). Unknown strings error at resolve time.
pub fn cloud_fallback_config_from_env() -> clawft_types::config::CloudFallbackConfig {
    let enabled = env::var("WEFT_VOICE_CLOUD_FALLBACK")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    clawft_types::config::CloudFallbackConfig {
        enabled,
        stt_provider: env::var("WEFT_VOICE_STT_PROVIDER").unwrap_or_default(),
        tts_provider: env::var("WEFT_VOICE_TTS_PROVIDER").unwrap_or_default(),
    }
}

/// Construct a cloud STT service from a resolved provider kind (WEFT-239).
///
/// Returns `Ok(None)` when no API key is available for the selected provider.
/// Returns `Err` for providers that have no live constructor in this crate
/// (should not happen for currently known kinds).
pub fn cloud_stt_for_provider(
    kind: clawft_types::config::CloudSttProviderKind,
    api_key: Option<String>,
) -> Result<Option<(Arc<dyn SttService>, &'static str)>, String> {
    use clawft_types::config::CloudSttProviderKind;
    match kind {
        CloudSttProviderKind::OpenAiWhisper => {
            let Some(key) = api_key.filter(|k| !k.is_empty()) else {
                return Ok(None);
            };
            Ok(Some((
                Arc::new(CloudWhisperStt::new(key)),
                CloudSttProviderKind::OpenAiWhisper.display_name(),
            )))
        }
        // Exhaustive for #[non_exhaustive] future-proofing via wildcard:
        #[allow(unreachable_patterns)]
        other => Err(format!(
            "no live STT constructor for cloud provider {:?}",
            other.as_str()
        )),
    }
}

/// Construct a cloud TTS service from a resolved provider kind (WEFT-239).
///
/// ElevenLabs is recognized by the config router but not yet implemented on
/// the live tools path — returns a clear error so startup fails loudly rather
/// than silently ignoring the selection.
pub fn cloud_tts_for_provider(
    kind: clawft_types::config::CloudTtsProviderKind,
    api_key: Option<String>,
) -> Result<Option<(Arc<dyn TtsService>, &'static str)>, String> {
    use clawft_types::config::CloudTtsProviderKind;
    match kind {
        CloudTtsProviderKind::OpenAi => {
            let Some(key) = api_key.filter(|k| !k.is_empty()) else {
                return Ok(None);
            };
            Ok(Some((
                Arc::new(CloudOpenAiTts::new(key)),
                CloudTtsProviderKind::OpenAi.display_name(),
            )))
        }
        CloudTtsProviderKind::ElevenLabs => Err(
            "cloud TTS provider \"elevenlabs\" is configured but not yet wired \
             on the live tools path; use \"openai\" or leave tts_provider empty"
                .into(),
        ),
        #[allow(unreachable_patterns)]
        other => Err(format!(
            "no live TTS constructor for cloud provider {:?}",
            other.as_str()
        )),
    }
}

/// Attach cloud STT from a [`clawft_types::config::CloudFallbackConfig`] (WEFT-239).
pub fn apply_cloud_stt_from_config(
    chain: FallbackStt,
    cfg: &clawft_types::config::CloudFallbackConfig,
    api_key: Option<String>,
) -> Result<FallbackStt, String> {
    let kind = cfg
        .resolve_stt_provider()
        .map_err(|e| e.to_string())?;
    let Some(kind) = kind else {
        return Ok(chain);
    };
    // When provider is named in config but cloud is not enabled and key is
    // missing, still no-op (local-only). When enabled or provider explicit
    // with a key, attach.
    match cloud_stt_for_provider(kind, api_key)? {
        Some((svc, label)) => Ok(chain.with_cloud_provider(svc, label)),
        None => Ok(chain),
    }
}

/// Attach cloud TTS from a [`clawft_types::config::CloudFallbackConfig`] (WEFT-239).
pub fn apply_cloud_tts_from_config(
    chain: FallbackTts,
    cfg: &clawft_types::config::CloudFallbackConfig,
    api_key: Option<String>,
) -> Result<FallbackTts, String> {
    let kind = cfg
        .resolve_tts_provider()
        .map_err(|e| e.to_string())?;
    let Some(kind) = kind else {
        return Ok(chain);
    };
    match cloud_tts_for_provider(kind, api_key)? {
        Some((svc, label)) => Ok(chain.with_cloud_provider(svc, label)),
        None => Ok(chain),
    }
}

/// Build the production STT chain (substrate local + optional OpenAI cloud).
///
/// Cloud provider is selected via [`cloud_fallback_config_from_env`] (WEFT-239)
/// when set; otherwise any `OPENAI_API_KEY` attaches OpenAI Whisper (legacy).
pub fn build_default_stt() -> Arc<dyn SttService> {
    let local: Arc<dyn SttService> = Arc::new(LocalSubstrateStt::new(default_stt_url()));
    let cfg = cloud_fallback_config_from_env();
    let key = openai_api_key();

    let mut attached = false;
    let mut chain = FallbackStt::new(local);
    match cfg.resolve_stt_provider() {
        Ok(Some(kind)) => match cloud_stt_for_provider(kind, key.clone()) {
            Ok(Some((svc, label))) => {
                chain = chain.with_cloud_provider(svc, label);
                attached = true;
            }
            Ok(None) => {
                debug!("cloud STT provider selected but no API key; local-only");
            }
            Err(e) => warn!(error = %e, "cloud STT constructor failed; local-only"),
        },
        Ok(None) => {}
        Err(e) => warn!(error = %e, "cloud STT config rejected; local-only"),
    }

    // Legacy: OPENAI_API_KEY without explicit cloud config still enables Whisper.
    if !attached && let Some(key) = key {
        chain = chain.with_cloud_provider(
            Arc::new(CloudWhisperStt::new(key)),
            PROVIDER_OPENAI_WHISPER,
        );
    }
    Arc::new(chain)
}

/// Build the production TTS chain (substrate local + optional OpenAI cloud).
pub fn build_default_tts() -> Arc<dyn TtsService> {
    let local: Arc<dyn TtsService> = Arc::new(LocalSubstrateTts::new(default_tts_url()));
    let cfg = cloud_fallback_config_from_env();
    let key = openai_api_key();

    let mut attached = false;
    let mut chain = FallbackTts::new(local);
    match cfg.resolve_tts_provider() {
        Ok(Some(kind)) => match cloud_tts_for_provider(kind, key.clone()) {
            Ok(Some((svc, label))) => {
                chain = chain.with_cloud_provider(svc, label);
                attached = true;
            }
            Ok(None) => {
                debug!("cloud TTS provider selected but no API key; local-only");
            }
            Err(e) => warn!(error = %e, "cloud TTS constructor failed; local-only"),
        },
        Ok(None) => {}
        Err(e) => warn!(error = %e, "cloud TTS config rejected; local-only"),
    }

    if !attached && let Some(key) = key {
        chain = chain.with_cloud_provider(
            Arc::new(CloudOpenAiTts::new(key)),
            PROVIDER_OPENAI_TTS,
        );
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
                speakers: Vec::new(),
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

    #[test]
    fn cloud_provider_router_stt_openai_whisper() {
        use clawft_types::config::{CloudFallbackConfig, CloudSttProviderKind};
        let cfg = CloudFallbackConfig {
            enabled: true,
            stt_provider: "whisper".into(),
            tts_provider: "openai".into(),
        };
        assert_eq!(
            cfg.resolve_stt_provider().unwrap(),
            Some(CloudSttProviderKind::OpenAiWhisper)
        );
        let built = cloud_stt_for_provider(
            CloudSttProviderKind::OpenAiWhisper,
            Some("sk-test".into()),
        )
        .unwrap()
        .expect("key present");
        assert_eq!(built.1, PROVIDER_OPENAI_WHISPER);
        assert!(
            cloud_stt_for_provider(CloudSttProviderKind::OpenAiWhisper, None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn cloud_provider_router_tts_openai_and_elevenlabs() {
        use clawft_types::config::CloudTtsProviderKind;
        let built = cloud_tts_for_provider(CloudTtsProviderKind::OpenAi, Some("sk-test".into()))
            .unwrap()
            .expect("openai");
        assert_eq!(built.1, PROVIDER_OPENAI_TTS);
        match cloud_tts_for_provider(CloudTtsProviderKind::ElevenLabs, Some("x".into())) {
            Ok(_) => panic!("elevenlabs should error on live tools path"),
            Err(err) => assert!(err.contains("elevenlabs")),
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

    // ── WEFT-224 / SC-3 transparency ────────────────────────────────────────

    #[test]
    fn cloud_fallback_message_format_sc3() {
        let msg = cloud_fallback_transparency_message(PROVIDER_OPENAI_WHISPER);
        assert_eq!(
            msg,
            "Cloud fallback active: sending audio to OpenAI Whisper API"
        );
        // Privacy: message must not embed credentials or raw audio markers.
        assert!(!msg.contains("sk-"));
        assert!(!msg.contains("api_key"));
        assert!(!msg.contains("pcm"));
    }

    #[test]
    fn cloud_fallback_reason_tags() {
        assert_eq!(CloudFallbackReason::LocalError.as_str(), "local_error");
        assert_eq!(
            CloudFallbackReason::LowConfidence.as_str(),
            "low_confidence"
        );
    }

    /// Capture `voice.cloud_fallback` events the same way SC-1/SC-4 tests do.
    fn capture_cloud_fallback_events<F>(f: F) -> Vec<(String, String, String, String)>
    where
        F: FnOnce(),
    {
        use std::sync::Mutex as StdMutex;
        use tracing::Subscriber;
        use tracing::field::{Field, Visit};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::{Context, SubscriberExt};
        use tracing_subscriber::registry::LookupSpan;

        #[derive(Default, Clone)]
        struct Captured {
            event: Option<String>,
            provider: Option<String>,
            reason: Option<String>,
            modality: Option<String>,
            message: Option<String>,
        }

        impl Visit for Captured {
            fn record_str(&mut self, field: &Field, value: &str) {
                self.assign(field.name(), value);
            }
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                // `%field` Display values and the default message arrive here.
                self.assign(field.name(), &format!("{value:?}"));
            }
        }

        impl Captured {
            fn assign(&mut self, name: &str, value: &str) {
                // Debug format of a &str is quoted; strip for stable asserts.
                let value = value
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(value);
                match name {
                    "event" => self.event = Some(value.to_string()),
                    "provider" => self.provider = Some(value.to_string()),
                    "reason" => self.reason = Some(value.to_string()),
                    "modality" => self.modality = Some(value.to_string()),
                    "message" => self.message = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        struct CapturingLayer {
            sink: Arc<StdMutex<Vec<Captured>>>,
        }

        impl<S> Layer<S> for CapturingLayer
        where
            S: Subscriber + for<'a> LookupSpan<'a>,
        {
            fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
                if event.metadata().target() != CLOUD_FALLBACK_TARGET {
                    return;
                }
                let mut cap = Captured::default();
                event.record(&mut cap);
                self.sink.lock().unwrap().push(cap);
            }
        }

        let sink: Arc<StdMutex<Vec<Captured>>> = Arc::new(StdMutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(CapturingLayer { sink: sink.clone() });
        with_default(subscriber, f);

        sink.lock()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c.event.clone().unwrap_or_default(),
                    c.provider.clone().unwrap_or_default(),
                    c.reason.clone().unwrap_or_default(),
                    c.message.clone().unwrap_or_default(),
                )
            })
            .collect()
    }

    #[test]
    fn sc3_stt_low_confidence_emits_transparency_warn() {
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
        let stt = FallbackStt::new(local)
            .with_cloud_provider(cloud, PROVIDER_OPENAI_WHISPER);
        let pcm = pcm_ms(50);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let events = capture_cloud_fallback_events(|| {
            rt.block_on(async {
                let r = stt.transcribe(&pcm, "en").await.unwrap();
                assert_eq!(r.text, "certain");
            });
        });

        assert_eq!(events.len(), 1, "expected one SC-3 event, got {events:?}");
        let (event, provider, reason, message) = &events[0];
        assert_eq!(event, "voice.cloud_fallback");
        assert_eq!(provider, PROVIDER_OPENAI_WHISPER);
        assert_eq!(reason, "low_confidence");
        assert!(
            message.contains("Cloud fallback active: sending audio to OpenAI Whisper API")
                || message.is_empty(), // some layers only expose structured fields
            "unexpected message: {message:?}"
        );
        // Privacy: no sample dumps or keys in structured fields we capture.
        assert!(!provider.contains("sk-"));
        assert!(!message.contains("sk-"));
    }

    #[test]
    fn sc3_stt_local_error_emits_transparency_warn() {
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
        let stt = FallbackStt::new(local)
            .with_cloud_provider(cloud, PROVIDER_OPENAI_WHISPER);
        let pcm = pcm_ms(20);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let events = capture_cloud_fallback_events(|| {
            rt.block_on(async {
                let r = stt.transcribe(&pcm, "en").await.unwrap();
                assert_eq!(r.text, "recovered");
            });
        });

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].2, "local_error");
        assert_eq!(events[0].1, PROVIDER_OPENAI_WHISPER);
    }

    #[test]
    fn sc3_tts_local_error_emits_transparency_warn() {
        let local = Arc::new(MockTts {
            fail: true,
            source: "local".into(),
        });
        let cloud = Arc::new(MockTts {
            fail: false,
            source: "cloud:openai-tts".into(),
        });
        let tts = FallbackTts::new(local).with_cloud_provider(cloud, PROVIDER_OPENAI_TTS);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let events = capture_cloud_fallback_events(|| {
            rt.block_on(async {
                // Synthesis text must not appear in the transparency log.
                let s = tts
                    .synthesize("secret utterance not for logs", "", 1.0)
                    .await
                    .unwrap();
                assert_eq!(s.source, "cloud:openai-tts");
            });
        });

        assert_eq!(events.len(), 1);
        let (event, provider, reason, message) = &events[0];
        assert_eq!(event, "voice.cloud_fallback");
        assert_eq!(provider, PROVIDER_OPENAI_TTS);
        assert_eq!(reason, "local_error");
        assert!(!message.contains("secret utterance"));
        assert!(!provider.contains("secret"));
    }

    #[test]
    fn sc3_high_confidence_local_does_not_emit() {
        let local = Arc::new(MockStt {
            text: "hello".into(),
            confidence: 0.95,
            fail: false,
            source: "local".into(),
        });
        let cloud = Arc::new(MockStt {
            text: "cloud".into(),
            confidence: 0.99,
            fail: false,
            source: "cloud:x".into(),
        });
        let stt = FallbackStt::new(local)
            .with_cloud_provider(cloud, PROVIDER_OPENAI_WHISPER);
        let pcm = pcm_ms(10);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let events = capture_cloud_fallback_events(|| {
            rt.block_on(async {
                let r = stt.transcribe(&pcm, "en").await.unwrap();
                assert_eq!(r.source, "local");
            });
        });
        assert!(
            events.is_empty(),
            "local-high-confidence must not emit SC-3: {events:?}"
        );
    }
}
