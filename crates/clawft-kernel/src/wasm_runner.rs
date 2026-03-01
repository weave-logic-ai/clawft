//! WASM tool execution sandbox.
//!
//! Provides types and configuration for running tools inside
//! isolated WASM sandboxes with fuel metering, memory limits,
//! and host filesystem isolation.
//!
//! # Feature Gate
//!
//! This module is compiled unconditionally, but the actual
//! Wasmtime runtime integration requires the `wasm-sandbox`
//! feature flag. Without it, [`WasmToolRunner::new`] returns
//! a runner that rejects all tool loads with [`WasmError::RuntimeUnavailable`].
//!
//! # Security
//!
//! Each tool execution gets its own isolated store:
//! - No host filesystem access (unless WASI explicitly enabled)
//! - No network access
//! - CPU bounded by fuel metering
//! - Memory bounded by configurable cap
//! - Wall-clock timeout as safety net

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for the WASM sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSandboxConfig {
    /// Maximum fuel units (roughly equivalent to instructions).
    /// Default: 1,000,000 (~100ms on modern hardware).
    #[serde(default = "default_max_fuel")]
    pub max_fuel: u64,

    /// Maximum memory in bytes the WASM module may allocate.
    /// Default: 16 MiB.
    #[serde(default = "default_max_memory")]
    pub max_memory_bytes: usize,

    /// Wall-clock timeout for execution.
    /// Default: 30 seconds.
    #[serde(
        default = "default_max_execution_secs",
        alias = "maxExecutionTimeSecs"
    )]
    pub max_execution_time_secs: u64,

    /// Host function calls the WASM module is allowed to make.
    /// Empty means no host calls permitted.
    #[serde(default)]
    pub allowed_host_calls: Vec<String>,

    /// Whether to enable WASI (basic I/O, no filesystem).
    #[serde(default)]
    pub wasi_enabled: bool,

    /// Maximum WASM module size in bytes before loading.
    /// Default: 10 MiB.
    #[serde(default = "default_max_module_size")]
    pub max_module_size_bytes: usize,
}

fn default_max_fuel() -> u64 {
    1_000_000
}

fn default_max_memory() -> usize {
    16 * 1024 * 1024 // 16 MiB
}

fn default_max_execution_secs() -> u64 {
    30
}

fn default_max_module_size() -> usize {
    10 * 1024 * 1024 // 10 MiB
}

impl Default for WasmSandboxConfig {
    fn default() -> Self {
        Self {
            max_fuel: default_max_fuel(),
            max_memory_bytes: default_max_memory(),
            max_execution_time_secs: default_max_execution_secs(),
            allowed_host_calls: Vec::new(),
            wasi_enabled: false,
            max_module_size_bytes: default_max_module_size(),
        }
    }
}

impl WasmSandboxConfig {
    /// Get the execution timeout as a Duration.
    pub fn execution_timeout(&self) -> Duration {
        Duration::from_secs(self.max_execution_time_secs)
    }
}

/// Per-execution state for a WASM tool.
#[derive(Debug, Clone, Default)]
pub struct ToolState {
    /// Name of the tool being executed.
    pub tool_name: String,

    /// Input data (stdin equivalent).
    pub stdin: Vec<u8>,

    /// Output data (stdout equivalent).
    pub stdout: Vec<u8>,

    /// Error output data (stderr equivalent).
    pub stderr: Vec<u8>,

    /// Environment variables available to the tool.
    pub env: HashMap<String, String>,
}

/// Result of a WASM tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmToolResult {
    /// Standard output from the tool.
    pub stdout: String,

    /// Standard error from the tool.
    pub stderr: String,

    /// Exit code (0 = success).
    pub exit_code: i32,

    /// Fuel units consumed during execution.
    pub fuel_consumed: u64,

    /// Peak memory usage in bytes.
    pub memory_peak: usize,

    /// Actual execution duration.
    #[serde(with = "duration_millis")]
    pub execution_time: Duration,
}

/// Serialization helper for Duration as milliseconds.
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}

/// Validation result for a WASM module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmValidation {
    /// Whether the module is valid.
    pub valid: bool,

    /// Exported function names.
    pub exports: Vec<String>,

    /// Required import names.
    pub imports: Vec<String>,

    /// Estimated initial memory requirement.
    pub estimated_memory: usize,

    /// Warnings about the module (non-fatal issues).
    pub warnings: Vec<String>,
}

/// WASM runner errors.
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    /// The WASM runtime is not available (feature not enabled).
    #[error("WASM runtime unavailable: compile with --features wasm-sandbox")]
    RuntimeUnavailable,

    /// The WASM module bytes are invalid.
    #[error("invalid WASM module: {0}")]
    InvalidModule(String),

    /// Module compilation failed.
    #[error("compilation failed: {0}")]
    CompilationFailed(String),

    /// The tool exhausted its fuel budget.
    #[error("fuel exhausted after {consumed} units (limit: {limit})")]
    FuelExhausted {
        /// Fuel consumed before exhaustion.
        consumed: u64,
        /// Configured fuel limit.
        limit: u64,
    },

    /// Memory allocation exceeded the configured limit.
    #[error("memory limit exceeded: {allocated} bytes (limit: {limit} bytes)")]
    MemoryLimitExceeded {
        /// Bytes allocated when limit was hit.
        allocated: usize,
        /// Configured memory limit.
        limit: usize,
    },

    /// Execution exceeded the wall-clock timeout.
    #[error("execution timeout after {0:?}")]
    ExecutionTimeout(Duration),

    /// A WASM trap occurred during execution.
    #[error("WASM trap: {0}")]
    WasmTrap(String),

    /// A host function call was denied by sandbox policy.
    #[error("host call denied: {0}")]
    HostCallDenied(String),

    /// The module exceeds the maximum allowed size.
    #[error("module too large: {size} bytes (limit: {limit} bytes)")]
    ModuleTooLarge {
        /// Actual module size.
        size: usize,
        /// Configured size limit.
        limit: usize,
    },
}

/// WASM tool runner.
///
/// When the `wasm-sandbox` feature is enabled, this uses Wasmtime
/// for actual WASM execution. Without the feature, all tool loads
/// are rejected with [`WasmError::RuntimeUnavailable`].
pub struct WasmToolRunner {
    config: WasmSandboxConfig,
}

impl WasmToolRunner {
    /// Create a new WASM tool runner with the given configuration.
    pub fn new(config: WasmSandboxConfig) -> Self {
        Self { config }
    }

    /// Get the sandbox configuration.
    pub fn config(&self) -> &WasmSandboxConfig {
        &self.config
    }

    /// Validate a WASM module's bytes without loading it.
    ///
    /// Checks module size, magic bytes, and (when the runtime is
    /// available) parses exports and imports.
    pub fn validate_wasm(&self, wasm_bytes: &[u8]) -> Result<WasmValidation, WasmError> {
        // Check module size
        if wasm_bytes.len() > self.config.max_module_size_bytes {
            return Err(WasmError::ModuleTooLarge {
                size: wasm_bytes.len(),
                limit: self.config.max_module_size_bytes,
            });
        }

        // Check WASM magic bytes (\0asm)
        if wasm_bytes.len() < 8 || &wasm_bytes[0..4] != b"\0asm" {
            return Err(WasmError::InvalidModule(
                "missing WASM magic bytes (\\0asm)".into(),
            ));
        }

        let mut warnings = Vec::new();

        // Parse version (bytes 4-7 in little-endian)
        let version = u32::from_le_bytes([
            wasm_bytes[4],
            wasm_bytes[5],
            wasm_bytes[6],
            wasm_bytes[7],
        ]);
        if version != 1 {
            warnings.push(format!("unexpected WASM version: {version} (expected 1)"));
        }

        // Without the wasm-sandbox feature, we can only do basic validation
        #[cfg(not(feature = "wasm-sandbox"))]
        {
            Ok(WasmValidation {
                valid: true,
                exports: Vec::new(),
                imports: Vec::new(),
                estimated_memory: 0,
                warnings,
            })
        }

        // With the feature, full validation would use wasmtime
        #[cfg(feature = "wasm-sandbox")]
        {
            // TODO: Use wasmtime::Module::validate() for full validation
            Ok(WasmValidation {
                valid: true,
                exports: Vec::new(),
                imports: Vec::new(),
                estimated_memory: 0,
                warnings,
            })
        }
    }

    /// Load a WASM tool from bytes.
    ///
    /// # Errors
    ///
    /// Returns [`WasmError::RuntimeUnavailable`] when the
    /// `wasm-sandbox` feature is not enabled.
    /// Returns [`WasmError::ModuleTooLarge`] if the module
    /// exceeds the configured size limit.
    pub fn load_tool(
        &self,
        name: &str,
        wasm_bytes: &[u8],
    ) -> Result<WasmTool, WasmError> {
        // Validate first
        let validation = self.validate_wasm(wasm_bytes)?;

        if !validation.valid {
            return Err(WasmError::InvalidModule(
                validation.warnings.join("; "),
            ));
        }

        #[cfg(not(feature = "wasm-sandbox"))]
        {
            let _ = name;
            Err(WasmError::RuntimeUnavailable)
        }

        #[cfg(feature = "wasm-sandbox")]
        {
            // TODO: Compile module with wasmtime::Engine
            Ok(WasmTool {
                name: name.to_owned(),
                module_size: wasm_bytes.len(),
                schema: None,
                exports: validation.exports,
            })
        }
    }

    /// Execute a loaded WASM tool.
    ///
    /// # Errors
    ///
    /// Returns [`WasmError::RuntimeUnavailable`] when the
    /// `wasm-sandbox` feature is not enabled.
    pub fn execute(
        &self,
        _tool: &WasmTool,
        _input: serde_json::Value,
    ) -> Result<WasmToolResult, WasmError> {
        #[cfg(not(feature = "wasm-sandbox"))]
        {
            Err(WasmError::RuntimeUnavailable)
        }

        #[cfg(feature = "wasm-sandbox")]
        {
            // TODO: Create Store, instantiate module, run with fuel metering
            Err(WasmError::RuntimeUnavailable)
        }
    }
}

/// A loaded WASM tool module.
#[derive(Debug, Clone)]
pub struct WasmTool {
    /// Tool name.
    pub name: String,

    /// Module size in bytes.
    pub module_size: usize,

    /// Tool parameter schema (if exported by the module).
    pub schema: Option<serde_json::Value>,

    /// Exported function names.
    pub exports: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = WasmSandboxConfig::default();
        assert_eq!(config.max_fuel, 1_000_000);
        assert_eq!(config.max_memory_bytes, 16 * 1024 * 1024);
        assert_eq!(config.max_execution_time_secs, 30);
        assert!(config.allowed_host_calls.is_empty());
        assert!(!config.wasi_enabled);
        assert_eq!(config.max_module_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = WasmSandboxConfig {
            max_fuel: 500_000,
            max_memory_bytes: 8 * 1024 * 1024,
            max_execution_time_secs: 10,
            allowed_host_calls: vec!["clock_time_get".into()],
            wasi_enabled: true,
            max_module_size_bytes: 5 * 1024 * 1024,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: WasmSandboxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_fuel, 500_000);
        assert!(restored.wasi_enabled);
        assert_eq!(restored.allowed_host_calls.len(), 1);
    }

    #[test]
    fn execution_timeout_duration() {
        let config = WasmSandboxConfig {
            max_execution_time_secs: 15,
            ..Default::default()
        };
        assert_eq!(config.execution_timeout(), Duration::from_secs(15));
    }

    #[test]
    fn tool_state_default() {
        let state = ToolState::default();
        assert!(state.tool_name.is_empty());
        assert!(state.stdin.is_empty());
        assert!(state.stdout.is_empty());
        assert!(state.stderr.is_empty());
        assert!(state.env.is_empty());
    }

    #[test]
    fn validate_wasm_rejects_too_large() {
        let runner = WasmToolRunner::new(WasmSandboxConfig {
            max_module_size_bytes: 100,
            ..Default::default()
        });
        let big_bytes = vec![0u8; 200];
        let result = runner.validate_wasm(&big_bytes);
        assert!(matches!(result, Err(WasmError::ModuleTooLarge { .. })));
    }

    #[test]
    fn validate_wasm_rejects_invalid_magic() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let bad_bytes = b"not a wasm module at all";
        let result = runner.validate_wasm(bad_bytes);
        assert!(matches!(result, Err(WasmError::InvalidModule(_))));
    }

    #[test]
    fn validate_wasm_rejects_too_short() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let short = b"\0asm";
        let result = runner.validate_wasm(short);
        assert!(matches!(result, Err(WasmError::InvalidModule(_))));
    }

    #[test]
    fn validate_wasm_accepts_valid_header() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        // Minimal valid WASM header: magic (4 bytes) + version 1 (4 bytes)
        let mut wasm = Vec::new();
        wasm.extend_from_slice(b"\0asm");
        wasm.extend_from_slice(&1u32.to_le_bytes());
        // Add some padding to reach a reasonable size
        wasm.extend_from_slice(&[0u8; 16]);

        let result = runner.validate_wasm(&wasm);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.valid);
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn validate_wasm_warns_on_wrong_version() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let mut wasm = Vec::new();
        wasm.extend_from_slice(b"\0asm");
        wasm.extend_from_slice(&2u32.to_le_bytes()); // Version 2, unexpected
        wasm.extend_from_slice(&[0u8; 16]);

        let result = runner.validate_wasm(&wasm).unwrap();
        assert!(result.valid);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("version: 2"));
    }

    #[test]
    fn load_tool_without_feature_rejects() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let mut wasm = Vec::new();
        wasm.extend_from_slice(b"\0asm");
        wasm.extend_from_slice(&1u32.to_le_bytes());
        wasm.extend_from_slice(&[0u8; 16]);

        // Without wasm-sandbox feature, this should fail
        #[cfg(not(feature = "wasm-sandbox"))]
        {
            let result = runner.load_tool("test-tool", &wasm);
            assert!(matches!(result, Err(WasmError::RuntimeUnavailable)));
        }
    }

    #[test]
    fn wasm_error_display() {
        let err = WasmError::RuntimeUnavailable;
        assert!(err.to_string().contains("wasm-sandbox"));

        let err = WasmError::ModuleTooLarge {
            size: 20_000_000,
            limit: 10_000_000,
        };
        assert!(err.to_string().contains("20000000"));
        assert!(err.to_string().contains("10000000"));

        let err = WasmError::FuelExhausted {
            consumed: 1_000_000,
            limit: 1_000_000,
        };
        assert!(err.to_string().contains("fuel exhausted"));

        let err = WasmError::MemoryLimitExceeded {
            allocated: 32_000_000,
            limit: 16_000_000,
        };
        assert!(err.to_string().contains("memory limit"));
    }

    #[test]
    fn wasm_tool_result_serde_roundtrip() {
        let result = WasmToolResult {
            stdout: "output".into(),
            stderr: String::new(),
            exit_code: 0,
            fuel_consumed: 50_000,
            memory_peak: 1024,
            execution_time: Duration::from_millis(150),
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: WasmToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.stdout, "output");
        assert_eq!(restored.exit_code, 0);
        assert_eq!(restored.fuel_consumed, 50_000);
        assert_eq!(restored.execution_time, Duration::from_millis(150));
    }

    #[test]
    fn wasm_validation_serde_roundtrip() {
        let validation = WasmValidation {
            valid: true,
            exports: vec!["execute".into(), "tool_schema".into()],
            imports: vec!["wasi_snapshot_preview1::fd_write".into()],
            estimated_memory: 65536,
            warnings: vec!["uses wasi".into()],
        };
        let json = serde_json::to_string(&validation).unwrap();
        let restored: WasmValidation = serde_json::from_str(&json).unwrap();
        assert!(restored.valid);
        assert_eq!(restored.exports.len(), 2);
        assert_eq!(restored.imports.len(), 1);
    }
}
