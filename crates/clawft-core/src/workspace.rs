//! Workspace discovery, lifecycle management, and merged config loading.
//!
//! Provides the 4-step workspace discovery algorithm and [`WorkspaceManager`]
//! for creating, listing, loading, and deleting workspaces.

use std::path::{Path, PathBuf};

use clawft_types::config::Config;
use clawft_types::workspace::{WorkspaceEntry, WorkspaceRegistry};
use clawft_types::{ClawftError, Result};

use crate::config_merge::{deep_merge, normalize_keys};

// ── Discovery ────────────────────────────────────────────────────────────

/// 4-step workspace discovery algorithm.
///
/// 1. `$CLAWFT_WORKSPACE` environment variable
/// 2. Walk from `cwd` upward looking for a `.clawft/` directory
/// 3. Fall back to `~/.clawft/` (global workspace)
///
/// Returns `None` only if the home directory cannot be determined.
pub fn discover_workspace() -> Option<PathBuf> {
    // Step 1: env var
    if let Ok(ws) = std::env::var("CLAWFT_WORKSPACE") {
        let path = PathBuf::from(&ws);
        if path.join(".clawft").is_dir() {
            return Some(path);
        }
    }

    // Step 2: walk cwd upward
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir: &Path = cwd.as_path();
        loop {
            if dir.join(".clawft").is_dir() {
                return Some(dir.to_path_buf());
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
    }

    // Step 3: global default
    dirs::home_dir().map(|h| h.join(".clawft"))
}

// ── Workspace status ─────────────────────────────────────────────────────

/// Summary of a workspace's current state.
pub struct WorkspaceStatus {
    /// Workspace display name.
    pub name: String,

    /// Absolute path to workspace root.
    pub path: PathBuf,

    /// Number of session files found in `.clawft/sessions/`.
    pub session_count: usize,

    /// Whether `.clawft/config.json` exists.
    pub has_config: bool,

    /// Whether `CLAWFT.md` exists at the workspace root.
    pub has_clawft_md: bool,
}

// ── WorkspaceManager ─────────────────────────────────────────────────────

/// The canonical subdirectories created inside `.clawft/`.
const WORKSPACE_SUBDIRS: &[&str] = &["sessions", "memory", "skills", "agents", "hooks"];

/// Manages workspace lifecycle (create, list, load, status, delete).
pub struct WorkspaceManager {
    /// Path to the global registry file (`~/.clawft/workspaces.json`).
    registry_path: PathBuf,

    /// In-memory copy of the registry.
    registry: WorkspaceRegistry,
}

impl WorkspaceManager {
    /// Create a new manager, loading the registry from the default path.
    ///
    /// The default registry path is `~/.clawft/workspaces.json`.
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| ClawftError::ConfigInvalid {
            reason: "cannot determine home directory".into(),
        })?;
        let registry_path = home.join(".clawft").join("workspaces.json");
        let registry =
            WorkspaceRegistry::load(&registry_path).map_err(|e| ClawftError::ConfigInvalid {
                reason: format!("failed to load workspace registry: {e}"),
            })?;

        Ok(Self {
            registry_path,
            registry,
        })
    }

    /// Create a new manager with an explicit registry path.
    ///
    /// Useful for testing.
    pub fn with_registry_path(registry_path: PathBuf) -> Result<Self> {
        let registry =
            WorkspaceRegistry::load(&registry_path).map_err(|e| ClawftError::ConfigInvalid {
                reason: format!("failed to load workspace registry: {e}"),
            })?;

        Ok(Self {
            registry_path,
            registry,
        })
    }

    /// Create a new workspace.
    ///
    /// Creates:
    /// 1. `.clawft/` and subdirectories (`sessions`, `memory`, `skills`,
    ///    `agents`, `hooks`)
    /// 2. `.clawft/config.json` with `{}`
    /// 3. `CLAWFT.md` with a starter template
    /// 4. Registers the workspace in the global registry
    ///
    /// Returns the absolute path to the workspace root.
    pub fn create(&mut self, name: &str, parent_dir: &Path) -> Result<PathBuf> {
        let ws_root = parent_dir.join(name);
        let dot_clawft = ws_root.join(".clawft");

        // Create .clawft/ and subdirectories
        for subdir in WORKSPACE_SUBDIRS {
            std::fs::create_dir_all(dot_clawft.join(subdir))?;
        }

        // Create config.json
        std::fs::write(dot_clawft.join("config.json"), "{}\n")?;

        // Create MEMORY.md and HISTORY.md (empty)
        std::fs::write(dot_clawft.join("MEMORY.md"), "")?;
        std::fs::write(dot_clawft.join("HISTORY.md"), "")?;

        // Create CLAWFT.md
        let clawft_md = format!(
            "# {name}\n\n\
             Workspace created by clawft.\n\n\
             ## Configuration\n\n\
             Edit `.clawft/config.json` to customize this workspace.\n"
        );
        std::fs::write(ws_root.join("CLAWFT.md"), clawft_md)?;

        // Register in global registry
        let now = chrono::Utc::now().to_rfc3339();
        let entry = WorkspaceEntry {
            name: name.into(),
            path: ws_root.clone(),
            last_accessed: Some(now.clone()),
            created_at: Some(now),
        };
        self.registry.register(entry);
        self.save_registry()?;

        Ok(ws_root)
    }

    /// List all registered workspaces.
    pub fn list(&self) -> Vec<&WorkspaceEntry> {
        self.registry.workspaces.iter().collect()
    }

    /// Load a workspace by name or path string.
    ///
    /// Returns the workspace root path if found.
    pub fn load(&mut self, name_or_path: &str) -> Result<PathBuf> {
        // Try by name first
        if let Some(entry) = self.registry.find_by_name(name_or_path) {
            return Ok(entry.path.clone());
        }

        // Try as a path
        let path = PathBuf::from(name_or_path);
        if path.join(".clawft").is_dir() {
            return Ok(path);
        }

        Err(ClawftError::ConfigInvalid {
            reason: format!("workspace not found: {name_or_path}"),
        })
    }

    /// Get the status of a workspace at the given path.
    pub fn status(&self, path: &Path) -> Result<WorkspaceStatus> {
        let dot_clawft = path.join(".clawft");
        let name = self
            .registry
            .find_by_path(path)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| {
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".into())
            });

        let sessions_dir = dot_clawft.join("sessions");
        let session_count = if sessions_dir.is_dir() {
            std::fs::read_dir(&sessions_dir)
                .map(|rd| rd.count())
                .unwrap_or(0)
        } else {
            0
        };

        Ok(WorkspaceStatus {
            name,
            path: path.to_path_buf(),
            session_count,
            has_config: dot_clawft.join("config.json").exists(),
            has_clawft_md: path.join("CLAWFT.md").exists(),
        })
    }

    /// Delete a workspace by name.
    ///
    /// Removes the entry from the registry but does NOT delete files
    /// from disk (that is the caller's responsibility).
    pub fn delete(&mut self, name: &str) -> Result<()> {
        if !self.registry.remove_by_name(name) {
            return Err(ClawftError::ConfigInvalid {
                reason: format!("workspace not found: {name}"),
            });
        }
        self.save_registry()?;
        Ok(())
    }

    /// Persist the registry to disk.
    fn save_registry(&self) -> Result<()> {
        self.registry
            .save(&self.registry_path)
            .map_err(|e| ClawftError::ConfigInvalid {
                reason: format!("failed to save workspace registry: {e}"),
            })
    }
}

// ── Merged config loading ────────────────────────────────────────────────

/// Load a config file as a raw JSON [`serde_json::Value`].
///
/// Returns `None` if the file does not exist.
fn load_config_file(path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Load config with 3-level merge: defaults < global < workspace.
///
/// 1. Start with [`Config::default()`] serialized to JSON.
/// 2. Merge `~/.clawft/config.json` (global overrides).
/// 3. Merge `<workspace>/.clawft/config.json` (workspace overrides).
/// 4. Deserialize the merged JSON back into [`Config`].
pub fn load_merged_config(workspace_path: Option<&Path>) -> Result<Config> {
    let global_config = dirs::home_dir().map(|h| h.join(".clawft").join("config.json"));
    load_merged_config_from(global_config.as_deref(), workspace_path)
}

/// Load config with 3-level merge from explicit paths.
///
/// This is the internal implementation that accepts explicit file paths
/// for both global and workspace config, enabling deterministic testing.
pub fn load_merged_config_from(
    global_config_path: Option<&Path>,
    workspace_path: Option<&Path>,
) -> Result<Config> {
    let defaults = Config::default();
    let mut merged = serde_json::to_value(&defaults).map_err(|e| ClawftError::ConfigInvalid {
        reason: format!("failed to serialize defaults: {e}"),
    })?;

    // Global config
    if let Some(gp) = global_config_path
        && let Some(mut global) = load_config_file(gp)
    {
        normalize_keys(&mut global);
        deep_merge(&mut merged, &global);
    }

    // Workspace config: <workspace>/.clawft/config.json
    if let Some(ws_path) = workspace_path
        && let Some(mut ws_config) = load_config_file(&ws_path.join(".clawft").join("config.json"))
    {
        normalize_keys(&mut ws_config);
        deep_merge(&mut merged, &ws_config);
    }

    serde_json::from_value(merged).map_err(ClawftError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Monotonic counter to give each test a unique temp directory.
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    // ── Discovery tests ──────────────────────────────────────────────

    #[test]
    fn discover_workspace_returns_some() {
        // The exact result depends on the environment (cwd might have .clawft/
        // above it, or we fall back to ~/.clawft). Either way, we should always
        // get Some on a system with a home directory.
        let result = discover_workspace();
        assert!(result.is_some(), "discover_workspace should return Some");
    }

    #[test]
    fn discover_workspace_env_var() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-discover-env-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".clawft")).unwrap();

        // SAFETY: test-only, single-threaded context.
        unsafe { std::env::set_var("CLAWFT_WORKSPACE", dir.to_str().unwrap()) };
        let result = discover_workspace();
        unsafe { std::env::remove_var("CLAWFT_WORKSPACE") };

        assert_eq!(result, Some(dir.clone()));

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_workspace_env_var_invalid_skipped() {
        // SAFETY: test-only, single-threaded context.
        unsafe { std::env::set_var("CLAWFT_WORKSPACE", "/nonexistent/path/for/test") };
        let result = discover_workspace();
        unsafe { std::env::remove_var("CLAWFT_WORKSPACE") };

        // Should still return something (the fallback), not the invalid path.
        assert!(result.is_some());
        assert_ne!(result.unwrap(), PathBuf::from("/nonexistent/path/for/test"));
    }

    // ── WorkspaceManager tests ───────────────────────────────────────

    /// Create a unique temp directory and registry path for each test.
    fn temp_registry(label: &str) -> (PathBuf, PathBuf) {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-wm-{label}-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let registry_path = dir.join("workspaces.json");
        (dir, registry_path)
    }

    #[test]
    fn workspace_manager_create_directories() {
        let (dir, registry_path) = temp_registry("create-dirs");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        let ws_path = wm.create("test-ws", &dir).unwrap();
        let dot_clawft = ws_path.join(".clawft");

        // Verify all subdirectories
        for subdir in WORKSPACE_SUBDIRS {
            assert!(dot_clawft.join(subdir).is_dir(), "missing subdir: {subdir}");
        }

        // Verify files
        assert!(dot_clawft.join("config.json").exists());
        assert!(dot_clawft.join("MEMORY.md").exists());
        assert!(dot_clawft.join("HISTORY.md").exists());
        assert!(ws_path.join("CLAWFT.md").exists());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_create_registers_in_registry() {
        let (dir, registry_path) = temp_registry("create-reg");
        let mut wm = WorkspaceManager::with_registry_path(registry_path.clone()).unwrap();

        wm.create("reg-test", &dir).unwrap();

        // Reload registry from disk and verify
        let loaded = WorkspaceRegistry::load(&registry_path).unwrap();
        assert!(loaded.find_by_name("reg-test").is_some());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_list() {
        let (dir, registry_path) = temp_registry("list");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        assert!(wm.list().is_empty(), "fresh registry should be empty");

        wm.create("ws-a", &dir).unwrap();
        wm.create("ws-b", &dir).unwrap();

        let list = wm.list();
        assert_eq!(list.len(), 2);

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_load_by_name() {
        let (dir, registry_path) = temp_registry("load-name");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        let created = wm.create("load-test", &dir).unwrap();
        let loaded = wm.load("load-test").unwrap();
        assert_eq!(created, loaded);

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_load_by_path() {
        let (dir, registry_path) = temp_registry("load-path");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        let created = wm.create("path-test", &dir).unwrap();
        let loaded = wm.load(created.to_str().unwrap()).unwrap();
        assert_eq!(created, loaded);

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_load_not_found() {
        let (dir, registry_path) = temp_registry("load-notfound");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        let result = wm.load("nonexistent");
        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_status() {
        let (dir, registry_path) = temp_registry("status");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        let ws_path = wm.create("status-test", &dir).unwrap();
        let status = wm.status(&ws_path).unwrap();

        assert_eq!(status.name, "status-test");
        assert_eq!(status.path, ws_path);
        assert_eq!(status.session_count, 0);
        assert!(status.has_config);
        assert!(status.has_clawft_md);

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_delete() {
        let (dir, registry_path) = temp_registry("delete");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        wm.create("del-test", &dir).unwrap();
        assert_eq!(wm.list().len(), 1);

        wm.delete("del-test").unwrap();
        assert!(wm.list().is_empty());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_manager_delete_not_found() {
        let (dir, registry_path) = temp_registry("delete-notfound");
        let mut wm = WorkspaceManager::with_registry_path(registry_path).unwrap();

        let result = wm.delete("nonexistent");
        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Config merge tests ───────────────────────────────────────────
    //
    // These use `load_merged_config_from` with explicit paths to avoid
    // contamination from the host's `~/.clawft/config.json`.

    #[test]
    fn load_merged_config_defaults_only() {
        // No global config, no workspace config => pure defaults.
        let config = load_merged_config_from(None, None).unwrap();
        assert_eq!(config.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert_eq!(config.agents.defaults.max_tokens, 8192);
    }

    #[test]
    fn load_merged_config_workspace_overrides() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-ws-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        let dot_clawft = dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();

        // Write workspace config that overrides max_tokens
        let ws_config = r#"{"agents": {"defaults": {"max_tokens": 4096}}}"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        // No global config => only defaults + workspace
        let config = load_merged_config_from(None, Some(&dir)).unwrap();
        assert_eq!(config.agents.defaults.max_tokens, 4096); // overridden
        assert_eq!(config.agents.defaults.model, "anthropic/claude-opus-4-5"); // from defaults

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_global_overrides() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-global-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Write a fake global config
        let global_config = r#"{"agents": {"defaults": {"model": "custom/model"}}}"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), None).unwrap();
        assert_eq!(config.agents.defaults.model, "custom/model"); // overridden
        assert_eq!(config.agents.defaults.max_tokens, 8192); // from defaults

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_workspace_over_global() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-both-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Global sets model
        let global_config =
            r#"{"agents": {"defaults": {"model": "global/model", "max_tokens": 2048}}}"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        // Workspace overrides max_tokens
        let ws_dir = dir.join("workspace");
        let dot_clawft = ws_dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();
        let ws_config = r#"{"agents": {"defaults": {"max_tokens": 4096}}}"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), Some(&ws_dir)).unwrap();
        assert_eq!(config.agents.defaults.model, "global/model"); // from global
        assert_eq!(config.agents.defaults.max_tokens, 4096); // workspace wins

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_missing_workspace_config() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-missing-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // No .clawft/config.json exists, no global config
        let config = load_merged_config_from(None, Some(&dir)).unwrap();
        // Should still return defaults
        assert_eq!(config.agents.defaults.max_tokens, 8192);

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_normalizes_keys() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-normalize-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Global config uses snake_case: max_tokens = 1000
        let global_config = r#"{"agents": {"defaults": {"max_tokens": 1000}}}"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        // Workspace config uses camelCase: maxTokens = 2000
        let ws_dir = dir.join("workspace");
        let dot_clawft = ws_dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();
        let ws_config = r#"{"agents": {"defaults": {"maxTokens": 2000}}}"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), Some(&ws_dir)).unwrap();
        // Workspace camelCase "maxTokens" should override global "max_tokens"
        assert_eq!(config.agents.defaults.max_tokens, 2000);

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_mcp_servers() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-mcp-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Global config defines two MCP servers (using camelCase mcpServers)
        let global_config = r#"{
            "tools": {
                "mcpServers": {
                    "github": {"command": "npx", "args": ["-y", "github-mcp"]},
                    "slack": {"command": "npx", "args": ["-y", "slack-mcp"]}
                }
            }
        }"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        // Workspace adds rvf, removes slack (null), leaves github untouched
        let ws_dir = dir.join("workspace");
        let dot_clawft = ws_dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();
        let ws_config = r#"{
            "tools": {
                "mcpServers": {
                    "rvf": {"command": "npx", "args": ["-y", "rvf-mcp"]},
                    "slack": null
                }
            }
        }"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), Some(&ws_dir)).unwrap();

        // github preserved from global
        assert!(
            config.tools.mcp_servers.contains_key("github"),
            "github server should be preserved"
        );
        assert_eq!(config.tools.mcp_servers["github"].command, "npx");

        // rvf added from workspace
        assert!(
            config.tools.mcp_servers.contains_key("rvf"),
            "rvf server should be added"
        );
        assert_eq!(config.tools.mcp_servers["rvf"].command, "npx");

        // slack removed (null in workspace overlay)
        assert!(
            !config.tools.mcp_servers.contains_key("slack"),
            "slack server should be removed by null overlay"
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }
}
