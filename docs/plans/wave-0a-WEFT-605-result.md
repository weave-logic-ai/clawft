# WEFT-605 result — exec_shell security denials visible to the model

**Ticket:** WEFT-605  
**Branch:** `wave0a/weft-605-shell-deny-visible`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb418-df94-7233-a1e3-5763402beca2`  
**Date:** 2026-07-30

## Problem

`exec_shell` policy denials returned a bare `command not allowed: …` with no
hint of what *is* allowed. Tool-happy models (e.g. Hermes 4.3-36B on
`17 * 23`) burned the full max-tool-iteration budget guessing `bc`, `perl`,
`ruby`, `php`, etc.

## Fix

1. **`CommandPolicy::format_denial` + `allowlist_summary`**
   (`crates/clawft-types/src/security.rs`)
   - States the command was **blocked by security policy**
   - Names the **reason**: not on allowlist / on denylist / dangerous pattern
   - For allowlist / dangerous denials in allowlist mode: includes a
     **sorted allowed-executables summary**
   - Advises not to retry unlisted / equivalent blocked variants

2. **`ShellExecTool`** (`crates/clawft-tools/src/shell_tool.rs`)
   - Uses `format_denial` for `PermissionDenied.reason` (instead of bare
     `CommandPolicyError` Display)
   - Tool description notes allowlist policy and that denials return allowed
     executables

3. **`spawn` tool** (`crates/clawft-tools/src/spawn_tool.rs`)
   - Same informative denial path for consistency

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-types/src/security.rs` | `allowlist_summary`, `format_denial`, unit tests |
| `crates/clawft-tools/src/shell_tool.rs` | wire denial + description + tests |
| `crates/clawft-tools/src/spawn_tool.rs` | wire denial |

## Tests

```bash
scripts/build.sh check
scripts/build.sh test clawft-types clawft-tools
```

- **check:** pass  
- **tests:** 521 passed (clawft-types + clawft-tools), 0 failed  
- New coverage:
  - `security::tests::format_denial_not_allowed_includes_allowlist_and_reason`
  - `security::tests::format_denial_dangerous_pattern_is_informative`
  - `security::tests::format_denial_denylist_blocked_is_informative`
  - `security::tests::allowlist_summary_sorted_and_non_empty`
  - `shell_tool::tests::test_denial_message_lists_allowlist_and_reason`
  - `shell_tool::tests::test_dangerous_denial_message_is_informative`

## Acceptance

| Criterion | Status |
|-----------|--------|
| Denied `exec_shell` gives enough signal to stop blind retries | **Met** — reason + allowlist + “do not retry unlisted” |
| `17*23`-style spiral can adapt (e.g. use `python3` / `node` / answer without shell) | **Met** at tool-result contract (model can see allowlist includes `python3`, `node`, etc.) |

## Commit

See branch tip: `git rev-parse wave0a/weft-605-shell-deny-visible`

Recorded at ship time by the implementer in the return summary (SHA below is filled on commit via trailer in git log).
