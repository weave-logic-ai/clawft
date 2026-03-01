//! Agent-first architecture: roles, agency, and agent manifests.
//!
//! In WeftOS, agents ARE the OS. Every feature is delivered by an
//! agent. The kernel boots a root agent which spawns service agents.
//! Each agent has:
//!
//! - A **role** (root, supervisor, service, worker, user)
//! - An **agency** (ability to spawn child agents)
//! - A **manifest** (`.agent.toml`) describing its capabilities
//!
//! Agency is hierarchical: the root agent has unlimited agency,
//! supervisors can spawn service/worker agents, services can spawn
//! workers, and workers have no agency (leaf agents).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::capability::AgentCapabilities;
use crate::process::Pid;

/// Agent role in the OS hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Superuser agent (user 1). Unlimited capabilities and agency.
    Root,
    /// Manages lifecycle of other agents. Can spawn/stop/restart.
    Supervisor,
    /// Provides a capability to other agents (memory, cron, API).
    Service,
    /// Performs specific tasks, spawned by others. No agency.
    Worker,
    /// Represents a human or external system.
    User,
    /// Custom role with a label.
    Custom(String),
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Root => write!(f, "root"),
            AgentRole::Supervisor => write!(f, "supervisor"),
            AgentRole::Service => write!(f, "service"),
            AgentRole::Worker => write!(f, "worker"),
            AgentRole::User => write!(f, "user"),
            AgentRole::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

/// Agency: the ability to spawn child agents.
///
/// Hierarchical: root has unlimited agency, supervisors can spawn
/// service/worker agents, services can spawn workers only, and
/// workers have no agency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agency {
    /// Maximum number of child agents this agent can spawn.
    /// None = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_children: Option<usize>,

    /// Which roles this agent is allowed to spawn.
    #[serde(default)]
    pub allowed_roles: Vec<AgentRole>,

    /// Capability ceiling: spawned agents cannot exceed these capabilities.
    #[serde(default)]
    pub capability_ceiling: AgentCapabilities,

    /// Currently spawned child PIDs.
    #[serde(default)]
    pub children: Vec<Pid>,
}

impl Agency {
    /// Root agent: unlimited agency.
    pub fn root() -> Self {
        Self {
            max_children: None,
            allowed_roles: vec![
                AgentRole::Supervisor,
                AgentRole::Service,
                AgentRole::Worker,
                AgentRole::User,
            ],
            capability_ceiling: AgentCapabilities::root(),
            children: Vec::new(),
        }
    }

    /// Supervisor: can spawn services and workers.
    pub fn supervisor(max_children: usize) -> Self {
        Self {
            max_children: Some(max_children),
            allowed_roles: vec![AgentRole::Service, AgentRole::Worker],
            capability_ceiling: AgentCapabilities::default(),
            children: Vec::new(),
        }
    }

    /// Service: can spawn workers only.
    pub fn service(max_workers: usize) -> Self {
        Self {
            max_children: Some(max_workers),
            allowed_roles: vec![AgentRole::Worker],
            capability_ceiling: AgentCapabilities::default(),
            children: Vec::new(),
        }
    }

    /// Worker: no agency (leaf agent).
    pub fn none() -> Self {
        Self {
            max_children: Some(0),
            allowed_roles: Vec::new(),
            capability_ceiling: AgentCapabilities::default(),
            children: Vec::new(),
        }
    }

    /// Check if this agency allows spawning a child with the given role.
    pub fn can_spawn(&self, role: &AgentRole) -> bool {
        if let Some(max) = self.max_children
            && self.children.len() >= max
        {
            return false;
        }
        self.allowed_roles.contains(role)
    }

    /// Record a spawned child.
    pub fn add_child(&mut self, pid: Pid) {
        self.children.push(pid);
    }

    /// Remove a terminated child.
    pub fn remove_child(&mut self, pid: Pid) {
        self.children.retain(|&p| p != pid);
    }

    /// How many children remain before hitting the limit.
    pub fn remaining_capacity(&self) -> Option<usize> {
        self.max_children
            .map(|max| max.saturating_sub(self.children.len()))
    }
}

impl Default for Agency {
    fn default() -> Self {
        Self::none()
    }
}

/// Agent manifest, parsed from `.agent.toml` files.
///
/// Describes what an agent does, what capabilities it needs,
/// and how to communicate with it. Analogous to systemd unit
/// files but for AI agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    /// Agent name (unique within the OS).
    pub name: String,

    /// Semantic version.
    #[serde(default = "default_version")]
    pub version: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Agent role.
    pub role: AgentRole,

    /// Capabilities this agent needs.
    #[serde(default)]
    pub capabilities: AgentCapabilities,

    /// Agency configuration (spawn permissions).
    #[serde(default)]
    pub agency: Agency,

    /// Tools this agent is allowed to use.
    #[serde(default)]
    pub tools: Vec<String>,

    /// IPC topics this agent publishes to.
    #[serde(default)]
    pub topics_publish: Vec<String>,

    /// IPC topics this agent subscribes to.
    #[serde(default)]
    pub topics_subscribe: Vec<String>,

    /// Resource limits.
    #[serde(default)]
    pub resources: AgentResources,

    /// Communication interface.
    #[serde(default)]
    pub interface: AgentInterface,

    /// Health check configuration.
    #[serde(default)]
    pub health: AgentHealth,

    /// Dependencies on other agents.
    #[serde(default)]
    pub dependencies: AgentDependencies,

    /// Filesystem paths this agent can access.
    #[serde(default)]
    pub filesystem_access: Vec<String>,

    /// Additional labels/metadata.
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

fn default_version() -> String {
    "0.1.0".into()
}

/// Agent resource configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResources {
    /// Maximum memory in MB.
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u64,

    /// Maximum concurrent requests this agent handles.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: u32,

    /// Priority level.
    #[serde(default)]
    pub priority: AgentPriority,
}

fn default_max_memory_mb() -> u64 {
    256
}

fn default_max_concurrent() -> u32 {
    100
}

impl Default for AgentResources {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_concurrent_requests: default_max_concurrent(),
            priority: AgentPriority::default(),
        }
    }
}

/// Agent priority level for scheduling.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AgentPriority {
    /// Low priority.
    Low,
    /// Normal priority (default).
    #[default]
    Normal,
    /// High priority.
    High,
    /// Critical -- must not be preempted.
    Critical,
}

/// Agent communication interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInterface {
    /// Communication protocol.
    #[serde(default)]
    pub protocol: InterfaceProtocol,

    /// Request topic (for IPC protocol).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_topic: Option<String>,

    /// Response mode.
    #[serde(default)]
    pub response_mode: ResponseMode,
}

impl Default for AgentInterface {
    fn default() -> Self {
        Self {
            protocol: InterfaceProtocol::Ipc,
            request_topic: None,
            response_mode: ResponseMode::Direct,
        }
    }
}

/// Interface protocol.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceProtocol {
    /// Inter-process communication via kernel IPC.
    #[default]
    Ipc,
    /// REST/HTTP API.
    Rest,
    /// gRPC.
    Grpc,
    /// Model Context Protocol.
    Mcp,
}

/// Response delivery mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseMode {
    /// Reply directly to the sender.
    #[default]
    Direct,
    /// Broadcast response to all subscribers.
    Broadcast,
    /// Publish response to a specific topic.
    Topic(String),
}

/// Agent health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealth {
    /// Health check interval.
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,

    /// Health check timeout.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Restart policy.
    #[serde(default)]
    pub restart_policy: AgentRestartPolicy,

    /// Maximum number of restarts before giving up.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
}

fn default_check_interval() -> u64 {
    30
}

fn default_timeout() -> u64 {
    5
}

fn default_max_restarts() -> u32 {
    5
}

impl Default for AgentHealth {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval(),
            timeout_secs: default_timeout(),
            restart_policy: AgentRestartPolicy::default(),
            max_restarts: default_max_restarts(),
        }
    }
}

/// Agent restart policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRestartPolicy {
    /// Never restart.
    Never,
    /// Restart on failure only.
    #[default]
    OnFailure,
    /// Always restart.
    Always,
}

/// Agent dependencies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentDependencies {
    /// Agents that must be running before this one starts.
    #[serde(default)]
    pub requires: Vec<String>,

    /// Agents that should start before this one (soft ordering).
    #[serde(default)]
    pub after: Vec<String>,
}

impl AgentCapabilities {
    /// Root capabilities: everything allowed.
    pub fn root() -> Self {
        Self {
            can_spawn: true,
            can_ipc: true,
            can_exec_tools: true,
            can_network: true,
            ipc_scope: crate::capability::IpcScope::All,
            resource_limits: crate::capability::ResourceLimits {
                max_memory_bytes: u64::MAX,
                max_cpu_time_ms: u64::MAX,
                max_tool_calls: u64::MAX,
                max_messages: u64::MAX,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_role_display() {
        assert_eq!(AgentRole::Root.to_string(), "root");
        assert_eq!(AgentRole::Service.to_string(), "service");
        assert_eq!(AgentRole::Worker.to_string(), "worker");
        assert_eq!(
            AgentRole::Custom("analytics".into()).to_string(),
            "custom(analytics)"
        );
    }

    #[test]
    fn root_agency_unlimited() {
        let agency = Agency::root();
        assert!(agency.max_children.is_none());
        assert!(agency.can_spawn(&AgentRole::Service));
        assert!(agency.can_spawn(&AgentRole::Worker));
        assert!(agency.can_spawn(&AgentRole::User));
        assert!(!agency.can_spawn(&AgentRole::Root)); // cannot spawn root
    }

    #[test]
    fn supervisor_agency() {
        let agency = Agency::supervisor(10);
        assert_eq!(agency.max_children, Some(10));
        assert!(agency.can_spawn(&AgentRole::Service));
        assert!(agency.can_spawn(&AgentRole::Worker));
        assert!(!agency.can_spawn(&AgentRole::Root));
        assert!(!agency.can_spawn(&AgentRole::User));
    }

    #[test]
    fn service_agency() {
        let agency = Agency::service(5);
        assert!(agency.can_spawn(&AgentRole::Worker));
        assert!(!agency.can_spawn(&AgentRole::Service));
    }

    #[test]
    fn worker_has_no_agency() {
        let agency = Agency::none();
        assert!(!agency.can_spawn(&AgentRole::Worker));
        assert_eq!(agency.remaining_capacity(), Some(0));
    }

    #[test]
    fn agency_child_tracking() {
        let mut agency = Agency::service(2);
        agency.add_child(100);
        agency.add_child(101);
        assert!(!agency.can_spawn(&AgentRole::Worker)); // at capacity
        assert_eq!(agency.remaining_capacity(), Some(0));

        agency.remove_child(100);
        assert!(agency.can_spawn(&AgentRole::Worker));
        assert_eq!(agency.remaining_capacity(), Some(1));
    }

    #[test]
    fn agency_serde_roundtrip() {
        let agency = Agency::supervisor(8);
        let json = serde_json::to_string(&agency).unwrap();
        let restored: Agency = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_children, Some(8));
        assert!(restored.allowed_roles.contains(&AgentRole::Service));
    }

    #[test]
    fn root_capabilities() {
        let caps = AgentCapabilities::root();
        assert!(caps.can_spawn);
        assert!(caps.can_ipc);
        assert!(caps.can_exec_tools);
        assert!(caps.can_network);
        assert_eq!(caps.resource_limits.max_memory_bytes, u64::MAX);
    }

    #[test]
    fn agent_manifest_serde_roundtrip() {
        let manifest = AgentManifest {
            name: "memory-service".into(),
            version: "0.1.0".into(),
            description: "Persistent memory storage".into(),
            role: AgentRole::Service,
            capabilities: AgentCapabilities::default(),
            agency: Agency::service(10),
            tools: vec!["memory_store".into(), "memory_retrieve".into()],
            topics_publish: vec!["memory.stored".into()],
            topics_subscribe: vec!["memory.request".into()],
            resources: AgentResources::default(),
            interface: AgentInterface {
                protocol: InterfaceProtocol::Ipc,
                request_topic: Some("memory.request".into()),
                response_mode: ResponseMode::Direct,
            },
            health: AgentHealth::default(),
            dependencies: AgentDependencies {
                requires: vec![],
                after: vec!["kernel-init".into()],
            },
            filesystem_access: vec!["data/memory/".into()],
            labels: HashMap::new(),
        };
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let restored: AgentManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "memory-service");
        assert_eq!(restored.role, AgentRole::Service);
        assert_eq!(restored.tools.len(), 2);
    }

    #[test]
    fn minimal_manifest_serde() {
        let json = r#"{"name":"worker-1","role":"Worker"}"#;
        let manifest: AgentManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "worker-1");
        assert_eq!(manifest.role, AgentRole::Worker);
        assert!(manifest.tools.is_empty());
        assert_eq!(manifest.version, "0.1.0"); // default
    }

    #[test]
    fn agent_priority_ordering() {
        assert!(AgentPriority::Low < AgentPriority::Normal);
        assert!(AgentPriority::Normal < AgentPriority::High);
        assert!(AgentPriority::High < AgentPriority::Critical);
    }

    #[test]
    fn agent_health_defaults() {
        let health = AgentHealth::default();
        assert_eq!(health.check_interval_secs, 30);
        assert_eq!(health.timeout_secs, 5);
        assert_eq!(health.max_restarts, 5);
        assert_eq!(health.restart_policy, AgentRestartPolicy::OnFailure);
    }

    #[test]
    fn interface_protocol_default() {
        assert_eq!(InterfaceProtocol::default(), InterfaceProtocol::Ipc);
    }

    #[test]
    fn response_mode_serde() {
        let modes = vec![
            ResponseMode::Direct,
            ResponseMode::Broadcast,
            ResponseMode::Topic("events".into()),
        ];
        for mode in &modes {
            let json = serde_json::to_string(mode).unwrap();
            let _restored: ResponseMode = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn agent_dependencies_serde() {
        let deps = AgentDependencies {
            requires: vec!["message-bus".into()],
            after: vec!["kernel-init".into(), "message-bus".into()],
        };
        let json = serde_json::to_string(&deps).unwrap();
        let restored: AgentDependencies = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.requires, vec!["message-bus"]);
        assert_eq!(restored.after.len(), 2);
    }

    #[test]
    fn agent_resources_defaults() {
        let res = AgentResources::default();
        assert_eq!(res.max_memory_mb, 256);
        assert_eq!(res.max_concurrent_requests, 100);
        assert_eq!(res.priority, AgentPriority::Normal);
    }
}
