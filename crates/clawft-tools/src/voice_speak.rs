//! voice_speak tool -- On-demand text-to-speech synthesis.
//!
//! Synthesizes text via local substrate TTS with optional cloud fallback,
//! then plays through an injected [`SpeakerPlayback`] (WEFT-214).
//! Gated behind the `voice` feature flag.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use serde_json::{Value, json};
use tracing::info;

use crate::voice_backend::{
    build_default_playback, build_default_tts, SpeakerPlayback, Synthesis, TtsService,
};

/// Tool that speaks text through the speaker using TTS.
pub struct VoiceSpeakTool {
    tts: Arc<dyn TtsService>,
    playback: Arc<dyn SpeakerPlayback>,
    /// When true, playback is a real sink (status `played`); when false, noop.
    playback_is_real: bool,
}

impl VoiceSpeakTool {
    /// Production defaults: env-based substrate TTS (+ optional OpenAI cloud)
    /// and noop playback until a sink is injected.
    pub fn new() -> Self {
        Self {
            tts: build_default_tts(),
            playback: build_default_playback(),
            playback_is_real: false,
        }
    }

    /// Inject TTS + playback. `playback_is_real` controls the status string
    /// (`played` vs `synthesized`).
    pub fn with_backends(
        tts: Arc<dyn TtsService>,
        playback: Arc<dyn SpeakerPlayback>,
        playback_is_real: bool,
    ) -> Self {
        Self {
            tts,
            playback,
            playback_is_real,
        }
    }
}

impl Default for VoiceSpeakTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for VoiceSpeakTool {
    fn name(&self) -> &str {
        "voice_speak"
    }

    fn description(&self) -> &str {
        "Speak text aloud through the system speaker using text-to-speech synthesis. \
         Uses local substrate TTS with optional cloud fallback."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to speak aloud"
                },
                "voice": {
                    "type": "string",
                    "description": "Voice ID to use (empty for default)",
                    "default": ""
                },
                "speed": {
                    "type": "number",
                    "description": "Speaking speed multiplier (1.0 = normal)",
                    "default": 1.0
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let voice = args.get("voice").and_then(|v| v.as_str()).unwrap_or("");
        let speed = args.get("speed").and_then(|v| v.as_f64()).unwrap_or(1.0);

        if text.is_empty() {
            return Err(ToolError::InvalidArgs(
                "\"text\" parameter is required and must be non-empty".into(),
            ));
        }
        if !(0.25..=4.0).contains(&speed) {
            return Err(ToolError::InvalidArgs(
                "\"speed\" must be between 0.25 and 4.0".into(),
            ));
        }

        let synth: Synthesis = self
            .tts
            .synthesize(text, voice, speed)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("TTS failed: {e}")))?;

        self.playback
            .play(&synth.samples, synth.sample_rate)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("playback failed: {e}")))?;

        let status = if self.playback_is_real {
            "played"
        } else {
            "synthesized"
        };

        info!(
            text_len = text.len(),
            duration_ms = synth.duration_ms,
            source = %synth.source,
            status = status,
            "voice_speak completed"
        );

        Ok(json!({
            "spoken": true,
            "text_length": text.len(),
            "duration_ms": synth.duration_ms,
            "sample_rate": synth.sample_rate,
            "source": synth.source,
            "status": status
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_backend::{FallbackTts, RecordingPlayback, TtsService};
    use async_trait::async_trait;

    struct MockTts {
        fail: bool,
        source: &'static str,
        samples: Vec<i16>,
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
                return Err("tts down".into());
            }
            Ok(Synthesis {
                samples: self.samples.clone(),
                sample_rate: 16_000,
                duration_ms: 100,
                source: self.source.into(),
            })
        }
    }

    #[test]
    fn tool_metadata() {
        let tool = VoiceSpeakTool::new();
        assert_eq!(tool.name(), "voice_speak");
        assert!(!tool.description().is_empty());
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["text"].is_object());
        assert!(params["properties"]["voice"].is_object());
        assert!(params["properties"]["speed"].is_object());
        assert_eq!(params["required"][0], "text");
    }

    #[tokio::test]
    async fn execute_speaks_and_reports_playback() {
        let tts = Arc::new(MockTts {
            fail: false,
            source: "local",
            samples: vec![1, 2, 3, 4],
        });
        let playback = Arc::new(RecordingPlayback::new());
        let tool = VoiceSpeakTool::with_backends(tts, playback.clone(), true);
        let result = tool
            .execute(json!({"text": "Hello world", "speed": 1.5}))
            .await
            .expect("speak should succeed");
        assert_eq!(result["status"], "played");
        assert_eq!(result["spoken"], true);
        assert_eq!(result["text_length"], 11);
        assert_eq!(result["duration_ms"], 100);
        assert_eq!(result["source"], "local");
        assert_ne!(result["status"], "stub_not_implemented");
        let last = playback.last.lock().unwrap().clone().unwrap();
        assert_eq!(last.samples, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn execute_synthesized_status_with_noop_playback() {
        let tts = Arc::new(MockTts {
            fail: false,
            source: "local",
            samples: vec![9],
        });
        let tool =
            VoiceSpeakTool::with_backends(tts, Arc::new(crate::voice_backend::NoopPlayback), false);
        let result = tool.execute(json!({"text": "hi"})).await.unwrap();
        assert_eq!(result["status"], "synthesized");
        assert_eq!(result["spoken"], true);
    }

    #[tokio::test]
    async fn execute_empty_text_errors() {
        let tool = VoiceSpeakTool::default();
        let result = tool.execute(json!({"text": ""})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_missing_text_errors() {
        let tool = VoiceSpeakTool::new();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_invalid_speed() {
        let tts = Arc::new(MockTts {
            fail: false,
            source: "local",
            samples: vec![1],
        });
        let tool =
            VoiceSpeakTool::with_backends(tts, Arc::new(crate::voice_backend::NoopPlayback), false);
        let err = tool
            .execute(json!({"text": "hi", "speed": 10.0}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn execute_cloud_fallback_on_local_tts_error() {
        let local = Arc::new(MockTts {
            fail: true,
            source: "local",
            samples: vec![],
        });
        let cloud = Arc::new(MockTts {
            fail: false,
            source: "cloud:openai-tts",
            samples: vec![7, 8],
        });
        let tts = Arc::new(FallbackTts::new(local).with_cloud(cloud));
        let playback = Arc::new(RecordingPlayback::new());
        let tool = VoiceSpeakTool::with_backends(tts, playback, true);
        let result = tool.execute(json!({"text": "fallback please"})).await.unwrap();
        assert_eq!(result["status"], "played");
        assert_eq!(result["source"], "cloud:openai-tts");
    }

    #[tokio::test]
    async fn execute_playback_failure() {
        let tts = Arc::new(MockTts {
            fail: false,
            source: "local",
            samples: vec![1],
        });
        let mut pb = RecordingPlayback::new();
        pb.fail = true;
        let tool = VoiceSpeakTool::with_backends(tts, Arc::new(pb), true);
        let err = tool.execute(json!({"text": "hi"})).await.unwrap_err();
        assert!(err.to_string().contains("playback failed"));
    }
}
