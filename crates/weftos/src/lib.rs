//! WeftOS: A portable AI kernel for any project.
//!
//! Add WeftOS to your project to get process management, mesh networking,
//! capability-based security, an append-only audit chain, and a cognitive
//! substrate that learns your codebase.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use weftos::WeftOs;
//!
//! #[tokio::main]
//! async fn main() {
//!     let os = WeftOs::boot_default().await.unwrap();
//!     println!("WeftOS running: {} services", os.service_count());
//!     os.shutdown().await.unwrap();
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `native` (default) -- Tokio runtime, native file I/O
//! - `exochain` -- Append-only hash chain with Ed25519 + ML-DSA-65 signing
//! - `cluster` -- Multi-node clustering via ruvector
//! - `mesh` -- Encrypted peer-to-peer mesh networking
//! - `ecc` -- Ephemeral Causal Cognition (causal DAG, HNSW, cognitive tick)
//! - `wasm-sandbox` -- Wasmtime-based tool execution
//! - `containers` -- Docker/Podman sidecar orchestration
//! - `os-patterns` -- Self-healing, metrics, reliable IPC, timers
//! - `full` -- Everything

pub mod init;

// Re-export the kernel under the weftos namespace
pub use clawft_kernel as kernel;

// Re-export key types at the top level for ergonomic API
pub use clawft_kernel::{
    Kernel, KernelState,
    // Process management
    Pid, ProcessEntry, ProcessState, ProcessTable,
    // Agent supervision
    AgentSupervisor, SpawnBackend, SpawnRequest, SpawnResult,
    // Capabilities
    AgentCapabilities, CapabilityChecker, IpcScope, ResourceLimits,
    // IPC
    KernelIpc, KernelMessage, KernelSignal, MessagePayload, MessageTarget,
    GlobalPid,
    // Services
    ServiceRegistry, ServiceEntry, SystemService,
    // Health
    HealthSystem, HealthStatus, OverallHealth,
    // Governance
    GovernanceEngine, GovernanceRule, GovernanceRequest, GovernanceDecision,
    EffectVector,
    // Topics
    TopicRouter, Subscription,
    // Cluster
    ClusterConfig, ClusterMembership, NodeState, PeerNode,
    // Config
    KernelConfigExt,
    // Console
    BootEvent, BootPhase, BootLog,
    // Error
    KernelError, KernelResult,
    // Cron
    CronService,
    // Apps
    AppManager, AppManifest, InstalledApp, AppState,
    // Containers
    ContainerManager, ContainerConfig, ContainerState,
};

// Conditional re-exports
#[cfg(feature = "exochain")]
pub use clawft_kernel::{
    ChainManager, ChainEvent, TreeManager, TreeStats,
    GateBackend, GateDecision, GovernanceGate, CapabilityGate,
};

#[cfg(feature = "ecc")]
pub use clawft_kernel::{
    CausalGraph, CausalEdgeType, CognitiveTick, CognitiveTickConfig,
    CrossRef, CrossRefStore, CrossRefType, UniversalNodeId,
    HnswService, HnswServiceConfig, HnswSearchResult,
    ImpulseQueue, ImpulseType,
    EccCalibration,
};

#[cfg(feature = "mesh")]
pub use clawft_kernel::{
    MeshError, MeshPeer, MeshStream, MeshTransport, TransportListener,
    WeftHandshake, MeshConnectionPool, TcpTransport,
    DiscoveryCoordinator, BootstrapDiscovery,
    MeshIpcEnvelope, DedupFilter,
    HeartbeatTracker, HeartbeatConfig, HeartbeatState,
    DistributedProcessTable, ClusterServiceRegistry,
};

#[cfg(feature = "os-patterns")]
pub use clawft_kernel::{
    MetricsRegistry, LogService, TimerService,
    DeadLetterQueue, ReliableQueue, NamedPipeRegistry,
    ReconciliationController,
};

use std::path::Path;
use std::sync::Arc;

use clawft_platform::NativePlatform;

/// The main WeftOS instance -- boots and manages the kernel.
pub struct WeftOs {
    kernel: clawft_kernel::Kernel<NativePlatform>,
    project_root: std::path::PathBuf,
}

impl WeftOs {
    /// Boot WeftOS with default configuration.
    pub async fn boot_default() -> Result<Self, KernelError> {
        Self::boot_in(std::env::current_dir().unwrap_or_else(|_| ".".into())).await
    }

    /// Boot WeftOS in a specific project directory.
    pub async fn boot_in(project_root: impl Into<std::path::PathBuf>) -> Result<Self, KernelError> {
        let project_root = project_root.into();
        let config = clawft_types::config::Config::default();
        let kernel_config = clawft_types::config::KernelConfig::default();
        let platform = Arc::new(NativePlatform::new());

        let kernel = Kernel::boot(config, kernel_config, platform).await?;

        Ok(Self { kernel, project_root })
    }

    /// Boot WeftOS with custom configuration.
    pub async fn boot_with(
        config: clawft_types::config::Config,
        kernel_config: clawft_types::config::KernelConfig,
        project_root: impl Into<std::path::PathBuf>,
    ) -> Result<Self, KernelError> {
        let project_root = project_root.into();
        let platform = Arc::new(NativePlatform::new());
        let kernel = Kernel::boot(config, kernel_config, platform).await?;
        Ok(Self { kernel, project_root })
    }

    /// Get the project root directory.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Get the kernel state.
    pub fn state(&self) -> &KernelState {
        self.kernel.state()
    }

    /// Get the number of registered services.
    pub fn service_count(&self) -> usize {
        self.kernel.services().len()
    }

    /// Get the number of active processes.
    pub fn process_count(&self) -> usize {
        self.kernel.process_table().len()
    }

    /// Get a reference to the underlying kernel.
    pub fn kernel(&self) -> &Kernel<NativePlatform> {
        &self.kernel
    }

    /// Get a mutable reference to the underlying kernel.
    pub fn kernel_mut(&mut self) -> &mut Kernel<NativePlatform> {
        &mut self.kernel
    }

    /// Shut down WeftOS gracefully.
    pub async fn shutdown(mut self) -> KernelResult<()> {
        tracing::info!("WeftOS shutting down");
        self.kernel.shutdown().await
    }
}

/// Version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if WeftOS is initialized in the given directory.
pub fn is_initialized(path: impl AsRef<Path>) -> bool {
    path.as_ref().join(".weftos").exists()
        || path.as_ref().join("weave.toml").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_not_initialized_in_random_dir() {
        assert!(!is_initialized("/tmp/nonexistent-weftos-test"));
    }

    #[test]
    fn init_creates_structure() {
        let dir = std::env::temp_dir().join("weftos-test-init");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = init::init_project(&dir).unwrap();
        assert!(result.weftos_dir_created);
        assert!(result.weave_toml_created);
        assert!(dir.join(".weftos").exists());
        assert!(dir.join("weave.toml").exists());
        assert!(dir.join(".weftos/chain").exists());
        assert!(dir.join(".weftos/logs").exists());

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn init_detects_rust_project() {
        let dir = std::env::temp_dir().join("weftos-test-rust");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let _result = init::init_project(&dir).unwrap();
        let config = std::fs::read_to_string(dir.join("weave.toml")).unwrap();
        assert!(config.contains("language = \"rust\""));
        assert!(config.contains("*.rs"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn weftos_boots_and_reports_state() {
        let os = WeftOs::boot_default().await.unwrap();
        assert!(matches!(os.state(), KernelState::Running));
        assert!(os.service_count() > 0);
        os.shutdown().await.unwrap();
    }
}
