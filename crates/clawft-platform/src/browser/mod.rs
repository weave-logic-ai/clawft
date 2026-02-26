//! Browser/WASM platform implementation.
//!
//! Provides [`BrowserPlatform`] which bundles browser-compatible
//! implementations of all platform sub-traits:
//!
//! - [`BrowserHttpClient`] -- HTTP via the fetch API ([`web_sys`]).
//! - [`BrowserFileSystem`] -- In-memory filesystem (OPFS planned for future).
//! - [`BrowserEnvironment`] -- In-memory key-value environment variables.
//!
//! Process spawning is not available in WASM, so
//! [`Platform::process`](crate::Platform::process) returns `None`.

pub mod env;
pub mod fs;
pub mod http;

pub use env::BrowserEnvironment;
pub use fs::BrowserFileSystem;
pub use http::BrowserHttpClient;

use async_trait::async_trait;

use crate::Platform;

/// Browser/WASM platform combining all browser-compatible sub-implementations.
///
/// This is the primary [`Platform`] implementation for use in browser
/// environments compiled to `wasm32-unknown-unknown`. It provides:
///
/// - HTTP via the browser fetch API.
/// - An in-memory filesystem (no persistence across page reloads).
/// - An in-memory environment variable store.
/// - No process spawning capability.
pub struct BrowserPlatform {
    http: BrowserHttpClient,
    fs: BrowserFileSystem,
    env: BrowserEnvironment,
}

impl BrowserPlatform {
    /// Create a new browser platform with default (empty) state.
    pub fn new() -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env: BrowserEnvironment::new(),
        }
    }

    /// Create a browser platform with a pre-populated environment.
    pub fn with_env(env: BrowserEnvironment) -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env,
        }
    }
}

impl Default for BrowserPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Platform for BrowserPlatform {
    fn http(&self) -> &dyn crate::http::HttpClient {
        &self.http
    }

    fn fs(&self) -> &dyn crate::fs::FileSystem {
        &self.fs
    }

    fn env(&self) -> &dyn crate::env::Environment {
        &self.env
    }

    fn process(&self) -> Option<&dyn crate::process::ProcessSpawner> {
        None
    }
}
