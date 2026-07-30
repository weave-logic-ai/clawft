# WEFT-73 result — T39 plugin-lifecycle tests

**Ticket:** WEFT-73  
**Branch:** `wave0e/weft-73-plugin-lifecycle-tests`  
**Base:** `release/0.8-staging`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef4-7621-9495-d5bb62542efc`  
**Commit:** `e34611e7` (impl); branch tip includes docs pin  
**Date:** 2026-07-30  
**Agent:** coder-73 (wave-0e)

## Problem

T39 plugin-lifecycle tests are named in the C2 security exit list
(`.planning/sparc/phase4/04-plugin-skill-system/00-orchestrator.md`,
`01-wasm-security-spec.md` §6.6) but never landed. Shell-skill approval
(WEFT-63 / task 5) was deferred in part because this test infrastructure
was missing.

## What shipped

Integration test binary under `crates/clawft-plugin/tests/lifecycle/`:

| File | Role |
|------|------|
| `main.rs` | Test crate root + module map |
| `harness.rs` | Shared fixtures: install gate, `.pending` skill layout, in-memory approval store, shell confirmation helper |
| `install.rs` | Manifest install accept/reject (parse + validate + on-disk tree) |
| `hot_reload.rs` | Version-upgrade `PermissionDiff` re-prompt; denied upgrade keeps prior approval |
| `approve_reject.rs` | Pending skill approve/reject; first-run permission gate; tool grants |
| `signed_install.rs` | T38 unsigned-local ok; T37 signature scaffold (K4 crypto TODO) |
| `rejected_skill_cleanup.rs` | Reject removes tree; grant-fail → cleanup; clean reinstall |
| `shell_skill.rs` | **T39 core**: `shell: true` first-run escalation; deny without approval; sandbox defense-in-depth |

Planning checkbox updates:

- `00-orchestrator.md` §4.3 — T39 tests marked done; product shell CLI left open (WEFT-63)
- `01-wasm-security-spec.md` §7 — same split

No production code changes — tests + planning notes only.

## Coverage

| Area | Real public API | Scaffold / TODO |
|------|-----------------|-----------------|
| **install** | `PluginManifest::from_json` + `validate`; disk tree | — |
| **hot-reload** | `PluginPermissions::diff` / `PermissionDiff` | Full FS watcher is in `clawft-core::skill_watcher` (not dep of this crate) |
| **approve/reject** | `.pending` convention + `validate_allowed_tools` | Interactive CLI = WEFT-59/63 |
| **signed-install** | Manifest ignores unknown signature fields; T38 unsigned local | **TODO(K4)**: real Ed25519 verify vs trust root (`clawft_services::clawhub`) |
| **rejected-skill-cleanup** | `SkillLoadError::ToolNotGranted` + dir removal | — |
| **shell-skill confirmation** | `shell_escalation` + `ProcessPolicy.allow_shell` default false | **TODO(WEFT-63)**: wire `--allow-shell` / interactive prompt |

**Lifecycle tests:** 46 (all passed)  
**Package total:** 181 passed (`scripts/build.sh test clawft-plugin`)

## Acceptance

| Criterion | Status |
|-----------|--------|
| T39 cases under `crates/clawft-plugin/tests/lifecycle/` | **Done** |
| Cover install, hot-reload, approve/reject, signed-install, rejected-skill-cleanup, shell-skill | **Done** (signed-install has honest K4 scaffold) |
| Pass `scripts/build.sh test` (package-scoped) | **Done** — 181/181 |
| Update C2 exit-criteria checkbox | **Done** (T39 tests checked; product CLI remains WEFT-63) |

## Tests

```bash
scripts/build.sh test clawft-plugin
# focused:
cargo test -p clawft-plugin --test lifecycle
```

## Limitations

1. **No circular dep on clawft-core**: skill_autogen / skill_watcher live in core; harness mirrors the `.pending` filesystem convention and pure permission gates without importing core.
2. **Signed install** is presence-based scaffold until ClawHub trust-root (K4) is available in-tree.
3. **Interactive prompts** are modeled as a boolean operator decision; CLI wiring is WEFT-63.

## Files changed

- `crates/clawft-plugin/tests/lifecycle/` (new; 8 files)
- `.planning/sparc/phase4/04-plugin-skill-system/00-orchestrator.md`
- `.planning/sparc/phase4/04-plugin-skill-system/01-wasm-security-spec.md`
- `docs/plans/wave-0e-WEFT-73-result.md` (this file)
