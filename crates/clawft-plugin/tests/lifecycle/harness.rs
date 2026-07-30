//! Shared helpers for T39 lifecycle integration tests.

use clawft_plugin::{
    PermissionDiff, PluginCapability, PluginManifest, PluginPermissions, PluginResourceConfig,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Build a known-good installable `clawft.plugin.json` Value.
pub fn plugin_json(id: &str, version: &str, shell: bool) -> Value {
    json!({
        "id": id,
        "name": "Lifecycle Fixture Plugin",
        "version": version,
        "capabilities": ["tool", "skill"],
        "permissions": {
            "network": [],
            "filesystem": ["/tmp/lifecycle-fixture"],
            "env_vars": [],
            "shell": shell
        },
        "resources": {
            "max_fuel": 1_000_000_000u64,
            "max_memory_mb": 16,
            "max_http_requests_per_minute": 10,
            "max_log_messages_per_minute": 100,
            "max_execution_seconds": 30,
            "max_table_elements": 10000
        },
        "wasm_module": "plugin.wasm",
        "skills": [],
        "tools": ["fixture_tool"]
    })
}

/// Parse + validate a plugin JSON body as the install gate would.
pub fn install_manifest(body: &Value) -> Result<PluginManifest, clawft_plugin::PluginError> {
    PluginManifest::from_json(&body.to_string())
}

/// Minimal skill-on-disk layout used by reject/cleanup tests.
///
/// Mirrors the `.pending` marker convention from
/// `clawft_core::agent::skill_autogen` without depending on that crate
/// (clawft-core → clawft-plugin; reverse would cycle).
#[derive(Debug)]
pub struct PendingSkillFixture {
    pub root: PathBuf,
    pub skill_dir: PathBuf,
}

impl PendingSkillFixture {
    pub fn create(name: &str, skill_md: &str) -> std::io::Result<Self> {
        let root = tempfile::tempdir()?.keep();
        let skill_dir = root.join(name);
        std::fs::create_dir_all(&skill_dir)?;
        std::fs::write(skill_dir.join("SKILL.md"), skill_md)?;
        std::fs::write(skill_dir.join(".pending"), "awaiting user approval\n")?;
        Ok(Self { root, skill_dir })
    }

    pub fn is_pending(&self) -> bool {
        self.skill_dir.join(".pending").exists()
    }

    /// Approve: remove `.pending` marker (activation gate).
    pub fn approve(&self) -> std::io::Result<()> {
        let marker = self.skill_dir.join(".pending");
        if !marker.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "skill is not in pending state",
            ));
        }
        std::fs::remove_file(marker)
    }

    /// Reject: remove the skill directory entirely (cleanup).
    pub fn reject(self) -> std::io::Result<()> {
        if self.skill_dir.exists() {
            std::fs::remove_dir_all(&self.skill_dir)?;
        }
        // Drop root contents if empty-ish; ignore errors on outer cleanup.
        let _ = std::fs::remove_dir_all(&self.root);
        Ok(())
    }

    pub fn skill_md_path(&self) -> PathBuf {
        self.skill_dir.join("SKILL.md")
    }
}

/// First-run / upgrade approval decision from a [`PermissionDiff`].
///
/// This is the pure decision kernel that host/CLI layers (WEFT-63) wrap
/// with an interactive prompt. Returning `true` means "user must confirm".
pub fn requires_user_confirmation(diff: &PermissionDiff) -> bool {
    !diff.is_empty()
}

/// Shell-skill confirmation: activate only if shell was requested **and**
/// the operator approved the escalation (or shell was already granted).
pub fn confirm_shell_skill(
    previously_approved: &PluginPermissions,
    requested: &PluginPermissions,
    operator_approved_escalation: bool,
) -> Result<(), String> {
    let diff = PluginPermissions::diff(previously_approved, requested);
    if diff.shell_escalation && !operator_approved_escalation {
        return Err(
            "shell-execution skill requires explicit user approval on install (T39)".into(),
        );
    }
    Ok(())
}

/// Simulated approved-permissions record for lifecycle tests (in-memory).
///
/// Production persistence lives in `clawft-wasm::permission_store`; we
/// only model the decision surface here so tests stay in clawft-plugin.
#[derive(Debug, Clone)]
pub struct InMemoryApprovalStore {
    records: std::collections::HashMap<String, PluginPermissions>,
}

impl InMemoryApprovalStore {
    pub fn new() -> Self {
        Self {
            records: std::collections::HashMap::new(),
        }
    }

    pub fn load(&self, plugin_id: &str) -> Option<&PluginPermissions> {
        self.records.get(plugin_id)
    }

    pub fn save(&mut self, plugin_id: &str, perms: PluginPermissions) {
        self.records.insert(plugin_id.to_string(), perms);
    }

    /// Install or upgrade: compute diff against stored approval and apply
    /// the operator decision. Returns the effective approved set.
    pub fn gate_install(
        &mut self,
        plugin_id: &str,
        requested: &PluginPermissions,
        operator_approves: bool,
    ) -> Result<PluginPermissions, String> {
        let approved = self
            .load(plugin_id)
            .cloned()
            .unwrap_or_else(PluginPermissions::default);
        let diff = PluginPermissions::diff(&approved, requested);
        if requires_user_confirmation(&diff) && !operator_approves {
            return Err(format!(
                "permission request denied for '{plugin_id}' (shell_escalation={}, new_network={:?})",
                diff.shell_escalation, diff.new_network
            ));
        }
        // On approval (or empty diff), store the full requested set.
        self.save(plugin_id, requested.clone());
        Ok(requested.clone())
    }
}

/// Default empty permissions (first-run baseline).
pub fn empty_perms() -> PluginPermissions {
    PluginPermissions::default()
}

/// Permissions requesting shell access.
pub fn shell_perms() -> PluginPermissions {
    PluginPermissions {
        shell: true,
        filesystem: vec!["/tmp/lifecycle-fixture".into()],
        ..Default::default()
    }
}

/// Minimal valid manifest for capability checks.
#[allow(dead_code)]
pub fn minimal_manifest(id: &str, version: &str) -> PluginManifest {
    PluginManifest {
        id: id.into(),
        name: "Minimal".into(),
        version: version.into(),
        capabilities: vec![PluginCapability::Tool],
        permissions: PluginPermissions::default(),
        resources: PluginResourceConfig::default(),
        wasm_module: None,
        skills: vec![],
        tools: vec![],
        voice: None,
    }
}

/// Write a temporary plugin install tree: `{root}/{id}/clawft.plugin.json`.
pub fn write_install_tree(id: &str, body: &Value) -> std::io::Result<(tempfile::TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path().join(id);
    std::fs::create_dir_all(&dir)?;
    let manifest_path = dir.join("clawft.plugin.json");
    std::fs::write(&manifest_path, body.to_string())?;
    Ok((tmp, manifest_path))
}

/// Load manifest from an on-disk install tree path.
pub fn load_installed(path: &Path) -> Result<PluginManifest, clawft_plugin::PluginError> {
    let raw = std::fs::read_to_string(path)?;
    PluginManifest::from_json(&raw)
}
