# Wave 0a integration summary — 2026-07-30

Team **wave-0a**: 8 worktree-isolated `ruflo-coder` agents → merged into `feat/hermes-loop-base`.

## Status matrix

| WEFT | Lane | Outcome | Branch / key commit | Notes |
|------|------|---------|---------------------|-------|
| **593** | A Release | **Shipped** | `09c2b754` | Root cause was GHA secret-scan wiping plan job `val`, not empty cargo-dist matrix |
| **596** | B Mesh | **Shipped** (slice) | `9a121457` | ADR-057 path ACL table + enforce on read/list/subscribe; residual for WEFT-429 |
| **660** | C Vector | **Shipped** | `747bf943` | DiskANN reverse `key→id` map; unlocks WEFT-661 |
| **605** | D Agent | **Shipped** | `be8c2ddd` | Policy denials include reason + allowlist summary for the model |
| **671** | E Voice | **Shipped** (decision) | `7113b6d9` | Keep wake in plugin transitional; canonical voice = channels/voice-* |
| **595** | F Leaf | **Shipped** | `91ea834c` | Single-buffer default + double-buffer dirty-rect blit; HW flash not run |
| **663** | G WASM | **Shipped** | `481e7650` | `async_trait(?Send)` on sinks under browser; WEFT-672 remains |
| **684** | J Tooling | **Shipped** | `96ad5329` | Pin `@claude-flow/cli`/`ruflo` **3.32.38**; MCP fails closed without install |

## Merge

All eight branches fetched via `git bundle` from grok worktrees and merged cleanly (ort / fast-forward) onto `feat/hermes-loop-base`.

## Plane

Tickets claimed **In Progress** before code. Close with ship notes after `scripts/build.sh check` green.

## Unlocks (Wave 0b)

| After | Next |
|-------|------|
| 593 | WEFT-594 Docker strategy |
| 596 | WEFT-429 gate wire |
| 660 | WEFT-661 hybrid metric merge |
| 605 | WEFT-651 retry breaker (soft) |
| 663 | WEFT-672 strip_think (independent) |
| 671 | WEFT-613 Chatterbox (decision unblocked) |

## Worktrees (preserve for audit)

```
~/.grok/worktrees/mathewbeane-weftos/subagent-019fb418-*
```
