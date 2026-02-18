//! WASM platform bundle.
//!
//! Provides [`WasmPlatform`], a self-contained struct that bundles the
//! WASI-oriented stubs ([`WasiHttpClient`], [`WasiFileSystem`], [`WasiEnvironment`])
//! into a single object suitable for constructing a clawft agent in a WASM environment.
//!
//! This module is fully decoupled from `clawft-platform` so it can compile for
//! `wasm32-wasip1` without pulling in tokio or reqwest.

use crate::env::WasiEnvironment;
use crate::fs::WasiFileSystem;
use crate::http::WasiHttpClient;

/// WASM platform implementation.
///
/// Bundles [`WasiHttpClient`], [`WasiFileSystem`], and [`WasiEnvironment`]
/// into a single struct. Process spawning is not available in WASM.
pub struct WasmPlatform {
    http: WasiHttpClient,
    fs: WasiFileSystem,
    env: WasiEnvironment,
}

impl WasmPlatform {
    /// Create a new WASM platform with default (empty) configuration.
    pub fn new() -> Self {
        Self {
            http: WasiHttpClient::new(),
            fs: WasiFileSystem::new(),
            env: WasiEnvironment::new(),
        }
    }

    /// Create a new WASM platform with a pre-populated environment.
    pub fn with_env(env: WasiEnvironment) -> Self {
        Self {
            http: WasiHttpClient::new(),
            fs: WasiFileSystem::new(),
            env,
        }
    }

    /// HTTP client for making API requests.
    pub fn http(&self) -> &WasiHttpClient {
        &self.http
    }

    /// Filesystem operations.
    pub fn fs(&self) -> &WasiFileSystem {
        &self.fs
    }

    /// Environment variable access.
    pub fn env(&self) -> &WasiEnvironment {
        &self.env
    }

    /// Process spawning capability.
    ///
    /// Always returns `None` -- process spawning is not available in WASM.
    pub fn process(&self) -> Option<()> {
        None
    }
}

impl Default for WasmPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn wasm_platform_can_be_created() {
        let _platform = WasmPlatform::new();
    }

    #[test]
    fn wasm_platform_default() {
        let _platform = WasmPlatform::default();
    }

    #[test]
    fn wasm_platform_with_env() {
        let mut vars = HashMap::new();
        vars.insert("API_KEY".to_string(), "test-key".to_string());
        let env = WasiEnvironment::with_vars(vars);

        let platform = WasmPlatform::with_env(env);
        assert_eq!(
            platform.env().get_var("API_KEY"),
            Some("test-key".to_string())
        );
    }

    #[test]
    fn process_returns_none() {
        let platform = WasmPlatform::new();
        assert!(platform.process().is_none());
    }

    #[test]
    fn accessors_return_valid_references() {
        let platform = WasmPlatform::new();
        let _http = platform.http();
        let _fs = platform.fs();
        let _env = platform.env();
    }

    #[test]
    fn env_is_functional() {
        let platform = WasmPlatform::new();
        platform.env().set_var("TEST_KEY", "test_value");
        assert_eq!(
            platform.env().get_var("TEST_KEY"),
            Some("test_value".to_string())
        );
        platform.env().remove_var("TEST_KEY");
        assert!(platform.env().get_var("TEST_KEY").is_none());
    }

    #[test]
    fn wasm_platform_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WasmPlatform>();
    }

    #[test]
    fn http_returns_error() {
        let platform = WasmPlatform::new();
        let headers = HashMap::new();
        let result = platform.http().get("https://example.com", &headers);
        assert!(result.is_err());
    }

    #[test]
    fn fs_returns_errors() {
        let platform = WasmPlatform::new();
        let result = platform
            .fs()
            .read_to_string(std::path::Path::new("/tmp/test"));
        assert!(result.is_err());
    }
}
