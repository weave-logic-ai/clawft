//! Async runtime abstraction.
//!
//! Provides platform-agnostic wrappers for time and async sync primitives.
//! On native, delegates to tokio. On browser WASM, uses futures-util /
//! js_sys equivalents.

// ── now ───────────────────────────────────────────────────────────────

/// Get current time as milliseconds since epoch.
///
/// Safe on both native and browser WASM.
#[cfg(feature = "native")]
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Get current time as milliseconds since epoch (browser).
#[cfg(feature = "browser")]
pub fn now_millis() -> u64 {
    js_sys::Date::now() as u64
}

/// Fallback when neither native nor browser is selected.
#[cfg(not(any(feature = "native", feature = "browser")))]
pub fn now_millis() -> u64 {
    0
}

// ── Async Mutex re-export ─────────────────────────────────────────────

/// Re-export the appropriate async Mutex.
///
/// On native: `tokio::sync::Mutex` (Send-safe).
/// On browser: `futures_util::lock::Mutex` (single-threaded WASM).
#[cfg(feature = "native")]
pub use tokio::sync::Mutex;

#[cfg(not(feature = "native"))]
pub use futures_util::lock::Mutex;

// ── Async RwLock re-export ────────────────────────────────────────────

/// Re-export the appropriate async RwLock.
///
/// On native: `tokio::sync::RwLock`.
/// On browser: A thin wrapper around `std::sync::RwLock` with async methods.
#[cfg(feature = "native")]
pub use tokio::sync::RwLock;

// For browser, provide a minimal async-compatible RwLock wrapper
// using std::sync::RwLock. In single-threaded WASM, contention is impossible
// so the sync lock never blocks.
#[cfg(not(feature = "native"))]
pub mod rw_lock_wrapper {
    use std::ops::{Deref, DerefMut};

    /// Async-compatible RwLock for single-threaded WASM.
    ///
    /// Wraps `std::sync::RwLock` and provides `.read().await` / `.write().await`
    /// methods that return the same guard types, making it a drop-in replacement
    /// for `tokio::sync::RwLock` on browser targets.
    pub struct RwLock<T>(std::sync::RwLock<T>);

    impl<T> RwLock<T> {
        /// Create a new RwLock.
        pub fn new(value: T) -> Self {
            Self(std::sync::RwLock::new(value))
        }

        /// Acquire a read lock (async-compatible, resolves immediately on WASM).
        pub async fn read(&self) -> RwLockReadGuard<'_, T> {
            RwLockReadGuard(self.0.read().expect("RwLock poisoned"))
        }

        /// Acquire a write lock (async-compatible, resolves immediately on WASM).
        pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
            RwLockWriteGuard(self.0.write().expect("RwLock poisoned"))
        }
    }

    /// Read guard for the async RwLock wrapper.
    pub struct RwLockReadGuard<'a, T>(std::sync::RwLockReadGuard<'a, T>);

    impl<T> Deref for RwLockReadGuard<'_, T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.0
        }
    }

    /// Write guard for the async RwLock wrapper.
    pub struct RwLockWriteGuard<'a, T>(std::sync::RwLockWriteGuard<'a, T>);

    impl<T> Deref for RwLockWriteGuard<'_, T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.0
        }
    }

    impl<T> DerefMut for RwLockWriteGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut T {
            &mut self.0
        }
    }
}

#[cfg(not(feature = "native"))]
pub use rw_lock_wrapper::RwLock;
