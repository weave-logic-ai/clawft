//! Agent supervisor for process lifecycle management.
//!
//! The [`AgentSupervisor`] manages the full lifecycle of kernel-managed
//! agents: spawn, stop, restart, inspect, and watch. It wraps the
//! existing `AgentLoop` spawn mechanism without replacing it, adding
//! capability enforcement, resource tracking, and process table integration.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use clawft_platform::Platform;

use crate::capability::AgentCapabilities;
use crate::error::{KernelError, KernelResult};
use crate::ipc::KernelIpc;
use crate::process::{Pid, ProcessEntry, ProcessState, ProcessTable, ResourceUsage};

/// Execution backend for spawning an agent process.
///
/// Determines how the agent's work is executed at runtime. Only `Native`
/// is implemented in K0-K2; other variants are defined to crystallize the
/// API surface (see Symposium decisions D2, D3, C1, C8) and will return
/// [`KernelError::BackendNotAvailable`] until their respective K-phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnBackend {
    /// Tokio task with agent_loop (K0-K2, default).
    Native,
    /// WASM sandbox via Wasmtime (K3).
    Wasm {
        /// Path to the compiled WASM module.
        module: PathBuf,
    },
    /// Docker/Podman container (K4).
    Container {
        /// Container image reference (e.g. "ghcr.io/org/agent:latest").
        image: String,
    },
    /// Trusted Execution Environment -- SGX, TrustZone, SEV (K6+).
    Tee {
        /// Enclave configuration.
        enclave: EnclaveConfig,
    },
    /// Delegate to a remote node in the cluster (K6).
    Remote {
        /// Cluster node identifier.
        node_id: String,
    },
}

/// Placeholder configuration for TEE enclaves (D14, C8).
///
/// Will be expanded with actual hardware parameters when TEE
/// runtime support is implemented.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnclaveConfig {
    /// Enclave type: "sgx", "trustzone", "sev".
    pub enclave_type: String,
}

/// Request to spawn a new supervised agent process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Unique identifier for the agent.
    pub agent_id: String,

    /// Capabilities to assign. If `None`, the supervisor's default
    /// capabilities are used.
    #[serde(default)]
    pub capabilities: Option<AgentCapabilities>,

    /// PID of the parent process (for tracking spawn lineage).
    #[serde(default)]
    pub parent_pid: Option<Pid>,

    /// Environment variables for the agent.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Execution backend. `None` defaults to `SpawnBackend::Native`.
    ///
    /// Non-Native backends return [`KernelError::BackendNotAvailable`]
    /// until their respective K-phase implements them.
    #[serde(default)]
    pub backend: Option<SpawnBackend>,
}

/// Result of a successful agent spawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResult {
    /// The PID assigned to the new process.
    pub pid: Pid,

    /// The agent identifier.
    pub agent_id: String,
}

/// Manages the lifecycle of kernel-managed agent processes.
///
/// The supervisor sits between the CLI/API surface and the core
/// `AgentLoop`, providing:
///
/// - **Spawn**: creates a process entry, assigns capabilities,
///   allocates a PID, and tracks the agent in the process table.
/// - **Stop**: signals cancellation (graceful) or immediate termination.
/// - **Restart**: stops then re-spawns with the same configuration.
/// - **Inspect**: returns full process entry with capabilities and
///   resource usage.
/// - **Watch**: returns a receiver for process state changes.
///
/// The supervisor does not own the actual `AgentLoop` execution; that
/// remains the responsibility of the caller (kernel boot or CLI).
/// Instead, the supervisor manages the process table entries and
/// provides the cancellation tokens that control agent lifecycle.
pub struct AgentSupervisor<P: Platform> {
    process_table: Arc<ProcessTable>,
    kernel_ipc: Arc<KernelIpc>,
    default_capabilities: AgentCapabilities,
    running_agents: Arc<DashMap<Pid, tokio::task::JoinHandle<()>>>,
    a2a_router: Option<Arc<crate::a2a::A2ARouter>>,
    cron_service: Option<Arc<crate::cron::CronService>>,
    #[cfg(feature = "exochain")]
    tree_manager: Option<Arc<crate::tree_manager::TreeManager>>,
    #[cfg(feature = "exochain")]
    chain_manager: Option<Arc<crate::chain::ChainManager>>,
    _platform: PhantomData<P>,
}

impl<P: Platform> AgentSupervisor<P> {
    /// Create a new agent supervisor.
    ///
    /// # Arguments
    ///
    /// * `process_table` - Shared process table (also held by Kernel)
    /// * `kernel_ipc` - IPC subsystem for sending lifecycle signals
    /// * `default_capabilities` - Capabilities assigned to agents that
    ///   don't specify their own
    pub fn new(
        process_table: Arc<ProcessTable>,
        kernel_ipc: Arc<KernelIpc>,
        default_capabilities: AgentCapabilities,
    ) -> Self {
        Self {
            process_table,
            kernel_ipc,
            default_capabilities,
            running_agents: Arc::new(DashMap::new()),
            a2a_router: None,
            cron_service: None,
            #[cfg(feature = "exochain")]
            tree_manager: None,
            #[cfg(feature = "exochain")]
            chain_manager: None,
            _platform: PhantomData,
        }
    }

    /// Configure A2A router and cron service.
    ///
    /// When set, `spawn_and_run` will create per-agent inboxes via the
    /// A2ARouter and pass the cron service handle to the agent work loop.
    pub fn with_a2a_router(
        mut self,
        a2a_router: Arc<crate::a2a::A2ARouter>,
        cron_service: Arc<crate::cron::CronService>,
    ) -> Self {
        self.a2a_router = Some(a2a_router);
        self.cron_service = Some(cron_service);
        self
    }

    /// Get the A2A router (if configured).
    pub fn a2a_router(&self) -> Option<&Arc<crate::a2a::A2ARouter>> {
        self.a2a_router.as_ref()
    }

    /// Get the cron service (if configured).
    pub fn cron_service(&self) -> Option<&Arc<crate::cron::CronService>> {
        self.cron_service.as_ref()
    }

    /// Configure exochain integration (tree + chain managers).
    ///
    /// When set, agent spawn/stop/restart events are recorded in
    /// the resource tree and hash chain.
    #[cfg(feature = "exochain")]
    pub fn with_exochain(
        mut self,
        tree_manager: Option<Arc<crate::tree_manager::TreeManager>>,
        chain_manager: Option<Arc<crate::chain::ChainManager>>,
    ) -> Self {
        self.tree_manager = tree_manager;
        self.chain_manager = chain_manager;
        self
    }

    /// Spawn a new supervised agent process.
    ///
    /// This creates a process table entry and returns the assigned PID.
    /// The actual agent execution (AgentLoop) must be started separately
    /// by the caller using the returned `SpawnResult` and the
    /// cancellation token from the process entry.
    ///
    /// # Errors
    ///
    /// Returns `KernelError::ProcessTableFull` if the process table
    /// has reached its maximum capacity.
    pub fn spawn(&self, request: SpawnRequest) -> KernelResult<SpawnResult> {
        // Check backend availability -- only Native is implemented.
        match &request.backend {
            None | Some(SpawnBackend::Native) => { /* supported */ }
            Some(SpawnBackend::Wasm { .. }) => {
                return Err(KernelError::BackendNotAvailable {
                    backend: "wasm".into(),
                    reason: "WASM sandbox requires K3 (Wasmtime integration)".into(),
                });
            }
            Some(SpawnBackend::Container { .. }) => {
                return Err(KernelError::BackendNotAvailable {
                    backend: "container".into(),
                    reason: "container runtime requires K4 (Docker/Podman integration)".into(),
                });
            }
            Some(SpawnBackend::Tee { .. }) => {
                return Err(KernelError::BackendNotAvailable {
                    backend: "tee".into(),
                    reason: "TEE runtime requires K6+ and hardware support".into(),
                });
            }
            Some(SpawnBackend::Remote { .. }) => {
                return Err(KernelError::BackendNotAvailable {
                    backend: "remote".into(),
                    reason: "remote delegation requires K6 (cluster networking)".into(),
                });
            }
        }

        let caps = request
            .capabilities
            .unwrap_or_else(|| self.default_capabilities.clone());

        info!(
            agent_id = %request.agent_id,
            parent_pid = ?request.parent_pid,
            "spawning supervised agent"
        );

        let entry = ProcessEntry {
            pid: 0, // Will be set by insert()
            agent_id: request.agent_id.clone(),
            state: ProcessState::Starting,
            capabilities: caps,
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: request.parent_pid,
        };

        let pid = self.process_table.insert(entry)?;

        debug!(pid, agent_id = %request.agent_id, "agent spawned");

        Ok(SpawnResult {
            pid,
            agent_id: request.agent_id,
        })
    }

    /// Spawn a supervised agent and run its work as a tokio task.
    ///
    /// Unlike `spawn`, this method also:
    /// 1. Transitions the process to `Running`
    /// 2. Registers the agent in the resource tree (if exochain enabled)
    /// 3. Spawns a tokio task to execute the provided work closure
    /// 4. On completion: transitions to `Exited`, unregisters from tree,
    ///    logs chain events, and cleans up the task handle
    ///
    /// The `work` closure receives the assigned PID and a
    /// [`CancellationToken`]; it should return an exit code (0 = success).
    ///
    /// # Errors
    ///
    /// Returns `KernelError::ProcessTableFull` if the process table
    /// has reached its maximum capacity.
    pub fn spawn_and_run<F, Fut>(
        &self,
        request: SpawnRequest,
        work: F,
    ) -> KernelResult<SpawnResult>
    where
        F: FnOnce(Pid, CancellationToken) -> Fut,
        Fut: std::future::Future<Output = i32> + Send + 'static,
    {
        // Capture parent_pid before spawn() consumes the request
        #[cfg(feature = "exochain")]
        let parent_pid = request.parent_pid;

        // 1. Create process entry via existing spawn()
        let result = self.spawn(request)?;
        let pid = result.pid;

        let entry = self
            .process_table
            .get(pid)
            .ok_or(KernelError::ProcessNotFound { pid })?;
        let cancel_token = entry.cancel_token.clone();

        // 2. Register in resource tree (exochain)
        #[cfg(feature = "exochain")]
        if let Some(ref tm) = self.tree_manager
            && let Err(e) = tm.register_agent(&result.agent_id, pid, &entry.capabilities)
        {
            warn!(error = %e, pid, "failed to register agent in resource tree");
        }

        // 3. Transition to Running
        let _ = self
            .process_table
            .update_state(pid, ProcessState::Running);

        // 3b. Log spawn chain event
        #[cfg(feature = "exochain")]
        if let Some(ref cm) = self.chain_manager {
            cm.append(
                "supervisor",
                "agent.spawn",
                Some(serde_json::json!({
                    "agent_id": result.agent_id,
                    "pid": pid,
                    "parent_pid": parent_pid,
                })),
            );
        }

        // 4. Spawn tokio task
        let process_table = Arc::clone(&self.process_table);
        let running_agents = Arc::clone(&self.running_agents);
        let agent_id = result.agent_id.clone();
        #[cfg(feature = "exochain")]
        let tree_manager = self.tree_manager.clone();
        #[cfg(feature = "exochain")]
        let chain_manager = self.chain_manager.clone();

        let future = work(pid, cancel_token);
        let handle = tokio::spawn(async move {
            let exit_code = future.await;

            // Transition to Exited
            let _ = process_table.update_state(pid, ProcessState::Exited(exit_code));

            // Blend scoring on agent exit — performance observation
            #[cfg(feature = "exochain")]
            if let Some(ref tm) = tree_manager {
                let agent_path = format!("/kernel/agents/{agent_id}");
                let rid = exo_resource_tree::ResourceId::new(&agent_path);

                // Build observation: successful exit boosts trust/reliability,
                // failure reduces them.
                let success = exit_code == 0;
                let observation = exo_resource_tree::NodeScoring {
                    trust: if success { 0.8 } else { 0.2 },
                    performance: if success { 0.7 } else { 0.3 },
                    difficulty: 0.5,
                    reward: if success { 0.6 } else { 0.1 },
                    reliability: if success { 0.9 } else { 0.1 },
                    velocity: 0.5,
                };
                // Blend with alpha=0.3 (30% observation, 70% prior)
                if let Err(e) = tm.blend_scoring(&rid, &observation, 0.3) {
                    debug!(error = %e, pid, "scoring blend skipped (node may be unregistered)");
                }
            }

            // Unregister from tree
            #[cfg(feature = "exochain")]
            if let Some(ref tm) = tree_manager
                && let Err(e) = tm.unregister_agent(&agent_id, pid, exit_code)
            {
                tracing::warn!(error = %e, pid, "failed to unregister agent from tree");
            }

            // Log exit chain event
            #[cfg(feature = "exochain")]
            if let Some(ref cm) = chain_manager {
                cm.append(
                    "supervisor",
                    "agent.exit",
                    Some(serde_json::json!({
                        "agent_id": agent_id,
                        "pid": pid,
                        "exit_code": exit_code,
                    })),
                );
            }

            // Remove from running agents map
            running_agents.remove(&pid);

            info!(pid, exit_code, agent_id = %agent_id, "agent task completed");
        });

        self.running_agents.insert(pid, handle);

        info!(pid, agent_id = %result.agent_id, "agent spawned and running");

        Ok(result)
    }

    /// Stop a supervised agent process.
    ///
    /// If `graceful` is true, the process is moved to `Stopping` state
    /// and its cancellation token is cancelled, allowing the agent to
    /// finish its current work. If `graceful` is false, the process is
    /// immediately moved to `Exited(-1)`.
    ///
    /// Stopping an already-exited process is idempotent and returns `Ok`.
    ///
    /// # Errors
    ///
    /// Returns `KernelError::ProcessNotFound` if the PID is not in
    /// the process table.
    pub fn stop(&self, pid: Pid, graceful: bool) -> KernelResult<()> {
        let entry = self
            .process_table
            .get(pid)
            .ok_or(KernelError::ProcessNotFound { pid })?;

        // Already exited -- idempotent
        if matches!(entry.state, ProcessState::Exited(_)) {
            warn!(pid, "stop called on already-exited process");
            return Ok(());
        }

        if graceful {
            info!(pid, "gracefully stopping agent");
            // Transition to Stopping, then cancel the token.
            // The spawned task (if any) will detect cancellation,
            // exit, and handle tree/chain cleanup.
            let _ = self.process_table.update_state(pid, ProcessState::Stopping);
            entry.cancel_token.cancel();
        } else {
            info!(pid, "force stopping agent");
            entry.cancel_token.cancel();
            let _ = self
                .process_table
                .update_state(pid, ProcessState::Exited(-1));

            // Abort the running task handle (cleanup won't run)
            if let Some((_, handle)) = self.running_agents.remove(&pid) {
                handle.abort();
            }

            // Since the spawned task was aborted, do tree/chain
            // cleanup directly here.
            #[cfg(feature = "exochain")]
            {
                if let Some(ref tm) = self.tree_manager {
                    let _ = tm.unregister_agent(&entry.agent_id, pid, -1);
                }
                if let Some(ref cm) = self.chain_manager {
                    cm.append(
                        "supervisor",
                        "agent.force_stop",
                        Some(serde_json::json!({
                            "agent_id": entry.agent_id,
                            "pid": pid,
                        })),
                    );
                }
            }
        }

        Ok(())
    }

    /// Restart a supervised agent process.
    ///
    /// Stops the existing process (gracefully), then spawns a new one
    /// with the same agent_id and capabilities. The new process gets
    /// a fresh PID; the old entry remains in the table with
    /// `Exited(0)` state.
    ///
    /// The `parent_pid` of the new process is set to the restarted
    /// PID, creating a restart lineage.
    ///
    /// # Errors
    ///
    /// Returns `KernelError::ProcessNotFound` if the PID is not in
    /// the process table.
    pub fn restart(&self, pid: Pid) -> KernelResult<SpawnResult> {
        let old_entry = self
            .process_table
            .get(pid)
            .ok_or(KernelError::ProcessNotFound { pid })?;

        info!(pid, agent_id = %old_entry.agent_id, "restarting agent");

        // Stop the old process
        self.stop(pid, true)?;

        // Mark as cleanly exited if not already
        if !matches!(old_entry.state, ProcessState::Exited(_)) {
            let _ = self
                .process_table
                .update_state(pid, ProcessState::Exited(0));
        }

        // Spawn replacement with same config
        let request = SpawnRequest {
            agent_id: old_entry.agent_id.clone(),
            capabilities: Some(old_entry.capabilities.clone()),
            parent_pid: Some(pid),
            env: HashMap::new(),
            backend: None, // restarts always use Native
        };

        let result = self.spawn(request)?;

        // Log restart chain event linking old PID to new PID
        #[cfg(feature = "exochain")]
        if let Some(ref cm) = self.chain_manager {
            cm.append(
                "supervisor",
                "agent.restart",
                Some(serde_json::json!({
                    "agent_id": result.agent_id,
                    "old_pid": pid,
                    "new_pid": result.pid,
                })),
            );
        }

        Ok(result)
    }

    /// Inspect a supervised agent process.
    ///
    /// Returns a clone of the full [`ProcessEntry`] including
    /// capabilities and resource usage.
    ///
    /// # Errors
    ///
    /// Returns `KernelError::ProcessNotFound` if the PID is not in
    /// the process table.
    pub fn inspect(&self, pid: Pid) -> KernelResult<ProcessEntry> {
        self.process_table
            .get(pid)
            .ok_or(KernelError::ProcessNotFound { pid })
    }

    /// List processes filtered by state.
    pub fn list_by_state(&self, state: ProcessState) -> Vec<ProcessEntry> {
        self.process_table
            .list()
            .into_iter()
            .filter(|e| e.state == state)
            .collect()
    }

    /// List all running agent processes (excludes kernel PID 0).
    pub fn list_agents(&self) -> Vec<ProcessEntry> {
        self.process_table
            .list()
            .into_iter()
            .filter(|e| e.pid != 0)
            .collect()
    }

    /// Get a reference to the shared process table.
    pub fn process_table(&self) -> &Arc<ProcessTable> {
        &self.process_table
    }

    /// Get a reference to the IPC subsystem.
    pub fn ipc(&self) -> &Arc<KernelIpc> {
        &self.kernel_ipc
    }

    /// Get the default capabilities assigned to new agents.
    pub fn default_capabilities(&self) -> &AgentCapabilities {
        &self.default_capabilities
    }

    /// Count running processes (excluding kernel PID 0).
    pub fn running_count(&self) -> usize {
        self.process_table
            .list()
            .iter()
            .filter(|e| e.pid != 0 && e.state == ProcessState::Running)
            .count()
    }

    /// Get the number of actively tracked running agent tasks.
    pub fn running_task_count(&self) -> usize {
        self.running_agents.len()
    }

    /// Abort all running agent tasks (used during forced shutdown).
    pub fn abort_all(&self) {
        for entry in self.running_agents.iter() {
            entry.value().abort();
        }
        self.running_agents.clear();
    }

    /// Sweep finished agent handles that were not cleaned up normally.
    ///
    /// Iterates `running_agents`, checks `is_finished()` on each
    /// `JoinHandle`, and for any that are finished:
    /// 1. Removes the handle from the map
    /// 2. If the process table still shows `Running`, transitions to
    ///    `Exited(-2)` (watchdog reap) or `Exited(-3)` (panic reap)
    /// 3. Logs a chain event (when exochain is enabled)
    ///
    /// Returns a list of (pid, exit_code) for all reaped processes.
    pub async fn watchdog_sweep(&self) -> Vec<(Pid, i32)> {
        let mut reaped = Vec::new();

        // Collect finished PIDs first to avoid holding DashMap refs across await
        let finished_pids: Vec<Pid> = self
            .running_agents
            .iter()
            .filter(|entry| entry.value().is_finished())
            .map(|entry| *entry.key())
            .collect();

        for pid in finished_pids {
            if let Some((_, handle)) = self.running_agents.remove(&pid) {
                // Check if the task panicked
                let exit_code = match handle.await {
                    Ok(()) => -2,  // Watchdog reap (task finished but cleanup didn't remove from map)
                    Err(e) if e.is_panic() => -3,  // Panic reap
                    Err(_) => -2,  // Cancelled or other
                };

                // Only transition if process table still shows Running
                if let Some(entry) = self.process_table.get(pid)
                    && entry.state == ProcessState::Running
                {
                    let _ = self
                        .process_table
                        .update_state(pid, ProcessState::Exited(exit_code));

                    #[cfg(feature = "exochain")]
                    if let Some(ref cm) = self.chain_manager {
                        cm.append(
                            "watchdog",
                            "agent.watchdog_reap",
                            Some(serde_json::json!({
                                "pid": pid,
                                "exit_code": exit_code,
                                "agent_id": entry.agent_id,
                            })),
                        );
                    }

                    reaped.push((pid, exit_code));
                    info!(pid, exit_code, agent_id = %entry.agent_id, "watchdog reaped stale agent");
                }
            }
        }

        reaped
    }

    /// Gracefully shut down all running agents with a timeout.
    ///
    /// 1. Cancels all agent cancellation tokens via the process table
    /// 2. Drains all JoinHandles from `running_agents`
    /// 3. Waits for all tasks to complete, with a timeout
    /// 4. On timeout, aborts any remaining tasks
    ///
    /// Returns a list of (pid, exit_code) for all agents.
    pub async fn shutdown_all(&self, timeout: std::time::Duration) -> Vec<(Pid, i32)> {
        // 1. Cancel all agent tokens
        for entry in self.process_table.list() {
            if entry.pid == 0 {
                continue; // Don't cancel the kernel process
            }
            entry.cancel_token.cancel();
        }

        // 2. Drain all handles from running_agents
        let handles: Vec<(Pid, tokio::task::JoinHandle<()>)> = {
            let pids: Vec<Pid> = self.running_agents.iter().map(|e| *e.key()).collect();
            let mut collected = Vec::with_capacity(pids.len());
            for pid in pids {
                if let Some((pid, handle)) = self.running_agents.remove(&pid) {
                    collected.push((pid, handle));
                }
            }
            collected
        };

        if handles.is_empty() {
            return Vec::new();
        }

        let process_table = &self.process_table;

        // 3. Wait for all handles concurrently with timeout.
        //    Use futures::future::join_all-style: wrap each handle in a
        //    tokio::time::timeout so no single stuck handle blocks the rest.
        let mut results = Vec::with_capacity(handles.len());

        match tokio::time::timeout(
            timeout,
            futures::future::join_all(
                handles
                    .into_iter()
                    .map(|(pid, handle)| async move { (pid, handle.await) }),
            ),
        )
        .await
        {
            Ok(join_results) => {
                // All handles completed within timeout
                for (pid, join_result) in join_results {
                    let exit_code = match join_result {
                        Ok(()) => process_table
                            .get(pid)
                            .and_then(|e| match e.state {
                                ProcessState::Exited(code) => Some(code),
                                _ => None,
                            })
                            .unwrap_or(0),
                        Err(e) if e.is_panic() => -3,
                        Err(_) => -1,
                    };
                    results.push((pid, exit_code));
                }
            }
            Err(_elapsed) => {
                info!("shutdown timeout reached, aborting remaining agents");
                // Timeout expired. Any handles still alive need to be aborted.
                // Since we moved the handles into join_all, they're already being
                // awaited. The timeout drop aborts them. Record all remaining
                // agents from the running_agents map.
                let remaining: Vec<Pid> =
                    self.running_agents.iter().map(|e| *e.key()).collect();
                for pid in remaining {
                    if let Some((pid, handle)) = self.running_agents.remove(&pid) {
                        handle.abort();
                        let _ = process_table.update_state(pid, ProcessState::Exited(-1));
                        results.push((pid, -1));
                    }
                }

                // If no remaining handles were in the map (handles were consumed by
                // join_all), check the process table for any non-exited agents.
                if results.is_empty() {
                    for entry in process_table.list() {
                        if entry.pid != 0 && !matches!(entry.state, ProcessState::Exited(_)) {
                            let _ = process_table.update_state(entry.pid, ProcessState::Exited(-1));
                            results.push((entry.pid, -1));
                        }
                    }
                }
            }
        }

        results
    }

    /// Get the tree manager (when exochain feature is enabled).
    #[cfg(feature = "exochain")]
    pub fn tree_manager(&self) -> Option<&Arc<crate::tree_manager::TreeManager>> {
        self.tree_manager.as_ref()
    }

    /// Get the chain manager (when exochain feature is enabled).
    #[cfg(feature = "exochain")]
    pub fn chain_manager(&self) -> Option<&Arc<crate::chain::ChainManager>> {
        self.chain_manager.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_core::bus::MessageBus;

    fn make_supervisor() -> AgentSupervisor<clawft_platform::NativePlatform> {
        let process_table = Arc::new(ProcessTable::new(16));
        let bus = Arc::new(MessageBus::new());
        let ipc = Arc::new(KernelIpc::new(bus));
        AgentSupervisor::new(process_table, ipc, AgentCapabilities::default())
    }

    fn simple_request(agent_id: &str) -> SpawnRequest {
        SpawnRequest {
            agent_id: agent_id.to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: None,
        }
    }

    #[test]
    fn spawn_creates_process_entry() {
        let sup = make_supervisor();
        let result = sup.spawn(simple_request("agent-1")).unwrap();

        assert!(result.pid > 0);
        assert_eq!(result.agent_id, "agent-1");

        let entry = sup.inspect(result.pid).unwrap();
        assert_eq!(entry.agent_id, "agent-1");
        assert_eq!(entry.state, ProcessState::Starting);
    }

    #[test]
    fn spawn_uses_default_capabilities() {
        let sup = make_supervisor();
        let result = sup.spawn(simple_request("agent-1")).unwrap();

        let entry = sup.inspect(result.pid).unwrap();
        assert!(entry.capabilities.can_spawn);
        assert!(entry.capabilities.can_ipc);
        assert!(entry.capabilities.can_exec_tools);
    }

    #[test]
    fn spawn_uses_custom_capabilities() {
        let sup = make_supervisor();
        let caps = AgentCapabilities {
            can_spawn: false,
            can_ipc: false,
            can_exec_tools: true,
            can_network: true,
            ..Default::default()
        };

        let request = SpawnRequest {
            agent_id: "restricted".to_owned(),
            capabilities: Some(caps.clone()),
            parent_pid: None,
            env: HashMap::new(),
            backend: None,
        };

        let result = sup.spawn(request).unwrap();
        let entry = sup.inspect(result.pid).unwrap();
        assert!(!entry.capabilities.can_spawn);
        assert!(!entry.capabilities.can_ipc);
        assert!(entry.capabilities.can_network);
    }

    #[test]
    fn spawn_with_parent_pid() {
        let sup = make_supervisor();
        let parent = sup.spawn(simple_request("parent")).unwrap();

        let request = SpawnRequest {
            agent_id: "child".to_owned(),
            capabilities: None,
            parent_pid: Some(parent.pid),
            env: HashMap::new(),
            backend: None,
        };

        let result = sup.spawn(request).unwrap();
        let entry = sup.inspect(result.pid).unwrap();
        assert_eq!(entry.parent_pid, Some(parent.pid));
    }

    #[test]
    fn spawn_fails_when_table_full() {
        let process_table = Arc::new(ProcessTable::new(2));
        let bus = Arc::new(MessageBus::new());
        let ipc = Arc::new(KernelIpc::new(bus));
        let sup: AgentSupervisor<clawft_platform::NativePlatform> =
            AgentSupervisor::new(process_table, ipc, AgentCapabilities::default());

        sup.spawn(simple_request("a1")).unwrap();
        sup.spawn(simple_request("a2")).unwrap();
        let result = sup.spawn(simple_request("a3"));
        assert!(result.is_err());
    }

    #[test]
    fn stop_graceful() {
        let sup = make_supervisor();
        let result = sup.spawn(simple_request("agent-1")).unwrap();

        // Move to Running first (Starting -> Running -> Stopping)
        sup.process_table()
            .update_state(result.pid, ProcessState::Running)
            .unwrap();

        sup.stop(result.pid, true).unwrap();

        let entry = sup.inspect(result.pid).unwrap();
        assert_eq!(entry.state, ProcessState::Stopping);
        assert!(entry.cancel_token.is_cancelled());
    }

    #[test]
    fn stop_force() {
        let sup = make_supervisor();
        let result = sup.spawn(simple_request("agent-1")).unwrap();

        // Move to Running first
        sup.process_table()
            .update_state(result.pid, ProcessState::Running)
            .unwrap();

        sup.stop(result.pid, false).unwrap();

        let entry = sup.inspect(result.pid).unwrap();
        assert!(entry.cancel_token.is_cancelled());
    }

    #[test]
    fn stop_already_exited_is_idempotent() {
        let sup = make_supervisor();
        let result = sup.spawn(simple_request("agent-1")).unwrap();

        // Move to exited
        sup.process_table()
            .update_state(result.pid, ProcessState::Exited(0))
            .unwrap();

        // Should succeed without error
        sup.stop(result.pid, true).unwrap();
    }

    #[test]
    fn stop_nonexistent_pid_fails() {
        let sup = make_supervisor();
        let result = sup.stop(999, true);
        assert!(result.is_err());
    }

    #[test]
    fn restart_creates_new_process() {
        let sup = make_supervisor();
        let original = sup.spawn(simple_request("agent-1")).unwrap();

        // Move to Running so it can be stopped
        sup.process_table()
            .update_state(original.pid, ProcessState::Running)
            .unwrap();

        let restarted = sup.restart(original.pid).unwrap();

        // New PID, same agent_id
        assert_ne!(restarted.pid, original.pid);
        assert_eq!(restarted.agent_id, "agent-1");

        // New process has parent_pid pointing to old PID
        let new_entry = sup.inspect(restarted.pid).unwrap();
        assert_eq!(new_entry.parent_pid, Some(original.pid));
    }

    #[test]
    fn restart_preserves_capabilities() {
        let sup = make_supervisor();
        let caps = AgentCapabilities {
            can_spawn: false,
            can_network: true,
            ..Default::default()
        };

        let request = SpawnRequest {
            agent_id: "restricted".to_owned(),
            capabilities: Some(caps),
            parent_pid: None,
            env: HashMap::new(),
            backend: None,
        };

        let original = sup.spawn(request).unwrap();
        sup.process_table()
            .update_state(original.pid, ProcessState::Running)
            .unwrap();

        let restarted = sup.restart(original.pid).unwrap();
        let entry = sup.inspect(restarted.pid).unwrap();
        assert!(!entry.capabilities.can_spawn);
        assert!(entry.capabilities.can_network);
    }

    #[test]
    fn list_by_state() {
        let sup = make_supervisor();
        let r1 = sup.spawn(simple_request("a1")).unwrap();
        let r2 = sup.spawn(simple_request("a2")).unwrap();
        sup.spawn(simple_request("a3")).unwrap();

        // Move first two to Running
        sup.process_table()
            .update_state(r1.pid, ProcessState::Running)
            .unwrap();
        sup.process_table()
            .update_state(r2.pid, ProcessState::Running)
            .unwrap();

        let running = sup.list_by_state(ProcessState::Running);
        assert_eq!(running.len(), 2);

        let starting = sup.list_by_state(ProcessState::Starting);
        assert_eq!(starting.len(), 1);
    }

    #[test]
    fn list_agents_excludes_kernel() {
        let sup = make_supervisor();

        // Insert kernel PID 0
        let kernel_entry = ProcessEntry {
            pid: 0,
            agent_id: "kernel".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        sup.process_table().insert_with_pid(kernel_entry).unwrap();

        // Spawn an agent
        sup.spawn(simple_request("agent-1")).unwrap();

        let agents = sup.list_agents();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "agent-1");
    }

    #[test]
    fn running_count() {
        let sup = make_supervisor();
        let r1 = sup.spawn(simple_request("a1")).unwrap();
        let r2 = sup.spawn(simple_request("a2")).unwrap();
        sup.spawn(simple_request("a3")).unwrap();

        assert_eq!(sup.running_count(), 0); // All Starting

        sup.process_table()
            .update_state(r1.pid, ProcessState::Running)
            .unwrap();
        assert_eq!(sup.running_count(), 1);

        sup.process_table()
            .update_state(r2.pid, ProcessState::Running)
            .unwrap();
        assert_eq!(sup.running_count(), 2);
    }

    #[test]
    fn default_capabilities_accessor() {
        let sup = make_supervisor();
        let caps = sup.default_capabilities();
        assert!(caps.can_spawn);
        assert!(caps.can_ipc);
        assert!(caps.can_exec_tools);
    }

    #[test]
    fn spawn_request_serde_roundtrip() {
        let request = SpawnRequest {
            agent_id: "test".to_owned(),
            capabilities: Some(AgentCapabilities {
                can_spawn: false,
                ..Default::default()
            }),
            parent_pid: Some(5),
            env: HashMap::from([("KEY".into(), "VALUE".into())]),
            backend: Some(SpawnBackend::Native),
        };

        let json = serde_json::to_string(&request).unwrap();
        let restored: SpawnRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agent_id, "test");
        assert_eq!(restored.parent_pid, Some(5));
        assert!(!restored.capabilities.unwrap().can_spawn);
    }

    #[test]
    fn spawn_result_serde_roundtrip() {
        let result = SpawnResult {
            pid: 42,
            agent_id: "agent-42".to_owned(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let restored: SpawnResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.pid, 42);
        assert_eq!(restored.agent_id, "agent-42");
    }

    #[tokio::test]
    async fn spawn_and_run_executes_work() {
        let sup = make_supervisor();

        let result = sup
            .spawn_and_run(simple_request("runner-1"), |_pid, _cancel| async { 0 })
            .unwrap();

        assert!(result.pid > 0);
        assert_eq!(result.agent_id, "runner-1");

        // Process should be Running immediately after spawn_and_run
        let entry = sup.inspect(result.pid).unwrap();
        assert_eq!(entry.state, ProcessState::Running);

        // Wait for the task to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Process should be Exited after work completes
        let entry = sup.inspect(result.pid).unwrap();
        assert!(matches!(entry.state, ProcessState::Exited(0)));

        // Running task should be cleaned up
        assert_eq!(sup.running_task_count(), 0);
    }

    #[tokio::test]
    async fn spawn_and_run_respects_cancellation() {
        let sup = make_supervisor();

        let result = sup
            .spawn_and_run(simple_request("cancellable"), |_pid, cancel| async move {
                cancel.cancelled().await;
                42
            })
            .unwrap();

        assert_eq!(sup.running_task_count(), 1);

        // Stop the agent
        sup.stop(result.pid, true).unwrap();

        // Wait for the task to detect cancellation and exit
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let entry = sup.inspect(result.pid).unwrap();
        assert!(matches!(entry.state, ProcessState::Exited(42)));
        assert_eq!(sup.running_task_count(), 0);
    }

    #[tokio::test]
    async fn spawn_and_run_force_stop_aborts() {
        let sup = make_supervisor();

        let result = sup
            .spawn_and_run(simple_request("force-me"), |_pid, cancel| async move {
                cancel.cancelled().await;
                0
            })
            .unwrap();

        // Force stop should abort the task immediately
        sup.stop(result.pid, false).unwrap();

        let entry = sup.inspect(result.pid).unwrap();
        assert!(matches!(entry.state, ProcessState::Exited(-1)));
        assert_eq!(sup.running_task_count(), 0);
    }

    #[tokio::test]
    async fn abort_all_clears_running_agents() {
        let sup = make_supervisor();

        sup.spawn_and_run(simple_request("a1"), |_pid, cancel| async move {
            cancel.cancelled().await;
            0
        })
        .unwrap();
        sup.spawn_and_run(simple_request("a2"), |_pid, cancel| async move {
            cancel.cancelled().await;
            0
        })
        .unwrap();

        assert_eq!(sup.running_task_count(), 2);

        sup.abort_all();

        assert_eq!(sup.running_task_count(), 0);
    }

    #[tokio::test]
    async fn watchdog_sweep_reaps_finished_task() {
        let sup = make_supervisor();

        // Spawn a task that completes instantly
        let result = sup
            .spawn_and_run(simple_request("instant"), |_pid, _cancel| async { 0 })
            .unwrap();

        // Give the task time to complete and clean up normally
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // The task should have cleaned itself up. If it did, sweep returns empty.
        // If by race condition it didn't, sweep should reap it.
        let reaped = sup.watchdog_sweep().await;

        // Either the task cleaned up on its own (reaped is empty and state is Exited)
        // or the watchdog reaped it.
        let entry = sup.inspect(result.pid).unwrap();
        assert!(
            matches!(entry.state, ProcessState::Exited(_)),
            "process should be Exited after sweep, got {:?}",
            entry.state
        );

        // Running task count should be 0 either way
        assert_eq!(sup.running_task_count(), 0);

        // If reaped, verify exit code
        for (pid, code) in &reaped {
            assert_eq!(*pid, result.pid);
            assert!(*code == -2 || *code == -3);
        }
    }

    #[tokio::test]
    async fn shutdown_all_graceful() {
        let sup = make_supervisor();

        sup.spawn_and_run(simple_request("g1"), |_pid, cancel| async move {
            cancel.cancelled().await;
            0
        })
        .unwrap();
        sup.spawn_and_run(simple_request("g2"), |_pid, cancel| async move {
            cancel.cancelled().await;
            42
        })
        .unwrap();

        assert_eq!(sup.running_task_count(), 2);

        let results = sup
            .shutdown_all(std::time::Duration::from_secs(5))
            .await;

        assert_eq!(results.len(), 2);
        assert_eq!(sup.running_task_count(), 0);
    }

    #[tokio::test]
    async fn shutdown_all_timeout_aborts() {
        let sup = make_supervisor();

        // Spawn a task that ignores cancellation (just sleeps forever)
        sup.spawn_and_run(simple_request("stubborn"), |_pid, _cancel| async move {
            // Ignore cancellation — sleep for a very long time
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            0
        })
        .unwrap();

        assert_eq!(sup.running_task_count(), 1);

        // shutdown_all with a very short timeout
        let results = sup
            .shutdown_all(std::time::Duration::from_millis(100))
            .await;

        // Should have at least 1 result (might be aborted)
        assert!(!results.is_empty());
        assert_eq!(sup.running_task_count(), 0);
    }

    #[cfg(feature = "exochain")]
    #[tokio::test]
    async fn chain_logs_agent_spawn() {
        let process_table = Arc::new(ProcessTable::new(16));
        let bus = Arc::new(MessageBus::new());
        let ipc = Arc::new(KernelIpc::new(bus));
        let cm = Arc::new(crate::chain::ChainManager::new(0, 1000));

        let sup: AgentSupervisor<clawft_platform::NativePlatform> =
            AgentSupervisor::new(process_table, ipc, AgentCapabilities::default())
                .with_exochain(None, Some(cm.clone()));

        let request = SpawnRequest {
            agent_id: "chain-agent".to_owned(),
            capabilities: None,
            parent_pid: Some(99),
            env: HashMap::new(),
            backend: None,
        };

        let result = sup
            .spawn_and_run(request, |_pid, _cancel| async { 0 })
            .unwrap();

        // Wait for the task to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify agent.spawn event on chain
        let events = cm.tail(10);
        let spawn_evt = events.iter().find(|e| e.kind == "agent.spawn");

        assert!(spawn_evt.is_some(), "expected agent.spawn event on chain");

        let payload = spawn_evt.unwrap().payload.as_ref().unwrap();
        assert_eq!(payload["agent_id"], "chain-agent");
        assert_eq!(payload["pid"], result.pid);
        assert_eq!(payload["parent_pid"], 99);

        // Should also have agent.exit from task completion
        let exit_evt = events.iter().find(|e| e.kind == "agent.exit");
        assert!(exit_evt.is_some(), "expected agent.exit event on chain");
    }

    // ── SpawnBackend tests (K2.1 T1: C1 + C8) ──────────────────

    #[test]
    fn spawn_native_explicit() {
        let sup = make_supervisor();
        let request = SpawnRequest {
            agent_id: "native-agent".to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: Some(SpawnBackend::Native),
        };
        let result = sup.spawn(request).unwrap();
        assert!(result.pid > 0);
        assert_eq!(result.agent_id, "native-agent");
    }

    #[test]
    fn spawn_backend_none_defaults_to_native() {
        let sup = make_supervisor();
        let request = SpawnRequest {
            agent_id: "default-agent".to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: None,
        };
        let result = sup.spawn(request).unwrap();
        assert!(result.pid > 0);
        assert_eq!(result.agent_id, "default-agent");
    }

    #[test]
    fn spawn_wasm_returns_not_available() {
        let sup = make_supervisor();
        let request = SpawnRequest {
            agent_id: "wasm-agent".to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: Some(SpawnBackend::Wasm {
                module: PathBuf::from("/tmp/agent.wasm"),
            }),
        };
        let result = sup.spawn(request);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("wasm"), "error should mention wasm: {msg}");
        assert!(msg.contains("not available"), "error should say not available: {msg}");
    }

    #[test]
    fn spawn_container_returns_not_available() {
        let sup = make_supervisor();
        let request = SpawnRequest {
            agent_id: "container-agent".to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: Some(SpawnBackend::Container {
                image: "ghcr.io/test/agent:latest".into(),
            }),
        };
        let result = sup.spawn(request);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("container"), "error should mention container: {msg}");
    }

    #[test]
    fn spawn_tee_returns_not_available() {
        let sup = make_supervisor();
        let request = SpawnRequest {
            agent_id: "tee-agent".to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: Some(SpawnBackend::Tee {
                enclave: EnclaveConfig {
                    enclave_type: "sgx".into(),
                },
            }),
        };
        let result = sup.spawn(request);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("tee"), "error should mention tee: {msg}");
    }

    #[test]
    fn spawn_remote_returns_not_available() {
        let sup = make_supervisor();
        let request = SpawnRequest {
            agent_id: "remote-agent".to_owned(),
            capabilities: None,
            parent_pid: None,
            env: HashMap::new(),
            backend: Some(SpawnBackend::Remote {
                node_id: "node-42".into(),
            }),
        };
        let result = sup.spawn(request);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("remote"), "error should mention remote: {msg}");
    }
}
