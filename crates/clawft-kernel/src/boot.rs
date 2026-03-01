//! Kernel boot sequence and state machine.
//!
//! The [`Kernel`] struct is the central coordinator. It wraps
//! [`AppContext`] and manages the process table, service registry,
//! IPC, and health subsystem through a boot/shutdown lifecycle.

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{error, info};

use clawft_core::bootstrap::AppContext;
use clawft_core::bus::MessageBus;
use clawft_platform::Platform;
use clawft_types::config::Config;

use crate::capability::AgentCapabilities;
use crate::console::{BootEvent, BootLog, BootPhase, KernelEventLog};
use crate::error::{KernelError, KernelResult};
use crate::health::HealthSystem;
use crate::ipc::KernelIpc;
use crate::process::{ProcessEntry, ProcessState, ProcessTable, ResourceUsage};
use crate::service::ServiceRegistry;
use crate::supervisor::AgentSupervisor;
use clawft_types::config::KernelConfig;

/// Kernel lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelState {
    /// Kernel is in the process of booting.
    Booting,
    /// Kernel is running and accepting commands.
    Running,
    /// Kernel is shutting down.
    ShuttingDown,
    /// Kernel has been halted.
    Halted,
}

impl std::fmt::Display for KernelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelState::Booting => write!(f, "booting"),
            KernelState::Running => write!(f, "running"),
            KernelState::ShuttingDown => write!(f, "shutting_down"),
            KernelState::Halted => write!(f, "halted"),
        }
    }
}

/// The WeftOS kernel.
///
/// Wraps `AppContext<P>` in a managed boot sequence with process
/// tracking, service lifecycle, IPC, and health monitoring.
///
/// # Lifecycle
///
/// 1. Call [`Kernel::boot`] to initialize all subsystems.
/// 2. The kernel transitions from `Booting` -> `Running`.
/// 3. Call [`Kernel::shutdown`] to gracefully stop everything.
/// 4. The kernel transitions from `Running` -> `ShuttingDown` -> `Halted`.
pub struct Kernel<P: Platform> {
    state: KernelState,
    config: KernelConfig,
    app_context: Option<AppContext<P>>,
    bus: Arc<MessageBus>,
    process_table: Arc<ProcessTable>,
    service_registry: Arc<ServiceRegistry>,
    ipc: Arc<KernelIpc>,
    health: HealthSystem,
    supervisor: AgentSupervisor<P>,
    boot_log: BootLog,
    event_log: Arc<KernelEventLog>,
    boot_time: Instant,
}

impl<P: Platform> Kernel<P> {
    /// Boot the kernel from configuration and platform.
    ///
    /// This is the primary entry point. It:
    /// 1. Creates subsystems (process table, service registry, IPC, health)
    /// 2. Creates AppContext (reuses existing bootstrap)
    /// 3. Registers the kernel process (PID 0)
    /// 4. Starts all registered services
    /// 5. Transitions to Running state
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::Boot`] if any critical subsystem fails
    /// to initialize.
    pub async fn boot(
        config: Config,
        kernel_config: KernelConfig,
        platform: Arc<P>,
    ) -> KernelResult<Self> {
        let boot_time = Instant::now();
        let mut boot_log = BootLog::new();

        info!("WeftOS kernel booting");
        boot_log.push(BootEvent::info(BootPhase::Init, "WeftOS v0.1.0 booting..."));
        boot_log.push(BootEvent::info(BootPhase::Init, "PID 0 (kernel)"));

        // 1. Create subsystems
        let process_table = Arc::new(ProcessTable::new(kernel_config.max_processes));
        let service_registry = Arc::new(ServiceRegistry::new());

        boot_log.push(BootEvent::info(
            BootPhase::Config,
            format!("Max processes: {}", kernel_config.max_processes),
        ));
        boot_log.push(BootEvent::info(
            BootPhase::Config,
            format!(
                "Health check interval: {}s",
                kernel_config.health_check_interval_secs
            ),
        ));

        // 2. Create AppContext
        let app_context = AppContext::new(config, platform)
            .await
            .map_err(|e| KernelError::Boot(format!("AppContext init failed: {e}")))?;

        // 3. Create IPC from the AppContext's MessageBus
        let bus = app_context.bus().clone();
        let ipc = Arc::new(KernelIpc::new(bus.clone()));

        // 4. Create health system
        let health = HealthSystem::new(kernel_config.health_check_interval_secs);

        // 5. Register kernel process (PID 0)
        let kernel_entry = ProcessEntry {
            pid: 0,
            agent_id: "kernel".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: tokio_util::sync::CancellationToken::new(),
            parent_pid: None,
        };
        process_table
            .insert_with_pid(kernel_entry)
            .map_err(|e| KernelError::Boot(format!("failed to register kernel process: {e}")))?;

        boot_log.push(BootEvent::info(
            BootPhase::Services,
            "Service registry ready",
        ));

        // 6. Start services (none registered by default at boot)
        service_registry
            .start_all()
            .await
            .map_err(|e| KernelError::Boot(format!("service start failed: {e}")))?;

        // TODO: ruvector integration -- when ruvector-cluster feature is enabled,
        // use ClusterManager for service registry and health checking.

        // TODO: exo-resource-tree -- when exo-dag feature is enabled,
        // load resource tree from checkpoint during boot.

        let elapsed = boot_time.elapsed();
        boot_log.push(BootEvent::info(
            BootPhase::Ready,
            format!(
                "Boot complete in {:.1}s ({} processes, {} services)",
                elapsed.as_secs_f64(),
                process_table.len(),
                service_registry.len(),
            ),
        ));

        info!(
            elapsed_ms = elapsed.as_millis(),
            processes = process_table.len(),
            services = service_registry.len(),
            "kernel boot complete"
        );

        // 7. Create agent supervisor
        let supervisor = AgentSupervisor::new(
            process_table.clone(),
            ipc.clone(),
            AgentCapabilities::default(),
        );

        // 8. Seed the event ring buffer with boot events
        let event_log = Arc::new(KernelEventLog::new());
        event_log.ingest_boot_log(&boot_log);

        Ok(Self {
            state: KernelState::Running,
            config: kernel_config,
            app_context: Some(app_context),
            bus,
            process_table,
            service_registry,
            ipc,
            health,
            supervisor,
            boot_log,
            event_log,
            boot_time,
        })
    }

    /// Shut down the kernel gracefully.
    ///
    /// Stops all services, cancels all processes, and transitions
    /// to the `Halted` state.
    pub async fn shutdown(&mut self) -> KernelResult<()> {
        if self.state != KernelState::Running {
            return Err(KernelError::WrongState {
                expected: "Running".into(),
                actual: self.state.to_string(),
            });
        }

        info!("kernel shutting down");
        self.state = KernelState::ShuttingDown;
        self.event_log.info("kernel", "shutdown initiated");

        // Stop all services
        if let Err(e) = self.service_registry.stop_all().await {
            error!(error = %e, "error stopping services during shutdown");
        }

        // Cancel all processes
        for entry in self.process_table.list() {
            if entry.pid == 0 {
                continue; // Don't cancel the kernel process
            }
            entry.cancel_token.cancel();
            let _ = self
                .process_table
                .update_state(entry.pid, ProcessState::Exited(0));
        }

        self.state = KernelState::Halted;
        self.event_log.info("kernel", "halted");
        info!("kernel halted");
        Ok(())
    }

    /// Get the current kernel state.
    pub fn state(&self) -> &KernelState {
        &self.state
    }

    /// Get the kernel configuration.
    pub fn kernel_config(&self) -> &KernelConfig {
        &self.config
    }

    /// Get the process table.
    pub fn process_table(&self) -> &Arc<ProcessTable> {
        &self.process_table
    }

    /// Get the service registry.
    pub fn services(&self) -> &Arc<ServiceRegistry> {
        &self.service_registry
    }

    /// Get the IPC subsystem.
    pub fn ipc(&self) -> &Arc<KernelIpc> {
        &self.ipc
    }

    /// Get the message bus.
    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    /// Get the health system.
    pub fn health(&self) -> &HealthSystem {
        &self.health
    }

    /// Get the agent supervisor.
    pub fn supervisor(&self) -> &AgentSupervisor<P> {
        &self.supervisor
    }

    /// Get the boot log.
    pub fn boot_log(&self) -> &BootLog {
        &self.boot_log
    }

    /// Get the runtime event log (ring buffer).
    pub fn event_log(&self) -> &Arc<KernelEventLog> {
        &self.event_log
    }

    /// Get kernel uptime.
    pub fn uptime(&self) -> std::time::Duration {
        self.boot_time.elapsed()
    }

    /// Take ownership of the AppContext for agent loop consumption.
    ///
    /// This is a one-shot operation: after calling this, the kernel
    /// no longer holds the AppContext. Use this before calling
    /// `AppContext::into_agent_loop()`.
    pub fn take_app_context(&mut self) -> Option<AppContext<P>> {
        self.app_context.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_platform::NativePlatform;
    use clawft_types::config::{AgentDefaults, AgentsConfig};

    fn test_config() -> Config {
        Config {
            agents: AgentsConfig {
                defaults: AgentDefaults {
                    workspace: "~/.clawft/workspace".into(),
                    model: "test/model".into(),
                    max_tokens: 1024,
                    temperature: 0.5,
                    max_tool_iterations: 5,
                    memory_window: 10,
                },
            },
            ..Config::default()
        }
    }

    fn test_kernel_config() -> KernelConfig {
        KernelConfig {
            enabled: true,
            max_processes: 16,
            health_check_interval_secs: 5,
        }
    }

    #[tokio::test]
    async fn boot_and_shutdown() {
        let platform = Arc::new(NativePlatform::new());
        let mut kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        assert_eq!(*kernel.state(), KernelState::Running);
        // Uptime should be non-negative (boot_time is set before boot completes)
        let _uptime = kernel.uptime();

        // Kernel process should be PID 0
        let kernel_proc = kernel.process_table().get(0).unwrap();
        assert_eq!(kernel_proc.agent_id, "kernel");
        assert_eq!(kernel_proc.state, ProcessState::Running);

        kernel.shutdown().await.unwrap();
        assert_eq!(*kernel.state(), KernelState::Halted);
    }

    #[tokio::test]
    async fn double_shutdown_fails() {
        let platform = Arc::new(NativePlatform::new());
        let mut kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        kernel.shutdown().await.unwrap();
        let result = kernel.shutdown().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn boot_log_has_events() {
        let platform = Arc::new(NativePlatform::new());
        let kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        let log = kernel.boot_log();
        assert!(!log.is_empty());

        let formatted = log.format_all();
        assert!(formatted.contains("WeftOS v0.1.0"));
        assert!(formatted.contains("Boot complete"));
    }

    #[tokio::test]
    async fn process_table_accessible() {
        let platform = Arc::new(NativePlatform::new());
        let kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        let pt = kernel.process_table();
        assert_eq!(pt.len(), 1); // kernel process only
        assert_eq!(pt.max_processes(), 16);
    }

    #[tokio::test]
    async fn services_accessible() {
        let platform = Arc::new(NativePlatform::new());
        let kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        assert!(kernel.services().is_empty());
    }

    #[tokio::test]
    async fn take_app_context() {
        let platform = Arc::new(NativePlatform::new());
        let mut kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        let ctx = kernel.take_app_context();
        assert!(ctx.is_some());

        // Second take returns None
        let ctx2 = kernel.take_app_context();
        assert!(ctx2.is_none());
    }

    #[tokio::test]
    async fn ipc_accessible() {
        let platform = Arc::new(NativePlatform::new());
        let kernel = Kernel::boot(test_config(), test_kernel_config(), platform)
            .await
            .unwrap();

        let ipc = kernel.ipc();
        assert!(Arc::ptr_eq(ipc.bus(), kernel.bus()));
    }

    #[test]
    fn kernel_state_display() {
        assert_eq!(KernelState::Booting.to_string(), "booting");
        assert_eq!(KernelState::Running.to_string(), "running");
        assert_eq!(KernelState::ShuttingDown.to_string(), "shutting_down");
        assert_eq!(KernelState::Halted.to_string(), "halted");
    }
}
