//! Plugin manifest types.
//!
//! Defines [`PluginManifest`], [`PluginCapability`], [`PluginPermissions`],
//! and [`PluginResourceConfig`] -- the schema for plugin metadata parsed
//! from `clawft.plugin.json` or `.yaml` files.

use serde::{Deserialize, Serialize};

use crate::PluginError;

/// Plugin manifest parsed from `clawft.plugin.json` or `.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (reverse-domain, e.g., `"com.example.my-plugin"`).
    pub id: String,

    /// Human-readable plugin name.
    pub name: String,

    /// Semantic version string (must be valid semver).
    pub version: String,

    /// Capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,

    /// Permissions the plugin requests.
    #[serde(default)]
    pub permissions: PluginPermissions,

    /// Resource limits configuration.
    #[serde(default)]
    pub resources: PluginResourceConfig,

    /// Path to the WASM module (relative to plugin directory).
    #[serde(default)]
    pub wasm_module: Option<String>,

    /// Skills provided by this plugin.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Tools provided by this plugin.
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Plugin capability types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Tool execution capability.
    Tool,
    /// Channel adapter capability.
    Channel,
    /// Pipeline stage capability.
    PipelineStage,
    /// Skill definition capability.
    Skill,
    /// Memory backend capability.
    MemoryBackend,
    /// Reserved for Workstream G (voice/audio).
    Voice,
}

/// Permissions requested by a plugin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    /// Allowed network hosts. Empty = no network. `["*"]` = all hosts.
    #[serde(default)]
    pub network: Vec<String>,

    /// Allowed filesystem paths.
    #[serde(default)]
    pub filesystem: Vec<String>,

    /// Allowed environment variable names.
    #[serde(default)]
    pub env_vars: Vec<String>,

    /// Whether the plugin can execute shell commands.
    #[serde(default)]
    pub shell: bool,
}

/// Resource limits for plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResourceConfig {
    /// Maximum WASM fuel per invocation (default: 1,000,000,000).
    #[serde(default = "default_max_fuel")]
    pub max_fuel: u64,

    /// Maximum WASM memory in MB (default: 16).
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: usize,

    /// Maximum HTTP requests per minute (default: 10).
    #[serde(default = "default_max_http_rpm")]
    pub max_http_requests_per_minute: u64,

    /// Maximum log messages per minute (default: 100).
    #[serde(default = "default_max_log_rpm")]
    pub max_log_messages_per_minute: u64,

    /// Maximum execution wall-clock seconds (default: 30).
    #[serde(default = "default_max_exec_seconds")]
    pub max_execution_seconds: u64,

    /// Maximum WASM table elements (default: 10,000).
    #[serde(default = "default_max_table_elements")]
    pub max_table_elements: u32,
}

fn default_max_fuel() -> u64 {
    1_000_000_000
}
fn default_max_memory_mb() -> usize {
    16
}
fn default_max_http_rpm() -> u64 {
    10
}
fn default_max_log_rpm() -> u64 {
    100
}
fn default_max_exec_seconds() -> u64 {
    30
}
fn default_max_table_elements() -> u32 {
    10_000
}

impl Default for PluginResourceConfig {
    fn default() -> Self {
        Self {
            max_fuel: default_max_fuel(),
            max_memory_mb: default_max_memory_mb(),
            max_http_requests_per_minute: default_max_http_rpm(),
            max_log_messages_per_minute: default_max_log_rpm(),
            max_execution_seconds: default_max_exec_seconds(),
            max_table_elements: default_max_table_elements(),
        }
    }
}

impl PluginManifest {
    /// Validate the manifest. Returns an error describing the first
    /// validation failure, or `Ok(())` if the manifest is valid.
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.id.is_empty() {
            return Err(PluginError::LoadFailed(
                "manifest: id is required".into(),
            ));
        }
        if self.name.is_empty() {
            return Err(PluginError::LoadFailed(
                "manifest: name is required".into(),
            ));
        }
        // Validate semver
        if semver::Version::parse(&self.version).is_err() {
            return Err(PluginError::LoadFailed(format!(
                "manifest: invalid semver version '{}'",
                self.version
            )));
        }
        if self.capabilities.is_empty() {
            return Err(PluginError::LoadFailed(
                "manifest: at least one capability is required".into(),
            ));
        }
        Ok(())
    }

    /// Parse a manifest from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, PluginError> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse a manifest from a YAML string.
    ///
    /// Note: `serde_yaml` is NOT a dependency of `clawft-plugin` to keep the
    /// crate lightweight. YAML manifest parsing is handled in the loader
    /// layer (C3) which calls `serde_yaml::from_str()` and then constructs a
    /// `PluginManifest`. This method is a convenience stub.
    pub fn from_yaml(_yaml: &str) -> Result<Self, PluginError> {
        Err(PluginError::NotImplemented(
            "YAML manifest parsing deferred to C3 skill loader".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_json() -> String {
        serde_json::json!({
            "id": "com.example.test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "capabilities": ["tool", "skill"],
            "permissions": {
                "network": ["api.example.com"],
                "filesystem": ["/tmp/plugin"],
                "env_vars": ["MY_API_KEY"],
                "shell": false
            },
            "resources": {
                "max_fuel": 500_000_000u64,
                "max_memory_mb": 8,
                "max_http_requests_per_minute": 5,
                "max_log_messages_per_minute": 50,
                "max_execution_seconds": 15,
                "max_table_elements": 5000
            },
            "wasm_module": "plugin.wasm",
            "skills": ["code-review"],
            "tools": ["lint_code"]
        })
        .to_string()
    }

    #[test]
    fn test_manifest_parse_json() {
        let json = valid_manifest_json();
        let manifest = PluginManifest::from_json(&json).unwrap();
        assert_eq!(manifest.id, "com.example.test-plugin");
        assert_eq!(manifest.name, "Test Plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(manifest.capabilities[0], PluginCapability::Tool);
        assert_eq!(manifest.capabilities[1], PluginCapability::Skill);
        assert_eq!(manifest.permissions.network, vec!["api.example.com"]);
        assert_eq!(manifest.permissions.filesystem, vec!["/tmp/plugin"]);
        assert_eq!(manifest.permissions.env_vars, vec!["MY_API_KEY"]);
        assert!(!manifest.permissions.shell);
        assert_eq!(manifest.resources.max_fuel, 500_000_000);
        assert_eq!(manifest.resources.max_memory_mb, 8);
        assert_eq!(manifest.resources.max_http_requests_per_minute, 5);
        assert_eq!(manifest.resources.max_log_messages_per_minute, 50);
        assert_eq!(manifest.resources.max_execution_seconds, 15);
        assert_eq!(manifest.resources.max_table_elements, 5000);
        assert_eq!(manifest.wasm_module, Some("plugin.wasm".into()));
        assert_eq!(manifest.skills, vec!["code-review"]);
        assert_eq!(manifest.tools, vec!["lint_code"]);
    }

    #[test]
    fn test_manifest_parse_yaml_returns_not_implemented() {
        let result = PluginManifest::from_yaml("name: test");
        assert!(result.is_err());
        match result.unwrap_err() {
            PluginError::NotImplemented(msg) => {
                assert!(msg.contains("YAML manifest parsing deferred"));
            }
            other => panic!("expected NotImplemented, got: {other}"),
        }
    }

    #[test]
    fn test_manifest_missing_id_fails() {
        let json = serde_json::json!({
            "id": "",
            "name": "Test",
            "version": "1.0.0",
            "capabilities": ["tool"]
        })
        .to_string();
        let err = PluginManifest::from_json(&json).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("id is required"), "got: {msg}");
    }

    #[test]
    fn test_manifest_invalid_version_fails() {
        let json = serde_json::json!({
            "id": "com.test",
            "name": "Test",
            "version": "not-semver",
            "capabilities": ["tool"]
        })
        .to_string();
        let err = PluginManifest::from_json(&json).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid semver"), "got: {msg}");
    }

    #[test]
    fn test_manifest_empty_capabilities_fails() {
        let json = serde_json::json!({
            "id": "com.test",
            "name": "Test",
            "version": "1.0.0",
            "capabilities": []
        })
        .to_string();
        let err = PluginManifest::from_json(&json).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("at least one capability"),
            "got: {msg}"
        );
    }

    #[test]
    fn test_manifest_missing_name_fails() {
        let json = serde_json::json!({
            "id": "com.test",
            "name": "",
            "version": "1.0.0",
            "capabilities": ["tool"]
        })
        .to_string();
        let err = PluginManifest::from_json(&json).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("name is required"), "got: {msg}");
    }

    #[test]
    fn test_plugin_capability_serde_roundtrip() {
        let capabilities = vec![
            PluginCapability::Tool,
            PluginCapability::Channel,
            PluginCapability::PipelineStage,
            PluginCapability::Skill,
            PluginCapability::MemoryBackend,
            PluginCapability::Voice,
        ];
        for cap in &capabilities {
            let json = serde_json::to_string(cap).unwrap();
            let restored: PluginCapability = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, cap);
        }
    }

    #[test]
    fn test_plugin_capability_json_values() {
        assert_eq!(
            serde_json::to_string(&PluginCapability::Tool).unwrap(),
            "\"tool\""
        );
        assert_eq!(
            serde_json::to_string(&PluginCapability::Channel).unwrap(),
            "\"channel\""
        );
        assert_eq!(
            serde_json::to_string(&PluginCapability::PipelineStage).unwrap(),
            "\"pipeline_stage\""
        );
        assert_eq!(
            serde_json::to_string(&PluginCapability::Skill).unwrap(),
            "\"skill\""
        );
        assert_eq!(
            serde_json::to_string(&PluginCapability::MemoryBackend).unwrap(),
            "\"memory_backend\""
        );
        assert_eq!(
            serde_json::to_string(&PluginCapability::Voice).unwrap(),
            "\"voice\""
        );
    }

    #[test]
    fn test_permissions_default_is_empty() {
        let perms = PluginPermissions::default();
        assert!(perms.network.is_empty());
        assert!(perms.filesystem.is_empty());
        assert!(perms.env_vars.is_empty());
        assert!(!perms.shell);
    }

    #[test]
    fn test_resource_config_defaults() {
        let config = PluginResourceConfig::default();
        assert_eq!(config.max_fuel, 1_000_000_000);
        assert_eq!(config.max_memory_mb, 16);
        assert_eq!(config.max_http_requests_per_minute, 10);
        assert_eq!(config.max_log_messages_per_minute, 100);
        assert_eq!(config.max_execution_seconds, 30);
        assert_eq!(config.max_table_elements, 10_000);
    }

    #[test]
    fn test_manifest_with_defaults() {
        let json = serde_json::json!({
            "id": "com.test.minimal",
            "name": "Minimal",
            "version": "0.1.0",
            "capabilities": ["tool"]
        })
        .to_string();
        let manifest = PluginManifest::from_json(&json).unwrap();
        // Permissions default to empty
        assert!(manifest.permissions.network.is_empty());
        assert!(!manifest.permissions.shell);
        // Resources default to standard values
        assert_eq!(manifest.resources.max_fuel, 1_000_000_000);
        assert_eq!(manifest.resources.max_memory_mb, 16);
        // Optional fields default to None/empty
        assert!(manifest.wasm_module.is_none());
        assert!(manifest.skills.is_empty());
        assert!(manifest.tools.is_empty());
    }

    #[test]
    fn test_manifest_serde_roundtrip() {
        let json = valid_manifest_json();
        let manifest = PluginManifest::from_json(&json).unwrap();
        let serialized = serde_json::to_string(&manifest).unwrap();
        let restored = PluginManifest::from_json(&serialized).unwrap();
        assert_eq!(manifest.id, restored.id);
        assert_eq!(manifest.name, restored.name);
        assert_eq!(manifest.version, restored.version);
        assert_eq!(manifest.capabilities, restored.capabilities);
    }

    #[test]
    fn test_permissions_serde_roundtrip() {
        let perms = PluginPermissions {
            network: vec!["*.example.com".into(), "api.test.com".into()],
            filesystem: vec!["/tmp".into(), "/data".into()],
            env_vars: vec!["MY_KEY".into()],
            shell: true,
        };
        let json = serde_json::to_string(&perms).unwrap();
        let restored: PluginPermissions = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.network, perms.network);
        assert_eq!(restored.filesystem, perms.filesystem);
        assert_eq!(restored.env_vars, perms.env_vars);
        assert_eq!(restored.shell, perms.shell);
    }
}
