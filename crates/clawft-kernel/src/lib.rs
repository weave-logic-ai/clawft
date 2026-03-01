//! WeftOS kernel layer for clawft.
//!
//! This crate provides the kernel abstraction layer that sits between
//! the CLI/API surface and `clawft-core`. It introduces:
//!
//! - **Boot sequence** ([`boot::Kernel`]) -- lifecycle management
//!   wrapping `AppContext` with structured startup/shutdown.
//! - **Process table** ([`process::ProcessTable`]) -- PID-based
//!   agent tracking with state machine transitions.
//! - **Service registry** ([`service::ServiceRegistry`]) -- named
//!   service lifecycle with health checks.
//! - **IPC** ([`ipc::KernelIpc`]) -- typed message envelopes over
//!   the existing `MessageBus`.
//! - **Capabilities** ([`capability::AgentCapabilities`]) -- permission
//!   model for agent processes.
//! - **Health monitoring** ([`health::HealthSystem`]) -- aggregated
//!   health checks across all services.
//! - **Console** ([`console`]) -- boot event types and output
//!   formatting for the interactive kernel terminal.
//! - **Configuration** ([`config::KernelConfig`]) -- kernel-specific
//!   settings embedded in the root config.
//! - **Containers** ([`container::ContainerManager`]) -- sidecar
//!   container lifecycle and health integration.
//! - **Applications** ([`app::AppManager`]) -- application manifest
//!   parsing, validation, and lifecycle state machine.
//! - **Cluster** ([`cluster::ClusterMembership`]) -- multi-node
//!   cluster membership, peer tracking, and health.
//! - **Environments** ([`environment::EnvironmentManager`]) --
//!   governance-scoped dev/staging/prod environments.
//! - **Governance** ([`governance::GovernanceEngine`]) -- three-branch
//!   constitutional governance with effect algebra scoring.
//!
//! # Feature Flags
//!
//! - `native` (default) -- enables tokio runtime, native file I/O.
//! - `wasm-sandbox` -- enables WASM tool runner (Phase K3).
//! - `containers` -- enables container manager (Phase K4).

pub mod a2a;
pub mod app;
pub mod boot;
pub mod capability;
pub mod cluster;
pub mod config;
pub mod console;
pub mod container;
pub mod environment;
pub mod error;
pub mod governance;
pub mod health;
pub mod ipc;
pub mod process;
pub mod service;
pub mod supervisor;
pub mod topic;
pub mod wasm_runner;

// Re-export key types at the crate level for convenience.
pub use a2a::A2ARouter;
pub use app::{
    AgentSpec, AppCapabilities, AppError, AppHooks, AppManager, AppManifest, AppState,
    InstalledApp, ServiceSpec, ToolSource, ToolSpec,
};
pub use boot::{Kernel, KernelState};
pub use capability::{
    AgentCapabilities, CapabilityChecker, IpcScope, ResourceLimits, ResourceType, SandboxPolicy,
    ToolPermissions,
};
pub use cluster::{
    ClusterConfig, ClusterError, ClusterMembership, NodeId, NodePlatform, NodeState, PeerNode,
};
pub use clawft_types::config::KernelConfig;
pub use config::KernelConfigExt;
pub use console::{BootEvent, BootLog, BootPhase, LogLevel};
pub use container::{
    ContainerConfig, ContainerError, ContainerManager, ContainerState, ManagedContainer,
    PortMapping, RestartPolicy, VolumeMount,
};
pub use environment::{
    AuditLevel, Environment, EnvironmentClass, EnvironmentError, EnvironmentManager,
    GovernanceBranches, GovernanceScope, LearningMode,
};
pub use error::{KernelError, KernelResult};
pub use governance::{
    EffectVector, GovernanceBranch, GovernanceDecision, GovernanceEngine, GovernanceRequest,
    GovernanceResult, GovernanceRule, RuleSeverity,
};
pub use health::{HealthStatus, HealthSystem, OverallHealth};
pub use ipc::{KernelIpc, KernelMessage, KernelSignal, MessagePayload, MessageTarget};
pub use process::{Pid, ProcessEntry, ProcessState, ProcessTable, ResourceUsage};
pub use service::{ServiceRegistry, ServiceType, SystemService};
pub use supervisor::{AgentSupervisor, SpawnRequest, SpawnResult};
pub use topic::{Subscription, TopicRouter};
pub use wasm_runner::{
    WasmError, WasmSandboxConfig, WasmTool, WasmToolResult, WasmToolRunner, WasmValidation,
};
