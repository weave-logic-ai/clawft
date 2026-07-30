//! WEFT-73 / T39: plugin lifecycle integration tests.
//!
//! Locks the **public security contracts** that gate install, upgrade
//! (hot-reload of a new manifest), first-run approval, shell-skill
//! confirmation, grant rejection, and (scaffold) signed install.
//!
//! ## Scope
//!
//! These tests live in `clawft-plugin` (trait + manifest + sandbox + grants
//! crate) and exercise only **public** APIs available here:
//!
//! - [`PluginManifest`] parse / validate (install gate)
//! - [`PluginPermissions::diff`] / [`PermissionDiff`] (upgrade + shell T39)
//! - [`validate_allowed_tools`] / [`SkillLoadError`] (skill grants)
//! - [`SandboxPolicy`] / [`ProcessPolicy`] (shell confirmation ↔ sandbox)
//!
//! Full ClawHub signature verification (T37) and the interactive
//! `weft skills approve` CLI (WEFT-59/63) live in other crates; those paths
//! are scaffolded with compile-green contracts and explicit `// TODO`
//! markers only where host/CLI infra is unavailable.
//!
//! Source: `.planning/sparc/phase4/04-plugin-skill-system/01-wasm-security-spec.md`
//! §6.6 Lifecycle Security Tests (T36–T41); orchestrator C2 exit criteria.

mod harness;

mod approve_reject;
mod hot_reload;
mod install;
mod rejected_skill_cleanup;
mod shell_skill;
mod signed_install;
