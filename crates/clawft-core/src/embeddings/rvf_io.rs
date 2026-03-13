//! RVF Segment I/O -- local serialization for vector memory (H2.3).
//!
//! Provides segment-based persistence for memory entries including vector
//! data, metadata, and timestamps. Uses a JSON-based format inspired by
//! the RVF segment model (each entry is a "segment" with a header).
//!
//! The `rvf-runtime` 0.2 API provides full segment I/O, but is tightly
//! coupled to its binary format. This module implements a local fallback
//! using `rvf-types` as the conceptual model, serializing to a portable
//! JSON format that integrates with the HNSW store and WITNESS chain.
//!
//! # File layout
//!
//! Each `.rvf.json` file contains a `SegmentFile` with a header and a
//! list of `MemorySegment` entries. The WITNESS chain (if present) is
//! stored inline.
//!
//! This module is gated behind the `rvf` feature flag.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::witness::WitnessChain;

/// File format version.
const FORMAT_VERSION: u32 = 1;

/// Default file extension for RVF segment files.
pub const RVF_EXTENSION: &str = "rvf.json";

// ── Segment types ───────────────────────────────────────────────────

/// The type of a memory segment (mirrors rvf-types SegmentType concept).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySegmentType {
    /// A vector embedding with metadata.
    Vector,
    /// A text-only entry (no embedding yet).
    Text,
    /// A policy routing entry.
    Policy,
}

/// A single memory segment containing vector data and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySegment {
    /// Unique identifier for this segment.
    pub id: String,
    /// Segment type.
    pub segment_type: MemorySegmentType,
    /// The namespace this segment belongs to.
    pub namespace: String,
    /// The embedding vector (empty for text-only segments).
    pub embedding: Vec<f32>,
    /// The original text content.
    pub text: String,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// When this segment was created.
    pub created_at: DateTime<Utc>,
    /// When this segment was last updated.
    pub updated_at: DateTime<Utc>,
    /// Embedding dimension (0 if no embedding).
    pub dimension: u32,
    /// The embedder name that produced the embedding.
    pub embedder_name: String,
}

/// File header for a segment file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentFileHeader {
    /// Format version.
    pub version: u32,
    /// The agent ID that owns this file.
    pub agent_id: String,
    /// The namespace.
    pub namespace: String,
    /// When this file was created.
    pub created_at: DateTime<Utc>,
    /// When this file was last modified.
    pub modified_at: DateTime<Utc>,
    /// Total number of segments in the file.
    pub segment_count: usize,
    /// Embedding dimension for this file (0 if mixed).
    pub dimension: u32,
    /// Embedder name used for this file.
    pub embedder_name: String,
}

/// A complete segment file with header, segments, and optional WITNESS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentFile {
    /// File header.
    pub header: SegmentFileHeader,
    /// The memory segments.
    pub segments: Vec<MemorySegment>,
    /// Optional WITNESS chain for tamper detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_chain: Option<WitnessChain>,
}

// ── Errors ──────────────────────────────────────────────────────────

/// Errors from RVF segment I/O.
#[derive(Debug)]
pub enum RvfIoError {
    /// An I/O error occurred.
    Io(std::io::Error),
    /// A serialization/deserialization error occurred.
    Serde(serde_json::Error),
    /// The file format version is unsupported.
    UnsupportedVersion(u32),
    /// The WITNESS chain verification failed.
    WitnessInvalid(String),
}

impl std::fmt::Display for RvfIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RvfIoError::Io(e) => write!(f, "RVF I/O error: {e}"),
            RvfIoError::Serde(e) => write!(f, "RVF serde error: {e}"),
            RvfIoError::UnsupportedVersion(v) => {
                write!(f, "unsupported RVF format version: {v}")
            }
            RvfIoError::WitnessInvalid(msg) => {
                write!(f, "RVF WITNESS validation failed: {msg}")
            }
        }
    }
}

impl std::error::Error for RvfIoError {}

impl From<std::io::Error> for RvfIoError {
    fn from(e: std::io::Error) -> Self {
        RvfIoError::Io(e)
    }
}

impl From<serde_json::Error> for RvfIoError {
    fn from(e: serde_json::Error) -> Self {
        RvfIoError::Serde(e)
    }
}

// ── Builder ─────────────────────────────────────────────────────────

/// Builder for creating a new segment file.
pub struct SegmentFileBuilder {
    agent_id: String,
    namespace: String,
    dimension: u32,
    embedder_name: String,
    segments: Vec<MemorySegment>,
    witness_chain: Option<WitnessChain>,
}

impl SegmentFileBuilder {
    /// Create a new builder.
    pub fn new(agent_id: &str, namespace: &str) -> Self {
        Self {
            agent_id: agent_id.to_owned(),
            namespace: namespace.to_owned(),
            dimension: 0,
            embedder_name: String::new(),
            segments: Vec::new(),
            witness_chain: None,
        }
    }

    /// Set the embedding dimension.
    pub fn dimension(mut self, dim: u32) -> Self {
        self.dimension = dim;
        self
    }

    /// Set the embedder name.
    pub fn embedder_name(mut self, name: &str) -> Self {
        self.embedder_name = name.to_owned();
        self
    }

    /// Add a segment.
    pub fn add_segment(mut self, segment: MemorySegment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Add multiple segments.
    pub fn add_segments(
        mut self,
        segments: impl IntoIterator<Item = MemorySegment>,
    ) -> Self {
        self.segments.extend(segments);
        self
    }

    /// Attach a WITNESS chain.
    pub fn witness_chain(mut self, chain: WitnessChain) -> Self {
        self.witness_chain = Some(chain);
        self
    }

    /// Build the segment file.
    pub fn build(self) -> SegmentFile {
        let now = Utc::now();
        SegmentFile {
            header: SegmentFileHeader {
                version: FORMAT_VERSION,
                agent_id: self.agent_id,
                namespace: self.namespace,
                created_at: now,
                modified_at: now,
                segment_count: self.segments.len(),
                dimension: self.dimension,
                embedder_name: self.embedder_name,
            },
            segments: self.segments,
            witness_chain: self.witness_chain,
        }
    }
}

// ── I/O functions ───────────────────────────────────────────────────

/// Write a segment file to disk.
pub fn write_segment_file(
    file: &SegmentFile,
    path: &Path,
) -> Result<(), RvfIoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(file)?;
    std::fs::write(path, json)?;

    debug!(
        path = %path.display(),
        segments = file.segments.len(),
        "wrote segment file"
    );
    Ok(())
}

/// Read a segment file from disk.
pub fn read_segment_file(path: &Path) -> Result<SegmentFile, RvfIoError> {
    let data = std::fs::read_to_string(path)?;
    let file: SegmentFile = serde_json::from_str(&data)?;

    if file.header.version > FORMAT_VERSION {
        return Err(RvfIoError::UnsupportedVersion(file.header.version));
    }

    debug!(
        path = %path.display(),
        segments = file.segments.len(),
        version = file.header.version,
        "read segment file"
    );
    Ok(file)
}

/// Read and verify a segment file, checking the WITNESS chain if present.
pub fn read_verified_segment_file(
    path: &Path,
) -> Result<SegmentFile, RvfIoError> {
    let file = read_segment_file(path)?;

    if let Some(ref chain) = file.witness_chain
        && !chain.verify()
    {
        return Err(RvfIoError::WitnessInvalid(
            "WITNESS chain verification failed".into(),
        ));
    }

    Ok(file)
}

/// Resolve the segment file path for a given agent and namespace.
///
/// Returns `~/.clawft/agents/<agent_id>/memory/<namespace>.rvf.json`.
pub fn segment_file_path(
    agent_id: &str,
    namespace: &str,
) -> Option<PathBuf> {
    #[cfg(feature = "native")]
    { dirs::home_dir().map(|h| {
        h.join(".clawft")
            .join("agents")
            .join(agent_id)
            .join("memory")
            .join(format!("{namespace}.{RVF_EXTENSION}"))
    }) }
    #[cfg(not(feature = "native"))]
    { Some(PathBuf::from(".clawft")
        .join("agents")
        .join(agent_id)
        .join("memory")
        .join(format!("{namespace}.{RVF_EXTENSION}"))) }
}

/// Create a [`MemorySegment`] from raw components.
pub fn make_segment(
    id: &str,
    text: &str,
    embedding: Vec<f32>,
    metadata: serde_json::Value,
    tags: Vec<String>,
    namespace: &str,
    embedder_name: &str,
) -> MemorySegment {
    let now = Utc::now();
    let dim = embedding.len() as u32;
    let seg_type = if embedding.is_empty() {
        MemorySegmentType::Text
    } else {
        MemorySegmentType::Vector
    };

    MemorySegment {
        id: id.to_owned(),
        segment_type: seg_type,
        namespace: namespace.to_owned(),
        embedding,
        text: text.to_owned(),
        metadata,
        tags,
        created_at: now,
        updated_at: now,
        dimension: dim,
        embedder_name: embedder_name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!(
            "clawft_rvf_io_test_{label}_{pid}_{n}.rvf.json"
        ))
    }

    #[test]
    fn builder_creates_empty_file() {
        let file = SegmentFileBuilder::new("agent-1", "default")
            .dimension(384)
            .embedder_name("hash")
            .build();

        assert_eq!(file.header.version, FORMAT_VERSION);
        assert_eq!(file.header.agent_id, "agent-1");
        assert_eq!(file.header.namespace, "default");
        assert_eq!(file.header.segment_count, 0);
        assert!(file.segments.is_empty());
        assert!(file.witness_chain.is_none());
    }

    #[test]
    fn builder_with_segments() {
        let seg = make_segment(
            "s1",
            "hello world",
            vec![1.0, 0.0, 0.0],
            serde_json::json!({"key": "value"}),
            vec!["tag1".into()],
            "default",
            "hash",
        );

        let file = SegmentFileBuilder::new("agent-1", "default")
            .dimension(3)
            .add_segment(seg)
            .build();

        assert_eq!(file.header.segment_count, 1);
        assert_eq!(file.segments.len(), 1);
        assert_eq!(file.segments[0].id, "s1");
        assert_eq!(file.segments[0].text, "hello world");
    }

    #[test]
    fn write_and_read_roundtrip() {
        let path = temp_path("roundtrip");
        let _ = std::fs::remove_file(&path);

        let seg = make_segment(
            "doc1",
            "test document",
            vec![0.5, 0.5],
            serde_json::json!({}),
            vec![],
            "default",
            "hash",
        );

        let file = SegmentFileBuilder::new("agent-1", "default")
            .dimension(2)
            .embedder_name("hash")
            .add_segment(seg)
            .build();

        write_segment_file(&file, &path).unwrap();
        let loaded = read_segment_file(&path).unwrap();

        assert_eq!(loaded.header.agent_id, "agent-1");
        assert_eq!(loaded.segments.len(), 1);
        assert_eq!(loaded.segments[0].id, "doc1");
        assert_eq!(loaded.segments[0].embedding, vec![0.5, 0.5]);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_and_read_with_witness() {
        let path = temp_path("witness");
        let _ = std::fs::remove_file(&path);

        let mut chain = WitnessChain::new();
        chain.append(
            super::super::witness::WitnessOperation::Store,
            b"doc1",
        );

        let seg = make_segment(
            "doc1",
            "witnessed",
            vec![1.0],
            serde_json::json!({}),
            vec![],
            "default",
            "hash",
        );

        let file = SegmentFileBuilder::new("agent-1", "default")
            .add_segment(seg)
            .witness_chain(chain)
            .build();

        write_segment_file(&file, &path).unwrap();

        let loaded = read_verified_segment_file(&path).unwrap();
        assert!(loaded.witness_chain.is_some());
        assert!(loaded.witness_chain.as_ref().unwrap().verify());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_rejects_tampered_witness() {
        let path = temp_path("tampered");
        let _ = std::fs::remove_file(&path);

        let mut chain = WitnessChain::new();
        chain.append(
            super::super::witness::WitnessOperation::Store,
            b"data",
        );

        let file = SegmentFileBuilder::new("agent-1", "default")
            .witness_chain(chain)
            .build();

        write_segment_file(&file, &path).unwrap();

        // Tamper with the witness chain in the file.
        let mut loaded = read_segment_file(&path).unwrap();
        if let Some(ref mut chain) = loaded.witness_chain {
            chain.segments_mut()[0].data_hash[0] ^= 0xFF;
        }
        let tampered_json = serde_json::to_string_pretty(&loaded).unwrap();
        std::fs::write(&path, tampered_json).unwrap();

        let result = read_verified_segment_file(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn make_segment_vector_type() {
        let seg = make_segment(
            "v1",
            "text",
            vec![1.0, 2.0],
            serde_json::json!({}),
            vec![],
            "ns",
            "hash",
        );
        assert_eq!(seg.segment_type, MemorySegmentType::Vector);
        assert_eq!(seg.dimension, 2);
    }

    #[test]
    fn make_segment_text_type() {
        let seg = make_segment(
            "t1",
            "text only",
            vec![],
            serde_json::json!({}),
            vec![],
            "ns",
            "hash",
        );
        assert_eq!(seg.segment_type, MemorySegmentType::Text);
        assert_eq!(seg.dimension, 0);
    }

    #[test]
    fn segment_file_path_format() {
        let path = segment_file_path("my-agent", "memories").unwrap();
        let s = path.to_string_lossy();
        assert!(s.contains("my-agent"));
        assert!(s.contains("memory"));
        assert!(s.ends_with("memories.rvf.json"));
    }

    #[test]
    fn error_display() {
        let err = RvfIoError::UnsupportedVersion(99);
        assert!(format!("{err}").contains("99"));

        let err = RvfIoError::WitnessInvalid("bad chain".into());
        assert!(format!("{err}").contains("bad chain"));
    }

    #[test]
    fn read_nonexistent_errors() {
        let result = read_segment_file(Path::new("/nonexistent/path.json"));
        assert!(result.is_err());
    }
}
