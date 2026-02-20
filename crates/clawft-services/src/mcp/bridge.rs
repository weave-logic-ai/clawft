//! Bidirectional MCP bridge between clawft and Claude Code.
//!
//! The bridge orchestrates both directions of MCP communication:
//!
//! - **Outbound (clawft -> Claude Code)**: clawft's MCP `server.rs` exposes
//!   tools to Claude Code. Claude Code registers clawft as an MCP server
//!   via `claude mcp add`.
//!
//! - **Inbound (Claude Code -> clawft)**: clawft connects to Claude Code's
//!   MCP server as a client, making Claude Code's tools available in clawft.
//!
//! # Hot-reload
//!
//! The bridge supports hot-reload via the drain-and-swap protocol defined
//! in `discovery.rs`. When bridge configuration changes, the old connection
//! is drained and a new one established.
//!
//! # Configuration
//!
//! ```toml
//! [tools.mcp_servers.claude-code]
//! command = "claude"
//! args = ["mcp", "serve"]
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Bridge status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeStatus {
    /// Bridge is not configured.
    Unconfigured,
    /// Bridge is initializing.
    Initializing,
    /// Bridge is active (both directions working).
    Active,
    /// Outbound only (clawft -> Claude Code).
    OutboundOnly,
    /// Inbound only (Claude Code -> clawft).
    InboundOnly,
    /// Bridge encountered an error.
    Error,
    /// Bridge is shutting down.
    ShuttingDown,
}

/// Configuration for the MCP bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Whether the bridge is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Command to start Claude Code's MCP server.
    #[serde(default = "default_claude_command")]
    pub claude_command: String,

    /// Arguments for the Claude Code MCP server.
    #[serde(default = "default_claude_args")]
    pub claude_args: Vec<String>,

    /// Optional environment variables for the Claude Code process.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Tool namespace prefix for Claude Code tools in clawft.
    /// Tools are exposed as `mcp:claude-code:<tool-name>`.
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_claude_command() -> String {
    "claude".into()
}

fn default_claude_args() -> Vec<String> {
    vec!["mcp".into(), "serve".into()]
}

fn default_namespace() -> String {
    "claude-code".into()
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            claude_command: default_claude_command(),
            claude_args: default_claude_args(),
            env: HashMap::new(),
            namespace: default_namespace(),
        }
    }
}

/// Bidirectional MCP bridge manager.
///
/// Manages the lifecycle of the clawft <-> Claude Code MCP connection.
pub struct McpBridge {
    config: BridgeConfig,
    status: BridgeStatus,
    /// Tools discovered from Claude Code's MCP server.
    inbound_tools: Vec<String>,
    /// Tools exposed by clawft to Claude Code.
    outbound_tools: Vec<String>,
}

impl McpBridge {
    /// Create a new bridge with the given configuration.
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            status: BridgeStatus::Unconfigured,
            inbound_tools: Vec::new(),
            outbound_tools: Vec::new(),
        }
    }

    /// Create a disabled bridge.
    pub fn disabled() -> Self {
        Self::new(BridgeConfig::default())
    }

    /// Get the current bridge status.
    pub fn status(&self) -> BridgeStatus {
        self.status
    }

    /// Get the bridge configuration.
    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }

    /// Whether the bridge is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Initialize the bridge.
    ///
    /// This sets up the outbound direction (clawft -> Claude Code) by
    /// registering outbound tools, and prepares for inbound connection.
    ///
    /// # Note
    ///
    /// Full MCP client connection depends on Element 07/F9a providing
    /// the `McpClient::connect()` implementation. Until then, this
    /// method sets up the configuration and marks the bridge as ready
    /// for connection.
    pub fn initialize(&mut self, outbound_tools: Vec<String>) {
        if !self.config.enabled {
            debug!("bridge not enabled, skipping initialization");
            return;
        }

        self.outbound_tools = outbound_tools;
        self.status = BridgeStatus::Initializing;
        info!(
            outbound_tool_count = self.outbound_tools.len(),
            namespace = %self.config.namespace,
            "MCP bridge initializing"
        );
    }

    /// Mark the inbound connection as active with discovered tools.
    pub fn set_inbound_connected(&mut self, tools: Vec<String>) {
        self.inbound_tools = tools;
        self.update_status();
        info!(
            inbound_tool_count = self.inbound_tools.len(),
            "inbound MCP connection active"
        );
    }

    /// Mark the outbound connection as active.
    pub fn set_outbound_connected(&mut self) {
        self.update_status();
        info!("outbound MCP connection active");
    }

    /// Set error status with a reason.
    pub fn set_error(&mut self, reason: &str) {
        self.status = BridgeStatus::Error;
        warn!(reason = %reason, "MCP bridge error");
    }

    /// Shut down the bridge.
    pub fn shutdown(&mut self) {
        self.status = BridgeStatus::ShuttingDown;
        self.inbound_tools.clear();
        info!("MCP bridge shutting down");
    }

    /// Tools discovered from Claude Code (inbound).
    pub fn inbound_tools(&self) -> &[String] {
        &self.inbound_tools
    }

    /// Tools exposed to Claude Code (outbound).
    pub fn outbound_tools(&self) -> &[String] {
        &self.outbound_tools
    }

    /// Get the namespaced tool name for an inbound Claude Code tool.
    ///
    /// Returns `mcp:<namespace>:<tool_name>`.
    pub fn namespaced_tool_name(&self, tool_name: &str) -> String {
        format!("mcp:{}:{}", self.config.namespace, tool_name)
    }

    /// Update status based on connection state.
    fn update_status(&mut self) {
        let has_inbound = !self.inbound_tools.is_empty();
        let has_outbound = !self.outbound_tools.is_empty();

        self.status = match (has_inbound, has_outbound) {
            (true, true) => BridgeStatus::Active,
            (true, false) => BridgeStatus::InboundOnly,
            (false, true) => BridgeStatus::OutboundOnly,
            (false, false) => BridgeStatus::Initializing,
        };
    }
}

impl std::fmt::Debug for McpBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpBridge")
            .field("status", &self.status)
            .field("enabled", &self.config.enabled)
            .field("inbound_tools", &self.inbound_tools.len())
            .field("outbound_tools", &self.outbound_tools.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_disabled_by_default() {
        let bridge = McpBridge::disabled();
        assert!(!bridge.is_enabled());
        assert_eq!(bridge.status(), BridgeStatus::Unconfigured);
    }

    #[test]
    fn bridge_initialize() {
        let mut bridge = McpBridge::new(BridgeConfig {
            enabled: true,
            ..Default::default()
        });

        bridge.initialize(vec!["read_file".into(), "write_file".into()]);
        assert_eq!(bridge.status(), BridgeStatus::Initializing);
        assert_eq!(bridge.outbound_tools().len(), 2);
    }

    #[test]
    fn bridge_skip_init_when_disabled() {
        let mut bridge = McpBridge::disabled();
        bridge.initialize(vec!["tool1".into()]);
        // Status remains unconfigured because bridge is disabled.
        assert_eq!(bridge.status(), BridgeStatus::Unconfigured);
        assert!(bridge.outbound_tools().is_empty());
    }

    #[test]
    fn bridge_active_when_both_connected() {
        let mut bridge = McpBridge::new(BridgeConfig {
            enabled: true,
            ..Default::default()
        });
        bridge.initialize(vec!["out_tool".into()]);
        bridge.set_inbound_connected(vec!["in_tool".into()]);
        assert_eq!(bridge.status(), BridgeStatus::Active);
    }

    #[test]
    fn bridge_inbound_only() {
        let mut bridge = McpBridge::new(BridgeConfig {
            enabled: true,
            ..Default::default()
        });
        // No outbound tools, but inbound connected.
        bridge.set_inbound_connected(vec!["tool1".into()]);
        assert_eq!(bridge.status(), BridgeStatus::InboundOnly);
    }

    #[test]
    fn bridge_outbound_only() {
        let mut bridge = McpBridge::new(BridgeConfig {
            enabled: true,
            ..Default::default()
        });
        bridge.initialize(vec!["tool1".into()]);
        bridge.set_outbound_connected();
        assert_eq!(bridge.status(), BridgeStatus::OutboundOnly);
    }

    #[test]
    fn bridge_error() {
        let mut bridge = McpBridge::new(BridgeConfig {
            enabled: true,
            ..Default::default()
        });
        bridge.set_error("connection refused");
        assert_eq!(bridge.status(), BridgeStatus::Error);
    }

    #[test]
    fn bridge_shutdown() {
        let mut bridge = McpBridge::new(BridgeConfig {
            enabled: true,
            ..Default::default()
        });
        bridge.initialize(vec!["tool".into()]);
        bridge.set_inbound_connected(vec!["in".into()]);
        bridge.shutdown();
        assert_eq!(bridge.status(), BridgeStatus::ShuttingDown);
        assert!(bridge.inbound_tools().is_empty());
    }

    #[test]
    fn namespaced_tool_name() {
        let bridge = McpBridge::new(BridgeConfig {
            namespace: "claude-code".into(),
            ..Default::default()
        });
        assert_eq!(
            bridge.namespaced_tool_name("read_file"),
            "mcp:claude-code:read_file"
        );
    }

    #[test]
    fn bridge_config_defaults() {
        let cfg = BridgeConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.claude_command, "claude");
        assert_eq!(cfg.claude_args, vec!["mcp", "serve"]);
        assert_eq!(cfg.namespace, "claude-code");
    }

    #[test]
    fn bridge_config_serde() {
        let cfg = BridgeConfig {
            enabled: true,
            claude_command: "claude-dev".into(),
            claude_args: vec!["mcp".into(), "start".into()],
            env: {
                let mut m = HashMap::new();
                m.insert("CLAUDE_KEY".into(), "test".into());
                m
            },
            namespace: "dev".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: BridgeConfig = serde_json::from_str(&json).unwrap();
        assert!(restored.enabled);
        assert_eq!(restored.claude_command, "claude-dev");
        assert_eq!(restored.namespace, "dev");
    }

    #[test]
    fn bridge_status_serde() {
        let json = serde_json::to_string(&BridgeStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");

        let restored: BridgeStatus =
            serde_json::from_str("\"shutting_down\"").unwrap();
        assert_eq!(restored, BridgeStatus::ShuttingDown);
    }
}
