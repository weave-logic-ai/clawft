//! Container integration for sidecar service orchestration.
//!
//! Provides types and configuration for managing containerized
//! sidecar services (databases, caches, external APIs) alongside
//! the agent process.
//!
//! # Feature Gate
//!
//! This module is compiled unconditionally, but actual Docker
//! integration requires the `containers` feature flag. Without it,
//! [`ContainerManager::new`] returns a manager that rejects all
//! operations with [`ContainerError::DockerNotAvailable`].
//!
//! # Architecture
//!
//! Each managed container is wrapped in a `ContainerService` and
//! registered in the kernel's `ServiceRegistry`, making container
//! health visible through the standard health monitoring system.

use std::collections::HashMap;
use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::health::HealthStatus;

/// Configuration for the container manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Docker socket path.
    /// Default: "unix:///var/run/docker.sock"
    #[serde(default = "default_docker_socket")]
    pub docker_socket: String,

    /// Docker network name for managed containers.
    /// Default: "weftos"
    #[serde(default = "default_network_name")]
    pub network_name: String,

    /// Default restart policy for new containers.
    #[serde(default)]
    pub default_restart_policy: RestartPolicy,

    /// Health check interval in seconds.
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
}

fn default_docker_socket() -> String {
    "unix:///var/run/docker.sock".into()
}

fn default_network_name() -> String {
    "weftos".into()
}

fn default_health_check_interval() -> u64 {
    30
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            docker_socket: default_docker_socket(),
            network_name: default_network_name(),
            default_restart_policy: RestartPolicy::default(),
            health_check_interval_secs: default_health_check_interval(),
        }
    }
}

impl ContainerConfig {
    /// Get the health check interval as a Duration.
    pub fn health_check_interval(&self) -> Duration {
        Duration::from_secs(self.health_check_interval_secs)
    }
}

/// Container lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerState {
    /// Image is being pulled.
    Pulling,
    /// Container is being created.
    Creating,
    /// Container is running.
    Running,
    /// Container is being stopped.
    Stopping,
    /// Container is stopped.
    Stopped,
    /// Container failed with an error.
    Failed(String),
}

impl std::fmt::Display for ContainerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerState::Pulling => write!(f, "pulling"),
            ContainerState::Creating => write!(f, "creating"),
            ContainerState::Running => write!(f, "running"),
            ContainerState::Stopping => write!(f, "stopping"),
            ContainerState::Stopped => write!(f, "stopped"),
            ContainerState::Failed(reason) => write!(f, "failed: {reason}"),
        }
    }
}

/// Port mapping between host and container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port number.
    pub host_port: u16,
    /// Container port number.
    pub container_port: u16,
    /// Protocol (tcp, udp).
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "tcp".into()
}

/// Volume mount configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Host path to mount.
    pub host_path: String,
    /// Container path to mount to.
    pub container_path: String,
    /// Whether the mount is read-only.
    #[serde(default)]
    pub read_only: bool,
}

/// Container restart policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Never restart.
    #[default]
    Never,
    /// Restart on failure up to max_retries.
    OnFailure {
        /// Maximum number of restart attempts.
        max_retries: u32,
    },
    /// Always restart.
    Always,
}

/// Specification for a managed container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedContainer {
    /// Container name (unique identifier).
    pub name: String,

    /// Docker image reference.
    pub image: String,

    /// Docker container ID (set after creation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,

    /// Current state.
    #[serde(default = "default_container_state")]
    pub state: ContainerState,

    /// Port mappings.
    #[serde(default)]
    pub ports: Vec<PortMapping>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Volume mounts.
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,

    /// HTTP health check endpoint (e.g. "http://localhost:6379/ping").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_endpoint: Option<String>,

    /// Restart policy override (uses manager default if None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<RestartPolicy>,
}

fn default_container_state() -> ContainerState {
    ContainerState::Stopped
}

/// Container manager errors.
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    /// Docker is not available on this system.
    #[error("Docker not available: {0}")]
    DockerNotAvailable(String),

    /// Image pull failed.
    #[error("image pull failed for '{image}': {reason}")]
    ImagePullFailed {
        /// Image reference.
        image: String,
        /// Failure reason.
        reason: String,
    },

    /// Container creation failed.
    #[error("container creation failed for '{name}': {reason}")]
    CreateFailed {
        /// Container name.
        name: String,
        /// Failure reason.
        reason: String,
    },

    /// Container start failed.
    #[error("container start failed for '{name}': {reason}")]
    StartFailed {
        /// Container name.
        name: String,
        /// Failure reason.
        reason: String,
    },

    /// Port conflict on the host.
    #[error("port conflict: host port {port} already in use")]
    PortConflict {
        /// Conflicting port.
        port: u16,
    },

    /// Container not found.
    #[error("container not found: '{name}'")]
    ContainerNotFound {
        /// Container name.
        name: String,
    },

    /// Health check failed.
    #[error("health check failed for '{name}': {reason}")]
    HealthCheckFailed {
        /// Container name.
        name: String,
        /// Failure reason.
        reason: String,
    },
}

/// Container lifecycle manager.
///
/// When the `containers` feature is enabled, this uses bollard
/// for Docker API access. Without the feature, all operations
/// return [`ContainerError::DockerNotAvailable`].
pub struct ContainerManager {
    config: ContainerConfig,
    managed: DashMap<String, ManagedContainer>,
}

impl ContainerManager {
    /// Create a new container manager.
    ///
    /// Does NOT attempt to connect to Docker at construction time.
    /// Connection is deferred to the first operation that needs it.
    pub fn new(config: ContainerConfig) -> Self {
        Self {
            config,
            managed: DashMap::new(),
        }
    }

    /// Get the container configuration.
    pub fn config(&self) -> &ContainerConfig {
        &self.config
    }

    /// Register a container specification for management.
    ///
    /// This does not start the container; it only registers it
    /// for tracking. Call `start_container` to actually start it.
    pub fn register(&self, spec: ManagedContainer) {
        debug!(name = %spec.name, image = %spec.image, "registering container");
        self.managed.insert(spec.name.clone(), spec);
    }

    /// Start a managed container.
    ///
    /// # Errors
    ///
    /// Returns [`ContainerError::DockerNotAvailable`] when the
    /// `containers` feature is not enabled.
    pub fn start_container(&self, name: &str) -> Result<(), ContainerError> {
        let mut entry = self
            .managed
            .get_mut(name)
            .ok_or_else(|| ContainerError::ContainerNotFound {
                name: name.to_owned(),
            })?;

        #[cfg(not(feature = "containers"))]
        {
            entry.state = ContainerState::Failed(
                "Docker runtime unavailable: compile with --features containers".into(),
            );
            Err(ContainerError::DockerNotAvailable(
                "compile with --features containers".into(),
            ))
        }

        #[cfg(feature = "containers")]
        {
            // TODO: Use bollard to pull image, create container, start
            entry.state = ContainerState::Failed(
                "Docker runtime not yet implemented".into(),
            );
            Err(ContainerError::DockerNotAvailable(
                "Docker runtime not yet implemented".into(),
            ))
        }
    }

    /// Stop a managed container.
    pub fn stop_container(&self, name: &str) -> Result<(), ContainerError> {
        let mut entry = self
            .managed
            .get_mut(name)
            .ok_or_else(|| ContainerError::ContainerNotFound {
                name: name.to_owned(),
            })?;

        debug!(name, "stopping container");
        entry.state = ContainerState::Stopped;
        Ok(())
    }

    /// Get the state of a managed container.
    pub fn container_state(&self, name: &str) -> Option<ContainerState> {
        self.managed.get(name).map(|e| e.state.clone())
    }

    /// List all managed containers with their states.
    pub fn list_containers(&self) -> Vec<(String, ContainerState)> {
        self.managed
            .iter()
            .map(|e| (e.key().clone(), e.value().state.clone()))
            .collect()
    }

    /// Health check for a specific container.
    pub fn health_check(&self, name: &str) -> Result<HealthStatus, ContainerError> {
        let entry = self
            .managed
            .get(name)
            .ok_or_else(|| ContainerError::ContainerNotFound {
                name: name.to_owned(),
            })?;

        match &entry.state {
            ContainerState::Running => Ok(HealthStatus::Healthy),
            ContainerState::Stopped => Ok(HealthStatus::Unhealthy("stopped".into())),
            ContainerState::Failed(reason) => {
                Ok(HealthStatus::Unhealthy(format!("failed: {reason}")))
            }
            other => Ok(HealthStatus::Degraded(format!("state: {other}"))),
        }
    }

    /// Stop all managed containers.
    pub fn stop_all(&self) {
        for mut entry in self.managed.iter_mut() {
            if matches!(entry.state, ContainerState::Running) {
                debug!(name = %entry.key(), "stopping container");
                entry.state = ContainerState::Stopped;
            }
        }
    }

    /// Get the number of managed containers.
    pub fn len(&self) -> usize {
        self.managed.len()
    }

    /// Check whether any containers are managed.
    pub fn is_empty(&self) -> bool {
        self.managed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ContainerConfig::default();
        assert!(config.docker_socket.contains("docker.sock"));
        assert_eq!(config.network_name, "weftos");
        assert_eq!(config.default_restart_policy, RestartPolicy::Never);
        assert_eq!(config.health_check_interval_secs, 30);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = ContainerConfig {
            docker_socket: "tcp://localhost:2375".into(),
            network_name: "custom-net".into(),
            default_restart_policy: RestartPolicy::Always,
            health_check_interval_secs: 10,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: ContainerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.network_name, "custom-net");
        assert_eq!(restored.default_restart_policy, RestartPolicy::Always);
    }

    #[test]
    fn health_check_interval_duration() {
        let config = ContainerConfig {
            health_check_interval_secs: 15,
            ..Default::default()
        };
        assert_eq!(config.health_check_interval(), Duration::from_secs(15));
    }

    #[test]
    fn container_state_display() {
        assert_eq!(ContainerState::Pulling.to_string(), "pulling");
        assert_eq!(ContainerState::Running.to_string(), "running");
        assert_eq!(ContainerState::Stopped.to_string(), "stopped");
        assert_eq!(
            ContainerState::Failed("oom".into()).to_string(),
            "failed: oom"
        );
    }

    #[test]
    fn register_and_list() {
        let manager = ContainerManager::new(ContainerConfig::default());
        manager.register(ManagedContainer {
            name: "redis".into(),
            image: "redis:7-alpine".into(),
            container_id: None,
            state: ContainerState::Stopped,
            ports: vec![PortMapping {
                host_port: 6379,
                container_port: 6379,
                protocol: "tcp".into(),
            }],
            env: HashMap::new(),
            volumes: Vec::new(),
            health_endpoint: None,
            restart_policy: None,
        });

        let containers = manager.list_containers();
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].0, "redis");
    }

    #[test]
    fn stop_container() {
        let manager = ContainerManager::new(ContainerConfig::default());
        manager.register(ManagedContainer {
            name: "redis".into(),
            image: "redis:7-alpine".into(),
            container_id: None,
            state: ContainerState::Running,
            ports: Vec::new(),
            env: HashMap::new(),
            volumes: Vec::new(),
            health_endpoint: None,
            restart_policy: None,
        });

        manager.stop_container("redis").unwrap();
        assert_eq!(
            manager.container_state("redis"),
            Some(ContainerState::Stopped)
        );
    }

    #[test]
    fn stop_nonexistent_fails() {
        let manager = ContainerManager::new(ContainerConfig::default());
        let result = manager.stop_container("nonexistent");
        assert!(matches!(
            result,
            Err(ContainerError::ContainerNotFound { .. })
        ));
    }

    #[test]
    fn health_check_running() {
        let manager = ContainerManager::new(ContainerConfig::default());
        manager.register(ManagedContainer {
            name: "redis".into(),
            image: "redis:7-alpine".into(),
            container_id: None,
            state: ContainerState::Running,
            ports: Vec::new(),
            env: HashMap::new(),
            volumes: Vec::new(),
            health_endpoint: None,
            restart_policy: None,
        });

        let health = manager.health_check("redis").unwrap();
        assert!(matches!(health, HealthStatus::Healthy));
    }

    #[test]
    fn health_check_stopped() {
        let manager = ContainerManager::new(ContainerConfig::default());
        manager.register(ManagedContainer {
            name: "redis".into(),
            image: "redis:7-alpine".into(),
            container_id: None,
            state: ContainerState::Stopped,
            ports: Vec::new(),
            env: HashMap::new(),
            volumes: Vec::new(),
            health_endpoint: None,
            restart_policy: None,
        });

        let health = manager.health_check("redis").unwrap();
        assert!(matches!(health, HealthStatus::Unhealthy(_)));
    }

    #[test]
    fn health_check_nonexistent() {
        let manager = ContainerManager::new(ContainerConfig::default());
        assert!(manager.health_check("nope").is_err());
    }

    #[test]
    fn stop_all() {
        let manager = ContainerManager::new(ContainerConfig::default());
        for name in &["redis", "postgres", "memcached"] {
            manager.register(ManagedContainer {
                name: (*name).into(),
                image: format!("{name}:latest"),
                container_id: None,
                state: ContainerState::Running,
                ports: Vec::new(),
                env: HashMap::new(),
                volumes: Vec::new(),
                health_endpoint: None,
                restart_policy: None,
            });
        }

        manager.stop_all();

        for (_, state) in manager.list_containers() {
            assert_eq!(state, ContainerState::Stopped);
        }
    }

    #[test]
    fn start_without_feature_fails() {
        let manager = ContainerManager::new(ContainerConfig::default());
        manager.register(ManagedContainer {
            name: "redis".into(),
            image: "redis:7-alpine".into(),
            container_id: None,
            state: ContainerState::Stopped,
            ports: Vec::new(),
            env: HashMap::new(),
            volumes: Vec::new(),
            health_endpoint: None,
            restart_policy: None,
        });

        #[cfg(not(feature = "containers"))]
        {
            let result = manager.start_container("redis");
            assert!(matches!(
                result,
                Err(ContainerError::DockerNotAvailable(_))
            ));
        }
    }

    #[test]
    fn managed_container_serde_roundtrip() {
        let container = ManagedContainer {
            name: "redis".into(),
            image: "redis:7-alpine".into(),
            container_id: Some("abc123".into()),
            state: ContainerState::Running,
            ports: vec![PortMapping {
                host_port: 6379,
                container_port: 6379,
                protocol: "tcp".into(),
            }],
            env: HashMap::from([("REDIS_PASSWORD".into(), "secret".into())]),
            volumes: vec![VolumeMount {
                host_path: "/data".into(),
                container_path: "/var/lib/redis".into(),
                read_only: false,
            }],
            health_endpoint: Some("http://localhost:6379/ping".into()),
            restart_policy: Some(RestartPolicy::OnFailure { max_retries: 3 }),
        };

        let json = serde_json::to_string(&container).unwrap();
        let restored: ManagedContainer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "redis");
        assert_eq!(restored.ports.len(), 1);
        assert_eq!(restored.volumes.len(), 1);
        assert!(!restored.volumes[0].read_only);
    }

    #[test]
    fn container_error_display() {
        let err = ContainerError::DockerNotAvailable("not installed".into());
        assert!(err.to_string().contains("Docker"));

        let err = ContainerError::ContainerNotFound {
            name: "redis".into(),
        };
        assert!(err.to_string().contains("redis"));

        let err = ContainerError::PortConflict { port: 8080 };
        assert!(err.to_string().contains("8080"));
    }

    #[test]
    fn restart_policy_serde() {
        let policies = vec![
            RestartPolicy::Never,
            RestartPolicy::OnFailure { max_retries: 5 },
            RestartPolicy::Always,
        ];
        for policy in policies {
            let json = serde_json::to_string(&policy).unwrap();
            let restored: RestartPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, policy);
        }
    }
}
