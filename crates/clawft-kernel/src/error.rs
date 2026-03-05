//! Kernel error types.
//!
//! All kernel operations return [`KernelError`] for typed error
//! handling. The error variants cover process table operations,
//! service lifecycle, IPC, and boot sequence failures.

use crate::process::ProcessState;

/// Kernel-level errors.
#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    /// Process not found in the process table.
    #[error("process not found: PID {pid}")]
    ProcessNotFound {
        /// The PID that was looked up.
        pid: u64,
    },

    /// Invalid process state transition.
    #[error("invalid state transition for PID {pid}: {from} -> {to}")]
    InvalidStateTransition {
        /// The affected PID.
        pid: u64,
        /// Current state.
        from: ProcessState,
        /// Requested state.
        to: ProcessState,
    },

    /// Process table has reached maximum capacity.
    #[error("process table full (max: {max})")]
    ProcessTableFull {
        /// Maximum number of processes allowed.
        max: u32,
    },

    /// Service-related error.
    #[error("service error: {0}")]
    Service(String),

    /// Boot sequence error.
    #[error("boot error: {0}")]
    Boot(String),

    /// IPC / messaging error.
    #[error("ipc error: {0}")]
    Ipc(String),

    /// Kernel is in wrong state for requested operation.
    #[error("kernel state error: expected {expected}, got {actual}")]
    WrongState {
        /// Expected state.
        expected: String,
        /// Actual state.
        actual: String,
    },

    /// Capability check denied an action.
    #[error("capability denied for PID {pid}: cannot {action} -- {reason}")]
    CapabilityDenied {
        /// The PID of the process that was denied.
        pid: u64,
        /// The action that was attempted.
        action: String,
        /// Why the action was denied.
        reason: String,
    },

    /// Resource limit exceeded.
    #[error("resource limit exceeded for PID {pid}: {resource} ({current} > {limit})")]
    ResourceLimitExceeded {
        /// The PID of the process.
        pid: u64,
        /// Name of the resource (memory, cpu_time, etc.).
        resource: String,
        /// Current usage value.
        current: u64,
        /// Configured limit.
        limit: u64,
    },

    /// Agent spawn failed.
    #[error("spawn failed for agent '{agent_id}': {reason}")]
    SpawnFailed {
        /// The agent that was being spawned.
        agent_id: String,
        /// Why the spawn failed.
        reason: String,
    },

    /// Spawn backend not available (defined but not yet implemented).
    #[error("backend not available: {backend} ({reason})")]
    BackendNotAvailable {
        /// The backend that was requested.
        backend: String,
        /// Why the backend is not available.
        reason: String,
    },

    /// Configuration error.
    #[error("config error: {0}")]
    Config(String),

    /// Wraps a generic error from downstream crates.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience alias for kernel results.
pub type KernelResult<T> = Result<T, KernelError>;
