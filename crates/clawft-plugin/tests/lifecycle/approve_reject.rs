//! T39 lifecycle — **approve / reject** pending skills + permission grants.
//!
//! Covers the on-disk `.pending` convention (shared with skill_autogen)
//! and first-run permission approval/rejection via the pure gate.

use crate::harness::{
    empty_perms, shell_perms, confirm_shell_skill, InMemoryApprovalStore, PendingSkillFixture,
};
use clawft_plugin::{validate_allowed_tools, SkillLoadError};

/// Pending skill install creates `.pending` + SKILL.md; not active until approved.
#[test]
fn approve_pending_skill_removes_marker() {
    let fixture = PendingSkillFixture::create(
        "autogen-review",
        "---\nname: autogen-review\nuser-invocable: false\n---\n# Review\n",
    )
    .expect("create fixture");
    assert!(fixture.is_pending());
    assert!(fixture.skill_md_path().exists());

    fixture.approve().expect("approve");
    assert!(!fixture.is_pending());
    // SKILL.md remains after approval (skill is now loadable).
    assert!(fixture.skill_md_path().exists());
}

/// Rejecting a pending skill removes the directory (cleanup).
#[test]
fn reject_pending_skill_removes_directory() {
    let fixture = PendingSkillFixture::create(
        "autogen-bad",
        "---\nname: autogen-bad\n---\n# Bad\n",
    )
    .expect("create fixture");
    let skill_dir = fixture.skill_dir.clone();
    assert!(skill_dir.exists());

    fixture.reject().expect("reject");
    assert!(!skill_dir.exists());
}

/// Approving a non-pending skill fails (idempotent safety).
#[test]
fn approve_non_pending_skill_fails() {
    let fixture = PendingSkillFixture::create("already-active", "# skill\n").expect("create");
    fixture.approve().expect("first approve");
    let err = fixture.approve().expect_err("second approve must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("not in pending state"));
}

/// First-run permission approve stores the full requested set.
#[test]
fn approve_first_run_permissions() {
    let mut store = InMemoryApprovalStore::new();
    let requested = shell_perms();
    let effective = store
        .gate_install("com.example.first-run", &requested, true)
        .expect("operator approved");
    assert!(effective.shell);
    assert!(store.load("com.example.first-run").unwrap().shell);
}

/// First-run permission reject leaves no stored approval.
#[test]
fn reject_first_run_permissions() {
    let mut store = InMemoryApprovalStore::new();
    let requested = shell_perms();
    let err = store
        .gate_install("com.example.denied", &requested, false)
        .expect_err("operator rejected");
    assert!(err.contains("denied"));
    assert!(store.load("com.example.denied").is_none());
}

/// Empty permission diff (re-install same set) does not require re-approval.
#[test]
fn reinstall_same_permissions_auto_passes() {
    let mut store = InMemoryApprovalStore::new();
    let perms = empty_perms();
    store
        .gate_install("com.example.same", &perms, true)
        .expect("first");
    // operator_approves=false but diff empty → still ok
    store
        .gate_install("com.example.same", &perms, false)
        .expect("reinstall with empty diff needs no approval");
}

/// Skill tool grants: approve path (all tools granted).
#[test]
fn approve_skill_tools_when_all_granted() {
    let declared = vec!["Read".into(), "Edit".into()];
    let granted = vec!["Read".into(), "Edit".into(), "WebSearch".into()];
    validate_allowed_tools("safe-skill", &declared, &granted).expect("all granted");
}

/// Skill tool grants: reject path (ungranted tool → SkillLoadError).
#[test]
fn reject_skill_tools_when_not_granted() {
    let declared = vec!["Read".into(), "exec_shell".into()];
    let granted = vec!["Read".into()];
    let err = validate_allowed_tools("risky-skill", &declared, &granted)
        .expect_err("exec_shell not granted");
    match err {
        SkillLoadError::ToolNotGranted { skill, denied } => {
            assert_eq!(skill, "risky-skill");
            assert_eq!(denied, vec!["exec_shell"]);
        }
        other => panic!("expected ToolNotGranted, got {other:?}"),
    }
}

/// confirm_shell_skill approve path.
#[test]
fn approve_shell_skill_confirmation() {
    confirm_shell_skill(&empty_perms(), &shell_perms(), true).expect("approved");
}

/// confirm_shell_skill reject path.
#[test]
fn reject_shell_skill_confirmation() {
    let err = confirm_shell_skill(&empty_perms(), &shell_perms(), false)
        .expect_err("must deny without approval");
    assert!(err.contains("explicit user approval"));
}
