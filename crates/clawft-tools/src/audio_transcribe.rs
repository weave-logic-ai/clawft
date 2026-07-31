//! audio_transcribe tool -- Transcribe audio files to text.
//!
//! Accepts a file path to a `.wav`, `.mp3`, `.ogg`, or `.webm` file, decodes
//! it to mono PCM (WEFT-233), and runs the live STT stack (WEFT-214 / WEFT-671).
//!
//! Gated behind the `voice` feature flag.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use serde_json::{Value, json};
use tracing::info;

use crate::audio_codec::{self, AudioFormat};
use crate::voice_backend::{PcmAudio, SttService, Transcript, build_default_stt};

/// Tool for transcribing audio files to text.
///
/// Parameters:
/// - `file_path` (required): Absolute path to the audio file.
/// - `language` (optional): BCP-47 language hint (e.g., "en", "es").
pub struct AudioTranscribeTool {
    stt: Arc<dyn SttService>,
}

impl AudioTranscribeTool {
    /// Production defaults: env-based substrate STT (+ optional cloud).
    pub fn new() -> Self {
        Self {
            stt: build_default_stt(),
        }
    }

    /// Inject STT (unit tests and daemon wiring).
    pub fn with_stt(stt: Arc<dyn SttService>) -> Self {
        Self { stt }
    }
}

impl Default for AudioTranscribeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AudioTranscribeTool {
    fn name(&self) -> &str {
        "audio_transcribe"
    }

    fn description(&self) -> &str {
        "Transcribe an audio file (.wav, .mp3, .ogg, .webm) to text \
         using speech-to-text. Decodes real codecs to mono PCM, then runs \
         local substrate STT with optional cloud fallback."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "required": ["file_path"],
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the audio file to transcribe (.wav, .mp3, .ogg, .webm)."
                },
                "language": {
                    "type": "string",
                    "description": "Optional BCP-47 language hint (e.g., 'en', 'es', 'ja')."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("file_path is required".into()))?;

        let language = args["language"].as_str().unwrap_or("").to_string();

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(ToolError::FileNotFound(format!(
                "File not found: {file_path}"
            )));
        }

        // Validate extension early for a crisp tool error (decode also checks).
        let format = audio_codec::format_from_path(path)
            .map_err(ToolError::InvalidArgs)?;

        let (pcm, detected): (PcmAudio, AudioFormat) = audio_codec::decode_file(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("audio decode failed: {e}")))?;

        if pcm.samples.is_empty() {
            return Ok(json!({
                "status": "no_speech",
                "file": file_path,
                "text": "",
                "mime_type": detected.mime_type(),
                "language": if language.is_empty() { "en".into() } else { language },
                "duration_ms": 0,
                "sample_rate": pcm.sample_rate,
                "source": "none",
                "format": format.extension(),
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
            "transcribed"
        };

        info!(
            file_path = file_path,
            mime_type = detected.mime_type(),
            language = %result.language,
            duration_ms = result.duration_ms,
            source = %result.source,
            "audio_transcribe completed"
        );

        Ok(json!({
            "status": status,
            "file": file_path,
            "text": result.text,
            "mime_type": detected.mime_type(),
            "language": if result.language.is_empty() {
                if language.is_empty() { "en".into() } else { language }
            } else {
                result.language
            },
            "confidence": result.confidence,
            "duration_ms": result.duration_ms,
            "sample_rate": pcm.sample_rate,
            "source": result.source,
            "format": format.extension(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_codec::{encode_wav, write_wav_file};
    use crate::voice_backend::{PcmAudio, Transcript};
    use async_trait::async_trait;

    struct MockStt {
        text: String,
    }

    #[async_trait]
    impl SttService for MockStt {
        async fn transcribe(
            &self,
            pcm: &PcmAudio,
            language: &str,
        ) -> Result<Transcript, String> {
            Ok(Transcript {
                text: self.text.clone(),
                confidence: 0.91,
                language: if language.is_empty() {
                    "en".into()
                } else {
                    language.into()
                },
                duration_ms: pcm.duration_ms(),
                source: "local".into(),
                speakers: Vec::new(),
            })
        }
    }

    fn fixture_wav(path: &Path, frames: usize) -> PcmAudio {
        let pcm = PcmAudio {
            samples: vec![1000i16; frames],
            sample_rate: 16_000,
        };
        write_wav_file(path, &pcm).unwrap();
        pcm
    }

    #[test]
    fn tool_metadata() {
        let tool = AudioTranscribeTool::with_stt(Arc::new(MockStt {
            text: String::new(),
        }));
        assert_eq!(tool.name(), "audio_transcribe");
        assert!(!tool.description().is_empty());
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["file_path"].is_object());
        assert!(params["properties"]["language"].is_object());
        assert_eq!(params["required"][0], "file_path");
    }

    #[tokio::test]
    async fn execute_missing_file_path_errors() {
        let tool = AudioTranscribeTool::with_stt(Arc::new(MockStt {
            text: String::new(),
        }));
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_nonexistent_file_errors() {
        let tool = AudioTranscribeTool::with_stt(Arc::new(MockStt {
            text: String::new(),
        }));
        let args = json!({"file_path": "/tmp/nonexistent_audio_file_12345.wav"});
        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_unsupported_extension_errors() {
        let tmp = std::env::temp_dir().join("test_audio_weft233.xyz");
        tokio::fs::write(&tmp, b"fake audio").await.unwrap();

        let tool = AudioTranscribeTool::with_stt(Arc::new(MockStt {
            text: String::new(),
        }));
        let args = json!({"file_path": tmp.to_string_lossy()});
        let result = tool.execute(args).await;
        assert!(result.is_err());

        let _ = tokio::fs::remove_file(&tmp).await;
    }

    #[tokio::test]
    async fn execute_valid_wav_decodes_and_transcribes() {
        let tmp = std::env::temp_dir().join("test_transcribe_weft233.wav");
        let pcm = fixture_wav(&tmp, 1600); // 100 ms @ 16 kHz
        assert!(!encode_wav(&pcm).is_empty());

        let tool = AudioTranscribeTool::with_stt(Arc::new(MockStt {
            text: "hello world".into(),
        }));
        let args = json!({
            "file_path": tmp.to_string_lossy(),
            "language": "en"
        });
        let result = tool.execute(args).await.unwrap();
        assert_eq!(result["status"], "transcribed");
        assert_eq!(result["mime_type"], "audio/wav");
        assert_eq!(result["language"], "en");
        assert_eq!(result["text"], "hello world");
        assert_eq!(result["source"], "local");
        assert_eq!(result["format"], "wav");
        assert!(result["duration_ms"].as_u64().unwrap() >= 90);

        let _ = tokio::fs::remove_file(&tmp).await;
    }

    #[tokio::test]
    async fn execute_corrupt_wav_decode_errors() {
        let tmp = std::env::temp_dir().join("test_transcribe_corrupt.wav");
        tokio::fs::write(&tmp, b"RIFF....not real wav data!!!!").await.unwrap();

        let tool = AudioTranscribeTool::with_stt(Arc::new(MockStt {
            text: "x".into(),
        }));
        let args = json!({"file_path": tmp.to_string_lossy()});
        let err = tool.execute(args).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("decode") || msg.contains("WAV") || msg.contains("probe") || msg.contains("audio"),
            "unexpected: {msg}"
        );

        let _ = tokio::fs::remove_file(&tmp).await;
    }
}
