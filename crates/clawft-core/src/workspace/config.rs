//! Merged / split config loading with 3-level precedence.
//!
//! Implements the config merge strategy:
//! 1. Start with [`Config::default()`]
//! 2. Merge `~/.clawft/config.json` (global overrides)
//! 3. Keep `<workspace>/.clawft/config.json` as a separate overlay
//!
//! WEFT-10 / FIX-04: callers that construct a `PermissionResolver` must
//! use the split form ([`load_split_config`] / [`load_split_config_from`])
//! so `enforce_workspace_ceiling` can clamp elevated workspace grants.
//! [`load_merged_config`] remains for non-security consumers that only
//! need most-specific-wins deep merge.
//!
//! Both `camelCase` and `snake_case` keys are supported via normalization.

use std::path::Path;

use clawft_types::config::Config;
use clawft_types::routing::RoutingConfig;
use clawft_types::{ClawftError, Result};

use crate::config_merge::{deep_merge, normalize_keys};

/// Split config layers: global ceiling + optional workspace overlay.
///
/// - [`Self::global`]: defaults + home/env JSON (system-wide bounds).
/// - [`Self::workspace`]: workspace-only overlay config when present.
/// - [`Self::merged`]: deep-merge of both (agents/tools/etc.).
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// System-wide config (defaults + global file). Ceiling for permissions.
    pub global: Config,
    /// Workspace overlay alone, when a workspace config file was found.
    pub workspace: Option<Config>,
    /// Deep-merged config for non-security fields (most-specific wins).
    pub merged: Config,
}

impl SplitConfig {
    /// Routing layer for the global (ceiling) side of `PermissionResolver`.
    pub fn global_routing(&self) -> &RoutingConfig {
        &self.global.routing
    }

    /// Routing layer for the workspace side of `PermissionResolver`.
    pub fn workspace_routing(&self) -> Option<&RoutingConfig> {
        self.workspace.as_ref().map(|c| &c.routing)
    }
}

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
///
/// Prefer [`load_split_config`] when wiring `PermissionResolver`.
pub fn load_merged_config(workspace_path: Option<&Path>) -> Result<Config> {
    Ok(load_split_config(workspace_path)?.merged)
}

/// Load split config layers from the default global path + workspace path.
pub fn load_split_config(workspace_path: Option<&Path>) -> Result<SplitConfig> {
    #[cfg(feature = "native")]
    let global_config = dirs::home_dir().map(|h| h.join(".clawft").join("config.json"));
    #[cfg(not(feature = "native"))]
    let global_config: Option<std::path::PathBuf> = None;
    load_split_config_from(global_config.as_deref(), workspace_path)
}

/// Load config with 3-level merge from explicit paths.
///
/// This is the internal implementation that accepts explicit file paths
/// for both global and workspace config, enabling deterministic testing.
pub fn load_merged_config_from(
    global_config_path: Option<&Path>,
    workspace_path: Option<&Path>,
) -> Result<Config> {
    Ok(load_split_config_from(global_config_path, workspace_path)?.merged)
}

/// Load split config layers from explicit paths (WEFT-10).
///
/// Returns [`SplitConfig`] with global, optional workspace overlay, and
/// the deep-merged view. Deterministic for tests.
pub fn load_split_config_from(
    global_config_path: Option<&Path>,
    workspace_path: Option<&Path>,
) -> Result<SplitConfig> {
    let defaults = Config::default();
    let mut global_value =
        serde_json::to_value(&defaults).map_err(|e| ClawftError::ConfigInvalid {
            reason: format!("failed to serialize defaults: {e}"),
        })?;

    // Global config
    if let Some(gp) = global_config_path
        && let Some(mut global) = load_config_file(gp)
    {
        normalize_keys(&mut global);
        deep_merge(&mut global_value, &global);
    }

    let global: Config =
        serde_json::from_value(global_value.clone()).map_err(ClawftError::Json)?;

    // Workspace config: <workspace>/.clawft/config.json — kept separate.
    let mut workspace = None;
    let mut merged_value = global_value;
    if let Some(ws_path) = workspace_path
        && let Some(mut ws_config) = load_config_file(&ws_path.join(".clawft").join("config.json"))
    {
        normalize_keys(&mut ws_config);
        // Workspace-only Config (defaults fill missing fields via serde).
        let ws_typed: Config =
            serde_json::from_value(ws_config.clone()).map_err(ClawftError::Json)?;
        workspace = Some(ws_typed);
        deep_merge(&mut merged_value, &ws_config);
    }

    let merged: Config = serde_json::from_value(merged_value).map_err(ClawftError::Json)?;

    // Chain event marker for workspace config load/merge.
    crate::chain_event!(
        "workspace",
        crate::chain_event::EVENT_KIND_WORKSPACE_CONFIG,
        {
            "global_path": global_config_path.map(|p| p.display().to_string()).unwrap_or_default(),
            "workspace_path": workspace_path.map(|p| p.display().to_string()).unwrap_or_default(),
            "workspace_overlay": workspace.is_some()
        }
    );

    Ok(SplitConfig {
        global,
        workspace,
        merged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn load_merged_config_defaults_only() {
        let config = load_merged_config_from(None, None).unwrap();
        assert_eq!(config.agents.defaults.model, clawft_types::config::DEFAULT_LOCAL_LLM_MODEL_ROUTED);
        assert_eq!(config.agents.defaults.max_tokens, 8192);
    }

    #[test]
    fn load_merged_config_workspace_overrides() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-ws-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        let dot_clawft = dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();

        let ws_config = r#"{"agents": {"defaults": {"max_tokens": 4096}}}"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let config = load_merged_config_from(None, Some(&dir)).unwrap();
        assert_eq!(config.agents.defaults.max_tokens, 4096);
        assert_eq!(config.agents.defaults.model, clawft_types::config::DEFAULT_LOCAL_LLM_MODEL_ROUTED);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_global_overrides() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-global-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let global_config = r#"{"agents": {"defaults": {"model": "custom/model"}}}"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), None).unwrap();
        assert_eq!(config.agents.defaults.model, "custom/model");
        assert_eq!(config.agents.defaults.max_tokens, 8192);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_workspace_over_global() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-both-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let global_config =
            r#"{"agents": {"defaults": {"model": "global/model", "max_tokens": 2048}}}"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        let ws_dir = dir.join("workspace");
        let dot_clawft = ws_dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();
        let ws_config = r#"{"agents": {"defaults": {"max_tokens": 4096}}}"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), Some(&ws_dir)).unwrap();
        assert_eq!(config.agents.defaults.model, "global/model");
        assert_eq!(config.agents.defaults.max_tokens, 4096);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_missing_workspace_config() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-missing-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let config = load_merged_config_from(None, Some(&dir)).unwrap();
        assert_eq!(config.agents.defaults.max_tokens, 8192);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_normalizes_keys() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-normalize-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let global_config = r#"{"agents": {"defaults": {"max_tokens": 1000}}}"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        let ws_dir = dir.join("workspace");
        let dot_clawft = ws_dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();
        let ws_config = r#"{"agents": {"defaults": {"maxTokens": 2000}}}"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let config = load_merged_config_from(Some(&global_path), Some(&ws_dir)).unwrap();
        assert_eq!(config.agents.defaults.max_tokens, 2000);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_split_config_keeps_workspace_permissions_separate() {
        // WEFT-10: workspace elevates user.level to 2; global caps at 1.
        // Split must preserve both so PermissionResolver can clamp.
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-split-ceil-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let global_config = r#"{
            "routing": {
                "permissions": {
                    "user": { "level": 1, "tool_access": ["read_file"] }
                }
            }
        }"#;
        let global_path = dir.join("global-config.json");
        std::fs::write(&global_path, global_config).unwrap();

        let ws_dir = dir.join("workspace");
        let dot_clawft = ws_dir.join(".clawft");
        std::fs::create_dir_all(&dot_clawft).unwrap();
        let ws_config = r#"{
            "routing": {
                "permissions": {
                    "user": {
                        "level": 2,
                        "tool_access": ["read_file", "dangerous"],
                        "cost_budget_daily_usd": 999.0
                    }
                }
            }
        }"#;
        std::fs::write(dot_clawft.join("config.json"), ws_config).unwrap();

        let split = load_split_config_from(Some(&global_path), Some(&ws_dir)).unwrap();
        assert!(split.workspace.is_some());
        assert_eq!(split.global.routing.permissions.user.level, Some(1));
        assert_eq!(
            split.workspace.as_ref().unwrap().routing.permissions.user.level,
            Some(2)
        );
        // Merged still deep-merges for non-resolver consumers.
        assert_eq!(split.merged.routing.permissions.user.level, Some(2));

        // End-to-end: PermissionResolver clamps elevated workspace grants.
        use crate::pipeline::permissions::PermissionResolver;
        let perms = PermissionResolver::new(
            split.global_routing(),
            split.workspace_routing(),
        )
        .resolve("someone", "telegram", true);
        assert!(
            perms.level <= 1,
            "workspace level 2 must clamp to global ceiling 1, got {}",
            perms.level
        );
        assert!(
            !perms.tool_access.contains(&"dangerous".to_string()),
            "workspace-only tool must be filtered by ceiling"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_merged_config_mcp_servers() {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-merge-mcp-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

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

        assert!(
            config.tools.mcp_servers.contains_key("github"),
            "github server should be preserved"
        );
        assert_eq!(config.tools.mcp_servers["github"].command, "npx");

        assert!(
            config.tools.mcp_servers.contains_key("rvf"),
            "rvf server should be added"
        );
        assert_eq!(config.tools.mcp_servers["rvf"].command, "npx");

        assert!(
            !config.tools.mcp_servers.contains_key("slack"),
            "slack server should be removed by null overlay"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
