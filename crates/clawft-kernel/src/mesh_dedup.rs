//! Message deduplication for mesh IPC (K6.3).
//!
//! Prevents duplicate delivery of messages that may arrive via
//! multiple paths in the mesh network. Uses a time-bounded set
//! of recently seen message/envelope IDs.
//!
//! # Status (WEFT-151 audit)
//!
//! **Orphan — library only.** No production callers outside this module
//! and re-exports (`clawft_kernel`, `weftos`). ADR-039 and K6 notes
//! describe `DedupFilter` as the intended SWIM/IPC dedup path, but
//! `MeshRuntime::handle_incoming` / `mesh_ipc` never call
//! [`DedupFilter::check_and_insert`]. Unit tests only.
//!
//! **Disposition:** keep; schedule into runtime receive path.
//! **Wiring schedule (0.8.x):** call `check_and_insert` on envelope
//! (or message) id inside `MeshRuntime` inbound handling (and any
//! future MeshAdapter). Intended default: [`DedupFilter::default_mesh`]
//! (60s TTL, 10k entries).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::mesh_clock::{Clock, MonoTime, RealClock};

/// Time-bounded deduplication filter.
///
/// Tracks recently seen IDs and automatically expires entries
/// older than the configured TTL. Uses an injectable [`Clock`]
/// (WEFT-113) so tests can advance TTL without sleeping.
pub struct DedupFilter {
    /// Seen IDs with their insertion time.
    seen: HashMap<String, MonoTime>,
    /// How long to remember IDs.
    ttl: Duration,
    /// Maximum number of entries before forced eviction.
    max_entries: usize,
    /// Injectable clock.
    clock: Arc<dyn Clock>,
}

impl DedupFilter {
    /// Create a new dedup filter with the given TTL and max capacity.
    ///
    /// Binds the production [`RealClock`].
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self::with_clock(ttl, max_entries, RealClock::shared())
    }

    /// Create a dedup filter with an injectable [`Clock`].
    pub fn with_clock(ttl: Duration, max_entries: usize, clock: Arc<dyn Clock>) -> Self {
        Self {
            seen: HashMap::new(),
            ttl,
            max_entries,
            clock,
        }
    }

    /// Create a dedup filter with default settings (60s TTL, 10000 entries).
    pub fn default_mesh() -> Self {
        Self::new(Duration::from_secs(60), 10_000)
    }

    /// Check if an ID has been seen before. If not, mark it as seen.
    /// Returns `true` if this is a **new** (not duplicate) message.
    pub fn check_and_insert(&mut self, id: &str) -> bool {
        self.evict_expired();

        if self.seen.contains_key(id) {
            return false; // duplicate
        }

        // Force eviction if at capacity
        if self.seen.len() >= self.max_entries {
            self.evict_oldest();
        }

        self.seen.insert(id.to_string(), self.clock.now());
        true // new message
    }

    /// Check if an ID has been seen (without inserting).
    pub fn is_duplicate(&self, id: &str) -> bool {
        if let Some(inserted) = self.seen.get(id) {
            self.clock.now().elapsed_since(*inserted) < self.ttl
        } else {
            false
        }
    }

    /// Number of tracked IDs.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Whether the filter is empty.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// Remove expired entries.
    fn evict_expired(&mut self) {
        let now = self.clock.now();
        let ttl = self.ttl;
        self.seen
            .retain(|_, inserted| now.elapsed_since(*inserted) < ttl);
    }

    /// Remove the oldest entry to make room for new ones.
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .seen
            .iter()
            .min_by_key(|(_, t)| **t)
            .map(|(k, _)| k.clone())
        {
            self.seen.remove(&oldest_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_clock::MockClock;

    #[test]
    fn new_message_returns_true() {
        let mut f = DedupFilter::new(Duration::from_secs(10), 100);
        assert!(f.check_and_insert("msg-1"));
    }

    #[test]
    fn duplicate_returns_false() {
        let mut f = DedupFilter::new(Duration::from_secs(10), 100);
        assert!(f.check_and_insert("msg-1"));
        assert!(!f.check_and_insert("msg-1"));
    }

    #[test]
    fn different_ids_both_new() {
        let mut f = DedupFilter::new(Duration::from_secs(10), 100);
        assert!(f.check_and_insert("a"));
        assert!(f.check_and_insert("b"));
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn is_duplicate_without_inserting() {
        let mut f = DedupFilter::new(Duration::from_secs(10), 100);
        assert!(!f.is_duplicate("x"));
        f.check_and_insert("x");
        assert!(f.is_duplicate("x"));
    }

    #[test]
    fn expired_entries_evicted() {
        let clock = MockClock::new();
        let mut f = DedupFilter::with_clock(
            Duration::from_secs(1),
            100,
            Arc::new(clock.clone()),
        );
        assert!(f.check_and_insert("old"));
        clock.advance(Duration::from_secs(2));
        // TTL elapsed → treated as new again.
        assert!(f.check_and_insert("old"));
    }

    #[test]
    fn max_capacity_evicts_oldest() {
        let clock = MockClock::new();
        let mut f =
            DedupFilter::with_clock(Duration::from_secs(60), 2, Arc::new(clock.clone()));
        f.check_and_insert("a");
        clock.advance(Duration::from_secs(1));
        f.check_and_insert("b");
        clock.advance(Duration::from_secs(1));
        // At capacity (2), inserting "c" should evict "a"
        f.check_and_insert("c");
        assert_eq!(f.len(), 2);
        assert!(!f.is_duplicate("a"));
        assert!(f.is_duplicate("b"));
        assert!(f.is_duplicate("c"));
    }

    #[test]
    fn empty_filter() {
        let f = DedupFilter::default_mesh();
        assert!(f.is_empty());
        assert_eq!(f.len(), 0);
    }

    #[test]
    fn default_mesh_settings() {
        let f = DedupFilter::default_mesh();
        assert_eq!(f.max_entries, 10_000);
        assert_eq!(f.ttl, Duration::from_secs(60));
    }
}
