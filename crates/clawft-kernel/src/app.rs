//! Application framework for WeftOS.
//!
//! Applications are packaged units that declare their agents, tools,
//! services, capabilities, and lifecycle hooks via a manifest file
//! (`weftapp.toml` or `weftapp.json`). The kernel manages application
//! installation, startup, shutdown, and removal.
//!
//! # Design
//!
//! All types compile unconditionally. The `AppManager` tracks installed
//! applications and their lifecycle state. Actual filesystem operations
//! (install from disk, hook execution) require the `native` feature
//! and a running async runtime -- those integrations are future work.
//!
//! Agent IDs are namespaced as `app-name/agent-id` to avoid conflicts
//! between apps and with built-in agents.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::capability::{AgentCapabilities, IpcScope};
use crate::container::PortMapping;
use crate::process::Pid;

// ── Manifest Types ──────────────────────────────────────────────────

/// Application manifest, parsed from `weftapp.toml` or `weftapp.json`.
///
/// Declares the agents, tools, services, capabilities, and lifecycle
/// hooks for a WeftOS application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    /// Application name (unique identifier).
    pub name: String,

    /// Semantic version string.
    pub version: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Application author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// License identifier (SPDX).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Agent specifications.
    #[serde(default)]
    pub agents: Vec<AgentSpec>,

    /// Tool specifications.
    #[serde(default)]
    pub tools: Vec<ToolSpec>,

    /// Service specifications (containers, processes).
    #[serde(default)]
    pub services: Vec<ServiceSpec>,

    /// Application-level capability requirements.
    #[serde(default)]
    pub capabilities: AppCapabilities,

    /// Lifecycle hooks.
    #[serde(default)]
    pub hooks: AppHooks,
}

/// Specification for an agent within an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Agent identifier (scoped to the app: `app-name/id`).
    pub id: String,

    /// Agent role (e.g. "code-review", "report-generator").
    #[serde(default)]
    pub role: String,

    /// Capabilities for this agent.
    #[serde(default)]
    pub capabilities: AgentCapabilities,

    /// Whether to start this agent automatically when the app starts.
    #[serde(default)]
    pub auto_start: bool,
}

/// Specification for a tool provided by an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool name (scoped to the app: `app-name/name`).
    pub name: String,

    /// Where the tool implementation comes from.
    pub source: ToolSource,

    /// JSON Schema for the tool's input parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Source of a tool implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    /// WASM module (path relative to app directory).
    Wasm(String),
    /// Built-in native tool (name).
    Native(String),
    /// Skill file (path relative to app directory).
    Skill(String),
}

/// Specification for a sidecar service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Service name.
    pub name: String,

    /// Docker image (for container services).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Native command (for process services).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Port mappings.
    #[serde(default)]
    pub ports: Vec<PortMapping>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Health check endpoint URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_endpoint: Option<String>,
}

/// Application-level capability requirements.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCapabilities {
    /// Whether the app needs network access.
    #[serde(default)]
    pub network: bool,

    /// Filesystem paths the app needs access to.
    #[serde(default)]
    pub filesystem: Vec<String>,

    /// Whether the app needs shell access.
    #[serde(default)]
    pub shell: bool,

    /// IPC scope for the app's agents.
    #[serde(default)]
    pub ipc: IpcScope,
}

/// Lifecycle hooks (scripts run at lifecycle transitions).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHooks {
    /// Script to run after installation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_install: Option<String>,

    /// Script to run before starting agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_start: Option<String>,

    /// Script to run after stopping agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_stop: Option<String>,

    /// Script to run before removal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_remove: Option<String>,
}

// ── Application Lifecycle ───────────────────────────────────────────

/// Application lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    /// Installed but not started.
    Installed,
    /// Starting agents and services.
    Starting,
    /// All agents and services running.
    Running,
    /// Shutting down agents and services.
    Stopping,
    /// Stopped (can be restarted).
    Stopped,
    /// Failed with a reason.
    Failed(String),
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppState::Installed => write!(f, "installed"),
            AppState::Starting => write!(f, "starting"),
            AppState::Running => write!(f, "running"),
            AppState::Stopping => write!(f, "stopping"),
            AppState::Stopped => write!(f, "stopped"),
            AppState::Failed(reason) => write!(f, "failed: {reason}"),
        }
    }
}

/// An installed application with its runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    /// Application manifest.
    pub manifest: AppManifest,

    /// Current lifecycle state.
    pub state: AppState,

    /// When the app was installed.
    pub installed_at: DateTime<Utc>,

    /// PIDs of agents spawned by this app (populated at start time).
    #[serde(default)]
    pub agent_pids: Vec<Pid>,

    /// Names of services started by this app (populated at start time).
    #[serde(default)]
    pub service_names: Vec<String>,
}

// ── Errors ──────────────────────────────────────────────────────────

/// Application framework errors.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Manifest file not found.
    #[error("manifest not found at '{path}'")]
    ManifestNotFound {
        /// Path that was checked.
        path: String,
    },

    /// Manifest parsing failed.
    #[error("invalid manifest: {reason}")]
    ManifestInvalid {
        /// Why parsing failed.
        reason: String,
    },

    /// App with this name already installed.
    #[error("app already installed: '{name}'")]
    AlreadyInstalled {
        /// App name.
        name: String,
    },

    /// App not found.
    #[error("app not found: '{name}'")]
    NotFound {
        /// App name.
        name: String,
    },

    /// Invalid state for the requested operation.
    #[error("invalid state for app '{name}': expected {expected}, got {actual}")]
    InvalidState {
        /// App name.
        name: String,
        /// Expected state description.
        expected: String,
        /// Actual state.
        actual: String,
    },

    /// Agent spawn failed.
    #[error("failed to spawn agent '{agent_id}' for app '{app_name}': {reason}")]
    SpawnFailed {
        /// App name.
        app_name: String,
        /// Agent ID within the app.
        agent_id: String,
        /// Failure reason.
        reason: String,
    },

    /// Hook execution failed.
    #[error("hook '{hook}' failed for app '{app_name}': {reason}")]
    HookFailed {
        /// App name.
        app_name: String,
        /// Hook name (on_install, on_start, etc.).
        hook: String,
        /// Failure reason.
        reason: String,
    },
}

// ── Manifest Validation ─────────────────────────────────────────────

/// Validate an application manifest for structural correctness.
///
/// Checks:
/// - Name is non-empty and contains only valid characters
/// - Version is non-empty
/// - Agent IDs are unique within the app
/// - Tool names are unique within the app
/// - Service names are unique within the app
/// - Tool sources are valid variants
pub fn validate_manifest(manifest: &AppManifest) -> Result<(), AppError> {
    // Name validation
    if manifest.name.is_empty() {
        return Err(AppError::ManifestInvalid {
            reason: "app name must not be empty".into(),
        });
    }

    if !manifest
        .name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::ManifestInvalid {
            reason: format!(
                "app name '{}' contains invalid characters (use alphanumeric, - or _)",
                manifest.name
            ),
        });
    }

    // Version validation
    if manifest.version.is_empty() {
        return Err(AppError::ManifestInvalid {
            reason: "version must not be empty".into(),
        });
    }

    // Unique agent IDs
    let mut agent_ids = std::collections::HashSet::new();
    for agent in &manifest.agents {
        if agent.id.is_empty() {
            return Err(AppError::ManifestInvalid {
                reason: "agent id must not be empty".into(),
            });
        }
        if !agent_ids.insert(&agent.id) {
            return Err(AppError::ManifestInvalid {
                reason: format!("duplicate agent id: '{}'", agent.id),
            });
        }
    }

    // Unique tool names
    let mut tool_names = std::collections::HashSet::new();
    for tool in &manifest.tools {
        if tool.name.is_empty() {
            return Err(AppError::ManifestInvalid {
                reason: "tool name must not be empty".into(),
            });
        }
        if !tool_names.insert(&tool.name) {
            return Err(AppError::ManifestInvalid {
                reason: format!("duplicate tool name: '{}'", tool.name),
            });
        }
    }

    // Unique service names
    let mut service_names = std::collections::HashSet::new();
    for service in &manifest.services {
        if service.name.is_empty() {
            return Err(AppError::ManifestInvalid {
                reason: "service name must not be empty".into(),
            });
        }
        if !service_names.insert(&service.name) {
            return Err(AppError::ManifestInvalid {
                reason: format!("duplicate service name: '{}'", service.name),
            });
        }
    }

    Ok(())
}

// ── AppManager ──────────────────────────────────────────────────────

/// Application lifecycle manager.
///
/// Tracks installed applications and their lifecycle state. Agent
/// spawning and service starting are delegated to the supervisor and
/// container manager respectively -- those integrations are wired in
/// the kernel boot sequence.
pub struct AppManager {
    apps: DashMap<String, InstalledApp>,
}

impl AppManager {
    /// Create a new application manager.
    pub fn new() -> Self {
        Self {
            apps: DashMap::new(),
        }
    }

    /// Register an application from a parsed manifest.
    ///
    /// The app is placed in the `Installed` state. Call `transition_to`
    /// to advance the state (e.g., to `Starting` or `Running`).
    ///
    /// # Errors
    ///
    /// Returns `AppError::AlreadyInstalled` if an app with the same
    /// name is already registered.
    pub fn install(&self, manifest: AppManifest) -> Result<String, AppError> {
        validate_manifest(&manifest)?;

        let name = manifest.name.clone();

        if self.apps.contains_key(&name) {
            return Err(AppError::AlreadyInstalled { name: name.clone() });
        }

        debug!(app = %name, version = %manifest.version, "installing application");

        self.apps.insert(
            name.clone(),
            InstalledApp {
                manifest,
                state: AppState::Installed,
                installed_at: Utc::now(),
                agent_pids: Vec::new(),
                service_names: Vec::new(),
            },
        );

        Ok(name)
    }

    /// Transition an app to a new state.
    ///
    /// Validates that the transition is legal per the state machine:
    /// - Installed -> Starting
    /// - Starting -> Running | Failed
    /// - Running -> Stopping
    /// - Stopping -> Stopped | Failed
    /// - Stopped -> Starting
    ///
    /// # Errors
    ///
    /// Returns `AppError::NotFound` or `AppError::InvalidState`.
    pub fn transition_to(&self, name: &str, new_state: AppState) -> Result<(), AppError> {
        let mut entry = self.apps.get_mut(name).ok_or_else(|| AppError::NotFound {
            name: name.to_owned(),
        })?;

        let valid = matches!(
            (&entry.state, &new_state),
            (AppState::Installed, AppState::Starting)
                | (AppState::Starting, AppState::Running)
                | (AppState::Starting, AppState::Failed(_))
                | (AppState::Running, AppState::Stopping)
                | (AppState::Stopping, AppState::Stopped)
                | (AppState::Stopping, AppState::Failed(_))
                | (AppState::Stopped, AppState::Starting)
        );

        if !valid {
            return Err(AppError::InvalidState {
                name: name.to_owned(),
                expected: format!("valid transition from {}", entry.state),
                actual: format!("{} -> {new_state}", entry.state),
            });
        }

        debug!(app = name, from = %entry.state, to = %new_state, "state transition");
        entry.state = new_state;
        Ok(())
    }

    /// Record an agent PID for a running app.
    pub fn add_agent_pid(&self, name: &str, pid: Pid) -> Result<(), AppError> {
        let mut entry = self.apps.get_mut(name).ok_or_else(|| AppError::NotFound {
            name: name.to_owned(),
        })?;
        entry.agent_pids.push(pid);
        Ok(())
    }

    /// Record a service name for a running app.
    pub fn add_service_name(&self, name: &str, service_name: String) -> Result<(), AppError> {
        let mut entry = self.apps.get_mut(name).ok_or_else(|| AppError::NotFound {
            name: name.to_owned(),
        })?;
        entry.service_names.push(service_name);
        Ok(())
    }

    /// Remove an installed application.
    ///
    /// The app must be in `Installed`, `Stopped`, or `Failed` state.
    pub fn remove(&self, name: &str) -> Result<AppManifest, AppError> {
        let entry = self.apps.get(name).ok_or_else(|| AppError::NotFound {
            name: name.to_owned(),
        })?;

        let removable = matches!(
            entry.state,
            AppState::Installed | AppState::Stopped | AppState::Failed(_)
        );

        if !removable {
            return Err(AppError::InvalidState {
                name: name.to_owned(),
                expected: "Installed, Stopped, or Failed".into(),
                actual: entry.state.to_string(),
            });
        }

        drop(entry); // release the read lock before remove
        let (_, app) = self.apps.remove(name).ok_or_else(|| AppError::NotFound {
            name: name.to_owned(),
        })?;

        debug!(app = name, "removed application");
        Ok(app.manifest)
    }

    /// List all installed applications.
    pub fn list(&self) -> Vec<(String, AppState, String)> {
        self.apps
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry.state.clone(),
                    entry.manifest.version.clone(),
                )
            })
            .collect()
    }

    /// Get details for an installed application.
    pub fn inspect(&self, name: &str) -> Result<InstalledApp, AppError> {
        self.apps
            .get(name)
            .map(|e| e.value().clone())
            .ok_or_else(|| AppError::NotFound {
                name: name.to_owned(),
            })
    }

    /// Get the number of installed apps.
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    /// Check whether any apps are installed.
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    /// Get namespaced agent IDs for an app's manifest.
    ///
    /// Returns IDs in the form `app-name/agent-id`.
    pub fn namespaced_agent_ids(manifest: &AppManifest) -> Vec<String> {
        manifest
            .agents
            .iter()
            .map(|a| format!("{}/{}", manifest.name, a.id))
            .collect()
    }

    /// Get namespaced tool names for an app's manifest.
    ///
    /// Returns names in the form `app-name/tool-name`.
    pub fn namespaced_tool_names(manifest: &AppManifest) -> Vec<String> {
        manifest
            .tools
            .iter()
            .map(|t| format!("{}/{}", manifest.name, t.name))
            .collect()
    }
}

impl Default for AppManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Manifest Parsing ────────────────────────────────────────────────

impl AppManifest {
    /// Parse an [`AppManifest`] from a JSON string.
    ///
    /// The manifest is validated after parsing; structural errors
    /// (empty name, duplicate IDs, etc.) are returned as
    /// [`AppError::ManifestInvalid`].
    pub fn from_json_str(json: &str) -> Result<Self, AppError> {
        let manifest: AppManifest =
            serde_json::from_str(json).map_err(|e| AppError::ManifestInvalid {
                reason: format!("JSON parse error: {e}"),
            })?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> AppManifest {
        AppManifest {
            name: "code-reviewer".into(),
            version: "1.0.0".into(),
            description: "Automated code review app".into(),
            author: Some("WeftOS Team".into()),
            license: Some("MIT".into()),
            agents: vec![
                AgentSpec {
                    id: "reviewer".into(),
                    role: "code-review".into(),
                    capabilities: AgentCapabilities::default(),
                    auto_start: true,
                },
                AgentSpec {
                    id: "reporter".into(),
                    role: "report-generator".into(),
                    capabilities: AgentCapabilities {
                        can_network: false,
                        ..Default::default()
                    },
                    auto_start: true,
                },
            ],
            tools: vec![ToolSpec {
                name: "diff-analyzer".into(),
                source: ToolSource::Wasm("tools/diff-analyzer.wasm".into()),
                schema: None,
            }],
            services: vec![ServiceSpec {
                name: "review-db".into(),
                image: Some("redis:7-alpine".into()),
                command: None,
                ports: vec![PortMapping {
                    host_port: 6380,
                    container_port: 6379,
                    protocol: "tcp".into(),
                }],
                env: HashMap::new(),
                health_endpoint: Some("redis://localhost:6380".into()),
            }],
            capabilities: AppCapabilities {
                network: true,
                filesystem: vec!["/workspace".into()],
                shell: false,
                ipc: IpcScope::All,
            },
            hooks: AppHooks {
                on_install: Some("scripts/setup.sh".into()),
                on_start: Some("scripts/migrate.sh".into()),
                on_stop: None,
                on_remove: None,
            },
        }
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let manifest = sample_manifest();
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let restored: AppManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "code-reviewer");
        assert_eq!(restored.version, "1.0.0");
        assert_eq!(restored.agents.len(), 2);
        assert_eq!(restored.tools.len(), 1);
        assert_eq!(restored.services.len(), 1);
    }

    #[test]
    fn manifest_minimal_serde() {
        let json = r#"{"name":"my-app","version":"0.1.0"}"#;
        let manifest: AppManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "my-app");
        assert!(manifest.agents.is_empty());
        assert!(manifest.tools.is_empty());
        assert!(manifest.services.is_empty());
        assert!(!manifest.capabilities.network);
    }

    #[test]
    fn validate_manifest_ok() {
        let manifest = sample_manifest();
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn validate_manifest_empty_name() {
        let mut manifest = sample_manifest();
        manifest.name = String::new();
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_manifest_invalid_name_chars() {
        let mut manifest = sample_manifest();
        manifest.name = "my app!".into();
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn validate_manifest_empty_version() {
        let mut manifest = sample_manifest();
        manifest.version = String::new();
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.to_string().contains("version"));
    }

    #[test]
    fn validate_manifest_duplicate_agent_ids() {
        let mut manifest = sample_manifest();
        manifest.agents.push(AgentSpec {
            id: "reviewer".into(), // duplicate
            role: "other".into(),
            capabilities: AgentCapabilities::default(),
            auto_start: false,
        });
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.to_string().contains("duplicate agent"));
    }

    #[test]
    fn validate_manifest_duplicate_tool_names() {
        let mut manifest = sample_manifest();
        manifest.tools.push(ToolSpec {
            name: "diff-analyzer".into(), // duplicate
            source: ToolSource::Native("builtin".into()),
            schema: None,
        });
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.to_string().contains("duplicate tool"));
    }

    #[test]
    fn validate_manifest_duplicate_service_names() {
        let mut manifest = sample_manifest();
        manifest.services.push(ServiceSpec {
            name: "review-db".into(), // duplicate
            image: None,
            command: Some("redis-server".into()),
            ports: Vec::new(),
            env: HashMap::new(),
            health_endpoint: None,
        });
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.to_string().contains("duplicate service"));
    }

    #[test]
    fn app_state_display() {
        assert_eq!(AppState::Installed.to_string(), "installed");
        assert_eq!(AppState::Running.to_string(), "running");
        assert_eq!(AppState::Stopped.to_string(), "stopped");
        assert_eq!(
            AppState::Failed("timeout".into()).to_string(),
            "failed: timeout"
        );
    }

    #[test]
    fn install_and_list() {
        let manager = AppManager::new();
        let name = manager.install(sample_manifest()).unwrap();
        assert_eq!(name, "code-reviewer");

        let list = manager.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "code-reviewer");
        assert_eq!(list[0].1, AppState::Installed);
    }

    #[test]
    fn install_duplicate_fails() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        let err = manager.install(sample_manifest()).unwrap_err();
        assert!(matches!(err, AppError::AlreadyInstalled { .. }));
    }

    #[test]
    fn inspect_installed_app() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        let app = manager.inspect("code-reviewer").unwrap();
        assert_eq!(app.state, AppState::Installed);
        assert_eq!(app.manifest.agents.len(), 2);
    }

    #[test]
    fn inspect_not_found() {
        let manager = AppManager::new();
        assert!(matches!(
            manager.inspect("nope"),
            Err(AppError::NotFound { .. })
        ));
    }

    #[test]
    fn state_transitions() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();

        // Installed -> Starting -> Running -> Stopping -> Stopped
        manager
            .transition_to("code-reviewer", AppState::Starting)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Running)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Stopping)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Stopped)
            .unwrap();

        let app = manager.inspect("code-reviewer").unwrap();
        assert_eq!(app.state, AppState::Stopped);
    }

    #[test]
    fn state_transition_restart() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();

        manager
            .transition_to("code-reviewer", AppState::Starting)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Running)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Stopping)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Stopped)
            .unwrap();
        // Restart: Stopped -> Starting
        manager
            .transition_to("code-reviewer", AppState::Starting)
            .unwrap();
    }

    #[test]
    fn invalid_state_transition() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();

        // Installed -> Running (should fail, must go through Starting)
        let err = manager
            .transition_to("code-reviewer", AppState::Running)
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidState { .. }));
    }

    #[test]
    fn state_transition_to_failed() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();

        manager
            .transition_to("code-reviewer", AppState::Starting)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Failed("agent crash".into()))
            .unwrap();

        let app = manager.inspect("code-reviewer").unwrap();
        assert_eq!(app.state, AppState::Failed("agent crash".into()));
    }

    #[test]
    fn remove_installed_app() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        let manifest = manager.remove("code-reviewer").unwrap();
        assert_eq!(manifest.name, "code-reviewer");
        assert!(manager.is_empty());
    }

    #[test]
    fn remove_running_app_fails() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        manager
            .transition_to("code-reviewer", AppState::Starting)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Running)
            .unwrap();

        let err = manager.remove("code-reviewer").unwrap_err();
        assert!(matches!(err, AppError::InvalidState { .. }));
    }

    #[test]
    fn remove_stopped_app() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        manager
            .transition_to("code-reviewer", AppState::Starting)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Running)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Stopping)
            .unwrap();
        manager
            .transition_to("code-reviewer", AppState::Stopped)
            .unwrap();

        assert!(manager.remove("code-reviewer").is_ok());
    }

    #[test]
    fn add_agent_pid() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        manager.add_agent_pid("code-reviewer", 42).unwrap();

        let app = manager.inspect("code-reviewer").unwrap();
        assert_eq!(app.agent_pids, vec![42]);
    }

    #[test]
    fn add_service_name() {
        let manager = AppManager::new();
        manager.install(sample_manifest()).unwrap();
        manager
            .add_service_name("code-reviewer", "review-db".into())
            .unwrap();

        let app = manager.inspect("code-reviewer").unwrap();
        assert_eq!(app.service_names, vec!["review-db"]);
    }

    #[test]
    fn namespaced_ids() {
        let manifest = sample_manifest();
        let agent_ids = AppManager::namespaced_agent_ids(&manifest);
        assert_eq!(
            agent_ids,
            vec!["code-reviewer/reviewer", "code-reviewer/reporter"]
        );

        let tool_names = AppManager::namespaced_tool_names(&manifest);
        assert_eq!(tool_names, vec!["code-reviewer/diff-analyzer"]);
    }

    #[test]
    fn tool_source_variants() {
        let wasm = ToolSource::Wasm("tools/my.wasm".into());
        let native = ToolSource::Native("read_file".into());
        let skill = ToolSource::Skill("skills/REVIEW.md".into());

        // Serde roundtrip
        for source in &[wasm, native, skill] {
            let json = serde_json::to_string(source).unwrap();
            let _restored: ToolSource = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn app_error_display() {
        let err = AppError::ManifestNotFound {
            path: "/tmp/weftapp.toml".into(),
        };
        assert!(err.to_string().contains("manifest not found"));

        let err = AppError::AlreadyInstalled {
            name: "my-app".into(),
        };
        assert!(err.to_string().contains("my-app"));

        let err = AppError::HookFailed {
            app_name: "my-app".into(),
            hook: "on_start".into(),
            reason: "exit code 1".into(),
        };
        assert!(err.to_string().contains("on_start"));
    }

    #[test]
    fn app_capabilities_serde_roundtrip() {
        let caps = AppCapabilities {
            network: true,
            filesystem: vec!["/workspace".into(), "/data".into()],
            shell: false,
            ipc: IpcScope::All,
        };
        let json = serde_json::to_string(&caps).unwrap();
        let restored: AppCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, caps);
    }

    #[test]
    fn app_hooks_serde_roundtrip() {
        let hooks = AppHooks {
            on_install: Some("setup.sh".into()),
            on_start: None,
            on_stop: Some("cleanup.sh".into()),
            on_remove: None,
        };
        let json = serde_json::to_string(&hooks).unwrap();
        let restored: AppHooks = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.on_install.as_deref(), Some("setup.sh"));
        assert!(restored.on_start.is_none());
    }

    #[test]
    fn parse_manifest_from_json() {
        let json = serde_json::json!({
            "name": "test-app",
            "version": "1.0.0",
            "description": "A test app",
            "agents": [],
            "tools": [],
            "services": [],
            "capabilities": {
                "network": false,
                "filesystem": [],
                "shell": false,
                "ipc": "None"
            },
            "hooks": {}
        });
        let manifest = AppManifest::from_json_str(&json.to_string()).unwrap();
        assert_eq!(manifest.name, "test-app");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.agents.is_empty());
    }

    #[test]
    fn parse_manifest_from_json_invalid() {
        let result = AppManifest::from_json_str("not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON parse error"));
    }

    #[test]
    fn parse_manifest_from_json_empty_name_fails() {
        let json = serde_json::json!({
            "name": "",
            "version": "1.0.0"
        });
        let result = AppManifest::from_json_str(&json.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn app_hooks_lifecycle() {
        let manifest = AppManifest {
            name: "hooks-test".into(),
            version: "0.1.0".into(),
            description: String::new(),
            author: None,
            license: None,
            agents: Vec::new(),
            tools: Vec::new(),
            services: Vec::new(),
            capabilities: AppCapabilities::default(),
            hooks: AppHooks {
                on_install: Some("scripts/setup.sh".into()),
                on_start: Some("scripts/migrate.sh".into()),
                on_stop: Some("scripts/cleanup.sh".into()),
                on_remove: None,
            },
        };
        assert_eq!(manifest.hooks.on_install, Some("scripts/setup.sh".into()));
        assert_eq!(manifest.hooks.on_start, Some("scripts/migrate.sh".into()));
        assert_eq!(manifest.hooks.on_stop, Some("scripts/cleanup.sh".into()));
        assert!(manifest.hooks.on_remove.is_none());
    }
}
