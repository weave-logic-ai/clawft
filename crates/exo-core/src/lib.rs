//! # exo-core — K6 substrate primitives (ADR-043 / WEFT-619)
//!
//! In-tree implementation of the **exo-core contract** used by SPARC docs
//! (`Blake3Hash`, domain-separated hashing, `HybridLogicalClock`, `EventId`).
//!
//! This crate intentionally does **not** vendor third-party exo-core source
//! (FSL / license clearance deferred). API names match the K6 migration path
//! so `exo-dag` and future `ChainManager` cutover can depend on it.
//!
//! ## Target support
//!
//! Pure Rust + `blake3` — builds for native and `wasm32` (no OS clocks required
//! for deterministic hashing; wall-clock HLC ticks use `std::time` when available).

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 32-byte BLAKE3 digest used as a content-addressed identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; 32]);

impl Blake3Hash {
    /// Zero / empty digest (genesis / unset).
    pub const ZERO: Self = Self([0u8; 32]);

    /// Construct from a raw 32-byte array.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex encoding (64 chars).
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for b in &self.0 {
            out.push_str(&format!("{b:02x}"));
        }
        out
    }

    /// Parse lowercase or uppercase hex (exactly 64 chars).
    pub fn from_hex(s: &str) -> Result<Self, ExoCoreError> {
        if s.len() != 64 {
            return Err(ExoCoreError::InvalidHexLength(s.len()));
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            let byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| ExoCoreError::InvalidHexChar)?;
            bytes[i] = byte;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3Hash({})", self.to_hex())
    }
}

impl fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl From<[u8; 32]> for Blake3Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Blake3Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Content-addressed event id (BLAKE3 of the event body).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventId(pub Blake3Hash);

impl EventId {
    /// Wrap an existing hash.
    pub const fn new(hash: Blake3Hash) -> Self {
        Self(hash)
    }

    /// Derive an event id from canonical payload bytes.
    pub fn from_payload(payload: &[u8]) -> Self {
        Self(blake3_hash(payload))
    }

    /// Hex form for logging / wire.
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hybrid Logical Clock: physical milliseconds + logical tick.
///
/// Monotone under concurrent updates when used via [`HybridLogicalClock`].
/// Matches the SPARC `HLC` / `HybridLogicalClock` contract for K6 DAG events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Hlc {
    /// Wall-clock milliseconds since Unix epoch (best-effort).
    pub physical_ms: u64,
    /// Logical counter for same-ms causality.
    pub logical: u32,
}

impl Hlc {
    /// Epoch zero timestamp.
    pub const ZERO: Self = Self {
        physical_ms: 0,
        logical: 0,
    };

    /// Construct from parts.
    pub const fn new(physical_ms: u64, logical: u32) -> Self {
        Self {
            physical_ms,
            logical,
        }
    }

    /// Canonical 12-byte encoding: `physical_ms` BE + `logical` BE.
    pub fn to_bytes(self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0..8].copy_from_slice(&self.physical_ms.to_be_bytes());
        out[8..12].copy_from_slice(&self.logical.to_be_bytes());
        out
    }

    /// Decode from [`Hlc::to_bytes`].
    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        let mut phys = [0u8; 8];
        let mut log = [0u8; 4];
        phys.copy_from_slice(&bytes[0..8]);
        log.copy_from_slice(&bytes[8..12]);
        Self {
            physical_ms: u64::from_be_bytes(phys),
            logical: u32::from_be_bytes(log),
        }
    }
}

/// Thread-safe hybrid logical clock generator.
///
/// `tick()` advances past both wall time and last observed HLC so concurrent
/// producers observe a total order suitable for DAG event stamping.
#[derive(Debug)]
pub struct HybridLogicalClock {
    last_physical: AtomicU64,
    last_logical: AtomicU64,
}

impl Default for HybridLogicalClock {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridLogicalClock {
    /// Create a clock starting at epoch zero.
    pub fn new() -> Self {
        Self {
            last_physical: AtomicU64::new(0),
            last_logical: AtomicU64::new(0),
        }
    }

    /// Wall-clock ms (falls back to last physical on failure).
    fn wall_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_else(|_| self.last_physical.load(Ordering::Relaxed))
    }

    /// Produce the next HLC tick (local observation only).
    pub fn tick(&self) -> Hlc {
        self.observe(None)
    }

    /// Merge a remote HLC (if any) then advance.
    pub fn observe(&self, remote: Option<Hlc>) -> Hlc {
        let wall = self.wall_ms();
        loop {
            let last_p = self.last_physical.load(Ordering::Acquire);
            let last_l = self.last_logical.load(Ordering::Acquire) as u32;

            let mut phys = wall.max(last_p);
            let mut logical = if phys == last_p {
                last_l.saturating_add(1)
            } else {
                0
            };

            if let Some(r) = remote {
                if r.physical_ms > phys {
                    phys = r.physical_ms;
                    logical = r.logical.saturating_add(1);
                } else if r.physical_ms == phys {
                    logical = logical.max(r.logical).saturating_add(1);
                }
            }

            // CAS pair — rare races re-loop
            if self
                .last_physical
                .compare_exchange(last_p, phys, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            if self
                .last_logical
                .compare_exchange(
                    last_l as u64,
                    logical as u64,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
            {
                // roll physical back only if we still own it — best-effort
                let _ = self.last_physical.compare_exchange(
                    phys,
                    last_p,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                continue;
            }
            return Hlc::new(phys, logical);
        }
    }

    /// Last stamped value without advancing (may be ZERO before first tick).
    pub fn peek(&self) -> Hlc {
        Hlc::new(
            self.last_physical.load(Ordering::Acquire),
            self.last_logical.load(Ordering::Acquire) as u32,
        )
    }
}

/// Alias used in SPARC sketches (`HLC`).
pub type HLC = Hlc;

/// BLAKE3 hash of `data`.
pub fn blake3_hash(data: &[u8]) -> Blake3Hash {
    Blake3Hash(*blake3::hash(data).as_bytes())
}

/// Domain-separated BLAKE3: `hash(domain || 0x00 || data)`.
///
/// Domain strings should be stable protocol constants (e.g.
/// `WEFTOS-RESOURCE-PATH-v1`).
pub fn blake3_hash_with_domain(domain: &[u8], data: &[u8]) -> Blake3Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&[0u8]);
    hasher.update(data);
    Blake3Hash(*hasher.finalize().as_bytes())
}

/// Errors from exo-core parsing helpers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExoCoreError {
    /// Hex string was not 64 characters.
    #[error("blake3 hex length {0}, expected 64")]
    InvalidHexLength(usize),
    /// Non-hex character in digest string.
    #[error("invalid hex character in blake3 digest")]
    InvalidHexChar,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake3_hash_stable() {
        let a = blake3_hash(b"weftos-k6");
        let b = blake3_hash(b"weftos-k6");
        assert_eq!(a, b);
        assert_ne!(a, Blake3Hash::ZERO);
        assert_eq!(a.to_hex().len(), 64);
        assert_eq!(Blake3Hash::from_hex(&a.to_hex()).unwrap(), a);
    }

    #[test]
    fn domain_separation_differs() {
        let plain = blake3_hash(b"path");
        let dom = blake3_hash_with_domain(b"WEFTOS-RESOURCE-PATH-v1", b"path");
        assert_ne!(plain, dom);
        let dom2 = blake3_hash_with_domain(b"WEFTOS-RESOURCE-PATH-v1", b"path");
        assert_eq!(dom, dom2);
    }

    #[test]
    fn hlc_bytes_roundtrip() {
        let h = Hlc::new(1_700_000_000_000, 42);
        assert_eq!(Hlc::from_bytes(h.to_bytes()), h);
    }

    #[test]
    fn hybrid_clock_monotone() {
        let clock = HybridLogicalClock::new();
        let mut prev = Hlc::ZERO;
        for _ in 0..64 {
            let t = clock.tick();
            assert!(t > prev, "expected {t:?} > {prev:?}");
            prev = t;
        }
    }

    #[test]
    fn hybrid_clock_observes_remote() {
        let clock = HybridLogicalClock::new();
        let local = clock.tick();
        let remote = Hlc::new(local.physical_ms + 10_000, 0);
        let merged = clock.observe(Some(remote));
        assert!(merged.physical_ms >= remote.physical_ms);
        assert!(merged > remote || merged.logical > remote.logical);
    }

    #[test]
    fn event_id_from_payload() {
        let id = EventId::from_payload(b"event-body");
        assert_eq!(id.0, blake3_hash(b"event-body"));
    }

    #[test]
    fn serde_blake3_roundtrip() {
        let h = blake3_hash(b"serde");
        let json = serde_json::to_string(&h).unwrap();
        let back: Blake3Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);
    }
}
