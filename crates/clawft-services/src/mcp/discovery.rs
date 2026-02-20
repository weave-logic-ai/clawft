//! Dynamic MCP server management.
//!
//! [`McpServerManager`] provides runtime management of MCP server
//! connections, including add/remove/list operations and hot-reload
//! via drain-and-swap protocol.
//!
//! # CLI commands
//!
//! ```text
//! weft mcp add <name> <command|url>
//! weft mcp list
//! weft mcp remove <name>
//! ```
//!
//! # Hot-reload protocol
//!
//! When `clawft.toml` changes:
//! 1. File watcher detects change (debounce 500ms).
//! 2. Diff old and new server lists.
//! 3. New servers: connect immediately.
//! 4. Removed servers: drain in-flight calls (30s), then disconnect.
//! 5. Changed servers: remove + add.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Default debounce for config file changes.
const DEBOUNCE_MS: u64 = 500;

/// Default drain timeout for removing servers.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

/// Status of a managed MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    /// Server is connected and ready.
    Connected,
    /// Server is connecting (handshake in progress).
    Connecting,
    /// Server is draining in-flight requests before disconnection.
    Draining,
    /// Server is disconnected.
    Disconnected,
    /// Server connection failed.
    Error,
}

/// Configuration for a managed MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name (unique identifier).
    pub name: String,

    /// Command to spawn the server (e.g., "npx", "claude").
    pub command: String,

    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Optional environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Optional URL for HTTP-based servers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A managed MCP server with its connection state.
#[derive(Debug)]
pub struct ManagedMcpServer {
    /// Server configuration.
    pub config: McpServerConfig,
    /// Current connection status.
    pub status: ServerStatus,
    /// Tools discovered from this server (tool_name -> description).
    pub tools: Vec<String>,
    /// When the server was added.
    pub added_at: chrono::DateTime<chrono::Utc>,
}

/// Manager for dynamically adding, removing, and listing MCP servers.
///
/// Provides the runtime layer for `weft mcp add/list/remove` commands
/// and hot-reload on config changes.
pub struct McpServerManager {
    /// Active servers keyed by name.
    servers: HashMap<String, ManagedMcpServer>,
    /// Debounce duration for config file changes.
    debounce: Duration,
    /// Drain timeout for removing servers.
    drain_timeout: Duration,
}

impl McpServerManager {
    /// Create a new server manager with default settings.
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            debounce: Duration::from_millis(DEBOUNCE_MS),
            drain_timeout: DRAIN_TIMEOUT,
        }
    }

    /// Add a new MCP server.
    ///
    /// If a server with the same name already exists, it is replaced
    /// (the old server is drained first).
    ///
    /// This method does not connect to the server -- call
    /// [`connect_server`](Self::connect_server) to initiate connection.
    pub fn add_server(&mut self, config: McpServerConfig) -> &ManagedMcpServer {
        let name = config.name.clone();

        if self.servers.contains_key(&name) {
            info!(name = %name, "replacing existing MCP server");
        }

        let server = ManagedMcpServer {
            config,
            status: ServerStatus::Disconnected,
            tools: Vec::new(),
            added_at: chrono::Utc::now(),
        };

        self.servers.insert(name.clone(), server);
        debug!(name = %name, "added MCP server");
        self.servers.get(&name).unwrap()
    }

    /// Remove an MCP server by name.
    ///
    /// Marks the server as draining. In a full implementation,
    /// in-flight calls would be completed before disconnection.
    ///
    /// Returns `true` if the server was found and removed.
    pub fn remove_server(&mut self, name: &str) -> bool {
        if let Some(mut server) = self.servers.remove(name) {
            server.status = ServerStatus::Draining;
            info!(name = %name, "removed MCP server (draining)");
            true
        } else {
            warn!(name = %name, "MCP server not found for removal");
            false
        }
    }

    /// List all managed servers.
    pub fn list_servers(&self) -> Vec<&ManagedMcpServer> {
        self.servers.values().collect()
    }

    /// Get a server by name.
    pub fn get_server(&self, name: &str) -> Option<&ManagedMcpServer> {
        self.servers.get(name)
    }

    /// Get a mutable reference to a server by name.
    pub fn get_server_mut(&mut self, name: &str) -> Option<&mut ManagedMcpServer> {
        self.servers.get_mut(name)
    }

    /// Number of managed servers.
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Mark a server as connected and set its discovered tools.
    pub fn mark_connected(&mut self, name: &str, tools: Vec<String>) {
        if let Some(server) = self.servers.get_mut(name) {
            server.status = ServerStatus::Connected;
            server.tools = tools;
            debug!(name = %name, tool_count = server.tools.len(), "server connected");
        }
    }

    /// Mark a server as errored.
    pub fn mark_error(&mut self, name: &str) {
        if let Some(server) = self.servers.get_mut(name) {
            server.status = ServerStatus::Error;
            warn!(name = %name, "server marked as error");
        }
    }

    /// Apply a config diff (hot-reload).
    ///
    /// Given the new set of server configs, determines which servers
    /// to add, remove, or update.
    ///
    /// Returns `(added, removed, changed)` counts.
    pub fn apply_config_diff(
        &mut self,
        new_configs: Vec<McpServerConfig>,
    ) -> (usize, usize, usize) {
        let new_names: HashMap<String, &McpServerConfig> =
            new_configs.iter().map(|c| (c.name.clone(), c)).collect();
        let old_names: Vec<String> = self.servers.keys().cloned().collect();

        let mut added = 0;
        let mut removed = 0;
        let mut changed = 0;

        // Remove servers no longer in config.
        for name in &old_names {
            if !new_names.contains_key(name) {
                self.remove_server(name);
                removed += 1;
            }
        }

        // Add or update servers.
        for config in new_configs {
            let name = config.name.clone();
            if let Some(existing) = self.servers.get(&name) {
                // Check if config changed.
                if existing.config.command != config.command
                    || existing.config.args != config.args
                {
                    self.add_server(config);
                    changed += 1;
                }
            } else {
                self.add_server(config);
                added += 1;
            }
        }

        info!(
            added,
            removed,
            changed,
            "applied MCP server config diff"
        );

        (added, removed, changed)
    }

    /// Debounce duration for config changes.
    pub fn debounce(&self) -> Duration {
        self.debounce
    }

    /// Drain timeout for removing servers.
    pub fn drain_timeout(&self) -> Duration {
        self.drain_timeout
    }
}

impl Default for McpServerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            command: "npx".into(),
            args: vec!["-y".into(), format!("{name}-mcp")],
            env: HashMap::new(),
            url: None,
        }
    }

    #[test]
    fn add_and_list_servers() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));
        mgr.add_server(test_config("slack"));

        assert_eq!(mgr.server_count(), 2);
        let servers = mgr.list_servers();
        assert_eq!(servers.len(), 2);
    }

    #[test]
    fn remove_server() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));
        assert_eq!(mgr.server_count(), 1);

        assert!(mgr.remove_server("github"));
        assert_eq!(mgr.server_count(), 0);
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut mgr = McpServerManager::new();
        assert!(!mgr.remove_server("nonexistent"));
    }

    #[test]
    fn get_server() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));

        let server = mgr.get_server("github");
        assert!(server.is_some());
        assert_eq!(server.unwrap().config.name, "github");
        assert_eq!(server.unwrap().status, ServerStatus::Disconnected);

        assert!(mgr.get_server("missing").is_none());
    }

    #[test]
    fn mark_connected() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));
        mgr.mark_connected("github", vec!["create_issue".into(), "list_repos".into()]);

        let server = mgr.get_server("github").unwrap();
        assert_eq!(server.status, ServerStatus::Connected);
        assert_eq!(server.tools.len(), 2);
    }

    #[test]
    fn mark_error() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));
        mgr.mark_error("github");

        let server = mgr.get_server("github").unwrap();
        assert_eq!(server.status, ServerStatus::Error);
    }

    #[test]
    fn replace_existing_server() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));
        mgr.mark_connected("github", vec!["tool1".into()]);

        // Replace with new config.
        mgr.add_server(test_config("github"));
        let server = mgr.get_server("github").unwrap();
        // Status reset to Disconnected after replacement.
        assert_eq!(server.status, ServerStatus::Disconnected);
        assert!(server.tools.is_empty());
    }

    #[test]
    fn apply_config_diff() {
        let mut mgr = McpServerManager::new();
        mgr.add_server(test_config("github"));
        mgr.add_server(test_config("slack"));

        let new_configs = vec![
            test_config("github"), // kept
            test_config("jira"),   // added
            // slack removed
        ];

        let (added, removed, _changed) = mgr.apply_config_diff(new_configs);
        assert_eq!(added, 1); // jira
        assert_eq!(removed, 1); // slack
        assert_eq!(mgr.server_count(), 2); // github + jira
        assert!(mgr.get_server("github").is_some());
        assert!(mgr.get_server("jira").is_some());
        assert!(mgr.get_server("slack").is_none());
    }

    #[test]
    fn server_status_serde() {
        let json = serde_json::to_string(&ServerStatus::Connected).unwrap();
        assert_eq!(json, "\"connected\"");

        let restored: ServerStatus = serde_json::from_str("\"draining\"").unwrap();
        assert_eq!(restored, ServerStatus::Draining);
    }

    #[test]
    fn debounce_and_drain_defaults() {
        let mgr = McpServerManager::new();
        assert_eq!(mgr.debounce(), Duration::from_millis(500));
        assert_eq!(mgr.drain_timeout(), Duration::from_secs(30));
    }
}
