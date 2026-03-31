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
pub mod artifact_store;
#[cfg(feature = "ecc")]
pub mod calibration;
#[cfg(feature = "ecc")]
pub mod causal;
#[cfg(feature = "ecc")]
pub mod cognitive_tick;
#[cfg(feature = "ecc")]
pub mod crossref;
#[cfg(feature = "ecc")]
pub mod democritus;
#[cfg(feature = "ecc")]
pub mod embedding;
#[cfg(feature = "ecc")]
pub mod embedding_onnx;
#[cfg(feature = "ecc")]
pub mod hnsw_service;
#[cfg(feature = "ecc")]
pub mod impulse;
#[cfg(feature = "ecc")]
pub mod persistence;
#[cfg(feature = "ecc")]
pub mod weaver;

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

// ── Self-healing & process management modules (08a) ─────────────
#[cfg(feature = "os-patterns")]
pub mod monitor;
#[cfg(feature = "os-patterns")]
pub mod reconciler;
#[allow(clippy::new_without_default)]
pub mod wasm_runner;

// ── Reliable IPC & observability modules (08b) ──────────────────
#[cfg(feature = "os-patterns")]
pub mod dead_letter;
#[cfg(feature = "os-patterns")]
pub mod reliable_queue;
#[cfg(feature = "os-patterns")]
pub mod named_pipe;
#[cfg(feature = "os-patterns")]
pub mod metrics;
#[cfg(feature = "os-patterns")]
pub mod log_service;
#[cfg(feature = "os-patterns")]
pub mod timer;

// ── Content integrity & operational services (08c) ───────────────
pub mod auth_service;
pub mod config_service;
pub mod tree_view;

// ── Mesh networking modules (K6) ──────────────────────────────
#[cfg(feature = "mesh")]
pub mod mesh;
#[cfg(feature = "mesh")]
pub mod mesh_noise;
#[cfg(feature = "mesh")]
pub mod mesh_framing;
#[cfg(feature = "mesh")]
pub mod mesh_listener;
#[cfg(feature = "mesh")]
pub mod mesh_discovery;
#[cfg(feature = "mesh")]
pub mod mesh_bootstrap;
#[cfg(feature = "mesh")]
pub mod mesh_ipc;
#[cfg(feature = "mesh")]
pub mod mesh_dedup;
#[cfg(feature = "mesh")]
pub mod mesh_service;
#[cfg(feature = "mesh")]
pub mod mesh_chain;
#[cfg(feature = "mesh")]
pub mod mesh_tree;
#[cfg(feature = "mesh")]
pub mod mesh_process;
#[cfg(feature = "mesh")]
pub mod mesh_service_adv;
#[cfg(feature = "mesh")]
pub mod mesh_heartbeat;
#[cfg(feature = "mesh")]
pub mod mesh_tcp;
#[cfg(feature = "mesh")]
pub mod mesh_ws;
#[cfg(feature = "mesh")]
pub mod mesh_mdns;
#[cfg(feature = "mesh")]
pub mod mesh_kad;
#[cfg(feature = "mesh")]
pub mod mesh_artifact;
#[cfg(feature = "mesh")]
pub mod mesh_log;
#[cfg(feature = "mesh")]
pub mod mesh_runtime;

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
    AgentCapabilities, CapabilityChecker, CapabilityElevationRequest, ElevationResult, IpcScope,
    ResourceLimits, ResourceType, SandboxPolicy, ToolPermissions,
};
#[cfg(feature = "exochain")]
pub use chain::{
    AnchorReceipt, ChainAnchor, ChainCheckpoint, ChainEvent, ChainLoggable, ChainManager,
    ChainStatus, ChainVerifyResult, GovernanceDecisionEvent, IpcDeadLetterEvent, MockAnchor,
    RestartEvent,
};
#[cfg(feature = "ecc")]
pub use calibration::{EccCalibration, EccCalibrationConfig};
#[cfg(feature = "ecc")]
pub use causal::{
    CausalEdge, CausalEdgeType, CausalGraph, CausalNode, ChangeEvent, ChangePrediction,
    CouplingPair, SpectralResult,
};
#[cfg(feature = "ecc")]
pub use cognitive_tick::{CognitiveTick, CognitiveTickConfig, CognitiveTickStats};
#[cfg(feature = "ecc")]
pub use crossref::{CrossRef, CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};
#[cfg(feature = "ecc")]
pub use democritus::{DemocritusConfig, DemocritusLoop, DemocritusTickResult};
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
pub use artifact_store::{ArtifactBackend, ArtifactStore, ArtifactType, StoredArtifact};
#[cfg(feature = "ecc")]
pub use persistence::PersistenceConfig;
#[cfg(feature = "ecc")]
pub use embedding::{
    select_embedding_provider, EmbeddingError, EmbeddingProvider, LlmEmbeddingConfig,
    LlmEmbeddingProvider, MockEmbeddingProvider,
};
#[cfg(feature = "ecc")]
pub use embedding_onnx::{
    AstEmbeddingProvider, OnnxEmbeddingProvider, RustCodeFeatures, SentenceTransformerProvider,
    extract_rust_features, preprocess_markdown, split_sentences,
};
#[cfg(feature = "ecc")]
pub use weaver::{
    ConfidenceGap, ConfidenceReport, DataSource, ExportedModel, IngestResult, MetaDecisionType,
    MetaLoomEvent, ModelingSession, ModelingSuggestion, StrategyPattern, TickResult,
    WeaverCommand, WeaverEngine, WeaverError, WeaverKnowledgeBase, WeaverResponse,
};
#[cfg(feature = "ecc")]
pub use cluster::NodeEccCapability;
pub use ipc::{
    ExitReason as SignalExitReason, GlobalPid, KernelIpc, KernelMessage, KernelSignal,
    MessagePayload, MessageTarget, ProcessDown as SignalProcessDown,
};
#[cfg(any(feature = "mesh", feature = "exochain"))]
pub use cluster::NodeIdentity;
#[cfg(feature = "mesh")]
pub use mesh::{MeshError, MeshPeer, MeshStream, MeshTransport, TransportListener, WeftHandshake, MAX_MESSAGE_SIZE};
#[cfg(feature = "mesh")]
pub use mesh_noise::{EncryptedChannel, NoiseConfig, NoisePattern};
#[cfg(feature = "mesh")]
pub use mesh_framing::{FrameType, MeshFrame};
#[cfg(feature = "mesh")]
pub use mesh_listener::{JoinRequest, JoinResponse, MeshConnectionPool, PeerInfo};
#[cfg(feature = "mesh")]
pub use mesh_discovery::{
    DiscoveredPeer, DiscoveryBackend, DiscoveryCoordinator, DiscoveryError, DiscoverySource,
};
#[cfg(feature = "mesh")]
pub use mesh_bootstrap::{BootstrapDiscovery, PeerExchangeDiscovery};
#[cfg(feature = "mesh")]
pub use mesh_ipc::{MeshIpcEnvelope, MeshIpcError};
#[cfg(feature = "mesh")]
pub use mesh_dedup::DedupFilter;
#[cfg(feature = "mesh")]
pub use mesh_service::{
    RemoteServiceEndpoint, ServiceResolutionCache, ServiceResolveRequest, ServiceResolveResponse,
};
#[cfg(feature = "mesh")]
pub use mesh_chain::{
    ChainBridgeEvent, ChainForkStatus, ChainSyncRequest, ChainSyncResponse, SyncStateDigest,
};
#[cfg(feature = "mesh")]
pub use mesh_tree::{
    MerkleProof, TreeDiffType, TreeNodeDiff, TreeSyncAction, TreeSyncRequest, TreeSyncResponse,
};
#[cfg(feature = "mesh")]
pub use mesh_process::{
    ConsensusEntry, ConsensusOp, ConsensusRole, ConsistentHashRing, CrdtGossipState,
    DistributedProcessTable, MetadataConsensus, ProcessAdvertisement, ProcessStatus,
    ResourceSummary,
};
#[cfg(feature = "mesh")]
pub use mesh_service_adv::{ClusterServiceRegistry, ServiceAdvertisement};
#[cfg(feature = "mesh")]
pub use mesh_heartbeat::{HeartbeatConfig, HeartbeatState, HeartbeatTracker, PeerHeartbeat, PingRequest, PingResponse};
#[cfg(feature = "mesh")]
pub use mesh_tcp::TcpTransport;
#[cfg(feature = "mesh")]
pub use mesh_ws::WsTransport;
#[cfg(feature = "mesh")]
pub use mesh_mdns::{MdnsAnnouncement, MdnsDiscovery, WEFTOS_SERVICE_NAME};
#[cfg(feature = "mesh")]
pub use mesh_kad::{
    DhtEntry, DhtKey, KademliaDiscovery, KademliaTable, NamespacedDhtKey,
    bucket_index, leading_zeros, xor_distance,
    K_BUCKET_SIZE, ALPHA, KEY_BITS,
};
#[cfg(feature = "mesh")]
pub use mesh_artifact::{ArtifactAnnouncement, ArtifactExchange, ArtifactRequest, ArtifactResponse};
#[cfg(feature = "mesh")]
pub use mesh_log::{LogAggregator, LogQuery as MeshLogQuery, RemoteLogEntry};
#[cfg(feature = "mesh")]
pub use mesh_runtime::{DiscoveryState, MeshRuntime, PeerConnection};
#[cfg(feature = "os-patterns")]
pub use auth_service::{
    AuditEntry, AuthService, AuthToken, CredentialGrant, CredentialRequest, CredentialType,
    HashedCredential, IssuedToken, StoredCredential as AuthStoredCredential,
};
#[cfg(feature = "os-patterns")]
pub use config_service::{ConfigChange, ConfigEntry, ConfigService, ConfigValue, SecretRef};
pub use tree_view::{AgentTreeView, TreeScope};
pub use process::{Pid, ProcessEntry, ProcessState, ProcessTable, ResourceUsage};
pub use service::{
    McpAdapter, ServiceApi, ServiceAuditLevel, ServiceContract, ServiceEndpoint, ServiceEntry,
    ServiceInfo, ServiceRegistry, ServiceType, ShellAdapter, SystemService,
};
pub use supervisor::{AgentSupervisor, EnclaveConfig, SpawnBackend, SpawnRequest, SpawnResult};
#[cfg(feature = "os-patterns")]
pub use supervisor::{
    RestartBudget, RestartStrategy, RestartTracker,
    ResourceCheckResult, check_resource_usage,
};
#[cfg(feature = "os-patterns")]
pub use monitor::{ExitReason, MonitorRegistry, ProcessDown, ProcessLink, ProcessMonitor};
#[cfg(feature = "os-patterns")]
pub use reconciler::{DesiredAgentState, DriftEvent, ReconciliationController};
#[cfg(feature = "os-patterns")]
pub use health::{ProbeConfig, ProbeResult, ProbeState};
// ── 08b re-exports ──────────────────────────────────────────────
#[cfg(feature = "os-patterns")]
pub use dead_letter::{DeadLetter, DeadLetterQueue, DeadLetterReason};
#[cfg(feature = "os-patterns")]
pub use reliable_queue::{DeliveryResult, PendingDelivery, ReliableConfig, ReliableQueue};
#[cfg(feature = "os-patterns")]
pub use named_pipe::{NamedPipe, NamedPipeRegistry, PipeInfo};
#[cfg(feature = "os-patterns")]
pub use metrics::{
    Histogram, MetricSnapshot, MetricsRegistry,
    METRIC_MESSAGES_SENT, METRIC_MESSAGES_DELIVERED, METRIC_MESSAGES_DROPPED,
    METRIC_AGENT_SPAWNS, METRIC_AGENT_CRASHES, METRIC_TOOL_EXECUTIONS,
    METRIC_ACTIVE_AGENTS, METRIC_ACTIVE_SERVICES, METRIC_CHAIN_LENGTH,
    METRIC_IPC_LATENCY_MS, METRIC_TOOL_EXECUTION_MS, METRIC_GOVERNANCE_EVAL_MS,
};
#[cfg(feature = "os-patterns")]
pub use log_service::{LogEntry, LogQuery, LogService};
#[cfg(feature = "os-patterns")]
pub use timer::{TimerEntry, TimerInfo, TimerService};
pub use topic::{Subscription, TopicRouter};
pub use wasm_runner::{
    AgentInspectTool, AgentListTool, AgentResumeTool, AgentSendTool, AgentSpawnTool,
    AgentStopTool, AgentSuspendTool, BackendSelection, BuiltinTool, BuiltinToolSpec, Certificate,
    IpcSendTool, IpcSubscribeTool,
    CompiledModuleCache, DeployedTool, FsCopyTool, FsCreateDirTool, FsExistsTool, FsGlobTool,
    FsMoveTool, FsReadDirTool, FsReadFileTool, FsRemoveTool, FsStatTool, FsWriteFileTool,
    SandboxConfig, SysCronAddTool, SysCronListTool, SysCronRemoveTool, SysEnvGetTool,
    SysServiceHealthTool, SysServiceListTool, ToolCategory, ToolError, ToolRegistry,
    ToolSigningAuthority, ToolVersion, WasmError, WasiFsScope, WasmSandboxConfig, WasmTool,
    ShellPipeline, WasmToolResult, WasmToolRunner, WasmValidation, builtin_tool_catalog,
    compute_module_hash, verify_tool_signature,
};
#[cfg(feature = "exochain")]
pub use wasm_runner::{
    SysChainQueryTool, SysChainStatusTool, SysTreeInspectTool, SysTreeReadTool,
};
