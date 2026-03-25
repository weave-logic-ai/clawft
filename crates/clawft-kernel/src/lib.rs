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
//! - **Agency** ([`agency::Agency`]) -- agent-first architecture
//!   with roles, spawn permissions, and agent manifests.
//!
//! # Feature Flags
//!
//! - `native` (default) -- enables tokio runtime, native file I/O.
//! - `wasm-sandbox` -- enables WASM tool runner (Phase K3).
//! - `containers` -- enables container manager (Phase K4).
//! - `ecc` -- enables ECC cognitive substrate (Phase K3c).

// ── ECC cognitive substrate modules (K3c) ────────────────────────
#[cfg(feature = "ecc")]
pub mod calibration;
#[cfg(feature = "ecc")]
pub mod causal;
#[cfg(feature = "ecc")]
pub mod cognitive_tick;
#[cfg(feature = "ecc")]
pub mod crossref;
#[cfg(feature = "ecc")]
pub mod hnsw_service;
#[cfg(feature = "ecc")]
pub mod impulse;

pub mod a2a;
pub mod agency;
pub mod agent_loop;
pub mod app;
pub mod boot;
pub mod capability;
pub mod cluster;
pub mod config;
pub mod console;
pub mod container;
pub mod cron;
#[cfg(feature = "exochain")]
pub mod chain;
#[cfg(feature = "exochain")]
pub mod tree_manager;
pub mod environment;
pub mod error;
#[cfg(feature = "exochain")]
pub mod gate;
pub mod governance;
pub mod health;
pub mod ipc;
pub mod process;
pub mod service;
pub mod supervisor;
pub mod topic;
#[allow(clippy::new_without_default)]
pub mod wasm_runner;

// Re-export key types at the crate level for convenience.
pub use a2a::A2ARouter;
pub use agency::{
    Agency, AgentHealth, AgentInterface, AgentManifest, AgentPriority, AgentResources,
    AgentRestartPolicy, AgentRole, InterfaceProtocol, ResponseMode,
};
pub use app::{
    AgentSpec, AppCapabilities, AppError, AppHooks, AppManager, AppManifest, AppState,
    InstalledApp, ServiceSpec, ToolSource, ToolSpec,
};
pub use boot::{Kernel, KernelState};
pub use capability::{
    AgentCapabilities, CapabilityChecker, IpcScope, ResourceLimits, ResourceType, SandboxPolicy,
    ToolPermissions,
};
#[cfg(feature = "exochain")]
pub use chain::{
    AnchorReceipt, ChainAnchor, ChainCheckpoint, ChainEvent, ChainManager, ChainStatus,
    ChainVerifyResult, MockAnchor,
};
#[cfg(feature = "ecc")]
pub use calibration::{EccCalibration, EccCalibrationConfig};
#[cfg(feature = "ecc")]
pub use causal::{CausalEdgeType, CausalGraph};
#[cfg(feature = "ecc")]
pub use cognitive_tick::{CognitiveTick, CognitiveTickConfig, CognitiveTickStats};
#[cfg(feature = "ecc")]
pub use crossref::{CrossRef, CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};
#[cfg(feature = "exochain")]
pub use gate::{CapabilityGate, GateBackend, GateDecision, GovernanceGate};
#[cfg(feature = "exochain")]
pub use tree_manager::{TreeManager, TreeStats};
pub use clawft_types::config::{ClusterNetworkConfig, KernelConfig};
pub use cluster::{
    ClusterConfig, ClusterError, ClusterMembership, NodeId, NodePlatform, NodeState, PeerNode,
};
#[cfg(feature = "cluster")]
pub use cluster::ClusterService;
pub use config::KernelConfigExt;
pub use console::{BootEvent, BootLog, BootPhase, KernelEventLog, LogLevel};
pub use cron::CronService;
pub use container::{
    ContainerConfig, ContainerError, ContainerManager, ContainerService, ContainerState,
    ManagedContainer, PortMapping, RestartPolicy, VolumeMount,
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
#[cfg(feature = "ecc")]
pub use hnsw_service::{HnswSearchResult, HnswService, HnswServiceConfig};
#[cfg(feature = "ecc")]
pub use impulse::{ImpulseQueue, ImpulseType};
#[cfg(feature = "ecc")]
pub use cluster::NodeEccCapability;
pub use ipc::{KernelIpc, KernelMessage, KernelSignal, MessagePayload, MessageTarget};
pub use process::{Pid, ProcessEntry, ProcessState, ProcessTable, ResourceUsage};
pub use service::{
    ServiceAuditLevel, ServiceContract, ServiceEndpoint, ServiceEntry, ServiceRegistry,
    ServiceType, SystemService,
};
pub use supervisor::{AgentSupervisor, EnclaveConfig, SpawnBackend, SpawnRequest, SpawnResult};
pub use topic::{Subscription, TopicRouter};
pub use wasm_runner::{
    AgentInspectTool, AgentListTool, AgentResumeTool, AgentSendTool, AgentSpawnTool,
    AgentStopTool, AgentSuspendTool, BackendSelection, BuiltinTool, BuiltinToolSpec, Certificate,
    CompiledModuleCache, DeployedTool, FsCopyTool, FsCreateDirTool, FsExistsTool, FsGlobTool,
    FsMoveTool, FsReadDirTool, FsReadFileTool, FsRemoveTool, FsStatTool, FsWriteFileTool,
    SandboxConfig, SysCronAddTool, SysCronListTool, SysCronRemoveTool, SysEnvGetTool,
    SysServiceHealthTool, SysServiceListTool, ToolCategory, ToolError, ToolRegistry,
    ToolSigningAuthority, ToolVersion, WasmError, WasiFsScope, WasmSandboxConfig, WasmTool,
    WasmToolResult, WasmToolRunner, WasmValidation, builtin_tool_catalog, compute_module_hash,
    verify_tool_signature,
};
#[cfg(feature = "exochain")]
pub use wasm_runner::{
    SysChainQueryTool, SysChainStatusTool, SysTreeInspectTool, SysTreeReadTool,
};
