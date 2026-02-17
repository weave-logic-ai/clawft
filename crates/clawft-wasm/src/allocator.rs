//! WASM global allocator configuration.
//!
//! Uses `dlmalloc` as the global allocator for WASM targets,
//! which is lightweight and well-tested for WebAssembly.

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
