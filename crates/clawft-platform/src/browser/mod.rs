//! Browser/WASM platform implementation.
//!
//! Provides [`BrowserPlatform`] which bundles browser-compatible
//! implementations of all platform sub-traits:
//!
//! - [`BrowserHttpClient`] -- HTTP via the fetch API ([`web_sys`]).
//! - [`BrowserFileSystem`] -- In-memory filesystem, or OPFS when built with
//!   the `browser-opfs` feature (WEFT-13 / WEFT-392).
//! - [`BrowserEnvironment`] -- In-memory env vars, optionally persisted to
//!   OPFS under the same feature (WEFT-14).
//!
//! Process spawning is not available in WASM, so
//! [`Platform::process`](crate::Platform::process) returns `None`.

pub mod env;
pub mod fs;
pub mod history_layout;
pub mod http;

pub use env::{
    BrowserEnvBackend, BrowserEnvironment, ENV_PERSIST_PATH,
};
pub use fs::{
    BrowserFileSystem, BrowserFsBackend, BROWSER_HOME_DIR, BROWSER_WORKSPACE_DIR,
};
pub use history_layout::{
    config_persist_path, conv_id_for_group, group_clawft_md_path, group_dir, resolve_group_id,
    sessions_dir, validate_group_id, BROWSER_CHANNEL, DEFAULT_GROUP_ID, GROUP_CLAWFT_MD,
};
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
/// - A filesystem: in-memory by default, or OPFS when constructed via
///   [`BrowserPlatform::open`] / [`BrowserPlatform::with_env_arc_open`]
///   under the `browser-opfs` feature (falls back to memory if OPFS is
///   unavailable).
/// - An environment store: memory by default, or OPFS-backed snapshot at
///   [`ENV_PERSIST_PATH`] when opened via [`BrowserPlatform::open`] /
///   [`BrowserEnvironment::open`] (WEFT-14).
/// - No process spawning capability.
///
/// The environment is held behind an [`Arc`] so callers (e.g. the
/// browser WASM `set_env` entry point) can retain a live handle after
/// the platform is moved into the agent loop (WEFT-391).
///
/// [`Platform::env`] remains a **sync** accessor (same as native); async
/// work lives on [`BrowserEnvironment::open`] / [`BrowserEnvironment::flush`].
pub struct BrowserPlatform {
    http: BrowserHttpClient,
    fs: BrowserFileSystem,
    env: Arc<BrowserEnvironment>,
}

impl BrowserPlatform {
    /// Create a new browser platform with default (empty) in-memory state.
    pub fn new() -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env: Arc::new(BrowserEnvironment::new()),
        }
    }

    /// Open a platform preferring OPFS for filesystem **and** environment
    /// when `browser-opfs` is enabled.
    ///
    /// Falls back to in-memory stores if OPFS is unavailable.
    /// Without the feature this is equivalent to [`new`].
    pub async fn open() -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::open().await,
            env: Arc::new(BrowserEnvironment::open().await),
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
    /// Uses the in-memory filesystem; prefer
    /// [`with_env_arc_open`] when OPFS is desired.
    pub fn with_env_arc(env: Arc<BrowserEnvironment>) -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::new(),
            env,
        }
    }

    /// Like [`with_env_arc`], but opens the best available filesystem
    /// backend (OPFS when `browser-opfs` is enabled).
    ///
    /// The provided `env` is used as-is (already opened / seeded by the
    /// caller — e.g. wasm `init` via [`BrowserEnvironment::open_with_seed`]).
    pub async fn with_env_arc_open(env: Arc<BrowserEnvironment>) -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs: BrowserFileSystem::open().await,
            env,
        }
    }

    /// Construct with an explicit filesystem (tests / custom wiring).
    pub fn with_env_arc_and_fs(env: Arc<BrowserEnvironment>, fs: BrowserFileSystem) -> Self {
        Self {
            http: BrowserHttpClient::new(),
            fs,
            env,
        }
    }

    /// Shared handle to the live environment (WEFT-391).
    ///
    /// Cloning the [`Arc`] lets the browser WASM runtime mutate env vars
    /// via `set_env` after this platform is owned by `AgentLoop`.
    pub fn env_arc(&self) -> Arc<BrowserEnvironment> {
        Arc::clone(&self.env)
    }

    /// Which filesystem backend is active (WEFT-13 / WEFT-392).
    pub fn fs_backend(&self) -> BrowserFsBackend {
        self.fs.backend()
    }

    /// Which environment durability backend is active (WEFT-14).
    pub fn env_backend(&self) -> BrowserEnvBackend {
        self.env.backend()
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
