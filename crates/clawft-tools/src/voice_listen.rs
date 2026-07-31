//! voice_listen tool -- On-demand speech-to-text transcription.
//!
//! Captures audio (via injected [`MicCapture`]), runs local substrate STT with
//! optional cloud fallback, and returns transcript + confidence (WEFT-214).
//! Gated behind the `voice` feature flag.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use serde_json::{Value, json};
use tracing::info;

use crate::voice_backend::{
    build_default_capture, build_default_stt, MicCapture, SttService, Transcript,
};

/// Tool that listens to the microphone and returns transcribed text.
pub struct VoiceListenTool {
    capture: Arc<dyn MicCapture>,
    stt: Arc<dyn SttService>,
}

impl VoiceListenTool {
    /// Production defaults: env-based substrate STT (+ optional OpenAI cloud)
    /// and an unavailable mic until a [`MicCapture`] is injected.
    pub fn new() -> Self {
        Self {
            capture: build_default_capture(),
            stt: build_default_stt(),
        }
    }

    /// Fully inject capture + STT (unit tests and daemon wiring).
    pub fn with_backends(capture: Arc<dyn MicCapture>, stt: Arc<dyn SttService>) -> Self {
        Self { capture, stt }
    }
}

impl Default for VoiceListenTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for VoiceListenTool {
    fn name(&self) -> &str {
        "voice_listen"
    }

    fn description(&self) -> &str {
        "Listen to the microphone and transcribe speech to text. \
         Returns the transcribed text and confidence when the user stops speaking \
         (or the timeout elapses). Uses local substrate STT with optional cloud fallback."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "timeout_seconds": {
                    "type": "number",
                    "description": "Maximum time to listen in seconds (default: 30)",
                    "default": 30
                },
                "language": {
                    "type": "string",
                    "description": "Language code for STT (e.g., 'en', 'es'). Empty for auto-detect.",
                    "default": ""
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let timeout = args
            .get("timeout_seconds")
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0);
        let language = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !(0.5..=300.0).contains(&timeout) {
            return Err(ToolError::InvalidArgs(
                "\"timeout_seconds\" must be between 0.5 and 300".into(),
            ));
        }

        let pcm = self
            .capture
            .capture(timeout)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("capture failed: {e}")))?;

        if pcm.samples.is_empty() {
            return Ok(json!({
                "text": "",
                "confidence": 0.0,
                "language": language,
                "duration_ms": 0,
                "source": "none",
                "status": "no_speech"
            }));
        }

        let result: Transcript = self
            .stt
            .transcribe(&pcm, &language)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("STT failed: {e}")))?;

        let status = if result.text.is_empty() {
            "no_speech"
        } else {
            "ok"
        };

        info!(
            text_len = result.text.len(),
            confidence = result.confidence,
            source = %result.source,
            status = status,
            "voice_listen completed"
        );

        Ok(json!({
            "text": result.text,
            "confidence": result.confidence,
            "language": if result.language.is_empty() { language } else { result.language },
            "duration_ms": result.duration_ms,
            "source": result.source,
            "status": status
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_backend::{FallbackStt, FixedMicCapture, PcmAudio, SttService};
    use async_trait::async_trait;

    struct MockStt {
        text: &'static str,
        confidence: f32,
        fail: bool,
        source: &'static str,
    }

    #[async_trait]
    impl SttService for MockStt {
        async fn transcribe(
            &self,
            pcm: &PcmAudio,
            language: &str,
        ) -> Result<Transcript, String> {
            if self.fail {
                return Err("stt down".into());
            }
            Ok(Transcript {
                text: self.text.into(),
                confidence: self.confidence,
                language: language.into(),
                duration_ms: pcm.duration_ms(),
                source: self.source.into(),
                speakers: Vec::new(),
            })
        }
    }

    fn sample_pcm() -> PcmAudio {
        PcmAudio {
            samples: vec![1000i16; 3_200], // 200 ms @ 16 kHz
            sample_rate: 16_000,
        }
    }

    #[test]
    fn tool_metadata() {
        let tool = VoiceListenTool::new();
        assert_eq!(tool.name(), "voice_listen");
        assert!(!tool.description().is_empty());
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["timeout_seconds"].is_object());
        assert!(params["properties"]["language"].is_object());
    }

    #[tokio::test]
    async fn execute_returns_transcript_and_confidence() {
        let capture = Arc::new(FixedMicCapture::new(sample_pcm()));
        let stt = Arc::new(MockStt {
            text: "hello world",
            confidence: 0.92,
            fail: false,
            source: "local",
        });
        let tool = VoiceListenTool::with_backends(capture, stt);
        let args = json!({"timeout_seconds": 10, "language": "en"});
        let result = tool.execute(args).await.expect("listen should succeed");
        assert_eq!(result["status"], "ok");
        assert_eq!(result["text"], "hello world");
        assert!((result["confidence"].as_f64().unwrap() - 0.92).abs() < 1e-6);
        assert_eq!(result["language"], "en");
        assert_eq!(result["source"], "local");
        assert!(result["duration_ms"].as_u64().unwrap() > 0);
        assert_ne!(result["status"], "stub_not_implemented");
    }

    #[tokio::test]
    async fn execute_no_speech_when_empty_transcript() {
        let capture = Arc::new(FixedMicCapture::new(sample_pcm()));
        let stt = Arc::new(MockStt {
            text: "",
            confidence: 0.0,
            fail: false,
            source: "local",
        });
        let tool = VoiceListenTool::with_backends(capture, stt);
        let result = tool
            .execute(json!({"language": "en"}))
            .await
            .expect("empty transcript is ok");
        assert_eq!(result["status"], "no_speech");
        assert_eq!(result["text"], "");
    }

    #[tokio::test]
    async fn execute_capture_failure() {
        let capture = Arc::new(crate::voice_backend::UnavailableMicCapture::new("no device"));
        let stt = Arc::new(MockStt {
            text: "x",
            confidence: 1.0,
            fail: false,
            source: "local",
        });
        let tool = VoiceListenTool::with_backends(capture, stt);
        let err = tool.execute(json!({})).await.unwrap_err();
        assert!(err.to_string().contains("capture failed"));
    }

    #[tokio::test]
    async fn execute_stt_fallback_to_cloud() {
        let capture = Arc::new(FixedMicCapture::new(sample_pcm()));
        let local = Arc::new(MockStt {
            text: "",
            confidence: 0.0,
            fail: true,
            source: "local",
        });
        let cloud = Arc::new(MockStt {
            text: "from cloud",
            confidence: 0.95,
            fail: false,
            source: "cloud:openai-whisper",
        });
        let stt = Arc::new(FallbackStt::new(local).with_cloud(cloud));
        let tool = VoiceListenTool::with_backends(capture, stt);
        let result = tool.execute(json!({"language": "en"})).await.unwrap();
        assert_eq!(result["text"], "from cloud");
        assert_eq!(result["source"], "cloud:openai-whisper");
        assert_eq!(result["status"], "ok");
    }

    #[tokio::test]
    async fn execute_invalid_timeout() {
        let capture = Arc::new(FixedMicCapture::new(sample_pcm()));
        let stt = Arc::new(MockStt {
            text: "x",
            confidence: 1.0,
            fail: false,
            source: "local",
        });
        let tool = VoiceListenTool::with_backends(capture, stt);
        let err = tool
            .execute(json!({"timeout_seconds": 0.1}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn default_tool_is_not_stub() {
        // Default has no mic — fails with capture error, not stub_not_implemented.
        let tool = VoiceListenTool::default();
        let err = tool.execute(json!({})).await.unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("stub_not_implemented"));
        assert!(msg.contains("capture failed"));
    }
}
