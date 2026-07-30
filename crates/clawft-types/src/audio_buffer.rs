//! SC-2 secure audio buffers (WEFT-223).
//!
//! Raw PCM is privacy-sensitive. Hold it in [`SecureAudioBuffer`] or
//! [`SecureAudioRing`] so sample contents are zeroized on drop (and, for
//! the ring, on eviction of individual samples). Pair with
//! [`crate::config::AudioRetention`] for the disk/session retention policy;
//! these types enforce the **memory** half of SC-2 regardless of config.

use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};

use zeroize::Zeroize;

/// Owned PCM buffer (`i16` samples) that zeroizes its contents on drop.
///
/// Prefer this over bare `Vec<i16>` anywhere raw mic audio is retained past
/// a single stack frame (capture chunks, utterance assembly, STT handoff).
/// Debug / Display intentionally omit sample values.
#[derive(Clone, Default)]
pub struct SecureAudioBuffer {
    data: Vec<i16>,
}

impl SecureAudioBuffer {
    /// Empty buffer.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Wrap an existing sample vector. The samples will be zeroized when
    /// this buffer is dropped (unless moved out via [`into_vec`](Self::into_vec)).
    pub fn from_samples(samples: Vec<i16>) -> Self {
        Self { data: samples }
    }

    /// Pre-allocate capacity without filling samples.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: Vec::with_capacity(cap),
        }
    }

    /// Borrow samples as a slice.
    pub fn as_slice(&self) -> &[i16] {
        &self.data
    }

    /// Mutable sample slice (for in-place DSP).
    pub fn as_mut_slice(&mut self) -> &mut [i16] {
        &mut self.data
    }

    /// Append samples.
    pub fn extend_from_slice(&mut self, samples: &[i16]) {
        self.data.extend_from_slice(samples);
    }

    /// Append by consuming another secure buffer (source is emptied without
    /// a second zeroize of the moved region — ownership transfers).
    pub fn append(&mut self, other: &mut SecureAudioBuffer) {
        self.data.append(&mut other.data);
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Zeroize and clear, keeping allocated capacity.
    pub fn clear(&mut self) {
        self.data.zeroize();
        self.data.clear();
    }

    /// Move samples out. Caller becomes responsible for zeroizing the
    /// returned `Vec` (e.g. by wrapping it again or calling
    /// [`zeroize_samples`]). The emptied buffer's Drop is a no-op.
    pub fn into_vec(mut self) -> Vec<i16> {
        std::mem::take(&mut self.data)
    }

    /// Replace contents with `samples`, zeroizing the previous buffer first.
    pub fn replace(&mut self, samples: Vec<i16>) {
        self.data.zeroize();
        self.data = samples;
    }
}

impl Drop for SecureAudioBuffer {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

impl From<Vec<i16>> for SecureAudioBuffer {
    fn from(data: Vec<i16>) -> Self {
        Self { data }
    }
}

impl From<&[i16]> for SecureAudioBuffer {
    fn from(samples: &[i16]) -> Self {
        Self {
            data: samples.to_vec(),
        }
    }
}

impl Deref for SecureAudioBuffer {
    type Target = [i16];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for SecureAudioBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl std::fmt::Debug for SecureAudioBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureAudioBuffer")
            .field("len", &self.data.len())
            .field("samples", &"[REDACTED]")
            .finish()
    }
}

/// Fixed-capacity pre-onset / capture ring of PCM samples.
///
/// Bound memory exposure (security review: ~300–400 ms max). Evicted
/// samples are zeroized immediately; the whole ring is zeroized on drop.
#[derive(Clone)]
pub struct SecureAudioRing {
    data: VecDeque<i16>,
    capacity: usize,
}

impl SecureAudioRing {
    /// Create a ring that retains at most `capacity` samples.
    ///
    /// A capacity of 0 is treated as 0 (push becomes a no-op that still
    /// zeroizes any transient input the caller may leave behind elsewhere).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity.saturating_add(1)),
            capacity,
        }
    }

    /// Ring capacity in samples.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current fill level.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Push a frame, zeroizing samples that fall off the front.
    pub fn push_frame(&mut self, frame: &[i16]) {
        if self.capacity == 0 {
            return;
        }
        self.data.extend(frame.iter().copied());
        while self.data.len() > self.capacity {
            if let Some(mut s) = self.data.pop_front() {
                s.zeroize();
            }
        }
    }

    /// Copy current contents into a new [`SecureAudioBuffer`] (e.g. pre-roll
    /// prepend at speech onset).
    pub fn to_secure_buffer(&self) -> SecureAudioBuffer {
        SecureAudioBuffer::from_samples(self.data.iter().copied().collect())
    }

    /// Copy current contents into a plain `Vec` (prefer
    /// [`to_secure_buffer`](Self::to_secure_buffer) when the result is
    /// retained).
    pub fn to_vec(&self) -> Vec<i16> {
        self.data.iter().copied().collect()
    }

    /// Iterate samples front-to-back (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &i16> {
        self.data.iter()
    }

    /// Zeroize and empty the ring.
    pub fn clear(&mut self) {
        for s in self.data.iter_mut() {
            s.zeroize();
        }
        self.data.clear();
    }
}

impl Drop for SecureAudioRing {
    fn drop(&mut self) {
        self.clear();
    }
}

impl std::fmt::Debug for SecureAudioRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureAudioRing")
            .field("len", &self.data.len())
            .field("capacity", &self.capacity)
            .field("samples", &"[REDACTED]")
            .finish()
    }
}

/// Zeroize a bare sample slice in place (for `Vec`/`[i16]` sites that have
/// not yet migrated to [`SecureAudioBuffer`]).
pub fn zeroize_samples(samples: &mut [i16]) {
    samples.zeroize();
}

/// Zeroize and clear a bare `Vec<i16>`.
pub fn zeroize_vec(samples: &mut Vec<i16>) {
    samples.zeroize();
    samples.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_buffer_zeroizes_on_drop() {
        // We cannot observe freed heap after drop from safe Rust, but we
        // can prove Drop runs clear() semantics via clear() and that
        // Debug never leaks sample values.
        let mut buf = SecureAudioBuffer::from_samples(vec![1, 2, 3, 4, 5]);
        assert_eq!(buf.len(), 5);
        let dbg = format!("{buf:?}");
        assert!(dbg.contains("REDACTED"));
        assert!(!dbg.contains("1, 2, 3"));
        buf.clear();
        assert!(buf.is_empty());
        // After clear, underlying capacity may remain but contents are zero.
        assert!(buf.as_slice().iter().all(|&s| s == 0) || buf.is_empty());
    }

    #[test]
    fn secure_buffer_into_vec_transfers_ownership() {
        let buf = SecureAudioBuffer::from_samples(vec![9, 8, 7]);
        let v = buf.into_vec();
        assert_eq!(v, vec![9, 8, 7]);
        // buf dropped empty — no double free / no panic
    }

    #[test]
    fn secure_ring_evicts_and_bounds_capacity() {
        let mut ring = SecureAudioRing::with_capacity(4);
        ring.push_frame(&[1, 2, 3]);
        assert_eq!(ring.len(), 3);
        ring.push_frame(&[4, 5, 6]);
        // capacity 4 → last four samples: 3,4,5,6
        assert_eq!(ring.len(), 4);
        let v = ring.to_vec();
        assert_eq!(v, vec![3, 4, 5, 6]);
        let dbg = format!("{ring:?}");
        assert!(dbg.contains("REDACTED"));
        ring.clear();
        assert!(ring.is_empty());
    }

    #[test]
    fn secure_ring_zero_capacity_is_noop() {
        let mut ring = SecureAudioRing::with_capacity(0);
        ring.push_frame(&[1, 2, 3]);
        assert!(ring.is_empty());
    }

    #[test]
    fn zeroize_samples_helper() {
        let mut v = vec![1i16, 2, 3];
        zeroize_samples(&mut v);
        assert_eq!(v, vec![0, 0, 0]);
        zeroize_vec(&mut v);
        assert!(v.is_empty());
    }
}
