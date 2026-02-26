//! STT/TTS fallback chains: local-first with cloud fallback.
//!
//! [`SttFallbackChain`] tries local STT first, falling back to a cloud
//! provider when the local result has low confidence or errors.
//! [`TtsFallbackChain`] tries local TTS first, falling back on error.

use crate::PluginError;

use super::cloud_stt::CloudSttProvider;
use super::cloud_tts::CloudTtsProvider;

/// Minimum confidence score to accept a local STT result without
/// attempting cloud fallback.
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.60;

// ---------------------------------------------------------------------------
// Local engine traits
// ---------------------------------------------------------------------------

/// Trait for the local STT engine (e.g., sherpa-rs based).
#[async_trait::async_trait]
pub trait LocalSttEngine: Send + Sync {
    /// Transcribe audio data using the local engine.
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
    ) -> Result<LocalSttResult, PluginError>;
}

/// Result from a local STT transcription.
#[derive(Debug, Clone)]
pub struct LocalSttResult {
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
}

/// Trait for the local TTS engine (e.g., sherpa-rs piper).
#[async_trait::async_trait]
pub trait LocalTtsEngine: Send + Sync {
    /// Synthesize text to audio. Returns `(audio_data, mime_type)`.
    async fn synthesize(&self, text: &str) -> Result<(Vec<u8>, String), PluginError>;
}

// ---------------------------------------------------------------------------
// STT fallback
// ---------------------------------------------------------------------------

/// Source of an STT transcription result.
#[derive(Debug, Clone)]
pub enum SttSource {
    /// Result came from the local STT engine.
    Local,
    /// Result came from a cloud provider (name stored).
    Cloud(String),
}

/// Combined result from the STT fallback chain.
#[derive(Debug, Clone)]
pub struct SttFallbackResult {
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
    /// Which engine produced the result.
    pub source: SttSource,
    /// Language code.
    pub language: String,
}

/// Fallback chain for STT: local -> cloud on failure or low confidence.
///
/// Decision logic:
/// 1. Try local engine.
/// 2. If local succeeds with confidence >= threshold: return local result.
/// 3. If local succeeds with low confidence: try cloud, return whichever
///    has higher confidence. On cloud error, return the local result.
/// 4. If local fails: try cloud. On cloud error too, propagate the local
///    error (unless no cloud provider is configured).
pub struct SttFallbackChain {
    local: Box<dyn LocalSttEngine>,
    cloud: Option<Box<dyn CloudSttProvider>>,
    confidence_threshold: f32,
}

impl SttFallbackChain {
    /// Create a new chain with only a local engine.
    pub fn new(local: Box<dyn LocalSttEngine>) -> Self {
        Self {
            local,
            cloud: None,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        }
    }

    /// Add a cloud provider for fallback.
    pub fn with_cloud(mut self, provider: Box<dyn CloudSttProvider>) -> Self {
        self.cloud = Some(provider);
        self
    }

    /// Override the confidence threshold (default: 0.60).
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Transcribe audio, falling back to cloud if local fails or
    /// confidence is below the threshold.
    pub async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        language: Option<&str>,
    ) -> Result<SttFallbackResult, PluginError> {
        match self.local.transcribe(audio_data, language).await {
            Ok(local_result) if local_result.confidence >= self.confidence_threshold => {
                // Local succeeded with good confidence.
                Ok(SttFallbackResult {
                    text: local_result.text,
                    confidence: local_result.confidence,
                    source: SttSource::Local,
                    language: language.unwrap_or("en").to_string(),
                })
            }
            Ok(low_confidence) => {
                // Local succeeded but confidence is too low -- try cloud.
                if let Some(cloud) = &self.cloud {
                    match cloud.transcribe(audio_data, mime_type, language).await {
                        Ok(cloud_result) if cloud_result.confidence > low_confidence.confidence => {
                            Ok(SttFallbackResult {
                                text: cloud_result.text,
                                confidence: cloud_result.confidence,
                                source: SttSource::Cloud(cloud.name().to_string()),
                                language: cloud_result.language,
                            })
                        }
                        Ok(_) => {
                            // Cloud confidence was not better; keep local.
                            Ok(SttFallbackResult {
                                text: low_confidence.text,
                                confidence: low_confidence.confidence,
                                source: SttSource::Local,
                                language: language.unwrap_or("en").to_string(),
                            })
                        }
                        Err(_) => {
                            // Cloud also failed -- return low-confidence local.
                            Ok(SttFallbackResult {
                                text: low_confidence.text,
                                confidence: low_confidence.confidence,
                                source: SttSource::Local,
                                language: language.unwrap_or("en").to_string(),
                            })
                        }
                    }
                } else {
                    // No cloud provider configured.
                    Ok(SttFallbackResult {
                        text: low_confidence.text,
                        confidence: low_confidence.confidence,
                        source: SttSource::Local,
                        language: language.unwrap_or("en").to_string(),
                    })
                }
            }
            Err(local_err) => {
                // Local failed entirely -- try cloud.
                if let Some(cloud) = &self.cloud {
                    let cloud_result = cloud.transcribe(audio_data, mime_type, language).await?;
                    Ok(SttFallbackResult {
                        text: cloud_result.text,
                        confidence: cloud_result.confidence,
                        source: SttSource::Cloud(cloud.name().to_string()),
                        language: cloud_result.language,
                    })
                } else {
                    Err(local_err)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TTS fallback
// ---------------------------------------------------------------------------

/// Source of a TTS synthesis result.
#[derive(Debug, Clone)]
pub enum TtsSource {
    /// Result came from the local TTS engine.
    Local,
    /// Result came from a cloud provider (name stored).
    Cloud(String),
}

/// Combined result from the TTS fallback chain.
#[derive(Debug, Clone)]
pub struct TtsFallbackResult {
    /// Raw audio data.
    pub audio_data: Vec<u8>,
    /// MIME type of the audio.
    pub mime_type: String,
    /// Which engine produced the result.
    pub source: TtsSource,
}

/// Fallback chain for TTS: local -> cloud on error.
pub struct TtsFallbackChain {
    local: Box<dyn LocalTtsEngine>,
    cloud: Option<Box<dyn CloudTtsProvider>>,
}

impl TtsFallbackChain {
    /// Create a new chain with only a local engine.
    pub fn new(local: Box<dyn LocalTtsEngine>) -> Self {
        Self { local, cloud: None }
    }

    /// Add a cloud provider for fallback.
    pub fn with_cloud(mut self, provider: Box<dyn CloudTtsProvider>) -> Self {
        self.cloud = Some(provider);
        self
    }

    /// Synthesize text to speech, falling back to cloud on local error.
    pub async fn synthesize(
        &self,
        text: &str,
        voice_id: Option<&str>,
    ) -> Result<TtsFallbackResult, PluginError> {
        match self.local.synthesize(text).await {
            Ok((audio, mime)) => Ok(TtsFallbackResult {
                audio_data: audio,
                mime_type: mime,
                source: TtsSource::Local,
            }),
            Err(local_err) => {
                if let Some(cloud) = &self.cloud {
                    let voice = voice_id.unwrap_or("alloy");
                    let result = cloud.synthesize(text, voice).await?;
                    Ok(TtsFallbackResult {
                        audio_data: result.audio_data,
                        mime_type: result.mime_type,
                        source: TtsSource::Cloud(cloud.name().to_string()),
                    })
                } else {
                    Err(local_err)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Mock implementations --

    struct MockLocalStt {
        text: String,
        confidence: f32,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl LocalSttEngine for MockLocalStt {
        async fn transcribe(
            &self,
            _audio_data: &[u8],
            _language: Option<&str>,
        ) -> Result<LocalSttResult, PluginError> {
            if self.should_fail {
                return Err(PluginError::ExecutionFailed("local STT failed".into()));
            }
            Ok(LocalSttResult {
                text: self.text.clone(),
                confidence: self.confidence,
            })
        }
    }

    struct MockCloudStt {
        text: String,
        confidence: f32,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl CloudSttProvider for MockCloudStt {
        fn name(&self) -> &str {
            "mock-cloud"
        }

        async fn transcribe(
            &self,
            _audio_data: &[u8],
            _mime_type: &str,
            _language: Option<&str>,
        ) -> Result<super::super::cloud_stt::CloudSttResult, PluginError> {
            if self.should_fail {
                return Err(PluginError::ExecutionFailed("cloud STT failed".into()));
            }
            Ok(super::super::cloud_stt::CloudSttResult {
                text: self.text.clone(),
                confidence: self.confidence,
                language: "en".into(),
                duration_ms: 1000,
            })
        }
    }

    struct MockLocalTts {
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl LocalTtsEngine for MockLocalTts {
        async fn synthesize(&self, _text: &str) -> Result<(Vec<u8>, String), PluginError> {
            if self.should_fail {
                return Err(PluginError::ExecutionFailed("local TTS failed".into()));
            }
            Ok((vec![1, 2, 3], "audio/wav".into()))
        }
    }

    struct MockCloudTts {
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl CloudTtsProvider for MockCloudTts {
        fn name(&self) -> &str {
            "mock-cloud-tts"
        }

        fn available_voices(&self) -> Vec<super::super::cloud_tts::VoiceInfo> {
            vec![]
        }

        async fn synthesize(
            &self,
            _text: &str,
            _voice_id: &str,
        ) -> Result<super::super::cloud_tts::CloudTtsResult, PluginError> {
            if self.should_fail {
                return Err(PluginError::ExecutionFailed("cloud TTS failed".into()));
            }
            Ok(super::super::cloud_tts::CloudTtsResult {
                audio_data: vec![4, 5, 6],
                mime_type: "audio/mp3".into(),
                duration_ms: Some(1000),
            })
        }
    }

    // -- STT fallback tests --

    #[tokio::test]
    async fn stt_local_success_high_confidence() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "hello local".into(),
            confidence: 0.90,
            should_fail: false,
        }));
        let result = chain.transcribe(b"audio", "audio/wav", None).await.unwrap();
        assert_eq!(result.text, "hello local");
        assert!(matches!(result.source, SttSource::Local));
        assert!((result.confidence - 0.90).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn stt_local_low_confidence_cloud_fallback() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "helo lcal".into(),
            confidence: 0.30,
            should_fail: false,
        }))
        .with_cloud(Box::new(MockCloudStt {
            text: "hello local".into(),
            confidence: 0.95,
            should_fail: false,
        }));

        let result = chain.transcribe(b"audio", "audio/wav", None).await.unwrap();
        assert_eq!(result.text, "hello local");
        assert!(matches!(result.source, SttSource::Cloud(_)));
        assert!((result.confidence - 0.95).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn stt_local_low_confidence_cloud_worse_keeps_local() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "helo".into(),
            confidence: 0.50,
            should_fail: false,
        }))
        .with_cloud(Box::new(MockCloudStt {
            text: "hello".into(),
            confidence: 0.40, // Worse than local
            should_fail: false,
        }));

        let result = chain.transcribe(b"audio", "audio/wav", None).await.unwrap();
        assert_eq!(result.text, "helo");
        assert!(matches!(result.source, SttSource::Local));
    }

    #[tokio::test]
    async fn stt_local_error_cloud_fallback() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "".into(),
            confidence: 0.0,
            should_fail: true,
        }))
        .with_cloud(Box::new(MockCloudStt {
            text: "cloud result".into(),
            confidence: 0.90,
            should_fail: false,
        }));

        let result = chain.transcribe(b"audio", "audio/wav", None).await.unwrap();
        assert_eq!(result.text, "cloud result");
        assert!(matches!(result.source, SttSource::Cloud(_)));
    }

    #[tokio::test]
    async fn stt_both_fail_returns_error() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "".into(),
            confidence: 0.0,
            should_fail: true,
        }))
        .with_cloud(Box::new(MockCloudStt {
            text: "".into(),
            confidence: 0.0,
            should_fail: true,
        }));

        let result = chain.transcribe(b"audio", "audio/wav", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stt_local_low_confidence_cloud_also_fails_returns_local() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "low conf".into(),
            confidence: 0.30,
            should_fail: false,
        }))
        .with_cloud(Box::new(MockCloudStt {
            text: "".into(),
            confidence: 0.0,
            should_fail: true,
        }));

        let result = chain.transcribe(b"audio", "audio/wav", None).await.unwrap();
        assert_eq!(result.text, "low conf");
        assert!(matches!(result.source, SttSource::Local));
    }

    #[tokio::test]
    async fn stt_custom_threshold() {
        let chain = SttFallbackChain::new(Box::new(MockLocalStt {
            text: "local".into(),
            confidence: 0.80,
            should_fail: false,
        }))
        .with_confidence_threshold(0.90) // Higher threshold
        .with_cloud(Box::new(MockCloudStt {
            text: "cloud".into(),
            confidence: 0.95,
            should_fail: false,
        }));

        let result = chain.transcribe(b"audio", "audio/wav", None).await.unwrap();
        // 0.80 < 0.90 threshold, so cloud should be tried
        assert_eq!(result.text, "cloud");
        assert!(matches!(result.source, SttSource::Cloud(_)));
    }

    // -- TTS fallback tests --

    #[tokio::test]
    async fn tts_local_success() {
        let chain = TtsFallbackChain::new(Box::new(MockLocalTts { should_fail: false }));
        let result = chain.synthesize("hello", None).await.unwrap();
        assert_eq!(result.audio_data, vec![1, 2, 3]);
        assert!(matches!(result.source, TtsSource::Local));
    }

    #[tokio::test]
    async fn tts_local_error_cloud_fallback() {
        let chain = TtsFallbackChain::new(Box::new(MockLocalTts { should_fail: true }))
            .with_cloud(Box::new(MockCloudTts { should_fail: false }));
        let result = chain.synthesize("hello", None).await.unwrap();
        assert_eq!(result.audio_data, vec![4, 5, 6]);
        assert!(matches!(result.source, TtsSource::Cloud(_)));
    }

    #[tokio::test]
    async fn tts_both_fail_returns_error() {
        let chain = TtsFallbackChain::new(Box::new(MockLocalTts { should_fail: true }))
            .with_cloud(Box::new(MockCloudTts { should_fail: true }));
        let result = chain.synthesize("hello", None).await;
        assert!(result.is_err());
    }
}
