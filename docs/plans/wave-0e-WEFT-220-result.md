# WEFT-220 result — Windows install-service (schtasks final)

**Ticket:** WEFT-220 — ws10: Windows install-service — automate schtasks or document manual route as final  
**Branch:** `weft-220-windows-install-service`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb69a-23ae-7e01-9cdd-90d79b98df6e`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-31  
**Agent:** developer (Grok Build / Ruflo team)  
**Choice:** **Automate schtasks + document manual route as final** (both)

## Problem

`weft voice install-service` on Windows only printed three vague Task
Scheduler steps and labeled the manager `unsupported`. It was unclear
whether that was temporary or the long-term plan.

## Decision

| Option | Verdict |
|--------|---------|
| Automate `schtasks` ONLOGON (current user, LIMITED) | **Chosen primary** — parity with systemd/launchd auto-install |
| Document manual schtasks / GUI as final fallback | **Chosen** — always printed on failure; guide marks path final |
| Windows Service (SCM) | **Rejected** for wake — needs interactive session + mic |

## What shipped

| Artifact | Change |
|----------|--------|
| `crates/clawft-cli/src/commands/voice.rs` | Detect `schtasks` on Windows; `install_schtasks_service`; manual route printer; unit tests for `/TR` quoting + OS detect |
| `scripts/install-clawft-wake-schtasks.ps1` | Optional companion: create/uninstall `ClawftWake` via schtasks |
| `docs/guides/voice.md` | § Wake word service — platform table + Windows final route |
| `docs/development/testing-three-workstreams.md` | §4.9 includes PS1 + schtasks verify |

### CLI behaviour

```text
weft voice install-service                 # auto: schtasks on Windows
weft voice install-service --manager schtasks
```

Creates task **`ClawftWake`**: `/SC ONLOGON /RL LIMITED /F`,
`/TR "<weft.exe> voice wake"` (exe from `current_exe()`). On failure,
prints the full manual schtasks + GUI procedure (non-provisional).

## Acceptance

- [x] Windows manager is no longer an unclear placeholder
- [x] Automated schtasks install path implemented
- [x] Manual route documented as **final** supported design
- [x] Optional PS1 script shipped under `scripts/`
- [x] Unit tests for pure helpers (`install_service_tests`)
- [ ] Host Windows smoke: `weft voice install-service` on a real Windows box (follow-up / tester)

## How to test

```bash
# templates present
ls scripts/clawft-wake.service scripts/com.clawft.wake.plist \
   scripts/install-clawft-wake-schtasks.ps1

# unit tests (any host)
scripts/build.sh check   # or targeted:
cargo test -p clawft-cli install_service_tests

# Windows only (manual):
#   weft voice install-service
#   schtasks /Query /TN ClawftWake /V /FO LIST
#   schtasks /Delete /TN ClawftWake /F
```

## Out of scope

- Windows Service Control Manager binary / service DLL
- Fixing Linux/macOS unit templates still passing unused `--daemon` (pre-existing)
- Live wake detection (WEFT-216 / OpenWakeWord follow-ups)
