//! Ephemeral causal impulse queue for inter-structure communication (ECC Phase K3c).
//!
//! Impulses are short-lived events that flow between the four ECC structures
//! (causal graph, spectral index, HNSW, cloud/edge bridge). The [`ImpulseQueue`]
//! provides a thread-safe, ordered buffer that producers [`emit`](ImpulseQueue::emit)
//! into and consumers [`drain_ready`](ImpulseQueue::drain_ready) from.
//!
//! Structure tags are represented as raw `u8` values to avoid cross-module
//! coupling. They correspond to `crossref::StructureTag::as_u8()`.

use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ImpulseType
// ---------------------------------------------------------------------------

/// Discriminant for the kind of causal event being signalled.
///
/// Variants carry a canonical *byte code* ([`code`](ImpulseType::code)) used as a
/// stable wire/semantic tag — analogous to [`crossref::StructureTag::as_u8`].
/// The byte code is **not** the Rust enum discriminant and serde tags this enum
/// **by variant name** (externally tagged); adding variants is therefore
/// backward-compatible with existing serialized impulses and `Custom(u8)` stays
/// the open extension point.
///
/// [`crossref::StructureTag::as_u8`]: crate::crossref::StructureTag::as_u8
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpulseType {
    /// causal -> hnsw (new embedding needed)
    BeliefUpdate,
    /// spectral -> causal (graph incoherent)
    CoherenceAlert,
    /// hnsw -> causal (new cluster found)
    NoveltyDetected,
    /// cloud -> edge (DEMOCRITUS validated edge)
    EdgeConfirmed,
    /// cloud -> edge (better embedding available)
    EmbeddingRefined,
    /// Turn-taking: a speaker claims/asserts the floor — barge-in (0x50, ADR-062 D5).
    TurnClaim,
    /// Turn-taking: end-of-utterance — triggers the Frontier→Committed commit (0x51, ADR-062 D5).
    EndOfUtterance,
    /// Turn-taking: floor released; the next utterance commits a `Follows` (0x52, ADR-062 D5).
    TurnShift,
    /// Turn-taking: a backchannel ("mm-hmm") — becomes a `Continuer` cross-ref,
    /// never a turn node (0x60, ADR-062 D5).
    Backchannel,
    /// Extension point for user-defined impulse kinds.
    Custom(u8),
}

impl ImpulseType {
    /// Canonical byte code for this impulse kind (stable semantic/wire tag).
    ///
    /// Turn-taking codes follow ADR-062 D5: `TurnClaim` 0x50, `EndOfUtterance`
    /// 0x51, `TurnShift` 0x52, `Backchannel` 0x60. `Custom(c)` returns `c`.
    pub fn code(&self) -> u8 {
        match self {
            Self::BeliefUpdate => 0x01,
            Self::CoherenceAlert => 0x02,
            Self::NoveltyDetected => 0x03,
            Self::EdgeConfirmed => 0x04,
            Self::EmbeddingRefined => 0x05,
            Self::TurnClaim => 0x50,
            Self::EndOfUtterance => 0x51,
            Self::TurnShift => 0x52,
            Self::Backchannel => 0x60,
            Self::Custom(c) => *c,
        }
    }
}

impl fmt::Display for ImpulseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeliefUpdate => write!(f, "BeliefUpdate"),
            Self::CoherenceAlert => write!(f, "CoherenceAlert"),
            Self::NoveltyDetected => write!(f, "NoveltyDetected"),
            Self::EdgeConfirmed => write!(f, "EdgeConfirmed"),
            Self::EmbeddingRefined => write!(f, "EmbeddingRefined"),
            Self::TurnClaim => write!(f, "TurnClaim"),
            Self::EndOfUtterance => write!(f, "EndOfUtterance"),
            Self::TurnShift => write!(f, "TurnShift"),
            Self::Backchannel => write!(f, "Backchannel"),
            Self::Custom(code) => write!(f, "Custom({code})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Impulse
// ---------------------------------------------------------------------------

/// A single causal event travelling between ECC structures.
///
/// `source_structure` and `target_structure` are `u8` tags that correspond to
/// `crossref::StructureTag::as_u8()` values:
///   0 = CausalGraph, 1 = SpectralIndex, 2 = Hnsw, 3 = CloudBridge.
pub struct Impulse {
    /// Monotonically increasing identifier assigned by the queue.
    pub id: u64,
    /// Originating structure (see `StructureTag::as_u8()`).
    pub source_structure: u8,
    /// 32-byte universal node identifier from the source structure.
    pub source_node: [u8; 32],
    /// Destination structure (see `StructureTag::as_u8()`).
    pub target_structure: u8,
    /// The kind of impulse.
    pub impulse_type: ImpulseType,
    /// Arbitrary JSON payload carried by this impulse.
    pub payload: serde_json::Value,
    /// Hybrid-logical-clock timestamp for causal ordering.
    pub hlc_timestamp: u64,
    /// Set to `true` once the consumer has processed this impulse.
    pub acknowledged: AtomicBool,
}

// AtomicBool is not Clone, so we implement Clone manually.
impl Clone for Impulse {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            source_structure: self.source_structure,
            source_node: self.source_node,
            target_structure: self.target_structure,
            impulse_type: self.impulse_type.clone(),
            payload: self.payload.clone(),
            hlc_timestamp: self.hlc_timestamp,
            acknowledged: AtomicBool::new(self.acknowledged.load(Ordering::Acquire)),
        }
    }
}

impl fmt::Debug for Impulse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Impulse")
            .field("id", &self.id)
            .field("source_structure", &self.source_structure)
            .field("target_structure", &self.target_structure)
            .field("impulse_type", &self.impulse_type)
            .field("hlc_timestamp", &self.hlc_timestamp)
            .field("acknowledged", &self.acknowledged.load(Ordering::Relaxed))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ImpulseQueue
// ---------------------------------------------------------------------------

/// Thread-safe queue of [`Impulse`] events awaiting consumption.
pub struct ImpulseQueue {
    queue: Mutex<Vec<Impulse>>,
    next_id: AtomicU64,
}

impl ImpulseQueue {
    /// Create a new, empty impulse queue.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Enqueue a new impulse and return its assigned id.
    pub fn emit(
        &self,
        source_structure: u8,
        source_node: [u8; 32],
        target_structure: u8,
        impulse_type: ImpulseType,
        payload: serde_json::Value,
        hlc_timestamp: u64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let impulse = Impulse {
            id,
            source_structure,
            source_node,
            target_structure,
            impulse_type,
            payload,
            hlc_timestamp,
            acknowledged: AtomicBool::new(false),
        };
        let mut q = self.queue.lock().expect("impulse queue poisoned");
        q.push(impulse);
        id
    }

    /// Drain all unacknowledged impulses, returning them sorted by
    /// `hlc_timestamp` (ascending). Acknowledged impulses are discarded.
    pub fn drain_ready(&self) -> Vec<Impulse> {
        let mut q = self.queue.lock().expect("impulse queue poisoned");
        let drained: Vec<Impulse> = q
            .drain(..)
            .filter(|imp| !imp.acknowledged.load(Ordering::Acquire))
            .collect();
        let mut sorted = drained;
        sorted.sort_by_key(|imp| imp.hlc_timestamp);
        sorted
    }

    /// Total number of impulses in the queue (acknowledged or not).
    pub fn len(&self) -> usize {
        self.queue.lock().expect("impulse queue poisoned").len()
    }

    /// Returns `true` if the queue contains no impulses.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove all impulses from the queue (e.g. during calibration).
    pub fn clear(&self) {
        self.queue.lock().expect("impulse queue poisoned").clear();
    }

    /// Count of impulses that have not yet been acknowledged.
    pub fn pending_count(&self) -> usize {
        self.queue
            .lock()
            .expect("impulse queue poisoned")
            .iter()
            .filter(|imp| !imp.acknowledged.load(Ordering::Acquire))
            .count()
    }
}

impl Default for ImpulseQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_type_display() {
        assert_eq!(ImpulseType::BeliefUpdate.to_string(), "BeliefUpdate");
        assert_eq!(ImpulseType::CoherenceAlert.to_string(), "CoherenceAlert");
        assert_eq!(ImpulseType::NoveltyDetected.to_string(), "NoveltyDetected");
        assert_eq!(ImpulseType::EdgeConfirmed.to_string(), "EdgeConfirmed");
        assert_eq!(
            ImpulseType::EmbeddingRefined.to_string(),
            "EmbeddingRefined"
        );
        assert_eq!(ImpulseType::Custom(42).to_string(), "Custom(42)");
    }

    #[test]
    fn impulse_queue_new_empty() {
        let q = ImpulseQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert_eq!(q.pending_count(), 0);
    }

    #[test]
    fn impulse_queue_emit_assigns_id() {
        let q = ImpulseQueue::new();
        let node = [0u8; 32];
        let id1 = q.emit(
            0,
            node,
            2,
            ImpulseType::BeliefUpdate,
            serde_json::json!({}),
            100,
        );
        let id2 = q.emit(
            1,
            node,
            0,
            ImpulseType::CoherenceAlert,
            serde_json::json!({}),
            200,
        );
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn impulse_queue_drain_sorted_by_hlc() {
        let q = ImpulseQueue::new();
        let node = [0u8; 32];
        // Emit in reverse timestamp order.
        q.emit(
            0,
            node,
            2,
            ImpulseType::BeliefUpdate,
            serde_json::json!({}),
            300,
        );
        q.emit(
            1,
            node,
            0,
            ImpulseType::CoherenceAlert,
            serde_json::json!({}),
            100,
        );
        q.emit(
            2,
            node,
            0,
            ImpulseType::NoveltyDetected,
            serde_json::json!({}),
            200,
        );

        let drained = q.drain_ready();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0].hlc_timestamp, 100);
        assert_eq!(drained[1].hlc_timestamp, 200);
        assert_eq!(drained[2].hlc_timestamp, 300);
    }

    #[test]
    fn impulse_queue_drain_removes_items() {
        let q = ImpulseQueue::new();
        let node = [0u8; 32];
        q.emit(
            0,
            node,
            2,
            ImpulseType::BeliefUpdate,
            serde_json::json!({}),
            1,
        );
        q.emit(
            0,
            node,
            2,
            ImpulseType::EdgeConfirmed,
            serde_json::json!({}),
            2,
        );
        assert_eq!(q.len(), 2);

        let drained = q.drain_ready();
        assert_eq!(drained.len(), 2);
        assert!(q.is_empty());
    }

    #[test]
    fn impulse_queue_clear() {
        let q = ImpulseQueue::new();
        let node = [0u8; 32];
        q.emit(
            0,
            node,
            1,
            ImpulseType::EmbeddingRefined,
            serde_json::json!({}),
            10,
        );
        q.emit(
            0,
            node,
            1,
            ImpulseType::Custom(7),
            serde_json::json!({}),
            20,
        );
        assert_eq!(q.len(), 2);

        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn impulse_queue_pending_count() {
        let q = ImpulseQueue::new();
        let node = [0u8; 32];
        q.emit(
            0,
            node,
            2,
            ImpulseType::BeliefUpdate,
            serde_json::json!({}),
            1,
        );
        q.emit(
            1,
            node,
            0,
            ImpulseType::CoherenceAlert,
            serde_json::json!({}),
            2,
        );
        assert_eq!(q.pending_count(), 2);

        // Acknowledge one via the internal queue.
        {
            let guard = q.queue.lock().unwrap();
            guard[0].acknowledged.store(true, Ordering::Release);
        }
        assert_eq!(q.pending_count(), 1);
    }

    #[test]
    fn turn_impulse_codes_and_display() {
        // ADR-062 D5 canonical byte codes.
        assert_eq!(ImpulseType::TurnClaim.code(), 0x50);
        assert_eq!(ImpulseType::EndOfUtterance.code(), 0x51);
        assert_eq!(ImpulseType::TurnShift.code(), 0x52);
        assert_eq!(ImpulseType::Backchannel.code(), 0x60);
        assert_eq!(ImpulseType::Custom(0x77).code(), 0x77);

        assert_eq!(ImpulseType::TurnClaim.to_string(), "TurnClaim");
        assert_eq!(ImpulseType::EndOfUtterance.to_string(), "EndOfUtterance");
        assert_eq!(ImpulseType::TurnShift.to_string(), "TurnShift");
        assert_eq!(ImpulseType::Backchannel.to_string(), "Backchannel");
    }

    #[test]
    fn turn_impulse_serde_roundtrip_by_name() {
        // serde tags by variant name → stable across added variants.
        let json = serde_json::to_string(&ImpulseType::EndOfUtterance).unwrap();
        assert_eq!(json, "\"EndOfUtterance\"");
        let back: ImpulseType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ImpulseType::EndOfUtterance);
        // Pre-existing variants still round-trip unchanged.
        let custom = serde_json::to_string(&ImpulseType::Custom(7)).unwrap();
        assert_eq!(custom, "{\"Custom\":7}");
    }

    #[test]
    fn turn_impulses_drain_hlc_ordered() {
        // A realistic turn: backchannel mid-utterance, then a barge-in claim,
        // then EOU — emitted out of HLC order, must drain in HLC order.
        let q = ImpulseQueue::new();
        let speaker = [9u8; 32];
        // 0 = CausalGraph tag (see StructureTag::as_u8).
        q.emit(0, speaker, 0, ImpulseType::EndOfUtterance, json_payload(), 300);
        q.emit(0, speaker, 0, ImpulseType::Backchannel, json_payload(), 100);
        q.emit(0, speaker, 0, ImpulseType::TurnClaim, json_payload(), 200);

        let drained = q.drain_ready();
        assert_eq!(drained.len(), 3);
        // HLC ascending.
        assert_eq!(drained[0].impulse_type, ImpulseType::Backchannel);
        assert_eq!(drained[0].hlc_timestamp, 100);
        assert_eq!(drained[1].impulse_type, ImpulseType::TurnClaim);
        assert_eq!(drained[1].hlc_timestamp, 200);
        assert_eq!(drained[2].impulse_type, ImpulseType::EndOfUtterance);
        assert_eq!(drained[2].hlc_timestamp, 300);
    }

    fn json_payload() -> serde_json::Value {
        serde_json::json!({})
    }

    #[test]
    fn impulse_emit_and_acknowledge() {
        let q = ImpulseQueue::new();
        let node = [1u8; 32];
        q.emit(
            0,
            node,
            2,
            ImpulseType::NoveltyDetected,
            serde_json::json!({"k": "v"}),
            50,
        );
        q.emit(
            0,
            node,
            3,
            ImpulseType::EdgeConfirmed,
            serde_json::json!(null),
            60,
        );

        let drained = q.drain_ready();
        assert_eq!(drained.len(), 2);
        // Queue is now empty after drain.
        assert!(q.is_empty());

        // Mark drained impulses as acknowledged.
        for imp in &drained {
            imp.acknowledged.store(true, Ordering::Release);
        }

        // Verify acknowledgement persists on the drained copies.
        for imp in &drained {
            assert!(imp.acknowledged.load(Ordering::Acquire));
        }
    }
}
