//! Platform abstraction layer for clawft.
//!
//! Provides traits for all platform-dependent operations (HTTP, filesystem,
//! environment, process spawning) so the core engine can be platform-agnostic
//! and WASM-compatible.
//!
//! # Architecture
//!
//! The [`Platform`] trait bundles all platform capabilities via accessor methods.
//! Each sub-capability has its own trait ([`http::HttpClient`], [`fs::FileSystem`],
//! [`env::Environment`], [`process::ProcessSpawner`]) with a corresponding native
//! implementation.
//!
//! [`ProcessSpawner`](process::ProcessSpawner) is intentionally excluded from the
//! [`Platform`] bundle because it is unavailable in WASM environments. It is
//! accessible via [`Platform::process`] which returns `Option`.
//!
//! # Example
//!
//! ```rust,no_run
//! use clawft_platform::{Platform, NativePlatform};
//! use clawft_platform::http::HttpClient;
//! use std::collections::HashMap;
//!
//! # async fn example() {
//! let platform = NativePlatform::new();
//! let response = platform.http()
//!     .get("https://example.com", &HashMap::new())
//!     .await
//!     .unwrap();
//! assert!(response.is_success());
//! # }
//! ```

pub mod config_loader;
pub mod env;
pub mod fs;
pub mod http;
pub mod process;

use async_trait::async_trait;

/// Bundle of all platform capabilities.
///
/// Implementors provide concrete implementations of all platform traits.
/// The native implementation ([`NativePlatform`]) uses reqwest, tokio::fs,
/// std::env, and tokio::process. A WASM implementation would use the fetch
/// API, WASI filesystem, and an in-memory environment.
#[async_trait]
pub trait Platform: Send + Sync {
    /// HTTP client for making API requests.
    fn http(&self) -> &dyn http::HttpClient;

    /// Filesystem operations.
    fn fs(&self) -> &dyn fs::FileSystem;

    /// Environment variable access.
    fn env(&self) -> &dyn env::Environment;

    /// Process spawning capability.
    ///
    /// Returns `None` in environments where process spawning is unavailable
    /// (e.g., WASM).
    fn process(&self) -> Option<&dyn process::ProcessSpawner>;
}

/// Native platform implementation using std, tokio, and reqwest.
///
/// This is the standard platform for server-side and CLI usage. It provides:
/// - HTTP via [`reqwest`] with connection pooling and TLS.
/// - Filesystem via [`tokio::fs`].
/// - Environment via [`std::env`].
/// - Process spawning via [`tokio::process`].
pub struct NativePlatform {
    http: http::NativeHttpClient,
    fs: fs::NativeFileSystem,
    env: env::NativeEnvironment,
    process: process::NativeProcessSpawner,
}

impl NativePlatform {
    /// Create a new native platform with default configuration.
    pub fn new() -> Self {
        Self {
            http: http::NativeHttpClient::new(),
            fs: fs::NativeFileSystem,
            env: env::NativeEnvironment,
            process: process::NativeProcessSpawner,
        }
    }
}

impl Default for NativePlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for NativePlatform {
    fn http(&self) -> &dyn http::HttpClient {
        &self.http
    }

    fn fs(&self) -> &dyn fs::FileSystem {
        &self.fs
    }

    fn env(&self) -> &dyn env::Environment {
        &self.env
    }

    fn process(&self) -> Option<&dyn process::ProcessSpawner> {
        Some(&self.process)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_platform_creation() {
        let platform = NativePlatform::new();
        // Verify all accessors return valid references
        let _http = platform.http();
        let _fs = platform.fs();
        let _env = platform.env();
        assert!(platform.process().is_some());
    }

    #[test]
    fn test_native_platform_default() {
        let platform = NativePlatform::default();
        assert!(platform.process().is_some());
    }

    #[test]
    fn test_platform_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NativePlatform>();
    }
}
