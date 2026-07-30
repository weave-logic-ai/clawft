//! T39 lifecycle — **shell-skill confirmation** (core of security T39).
//!
//! Spec (01-wasm-security-spec.md §6.6):
//! > T39 | Plugin requests `shell: true` on first run
//! >     | User prompted for approval; denied if not approved
//!
//! Orchestrator C2 exit criterion:
//! > Shell-execution skills require explicit user approval on install
//!
//! These tests lock the pure decision contracts that WEFT-63 (CLI prompt)
//! and the WASM permission store will drive interactively.

use crate::harness::{
    confirm_shell_skill, empty_perms, install_manifest, plugin_json, requires_user_confirmation,
    shell_perms, InMemoryApprovalStore,
};
use clawft_plugin::{
    validate_allowed_tools, PermissionDiff, PluginPermissions, ProcessPolicy, SandboxPolicy,
    SkillLoadError,
};

/// T39: first-run with `shell: true` produces shell_escalation in the diff.
#[test]
fn t39_first_run_shell_true_is_escalation() {
    let approved = empty_perms(); // first run: nothing approved
    let requested = shell_perms();
    let diff = PluginPermissions::diff(&approved, &requested);
    assert!(
        diff.shell_escalation,
        "first-run shell:true must be shell_escalation"
    );
    assert!(requires_user_confirmation(&diff));
}

/// T39: denied if not approved.
#[test]
fn t39_shell_denied_without_operator_approval() {
    let err = confirm_shell_skill(&empty_perms(), &shell_perms(), false)
        .expect_err("must deny shell skill without approval");
    assert!(
        err.contains("explicit user approval"),
        "error should mention approval: {err}"
    );
}

/// T39: allowed when operator approves.
#[test]
fn t39_shell_allowed_with_operator_approval() {
    confirm_shell_skill(&empty_perms(), &shell_perms(), true)
        .expect("approved shell skill must activate");
}

/// Already-approved shell does not re-prompt.
#[test]
fn t39_shell_already_approved_no_reprompt() {
    let approved = shell_perms();
    let requested = shell_perms();
    let diff = PluginPermissions::diff(&approved, &requested);
    assert!(!diff.shell_escalation);
    assert!(!requires_user_confirmation(&diff));
    confirm_shell_skill(&approved, &requested, false)
        .expect("no escalation → no approval required");
}

/// Install-time: shell plugin manifest is parseable, but gate still requires approval.
#[test]
fn t39_shell_manifest_install_still_requires_gate() {
    let body = plugin_json("com.example.shell-plugin", "1.0.0", true);
    let m = install_manifest(&body).expect("manifest with shell:true still parses");
    assert!(m.permissions.shell);

    let mut store = InMemoryApprovalStore::new();
    // Deny at gate → not activated
    let err = store
        .gate_install(&m.id, &m.permissions, false)
        .expect_err("shell install without approval denied");
    assert!(err.contains("denied") || err.contains("shell"));
    assert!(store.load(&m.id).is_none());

    // Approve at gate → activated
    store
        .gate_install(&m.id, &m.permissions, true)
        .expect("approved shell install");
    assert!(store.load(&m.id).unwrap().shell);
}

/// Skill declaring `exec_shell` without grant is refused (install-time intersection).
#[test]
fn t39_skill_declaring_exec_shell_requires_grant() {
    let declared = vec!["exec_shell".to_string()];
    let granted_without_shell: Vec<String> = vec!["Read".into(), "Edit".into()];
    let err = validate_allowed_tools("shell-skill", &declared, &granted_without_shell)
        .expect_err("exec_shell without grant must fail");
    match err {
        SkillLoadError::ToolNotGranted { denied, .. } => {
            assert_eq!(denied, vec!["exec_shell"]);
        }
        other => panic!("expected ToolNotGranted, got {other:?}"),
    }

    // With explicit grant (post-confirmation), load succeeds.
    let granted_with_shell = vec!["exec_shell".to_string()];
    validate_allowed_tools("shell-skill", &declared, &granted_with_shell)
        .expect("granted exec_shell allows skill load");
}

/// Sandbox `ProcessPolicy.allow_shell` defaults to false (secure by default).
#[test]
fn t39_sandbox_process_policy_shell_default_false() {
    let policy = ProcessPolicy::default();
    assert!(
        !policy.allow_shell,
        "default ProcessPolicy must deny shell"
    );
    let sandbox = SandboxPolicy::default();
    assert!(
        !sandbox.process.allow_shell,
        "default SandboxPolicy must deny shell"
    );
}

/// Shell skill confirmation + sandbox: even after manifest shell:true is
/// approved, runtime sandbox can still deny shell if process policy is off.
/// (Defense in depth — host should align these; contract documents both layers.)
#[test]
fn t39_approved_manifest_shell_still_respects_sandbox_process_policy() {
    let manifest_shell = shell_perms();
    confirm_shell_skill(&empty_perms(), &manifest_shell, true).expect("manifest approved");

    let mut sandbox = SandboxPolicy::default();
    // Operator approved plugin shell bit, but agent sandbox still locked down.
    assert!(!sandbox.process.allow_shell);
    // Runtime check host would do:
    let runtime_allows = sandbox.process.allow_shell && manifest_shell.shell;
    assert!(
        !runtime_allows,
        "runtime must deny shell when sandbox process.allow_shell is false"
    );

    // After aligning sandbox with approval:
    sandbox.process.allow_shell = true;
    let runtime_allows = sandbox.process.allow_shell && manifest_shell.shell;
    assert!(runtime_allows);
}

/// Combined escalation: shell + new network both surface in one diff.
#[test]
fn t39_combined_shell_and_network_escalation() {
    let approved = empty_perms();
    let requested = PluginPermissions {
        shell: true,
        network: vec!["api.untrusted.example".into()],
        ..Default::default()
    };
    let diff = PluginPermissions::diff(&approved, &requested);
    assert_eq!(
        diff,
        PermissionDiff {
            new_network: vec!["api.untrusted.example".into()],
            new_filesystem: vec![],
            new_env_vars: vec![],
            shell_escalation: true,
        }
    );
    assert!(requires_user_confirmation(&diff));
}

/// Non-interactive install path: without approval flag, shell skill is denied.
///
/// // TODO(WEFT-63): wire CLI `--allow-shell` / env opt-in to this gate.
#[test]
fn t39_noninteractive_without_allow_shell_denies() {
    let allow_shell_flag = false; // simulates missing --allow-shell
    let result = confirm_shell_skill(&empty_perms(), &shell_perms(), allow_shell_flag);
    assert!(result.is_err());
}
