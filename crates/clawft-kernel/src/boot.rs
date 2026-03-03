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

use crate::a2a::A2ARouter;
use crate::capability::{AgentCapabilities, CapabilityChecker};
use crate::cluster::{ClusterConfig, ClusterMembership};
use crate::console::{BootEvent, BootLog, BootPhase, KernelEventLog};
use crate::error::{KernelError, KernelResult};
use crate::health::HealthSystem;
use crate::ipc::KernelIpc;
use crate::process::{ProcessEntry, ProcessState, ProcessTable, ResourceUsage};
use crate::service::ServiceRegistry;
use crate::supervisor::AgentSupervisor;
use crate::topic::TopicRouter;
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
    a2a_router: Arc<A2ARouter>,
    cron_service: Arc<crate::cron::CronService>,
    health: HealthSystem,
    supervisor: AgentSupervisor<P>,
    boot_log: BootLog,
    event_log: Arc<KernelEventLog>,
    boot_time: Instant,
    cluster_membership: Arc<ClusterMembership>,
    #[cfg(feature = "exochain")]
    chain_manager: Option<Arc<crate::chain::ChainManager>>,
    #[cfg(feature = "exochain")]
    tree_manager: Option<Arc<crate::tree_manager::TreeManager>>,
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

        // 5a. Construct A2ARouter (per-PID inboxes, capability-checked routing)
        let capability_checker = Arc::new(CapabilityChecker::new(process_table.clone()));
        let topic_router = Arc::new(TopicRouter::new(process_table.clone()));
        let a2a_router = Arc::new(A2ARouter::new(
            process_table.clone(),
            capability_checker,
            topic_router,
        ));

        boot_log.push(BootEvent::info(BootPhase::Services, "A2A router ready"));

        // 5b. Register cron service (K0 gate requirement)
        let cron_svc = Arc::new(crate::cron::CronService::new());
        if let Err(e) = service_registry.register(cron_svc.clone()) {
            error!(error = %e, "failed to register cron service");
        } else {
            boot_log.push(BootEvent::info(BootPhase::Services, "Cron service registered"));
        }

        // 6. Create cluster membership (universal, always present)
        let cluster_config = ClusterConfig {
            node_id: uuid::Uuid::new_v4().to_string(),
            node_name: kernel_config
                .cluster
                .as_ref()
                .and_then(|c| c.node_name.clone())
                .unwrap_or_else(|| "local".into()),
            heartbeat_interval_secs: kernel_config
                .cluster
                .as_ref()
                .map(|c| c.heartbeat_interval_secs)
                .unwrap_or(5),
            ..ClusterConfig::default()
        };
        let cluster_membership = Arc::new(ClusterMembership::new(cluster_config));

        boot_log.push(BootEvent::info(
            BootPhase::Network,
            format!(
                "Cluster membership ready (node {})",
                cluster_membership.local_node_id()
            ),
        ));

        // 7. Register cluster service (when feature-gated ruvector integration is enabled)
        #[cfg(feature = "cluster")]
        {
            use crate::cluster::ClusterService;
            use ruvector_cluster::StaticDiscovery;

            let net_config = kernel_config
                .cluster
                .clone()
                .unwrap_or_default();
            let seed_addrs: Vec<std::net::SocketAddr> = net_config
                .seed_nodes
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            let seed_nodes: Vec<ruvector_cluster::ClusterNode> = seed_addrs
                .into_iter()
                .map(|addr| ruvector_cluster::ClusterNode::new(addr.to_string(), addr))
                .collect();
            let discovery = Box::new(StaticDiscovery::new(seed_nodes));
            let node_id = cluster_membership.local_node_id().to_owned();

            match ClusterService::new(
                net_config,
                node_id,
                discovery,
                Arc::clone(&cluster_membership),
            ) {
                Ok(cluster_svc) => {
                    let svc = Arc::new(cluster_svc);
                    if let Err(e) = service_registry.register(svc) {
                        error!(error = %e, "failed to register cluster service");
                    } else {
                        boot_log.push(BootEvent::info(
                            BootPhase::Network,
                            "Cluster service registered (ruvector)",
                        ));
                    }
                }
                Err(e) => {
                    error!(error = %e, "failed to create cluster service");
                    boot_log.push(BootEvent::info(
                        BootPhase::Network,
                        format!("Cluster service failed: {e}"),
                    ));
                }
            }
        }

        // 8. Start services (none registered by default at boot, unless cluster feature added one)
        service_registry
            .start_all()
            .await
            .map_err(|e| KernelError::Boot(format!("service start failed: {e}")))?;

        // 8b. Initialize local exochain (when exochain feature is enabled)
        //     Restores from checkpoint file if available; otherwise fresh genesis.
        #[cfg(feature = "exochain")]
        let chain_manager = {
            let chain_config = kernel_config.chain.clone().unwrap_or_default();
            if chain_config.enabled {
                let cm = if let Some(ref ckpt_path) = chain_config.effective_checkpoint_path() {
                    let json_path = std::path::PathBuf::from(ckpt_path);
                    // Derive RVF path from JSON path: same directory, `.rvf` extension
                    let rvf_path = json_path.with_extension("rvf");

                    if rvf_path.exists() {
                        // Prefer RVF format (cryptographic integrity verification)
                        match crate::chain::ChainManager::load_from_rvf(&rvf_path, chain_config.checkpoint_interval) {
                            Ok(restored) => {
                                let seq = restored.sequence();
                                boot_log.push(BootEvent::info(
                                    BootPhase::Services,
                                    format!(
                                        "Chain restored from RVF (seq={}, chain_id={})",
                                        seq, chain_config.chain_id,
                                    ),
                                ));
                                Arc::new(restored)
                            }
                            Err(e) => {
                                error!(error = %e, "failed to restore RVF chain, trying JSON fallback");
                                // Fall back to JSON
                                if json_path.exists() {
                                    match crate::chain::ChainManager::load_from_file(&json_path, chain_config.checkpoint_interval) {
                                        Ok(restored) => {
                                            let seq = restored.sequence();
                                            boot_log.push(BootEvent::info(
                                                BootPhase::Services,
                                                format!(
                                                    "Chain restored from JSON fallback (seq={}, chain_id={})",
                                                    seq, chain_config.chain_id,
                                                ),
                                            ));
                                            Arc::new(restored)
                                        }
                                        Err(e2) => {
                                            error!(error = %e2, "JSON fallback also failed, starting fresh");
                                            boot_log.push(BootEvent::info(
                                                BootPhase::Services,
                                                format!("Chain restore failed (RVF: {e}, JSON: {e2}), starting fresh"),
                                            ));
                                            Arc::new(crate::chain::ChainManager::new(
                                                chain_config.chain_id,
                                                chain_config.checkpoint_interval,
                                            ))
                                        }
                                    }
                                } else {
                                    boot_log.push(BootEvent::info(
                                        BootPhase::Services,
                                        format!("RVF restore failed: {e}, starting fresh"),
                                    ));
                                    Arc::new(crate::chain::ChainManager::new(
                                        chain_config.chain_id,
                                        chain_config.checkpoint_interval,
                                    ))
                                }
                            }
                        }
                    } else if json_path.exists() {
                        // Legacy JSON format
                        match crate::chain::ChainManager::load_from_file(&json_path, chain_config.checkpoint_interval) {
                            Ok(restored) => {
                                let seq = restored.sequence();
                                boot_log.push(BootEvent::info(
                                    BootPhase::Services,
                                    format!(
                                        "Chain restored from JSON (seq={}, chain_id={}, will migrate to RVF)",
                                        seq, chain_config.chain_id,
                                    ),
                                ));
                                Arc::new(restored)
                            }
                            Err(e) => {
                                error!(error = %e, "failed to restore chain, starting fresh");
                                boot_log.push(BootEvent::info(
                                    BootPhase::Services,
                                    format!("Chain restore failed: {e}, starting fresh"),
                                ));
                                Arc::new(crate::chain::ChainManager::new(
                                    chain_config.chain_id,
                                    chain_config.checkpoint_interval,
                                ))
                            }
                        }
                    } else {
                        Arc::new(crate::chain::ChainManager::new(
                            chain_config.chain_id,
                            chain_config.checkpoint_interval,
                        ))
                    }
                } else {
                    Arc::new(crate::chain::ChainManager::new(
                        chain_config.chain_id,
                        chain_config.checkpoint_interval,
                    ))
                };

                boot_log.push(BootEvent::info(
                    BootPhase::Services,
                    format!(
                        "Local chain ready (chain_id={}, seq={})",
                        chain_config.chain_id,
                        cm.sequence(),
                    ),
                ));

                // Log boot phases to chain
                cm.append(
                    "kernel",
                    "boot.init",
                    Some(serde_json::json!({"version": "0.1.0"})),
                );
                cm.append(
                    "kernel",
                    "boot.config",
                    Some(serde_json::json!({
                        "max_processes": kernel_config.max_processes,
                        "health_interval": kernel_config.health_check_interval_secs,
                    })),
                );
                cm.append(
                    "kernel",
                    "boot.services",
                    Some(serde_json::json!({
                        "count": service_registry.len(),
                    })),
                );

                Some(cm)
            } else {
                boot_log.push(BootEvent::info(
                    BootPhase::Services,
                    "Local chain disabled",
                ));
                None
            }
        };

        // 8c. Bootstrap resource tree via TreeManager (when exochain feature is enabled)
        //     First attempt to restore from checkpoint; fall back to fresh bootstrap.
        #[cfg(feature = "exochain")]
        let tree_manager = {
            let rt_config = kernel_config.resource_tree.clone().unwrap_or_default();
            if rt_config.enabled {
                if let Some(ref cm) = chain_manager {
                    let tm = Arc::new(crate::tree_manager::TreeManager::new(Arc::clone(cm)));

                    // Derive tree checkpoint path from chain checkpoint path
                    let chain_cfg = kernel_config.chain.clone().unwrap_or_default();
                    let tree_ckpt_path = chain_cfg
                        .effective_checkpoint_path()
                        .map(|p| std::path::PathBuf::from(p).with_extension("tree.json"));

                    let mut restored_from_checkpoint = false;
                    if let Some(ref tree_path) = tree_ckpt_path {
                        if tree_path.exists() {
                            match tm.load_checkpoint(tree_path) {
                                Ok(()) => {
                                    let stats = tm.stats();
                                    boot_log.push(BootEvent::info(
                                        BootPhase::ResourceTree,
                                        format!(
                                            "Resource tree restored from checkpoint ({} nodes, root={}...)",
                                            stats.node_count,
                                            &stats.root_hash[..12],
                                        ),
                                    ));
                                    restored_from_checkpoint = true;
                                }
                                Err(e) => {
                                    error!(error = %e, "failed to restore tree checkpoint, bootstrapping fresh");
                                }
                            }
                        }
                    }

                    if !restored_from_checkpoint {
                        if let Err(e) = tm.bootstrap() {
                            error!(error = %e, "failed to bootstrap resource tree");
                            // Still allow boot to proceed without tree
                        } else {
                            let stats = tm.stats();
                            boot_log.push(BootEvent::info(
                                BootPhase::ResourceTree,
                                format!(
                                    "Resource tree bootstrapped ({} nodes, root={}...)",
                                    stats.node_count,
                                    &stats.root_hash[..12],
                                ),
                            ));
                        }
                    }

                    // Register cron service in tree with manifest (5b wiring)
                    if let Err(e) = tm.register_service_with_manifest("cron", "scheduler") {
                        tracing::debug!(error = %e, "failed to register cron in tree (may already exist)");
                    }

                    Some(tm)
                } else {
                    boot_log.push(BootEvent::info(
                        BootPhase::ResourceTree,
                        "Resource tree requires chain — skipped",
                    ));
                    None
                }
            } else {
                boot_log.push(BootEvent::info(
                    BootPhase::ResourceTree,
                    "Resource tree disabled",
                ));
                None
            }
        };

        // 8d. Log cluster and boot.ready chain events
        #[cfg(feature = "exochain")]
        if let Some(ref cm) = chain_manager {
            cm.append(
                "kernel",
                "boot.cluster",
                Some(serde_json::json!({
                    "node_id": cluster_membership.local_node_id(),
                })),
            );

            let elapsed_ms = boot_time.elapsed().as_millis() as u64;
            let mut ready_payload = serde_json::json!({
                "elapsed_ms": elapsed_ms,
                "processes": process_table.len(),
                "services": service_registry.len(),
            });

            if let Some(ref tm) = tree_manager {
                let root_hash = tm.stats().root_hash;
                ready_payload
                    .as_object_mut()
                    .unwrap()
                    .insert("tree_root_hash".to_string(), serde_json::json!(root_hash));
            }

            cm.append("kernel", "boot.ready", Some(ready_payload));

            // Emit boot manifest as a chain event (becomes an ExochainCheckpoint
            // segment in RVF persistence, capturing the complete boot state).
            let mut manifest = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "node_id": cluster_membership.local_node_id(),
                "process_count": process_table.len(),
                "service_count": service_registry.len(),
                "chain_sequence": cm.sequence(),
                "boot_elapsed_ms": boot_time.elapsed().as_millis() as u64,
            });
            if let Some(ref tm) = tree_manager {
                let stats = tm.stats();
                manifest.as_object_mut().unwrap().insert(
                    "tree_root_hash".to_string(),
                    serde_json::json!(stats.root_hash),
                );
                manifest.as_object_mut().unwrap().insert(
                    "tree_node_count".to_string(),
                    serde_json::json!(stats.node_count),
                );
            }
            cm.append("kernel", "boot.manifest", Some(manifest));
        }

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

        // 9. Create agent supervisor
        let supervisor = AgentSupervisor::new(
            process_table.clone(),
            ipc.clone(),
            AgentCapabilities::default(),
        );

        // 9b. Wire A2ARouter and cron into supervisor
        let supervisor = supervisor.with_a2a_router(a2a_router.clone(), cron_svc.clone());

        // 9c. Wire exochain managers into supervisor
        #[cfg(feature = "exochain")]
        let supervisor = supervisor.with_exochain(
            tree_manager.clone(),
            chain_manager.clone(),
        );

        // 10. Seed the event ring buffer with boot events
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
            a2a_router,
            cron_service: cron_svc,
            health,
            supervisor,
            boot_log,
            event_log,
            boot_time,
            cluster_membership,
            #[cfg(feature = "exochain")]
            chain_manager,
            #[cfg(feature = "exochain")]
            tree_manager,
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

        // Checkpoint tree+chain before shutting down
        #[cfg(feature = "exochain")]
        if let Some(ref tm) = self.tree_manager
            && let Some(ref cm) = self.chain_manager
        {
            let stats = tm.stats();
            cm.append(
                "kernel",
                "shutdown",
                Some(serde_json::json!({
                    "tree_root_hash": stats.root_hash,
                    "chain_seq": cm.sequence(),
                    "tree_nodes": stats.node_count,
                })),
            );
        }

        // Abort all running agent tasks
        self.supervisor.abort_all();

        // Cancel all processes
        for entry in self.process_table.list() {
            if entry.pid == 0 {
                continue; // Don't cancel the kernel process
            }
            entry.cancel_token.cancel();

            // Log agent stop in tree/chain
            #[cfg(feature = "exochain")]
            if let Some(ref tm) = self.tree_manager {
                let _ = tm.unregister_agent(&entry.agent_id, entry.pid, 0);
            }

            let _ = self
                .process_table
                .update_state(entry.pid, ProcessState::Exited(0));
        }

        // Persist chain to RVF checkpoint (primary), JSON as fallback
        #[cfg(feature = "exochain")]
        if let Some(ref cm) = self.chain_manager {
            let chain_config = self.config.chain.clone().unwrap_or_default();
            if let Some(ref ckpt_path) = chain_config.effective_checkpoint_path() {
            let json_path = std::path::PathBuf::from(ckpt_path);
            let rvf_path = json_path.with_extension("rvf");

            // Save RVF format (primary)
            match cm.save_to_rvf(&rvf_path) {
                Ok(()) => info!(path = %rvf_path.display(), "chain saved to RVF checkpoint"),
                Err(e) => {
                    error!(error = %e, "failed to save RVF checkpoint, falling back to JSON");
                    // Fallback: save JSON
                    match cm.save_to_file(&json_path) {
                        Ok(()) => info!(path = %json_path.display(), "chain saved to JSON checkpoint (fallback)"),
                        Err(e2) => error!(error = %e2, "failed to save JSON checkpoint fallback"),
                    }
                }
            }

            // Save tree checkpoint alongside chain
            if let Some(ref tm) = self.tree_manager {
                let tree_path = json_path.with_extension("tree.json");
                match tm.save_checkpoint(&tree_path) {
                    Ok(()) => {
                        info!(path = %tree_path.display(), "tree checkpoint saved");
                        cm.append(
                            "tree",
                            "tree.checkpoint",
                            Some(serde_json::json!({
                                "path": tree_path.display().to_string(),
                                "root_hash": tm.stats().root_hash,
                            })),
                        );
                    }
                    Err(e) => error!(error = %e, "failed to save tree checkpoint"),
                }
            }
            }
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

    /// Get the A2A router.
    pub fn a2a_router(&self) -> &Arc<A2ARouter> {
        &self.a2a_router
    }

    /// Get the cron service.
    pub fn cron_service(&self) -> &Arc<crate::cron::CronService> {
        &self.cron_service
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

    /// Get the cluster membership tracker.
    pub fn cluster_membership(&self) -> &Arc<ClusterMembership> {
        &self.cluster_membership
    }

    /// Get the local chain manager (when exochain feature is enabled).
    #[cfg(feature = "exochain")]
    pub fn chain_manager(&self) -> Option<&Arc<crate::chain::ChainManager>> {
        self.chain_manager.as_ref()
    }

    /// Get the tree manager (when exochain feature is enabled).
    #[cfg(feature = "exochain")]
    pub fn tree_manager(&self) -> Option<&Arc<crate::tree_manager::TreeManager>> {
        self.tree_manager.as_ref()
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
            cluster: None,
            chain: None,
            resource_tree: None,
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

        assert_eq!(kernel.services().len(), 1); // cron service registered at boot
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
