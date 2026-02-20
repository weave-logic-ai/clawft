//! WASM entrypoint for clawft.
//!
//! This crate provides a WebAssembly build of the clawft agent core.
//! It excludes components that require native OS features:
//! - Shell execution tools (exec_shell, spawn)
//! - Channel plugins (Telegram, Slack, Discord)
//! - Native CLI terminal I/O
//! - Process spawning
//!
//! # Platform Support
//!
//! The WASM build targets `wasm32-wasip1` and uses WASI preview 1 for:
//! - HTTP outbound (LLM API calls)
//! - Filesystem (config, sessions)
//! - Environment variables
//!
//! # Dependencies
//!
//! This crate is intentionally decoupled from `clawft-core` and `clawft-platform`
//! to avoid pulling in tokio["full"] and reqwest, neither of which compiles for
//! WASM targets. It depends only on `clawft-types`, `serde`, and `serde_json`.
//!
//! # Size Budget
//!
//! Target: < 300 KB uncompressed, < 120 KB gzipped.

#[cfg(feature = "alloc-tracing")]
pub mod alloc_trace;
pub mod allocator;
pub mod env;
pub mod fs;
pub mod http;
pub mod platform;

/// Plugin sandbox enforcement for WASM plugins.
///
/// This module is only available when the `wasm-plugins` feature is enabled.
/// It provides [`sandbox::PluginSandbox`], validation functions for HTTP,
/// filesystem, and environment access, plus rate limiting and size enforcement.
#[cfg(feature = "wasm-plugins")]
pub mod sandbox;

pub use platform::WasmPlatform;

/// Version information for the WASM build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the WASM agent.
///
/// Called once when the WASM module is instantiated. Sets up the agent
/// configuration and pipeline from WASI-accessible config files.
///
/// # Returns
///
/// Returns 0 on success, non-zero on failure.
pub fn init() -> i32 {
    // Phase 3A Week 11: Will load config from WASI filesystem
    // and set up the agent pipeline.
    0
}

/// Process a single message through the agent pipeline.
///
/// # Arguments
///
/// * `input` - The user message as a UTF-8 string.
///
/// # Returns
///
/// The agent's response as a string, or an error message.
pub fn process_message(input: &str) -> String {
    // Phase 3A Week 12: Will run the full 6-stage pipeline.
    format!(
        "clawft-wasm v{}: received '{}' (pipeline not yet wired)",
        VERSION, input
    )
}

/// Get the agent's capabilities as a JSON string.
///
/// Returns a JSON object describing available tools, providers,
/// and configuration for this WASM instance.
pub fn capabilities() -> String {
    serde_json::json!({
        "version": VERSION,
        "platform": "wasm32-wasip1",
        "tools": ["read_file", "write_file", "edit_file", "list_directory", "memory_read", "memory_write", "web_fetch", "web_search"],
        "excluded_tools": ["exec_shell", "spawn", "message"],
        "channels": [],
        "status": "initializing"
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_returns_zero() {
        assert_eq!(init(), 0);
    }

    #[test]
    fn process_message_returns_response() {
        let response = process_message("hello");
        assert!(response.contains("hello"));
        assert!(response.contains("clawft-wasm"));
    }

    #[test]
    fn capabilities_is_valid_json() {
        let caps = capabilities();
        let parsed: serde_json::Value = serde_json::from_str(&caps).unwrap();
        assert_eq!(parsed["platform"], "wasm32-wasip1");
        assert!(parsed["tools"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn excluded_tools_listed() {
        let caps = capabilities();
        let parsed: serde_json::Value = serde_json::from_str(&caps).unwrap();
        let excluded = parsed["excluded_tools"].as_array().unwrap();
        assert!(excluded.contains(&serde_json::json!("exec_shell")));
        assert!(excluded.contains(&serde_json::json!("spawn")));
    }
}
