//! T39 lifecycle — **hot-reload / version upgrade** permission re-prompt.
//!
//! When a plugin's on-disk manifest changes (hot-reload) or a new version
//! is installed, only **new** permissions relative to the previously
//! approved set must re-trigger confirmation (T41 family / PermissionDiff).

use crate::harness::{
    empty_perms, install_manifest, plugin_json, requires_user_confirmation, InMemoryApprovalStore,
};
use clawft_plugin::{PermissionDiff, PluginPermissions};

/// Reload with identical permissions → no re-prompt.
#[test]
fn hot_reload_identical_permissions_no_prompt() {
    let approved = PluginPermissions {
        network: vec!["api.example.com".into()],
        filesystem: vec!["/tmp/plugin".into()],
        env_vars: vec!["HOME".into()],
        shell: false,
    };
    let requested = approved.clone();
    let diff = PluginPermissions::diff(&approved, &requested);
    assert!(diff.is_empty());
    assert!(!requires_user_confirmation(&diff));
}

/// Upgrade that adds a network host re-prompts for the new host only (T41).
#[test]
fn hot_reload_new_network_permission_requires_confirmation() {
    let approved = PluginPermissions {
        network: vec!["api.example.com".into()],
        ..Default::default()
    };
    let requested = PluginPermissions {
        network: vec!["api.example.com".into(), "cdn.example.com".into()],
        ..Default::default()
    };
    let diff = PluginPermissions::diff(&approved, &requested);
    assert_eq!(diff.new_network, vec!["cdn.example.com"]);
    assert!(diff.new_filesystem.is_empty());
    assert!(!diff.shell_escalation);
    assert!(requires_user_confirmation(&diff));
}

/// Upgrade that escalates shell on reload re-prompts (T39 + T41).
#[test]
fn hot_reload_shell_escalation_requires_confirmation() {
    let approved = PluginPermissions {
        shell: false,
        network: vec!["api.example.com".into()],
        ..Default::default()
    };
    let requested = PluginPermissions {
        shell: true,
        network: vec!["api.example.com".into()],
        ..Default::default()
    };
    let diff = PluginPermissions::diff(&approved, &requested);
    assert!(diff.shell_escalation);
    assert!(requires_user_confirmation(&diff));
}

/// Downgrade (shell true → false) does not re-prompt.
#[test]
fn hot_reload_shell_downgrade_no_prompt() {
    let approved = PluginPermissions {
        shell: true,
        ..Default::default()
    };
    let requested = PluginPermissions {
        shell: false,
        ..Default::default()
    };
    let diff = PluginPermissions::diff(&approved, &requested);
    assert!(!diff.shell_escalation);
    assert!(diff.is_empty());
}

/// Simulated hot-reload through an approval store: operator denies new
/// network host → upgrade blocked; store retains old permissions.
#[test]
fn hot_reload_denied_upgrade_keeps_previous_approval() {
    let mut store = InMemoryApprovalStore::new();
    let v1 = PluginPermissions {
        network: vec!["api.example.com".into()],
        ..Default::default()
    };
    store
        .gate_install("com.example.reload", &v1, true)
        .expect("v1 install");

    let v2 = PluginPermissions {
        network: vec!["api.example.com".into(), "evil.example.com".into()],
        ..Default::default()
    };
    let err = store
        .gate_install("com.example.reload", &v2, false)
        .expect_err("operator deny must block upgrade");
    assert!(err.contains("permission request denied"));

    let still = store.load("com.example.reload").expect("v1 still stored");
    assert_eq!(still.network, vec!["api.example.com"]);
    assert!(!still.network.iter().any(|h| h == "evil.example.com"));
}

/// Simulated hot-reload: operator approves → new permissions stored.
#[test]
fn hot_reload_approved_upgrade_updates_store() {
    let mut store = InMemoryApprovalStore::new();
    let v1 = empty_perms();
    store
        .gate_install("com.example.reload-ok", &v1, true)
        .expect("v1");

    let v2 = PluginPermissions {
        filesystem: vec!["/data".into()],
        env_vars: vec!["API_KEY".into()],
        ..Default::default()
    };
    let effective = store
        .gate_install("com.example.reload-ok", &v2, true)
        .expect("v2 approved");
    assert_eq!(effective.filesystem, vec!["/data"]);
    assert_eq!(effective.env_vars, vec!["API_KEY"]);
}

/// Full install → parse new version JSON → diff against stored.
/// Models FS hot-reload of `clawft.plugin.json` contents.
#[test]
fn hot_reload_manifest_version_bump_diff() {
    let v1_body = plugin_json("com.example.versioned", "1.0.0", false);
    let v1 = install_manifest(&v1_body).expect("v1");

    let mut v2_body = plugin_json("com.example.versioned", "1.1.0", false);
    v2_body["permissions"]["network"] = serde_json::json!(["api.example.com"]);
    let v2 = install_manifest(&v2_body).expect("v2");

    assert_ne!(v1.version, v2.version);
    let diff = PluginPermissions::diff(&v1.permissions, &v2.permissions);
    assert_eq!(
        diff,
        PermissionDiff {
            new_network: vec!["api.example.com".into()],
            new_filesystem: vec![],
            new_env_vars: vec![],
            shell_escalation: false,
        }
    );
    assert!(requires_user_confirmation(&diff));
}

/// Removed permissions on reload are not reported as new (only additions).
#[test]
fn hot_reload_removed_permissions_not_in_diff() {
    let approved = PluginPermissions {
        network: vec!["old.example.com".into(), "keep.example.com".into()],
        filesystem: vec!["/tmp".into(), "/legacy".into()],
        ..Default::default()
    };
    let requested = PluginPermissions {
        network: vec!["keep.example.com".into()],
        filesystem: vec!["/tmp".into()],
        ..Default::default()
    };
    let diff = PluginPermissions::diff(&approved, &requested);
    assert!(diff.is_empty());
    assert!(!requires_user_confirmation(&diff));
}
