//! audio_synthesize tool -- Generate audio files from text.
//!
//! Synthesizes text to speech via the live TTS stack (WEFT-214) and writes a
//! real `.wav` file (WEFT-233 codec). Gated behind the `voice` feature flag.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use serde_json::{Value, json};
use tracing::info;

use crate::audio_codec::{self, AudioFormat};
use crate::voice_backend::{PcmAudio, Synthesis, TtsService, build_default_tts};

/// Tool for synthesizing text to an audio file.
///
/// Parameters:
/// - `text` (required): Text to synthesize.
/// - `output_path` (required): Absolute path for the output `.wav` file.
/// - `voice` (optional): Voice ID to use.
/// - `speed` (optional): Speech rate multiplier (0.5-2.0).
pub struct AudioSynthesizeTool {
    tts: Arc<dyn TtsService>,
}

impl AudioSynthesizeTool {
    /// Production defaults: env-based substrate TTS (+ optional cloud).
    pub fn new() -> Self {
        Self {
            tts: build_default_tts(),
        }
    }

    /// Inject TTS (unit tests and daemon wiring).
    pub fn with_tts(tts: Arc<dyn TtsService>) -> Self {
        Self { tts }
    }
}

impl Default for AudioSynthesizeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AudioSynthesizeTool {
    fn name(&self) -> &str {
        "audio_synthesize"
    }

    fn description(&self) -> &str {
        "Synthesize text to speech and save as a real .wav audio file \
         (PCM s16le). Uses local substrate TTS with optional cloud fallback."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text", "output_path"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to synthesize into speech."
                },
                "output_path": {
                    "type": "string",
                    "description": "Absolute path for the output .wav file."
                },
                "voice": {
                    "type": "string",
                    "description": "Voice ID to use (default: system default voice)."
                },
                "speed": {
                    "type": "number",
                    "description": "Speech rate multiplier (0.5-2.0, default 1.0)."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let text = args["text"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("text is required".into()))?;

        let output_path = args["output_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("output_path is required".into()))?;

        let voice = args["voice"].as_str().unwrap_or("default");
        let speed = args["speed"].as_f64().unwrap_or(1.0);

        if text.is_empty() {
            return Err(ToolError::InvalidArgs("text must be non-empty".into()));
        }

        if !(0.5..=2.0).contains(&speed) {
            return Err(ToolError::InvalidArgs(
                "speed must be between 0.5 and 2.0".into(),
            ));
        }

        let path = Path::new(output_path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            return Err(ToolError::InvalidPath(format!(
                "Output directory does not exist: {}",
                parent.display()
            )));
        }

        // Only WAV encode is implemented (max feasible write path).
        match path.extension().and_then(|e| e.to_str()) {
            Some("wav") => {}
            Some(ext) => {
                return Err(ToolError::InvalidArgs(format!(
                    "Only .wav output is supported (got .{ext}); MP3/OGG/WebM encode is not implemented"
                )));
            }
            None => {
                return Err(ToolError::InvalidArgs(
                    "Output path must have .wav extension".into(),
                ));
            }
        }

        let synth: Synthesis = self
            .tts
            .synthesize(text, voice, speed)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("TTS failed: {e}")))?;

        let pcm = PcmAudio {
            samples: synth.samples,
            sample_rate: synth.sample_rate,
        };

        audio_codec::write_wav_file(path, &pcm)
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        // Verify the written file is a real decodable WAV.
        let (decoded, fmt) = audio_codec::decode_file(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("WAV verify failed: {e}")))?;
        if fmt != AudioFormat::Wav {
            return Err(ToolError::ExecutionFailed(
                "WAV verify produced unexpected format".into(),
            ));
        }
        if decoded.samples.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "TTS produced empty audio".into(),
            ));
        }

        info!(
            text_len = text.len(),
            output_path = output_path,
            voice = voice,
            speed = speed,
            duration_ms = synth.duration_ms,
            source = %synth.source,
            "audio_synthesize completed"
        );

        Ok(json!({
            "status": "synthesized",
            "output_path": output_path,
            "voice": voice,
            "speed": speed,
            "duration_ms": synth.duration_ms,
            "sample_rate": synth.sample_rate,
            "samples": decoded.samples.len(),
            "source": synth.source,
            "format": "wav",
            "mime_type": AudioFormat::Wav.mime_type(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_codec::decode_file;
    use async_trait::async_trait;

    struct MockTts;

    #[async_trait]
    impl TtsService for MockTts {
        async fn synthesize(
            &self,
            _text: &str,
            _voice: &str,
            _speed: f64,
        ) -> Result<Synthesis, String> {
            Ok(Synthesis {
                samples: (0..1600).map(|i| (i % 200) as i16).collect(),
                sample_rate: 16_000,
                duration_ms: 100,
                source: "local".into(),
            })
        }
    }

    #[test]
    fn tool_metadata() {
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        assert_eq!(tool.name(), "audio_synthesize");
        assert!(!tool.description().is_empty());
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["text"].is_object());
        assert!(params["properties"]["output_path"].is_object());
        assert!(params["properties"]["voice"].is_object());
        assert!(params["properties"]["speed"].is_object());
        assert_eq!(params["required"][0], "text");
        assert_eq!(params["required"][1], "output_path");
    }

    #[tokio::test]
    async fn execute_missing_text_errors() {
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        let args = json!({"output_path": "/tmp/out.wav"});
        assert!(tool.execute(args).await.is_err());
    }

    #[tokio::test]
    async fn execute_missing_output_path_errors() {
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        let args = json!({"text": "hello"});
        assert!(tool.execute(args).await.is_err());
    }

    #[tokio::test]
    async fn execute_empty_text_errors() {
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        let args = json!({"text": "", "output_path": "/tmp/out.wav"});
        assert!(tool.execute(args).await.is_err());
    }

    #[tokio::test]
    async fn execute_wrong_extension_errors() {
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        let args = json!({"text": "hello", "output_path": "/tmp/out.mp3"});
        assert!(tool.execute(args).await.is_err());
    }

    #[tokio::test]
    async fn execute_writes_real_wav() {
        let tmp = std::env::temp_dir().join("test_synth_output_weft233.wav");
        let _ = tokio::fs::remove_file(&tmp).await;
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        let args = json!({
            "text": "Hello world",
            "output_path": tmp.to_string_lossy(),
            "voice": "nova",
            "speed": 1.2
        });
        let result = tool.execute(args).await.unwrap();
        assert_eq!(result["status"], "synthesized");
        assert_eq!(result["voice"], "nova");
        assert_eq!(result["format"], "wav");
        assert_eq!(result["source"], "local");
        assert!(tmp.exists());

        let (pcm, fmt) = decode_file(&tmp).unwrap();
        assert_eq!(fmt, AudioFormat::Wav);
        assert_eq!(pcm.samples.len(), 1600);
        assert_eq!(pcm.sample_rate, 16_000);

        let _ = tokio::fs::remove_file(&tmp).await;
    }

    #[tokio::test]
    async fn execute_nonexistent_output_dir_errors() {
        let tool = AudioSynthesizeTool::with_tts(Arc::new(MockTts));
        let args = json!({
            "text": "hello",
            "output_path": "/nonexistent_dir_12345/out.wav"
        });
        assert!(tool.execute(args).await.is_err());
    }
}
