//! T39 lifecycle — **install** gate.
//!
//! A plugin/skill may only be installed when its public manifest parses
//! and validates. Invalid manifests are rejected before any activation.

use crate::harness::{install_manifest, load_installed, plugin_json, write_install_tree};
use clawft_plugin::{PluginCapability, PluginError};

/// Known-good plugin body installs (parse + validate succeeds).
#[test]
fn install_accepts_valid_plugin_manifest() {
    let body = plugin_json("com.example.lifecycle-install", "1.0.0", false);
    let m = install_manifest(&body).expect("valid install should succeed");
    assert_eq!(m.id, "com.example.lifecycle-install");
    assert_eq!(m.version, "1.0.0");
    assert!(m.capabilities.contains(&PluginCapability::Tool));
    assert!(!m.permissions.shell);
}

/// On-disk install tree: write `clawft.plugin.json`, load as installer would.
#[test]
fn install_from_disk_roundtrip() {
    let body = plugin_json("com.example.disk-install", "0.1.0", false);
    let (_tmp, path) = write_install_tree("com.example.disk-install", &body).expect("write tree");
    let m = load_installed(&path).expect("load installed manifest");
    assert_eq!(m.id, "com.example.disk-install");
    assert_eq!(m.tools, vec!["fixture_tool"]);
}

/// Malformed JSON is rejected at install.
#[test]
fn install_rejects_malformed_json() {
    let err = clawft_plugin::PluginManifest::from_json("{not json")
        .expect_err("malformed JSON must fail install");
    assert!(matches!(err, PluginError::Serialization(_)));
}

/// Missing required fields rejected at install.
#[test]
fn install_rejects_missing_id() {
    let body = serde_json::json!({
        "id": "",
        "name": "Broken",
        "version": "1.0.0",
        "capabilities": ["tool"]
    });
    let err = install_manifest(&body).expect_err("empty id must fail");
    assert!(matches!(err, PluginError::LoadFailed(_)));
    assert!(err.to_string().contains("id"));
}

/// Non-semver version rejected at install.
#[test]
fn install_rejects_invalid_semver() {
    let mut body = plugin_json("com.example.badver", "not-a-version", false);
    body["version"] = serde_json::json!("not-a-version");
    let err = install_manifest(&body).expect_err("bad semver must fail");
    assert!(matches!(err, PluginError::LoadFailed(_)));
    assert!(err.to_string().contains("semver") || err.to_string().contains("version"));
}

/// Empty capabilities rejected at install.
#[test]
fn install_rejects_empty_capabilities() {
    let mut body = plugin_json("com.example.nocap", "1.0.0", false);
    body["capabilities"] = serde_json::json!([]);
    let err = install_manifest(&body).expect_err("empty capabilities must fail");
    assert!(matches!(err, PluginError::LoadFailed(_)));
}

/// Install of a skill-capable plugin with tools list is accepted when
/// the tools will later pass grant intersection (install itself only
/// validates the wire format).
#[test]
fn install_skill_capable_plugin_with_tools() {
    let mut body = plugin_json("com.example.skill-plugin", "2.0.0", false);
    body["capabilities"] = serde_json::json!(["skill", "tool"]);
    body["skills"] = serde_json::json!(["code-review"]);
    body["tools"] = serde_json::json!(["Read", "Edit"]);
    let m = install_manifest(&body).expect("skill-capable install");
    assert_eq!(m.skills, vec!["code-review"]);
    assert_eq!(m.tools, vec!["Read", "Edit"]);
}
