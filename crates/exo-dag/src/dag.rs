//! Content-addressed append-only DAG store.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use exo_core::{blake3_hash, Blake3Hash, EventId, HybridLogicalClock, Hlc};

/// Contract version for the K6 ChainManager seam mapping (bump on wire break).
pub const K6_CHAIN_SEAM_VERSION: u32 = 1;

/// One DAG node: content-addressed event with optional parents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagEvent {
    /// Content id (`blake3` of canonical body).
    pub id: EventId,
    /// Hybrid logical clock stamp.
    pub hlc: Hlc,
    /// Emitting subsystem (maps to `ChainEvent.source`).
    pub source: String,
    /// Event kind (maps to `ChainEvent.kind`).
    pub kind: String,
    /// Opaque payload bytes (JSON / CBOR / raw).
    pub payload: Vec<u8>,
    /// Parent event ids (empty = genesis-style root).
    pub parents: Vec<EventId>,
}

impl DagEvent {
    /// Canonical bytes used for content addressing (excludes `id`).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.hlc.to_bytes());
        buf.extend_from_slice(self.source.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.kind.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&(self.payload.len() as u64).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        for p in &self.parents {
            buf.extend_from_slice(p.0.as_bytes());
        }
        buf
    }

    /// Compute content id for this event body.
    pub fn compute_id(&self) -> EventId {
        EventId::new(blake3_hash(&self.canonical_bytes()))
    }
}

#[derive(Default)]
struct Inner {
    by_id: HashMap<EventId, DagEvent>,
    /// Insertion order for deterministic iteration.
    order: Vec<EventId>,
}

/// In-memory append-only DAG (no external store / no postgres).
#[derive(Clone)]
pub struct DagStore {
    inner: Arc<RwLock<Inner>>,
    clock: Arc<HybridLogicalClock>,
}

impl Default for DagStore {
    fn default() -> Self {
        Self::new(HybridLogicalClock::new())
    }
}

impl DagStore {
    /// Create an empty store with the given HLC.
    pub fn new(clock: HybridLogicalClock) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::default())),
            clock: Arc::new(clock),
        }
    }

    /// Shared clock (for external stamping if needed).
    pub fn clock(&self) -> Arc<HybridLogicalClock> {
        Arc::clone(&self.clock)
    }

    /// Number of events stored.
    pub fn len(&self) -> usize {
        self.inner.read().order.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Lookup by content id.
    pub fn get(&self, id: &EventId) -> Option<DagEvent> {
        self.inner.read().by_id.get(id).cloned()
    }

    /// Append a new event. Parents must already exist (unless empty).
    pub fn append(
        &self,
        source: impl Into<String>,
        kind: impl Into<String>,
        payload: &[u8],
        parents: &[EventId],
    ) -> Result<DagEvent, DagError> {
        let source = source.into();
        let kind = kind.into();
        {
            let guard = self.inner.read();
            for p in parents {
                if !guard.by_id.contains_key(p) {
                    return Err(DagError::UnknownParent(p.to_hex()));
                }
            }
        }

        let hlc = self.clock.tick();
        let mut event = DagEvent {
            id: EventId::new(Blake3Hash::ZERO), // filled below
            hlc,
            source,
            kind,
            payload: payload.to_vec(),
            parents: parents.to_vec(),
        };
        event.id = event.compute_id();

        let mut guard = self.inner.write();
        if guard.by_id.contains_key(&event.id) {
            return Err(DagError::DuplicateEvent(event.id.to_hex()));
        }
        guard.order.push(event.id);
        guard.by_id.insert(event.id, event.clone());
        Ok(event)
    }

    /// All events in insertion order.
    pub fn events_in_order(&self) -> Vec<DagEvent> {
        let guard = self.inner.read();
        guard
            .order
            .iter()
            .filter_map(|id| guard.by_id.get(id).cloned())
            .collect()
    }

    /// Ancestors of `tip` (inclusive), topological insertion order among the set.
    pub fn ancestors_of(&self, tip: &EventId) -> Result<Vec<DagEvent>, DagError> {
        let guard = self.inner.read();
        if !guard.by_id.contains_key(tip) {
            return Err(DagError::UnknownEvent(tip.to_hex()));
        }
        let mut seen = HashSet::new();
        let mut q = VecDeque::new();
        q.push_back(*tip);
        while let Some(id) = q.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(ev) = guard.by_id.get(&id) {
                for p in &ev.parents {
                    q.push_back(*p);
                }
            }
        }
        let mut out: Vec<DagEvent> = guard
            .order
            .iter()
            .filter(|id| seen.contains(id))
            .filter_map(|id| guard.by_id.get(id).cloned())
            .collect();
        // stable: already insertion order
        let _ = &mut out;
        Ok(out)
    }
}

/// Checkpoint view loaded from a tip (SPARC `load_checkpoint`).
#[derive(Debug, Clone)]
pub struct Checkpoint {
    tip: EventId,
    events: Vec<DagEvent>,
}

impl Checkpoint {
    /// Tip event id.
    pub fn tip(&self) -> EventId {
        self.tip
    }

    /// Events reachable from tip, in store insertion order.
    pub fn events_in_order(&self) -> &[DagEvent] {
        &self.events
    }
}

/// Load a checkpoint rooted at `root_event_id` (tip).
pub fn load_checkpoint(dag: &DagStore, root_event_id: &EventId) -> Result<Checkpoint, DagError> {
    let events = dag.ancestors_of(root_event_id)?;
    Ok(Checkpoint {
        tip: *root_event_id,
        events,
    })
}

/// DAG store errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DagError {
    /// Parent id not present in the store.
    #[error("unknown parent event {0}")]
    UnknownParent(String),
    /// Requested event missing.
    #[error("unknown event {0}")]
    UnknownEvent(String),
    /// Content-address collision / re-append.
    #[error("duplicate event {0}")]
    DuplicateEvent(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_get() {
        let store = DagStore::default();
        let e = store.append("src", "kind", b"pay", &[]).unwrap();
        assert_eq!(store.get(&e.id).unwrap().payload, b"pay");
    }

    #[test]
    fn rejects_unknown_parent() {
        let store = DagStore::default();
        let fake = EventId::from_payload(b"missing");
        let err = store.append("s", "k", b"", &[fake]).unwrap_err();
        assert!(matches!(err, DagError::UnknownParent(_)));
    }

    #[test]
    fn checkpoint_includes_lineage() {
        let store = DagStore::default();
        let a = store.append("k", "a", b"1", &[]).unwrap();
        let b = store.append("k", "b", b"2", &[a.id]).unwrap();
        let c = store.append("k", "c", b"3", &[b.id]).unwrap();
        // sibling branch
        let _d = store.append("k", "d", b"4", &[a.id]).unwrap();
        let cp = load_checkpoint(&store, &c.id).unwrap();
        let kinds: Vec<_> = cp.events_in_order().iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, vec!["a", "b", "c"]);
    }
}
