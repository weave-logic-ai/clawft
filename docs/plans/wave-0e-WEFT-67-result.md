# WEFT-67 result — skills autogen CLI

**Branch:** `wave0e/weft-67-skills-autogen-cli`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef5-7951-8346-379033f028ae`  
**Base:** `release/0.8-staging`

## Problem

`AutogenConfig::default().enabled == false`. Operators could only enable
autonomous skill creation by constructing a config in code — no CLI surface,
no onboarding flag, no status view.

## What shipped

| Surface | Implementation |
|---------|----------------|
| Config schema | `skills.autogen.{enabled,threshold,max_pending}` on `Config` (`clawft-types`) |
| `weft skills autogen enable` | Sets `skills.autogen.enabled = true` in user config |
| `weft skills autogen disable` | Sets `enabled = false` |
| `weft skills autogen status` | Prints state + threshold + max_pending + config path |
| Persistence | `CLAWFT_CONFIG` or `~/.clawft/config.json` (create parent dir; tmp+rename) |
| Help | `weft help skills` lists autogen commands + examples |
| Runtime | `weft agent` / `weft gateway` attach `PatternDetector` when enabled |

### Files

- `crates/clawft-types/src/config/skills.rs` — **new** `SkillsConfig` / `SkillAutogenConfig`
- `crates/clawft-types/src/config/mod.rs` — `Config.skills` field + tests
- `crates/clawft-cli/src/commands/skills_cmd.rs` — `Autogen` subcommand + persist helpers + unit tests
- `crates/clawft-cli/src/commands/agent.rs` — wire detector when enabled
- `crates/clawft-cli/src/commands/gateway.rs` — same for gateway path
- `crates/clawft-cli/src/help_text.rs` — help topic + examples
- `crates/clawft-cli/src/main.rs` — parse tests for enable/disable/status

### Config shape

```json
{
  "skills": {
    "autogen": {
      "enabled": false,
      "threshold": 3,
      "max_pending": 10
    }
  }
}
```

Defaults match `AutogenConfig::default()` (disabled, threshold 3, max_pending 10).
Existing keys (agents, gateway, …) are preserved on toggle.

## Verification

```text
scripts/build.sh test clawft-types clawft-cli
# 750 passed (includes:
#   skills_autogen_defaults_disabled
#   skills_autogen_deserializes_from_json
#   ensure_autogen_object_preserves_threshold
#   persist_and_toggle_autogen_roundtrip
#   cli_skills_autogen_{enable,disable,status}_parses
# )
```

Workspace `scripts/build.sh clippy` currently fails on a pre-existing
`collapsible_if` in `loop_core.rs:1842` (unrelated to this change).

## Acceptance

| Criterion | Status |
|-----------|--------|
| `weft skills autogen enable` flips enabled: true | Yes |
| `weft skills autogen disable` reverts | Yes |
| `weft skills autogen status` shows state + threshold + max_pending | Yes |
| Help text + examples | Yes (`weft help skills`) |
| Persist to `~/.clawft/config.json` (or `CLAWFT_CONFIG`) | Yes |

## How to test

```bash
# Isolated config (do not touch real home config):
export CLAWFT_CONFIG=/tmp/weft-67-test-config.json
rm -f "$CLAWFT_CONFIG"

cargo run -p clawft-cli -- skills autogen status
# state: disabled, threshold 3, max_pending 10

cargo run -p clawft-cli -- skills autogen enable
# enabled: true; file written

cargo run -p clawft-cli -- skills autogen status
jq .skills.autogen "$CLAWFT_CONFIG"

cargo run -p clawft-cli -- skills autogen disable
jq .skills.autogen "$CLAWFT_CONFIG"
# enabled: false
```

## Follow-ups

- Optional CLI flags to set `threshold` / `max_pending` without hand-editing JSON
- Reload autogen settings in a long-running daemon without restart (today: read at agent/gateway start)
