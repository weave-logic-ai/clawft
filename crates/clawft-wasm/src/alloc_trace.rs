//! Optional allocation-counting wrapper for WASM memory profiling.
//!
//! Enabled with the `alloc-tracing` feature flag. Wraps any `GlobalAlloc`
//! and counts allocations by size class, tracking total bytes allocated,
//! freed, and peak live bytes.
//!
//! # Size Classes
//!
//! Buckets match common allocation sizes in `serde_json` workloads:
//! - **Small** (<=64): keys, short strings, `Value` enum variants
//! - **Medium** (<=256): typical JSON string values, small objects
//! - **Large** (<=1024): medium objects, small arrays
//! - **XLarge** (<=4096): large objects, conversation message arrays
//! - **Huge** (>4096): complete JSON documents, large buffers
//!
//! # Thread Safety
//!
//! All counters use `AtomicUsize` with `Ordering::Relaxed`. This is safe
//! for single-threaded WASM where no data races are possible, and provides
//! minimal overhead.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

// ── Size class counters ──────────────────────────────────────
static ALLOC_SMALL: AtomicUsize = AtomicUsize::new(0); // <= 64 bytes
static ALLOC_MEDIUM: AtomicUsize = AtomicUsize::new(0); // <= 256 bytes
static ALLOC_LARGE: AtomicUsize = AtomicUsize::new(0); // <= 1024 bytes
static ALLOC_XLARGE: AtomicUsize = AtomicUsize::new(0); // <= 4096 bytes
static ALLOC_HUGE: AtomicUsize = AtomicUsize::new(0); // > 4096 bytes

// ── Aggregate counters ───────────────────────────────────────
static TOTAL_ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_FREED_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL_FREE_COUNT: AtomicUsize = AtomicUsize::new(0);
static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Allocation-counting wrapper around any `GlobalAlloc`.
///
/// Tracks allocations by size class and total bytes allocated/freed.
/// Use with the `alloc-tracing` feature flag.
///
/// # Safety
///
/// This wrapper delegates all actual allocation to the inner allocator.
/// The counting uses `Relaxed` ordering which is safe for single-threaded
/// WASM (no data races possible) and provides minimal overhead.
pub struct TracingAllocator<A: GlobalAlloc> {
    inner: A,
}

impl<A: GlobalAlloc> TracingAllocator<A> {
    /// Create a new tracing wrapper around the given allocator.
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TracingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();

        // Classify by size bucket
        match size {
            0..=64 => {
                ALLOC_SMALL.fetch_add(1, Ordering::Relaxed);
            }
            65..=256 => {
                ALLOC_MEDIUM.fetch_add(1, Ordering::Relaxed);
            }
            257..=1024 => {
                ALLOC_LARGE.fetch_add(1, Ordering::Relaxed);
            }
            1025..=4096 => {
                ALLOC_XLARGE.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                ALLOC_HUGE.fetch_add(1, Ordering::Relaxed);
            }
        }

        TOTAL_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        let new_total = TOTAL_ALLOC_BYTES.fetch_add(size, Ordering::Relaxed) + size;

        // Track peak live bytes (alloc - freed high-water mark)
        let freed = TOTAL_FREED_BYTES.load(Ordering::Relaxed);
        let live = new_total.saturating_sub(freed);
        let mut current_peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
        while live > current_peak {
            match PEAK_LIVE_BYTES.compare_exchange_weak(
                current_peak,
                live,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }

        // SAFETY: caller guarantees layout is valid (non-zero size, valid alignment)
        unsafe { self.inner.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TOTAL_FREE_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_FREED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);

        // SAFETY: caller guarantees ptr was allocated by this allocator
        // with this layout
        unsafe { self.inner.dealloc(ptr, layout) }
    }
}

/// Allocation statistics snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AllocStats {
    /// Allocations <= 64 bytes
    pub small_count: usize,
    /// Allocations 65-256 bytes
    pub medium_count: usize,
    /// Allocations 257-1024 bytes
    pub large_count: usize,
    /// Allocations 1025-4096 bytes
    pub xlarge_count: usize,
    /// Allocations > 4096 bytes
    pub huge_count: usize,
    /// Total number of allocations
    pub total_alloc_count: usize,
    /// Total number of frees
    pub total_free_count: usize,
    /// Total bytes allocated (cumulative)
    pub total_alloc_bytes: usize,
    /// Total bytes freed (cumulative)
    pub total_freed_bytes: usize,
    /// Currently live bytes (alloc - freed)
    pub live_bytes: usize,
    /// Peak live bytes observed
    pub peak_live_bytes: usize,
    /// Fragmentation ratio: 1.0 - (live / peak), lower is better
    pub fragmentation_ratio: f64,
}

/// Read current allocation statistics.
pub fn read_stats() -> AllocStats {
    let total_alloc = TOTAL_ALLOC_BYTES.load(Ordering::Relaxed);
    let total_freed = TOTAL_FREED_BYTES.load(Ordering::Relaxed);
    let live = total_alloc.saturating_sub(total_freed);
    let peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);

    let frag = if peak > 0 {
        1.0 - (live as f64 / peak as f64)
    } else {
        0.0
    };

    AllocStats {
        small_count: ALLOC_SMALL.load(Ordering::Relaxed),
        medium_count: ALLOC_MEDIUM.load(Ordering::Relaxed),
        large_count: ALLOC_LARGE.load(Ordering::Relaxed),
        xlarge_count: ALLOC_XLARGE.load(Ordering::Relaxed),
        huge_count: ALLOC_HUGE.load(Ordering::Relaxed),
        total_alloc_count: TOTAL_ALLOC_COUNT.load(Ordering::Relaxed),
        total_free_count: TOTAL_FREE_COUNT.load(Ordering::Relaxed),
        total_alloc_bytes: total_alloc,
        total_freed_bytes: total_freed,
        live_bytes: live,
        peak_live_bytes: peak,
        fragmentation_ratio: frag,
    }
}

/// Reset all counters to zero. Useful for test isolation.
pub fn reset_stats() {
    ALLOC_SMALL.store(0, Ordering::Relaxed);
    ALLOC_MEDIUM.store(0, Ordering::Relaxed);
    ALLOC_LARGE.store(0, Ordering::Relaxed);
    ALLOC_XLARGE.store(0, Ordering::Relaxed);
    ALLOC_HUGE.store(0, Ordering::Relaxed);
    TOTAL_ALLOC_COUNT.store(0, Ordering::Relaxed);
    TOTAL_FREE_COUNT.store(0, Ordering::Relaxed);
    TOTAL_ALLOC_BYTES.store(0, Ordering::Relaxed);
    TOTAL_FREED_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
}

/// Export stats as a JSON string.
pub fn stats_json() -> String {
    serde_json::to_string_pretty(&read_stats()).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_zeroes_all_counters() {
        // Force some non-zero state
        ALLOC_SMALL.store(5, Ordering::Relaxed);
        TOTAL_ALLOC_BYTES.store(100, Ordering::Relaxed);
        PEAK_LIVE_BYTES.store(50, Ordering::Relaxed);

        reset_stats();

        let stats = read_stats();
        assert_eq!(stats.small_count, 0);
        assert_eq!(stats.total_alloc_bytes, 0);
        assert_eq!(stats.peak_live_bytes, 0);
        assert_eq!(stats.live_bytes, 0);
        assert_eq!(stats.fragmentation_ratio, 0.0);
    }

    #[test]
    fn stats_json_is_valid() {
        reset_stats();
        let json = stats_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["small_count"], 0);
        assert_eq!(parsed["total_alloc_count"], 0);
        assert!(parsed["fragmentation_ratio"].is_f64());
    }

    #[test]
    fn read_stats_computes_fragmentation() {
        reset_stats();
        // Simulate: allocated 1000, freed 600, peak was 800
        TOTAL_ALLOC_BYTES.store(1000, Ordering::Relaxed);
        TOTAL_FREED_BYTES.store(600, Ordering::Relaxed);
        PEAK_LIVE_BYTES.store(800, Ordering::Relaxed);

        let stats = read_stats();
        assert_eq!(stats.live_bytes, 400);
        assert_eq!(stats.peak_live_bytes, 800);
        // frag = 1.0 - (400 / 800) = 0.5
        assert!((stats.fragmentation_ratio - 0.5).abs() < f64::EPSILON);
    }
}
