//! WASM global allocator configuration.
//!
//! Uses feature flags to select between allocators for benchmarking.
//! Only one allocator can be active at a time. The default is `dlmalloc`,
//! which is lightweight and well-tested for WebAssembly.
//!
//! # Feature Flags
//!
//! - `alloc-talc`: Use the `talc` allocator (fast, Rust-native, small code)
//! - `alloc-lol`: Use `lol_alloc` (minimal code, never frees -- short-lived modules only)
//! - `alloc-tracing`: Wrap dlmalloc with allocation counting
//!
//! # Safety
//!
//! The `#[global_allocator]` attribute requires `GlobalAlloc`. All three allocators
//! are well-known WASM allocators with documented safety properties.

// ──────────────────────────────────────────────────────────────
// Option A: dlmalloc (current default)
// Proven, well-tested. ~2-10 KB code depending on optimization.
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    not(feature = "alloc-talc"),
    not(feature = "alloc-lol"),
    not(feature = "alloc-tracing"),
))]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// ──────────────────────────────────────────────────────────────
// Option B: talc (Rust-native, fast, ~1-2 KB code footprint)
//
// talc uses ClaimOnOom to grow from WASM linear memory on demand.
// On wasm32 (single-threaded), locking is effectively a no-op.
// Smaller minimum chunk size (3 * usize) than dlmalloc.
//
// See: https://github.com/SFBdragon/talc
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    feature = "alloc-talc",
    not(feature = "alloc-tracing"),
))]
#[global_allocator]
static ALLOC: talc::Talck<talc::locking::AssumeUnlockable, talc::ClaimOnOom> =
    talc::Talc::new(unsafe {
        // SAFETY: ClaimOnOom::new(Span::empty()) creates an allocator that
        // claims WASM linear memory pages on first allocation. The empty span
        // means no pre-reserved memory; all memory comes from memory.grow.
        // This is the standard pattern for WASM allocators.
        talc::ClaimOnOom::new(talc::Span::empty())
    })
    .lock();

// ──────────────────────────────────────────────────────────────
// Option C: lol_alloc (minimal code footprint, ~200 bytes)
//
// lol_alloc NEVER frees memory. It is a bump allocator that only
// grows. Suitable for short-lived WASM modules where the entire
// linear memory is discarded after each invocation.
//
// WARNING: Do NOT use for long-running WASM agents. Memory will
// grow unbounded. Only suitable for request-per-invocation patterns.
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    feature = "alloc-lol",
    not(feature = "alloc-tracing"),
))]
#[global_allocator]
static ALLOC: lol_alloc::AssumeSingleThreaded<lol_alloc::LeakingAllocator> =
    // SAFETY: WASM is single-threaded. AssumeSingleThreaded is the documented
    // pattern for wasm32 targets where no threading exists.
    unsafe { lol_alloc::AssumeSingleThreaded::new(lol_alloc::LeakingAllocator) };

// ──────────────────────────────────────────────────────────────
// Tracing wrapper (enabled with alloc-tracing feature)
// Wraps dlmalloc with allocation counting. See alloc_trace.rs.
// ──────────────────────────────────────────────────────────────
#[cfg(all(target_arch = "wasm32", feature = "alloc-tracing"))]
#[global_allocator]
static ALLOC: crate::alloc_trace::TracingAllocator<dlmalloc::GlobalDlmalloc> =
    crate::alloc_trace::TracingAllocator::new(dlmalloc::GlobalDlmalloc);
