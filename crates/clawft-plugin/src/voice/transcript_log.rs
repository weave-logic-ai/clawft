//! Voice session transcript logging.
//!
//! [`TranscriptLogger`] writes voice session transcripts to JSONL files
//! in the workspace directory. Each line is a JSON-serialized
//! [`TranscriptEntry`].
//!
//! # SC-2 / WEFT-223
//!
//! Transcript logs are **text-only by default**. Raw PCM is never
//! embedded in the JSONL. An optional `audio_path` field is only
//! written when the logger's [`AudioRetention`] is
//! [`AudioRetention::Persist`] **and** the caller supplies a path
//! produced by capture-side persistence. Session/None policies strip
//! any audio path before write.

use clawft_types::config::AudioRetention;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// A single entry in the voice transcript log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Speaker identifier ("user", "agent", or diarized speaker label).
    pub speaker: String,
    /// Transcribed or synthesized text.
    pub text: String,
    /// Source of transcription ("local", "cloud:openai-whisper", etc.).
    pub source: String,
    /// Confidence score (0.0-1.0) for STT entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Detected language code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Duration of the audio segment in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Optional path to persisted raw audio (SC-2).
    ///
    /// Only serialized when the logger's retention is
    /// [`AudioRetention::Persist`]. Never contains sample bytes —
    /// only a filesystem path under `.clawft/audio/`.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "audioPath")]
    pub audio_path: Option<String>,
}

/// Appends voice transcript entries to a JSONL file.
///
/// Log files are stored at: `{workspace}/.clawft/transcripts/{session_key}.jsonl`
///
/// This logger is append-only and does not require locking for
/// single-session use.
///
/// # Join key contract (WEFT-241)
///
/// The `session_id` passed to [`TranscriptLogger::new`] is the join
/// key against substrate-side transcripts produced by
/// `clawft-service-whisper`. The substrate publishes transcripts on
/// `substrate/_derived/transcript/<source-node-id>/mic` (see
/// `clawft_service_whisper::derive_source_node_from_path`); the
/// `<source-node-id>` segment is the canonical key.
///
/// To correlate an in-process [`TranscriptLogger`] log file with the
/// substrate transcript stream, callers MUST set
/// `session_id == <source-node-id>`. The sensor node publishing the
/// PCM owns this identifier; the agent / consumer reads it off the
/// transcript path. Any other choice (UUID per session, agent name,
/// etc.) breaks the join.
///
/// Per ADR-053, substrate-side `clawft-service-whisper` is the
/// canonical 0.7.0 STT path; this logger is the agent-side companion
/// that records consumed transcripts in the workspace.
///
/// # SC-2 (WEFT-223)
///
/// Construct with [`TranscriptLogger::with_retention`] (or the default
/// [`AudioRetention::None`] via [`new`](Self::new)) so raw-audio path
/// fields are gated. Text transcriptions are always allowed.
pub struct TranscriptLogger {
    path: PathBuf,
    retention: AudioRetention,
}

impl TranscriptLogger {
    /// Create a new logger for the given session.
    ///
    /// Creates the transcript directory if it does not exist.
    ///
    /// `session_id` should be the substrate `<source-node-id>` that
    /// produced the transcripts being recorded — see the type-level
    /// docs (WEFT-241) for the join-key contract.
    ///
    /// Retention defaults to [`AudioRetention::None`] (no raw-audio
    /// path fields written).
    pub fn new(workspace: &Path, session_id: &str) -> std::io::Result<Self> {
        Self::with_retention(workspace, session_id, AudioRetention::None)
    }

    /// Create a logger with an explicit SC-2 retention policy.
    pub fn with_retention(
        workspace: &Path,
        session_id: &str,
        retention: AudioRetention,
    ) -> std::io::Result<Self> {
        let dir = workspace.join(".clawft").join("transcripts");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{session_id}.jsonl"));
        Ok(Self { path, retention })
    }

    /// Current retention policy.
    pub fn retention(&self) -> AudioRetention {
        self.retention
    }

    /// Append a transcript entry to the log file.
    ///
    /// SC-2: strips `audio_path` unless retention is
    /// [`AudioRetention::Persist`]. Never writes sample bytes.
    pub async fn log(&self, entry: &TranscriptEntry) -> std::io::Result<()> {
        let mut entry = entry.clone();
        if !self.retention.allows_disk_write() {
            entry.audio_path = None;
        }
        let mut line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    /// Read all entries from the log file.
    pub async fn read_all(&self) -> std::io::Result<Vec<TranscriptEntry>> {
        let content = tokio::fs::read_to_string(&self.path).await?;
        let entries: Vec<TranscriptEntry> = content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(entries)
    }

    /// Path to the log file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        speaker: &str,
        text: &str,
        audio_path: Option<&str>,
    ) -> TranscriptEntry {
        TranscriptEntry {
            timestamp: "2026-02-24T12:00:00Z".into(),
            speaker: speaker.into(),
            text: text.into(),
            source: "local".into(),
            confidence: Some(0.95),
            language: Some("en".into()),
            duration_ms: Some(1500),
            audio_path: audio_path.map(str::to_string),
        }
    }

    #[test]
    fn transcript_entry_serde_roundtrip() {
        let entry = entry("user", "hello world", None);
        let json = serde_json::to_string(&entry).unwrap();
        let restored: TranscriptEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "hello world");
        assert_eq!(restored.speaker, "user");
        assert!((restored.confidence.unwrap() - 0.95).abs() < f32::EPSILON);
        assert!(restored.audio_path.is_none());
    }

    #[test]
    fn transcript_entry_optional_fields_omitted() {
        let entry = TranscriptEntry {
            timestamp: "2026-02-24T12:00:00Z".into(),
            speaker: "agent".into(),
            text: "hi".into(),
            source: "cloud:openai-whisper".into(),
            confidence: None,
            language: None,
            duration_ms: None,
            audio_path: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("confidence"));
        assert!(!json.contains("language"));
        assert!(!json.contains("duration_ms"));
        assert!(!json.contains("audio_path"));
    }

    #[tokio::test]
    async fn logger_write_and_read() {
        let tmp_dir = std::env::temp_dir().join("clawft_test_transcript");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let logger = TranscriptLogger::new(&tmp_dir, "test-session-001").unwrap();
        assert!(
            logger
                .path()
                .to_string_lossy()
                .contains("test-session-001.jsonl")
        );
        assert_eq!(logger.retention(), AudioRetention::None);

        // Write two entries
        let entry1 = entry("user", "what time is it", None);
        let entry2 = TranscriptEntry {
            timestamp: "2026-02-24T12:00:01Z".into(),
            speaker: "agent".into(),
            text: "it is noon".into(),
            source: "local".into(),
            confidence: None,
            language: None,
            duration_ms: None,
            audio_path: None,
        };

        logger.log(&entry1).await.unwrap();
        logger.log(&entry2).await.unwrap();

        // Read back
        let entries = logger.read_all().await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].speaker, "user");
        assert_eq!(entries[0].text, "what time is it");
        assert_eq!(entries[1].speaker, "agent");
        assert_eq!(entries[1].text, "it is noon");

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn logger_creates_directory() {
        let tmp_dir = std::env::temp_dir().join("clawft_test_transcript_dir");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let logger = TranscriptLogger::new(&tmp_dir, "dir-test").unwrap();
        assert!(logger.path().parent().unwrap().exists());

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[tokio::test]
    async fn retention_none_strips_audio_path() {
        let tmp_dir = std::env::temp_dir().join("clawft_test_transcript_sc2_none");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let logger = TranscriptLogger::new(&tmp_dir, "sess-none").unwrap();
        let e = entry(
            "user",
            "secret utterance",
            Some("/tmp/.clawft/audio/should-not-appear.pcm"),
        );
        logger.log(&e).await.unwrap();

        let raw = tokio::fs::read_to_string(logger.path()).await.unwrap();
        assert!(!raw.contains("audio_path"));
        assert!(!raw.contains("should-not-appear"));
        assert!(raw.contains("secret utterance"));

        let entries = logger.read_all().await.unwrap();
        assert!(entries[0].audio_path.is_none());

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[tokio::test]
    async fn retention_persist_keeps_audio_path() {
        let tmp_dir = std::env::temp_dir().join("clawft_test_transcript_sc2_persist");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let logger =
            TranscriptLogger::with_retention(&tmp_dir, "sess-persist", AudioRetention::Persist)
                .unwrap();
        let e = entry(
            "user",
            "kept",
            Some("/workspace/.clawft/audio/utt-1.pcm"),
        );
        logger.log(&e).await.unwrap();

        let entries = logger.read_all().await.unwrap();
        assert_eq!(
            entries[0].audio_path.as_deref(),
            Some("/workspace/.clawft/audio/utt-1.pcm")
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[tokio::test]
    async fn retention_session_strips_audio_path() {
        // Session = memory only; transcript writer must not reference disk audio.
        let tmp_dir = std::env::temp_dir().join("clawft_test_transcript_sc2_session");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let logger =
            TranscriptLogger::with_retention(&tmp_dir, "sess-session", AudioRetention::Session)
                .unwrap();
        let e = entry("user", "session hold", Some("/x/y.pcm"));
        logger.log(&e).await.unwrap();
        let entries = logger.read_all().await.unwrap();
        assert!(entries[0].audio_path.is_none());

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
