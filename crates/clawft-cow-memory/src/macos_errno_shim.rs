//! Link-time compatibility shim for a real upstream portability bug.
//!
//! `rvf-runtime` 0.2.0's `locking.rs` (the file backing every
//! `RvfStore::create`/`open`/`derive`/`branch` call -- i.e. everything this
//! crate does) declares its own `extern "C" fn __errno_location() -> *mut
//! i32` and calls it directly, instead of going through the `libc` crate's
//! portable `errno()` accessor:
//!
//! ```text
//! extern "C" {
//!     fn __errno_location() -> *mut i32;
//! }
//! fn libc_errno() -> *mut i32 {
//!     unsafe { __errno_location() }
//! }
//! ```
//! (verified at `rvf-runtime-0.2.0/src/locking.rs:277-294`)
//!
//! `__errno_location` is a glibc (Linux) symbol name. macOS's libc exports
//! the equivalent accessor as `__error`, not `__errno_location` -- so on
//! `target_os = "macos"` this is a straight-up undefined symbol at link
//! time (`cargo check`/`cargo build --lib` succeed, because compiling an
//! rlib never resolves symbols; the failure only shows up when something
//! finally links a binary or test executable against `rvf-runtime`, which
//! `clawft-cow-memory`'s own integration tests were the first thing in this
//! workspace to do -- `rvf-runtime` is gated behind optional features
//! everywhere else it's used, so nothing had exercised this path before).
//!
//! We cannot patch the vendored crates.io source. What we *can* do, fully
//! within this crate: define the missing symbol ourselves. A `#[no_mangle]
//! extern "C"` function in any rlib on the link line satisfies an undefined
//! reference to that symbol from any other rlib on the same line -- so this
//! shim, compiled into `clawft-cow-memory`, resolves the link for every
//! binary/test that depends on this crate (which is every consumer of
//! `BranchableMemory`). It does **not** fix the symbol for a binary that
//! links `rvf-runtime` without also linking `clawft-cow-memory` (e.g.
//! `clawft-core`/`clawft-kernel`'s own `rvf` feature, once their Phase 0
//! migration off `rvf_stub` starts producing real binaries/tests on macOS)
//! -- that needs either the same shim added there, or an upstream fix.
//! Flagged in this crate's Phase 1 report as a cross-cutting discovery.

#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __errno_location() -> *mut i32 {
    unsafe { libc::__error() }
}
