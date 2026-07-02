//! Speaker identity → per-speaker node (ADR-061 §6, ADR-056).
//!
//! An ECAPA d-vector is extracted per utterance and matched against a
//! **persistent named registry**. Each identity **is** an ECC per-speaker
//! node (ADR-056 CrossRef): enrollment = node creation, `identify()` =
//! CrossRef resolve, and the running-mean centroid = the node's evolving
//! embedding. The matched speaker is fed to the LLM as *private* context
//! (never spoken). This is the multiplayer foundation — N speakers, each a
//! stable node, turns attributed by who spoke.
//!
//! Layering, consistent with the rest of the voice module:
//! - [`SpeakerEmbedder`] is the model seam (an ECAPA ONNX embedder drops in
//!   here; weights are out-of-band, so the live path is gated).
//! - [`SpeakerRegistry`] is pure, model-free logic: enroll / identify /
//!   attribute / persist. Each [`SpeakerNode`] maps 1:1 to the ECC
//!   per-speaker node the Talk-Mode controller creates on the kernel side.

use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::types::VoiceError;

/// Stable speaker identifier (the ECC per-speaker node id).
pub type SpeakerId = String;

/// Extracts a fixed-dimension speaker embedding (d-vector) from audio.
/// Implemented by an ECAPA model; the trait is the seam so the registry logic
/// stays model-free and testable.
#[async_trait]
pub trait SpeakerEmbedder: Send + Sync {
    /// Embed an utterance to a d-vector. Implementations should L2-normalize.
    async fn embed(&self, audio: &[i16], sample_rate: u32) -> Result<Vec<f32>, VoiceError>;
    /// Embedding dimensionality (ECAPA = 192).
    fn dim(&self) -> usize;
}

/// One enrolled speaker — the ECC per-speaker node's data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerNode {
    /// Stable node id.
    pub id: SpeakerId,
    /// Human-friendly name (private context for the LLM).
    pub name: String,
    /// Running-mean d-vector centroid (the node's evolving embedding).
    pub centroid: Vec<f32>,
    /// Number of utterances attributed to this speaker.
    pub utterances: u64,
}

/// Result of an [`SpeakerRegistry::identify`] hit.
#[derive(Debug, Clone)]
pub struct SpeakerMatch {
    /// Matched speaker id.
    pub id: SpeakerId,
    /// Matched speaker name.
    pub name: String,
    /// Cosine similarity of the query to the speaker centroid.
    pub score: f32,
}

/// Persistent named speaker registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerRegistry {
    /// Cosine acceptance threshold (ADR-061: ~0.45).
    threshold: f32,
    speakers: Vec<SpeakerNode>,
    #[serde(default)]
    next_seq: u64,
}

impl SpeakerRegistry {
    /// Build an empty registry with the given cosine acceptance threshold.
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            speakers: Vec::new(),
            next_seq: 0,
        }
    }

    /// Number of enrolled speakers.
    pub fn len(&self) -> usize {
        self.speakers.len()
    }
    /// Whether no speakers are enrolled.
    pub fn is_empty(&self) -> bool {
        self.speakers.is_empty()
    }
    /// Acceptance threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
    /// Borrow a speaker node by id.
    pub fn get(&self, id: &str) -> Option<&SpeakerNode> {
        self.speakers.iter().find(|s| s.id == id)
    }
    /// All enrolled nodes.
    pub fn nodes(&self) -> &[SpeakerNode] {
        &self.speakers
    }

    /// Enroll a new speaker (ECC per-speaker node creation). Returns the new
    /// node id. The embedding is L2-normalized as the initial centroid.
    pub fn enroll(&mut self, name: impl Into<String>, embedding: &[f32]) -> SpeakerId {
        let id = format!("spk-{}", self.next_seq);
        self.next_seq += 1;
        self.speakers.push(SpeakerNode {
            id: id.clone(),
            name: name.into(),
            centroid: normalize(embedding),
            utterances: 1,
        });
        id
    }

    /// Identify the speaker of an embedding (CrossRef resolve). Returns the
    /// best match at/above the threshold, else `None` (unknown speaker
    /// rejected).
    pub fn identify(&self, embedding: &[f32]) -> Option<SpeakerMatch> {
        let q = normalize(embedding);
        let mut best: Option<SpeakerMatch> = None;
        for s in &self.speakers {
            let score = cosine_normalized(&q, &s.centroid);
            if score >= self.threshold && best.as_ref().map(|b| score > b.score).unwrap_or(true) {
                best = Some(SpeakerMatch {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    score,
                });
            }
        }
        best
    }

    /// Attribute an utterance to a known speaker: fold the embedding into the
    /// running-mean centroid and bump the count. No-op if `id` is unknown.
    pub fn attribute(&mut self, id: &str, embedding: &[f32]) {
        let q = normalize(embedding);
        if let Some(s) = self.speakers.iter_mut().find(|s| s.id == id) {
            let n = s.utterances as f32;
            for (c, e) in s.centroid.iter_mut().zip(q.iter()) {
                *c = (*c * n + *e) / (n + 1.0);
            }
            s.centroid = normalize(&s.centroid);
            s.utterances += 1;
        }
    }

    /// Rename a known speaker (spoken self-enrollment: an unknown voice
    /// saying "my name is X" upgrades its auto-enrolled placeholder name).
    /// Returns `true` when `id` exists.
    pub fn rename(&mut self, id: &str, name: impl Into<String>) -> bool {
        if let Some(s) = self.speakers.iter_mut().find(|s| s.id == id) {
            s.name = name.into();
            true
        } else {
            false
        }
    }

    /// Identify-or-enroll convenience: returns `(id, true)` if a new speaker
    /// was enrolled, `(id, false)` if an existing one matched (and was
    /// attributed). `name_for_new` names a freshly enrolled speaker.
    pub fn identify_or_enroll(
        &mut self,
        embedding: &[f32],
        name_for_new: impl Into<String>,
    ) -> (SpeakerId, bool) {
        if let Some(m) = self.identify(embedding) {
            self.attribute(&m.id, embedding);
            (m.id, false)
        } else {
            (self.enroll(name_for_new, embedding), true)
        }
    }

    /// Persist the registry as JSON to `path`.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), VoiceError> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| VoiceError::Malformed(format!("registry encode: {e}")))?;
        std::fs::write(path, json).map_err(|e| VoiceError::Audio(format!("registry write: {e}")))
    }

    /// Load a registry from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, VoiceError> {
        let bytes =
            std::fs::read(path).map_err(|e| VoiceError::Audio(format!("registry read: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| VoiceError::Malformed(format!("registry decode: {e}")))
    }
}

/// L2-normalize a vector (zero vector → zero vector).
fn normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

/// Cosine similarity of two already-normalized vectors (dot product).
fn cosine_normalized(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Cosine similarity of two arbitrary vectors. Public for callers comparing
/// raw embeddings outside the registry.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    cosine_normalized(&normalize(a), &normalize(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Three roughly-orthogonal base directions standing in for d-vectors.
    fn alice() -> Vec<f32> {
        vec![1.0, 0.1, 0.0, 0.0]
    }
    fn alice_variant() -> Vec<f32> {
        vec![0.9, 0.2, 0.05, 0.0]
    }
    fn bob() -> Vec<f32> {
        vec![0.0, 0.0, 1.0, 0.1]
    }
    fn stranger() -> Vec<f32> {
        vec![0.0, 1.0, 0.0, 0.9]
    }

    #[test]
    fn enroll_then_identify_same_speaker() {
        let mut reg = SpeakerRegistry::new(0.45);
        let id = reg.enroll("Alice", &alice());
        let m = reg.identify(&alice_variant()).expect("should match Alice");
        assert_eq!(m.id, id);
        assert_eq!(m.name, "Alice");
        assert!(m.score >= 0.45);
    }

    #[test]
    fn unknown_speaker_rejected() {
        let mut reg = SpeakerRegistry::new(0.6);
        reg.enroll("Alice", &alice());
        reg.enroll("Bob", &bob());
        assert!(
            reg.identify(&stranger()).is_none(),
            "an unenrolled voice must be rejected"
        );
    }

    #[test]
    fn attribution_updates_centroid_and_count() {
        let mut reg = SpeakerRegistry::new(0.45);
        let id = reg.enroll("Alice", &alice());
        assert_eq!(reg.get(&id).unwrap().utterances, 1);
        reg.attribute(&id, &alice_variant());
        assert_eq!(reg.get(&id).unwrap().utterances, 2);
        // Still identifies Alice after the centroid moved.
        assert_eq!(reg.identify(&alice()).unwrap().id, id);
    }

    #[test]
    fn identify_or_enroll_flow() {
        let mut reg = SpeakerRegistry::new(0.45);
        let (id1, new1) = reg.identify_or_enroll(&alice(), "speaker-1");
        assert!(new1);
        let (id2, new2) = reg.identify_or_enroll(&alice_variant(), "speaker-2");
        assert!(!new2, "the variant should match the enrolled speaker");
        assert_eq!(id1, id2);
        let (_, new3) = reg.identify_or_enroll(&bob(), "speaker-3");
        assert!(new3, "a distinct voice should enroll fresh");
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn persistence_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("speakers.json");
        let mut reg = SpeakerRegistry::new(0.5);
        let id = reg.enroll("Alice", &alice());
        reg.attribute(&id, &alice_variant());
        reg.save(&path).unwrap();

        let loaded = SpeakerRegistry::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.threshold(), 0.5);
        let node = loaded.get(&id).unwrap();
        assert_eq!(node.name, "Alice");
        assert_eq!(node.utterances, 2);
        // The reloaded centroid still identifies Alice.
        assert_eq!(loaded.identify(&alice()).unwrap().id, id);
    }

    #[test]
    fn cosine_basics() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert!(cosine(&[0.0, 0.0], &[1.0, 1.0]).abs() < 1e-6);
    }
}
