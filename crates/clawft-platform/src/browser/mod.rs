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

use std::sync::Arc;

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
///
/// The environment is held behind an [`Arc`] so callers (e.g. the
/// browser WASM `set_env` entry point) can retain a live handle after
/// the platform is moved into the agent loop (WEFT-391).
pub struct BrowserPlatform {
    http: BrowserHttpClient,
    fs: BrowserFileSystem,
    env: Arc<BrowserEnvironment>,
}

impl BrowserPlatform {
    /// Create a new browser platform with default (empty) state.
    pub fn new() -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env: Arc::new(BrowserEnvironment::new()),
        }
    }

    /// Create a browser platform with a pre-populated environment.
    pub fn with_env(env: BrowserEnvironment) -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env: Arc::new(env),
        }
    }

    /// Create a browser platform sharing an existing environment handle.
    ///
    /// Used when the WASM runtime needs the same [`Arc`] both inside the
    /// platform (for agent tools / config loaders) and outside it (for
    /// `set_env` after `BrowserPlatform` is moved into `AgentLoop`).
    pub fn with_env_arc(env: Arc<BrowserEnvironment>) -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env,
        }
    }

    /// Shared handle to the live in-memory environment (WEFT-391).
    ///
    /// Cloning the [`Arc`] lets the browser WASM runtime mutate env vars
    /// via `set_env` after this platform is owned by `AgentLoop`.
    pub fn env_arc(&self) -> Arc<BrowserEnvironment> {
        Arc::clone(&self.env)
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
        self.env.as_ref()
    }

    fn process(&self) -> Option<&dyn crate::process::ProcessSpawner> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Environment;
    use crate::Platform;

    /// WEFT-391: set_env-style mutation through a shared Arc must be
    /// visible via `Platform::env()`.
    #[test]
    fn set_env_via_arc_visible_through_platform_env() {
        let env = Arc::new(BrowserEnvironment::new());
        let platform = BrowserPlatform::with_env_arc(Arc::clone(&env));

        env.set_var("CLAWFT_WEFT391_KEY", "live-value");

        assert_eq!(
            platform.env().get_var("CLAWFT_WEFT391_KEY"),
            Some("live-value".to_string())
        );
        // env_arc returns the same allocation.
        assert_eq!(
            platform.env_arc().get_var("CLAWFT_WEFT391_KEY"),
            Some("live-value".to_string())
        );

        // Overwrite through Platform::env() and re-read via Arc.
        platform.env().set_var("CLAWFT_WEFT391_KEY", "updated");
        assert_eq!(
            env.get_var("CLAWFT_WEFT391_KEY"),
            Some("updated".to_string())
        );
    }

    #[test]
    fn with_env_pre_seeds_vars() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("FOO".to_string(), "bar".to_string());
        let platform = BrowserPlatform::with_env(BrowserEnvironment::with_vars(vars));
        assert_eq!(
            platform.env().get_var("FOO"),
            Some("bar".to_string())
        );
    }

    #[test]
    fn process_spawner_is_none() {
        let platform = BrowserPlatform::new();
        assert!(platform.process().is_none());
    }
}
