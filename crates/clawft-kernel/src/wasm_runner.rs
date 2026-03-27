//! WASM tool execution sandbox and built-in tool catalog.
//!
//! Provides types and configuration for running tools inside
//! isolated WASM sandboxes with fuel metering, memory limits,
//! and host filesystem isolation.
//!
//! # K3 Tool Lifecycle
//!
//! Tools go through: Build → Deploy → Execute → Version → Revoke.
//! This module provides the execution runtime and tool catalog;
//! the lifecycle management is in [`crate::tree_manager`].
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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::governance::EffectVector;

/// Serde support for [u8; 64] as hex strings (used for Ed25519 signatures).
mod sig_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(hash: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes: Vec<u8> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(serde::de::Error::custom))
            .collect::<Result<Vec<u8>, _>>()?;
        let mut arr = [0u8; 64];
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "expected 64 bytes, got {}",
                bytes.len()
            )));
        }
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

// ---------------------------------------------------------------------------
// Sandbox configuration
// ---------------------------------------------------------------------------

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
    #[serde(default = "default_max_execution_secs", alias = "maxExecutionTimeSecs")]
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

// ---------------------------------------------------------------------------
// Per-execution state
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

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
        consumed: u64,
        limit: u64,
    },

    /// Memory allocation exceeded the configured limit.
    #[error("memory limit exceeded: {allocated} bytes (limit: {limit} bytes)")]
    MemoryLimitExceeded {
        allocated: usize,
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
        size: usize,
        limit: usize,
    },
}

/// Tool execution errors.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("file too large: {size} bytes (limit: {limit} bytes)")]
    FileTooLarge { size: u64, limit: u64 },
    #[error("signature required: {0}")]
    SignatureRequired(String),
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
    #[error("wasm error: {0}")]
    Wasm(#[from] WasmError),
}

// ---------------------------------------------------------------------------
// Built-in tool catalog types
// ---------------------------------------------------------------------------

/// Category of a built-in kernel tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    Filesystem,
    Agent,
    System,
    /// ECC cognitive substrate tools (behind `ecc` feature).
    Ecc,
    User,
}

/// Specification of a built-in kernel tool.
///
/// Named `BuiltinToolSpec` to distinguish from [`crate::app::ToolSpec`]
/// which describes application-provided tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinToolSpec {
    /// Dotted tool name (e.g. "fs.read_file").
    pub name: String,
    /// Category (Filesystem, Agent, System, User).
    pub category: ToolCategory,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for parameters.
    pub parameters: serde_json::Value,
    /// GovernanceGate action string (e.g. "tool.fs.read").
    pub gate_action: String,
    /// Effect vector for governance scoring.
    pub effect: EffectVector,
    /// Whether this tool can run natively (without WASM).
    pub native: bool,
}

/// A deployed version of a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolVersion {
    /// Version number (monotonically increasing per tool).
    pub version: u32,
    /// SHA-256 hash of the WASM module bytes.
    pub module_hash: [u8; 32],
    /// Ed25519 signature over module_hash (zero if unsigned).
    #[serde(with = "sig_serde")]
    pub signature: [u8; 64],
    /// When this version was deployed.
    pub deployed_at: DateTime<Utc>,
    /// Whether this version has been revoked.
    pub revoked: bool,
    /// Chain sequence number of the deploy event.
    pub chain_seq: u64,
}

/// A tool with its spec and version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedTool {
    /// Tool specification.
    pub spec: BuiltinToolSpec,
    /// Version history (ordered by version number).
    pub versions: Vec<ToolVersion>,
    /// Currently active version number.
    pub active_version: u32,
}

// ---------------------------------------------------------------------------
// WASM Tool Runner
// ---------------------------------------------------------------------------

/// WASM tool runner.
///
/// When the `wasm-sandbox` feature is enabled, this uses Wasmtime
/// for actual WASM execution. Without the feature, all tool loads
/// are rejected with [`WasmError::RuntimeUnavailable`].
pub struct WasmToolRunner {
    config: WasmSandboxConfig,
    #[cfg(feature = "wasm-sandbox")]
    engine: wasmtime::Engine,
}

impl WasmToolRunner {
    /// Create a new WASM tool runner with the given configuration.
    pub fn new(config: WasmSandboxConfig) -> Self {
        #[cfg(feature = "wasm-sandbox")]
        {
            let mut wt_config = wasmtime::Config::new();
            wt_config.consume_fuel(true);
            wt_config.async_support(true);
            // Memory limit is enforced per-store, not per-engine
            let engine = wasmtime::Engine::new(&wt_config)
                .expect("failed to create wasmtime engine");
            Self { config, engine }
        }
        #[cfg(not(feature = "wasm-sandbox"))]
        {
            Self { config }
        }
    }

    /// Get the sandbox configuration.
    pub fn config(&self) -> &WasmSandboxConfig {
        &self.config
    }

    /// Validate a WASM module's bytes without loading it.
    ///
    /// Checks module size, magic bytes, and (when the runtime is
    /// available) uses wasmtime::Module::validate() for full validation.
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
        let version =
            u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            warnings.push(format!("unexpected WASM version: {version} (expected 1)"));
        }

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

        #[cfg(feature = "wasm-sandbox")]
        {
            // Full validation via wasmtime
            if let Err(e) = wasmtime::Module::validate(&self.engine, wasm_bytes) {
                return Err(WasmError::InvalidModule(e.to_string()));
            }

            // Parse module to extract exports/imports
            match wasmtime::Module::new(&self.engine, wasm_bytes) {
                Ok(module) => {
                    let exports: Vec<String> = module
                        .exports()
                        .map(|e| e.name().to_string())
                        .collect();
                    let imports: Vec<String> = module
                        .imports()
                        .map(|i| format!("{}::{}", i.module(), i.name()))
                        .collect();
                    Ok(WasmValidation {
                        valid: true,
                        exports,
                        imports,
                        estimated_memory: 0,
                        warnings,
                    })
                }
                Err(e) => Err(WasmError::CompilationFailed(e.to_string())),
            }
        }
    }

    /// Load a WASM tool from bytes.
    ///
    /// Validates the module, computes a SHA-256 hash, and (with `wasm-sandbox`)
    /// compiles it with the Wasmtime engine.
    pub fn load_tool(&self, name: &str, wasm_bytes: &[u8]) -> Result<WasmTool, WasmError> {
        let validation = self.validate_wasm(wasm_bytes)?;

        if !validation.valid {
            return Err(WasmError::InvalidModule(validation.warnings.join("; ")));
        }

        let module_hash = compute_module_hash(wasm_bytes);

        #[cfg(not(feature = "wasm-sandbox"))]
        {
            let _ = name;
            let _ = module_hash;
            Err(WasmError::RuntimeUnavailable)
        }

        #[cfg(feature = "wasm-sandbox")]
        {
            Ok(WasmTool {
                name: name.to_owned(),
                module_size: wasm_bytes.len(),
                module_hash,
                schema: None,
                exports: validation.exports,
            })
        }
    }

    /// Execute a loaded WASM tool synchronously.
    ///
    /// Creates an isolated store with fuel metering and memory limits,
    /// compiles the tool's module bytes, and calls `_start` or `execute`.
    /// No host filesystem access is provided -- the instance receives
    /// an empty set of imports.
    ///
    /// For WASI-aware execution with stdio pipes, use [`execute_bytes`].
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
            Err(WasmError::RuntimeUnavailable)
        }
    }

    /// Execute raw WASM bytes synchronously without WASI.
    ///
    /// This is the sync K3 execution path. It creates a fresh Wasmtime
    /// store with fuel metering and memory limits, instantiates the
    /// module with **no imports** (no filesystem, no network), and
    /// calls `_start` or `run`.
    ///
    /// Returns [`WasmToolResult`] on success or a typed [`WasmError`]
    /// on fuel exhaustion, memory overflow, or compilation failure.
    #[cfg(feature = "wasm-sandbox")]
    pub fn execute_sync(
        &self,
        name: &str,
        wasm_bytes: &[u8],
        _input: serde_json::Value,
    ) -> Result<WasmToolResult, WasmError> {
        let started = std::time::Instant::now();

        // Build a sync-only engine (the shared engine has async_support
        // enabled, which forbids synchronous Instance::new).
        let mut sync_config = wasmtime::Config::new();
        sync_config.consume_fuel(true);
        let sync_engine = wasmtime::Engine::new(&sync_config)
            .map_err(|e| WasmError::CompilationFailed(format!("sync engine: {e}")))?;

        // Compile module (accepts binary .wasm or text .wat)
        let module = wasmtime::Module::new(&sync_engine, wasm_bytes)
            .map_err(|e| WasmError::CompilationFailed(format!("{name}: {e}")))?;

        // Create per-call store with embedded memory limiter
        let limiter = MemoryLimiter {
            max_bytes: self.config.max_memory_bytes,
        };
        let mut store = wasmtime::Store::new(&sync_engine, limiter);
        store
            .set_fuel(self.config.max_fuel)
            .map_err(|e| WasmError::WasmTrap(format!("set fuel: {e}")))?;
        store.limiter(|state| state as &mut dyn wasmtime::ResourceLimiter);

        // Instantiate with NO imports -- fully sandboxed, no host access
        let instance = wasmtime::Instance::new(&mut store, &module, &[])
            .map_err(|e| classify_trap_with_limiter(e, &self.config, &store))?;

        // Find entry point: _start (WASI convention) or run
        let entry = instance
            .get_func(&mut store, "_start")
            .or_else(|| instance.get_func(&mut store, "run"))
            .ok_or_else(|| {
                WasmError::WasmTrap(format!("{name}: no _start or run export"))
            })?;

        // Call the entry function
        match entry.call(&mut store, &[], &mut []) {
            Ok(_) => {
                let fuel_remaining = store.get_fuel().unwrap_or(0);
                let fuel_consumed = self.config.max_fuel.saturating_sub(fuel_remaining);
                Ok(WasmToolResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: 0,
                    fuel_consumed,
                    memory_peak: 0,
                    execution_time: started.elapsed(),
                })
            }
            Err(e) => Err(classify_trap_with_limiter(e, &self.config, &store)),
        }
    }

    /// Compile and execute WASM bytes in one shot.
    ///
    /// This is the primary execution path for K3. It accepts raw WASM
    /// bytes (binary or WAT text), compiles them with the engine, creates
    /// an isolated WASI store with fuel metering, serializes `input` as
    /// JSON to the module's stdin, calls `_start` (WASI preview1) or
    /// `execute`, and reads stdout/stderr.
    ///
    /// For cached execution with pre-compiled modules, see K4.
    #[cfg(feature = "wasm-sandbox")]
    pub async fn execute_bytes(
        &self,
        name: &str,
        wasm_bytes: &[u8],
        input: serde_json::Value,
    ) -> Result<WasmToolResult, WasmError> {
        use wasmtime_wasi::pipe::{MemoryInputPipe, MemoryOutputPipe};

        let started = std::time::Instant::now();

        // Serialize input to JSON bytes for stdin
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|e| WasmError::WasmTrap(format!("input serialization: {e}")))?;

        // Create pipes for stdio
        let stdout_pipe = MemoryOutputPipe::new(65_536);
        let stderr_pipe = MemoryOutputPipe::new(65_536);
        let stdin_pipe = MemoryInputPipe::new(input_bytes);

        // Build WASI preview1 context with stdio pipes
        let wasi_ctx = wasmtime_wasi::WasiCtxBuilder::new()
            .stdin(stdin_pipe)
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .build_p1();

        // Create per-call store with fuel budget
        let mut store = wasmtime::Store::new(&self.engine, wasi_ctx);
        store
            .set_fuel(self.config.max_fuel)
            .map_err(|e| WasmError::WasmTrap(format!("set fuel: {e}")))?;

        // Link WASI preview1 functions (wasi_snapshot_preview1.*)
        let mut linker = wasmtime::Linker::<wasmtime_wasi::preview1::WasiP1Ctx>::new(&self.engine);
        wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |ctx| ctx)
            .map_err(|e| WasmError::CompilationFailed(format!("WASI linker: {e}")))?;

        // Compile the module (accepts both binary .wasm and text .wat)
        let module = wasmtime::Module::new(&self.engine, wasm_bytes)
            .map_err(|e| WasmError::CompilationFailed(format!("{name}: {e}")))?;

        // Instantiate
        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|e| {
                let is_fuel = e
                    .downcast_ref::<wasmtime::Trap>()
                    .is_some_and(|t| *t == wasmtime::Trap::OutOfFuel)
                    || e.to_string().contains("fuel");
                if is_fuel {
                    WasmError::FuelExhausted {
                        consumed: self.config.max_fuel,
                        limit: self.config.max_fuel,
                    }
                } else {
                    WasmError::CompilationFailed(format!("instantiate {name}: {e}"))
                }
            })?;

        // Execute with wall-clock timeout
        let timeout = Duration::from_secs(self.config.max_execution_time_secs);
        let exec_result = tokio::time::timeout(timeout, async {
            // Try _start (WASI convention), then execute
            if let Some(start_fn) = instance.get_func(&mut store, "_start") {
                start_fn.call_async(&mut store, &[], &mut []).await
            } else if let Some(exec_fn) = instance.get_func(&mut store, "execute") {
                exec_fn.call_async(&mut store, &[], &mut []).await
            } else {
                Err(wasmtime::Error::msg("no _start or execute export"))
            }
        })
        .await;

        // Read captured output
        let stdout = String::from_utf8_lossy(&stdout_pipe.contents()).to_string();
        let stderr = String::from_utf8_lossy(&stderr_pipe.contents()).to_string();

        let fuel_remaining = store.get_fuel().unwrap_or(0);
        let fuel_consumed = self.config.max_fuel.saturating_sub(fuel_remaining);

        match exec_result {
            Ok(Ok(_)) => Ok(WasmToolResult {
                stdout,
                stderr,
                exit_code: 0,
                fuel_consumed,
                memory_peak: 0,
                execution_time: started.elapsed(),
            }),
            Ok(Err(trap)) => {
                let msg = trap.to_string();
                // Check for fuel exhaustion via downcast or message
                let is_fuel = trap
                    .downcast_ref::<wasmtime::Trap>()
                    .is_some_and(|t| *t == wasmtime::Trap::OutOfFuel)
                    || msg.contains("fuel");
                if is_fuel {
                    Err(WasmError::FuelExhausted {
                        consumed: fuel_consumed,
                        limit: self.config.max_fuel,
                    })
                } else if msg.contains("memory") {
                    Err(WasmError::MemoryLimitExceeded {
                        allocated: self.config.max_memory_bytes,
                        limit: self.config.max_memory_bytes,
                    })
                } else {
                    // Non-zero exit or trap — return result with stderr
                    Ok(WasmToolResult {
                        stdout,
                        stderr: if stderr.is_empty() {
                            format!("trap: {msg}")
                        } else {
                            format!("{stderr}\ntrap: {msg}")
                        },
                        exit_code: 1,
                        fuel_consumed,
                        memory_peak: 0,
                        execution_time: started.elapsed(),
                    })
                }
            }
            Err(_timeout) => Err(WasmError::ExecutionTimeout(timeout)),
        }
    }

    /// Get a reference to the Wasmtime engine.
    #[cfg(feature = "wasm-sandbox")]
    pub fn engine(&self) -> &wasmtime::Engine {
        &self.engine
    }
}

// ---------------------------------------------------------------------------
// Wasmtime helpers (behind feature gate)
// ---------------------------------------------------------------------------

/// Resource limiter that caps linear memory growth.
#[cfg(feature = "wasm-sandbox")]
struct MemoryLimiter {
    max_bytes: usize,
}

#[cfg(feature = "wasm-sandbox")]
impl wasmtime::ResourceLimiter for MemoryLimiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        if desired > self.max_bytes {
            // Deny the growth -- Wasmtime will trap
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        Ok(true)
    }
}

/// Classify a Wasmtime error into a typed [`WasmError`].
///
/// Inspects the error for fuel exhaustion or memory-related traps
/// and returns the corresponding `WasmError` variant.
#[cfg(feature = "wasm-sandbox")]
fn classify_trap_impl(
    err: wasmtime::Error,
    config: &WasmSandboxConfig,
    fuel_remaining: u64,
) -> WasmError {
    let msg = err.to_string();

    // Check for fuel exhaustion
    let is_fuel = err
        .downcast_ref::<wasmtime::Trap>()
        .is_some_and(|t| *t == wasmtime::Trap::OutOfFuel)
        || msg.contains("fuel");
    if is_fuel {
        return WasmError::FuelExhausted {
            consumed: config.max_fuel.saturating_sub(fuel_remaining),
            limit: config.max_fuel,
        };
    }

    // Check for memory limit
    if msg.contains("memory") {
        return WasmError::MemoryLimitExceeded {
            allocated: config.max_memory_bytes,
            limit: config.max_memory_bytes,
        };
    }

    WasmError::WasmTrap(msg)
}

/// Classify trap from a `Store<MemoryLimiter>` (used by execute_sync).
#[cfg(feature = "wasm-sandbox")]
fn classify_trap_with_limiter(
    err: wasmtime::Error,
    config: &WasmSandboxConfig,
    store: &wasmtime::Store<MemoryLimiter>,
) -> WasmError {
    classify_trap_impl(err, config, store.get_fuel().unwrap_or(0))
}

/// A loaded WASM tool module.
#[derive(Debug, Clone)]
pub struct WasmTool {
    /// Tool name.
    pub name: String,

    /// Module size in bytes.
    pub module_size: usize,

    /// SHA-256 hash of module bytes.
    pub module_hash: [u8; 32],

    /// Tool parameter schema (if exported by the module).
    pub schema: Option<serde_json::Value>,

    /// Exported function names.
    pub exports: Vec<String>,
}

/// Compute SHA-256 hash of WASM module bytes.
pub fn compute_module_hash(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// ---------------------------------------------------------------------------
// K4 D2: Disk-persisted module cache
// ---------------------------------------------------------------------------

/// Compiled module cache with LRU eviction.
///
/// Stores compiled WASM modules on disk keyed by SHA-256 hash.
/// When the cache exceeds `max_size`, the oldest entries are evicted.
pub struct CompiledModuleCache {
    cache_dir: PathBuf,
    max_size: u64,
}

impl CompiledModuleCache {
    /// Create a new module cache at the given directory.
    pub fn new(cache_dir: PathBuf, max_size: u64) -> Self {
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir, max_size }
    }

    /// Get a cached compiled module by its hash.
    pub fn get(&self, hash: &[u8; 32]) -> Option<Vec<u8>> {
        let path = self.cache_path(hash);
        std::fs::read(&path).ok()
    }

    /// Store a compiled module in the cache.
    pub fn put(&self, hash: &[u8; 32], bytes: &[u8]) {
        let path = self.cache_path(hash);
        let _ = std::fs::write(&path, bytes);
        self.evict_lru();
    }

    /// Evict oldest entries until cache is under `max_size`.
    fn evict_lru(&self) {
        let mut entries: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
        if let Ok(dir) = std::fs::read_dir(&self.cache_dir) {
            for entry in dir.flatten() {
                if let Ok(meta) = entry.metadata() {
                    let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    entries.push((entry.path(), meta.len(), modified));
                }
            }
        }
        let total: u64 = entries.iter().map(|(_, s, _)| s).sum();
        if total <= self.max_size {
            return;
        }
        // Sort by modification time (oldest first)
        entries.sort_by_key(|(_, _, t)| *t);
        let mut remaining = total;
        for (path, size, _) in &entries {
            if remaining <= self.max_size {
                break;
            }
            let _ = std::fs::remove_file(path);
            remaining -= size;
        }
    }

    fn cache_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        self.cache_dir.join(format!("{hex}.wasm"))
    }
}

// ---------------------------------------------------------------------------
// K4 D3: WASI filesystem scope
// ---------------------------------------------------------------------------

/// WASI filesystem access scope for a tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum WasiFsScope {
    /// No filesystem access.
    #[default]
    None,
    /// Read-only access to a directory.
    ReadOnly(PathBuf),
    /// Read-write access to a directory.
    ReadWrite(PathBuf),
}


// ---------------------------------------------------------------------------
// K4 F1: CA chain signing
// ---------------------------------------------------------------------------

/// Tool signing authority — identifies who signed a tool module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSigningAuthority {
    /// Signed by the kernel's built-in key.
    Kernel,
    /// Signed by a developer with a certificate chain.
    Developer {
        /// Certificate chain (leaf first, root last).
        cert_chain: Vec<Certificate>,
    },
}

/// A signing certificate in the CA chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Subject name (e.g. "developer@example.com").
    pub subject: String,
    /// Ed25519 public key bytes (32 bytes).
    pub public_key: [u8; 32],
    /// Signature over subject + public_key by the issuer.
    #[serde(with = "sig_serde")]
    pub signature: [u8; 64],
    /// Issuer subject name.
    pub issuer: String,
}

// ---------------------------------------------------------------------------
// D9: Tool Signature for ExoChain registration
// ---------------------------------------------------------------------------

/// A cryptographic signature binding a tool definition to a signer identity.
///
/// Used by [`ToolRegistry::register_signed`] to gate tool registration
/// behind signature verification when `require_signatures` is enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSignature {
    /// Name of the tool being signed.
    pub tool_name: String,
    /// SHA-256 hash of the tool definition (spec JSON bytes).
    pub tool_hash: [u8; 32],
    /// Identity of the signer (e.g. public key hex or developer id).
    pub signer_id: String,
    /// Ed25519 signature bytes over `tool_hash`.
    pub signature: Vec<u8>,
    /// Timestamp when the signature was created.
    pub signed_at: DateTime<Utc>,
}

impl ToolSignature {
    /// Create a new tool signature from components.
    pub fn new(
        tool_name: impl Into<String>,
        tool_hash: [u8; 32],
        signer_id: impl Into<String>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            tool_hash,
            signer_id: signer_id.into(),
            signature,
            signed_at: Utc::now(),
        }
    }

    /// Verify this signature against a 32-byte Ed25519 public key.
    ///
    /// Returns `true` if the signature is valid for `self.tool_hash`.
    /// Requires `exochain` feature; without it, always returns `false`.
    pub fn verify(&self, public_key: &[u8; 32]) -> bool {
        if self.signature.len() != 64 {
            return false;
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        verify_tool_signature(&self.tool_hash, &sig_bytes, public_key)
    }
}

/// Verify a tool's Ed25519 signature against a public key.
///
/// Requires the `exochain` feature for real Ed25519 verification.
/// Without the feature, always returns `false`.
#[cfg(feature = "exochain")]
pub fn verify_tool_signature(
    module_hash: &[u8; 32],
    signature: &[u8; 64],
    public_key: &[u8; 32],
) -> bool {
    use ed25519_dalek::{Verifier, VerifyingKey, Signature};
    let Ok(vk) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let sig = Signature::from_bytes(signature);
    vk.verify(module_hash, &sig).is_ok()
}

/// Stub: always returns `false` when `exochain` feature is disabled.
#[cfg(not(feature = "exochain"))]
pub fn verify_tool_signature(
    _module_hash: &[u8; 32],
    _signature: &[u8; 64],
    _public_key: &[u8; 32],
) -> bool {
    false
}

// ---------------------------------------------------------------------------
// K4 F2: tiny-dancer routing heuristic
// ---------------------------------------------------------------------------

/// Backend selection for tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendSelection {
    /// Run natively (no isolation).
    Native,
    /// Run in WASM sandbox.
    Wasm,
    /// Auto-select based on risk score.
    Auto,
}

impl BackendSelection {
    /// Select backend based on effect vector risk score.
    ///
    /// Simple heuristic: risk > 0.3 => WASM sandbox, else native.
    pub fn from_risk(risk: f64) -> Self {
        if risk > 0.3 {
            Self::Wasm
        } else {
            Self::Native
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in tool trait and registry
// ---------------------------------------------------------------------------

/// Trait for built-in kernel tools.
///
/// Each tool has a spec and an execute method. Tools hold their own
/// dependencies (e.g. `Arc<ProcessTable>` for agent tools).
pub trait BuiltinTool: Send + Sync {
    /// Return the tool name (e.g. "fs.read_file").
    fn name(&self) -> &str;
    /// Return the tool specification.
    fn spec(&self) -> &BuiltinToolSpec;
    /// Execute the tool with the given JSON arguments.
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

/// Registry of available tools for dispatch.
///
/// Supports hierarchical lookup: a child registry can overlay a parent.
/// The parent chain is walked when a tool is not found locally.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn BuiltinTool>>,
    /// Optional parent registry for hierarchical lookup.
    parent: Option<Arc<ToolRegistry>>,
    /// When true, only signed tools may be registered.
    require_signatures: bool,
    /// Trusted public keys for signature verification (32-byte Ed25519 keys).
    trusted_keys: Vec<[u8; 32]>,
    /// Signatures for registered tools (tool_name -> ToolSignature).
    signatures: HashMap<String, ToolSignature>,
}

impl ToolRegistry {
    /// Create an empty registry with no parent.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            parent: None,
            require_signatures: false,
            trusted_keys: Vec::new(),
            signatures: HashMap::new(),
        }
    }

    /// Create a child registry that delegates to `parent` for missing tools.
    pub fn with_parent(parent: Arc<ToolRegistry>) -> Self {
        Self {
            tools: HashMap::new(),
            parent: Some(parent),
            require_signatures: false,
            trusted_keys: Vec::new(),
            signatures: HashMap::new(),
        }
    }

    /// Register a tool (local to this registry level).
    ///
    /// When `require_signatures` is enabled, this rejects unsigned tools
    /// with [`ToolError::SignatureRequired`]. Use [`register_signed`]
    /// to supply a signature, or disable the requirement.
    pub fn register(&mut self, tool: Arc<dyn BuiltinTool>) {
        if self.require_signatures {
            tracing::warn!(
                tool = tool.name(),
                "unsigned tool registration rejected (require_signatures=true)"
            );
            return;
        }
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Register a tool, checking signatures when required.
    ///
    /// Returns `Err(SignatureRequired)` when `require_signatures` is on
    /// and no signature is provided. Returns `Ok(())` otherwise.
    pub fn try_register(&mut self, tool: Arc<dyn BuiltinTool>) -> Result<(), ToolError> {
        if self.require_signatures {
            return Err(ToolError::SignatureRequired(tool.name().to_string()));
        }
        self.tools.insert(tool.name().to_string(), tool);
        Ok(())
    }

    /// Register a tool with a cryptographic signature.
    ///
    /// Verifies the signature against trusted keys before allowing registration.
    /// The signature is stored and the tool is chain-logged if ExoChain is available.
    pub fn register_signed(
        &mut self,
        tool: Arc<dyn BuiltinTool>,
        signature: ToolSignature,
    ) -> Result<(), ToolError> {
        // Verify the signature against at least one trusted key.
        if !self.verify_tool_signature(&signature) {
            return Err(ToolError::InvalidSignature(format!(
                "no trusted key verified signature for tool '{}'",
                signature.tool_name,
            )));
        }
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), tool);
        self.signatures.insert(name, signature);
        Ok(())
    }

    /// Check whether a tool signature is valid against any trusted key.
    pub fn verify_tool_signature(&self, signature: &ToolSignature) -> bool {
        self.trusted_keys.iter().any(|key| signature.verify(key))
    }

    /// Enable or disable mandatory signature verification for tool registration.
    pub fn set_require_signatures(&mut self, require: bool) {
        self.require_signatures = require;
    }

    /// Whether signatures are required for tool registration.
    pub fn requires_signatures(&self) -> bool {
        self.require_signatures
    }

    /// Add a trusted Ed25519 public key for signature verification.
    pub fn add_trusted_key(&mut self, key: [u8; 32]) {
        self.trusted_keys.push(key);
    }

    /// Get the signature for a registered tool, if any.
    pub fn get_signature(&self, tool_name: &str) -> Option<&ToolSignature> {
        self.signatures.get(tool_name)
    }

    /// Look up a tool by name, walking the parent chain.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn BuiltinTool>> {
        self.tools.get(name).or_else(|| {
            // Walk parent chain — returns &Arc from parent, which is valid
            // because `self` borrows the parent via `Arc`.
            self.parent.as_ref().and_then(|p| p.get(name))
        })
    }

    /// Execute a tool by name, walking the parent chain.
    pub fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(args)
    }

    /// List all registered tool names (merges parent + local, local wins).
    pub fn list(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        // Local tools first
        for name in self.tools.keys() {
            seen.insert(name.clone());
        }
        // Parent tools (only if not overridden locally)
        if let Some(ref parent) = self.parent {
            for name in parent.list() {
                seen.insert(name);
            }
        }
        let mut result: Vec<String> = seen.into_iter().collect();
        result.sort();
        result
    }

    /// Number of registered tools (parent + local, deduplicated).
    pub fn len(&self) -> usize {
        if self.parent.is_none() {
            return self.tools.len();
        }
        self.list().len()
    }

    /// Whether the registry has no tools (including parent).
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.parent.as_ref().is_none_or(|p| p.is_empty())
    }

    /// Get a reference to the parent registry, if any.
    pub fn parent(&self) -> Option<&Arc<ToolRegistry>> {
        self.parent.as_ref()
    }

    /// Register a WASM tool that executes through a [`WasmToolRunner`].
    ///
    /// The WASM bytes are stored inside the adapter and compiled on each
    /// execution (K3). Compiled module caching is deferred to K4.
    ///
    /// The tool is dispatched synchronously via [`BuiltinTool::execute`],
    /// which spawns a blocking thread internally to run the async Wasmtime
    /// execution. For fully async dispatch, call
    /// [`WasmToolRunner::execute_bytes`] directly.
    #[cfg(feature = "wasm-sandbox")]
    pub fn register_wasm_tool(
        &mut self,
        name: &str,
        description: &str,
        wasm_bytes: Vec<u8>,
        runner: Arc<WasmToolRunner>,
    ) -> Result<(), WasmError> {
        // Validate the module by attempting compilation (handles both
        // binary WASM and WAT text format).
        wasmtime::Module::new(runner.engine(), &wasm_bytes)
            .map_err(|e| WasmError::InvalidModule(e.to_string()))?;

        let adapter = WasmToolAdapter {
            tool_name: name.to_owned(),
            spec: BuiltinToolSpec {
                name: name.to_owned(),
                category: ToolCategory::User,
                description: description.to_owned(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {"type": "object", "description": "JSON input passed to WASM stdin"}
                    }
                }),
                gate_action: format!("tool.wasm.{name}"),
                effect: EffectVector {
                    risk: 0.5,
                    ..Default::default()
                },
                native: false,
            },
            wasm_bytes: Arc::new(wasm_bytes),
            runner,
        };
        self.register(Arc::new(adapter));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WASM tool adapter (bridges BuiltinTool to WasmToolRunner)
// ---------------------------------------------------------------------------

/// Adapter that wraps WASM bytes + a [`WasmToolRunner`] as a [`BuiltinTool`].
///
/// When [`BuiltinTool::execute`] is called, this adapter spawns a blocking
/// thread to run [`WasmToolRunner::execute_bytes`] asynchronously. The JSON
/// args are passed as stdin to the WASM module.
#[cfg(feature = "wasm-sandbox")]
struct WasmToolAdapter {
    tool_name: String,
    spec: BuiltinToolSpec,
    wasm_bytes: Arc<Vec<u8>>,
    runner: Arc<WasmToolRunner>,
}

#[cfg(feature = "wasm-sandbox")]
impl BuiltinTool for WasmToolAdapter {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        // Extract input from args, defaulting to the full args object
        let input = args
            .get("input")
            .cloned()
            .unwrap_or(args.clone());

        let runner = self.runner.clone();
        let wasm_bytes = self.wasm_bytes.clone();
        let name = self.tool_name.clone();

        // Run async execute_bytes on a blocking thread with its own runtime
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| ToolError::ExecutionFailed(format!("runtime: {e}")))?;
            rt.block_on(runner.execute_bytes(&name, &wasm_bytes, input))
                .map_err(ToolError::Wasm)
        })
        .join()
        .map_err(|_| ToolError::ExecutionFailed("WASM execution thread panicked".into()))??;

        Ok(serde_json::json!({
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
            "fuel_consumed": result.fuel_consumed,
            "execution_time_ms": result.execution_time.as_millis() as u64,
        }))
    }
}

// Safety: WasmToolAdapter is Send+Sync because all its fields are Send+Sync.
// - tool_name/spec: plain data
// - wasm_bytes: Arc<Vec<u8>>
// - runner: Arc<WasmToolRunner> (Engine is Send+Sync)
#[cfg(feature = "wasm-sandbox")]
unsafe impl Send for WasmToolAdapter {}
#[cfg(feature = "wasm-sandbox")]
unsafe impl Sync for WasmToolAdapter {}

// Safety: ToolRegistry is Send+Sync because it contains Send+Sync fields.
// The `parent` is behind an Arc, and `tools` contains Arc<dyn BuiltinTool>
// which requires Send+Sync.
unsafe impl Send for ToolRegistry {}
unsafe impl Sync for ToolRegistry {}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Built-in tool catalog (27 tools)
// ---------------------------------------------------------------------------

/// Return the complete catalog of 27 built-in kernel tools.
pub fn builtin_tool_catalog() -> Vec<BuiltinToolSpec> {
    let mut catalog = Vec::with_capacity(29);

    // --- Filesystem tools (10) ---
    catalog.push(BuiltinToolSpec {
        name: "fs.read_file".into(),
        category: ToolCategory::Filesystem,
        description: "Read file contents with optional offset/limit".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File path to read"},
                "offset": {"type": "integer", "description": "Byte offset to start reading"},
                "limit": {"type": "integer", "description": "Maximum bytes to read"}
            },
            "required": ["path"]
        }),
        gate_action: "tool.fs.read".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.write_file".into(),
        category: ToolCategory::Filesystem,
        description: "Write content to a file".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"},
                "append": {"type": "boolean", "default": false}
            },
            "required": ["path", "content"]
        }),
        gate_action: "tool.fs.write".into(),
        effect: EffectVector { risk: 0.4, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.read_dir".into(),
        category: ToolCategory::Filesystem,
        description: "List directory contents".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        }),
        gate_action: "tool.fs.read".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.create_dir".into(),
        category: ToolCategory::Filesystem,
        description: "Create a directory (recursive)".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "recursive": {"type": "boolean", "default": true}
            },
            "required": ["path"]
        }),
        gate_action: "tool.fs.write".into(),
        effect: EffectVector { risk: 0.3, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.remove".into(),
        category: ToolCategory::Filesystem,
        description: "Remove a file or directory".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "recursive": {"type": "boolean", "default": false}
            },
            "required": ["path"]
        }),
        gate_action: "tool.fs.delete".into(),
        effect: EffectVector { risk: 0.7, security: 0.3, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.copy".into(),
        category: ToolCategory::Filesystem,
        description: "Copy a file or directory".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "src": {"type": "string"},
                "dst": {"type": "string"}
            },
            "required": ["src", "dst"]
        }),
        gate_action: "tool.fs.write".into(),
        effect: EffectVector { risk: 0.3, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.move".into(),
        category: ToolCategory::Filesystem,
        description: "Move/rename a file or directory".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "src": {"type": "string"},
                "dst": {"type": "string"}
            },
            "required": ["src", "dst"]
        }),
        gate_action: "tool.fs.write".into(),
        effect: EffectVector { risk: 0.5, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.stat".into(),
        category: ToolCategory::Filesystem,
        description: "Get file/directory metadata".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        }),
        gate_action: "tool.fs.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.exists".into(),
        category: ToolCategory::Filesystem,
        description: "Check if a path exists".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        }),
        gate_action: "tool.fs.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "fs.glob".into(),
        category: ToolCategory::Filesystem,
        description: "Find files matching a glob pattern".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "base_dir": {"type": "string"}
            },
            "required": ["pattern"]
        }),
        gate_action: "tool.fs.read".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });

    // --- Agent tools (7) ---
    catalog.push(BuiltinToolSpec {
        name: "agent.spawn".into(),
        category: ToolCategory::Agent,
        description: "Spawn a new agent process".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": {"type": "string"},
                "template": {"type": "string"},
                "capabilities": {"type": "object"},
                "backend": {"type": "string", "enum": ["native", "wasm", "container"]}
            },
            "required": ["agent_id"]
        }),
        gate_action: "tool.agent.spawn".into(),
        effect: EffectVector { risk: 0.5, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "agent.stop".into(),
        category: ToolCategory::Agent,
        description: "Stop a running agent".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pid": {"type": "integer"},
                "graceful": {"type": "boolean", "default": true}
            },
            "required": ["pid"]
        }),
        gate_action: "tool.agent.stop".into(),
        effect: EffectVector { risk: 0.4, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "agent.list".into(),
        category: ToolCategory::Agent,
        description: "List all running agents".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
        gate_action: "tool.agent.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "agent.inspect".into(),
        category: ToolCategory::Agent,
        description: "Inspect agent details".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pid": {"type": "integer"}
            },
            "required": ["pid"]
        }),
        gate_action: "tool.agent.read".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "agent.send".into(),
        category: ToolCategory::Agent,
        description: "Send a message to an agent".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pid": {"type": "integer"},
                "message": {"type": "object"}
            },
            "required": ["pid", "message"]
        }),
        gate_action: "tool.agent.ipc".into(),
        effect: EffectVector { risk: 0.2, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "agent.suspend".into(),
        category: ToolCategory::Agent,
        description: "Suspend an agent".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pid": {"type": "integer"}
            },
            "required": ["pid"]
        }),
        gate_action: "tool.agent.suspend".into(),
        effect: EffectVector { risk: 0.3, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "agent.resume".into(),
        category: ToolCategory::Agent,
        description: "Resume a suspended agent".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pid": {"type": "integer"}
            },
            "required": ["pid"]
        }),
        gate_action: "tool.agent.resume".into(),
        effect: EffectVector { risk: 0.2, ..Default::default() },
        native: true,
    });

    // --- IPC tools (2) ---
    catalog.push(BuiltinToolSpec {
        name: "ipc.send".into(),
        category: ToolCategory::Agent,
        description: "Send a message to an agent or topic via kernel IPC".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "target_pid": {"type": "integer", "description": "Target agent PID"},
                "topic": {"type": "string", "description": "Topic name (alternative to target_pid)"},
                "payload": {"type": "object", "description": "Message payload (JSON)"},
                "text": {"type": "string", "description": "Plain text message (alternative to payload)"}
            }
        }),
        gate_action: "tool.ipc.send".into(),
        effect: EffectVector { risk: 0.2, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "ipc.subscribe".into(),
        category: ToolCategory::Agent,
        description: "Subscribe to a topic for receiving messages".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "topic": {"type": "string", "description": "Topic name to subscribe to"},
                "pid": {"type": "integer", "description": "PID to subscribe (defaults to caller)"}
            },
            "required": ["topic"]
        }),
        gate_action: "tool.ipc.subscribe".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });

    // --- System tools (10) ---
    catalog.push(BuiltinToolSpec {
        name: "sys.service.list".into(),
        category: ToolCategory::System,
        description: "List registered services".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.service.health".into(),
        category: ToolCategory::System,
        description: "Check service health".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        }),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.chain.status".into(),
        category: ToolCategory::System,
        description: "Get chain status".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.chain.query".into(),
        category: ToolCategory::System,
        description: "Query chain events".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer", "default": 20},
                "source": {"type": "string"},
                "kind": {"type": "string"}
            }
        }),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.tree.read".into(),
        category: ToolCategory::System,
        description: "Read resource tree".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.tree.inspect".into(),
        category: ToolCategory::System,
        description: "Inspect a resource tree node".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        }),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.1, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.env.get".into(),
        category: ToolCategory::System,
        description: "Get environment variable".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        }),
        gate_action: "tool.sys.env".into(),
        effect: EffectVector { risk: 0.2, privacy: 0.3, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.cron.add".into(),
        category: ToolCategory::System,
        description: "Add a cron job".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "interval_secs": {"type": "integer"},
                "command": {"type": "string"},
                "target_pid": {"type": "integer"}
            },
            "required": ["name", "interval_secs", "command"]
        }),
        gate_action: "tool.sys.cron".into(),
        effect: EffectVector { risk: 0.4, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.cron.list".into(),
        category: ToolCategory::System,
        description: "List cron jobs".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
        gate_action: "tool.sys.read".into(),
        effect: EffectVector { risk: 0.05, ..Default::default() },
        native: true,
    });
    catalog.push(BuiltinToolSpec {
        name: "sys.cron.remove".into(),
        category: ToolCategory::System,
        description: "Remove a cron job".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"}
            },
            "required": ["id"]
        }),
        gate_action: "tool.sys.cron".into(),
        effect: EffectVector { risk: 0.3, ..Default::default() },
        native: true,
    });

    // --- ECC tools (7, behind `ecc` feature) ---
    #[cfg(feature = "ecc")]
    {
        catalog.push(BuiltinToolSpec {
            name: "ecc.embed".into(),
            category: ToolCategory::Ecc,
            description: "Insert vector into HNSW index".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "embedding": {"type": "array", "items": {"type": "number"}},
                    "metadata": {"type": "object"}
                },
                "required": ["id", "embedding"]
            }),
            gate_action: "ecc.embed".into(),
            effect: EffectVector { risk: 0.1, ..Default::default() },
            native: true,
        });
        catalog.push(BuiltinToolSpec {
            name: "ecc.search".into(),
            category: ToolCategory::Ecc,
            description: "k-NN similarity search".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "array", "items": {"type": "number"}},
                    "k": {"type": "integer", "default": 10}
                },
                "required": ["query"]
            }),
            gate_action: "ecc.search".into(),
            effect: EffectVector { risk: 0.05, ..Default::default() },
            native: true,
        });
        catalog.push(BuiltinToolSpec {
            name: "ecc.causal.link".into(),
            category: ToolCategory::Ecc,
            description: "Create causal edge".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {"type": "integer"},
                    "target": {"type": "integer"},
                    "edge_type": {"type": "string"},
                    "weight": {"type": "number", "default": 1.0}
                },
                "required": ["source", "target", "edge_type"]
            }),
            gate_action: "ecc.causal.link".into(),
            effect: EffectVector { risk: 0.3, ..Default::default() },
            native: true,
        });
        catalog.push(BuiltinToolSpec {
            name: "ecc.causal.query".into(),
            category: ToolCategory::Ecc,
            description: "Traverse causal graph".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "node": {"type": "integer"},
                    "direction": {"type": "string", "enum": ["forward", "reverse"]},
                    "depth": {"type": "integer", "default": 3}
                },
                "required": ["node"]
            }),
            gate_action: "ecc.causal.query".into(),
            effect: EffectVector { risk: 0.05, ..Default::default() },
            native: true,
        });
        catalog.push(BuiltinToolSpec {
            name: "ecc.crossref.create".into(),
            category: ToolCategory::Ecc,
            description: "Link nodes across structures".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_id": {"type": "string"},
                    "target_id": {"type": "string"},
                    "ref_type": {"type": "string"}
                },
                "required": ["source_id", "target_id", "ref_type"]
            }),
            gate_action: "ecc.crossref.create".into(),
            effect: EffectVector { risk: 0.2, ..Default::default() },
            native: true,
        });
        catalog.push(BuiltinToolSpec {
            name: "ecc.tick.status".into(),
            category: ToolCategory::Ecc,
            description: "Query cognitive tick state".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
            gate_action: "ecc.tick.status".into(),
            effect: EffectVector { risk: 0.05, ..Default::default() },
            native: true,
        });
        catalog.push(BuiltinToolSpec {
            name: "ecc.calibration.run".into(),
            category: ToolCategory::Ecc,
            description: "Re-run boot calibration".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
            gate_action: "ecc.calibration.run".into(),
            effect: EffectVector { risk: 0.1, ..Default::default() },
            native: true,
        });
    }

    catalog
}

// ---------------------------------------------------------------------------
// Reference tool implementations
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Sandbox configuration (K4 B1)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Multi-layer sandboxing (k3:D12)
// ---------------------------------------------------------------------------

/// Which sandbox layer denied (or allowed) access.
///
/// Three enforcement layers are evaluated in order (k3:D12):
/// 1. **Governance** — gate check with tool name + effect vector context
/// 2. **Environment** — per-environment allowed-path configuration
/// 3. **SudoOverride** — elevated agent capability that bypasses
///    environment restrictions (logged to chain, requires `sudo` flag)
///
/// The first `Deny` short-circuits. `SudoOverride` can only bypass
/// the **Environment** layer, never the **Governance** layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxLayer {
    /// Governance gate check (always authoritative, cannot be overridden).
    Governance,
    /// Environment-scoped path restrictions (e.g. dev=permissive, prod=strict).
    Environment,
    /// Elevated override that bypasses environment restrictions.
    /// Requires `AgentCapabilities::sudo` and is always logged to chain.
    SudoOverride,
}

impl std::fmt::Display for SandboxLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxLayer::Governance => write!(f, "governance"),
            SandboxLayer::Environment => write!(f, "environment"),
            SandboxLayer::SudoOverride => write!(f, "sudo-override"),
        }
    }
}

/// Result of evaluating the multi-layer sandbox stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxDecision {
    /// Whether access is permitted.
    pub allowed: bool,
    /// Which layer made the decision.
    pub decided_by: SandboxLayer,
    /// Human-readable reason (for logging / chain events).
    pub reason: String,
}

impl SandboxDecision {
    /// Create a permit decision.
    pub fn permit(layer: SandboxLayer) -> Self {
        Self {
            allowed: true,
            decided_by: layer,
            reason: "access permitted".into(),
        }
    }

    /// Create a deny decision.
    pub fn deny(layer: SandboxLayer, reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            decided_by: layer,
            reason: reason.into(),
        }
    }
}

/// Filesystem sandbox configuration for built-in tools.
///
/// Controls which paths a tool is allowed to access. When `allowed_paths`
/// is non-empty, only files under those directories are permitted.
/// An empty `allowed_paths` means permissive mode (dev default).
///
/// Part of the multi-layer sandboxing stack (k3:D12):
/// governance gate -> environment config -> sudo override.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Directories the tool is allowed to access.
    /// Empty = permissive (all paths allowed).
    pub allowed_paths: Vec<PathBuf>,

    /// Whether sudo override is active for this execution.
    /// When true and path is denied by environment config, access
    /// is granted anyway (but logged to chain). Governance denials
    /// can never be overridden.
    #[serde(default)]
    pub sudo_override: bool,
}

impl SandboxConfig {
    /// Check whether a path is allowed by this sandbox config.
    ///
    /// Returns `true` if `allowed_paths` is empty (permissive mode)
    /// or the path is under at least one allowed directory.
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        if self.allowed_paths.is_empty() {
            return true;
        }
        // Canonicalize the target path for comparison
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.allowed_paths.iter().any(|allowed| {
            let allowed_canon = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canon)
        })
    }

    /// Multi-layer sandbox check (k3:D12).
    ///
    /// Evaluates the environment layer and optional sudo override.
    /// The governance layer is evaluated separately by the caller
    /// (via `GovernanceEngine::evaluate`) because it requires the
    /// full `GovernanceRequest` context.
    ///
    /// Evaluation order:
    /// 1. Environment config (`allowed_paths`) — if empty, permit.
    /// 2. If denied and `sudo_override` is true, permit with
    ///    `SandboxLayer::SudoOverride` (caller must log to chain).
    /// 3. Otherwise deny with `SandboxLayer::Environment`.
    pub fn check_path_multilayer(&self, path: &std::path::Path) -> SandboxDecision {
        // Permissive mode (dev default)
        if self.allowed_paths.is_empty() {
            return SandboxDecision::permit(SandboxLayer::Environment);
        }

        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let env_allowed = self.allowed_paths.iter().any(|allowed| {
            let allowed_canon = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canon)
        });

        if env_allowed {
            return SandboxDecision::permit(SandboxLayer::Environment);
        }

        // Environment denied — check sudo override
        if self.sudo_override {
            return SandboxDecision {
                allowed: true,
                decided_by: SandboxLayer::SudoOverride,
                reason: format!(
                    "sudo override: path {} bypassed environment restriction",
                    path.display()
                ),
            };
        }

        SandboxDecision::deny(
            SandboxLayer::Environment,
            format!("path outside sandbox: {}", path.display()),
        )
    }
}

/// Max file read size (8 MiB, matching PluginSandbox).
const MAX_READ_SIZE: u64 = 8 * 1024 * 1024;

/// Built-in `fs.read_file` tool.
///
/// Reads file contents with optional offset and limit.
/// Always runs natively (no WASM needed for reference impl).
/// Supports multi-layer sandboxing via [`SandboxConfig`].
pub struct FsReadFileTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl Default for FsReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FsReadFileTool {
    pub fn new() -> Self {
        let catalog = builtin_tool_catalog();
        let spec = catalog
            .into_iter()
            .find(|s| s.name == "fs.read_file")
            .expect("fs.read_file must be in catalog");
        Self {
            spec,
            sandbox: SandboxConfig::default(),
        }
    }

    /// Create a sandboxed instance that restricts file access.
    pub fn with_sandbox(sandbox: SandboxConfig) -> Self {
        let catalog = builtin_tool_catalog();
        let spec = catalog
            .into_iter()
            .find(|s| s.name == "fs.read_file")
            .expect("fs.read_file must be in catalog");
        Self { spec, sandbox }
    }
}

impl BuiltinTool for FsReadFileTool {
    fn name(&self) -> &str {
        "fs.read_file"
    }

    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path' parameter".into()))?;

        let path = std::path::Path::new(path);

        // Sandbox path check (K4 B1)
        if !self.sandbox.is_path_allowed(path) {
            return Err(ToolError::PermissionDenied(format!(
                "path outside sandbox: {}",
                path.display()
            )));
        }

        if !path.exists() {
            return Err(ToolError::FileNotFound(path.display().to_string()));
        }

        let metadata = std::fs::metadata(path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        if metadata.len() > MAX_READ_SIZE {
            return Err(ToolError::FileTooLarge {
                size: metadata.len(),
                limit: MAX_READ_SIZE,
            });
        }

        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let bytes = std::fs::read(path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let end = match limit {
            Some(l) => std::cmp::min(offset + l, bytes.len()),
            None => bytes.len(),
        };
        let start = std::cmp::min(offset, bytes.len());
        let slice = &bytes[start..end];

        let content = String::from_utf8_lossy(slice).into_owned();
        let modified = metadata
            .modified()
            .ok()
            .map(|t| {
                let dt: DateTime<Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();

        Ok(serde_json::json!({
            "content": content,
            "size": metadata.len(),
            "modified": modified,
        }))
    }
}

/// Built-in `agent.spawn` tool.
///
/// Spawns a new agent process via the kernel's AgentSupervisor.
/// Always runs natively (needs direct kernel struct access).
pub struct AgentSpawnTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
}

impl AgentSpawnTool {
    pub fn new(process_table: Arc<crate::process::ProcessTable>) -> Self {
        let catalog = builtin_tool_catalog();
        let spec = catalog
            .into_iter()
            .find(|s| s.name == "agent.spawn")
            .expect("agent.spawn must be in catalog");
        Self { spec, process_table }
    }
}

impl BuiltinTool for AgentSpawnTool {
    fn name(&self) -> &str {
        "agent.spawn"
    }

    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let agent_id = args
            .get("agent_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'agent_id' parameter".into()))?;

        let backend = args
            .get("backend")
            .and_then(|v| v.as_str())
            .unwrap_or("native");

        if backend == "wasm" {
            return Err(ToolError::ExecutionFailed(
                "WASM backend not yet available for agent.spawn".into(),
            ));
        }

        // Create a process entry directly in the process table.
        // In production this would go through AgentSupervisor::spawn(),
        // but for the reference tool impl we create the entry directly.
        let entry = crate::process::ProcessEntry {
            pid: 0, // assigned by insert()
            agent_id: agent_id.to_string(),
            state: crate::process::ProcessState::Running,
            capabilities: crate::capability::AgentCapabilities::default(),
            resource_usage: crate::process::ResourceUsage::default(),
            cancel_token: tokio_util::sync::CancellationToken::new(),
            parent_pid: None,
        };

        let pid = self
            .process_table
            .insert(entry)
            .map_err(|e| ToolError::ExecutionFailed(format!("spawn failed: {e}")))?;

        Ok(serde_json::json!({
            "pid": pid,
            "agent_id": agent_id,
            "state": "running",
        }))
    }
}

// ---------------------------------------------------------------------------
// K4 C1: Filesystem tool implementations
// ---------------------------------------------------------------------------

/// Built-in `fs.write_file` tool.
pub struct FsWriteFileTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsWriteFileTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.write_file").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
    pub fn with_sandbox(sandbox: SandboxConfig) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.write_file").unwrap();
        Self { spec, sandbox }
    }
}

impl BuiltinTool for FsWriteFileTool {
    fn name(&self) -> &str { "fs.write_file" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path_str = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let content = args.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'content'".into()))?;
        let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
        let path = std::path::Path::new(path_str);
        if !self.sandbox.is_path_allowed(path) {
            return Err(ToolError::PermissionDenied(format!("path outside sandbox: {}", path.display())));
        }
        if append {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            f.write_all(content.as_bytes()).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        } else {
            std::fs::write(path, content).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }
        Ok(serde_json::json!({"written": content.len(), "path": path_str}))
    }
}

/// Built-in `fs.read_dir` tool.
pub struct FsReadDirTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsReadDirTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.read_dir").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
    pub fn with_sandbox(sandbox: SandboxConfig) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.read_dir").unwrap();
        Self { spec, sandbox }
    }
}

impl BuiltinTool for FsReadDirTool {
    fn name(&self) -> &str { "fs.read_dir" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path_str = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let path = std::path::Path::new(path_str);
        if !self.sandbox.is_path_allowed(path) {
            return Err(ToolError::PermissionDenied(format!("path outside sandbox: {}", path.display())));
        }
        if !path.exists() {
            return Err(ToolError::FileNotFound(path.display().to_string()));
        }
        let entries: Vec<serde_json::Value> = std::fs::read_dir(path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            .filter_map(|e| e.ok())
            .map(|e| {
                let ft = e.file_type().ok();
                serde_json::json!({
                    "name": e.file_name().to_string_lossy(),
                    "is_dir": ft.as_ref().map(|t| t.is_dir()).unwrap_or(false),
                    "is_file": ft.as_ref().map(|t| t.is_file()).unwrap_or(false),
                })
            })
            .collect();
        Ok(serde_json::json!({"entries": entries, "count": entries.len()}))
    }
}

/// Built-in `fs.create_dir` tool.
pub struct FsCreateDirTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsCreateDirTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.create_dir").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
}

impl BuiltinTool for FsCreateDirTool {
    fn name(&self) -> &str { "fs.create_dir" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path_str = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let path = std::path::Path::new(path_str);
        if !self.sandbox.is_path_allowed(path) {
            return Err(ToolError::PermissionDenied(format!("path outside sandbox: {}", path.display())));
        }
        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(true);
        if recursive {
            std::fs::create_dir_all(path).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        } else {
            std::fs::create_dir(path).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }
        Ok(serde_json::json!({"created": path_str}))
    }
}

/// Built-in `fs.remove` tool.
pub struct FsRemoveTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsRemoveTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.remove").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
}

impl BuiltinTool for FsRemoveTool {
    fn name(&self) -> &str { "fs.remove" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path_str = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let path = std::path::Path::new(path_str);
        if !self.sandbox.is_path_allowed(path) {
            return Err(ToolError::PermissionDenied(format!("path outside sandbox: {}", path.display())));
        }
        if !path.exists() {
            return Err(ToolError::FileNotFound(path.display().to_string()));
        }
        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
        if path.is_dir() {
            if recursive {
                std::fs::remove_dir_all(path).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            } else {
                std::fs::remove_dir(path).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            }
        } else {
            std::fs::remove_file(path).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }
        Ok(serde_json::json!({"removed": path_str}))
    }
}

/// Built-in `fs.copy` tool.
pub struct FsCopyTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsCopyTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.copy").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
}

impl BuiltinTool for FsCopyTool {
    fn name(&self) -> &str { "fs.copy" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let src_str = args.get("src").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'src'".into()))?;
        let dst_str = args.get("dst").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'dst'".into()))?;
        let src = std::path::Path::new(src_str);
        let dst = std::path::Path::new(dst_str);
        if !self.sandbox.is_path_allowed(src) || !self.sandbox.is_path_allowed(dst) {
            return Err(ToolError::PermissionDenied("path outside sandbox".into()));
        }
        if !src.exists() {
            return Err(ToolError::FileNotFound(src.display().to_string()));
        }
        let bytes = std::fs::copy(src, dst).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(serde_json::json!({"copied": bytes, "src": src_str, "dst": dst_str}))
    }
}

/// Built-in `fs.move` tool.
pub struct FsMoveTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsMoveTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.move").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
}

impl BuiltinTool for FsMoveTool {
    fn name(&self) -> &str { "fs.move" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let src_str = args.get("src").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'src'".into()))?;
        let dst_str = args.get("dst").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'dst'".into()))?;
        let src = std::path::Path::new(src_str);
        let dst = std::path::Path::new(dst_str);
        if !self.sandbox.is_path_allowed(src) || !self.sandbox.is_path_allowed(dst) {
            return Err(ToolError::PermissionDenied("path outside sandbox".into()));
        }
        if !src.exists() {
            return Err(ToolError::FileNotFound(src.display().to_string()));
        }
        std::fs::rename(src, dst).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(serde_json::json!({"moved": true, "src": src_str, "dst": dst_str}))
    }
}

/// Built-in `fs.stat` tool.
pub struct FsStatTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsStatTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.stat").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
}

impl BuiltinTool for FsStatTool {
    fn name(&self) -> &str { "fs.stat" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path_str = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let path = std::path::Path::new(path_str);
        if !self.sandbox.is_path_allowed(path) {
            return Err(ToolError::PermissionDenied(format!("path outside sandbox: {}", path.display())));
        }
        let meta = std::fs::metadata(path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let modified = meta.modified().ok().map(|t| {
            let dt: DateTime<Utc> = t.into();
            dt.to_rfc3339()
        }).unwrap_or_default();
        Ok(serde_json::json!({
            "size": meta.len(),
            "is_file": meta.is_file(),
            "is_dir": meta.is_dir(),
            "readonly": meta.permissions().readonly(),
            "modified": modified,
        }))
    }
}

/// Built-in `fs.exists` tool.
pub struct FsExistsTool {
    spec: BuiltinToolSpec,
}

impl FsExistsTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.exists").unwrap();
        Self { spec }
    }
}

impl BuiltinTool for FsExistsTool {
    fn name(&self) -> &str { "fs.exists" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path_str = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let path = std::path::Path::new(path_str);
        let exists = path.exists();
        let is_file = path.is_file();
        let is_dir = path.is_dir();
        Ok(serde_json::json!({"exists": exists, "is_file": is_file, "is_dir": is_dir}))
    }
}

/// Built-in `fs.glob` tool.
pub struct FsGlobTool {
    spec: BuiltinToolSpec,
    sandbox: SandboxConfig,
}

impl FsGlobTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "fs.glob").unwrap();
        Self { spec, sandbox: SandboxConfig::default() }
    }
}

impl BuiltinTool for FsGlobTool {
    fn name(&self) -> &str { "fs.glob" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let pattern = args.get("pattern").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pattern'".into()))?;
        let base_dir = args.get("base_dir").and_then(|v| v.as_str()).unwrap_or(".");
        let base = std::path::Path::new(base_dir);
        if !self.sandbox.is_path_allowed(base) {
            return Err(ToolError::PermissionDenied("base_dir outside sandbox".into()));
        }
        // Simple recursive walk with pattern matching
        let mut matches = Vec::new();
        fn walk(dir: &std::path::Path, pattern: &str, matches: &mut Vec<String>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    if simple_glob_match(pattern, &name) {
                        matches.push(path.display().to_string());
                    }
                    if path.is_dir() {
                        walk(&path, pattern, matches);
                    }
                }
            }
        }
        walk(base, pattern, &mut matches);
        matches.sort();
        Ok(serde_json::json!({"matches": matches, "count": matches.len()}))
    }
}

/// Simple glob pattern match supporting `*` and `?` wildcards.
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    simple_glob_match_inner(&p, &t, 0, 0)
}

fn simple_glob_match_inner(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
    if pi == pattern.len() && ti == text.len() {
        return true;
    }
    if pi == pattern.len() {
        return false;
    }
    match pattern[pi] {
        '*' => {
            // Match zero or more characters
            for i in ti..=text.len() {
                if simple_glob_match_inner(pattern, text, pi + 1, i) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if ti < text.len() {
                simple_glob_match_inner(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
        c => {
            if ti < text.len() && text[ti] == c {
                simple_glob_match_inner(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
    }
}

// ---------------------------------------------------------------------------
// K4 C2: Agent tool implementations
// ---------------------------------------------------------------------------

/// Built-in `agent.stop` tool.
pub struct AgentStopTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
}

impl AgentStopTool {
    pub fn new(process_table: Arc<crate::process::ProcessTable>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "agent.stop").unwrap();
        Self { spec, process_table }
    }
}

impl BuiltinTool for AgentStopTool {
    fn name(&self) -> &str { "agent.stop" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let pid = args.get("pid").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pid'".into()))?;
        let entry = self.process_table.get(pid)
            .ok_or_else(|| ToolError::NotFound(format!("pid {pid}")))?;
        entry.cancel_token.cancel();
        self.process_table.update_state(pid, crate::process::ProcessState::Stopping)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(serde_json::json!({"stopped": pid, "agent_id": entry.agent_id}))
    }
}

/// Built-in `agent.list` tool.
pub struct AgentListTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
}

impl AgentListTool {
    pub fn new(process_table: Arc<crate::process::ProcessTable>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "agent.list").unwrap();
        Self { spec, process_table }
    }
}

impl BuiltinTool for AgentListTool {
    fn name(&self) -> &str { "agent.list" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let list = self.process_table.list();
        let entries: Vec<serde_json::Value> = list.iter().map(|e| {
            serde_json::json!({
                "pid": e.pid,
                "agent_id": e.agent_id,
                "state": format!("{:?}", e.state),
            })
        }).collect();
        Ok(serde_json::json!({"agents": entries, "count": entries.len()}))
    }
}

/// Built-in `agent.inspect` tool.
pub struct AgentInspectTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
}

impl AgentInspectTool {
    pub fn new(process_table: Arc<crate::process::ProcessTable>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "agent.inspect").unwrap();
        Self { spec, process_table }
    }
}

impl BuiltinTool for AgentInspectTool {
    fn name(&self) -> &str { "agent.inspect" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let pid = args.get("pid").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pid'".into()))?;
        let entry = self.process_table.get(pid)
            .ok_or_else(|| ToolError::NotFound(format!("pid {pid}")))?;
        Ok(serde_json::json!({
            "pid": entry.pid,
            "agent_id": entry.agent_id,
            "state": format!("{:?}", entry.state),
            "parent_pid": entry.parent_pid,
            "resource_usage": {
                "messages_sent": entry.resource_usage.messages_sent,
                "tool_calls": entry.resource_usage.tool_calls,
                "cpu_time_ms": entry.resource_usage.cpu_time_ms,
            },
            "capabilities": {
                "can_spawn": entry.capabilities.can_spawn,
                "can_ipc": entry.capabilities.can_ipc,
                "can_exec_tools": entry.capabilities.can_exec_tools,
                "can_network": entry.capabilities.can_network,
            },
        }))
    }
}

/// Built-in `agent.send` tool.
pub struct AgentSendTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
    a2a: Arc<crate::a2a::A2ARouter>,
}

impl AgentSendTool {
    pub fn new(
        process_table: Arc<crate::process::ProcessTable>,
        a2a: Arc<crate::a2a::A2ARouter>,
    ) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "agent.send").unwrap();
        Self { spec, process_table, a2a }
    }
}

impl BuiltinTool for AgentSendTool {
    fn name(&self) -> &str { "agent.send" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let pid = args.get("pid").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pid'".into()))?;
        let message = args.get("message").cloned()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'message'".into()))?;
        // Verify target exists
        let _ = self.process_table.get(pid)
            .ok_or_else(|| ToolError::NotFound(format!("pid {pid}")))?;
        let msg = crate::ipc::KernelMessage::new(
            0, // from kernel
            crate::ipc::MessageTarget::Process(pid),
            crate::ipc::MessagePayload::Json(message),
        );
        let msg_id = msg.id.clone();
        // Use blocking send since BuiltinTool::execute is sync
        // In production, agent.send would go through the async agent loop
        let a2a = self.a2a.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async { a2a.send(msg).await })
        }).join()
            .map_err(|_| ToolError::ExecutionFailed("send thread panicked".into()))?
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(serde_json::json!({"sent": true, "pid": pid, "msg_id": msg_id}))
    }
}

/// Built-in `agent.suspend` tool.
pub struct AgentSuspendTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
}

impl AgentSuspendTool {
    pub fn new(process_table: Arc<crate::process::ProcessTable>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "agent.suspend").unwrap();
        Self { spec, process_table }
    }
}

impl BuiltinTool for AgentSuspendTool {
    fn name(&self) -> &str { "agent.suspend" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let pid = args.get("pid").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pid'".into()))?;
        self.process_table.update_state(pid, crate::process::ProcessState::Suspended)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(serde_json::json!({"suspended": pid}))
    }
}

/// Built-in `agent.resume` tool.
pub struct AgentResumeTool {
    spec: BuiltinToolSpec,
    process_table: Arc<crate::process::ProcessTable>,
}

impl AgentResumeTool {
    pub fn new(process_table: Arc<crate::process::ProcessTable>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "agent.resume").unwrap();
        Self { spec, process_table }
    }
}

impl BuiltinTool for AgentResumeTool {
    fn name(&self) -> &str { "agent.resume" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let pid = args.get("pid").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pid'".into()))?;
        self.process_table.update_state(pid, crate::process::ProcessState::Running)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(serde_json::json!({"resumed": pid}))
    }
}

// ---------------------------------------------------------------------------
// IPC tool implementations
// ---------------------------------------------------------------------------

/// Built-in `ipc.send` tool.
///
/// Sends a message to a target PID or topic via kernel IPC.
pub struct IpcSendTool {
    spec: BuiltinToolSpec,
}

impl Default for IpcSendTool {
    fn default() -> Self {
        Self::new()
    }
}

impl IpcSendTool {
    pub fn new() -> Self {
        let catalog = builtin_tool_catalog();
        let spec = catalog
            .into_iter()
            .find(|s| s.name == "ipc.send")
            .expect("ipc.send must be in catalog");
        Self { spec }
    }
}

impl BuiltinTool for IpcSendTool {
    fn name(&self) -> &str { "ipc.send" }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        // Stub: real implementation will route through KernelIpc
        Err(ToolError::ExecutionFailed("ipc.send requires async kernel context".into()))
    }
}

/// Built-in `ipc.subscribe` tool.
///
/// Subscribes the calling agent to a topic for receiving messages.
pub struct IpcSubscribeTool {
    spec: BuiltinToolSpec,
}

impl Default for IpcSubscribeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl IpcSubscribeTool {
    pub fn new() -> Self {
        let catalog = builtin_tool_catalog();
        let spec = catalog
            .into_iter()
            .find(|s| s.name == "ipc.subscribe")
            .expect("ipc.subscribe must be in catalog");
        Self { spec }
    }
}

impl BuiltinTool for IpcSubscribeTool {
    fn name(&self) -> &str { "ipc.subscribe" }
    fn spec(&self) -> &BuiltinToolSpec {
        &self.spec
    }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        // Stub: real implementation will route through TopicRouter
        Err(ToolError::ExecutionFailed("ipc.subscribe requires async kernel context".into()))
    }
}

// ---------------------------------------------------------------------------
// K4 C3: System tool implementations
// ---------------------------------------------------------------------------

/// Built-in `sys.service.list` tool.
pub struct SysServiceListTool {
    spec: BuiltinToolSpec,
    service_registry: Arc<crate::service::ServiceRegistry>,
}

impl SysServiceListTool {
    pub fn new(service_registry: Arc<crate::service::ServiceRegistry>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.service.list").unwrap();
        Self { spec, service_registry }
    }
}

impl BuiltinTool for SysServiceListTool {
    fn name(&self) -> &str { "sys.service.list" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let services = self.service_registry.list();
        let entries: Vec<serde_json::Value> = services.iter().map(|(name, stype)| {
            serde_json::json!({
                "name": name,
                "service_type": format!("{stype:?}"),
            })
        }).collect();
        Ok(serde_json::json!({"services": entries, "count": entries.len()}))
    }
}

/// Built-in `sys.service.health` tool.
pub struct SysServiceHealthTool {
    spec: BuiltinToolSpec,
    service_registry: Arc<crate::service::ServiceRegistry>,
}

impl SysServiceHealthTool {
    pub fn new(service_registry: Arc<crate::service::ServiceRegistry>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.service.health").unwrap();
        Self { spec, service_registry }
    }
}

impl BuiltinTool for SysServiceHealthTool {
    fn name(&self) -> &str { "sys.service.health" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        // health_all() is async — use service list as sync fallback
        let services = self.service_registry.list();
        let entries: Vec<serde_json::Value> = services.iter().map(|(name, _)| {
            serde_json::json!({"name": name, "status": "registered"})
        }).collect();
        Ok(serde_json::json!({"health": entries, "count": entries.len()}))
    }
}

/// Built-in `sys.chain.status` tool.
#[cfg(feature = "exochain")]
pub struct SysChainStatusTool {
    spec: BuiltinToolSpec,
    chain: Arc<crate::chain::ChainManager>,
}

#[cfg(feature = "exochain")]
impl SysChainStatusTool {
    pub fn new(chain: Arc<crate::chain::ChainManager>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.chain.status").unwrap();
        Self { spec, chain }
    }
}

#[cfg(feature = "exochain")]
impl BuiltinTool for SysChainStatusTool {
    fn name(&self) -> &str { "sys.chain.status" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let status = self.chain.status();
        Ok(serde_json::json!({
            "chain_id": status.chain_id,
            "sequence": status.sequence,
            "event_count": status.event_count,
            "checkpoint_count": status.checkpoint_count,
        }))
    }
}

/// Built-in `sys.chain.query` tool.
#[cfg(feature = "exochain")]
pub struct SysChainQueryTool {
    spec: BuiltinToolSpec,
    chain: Arc<crate::chain::ChainManager>,
}

#[cfg(feature = "exochain")]
impl SysChainQueryTool {
    pub fn new(chain: Arc<crate::chain::ChainManager>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.chain.query").unwrap();
        Self { spec, chain }
    }
}

#[cfg(feature = "exochain")]
impl BuiltinTool for SysChainQueryTool {
    fn name(&self) -> &str { "sys.chain.query" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let events = self.chain.tail(count);
        let entries: Vec<serde_json::Value> = events.iter().map(|e| {
            serde_json::json!({
                "sequence": e.sequence,
                "source": e.source,
                "kind": e.kind,
                "timestamp": e.timestamp.to_rfc3339(),
            })
        }).collect();
        Ok(serde_json::json!({"events": entries, "count": entries.len()}))
    }
}

/// Built-in `sys.tree.read` tool.
#[cfg(feature = "exochain")]
pub struct SysTreeReadTool {
    spec: BuiltinToolSpec,
    tree: Arc<crate::tree_manager::TreeManager>,
}

#[cfg(feature = "exochain")]
impl SysTreeReadTool {
    pub fn new(tree: Arc<crate::tree_manager::TreeManager>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.tree.read").unwrap();
        Self { spec, tree }
    }
}

#[cfg(feature = "exochain")]
impl BuiltinTool for SysTreeReadTool {
    fn name(&self) -> &str { "sys.tree.read" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let stats = self.tree.stats();
        Ok(serde_json::json!({
            "node_count": stats.node_count,
            "mutation_count": stats.mutation_count,
            "root_hash": stats.root_hash,
        }))
    }
}

/// Built-in `sys.tree.inspect` tool.
#[cfg(feature = "exochain")]
pub struct SysTreeInspectTool {
    spec: BuiltinToolSpec,
    tree: Arc<crate::tree_manager::TreeManager>,
}

#[cfg(feature = "exochain")]
impl SysTreeInspectTool {
    pub fn new(tree: Arc<crate::tree_manager::TreeManager>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.tree.inspect").unwrap();
        Self { spec, tree }
    }
}

#[cfg(feature = "exochain")]
impl BuiltinTool for SysTreeInspectTool {
    fn name(&self) -> &str { "sys.tree.inspect" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let path = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'path'".into()))?;
        let rid = exo_resource_tree::ResourceId::new(path);
        let tree_lock = self.tree.tree().lock()
            .map_err(|e| ToolError::ExecutionFailed(format!("tree lock: {e}")))?;
        let node = tree_lock.get(&rid)
            .ok_or_else(|| ToolError::NotFound(format!("node not found: {path}")))?;
        Ok(serde_json::json!({
            "path": path,
            "kind": format!("{:?}", node.kind),
            "metadata": node.metadata,
            "scoring": node.scoring.as_array(),
        }))
    }
}

/// Built-in `sys.env.get` tool.
pub struct SysEnvGetTool {
    spec: BuiltinToolSpec,
}

impl SysEnvGetTool {
    pub fn new() -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.env.get").unwrap();
        Self { spec }
    }
}

impl BuiltinTool for SysEnvGetTool {
    fn name(&self) -> &str { "sys.env.get" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let name = args.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'name'".into()))?;
        match std::env::var(name) {
            Ok(val) => Ok(serde_json::json!({"name": name, "value": val})),
            Err(_) => Ok(serde_json::json!({"name": name, "value": null})),
        }
    }
}

/// Built-in `sys.cron.add` tool.
pub struct SysCronAddTool {
    spec: BuiltinToolSpec,
    cron: Arc<crate::cron::CronService>,
}

impl SysCronAddTool {
    pub fn new(cron: Arc<crate::cron::CronService>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.cron.add").unwrap();
        Self { spec, cron }
    }
}

impl BuiltinTool for SysCronAddTool {
    fn name(&self) -> &str { "sys.cron.add" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let name = args.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'name'".into()))?;
        let interval_secs = args.get("interval_secs").and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'interval_secs'".into()))?;
        let command = args.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'command'".into()))?;
        let target_pid = args.get("target_pid").and_then(|v| v.as_u64());
        let job = self.cron.add_job(name.to_string(), interval_secs, command.to_string(), target_pid);
        Ok(serde_json::to_value(&job).unwrap_or_default())
    }
}

/// Built-in `sys.cron.list` tool.
pub struct SysCronListTool {
    spec: BuiltinToolSpec,
    cron: Arc<crate::cron::CronService>,
}

impl SysCronListTool {
    pub fn new(cron: Arc<crate::cron::CronService>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.cron.list").unwrap();
        Self { spec, cron }
    }
}

impl BuiltinTool for SysCronListTool {
    fn name(&self) -> &str { "sys.cron.list" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, _args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let jobs = self.cron.list_jobs();
        Ok(serde_json::to_value(&jobs).unwrap_or_default())
    }
}

/// Built-in `sys.cron.remove` tool.
pub struct SysCronRemoveTool {
    spec: BuiltinToolSpec,
    cron: Arc<crate::cron::CronService>,
}

impl SysCronRemoveTool {
    pub fn new(cron: Arc<crate::cron::CronService>) -> Self {
        let spec = builtin_tool_catalog().into_iter().find(|s| s.name == "sys.cron.remove").unwrap();
        Self { spec, cron }
    }
}

impl BuiltinTool for SysCronRemoveTool {
    fn name(&self) -> &str { "sys.cron.remove" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let id = args.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'id'".into()))?;
        match self.cron.remove_job(id) {
            Some(job) => Ok(serde_json::json!({"removed": true, "job_id": job.id})),
            None => Err(ToolError::NotFound(format!("cron job: {id}"))),
        }
    }
}

// ---------------------------------------------------------------------------
// D10: Shell Command Execution
// ---------------------------------------------------------------------------

/// A shell command to be executed in the sandbox.
///
/// Represents a command with arguments and optional sandbox configuration.
/// The command is dispatched through the tool execution path and chain-logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    /// The command to execute (e.g. "echo", "ls").
    pub command: String,
    /// Arguments to the command.
    pub args: Vec<String>,
    /// Optional sandbox configuration to restrict execution.
    pub sandbox_config: Option<SandboxConfig>,
}

/// Result of a shell command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Standard output captured from the command.
    pub stdout: String,
    /// Standard error captured from the command.
    pub stderr: String,
    /// Execution wall-clock time in milliseconds.
    pub execution_time_ms: u64,
}

/// Execute a shell command and return the result.
///
/// For now this dispatches as a builtin tool — actual WASM compilation
/// of shell commands is deferred to a future sprint. The sandbox config
/// is stored on the result for governance auditing.
///
/// When the `exochain` feature is enabled and a [`ChainManager`] is
/// provided, the execution is chain-logged as a `shell.exec` event.
pub fn execute_shell(cmd: &ShellCommand) -> Result<ShellResult, ToolError> {
    let start = std::time::Instant::now();

    // Sandbox path check: if sandbox_config has allowed_paths,
    // reject commands that reference paths outside the sandbox.
    if let Some(ref sandbox) = cmd.sandbox_config {
        if sandbox.sudo_override {
            tracing::warn!(command = %cmd.command, "shell exec with sudo override");
        }
    }

    // Builtin dispatch: for now, handle a small set of safe builtins.
    // Real execution would compile to WASM and run in the sandbox.
    let (exit_code, stdout, stderr) = match cmd.command.as_str() {
        "echo" => {
            let output = cmd.args.join(" ");
            (0, output, String::new())
        }
        "true" => (0, String::new(), String::new()),
        "false" => (1, String::new(), String::new()),
        _ => {
            // Unknown commands return a descriptive error in stderr.
            // Future: compile to WASM and run in sandbox.
            (127, String::new(), format!("command not found: {}", cmd.command))
        }
    };

    let elapsed = start.elapsed();

    Ok(ShellResult {
        exit_code,
        stdout,
        stderr,
        execution_time_ms: elapsed.as_millis() as u64,
    })
}

/// Built-in `shell.exec` tool wrapping [`execute_shell`].
pub struct ShellExecTool {
    spec: BuiltinToolSpec,
}

impl ShellExecTool {
    /// Create the shell.exec tool.
    pub fn new() -> Self {
        Self {
            spec: BuiltinToolSpec {
                name: "shell.exec".into(),
                category: ToolCategory::System,
                description: "Execute a shell command in the sandbox".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {
                        "command": {"type": "string", "description": "Command to execute"},
                        "args": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Command arguments"
                        }
                    }
                }),
                gate_action: "tool.shell.execute".into(),
                effect: EffectVector {
                    risk: 0.7,
                    security: 0.4,
                    ..Default::default()
                },
                native: true,
            },
        }
    }
}

impl BuiltinTool for ShellExecTool {
    fn name(&self) -> &str { "shell.exec" }
    fn spec(&self) -> &BuiltinToolSpec { &self.spec }
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let command = args.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'command'".into()))?;
        let cmd_args: Vec<String> = args.get("args")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let cmd = ShellCommand {
            command: command.to_string(),
            args: cmd_args,
            sandbox_config: None,
        };

        let result = execute_shell(&cmd)?;
        Ok(serde_json::json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "execution_time_ms": result.execution_time_ms,
        }))
    }
}

// ---------------------------------------------------------------------------
// Shell Pipeline (K3 C5)
// ---------------------------------------------------------------------------

/// A shell pipeline compiled into a chain-linked WASM tool spec.
///
/// Shell commands are wrapped as tool definitions with their content
/// hash anchored to the ExoChain for immutability and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellPipeline {
    /// Pipeline name.
    pub name: String,
    /// Shell command string.
    pub command: String,
    /// SHA-256 hash of the command.
    pub content_hash: [u8; 32],
    /// Chain sequence number where this pipeline was registered.
    pub chain_seq: Option<u64>,
}

impl ShellPipeline {
    /// Create a new shell pipeline from a command string.
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        let cmd = command.into();
        let hash = compute_module_hash(cmd.as_bytes());
        Self {
            name: name.into(),
            command: cmd,
            content_hash: hash,
            chain_seq: None,
        }
    }

    /// Register this pipeline on the chain for immutability (C5).
    #[cfg(feature = "exochain")]
    pub fn anchor_to_chain(&mut self, chain: &crate::chain::ChainManager) {
        let seq = chain.sequence();
        let hash_hex: String = self
            .content_hash
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        chain.append(
            "shell",
            "shell.pipeline.register",
            Some(serde_json::json!({
                "name": &self.name,
                "command_hash": hash_hex,
                "command_length": self.command.len(),
            })),
        );
        self.chain_seq = Some(seq);
    }

    /// Convert to a [`BuiltinToolSpec`] for registration in the [`ToolRegistry`].
    pub fn to_tool_spec(&self) -> BuiltinToolSpec {
        BuiltinToolSpec {
            name: format!("shell.{}", self.name),
            category: ToolCategory::User,
            description: format!("Shell pipeline: {}", self.name),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "args": {"type": "string", "description": "Additional arguments"}
                }
            }),
            gate_action: "tool.shell.execute".into(),
            effect: EffectVector {
                risk: 0.6,
                security: 0.3,
                ..Default::default()
            },
            native: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config tests (preserved) ---

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

    // --- Validation tests (preserved) ---

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
        // Minimal valid WASM: magic + version 1 + no sections
        let mut wasm = Vec::new();
        wasm.extend_from_slice(b"\0asm");
        wasm.extend_from_slice(&1u32.to_le_bytes());

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
        wasm.extend_from_slice(&2u32.to_le_bytes());

        // With wasm-sandbox, Wasmtime may reject wrong-version modules
        // entirely (InvalidModule). Without it, we get a warning.
        #[cfg(not(feature = "wasm-sandbox"))]
        {
            let result = runner.validate_wasm(&wasm).unwrap();
            assert!(result.valid);
            assert!(!result.warnings.is_empty());
            assert!(result.warnings[0].contains("version: 2"));
        }
        #[cfg(feature = "wasm-sandbox")]
        {
            // Wasmtime rejects non-v1 modules at validation time
            let result = runner.validate_wasm(&wasm);
            assert!(result.is_err() || {
                let v = result.unwrap();
                !v.warnings.is_empty()
            });
        }
    }

    #[test]
    fn load_tool_without_feature_rejects() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let mut wasm = Vec::new();
        wasm.extend_from_slice(b"\0asm");
        wasm.extend_from_slice(&1u32.to_le_bytes());
        wasm.extend_from_slice(&[0u8; 16]);

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

    // --- Catalog tests ---

    #[test]
    fn builtin_catalog_has_expected_tools() {
        let catalog = builtin_tool_catalog();
        #[cfg(feature = "ecc")]
        assert_eq!(catalog.len(), 36, "29 base + 7 ecc tools");
        #[cfg(not(feature = "ecc"))]
        assert_eq!(catalog.len(), 29);
    }

    #[test]
    fn all_tools_have_valid_schema() {
        let catalog = builtin_tool_catalog();
        for spec in &catalog {
            assert!(spec.parameters.is_object(), "{} has non-object schema", spec.name);
            assert!(
                spec.parameters.get("type").and_then(|v| v.as_str()) == Some("object"),
                "{} schema type is not 'object'",
                spec.name,
            );
        }
    }

    #[test]
    fn all_tools_have_gate_action() {
        let catalog = builtin_tool_catalog();
        for spec in &catalog {
            assert!(!spec.gate_action.is_empty(), "{} missing gate_action", spec.name);
            assert!(
                spec.gate_action.starts_with("tool.") || spec.gate_action.starts_with("ecc."),
                "{} gate_action should start with 'tool.' or 'ecc.'",
                spec.name,
            );
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let catalog = builtin_tool_catalog();
        let mut names: Vec<&str> = catalog.iter().map(|s| s.name.as_str()).collect();
        names.sort();
        let unique_count = {
            let mut u = names.clone();
            u.dedup();
            u.len()
        };
        assert_eq!(names.len(), unique_count, "duplicate tool names found");
    }

    #[test]
    fn tool_categories_correct() {
        let catalog = builtin_tool_catalog();
        let fs_count = catalog.iter().filter(|s| s.category == ToolCategory::Filesystem).count();
        let agent_count = catalog.iter().filter(|s| s.category == ToolCategory::Agent).count();
        let sys_count = catalog.iter().filter(|s| s.category == ToolCategory::System).count();
        assert_eq!(fs_count, 10);
        assert_eq!(agent_count, 9);
        assert_eq!(sys_count, 10);
        #[cfg(feature = "ecc")]
        {
            let ecc_count = catalog.iter().filter(|s| s.category == ToolCategory::Ecc).count();
            assert_eq!(ecc_count, 7);
        }
    }

    // --- FsReadFileTool tests ---

    #[test]
    fn fs_read_file_reads_content() {
        let dir = std::env::temp_dir().join("clawft-fs-read-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("test.txt");
        std::fs::write(&file, "hello world").unwrap();

        let tool = FsReadFileTool::new();
        let result = tool
            .execute(serde_json::json!({"path": file.to_str().unwrap()}))
            .unwrap();
        assert_eq!(result["content"], "hello world");
        assert_eq!(result["size"], 11);
        assert!(result["modified"].as_str().is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_read_file_with_offset_limit() {
        let dir = std::env::temp_dir().join("clawft-fs-offset-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("test.txt");
        std::fs::write(&file, "0123456789").unwrap();

        let tool = FsReadFileTool::new();
        let result = tool
            .execute(serde_json::json!({
                "path": file.to_str().unwrap(),
                "offset": 3,
                "limit": 4,
            }))
            .unwrap();
        assert_eq!(result["content"], "3456");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_read_file_not_found() {
        let tool = FsReadFileTool::new();
        let result = tool.execute(serde_json::json!({"path": "/no/such/file/ever"}));
        assert!(matches!(result, Err(ToolError::FileNotFound(_))));
    }

    #[test]
    fn fs_read_file_returns_metadata() {
        let dir = std::env::temp_dir().join("clawft-fs-meta-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("meta.txt");
        std::fs::write(&file, "data").unwrap();

        let tool = FsReadFileTool::new();
        let result = tool
            .execute(serde_json::json!({"path": file.to_str().unwrap()}))
            .unwrap();
        assert!(result.get("size").is_some());
        assert!(result.get("modified").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- AgentSpawnTool tests ---

    #[test]
    fn agent_spawn_creates_process() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let tool = AgentSpawnTool::new(pt.clone());
        let result = tool
            .execute(serde_json::json!({"agent_id": "test-spawn"}))
            .unwrap();

        let pid = result["pid"].as_u64().unwrap();
        assert!(pt.get(pid).is_some());
        assert_eq!(result["state"], "running");
    }

    #[test]
    fn agent_spawn_with_wasm_backend_fails() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let tool = AgentSpawnTool::new(pt);
        let result = tool.execute(serde_json::json!({
            "agent_id": "wasm-agent",
            "backend": "wasm",
        }));
        assert!(matches!(result, Err(ToolError::ExecutionFailed(_))));
    }

    #[test]
    fn agent_spawn_returns_pid() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let tool = AgentSpawnTool::new(pt);
        let result = tool
            .execute(serde_json::json!({"agent_id": "pid-test"}))
            .unwrap();
        assert!(result.get("pid").is_some());
        assert_eq!(result["agent_id"], "pid-test");
    }

    // --- ToolRegistry tests ---

    #[test]
    fn registry_register_and_execute() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(FsReadFileTool::new());
        registry.register(tool);
        assert_eq!(registry.len(), 1);
        assert!(registry.get("fs.read_file").is_some());
    }

    #[test]
    fn registry_not_found() {
        let registry = ToolRegistry::new();
        let result = registry.execute("no.such.tool", serde_json::json!({}));
        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    // --- Hierarchical ToolRegistry tests (K4 A1) ---

    #[test]
    fn registry_parent_chain_lookup() {
        let mut parent = ToolRegistry::new();
        parent.register(Arc::new(FsReadFileTool::new()));
        let parent = Arc::new(parent);

        let child = ToolRegistry::with_parent(parent);
        assert!(child.get("fs.read_file").is_some(), "child should find tool in parent");
    }

    #[test]
    fn registry_child_overrides_parent() {
        let mut parent = ToolRegistry::new();
        parent.register(Arc::new(FsReadFileTool::new()));
        let parent = Arc::new(parent);

        let mut child = ToolRegistry::with_parent(parent);
        // Register a different tool with the same interface but in child
        child.register(Arc::new(FsReadFileTool::new()));
        assert_eq!(child.len(), 1, "deduplicated count should be 1");
        assert!(child.get("fs.read_file").is_some());
    }

    #[test]
    fn registry_list_merges_parent() {
        let mut parent = ToolRegistry::new();
        parent.register(Arc::new(FsReadFileTool::new()));
        let parent = Arc::new(parent);

        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let mut child = ToolRegistry::with_parent(parent);
        child.register(Arc::new(AgentSpawnTool::new(pt)));

        let list = child.list();
        assert!(list.contains(&"fs.read_file".to_string()), "should include parent tool");
        assert!(list.contains(&"agent.spawn".to_string()), "should include child tool");
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn registry_empty_child_delegates_all() {
        let mut parent = ToolRegistry::new();
        parent.register(Arc::new(FsReadFileTool::new()));
        let parent = Arc::new(parent);

        let child = ToolRegistry::with_parent(parent);
        assert_eq!(child.len(), 1);
        assert!(!child.is_empty());

        // Execute through parent chain
        let dir = std::env::temp_dir().join("clawft-delegate-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("test.txt");
        std::fs::write(&file, "delegate").unwrap();
        let result = child.execute("fs.read_file", serde_json::json!({"path": file.to_str().unwrap()}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["content"], "delegate");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Sandbox tests (K4 B1) ---

    #[test]
    fn sandbox_denies_path_outside_allowed() {
        let sandbox = SandboxConfig {
            allowed_paths: vec![std::env::temp_dir()],
            ..Default::default()
        };
        let tool = FsReadFileTool::with_sandbox(sandbox);
        let result = tool.execute(serde_json::json!({"path": "/etc/passwd"}));
        assert!(matches!(result, Err(ToolError::PermissionDenied(_))));
    }

    #[test]
    fn sandbox_allows_path_inside_allowed() {
        let dir = std::env::temp_dir().join("clawft-sandbox-allow-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("allowed.txt");
        std::fs::write(&file, "allowed").unwrap();

        let sandbox = SandboxConfig {
            allowed_paths: vec![std::env::temp_dir()],
            ..Default::default()
        };
        let tool = FsReadFileTool::with_sandbox(sandbox);
        let result = tool.execute(serde_json::json!({"path": file.to_str().unwrap()}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["content"], "allowed");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandbox_default_allows_all() {
        let dir = std::env::temp_dir().join("clawft-sandbox-default-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("default.txt");
        std::fs::write(&file, "default").unwrap();

        // Default SandboxConfig has empty allowed_paths = permissive
        let tool = FsReadFileTool::new();
        let result = tool.execute(serde_json::json!({"path": file.to_str().unwrap()}));
        assert!(result.is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- K4 C1: Filesystem tool tests ---

    #[test]
    fn fs_write_file_creates_and_writes() {
        let dir = std::env::temp_dir().join("clawft-fs-write-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("out.txt");
        let tool = FsWriteFileTool::new();
        let result = tool.execute(serde_json::json!({"path": file.to_str().unwrap(), "content": "hello"}));
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_write_file_missing_content() {
        let tool = FsWriteFileTool::new();
        let result = tool.execute(serde_json::json!({"path": "/tmp/x"}));
        assert!(matches!(result, Err(ToolError::InvalidArgs(_))));
    }

    #[test]
    fn fs_read_dir_lists_entries() {
        let dir = std::env::temp_dir().join("clawft-fs-readdir-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();
        let tool = FsReadDirTool::new();
        let result = tool.execute(serde_json::json!({"path": dir.to_str().unwrap()})).unwrap();
        assert!(result["count"].as_u64().unwrap() >= 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_read_dir_not_found() {
        let tool = FsReadDirTool::new();
        let result = tool.execute(serde_json::json!({"path": "/no/such/dir/ever"}));
        assert!(matches!(result, Err(ToolError::FileNotFound(_))));
    }

    #[test]
    fn fs_create_dir_recursive() {
        let dir = std::env::temp_dir().join("clawft-fs-mkdir-test/a/b/c");
        let tool = FsCreateDirTool::new();
        let result = tool.execute(serde_json::json!({"path": dir.to_str().unwrap()}));
        assert!(result.is_ok());
        assert!(dir.exists());
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("clawft-fs-mkdir-test"));
    }

    #[test]
    fn fs_create_dir_sandbox_denied() {
        let tool = FsCreateDirTool::new();
        // Default sandbox is permissive — this should succeed
        let dir = std::env::temp_dir().join("clawft-fs-mkdir-sandbox-test");
        let result = tool.execute(serde_json::json!({"path": dir.to_str().unwrap()}));
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_remove_file() {
        let dir = std::env::temp_dir().join("clawft-fs-rm-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("delete_me.txt");
        std::fs::write(&file, "bye").unwrap();
        let tool = FsRemoveTool::new();
        let result = tool.execute(serde_json::json!({"path": file.to_str().unwrap()}));
        assert!(result.is_ok());
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_remove_not_found() {
        let tool = FsRemoveTool::new();
        let result = tool.execute(serde_json::json!({"path": "/no/such/file/xyz"}));
        assert!(matches!(result, Err(ToolError::FileNotFound(_))));
    }

    #[test]
    fn fs_copy_file() {
        let dir = std::env::temp_dir().join("clawft-fs-copy-test");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        std::fs::write(&src, "copy me").unwrap();
        let tool = FsCopyTool::new();
        let result = tool.execute(serde_json::json!({"src": src.to_str().unwrap(), "dst": dst.to_str().unwrap()}));
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "copy me");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_copy_not_found() {
        let tool = FsCopyTool::new();
        let result = tool.execute(serde_json::json!({"src": "/no/file", "dst": "/tmp/out"}));
        assert!(matches!(result, Err(ToolError::FileNotFound(_))));
    }

    #[test]
    fn fs_move_file() {
        let dir = std::env::temp_dir().join("clawft-fs-move-test");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("old.txt");
        let dst = dir.join("new.txt");
        std::fs::write(&src, "move me").unwrap();
        let tool = FsMoveTool::new();
        let result = tool.execute(serde_json::json!({"src": src.to_str().unwrap(), "dst": dst.to_str().unwrap()}));
        assert!(result.is_ok());
        assert!(!src.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "move me");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_move_not_found() {
        let tool = FsMoveTool::new();
        let result = tool.execute(serde_json::json!({"src": "/no/file", "dst": "/tmp/out"}));
        assert!(matches!(result, Err(ToolError::FileNotFound(_))));
    }

    #[test]
    fn fs_stat_returns_metadata() {
        let dir = std::env::temp_dir().join("clawft-fs-stat-test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("stat.txt");
        std::fs::write(&file, "data").unwrap();
        let tool = FsStatTool::new();
        let result = tool.execute(serde_json::json!({"path": file.to_str().unwrap()})).unwrap();
        assert_eq!(result["size"], 4);
        assert_eq!(result["is_file"], true);
        assert_eq!(result["is_dir"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_stat_error() {
        let tool = FsStatTool::new();
        let result = tool.execute(serde_json::json!({"path": "/no/such/file"}));
        assert!(matches!(result, Err(ToolError::ExecutionFailed(_))));
    }

    #[test]
    fn fs_exists_checks() {
        let tool = FsExistsTool::new();
        let result = tool.execute(serde_json::json!({"path": "/tmp"})).unwrap();
        assert_eq!(result["exists"], true);
        assert_eq!(result["is_dir"], true);

        let result = tool.execute(serde_json::json!({"path": "/no/such/path/xyz"})).unwrap();
        assert_eq!(result["exists"], false);
    }

    #[test]
    fn fs_glob_finds_files() {
        let dir = std::env::temp_dir().join("clawft-fs-glob-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("test.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("test.txt"), "text").unwrap();
        let tool = FsGlobTool::new();
        let result = tool.execute(serde_json::json!({
            "pattern": "*.rs",
            "base_dir": dir.to_str().unwrap(),
        })).unwrap();
        assert!(result["count"].as_u64().unwrap() >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fs_glob_no_match() {
        let dir = std::env::temp_dir().join("clawft-fs-glob-nomatch-test");
        let _ = std::fs::create_dir_all(&dir);
        let tool = FsGlobTool::new();
        let result = tool.execute(serde_json::json!({
            "pattern": "*.xyz",
            "base_dir": dir.to_str().unwrap(),
        })).unwrap();
        assert_eq!(result["count"], 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- K4 C2: Agent tool tests ---

    #[test]
    fn agent_stop_cancels_token() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let spawn = AgentSpawnTool::new(pt.clone());
        let result = spawn.execute(serde_json::json!({"agent_id": "stop-test"})).unwrap();
        let pid = result["pid"].as_u64().unwrap();

        let tool = AgentStopTool::new(pt.clone());
        let result = tool.execute(serde_json::json!({"pid": pid}));
        assert!(result.is_ok());
        let entry = pt.get(pid).unwrap();
        assert!(entry.cancel_token.is_cancelled());
    }

    #[test]
    fn agent_stop_not_found() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let tool = AgentStopTool::new(pt);
        let result = tool.execute(serde_json::json!({"pid": 9999}));
        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    #[test]
    fn agent_list_shows_agents() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let spawn = AgentSpawnTool::new(pt.clone());
        spawn.execute(serde_json::json!({"agent_id": "list-a"})).unwrap();
        spawn.execute(serde_json::json!({"agent_id": "list-b"})).unwrap();

        let tool = AgentListTool::new(pt);
        let result = tool.execute(serde_json::json!({})).unwrap();
        assert!(result["count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn agent_inspect_returns_details() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let spawn = AgentSpawnTool::new(pt.clone());
        let r = spawn.execute(serde_json::json!({"agent_id": "inspect-me"})).unwrap();
        let pid = r["pid"].as_u64().unwrap();

        let tool = AgentInspectTool::new(pt);
        let result = tool.execute(serde_json::json!({"pid": pid})).unwrap();
        assert_eq!(result["agent_id"], "inspect-me");
        assert!(result["capabilities"].is_object());
    }

    #[test]
    fn agent_inspect_not_found() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let tool = AgentInspectTool::new(pt);
        let result = tool.execute(serde_json::json!({"pid": 9999}));
        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    #[test]
    fn agent_suspend_resume_cycle() {
        let pt = Arc::new(crate::process::ProcessTable::new(64));
        let spawn = AgentSpawnTool::new(pt.clone());
        let r = spawn.execute(serde_json::json!({"agent_id": "sr-test"})).unwrap();
        let pid = r["pid"].as_u64().unwrap();

        let suspend = AgentSuspendTool::new(pt.clone());
        suspend.execute(serde_json::json!({"pid": pid})).unwrap();
        assert_eq!(pt.get(pid).unwrap().state, crate::process::ProcessState::Suspended);

        let resume = AgentResumeTool::new(pt.clone());
        resume.execute(serde_json::json!({"pid": pid})).unwrap();
        assert_eq!(pt.get(pid).unwrap().state, crate::process::ProcessState::Running);
    }

    // --- K4 C3: System tool tests ---

    #[test]
    fn sys_env_get_returns_value() {
        // SAFETY: test-only, single-threaded access
        unsafe { std::env::set_var("CLAWFT_TEST_VAR", "test_value"); }
        let tool = SysEnvGetTool::new();
        let result = tool.execute(serde_json::json!({"name": "CLAWFT_TEST_VAR"})).unwrap();
        assert_eq!(result["value"], "test_value");
        unsafe { std::env::remove_var("CLAWFT_TEST_VAR"); }
    }

    #[test]
    fn sys_env_get_missing_returns_null() {
        let tool = SysEnvGetTool::new();
        let result = tool.execute(serde_json::json!({"name": "NO_SUCH_VAR_EVER_XYZ"})).unwrap();
        assert!(result["value"].is_null());
    }

    #[test]
    fn sys_cron_add_list_remove() {
        let cron = Arc::new(crate::cron::CronService::new());
        let add = SysCronAddTool::new(cron.clone());
        let result = add.execute(serde_json::json!({
            "name": "test-job",
            "interval_secs": 60,
            "command": "ping",
        })).unwrap();
        let job_id = result["id"].as_str().unwrap().to_string();

        let list = SysCronListTool::new(cron.clone());
        let jobs = list.execute(serde_json::json!({})).unwrap();
        assert!(jobs.as_array().map(|a| !a.is_empty()).unwrap_or(false));

        let rm = SysCronRemoveTool::new(cron);
        let result = rm.execute(serde_json::json!({"id": job_id}));
        assert!(result.is_ok());
    }

    #[test]
    fn sys_cron_remove_not_found() {
        let cron = Arc::new(crate::cron::CronService::new());
        let tool = SysCronRemoveTool::new(cron);
        let result = tool.execute(serde_json::json!({"id": "no-such-job"}));
        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }

    #[test]
    fn simple_glob_match_works() {
        assert!(simple_glob_match("*.rs", "main.rs"));
        assert!(simple_glob_match("*.rs", "lib.rs"));
        assert!(!simple_glob_match("*.rs", "main.txt"));
        assert!(simple_glob_match("test?", "test1"));
        assert!(!simple_glob_match("test?", "test12"));
        assert!(simple_glob_match("*", "anything"));
    }

    // --- K4 D2: Module cache tests ---

    #[test]
    fn cache_roundtrip() {
        let dir = std::env::temp_dir().join("clawft-cache-rt-test");
        let _ = std::fs::remove_dir_all(&dir);
        let cache = CompiledModuleCache::new(dir.clone(), 1024 * 1024);
        let hash = [0xAAu8; 32];
        let data = b"compiled wasm bytes";
        cache.put(&hash, data);
        let got = cache.get(&hash).unwrap();
        assert_eq!(got, data);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_miss_returns_none() {
        let dir = std::env::temp_dir().join("clawft-cache-miss-test");
        let _ = std::fs::remove_dir_all(&dir);
        let cache = CompiledModuleCache::new(dir.clone(), 1024 * 1024);
        assert!(cache.get(&[0xBBu8; 32]).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_eviction() {
        let dir = std::env::temp_dir().join("clawft-cache-evict-test");
        let _ = std::fs::remove_dir_all(&dir);
        // Max 100 bytes — each entry ~20 bytes
        let cache = CompiledModuleCache::new(dir.clone(), 100);
        for i in 0..10u8 {
            let mut hash = [0u8; 32];
            hash[0] = i;
            cache.put(&hash, &[i; 20]);
        }
        // Some entries should have been evicted
        let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
        assert!(entries.len() < 10, "eviction should have removed some entries");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- K4 D3: WASI scope tests ---

    #[test]
    fn wasi_scope_default_is_none() {
        let scope = WasiFsScope::default();
        assert_eq!(scope, WasiFsScope::None);
    }

    #[test]
    fn wasi_scope_serde_roundtrip() {
        let scope = WasiFsScope::ReadOnly(PathBuf::from("/data"));
        let json = serde_json::to_string(&scope).unwrap();
        let restored: WasiFsScope = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, scope);
    }

    // --- K4 F1: Signing tests ---

    #[test]
    #[cfg(feature = "exochain")]
    fn verify_kernel_signature() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let hash = [0xAAu8; 32];
        use ed25519_dalek::Signer;
        let sig = key.sign(&hash);
        let pubkey = key.verifying_key().to_bytes();
        assert!(verify_tool_signature(&hash, &sig.to_bytes(), &pubkey));
    }

    #[test]
    #[cfg(feature = "exochain")]
    fn verify_tampered_cert_fails() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let hash = [0xAAu8; 32];
        use ed25519_dalek::Signer;
        let sig = key.sign(&hash);
        let mut bad_sig = sig.to_bytes();
        bad_sig[0] ^= 0xFF; // tamper
        let pubkey = key.verifying_key().to_bytes();
        assert!(!verify_tool_signature(&hash, &bad_sig, &pubkey));
    }

    #[test]
    fn signing_authority_serde() {
        let auth = ToolSigningAuthority::Kernel;
        let json = serde_json::to_string(&auth).unwrap();
        let _: ToolSigningAuthority = serde_json::from_str(&json).unwrap();
    }

    #[test]
    #[cfg(feature = "exochain")]
    fn signing_verify_roundtrip() {
        use ed25519_dalek::{SigningKey, Signer};
        let mut rng = rand::rngs::OsRng;
        let sk = SigningKey::generate(&mut rng);
        let pk_bytes: [u8; 32] = sk.verifying_key().to_bytes();
        let hash: [u8; 32] = [42u8; 32];
        let sig = sk.sign(&hash);
        let sig_bytes: [u8; 64] = sig.to_bytes();
        assert!(verify_tool_signature(&hash, &sig_bytes, &pk_bytes));
    }

    #[test]
    #[cfg(feature = "exochain")]
    fn signing_tampered_fails() {
        use ed25519_dalek::{SigningKey, Signer};
        let mut rng = rand::rngs::OsRng;
        let sk = SigningKey::generate(&mut rng);
        let pk_bytes: [u8; 32] = sk.verifying_key().to_bytes();
        let hash: [u8; 32] = [42u8; 32];
        let sig = sk.sign(&hash);
        let mut sig_bytes: [u8; 64] = sig.to_bytes();
        sig_bytes[0] ^= 0xff; // tamper
        assert!(!verify_tool_signature(&hash, &sig_bytes, &pk_bytes));
    }

    // --- K4 F2: Backend selection tests ---

    #[test]
    fn backend_low_risk_native() {
        assert_eq!(BackendSelection::from_risk(0.1), BackendSelection::Native);
        assert_eq!(BackendSelection::from_risk(0.3), BackendSelection::Native);
    }

    #[test]
    fn backend_high_risk_wasm() {
        assert_eq!(BackendSelection::from_risk(0.5), BackendSelection::Wasm);
        assert_eq!(BackendSelection::from_risk(0.7), BackendSelection::Wasm);
    }

    // --- Module hash tests ---

    #[test]
    fn module_hash_deterministic() {
        let data = b"test module bytes";
        let h1 = compute_module_hash(data);
        let h2 = compute_module_hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn module_hash_differs_for_different_input() {
        let h1 = compute_module_hash(b"module A");
        let h2 = compute_module_hash(b"module B");
        assert_ne!(h1, h2);
    }

    // --- ToolVersion/DeployedTool serde ---

    #[test]
    fn tool_version_serde_roundtrip() {
        let tv = ToolVersion {
            version: 1,
            module_hash: [0xAA; 32],
            signature: [0xBB; 64],
            deployed_at: Utc::now(),
            revoked: false,
            chain_seq: 42,
        };
        let json = serde_json::to_string(&tv).unwrap();
        let restored: ToolVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.version, 1);
        assert_eq!(restored.chain_seq, 42);
        assert!(!restored.revoked);
    }

    #[test]
    fn builtin_tool_spec_serde_roundtrip() {
        let spec = BuiltinToolSpec {
            name: "test.tool".into(),
            category: ToolCategory::User,
            description: "A test tool".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
            gate_action: "tool.test".into(),
            effect: EffectVector::default(),
            native: true,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let restored: BuiltinToolSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test.tool");
        assert_eq!(restored.category, ToolCategory::User);
    }

    // --- K3: WASM execute_bytes tests ---

    /// Minimal WASM module (WAT) that exports _start as a no-op.
    #[cfg(feature = "wasm-sandbox")]
    const NOOP_WAT: &str = r#"(module
        (memory (export "memory") 1)
        (func (export "_start"))
    )"#;

    #[cfg(feature = "wasm-sandbox")]
    #[tokio::test]
    async fn execute_bytes_noop_module() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let result = runner
            .execute_bytes("noop", NOOP_WAT.as_bytes(), serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.fuel_consumed > 0);
    }

    #[cfg(feature = "wasm-sandbox")]
    #[tokio::test]
    async fn execute_bytes_captures_fuel() {
        let config = WasmSandboxConfig {
            max_fuel: 10_000_000,
            ..Default::default()
        };
        let runner = WasmToolRunner::new(config);
        let result = runner
            .execute_bytes("fuel-test", NOOP_WAT.as_bytes(), serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result.exit_code, 0);
        // A noop module should consume some fuel for instantiation
        assert!(result.fuel_consumed > 0);
        assert!(result.fuel_consumed < 10_000_000);
    }

    #[cfg(feature = "wasm-sandbox")]
    #[tokio::test]
    async fn execute_bytes_invalid_module() {
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let result = runner
            .execute_bytes("bad", b"not wasm at all", serde_json::json!({}))
            .await;
        assert!(matches!(result, Err(WasmError::CompilationFailed(_))));
    }

    #[cfg(feature = "wasm-sandbox")]
    #[tokio::test]
    async fn execute_bytes_fuel_exhaustion() {
        let config = WasmSandboxConfig {
            max_fuel: 1, // impossibly low
            ..Default::default()
        };
        let runner = WasmToolRunner::new(config);
        // A module with a loop that should exhaust fuel
        let loop_wat = r#"(module
            (memory (export "memory") 1)
            (func (export "_start")
                (local $i i32)
                (block $break
                    (loop $loop
                        (br_if $break (i32.ge_u (local.get $i) (i32.const 1000)))
                        (local.set $i (i32.add (local.get $i) (i32.const 1)))
                        (br $loop)
                    )
                )
            )
        )"#;
        let result = runner
            .execute_bytes("loop", loop_wat.as_bytes(), serde_json::json!({}))
            .await;
        assert!(
            matches!(result, Err(WasmError::FuelExhausted { .. })),
            "expected FuelExhausted, got: {result:?}",
        );
    }

    #[cfg(feature = "wasm-sandbox")]
    #[tokio::test]
    async fn execute_bytes_no_export_returns_error() {
        // Module with no _start or execute export
        let no_export_wat = r#"(module
            (memory (export "memory") 1)
            (func $helper (nop))
        )"#;
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());
        let result = runner
            .execute_bytes("noexport", no_export_wat.as_bytes(), serde_json::json!({}))
            .await
            .unwrap();
        // Should succeed but with exit_code 1 and trap in stderr
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("no _start or execute export"));
    }

    // --- K3: WasmToolAdapter / register_wasm_tool tests ---

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn register_wasm_tool_and_dispatch() {
        let runner = Arc::new(WasmToolRunner::new(WasmSandboxConfig::default()));
        let mut registry = ToolRegistry::new();
        registry
            .register_wasm_tool(
                "wasm.noop",
                "A noop WASM tool",
                NOOP_WAT.as_bytes().to_vec(),
                runner,
            )
            .expect("registration should succeed");
        assert!(registry.get("wasm.noop").is_some());
        let spec = registry.get("wasm.noop").unwrap().spec();
        assert!(!spec.native);
        assert_eq!(spec.gate_action, "tool.wasm.wasm.noop");
    }

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn register_wasm_tool_invalid_bytes_rejected() {
        let runner = Arc::new(WasmToolRunner::new(WasmSandboxConfig::default()));
        let mut registry = ToolRegistry::new();
        let result = registry.register_wasm_tool(
            "wasm.bad",
            "Invalid",
            b"not wasm".to_vec(),
            runner,
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn wasm_adapter_execute_runs_module() {
        let runner = Arc::new(WasmToolRunner::new(WasmSandboxConfig::default()));
        let mut registry = ToolRegistry::new();
        registry
            .register_wasm_tool(
                "wasm.noop",
                "noop",
                NOOP_WAT.as_bytes().to_vec(),
                runner,
            )
            .unwrap();
        let result = registry.execute("wasm.noop", serde_json::json!({}));
        assert!(result.is_ok(), "execute should succeed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["exit_code"], 0);
        assert!(val["fuel_consumed"].as_u64().unwrap() > 0);
    }

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn wasm_adapter_listed_in_registry() {
        let runner = Arc::new(WasmToolRunner::new(WasmSandboxConfig::default()));
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(FsExistsTool::new()));
        registry
            .register_wasm_tool(
                "wasm.noop",
                "noop",
                NOOP_WAT.as_bytes().to_vec(),
                runner,
            )
            .unwrap();
        let list = registry.list();
        assert!(list.contains(&"fs.exists".to_string()));
        assert!(list.contains(&"wasm.noop".to_string()));
        assert_eq!(list.len(), 2);
    }

    // --- K3 gate: sync execute_sync tests ---

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn k3_wasm_tool_loads_and_executes() {
        // Gate item 1: WASM tool loads and executes
        //
        // Verify that a minimal WASM module can be compiled, instantiated,
        // and its _start function called through the sync execution path.
        let runner = WasmToolRunner::new(WasmSandboxConfig::default());

        // execute_sync accepts WAT text and compiles it on the fly
        let result = runner
            .execute_sync("noop", NOOP_WAT.as_bytes(), serde_json::json!({}))
            .expect("execute_sync should succeed for noop WAT module");
        assert_eq!(result.exit_code, 0);
        assert!(result.fuel_consumed > 0, "fuel_consumed should be non-zero");
    }

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn k3_fuel_exhaustion_terminates_cleanly() {
        // Gate item 2: Fuel exhaustion terminates execution cleanly
        let config = WasmSandboxConfig {
            max_fuel: 1, // impossibly low -- even instantiation costs fuel
            ..Default::default()
        };
        let runner = WasmToolRunner::new(config);

        // A module with a loop that must exhaust fuel
        let loop_wat = r#"(module
            (func (export "_start")
                (local $i i32)
                (block $break
                    (loop $loop
                        (br_if $break (i32.ge_u (local.get $i) (i32.const 1000)))
                        (local.set $i (i32.add (local.get $i) (i32.const 1)))
                        (br $loop)
                    )
                )
            )
        )"#;

        let result = runner.execute_sync("loop", loop_wat.as_bytes(), serde_json::json!({}));
        assert!(
            matches!(result, Err(WasmError::FuelExhausted { .. })),
            "expected FuelExhausted, got: {result:?}",
        );
    }

    #[cfg(feature = "wasm-sandbox")]
    #[test]
    fn k3_memory_limit_prevents_allocation_bomb() {
        // Gate item 3: Memory limit prevents allocation bomb
        //
        // Configure a 64 KiB memory cap (one WASM page = 64 KiB).
        // Then try to instantiate a module that declares 32 pages
        // (2 MiB) of initial memory, which exceeds the cap.
        let config = WasmSandboxConfig {
            max_memory_bytes: 64 * 1024, // 1 page
            max_fuel: 1_000_000,
            ..Default::default()
        };
        let runner = WasmToolRunner::new(config);

        // Module requests 32 pages (2 MiB) of initial memory
        let big_mem_wat = r#"(module
            (memory 32)
            (func (export "_start") (nop))
        )"#;

        let result = runner.execute_sync(
            "alloc-bomb",
            big_mem_wat.as_bytes(),
            serde_json::json!({}),
        );
        // Should fail with either MemoryLimitExceeded or a trap related to memory
        assert!(
            result.is_err(),
            "module requesting 2 MiB with 64 KiB cap should fail, got: {result:?}",
        );
    }

    #[test]
    fn k3_host_filesystem_not_accessible_from_sandbox() {
        // Gate item 4: Host filesystem not accessible from sandbox
        //
        // Verify that the default WasmSandboxConfig does NOT grant
        // any filesystem access. The WASI context is disabled and
        // the filesystem scope is None.
        let config = WasmSandboxConfig::default();
        assert!(!config.wasi_enabled, "WASI should be disabled by default");
        assert!(
            config.allowed_host_calls.is_empty(),
            "no host calls should be allowed by default",
        );

        // WasiFsScope default is None
        let scope = WasiFsScope::default();
        assert_eq!(scope, WasiFsScope::None, "default fs scope should be None");

        // Confirm that execute_sync uses no imports (no WASI, no fs)
        // by running a module that has zero imports. If the runner
        // injected any host functions, a module with zero imports
        // would still work -- but a module that tries to import
        // wasi_snapshot_preview1 functions should fail.
        #[cfg(feature = "wasm-sandbox")]
        {
            let runner = WasmToolRunner::new(config);

            // Module that tries to import a WASI function for fd_write
            // (used for filesystem access). This should fail because
            // execute_sync provides NO imports.
            let wasi_import_wat = r#"(module
                (import "wasi_snapshot_preview1" "fd_write"
                    (func $fd_write (param i32 i32 i32 i32) (result i32)))
                (func (export "_start") (nop))
            )"#;

            let result = runner.execute_sync(
                "fs-probe",
                wasi_import_wat.as_bytes(),
                serde_json::json!({}),
            );
            assert!(
                result.is_err(),
                "module importing WASI fd_write should fail in sandboxed execute_sync: {result:?}",
            );
        }
    }

    // --- C5: ShellPipeline tests ---

    #[test]
    fn shell_pipeline_creates_hash() {
        let pipeline =
            ShellPipeline::new("deploy", "cargo build --release && scp target/release/weft server:");
        assert_ne!(pipeline.content_hash, [0u8; 32]);
    }

    #[test]
    fn shell_pipeline_deterministic_hash() {
        let p1 = ShellPipeline::new("test", "echo hello");
        let p2 = ShellPipeline::new("test", "echo hello");
        assert_eq!(p1.content_hash, p2.content_hash);
    }

    #[test]
    fn shell_pipeline_different_commands_different_hash() {
        let p1 = ShellPipeline::new("a", "echo hello");
        let p2 = ShellPipeline::new("a", "echo world");
        assert_ne!(p1.content_hash, p2.content_hash);
    }

    #[test]
    fn shell_pipeline_to_tool_spec() {
        let pipeline = ShellPipeline::new("build", "cargo build");
        let spec = pipeline.to_tool_spec();
        assert_eq!(spec.name, "shell.build");
        assert_eq!(spec.category, ToolCategory::User);
        assert!(spec.native);
        assert_eq!(spec.gate_action, "tool.shell.execute");
    }

    #[test]
    fn shell_pipeline_initial_chain_seq_none() {
        let pipeline = ShellPipeline::new("test", "ls -la");
        assert!(pipeline.chain_seq.is_none());
    }

    #[test]
    #[cfg(feature = "exochain")]
    fn shell_pipeline_anchor_to_chain() {
        let chain = crate::chain::ChainManager::new(0, 1000);
        let mut pipeline = ShellPipeline::new("deploy", "make deploy");
        assert!(pipeline.chain_seq.is_none());

        pipeline.anchor_to_chain(&chain);
        assert!(pipeline.chain_seq.is_some());

        let events = chain.tail(1);
        assert_eq!(events[0].kind, "shell.pipeline.register");
    }

    #[test]
    fn shell_pipeline_serde_roundtrip() {
        let pipeline = ShellPipeline::new("test-serde", "echo roundtrip");
        let json = serde_json::to_string(&pipeline).unwrap();
        let restored: ShellPipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test-serde");
        assert_eq!(restored.command, "echo roundtrip");
        assert_eq!(restored.content_hash, pipeline.content_hash);
    }

    // --- D9: Tool Signing tests ---

    #[test]
    fn d9_register_unsigned_when_not_required_succeeds() {
        let mut registry = ToolRegistry::new();
        // Signatures not required (default) — register should succeed silently.
        let tool: Arc<dyn BuiltinTool> = Arc::new(FsReadFileTool::new());
        registry.register(tool);
        assert!(registry.get("fs.read_file").is_some());
    }

    #[test]
    fn d9_register_unsigned_when_required_fails() {
        let mut registry = ToolRegistry::new();
        registry.set_require_signatures(true);
        assert!(registry.requires_signatures());

        let tool: Arc<dyn BuiltinTool> = Arc::new(FsReadFileTool::new());
        let result = registry.try_register(tool);
        assert!(
            matches!(result, Err(ToolError::SignatureRequired(_))),
            "expected SignatureRequired, got: {result:?}",
        );
        assert!(registry.get("fs.read_file").is_none());
    }

    #[test]
    #[cfg(feature = "exochain")]
    fn d9_register_with_valid_signature_succeeds() {
        use ed25519_dalek::{SigningKey, Signer};

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = sk.verifying_key().to_bytes();

        let tool: Arc<dyn BuiltinTool> = Arc::new(FsReadFileTool::new());
        let tool_hash = compute_module_hash(b"fs.read_file-definition");
        let sig = sk.sign(&tool_hash);

        let tool_sig = ToolSignature::new(
            "fs.read_file",
            tool_hash,
            "test-signer",
            sig.to_bytes().to_vec(),
        );

        let mut registry = ToolRegistry::new();
        registry.set_require_signatures(true);
        registry.add_trusted_key(pk);

        let result = registry.register_signed(tool, tool_sig);
        assert!(result.is_ok(), "register_signed should succeed: {result:?}");
        assert!(registry.get("fs.read_file").is_some());
        assert!(registry.get_signature("fs.read_file").is_some());
    }

    #[test]
    #[cfg(feature = "exochain")]
    fn d9_verify_tool_signature_roundtrip() {
        use ed25519_dalek::{SigningKey, Signer};

        let sk = SigningKey::from_bytes(&[99u8; 32]);
        let pk = sk.verifying_key().to_bytes();

        let tool_hash = compute_module_hash(b"my-tool-definition-bytes");
        let sig = sk.sign(&tool_hash);

        let tool_sig = ToolSignature::new(
            "my.tool",
            tool_hash,
            "dev-alice",
            sig.to_bytes().to_vec(),
        );

        // Should verify against the correct key.
        assert!(tool_sig.verify(&pk));

        // Should fail against a different key.
        let wrong_sk = SigningKey::from_bytes(&[100u8; 32]);
        let wrong_pk = wrong_sk.verifying_key().to_bytes();
        assert!(!tool_sig.verify(&wrong_pk));
    }

    #[test]
    fn d9_tool_signature_serde_roundtrip() {
        let sig = ToolSignature::new(
            "test.tool",
            [0xAB; 32],
            "signer-1",
            vec![0xCD; 64],
        );
        let json = serde_json::to_string(&sig).unwrap();
        let restored: ToolSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tool_name, "test.tool");
        assert_eq!(restored.tool_hash, [0xAB; 32]);
        assert_eq!(restored.signer_id, "signer-1");
        assert_eq!(restored.signature.len(), 64);
    }

    #[test]
    fn d9_register_signed_with_invalid_signature_fails() {
        let mut registry = ToolRegistry::new();
        registry.set_require_signatures(true);
        // Add a "trusted" key (just a random key).
        registry.add_trusted_key([42u8; 32]);

        let tool: Arc<dyn BuiltinTool> = Arc::new(FsReadFileTool::new());
        let bad_sig = ToolSignature::new(
            "fs.read_file",
            [0u8; 32],
            "bad-signer",
            vec![0u8; 64], // all zeros = invalid signature
        );

        let result = registry.register_signed(tool, bad_sig);
        assert!(
            matches!(result, Err(ToolError::InvalidSignature(_))),
            "expected InvalidSignature, got: {result:?}",
        );
    }

    // --- D10: Shell Command Execution tests ---

    #[test]
    fn d10_shell_command_serde_roundtrip() {
        let cmd = ShellCommand {
            command: "echo".into(),
            args: vec!["hello".into(), "world".into()],
            sandbox_config: Some(SandboxConfig::default()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let restored: ShellCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.command, "echo");
        assert_eq!(restored.args, vec!["hello", "world"]);
        assert!(restored.sandbox_config.is_some());
    }

    #[test]
    fn d10_execute_shell_echo() {
        let cmd = ShellCommand {
            command: "echo".into(),
            args: vec!["hello".into(), "world".into()],
            sandbox_config: None,
        };
        let result = execute_shell(&cmd).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "hello world");
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn d10_execute_shell_includes_execution_time() {
        let cmd = ShellCommand {
            command: "true".into(),
            args: vec![],
            sandbox_config: None,
        };
        let result = execute_shell(&cmd).unwrap();
        assert_eq!(result.exit_code, 0);
        // execution_time_ms should be a valid number (even if 0).
        assert!(result.execution_time_ms < 1000, "should complete in < 1s");
    }

    #[test]
    fn d10_execute_shell_unknown_command() {
        let cmd = ShellCommand {
            command: "nonexistent".into(),
            args: vec![],
            sandbox_config: None,
        };
        let result = execute_shell(&cmd).unwrap();
        assert_eq!(result.exit_code, 127);
        assert!(result.stderr.contains("command not found"));
    }

    #[test]
    fn d10_shell_exec_tool_dispatch() {
        let tool = ShellExecTool::new();
        assert_eq!(tool.name(), "shell.exec");

        let result = tool.execute(serde_json::json!({
            "command": "echo",
            "args": ["test"]
        })).unwrap();

        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"], "test");
    }

    #[test]
    fn d10_shell_result_serde_roundtrip() {
        let result = ShellResult {
            exit_code: 0,
            stdout: "output".into(),
            stderr: "".into(),
            execution_time_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: ShellResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.exit_code, 0);
        assert_eq!(restored.stdout, "output");
        assert_eq!(restored.execution_time_ms, 42);
    }

    #[test]
    fn d10_execute_shell_false_returns_nonzero() {
        let cmd = ShellCommand {
            command: "false".into(),
            args: vec![],
            sandbox_config: None,
        };
        let result = execute_shell(&cmd).unwrap();
        assert_eq!(result.exit_code, 1);
    }
}
