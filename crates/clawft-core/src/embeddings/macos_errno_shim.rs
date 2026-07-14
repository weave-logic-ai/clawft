//! Link-time compatibility shim for a real upstream portability bug.
//!
//! `rvf-runtime` 0.2.0's `locking.rs` (the file backing every
//! `RvfStore::create`/`open`/`derive`/`branch` call -- i.e. everything
//! [`crate::embeddings::rvf_real`] does) declares its own `extern "C" fn
//! __errno_location() -> *mut i32` and calls it directly, instead of going
//! through the `libc` crate's portable `errno()` accessor (verified at
//! `rvf-runtime-0.2.0/src/locking.rs:277-294`).
//!
//! `__errno_location` is a glibc (Linux) symbol name. macOS's libc exports
//! the equivalent accessor as `__error`, not `__errno_location` -- so on
//! `target_os = "macos"` this is an undefined symbol at link time
//! (`cargo check`/`cargo build --lib` succeed, because compiling an rlib
//! never resolves symbols; the failure only shows up when something finally
//! links a binary or test executable that calls into `rvf-runtime`'s
//! locking path -- which nothing in this crate did until the Phase 0
//! migration off `rvf_stub` started exercising `RvfStore::create`/`open`
//! for real).
//!
//! `crates/clawft-cow-memory` hit and fixed this exact same bug first (see
//! its `src/macos_errno_shim.rs`); this is the identical fix, applied here
//! because `clawft-core`'s `rvf` feature links `rvf-runtime` independently
//! and needs the symbol satisfied in its own link units too.
//!
//! We cannot patch the vendored crates.io source. What we *can* do, fully
//! within this crate: define the missing symbol ourselves. A `#[no_mangle]
//! extern "C"` function in any rlib on the link line satisfies an undefined
//! reference to that symbol from any other rlib on the same line -- so this
//! shim resolves the link for every binary/test that depends on
//! `clawft-core` with the `rvf` feature enabled on macOS.

#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __errno_location() -> *mut i32 {
    unsafe { libc::__error() }
}
