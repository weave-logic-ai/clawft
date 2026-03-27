//! Cloud-based speech-to-text providers.
//!
//! Defines the [`CloudSttProvider`] trait for cloud STT services and
//! provides a [`WhisperSttProvider`] implementation using the OpenAI
//! Whisper API.

use async_trait::async_trait;

use crate::PluginError;

/// Result from a cloud STT transcription.
#[derive(Debug, Clone)]
pub struct CloudSttResult {
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f32,
    /// Detected or hinted language code.
    pub language: String,
    /// Duration of the audio in milliseconds.
    pub duration_ms: u64,
}

/// Trait for cloud-based speech-to-text providers.
#[async_trait]
pub trait CloudSttProvider: Send + Sync {
    /// Provider name (e.g., "openai-whisper").
    fn name(&self) -> &str;

    /// Transcribe audio bytes to text.
    ///
    /// * `audio_data` - Raw audio in the format specified by `mime_type`.
    /// * `mime_type` - MIME type of the audio (e.g., "audio/wav").
    /// * `language` - Optional BCP-47 language hint (e.g., "en").
    async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        language: Option<&str>,
    ) -> Result<CloudSttResult, PluginError>;
}

/// OpenAI Whisper API implementation of [`CloudSttProvider`].
///
/// Sends audio to `https://api.openai.com/v1/audio/transcriptions`
/// using multipart form upload. Returns transcription with a default
/// confidence of 0.95 (Whisper does not return per-utterance confidence).
pub struct WhisperSttProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl WhisperSttProvider {
    /// Create a new Whisper provider with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "whisper-1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Override the model name (default: "whisper-1").
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Map a MIME type to a file extension for the multipart upload.
    fn mime_to_extension(mime_type: &str) -> &'static str {
        match mime_type {
            "audio/wav" => "wav",
            "audio/webm" | "audio/webm;codecs=opus" => "webm",
            "audio/mp3" | "audio/mpeg" => "mp3",
            "audio/ogg" | "audio/ogg;codecs=opus" => "ogg",
            _ => "wav",
        }
    }
}

#[async_trait]
impl CloudSttProvider for WhisperSttProvider {
    fn name(&self) -> &str {
        "openai-whisper"
    }

    async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        language: Option<&str>,
    ) -> Result<CloudSttResult, PluginError> {
        let extension = Self::mime_to_extension(mime_type);

        let file_part = reqwest::multipart::Part::bytes(audio_data.to_vec())
            .file_name(format!("audio.{extension}"))
            .mime_str(mime_type)
            .map_err(|e| PluginError::ExecutionFailed(format!("MIME error: {e}")))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json");

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("Whisper API request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(format!(
                "Whisper API returned {status}: {body}"
            )));
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            PluginError::ExecutionFailed(format!("Whisper response parse error: {e}"))
        })?;

        Ok(CloudSttResult {
            text: body["text"].as_str().unwrap_or("").to_string(),
            confidence: 0.95, // Whisper does not return confidence; assume high
            language: body["language"].as_str().unwrap_or("en").to_string(),
            duration_ms: (body["duration"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_provider_name() {
        let provider = WhisperSttProvider::new("test-key".into());
        assert_eq!(provider.name(), "openai-whisper");
    }

    #[test]
    fn whisper_with_model_builder() {
        let provider = WhisperSttProvider::new("test-key".into()).with_model("whisper-2");
        assert_eq!(provider.model, "whisper-2");
    }

    #[test]
    fn mime_to_extension_mapping() {
        assert_eq!(WhisperSttProvider::mime_to_extension("audio/wav"), "wav");
        assert_eq!(WhisperSttProvider::mime_to_extension("audio/webm"), "webm");
        assert_eq!(
            WhisperSttProvider::mime_to_extension("audio/webm;codecs=opus"),
            "webm"
        );
        assert_eq!(WhisperSttProvider::mime_to_extension("audio/mp3"), "mp3");
        assert_eq!(WhisperSttProvider::mime_to_extension("audio/mpeg"), "mp3");
        assert_eq!(WhisperSttProvider::mime_to_extension("audio/ogg"), "ogg");
        assert_eq!(
            WhisperSttProvider::mime_to_extension("audio/ogg;codecs=opus"),
            "ogg"
        );
        assert_eq!(
            WhisperSttProvider::mime_to_extension("audio/unknown"),
            "wav"
        );
    }

    #[test]
    fn cloud_stt_result_fields() {
        let result = CloudSttResult {
            text: "hello world".into(),
            confidence: 0.95,
            language: "en".into(),
            duration_ms: 1500,
        };
        assert_eq!(result.text, "hello world");
        assert!((result.confidence - 0.95).abs() < f32::EPSILON);
        assert_eq!(result.language, "en");
        assert_eq!(result.duration_ms, 1500);
    }

    #[tokio::test]
    async fn whisper_transcribe_invalid_key_errors() {
        // Using a clearly invalid key should produce an error when
        // actually calling the API. We test that the provider can be
        // constructed and that the transcribe method returns an error
        // for a connection that will fail (no real API server).
        let provider = WhisperSttProvider::new("invalid-key".into());
        let result = provider
            .transcribe(b"fake audio", "audio/wav", Some("en"))
            .await;
        // Will fail because we cannot reach the API in tests
        assert!(result.is_err());
    }
}
