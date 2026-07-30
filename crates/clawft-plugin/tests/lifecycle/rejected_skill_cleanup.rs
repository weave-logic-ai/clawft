//! T39 lifecycle — **rejected-skill cleanup**.
//!
//! When a skill fails grant validation or is operator-rejected, it must
//! not remain loadable. Cleanup removes the skill directory / leaves no
//! pending marker that could be accidentally approved later.

use crate::harness::PendingSkillFixture;
use clawft_plugin::{validate_allowed_tools, SkillLoadError};

/// Operator reject deletes the skill tree (no residual `.pending` / SKILL.md).
#[test]
fn rejected_skill_directory_is_removed() {
    let fixture = PendingSkillFixture::create(
        "to-reject",
        "---\nname: to-reject\nallowed-tools:\n  - exec_shell\n---\n# x\n",
    )
    .expect("fixture");
    let skill_dir = fixture.skill_dir.clone();
    let root = fixture.root.clone();

    fixture.reject().expect("reject cleanup");

    assert!(
        !skill_dir.exists(),
        "rejected skill dir must be gone: {}",
        skill_dir.display()
    );
    // Outer temp root may also be cleaned by reject().
    let _ = root;
}

/// Grant-failure path: skill declaring ungranted tools is refused and
/// callers should treat that as "do not activate" (cleanup / leave pending).
#[test]
fn grant_rejection_surfaces_tool_not_granted() {
    let skill = "malicious-shell-skill";
    let declared = vec!["exec_shell".to_string(), "Read".to_string()];
    // Operator never granted exec_shell for this skill.
    let granted = vec!["Read".to_string()];

    let err = validate_allowed_tools(skill, &declared, &granted).expect_err("must reject");
    match &err {
        SkillLoadError::ToolNotGranted { skill: s, denied } => {
            assert_eq!(s, skill);
            assert!(denied.contains(&"exec_shell".to_string()));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Contract: loaders must not activate on ToolNotGranted.
    // Simulated cleanup after failed load of a pending skill:
    let fixture = PendingSkillFixture::create(skill, "# pending bad skill\n").expect("fixture");
    assert!(fixture.is_pending());
    // Policy under test: reject (cleanup) rather than leave a half-loaded skill.
    let skill_dir = fixture.skill_dir.clone();
    fixture.reject().expect("cleanup after grant reject");
    assert!(!skill_dir.exists());
}

/// Empty grant set rejects any non-empty declared tools (locked-down default).
#[test]
fn empty_grants_reject_all_declared_tools() {
    let err = validate_allowed_tools("locked", &["Read".into()], &[])
        .expect_err("empty grants reject declared tools");
    assert!(matches!(err, SkillLoadError::ToolNotGranted { .. }));
}

/// After reject, re-creating the skill is a fresh pending install (no stale state).
#[test]
fn cleanup_allows_clean_reinstall() {
    let name = "reinstallable";
    let first = PendingSkillFixture::create(name, "# v1\n").expect("first");
    let path = first.skill_dir.clone();
    first.reject().expect("cleanup");
    assert!(!path.exists());

    let second = PendingSkillFixture::create(name, "# v2\n").expect("second");
    assert!(second.is_pending());
    let md = std::fs::read_to_string(second.skill_md_path()).expect("read");
    assert!(md.contains("v2"));
    // leave second for drop/cleanup via reject
    second.reject().ok();
}

/// Approve then later "uninstall" style cleanup: removing skill dir after
/// active use is a full remove (no `.pending` required).
#[test]
fn active_skill_uninstall_removes_tree() {
    let fixture = PendingSkillFixture::create("active-uninstall", "# active\n").expect("f");
    fixture.approve().expect("activate");
    assert!(!fixture.is_pending());
    let skill_dir = fixture.skill_dir.clone();
    // Uninstall = same as reject cleanup path.
    fixture.reject().expect("uninstall");
    assert!(!skill_dir.exists());
}

/// Multiple denied tools are reported sorted/dedup'd for cleanup diagnostics.
#[test]
fn grant_rejection_lists_all_denied_tools() {
    let declared = vec![
        "exec_shell".into(),
        "WebFetch".into(),
        "exec_shell".into(),
    ];
    let err = validate_allowed_tools("multi", &declared, &["Read".into()])
        .expect_err("multiple denied");
    match err {
        SkillLoadError::ToolNotGranted { denied, .. } => {
            assert_eq!(denied, vec!["WebFetch", "exec_shell"]);
        }
        other => panic!("unexpected: {other:?}"),
    }
}
