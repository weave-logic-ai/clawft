# Wave 0h — WEFT-342 result

**Ticket**: ws11: agent-core-v1.1 — hard-refuse on binding-thread mismatch (governance rule)  
**Branch**: `wave0h/weft-342-binding-refuse`  
**Base**: `release/0.8-staging`  
**Status**: implemented; focused unit tests green  
**Commit**: branch tip (`git log -1 --oneline` on `wave0h/weft-342-binding-refuse`)  
**Pushed**: no (lead merge)

## What shipped

1. **Hard refuse by default** (`identity.rs` follow-up + `system_prompt.rs`)
   - `IdentityError::BindingThreadMismatch` when SOUL.md lacks `BINDING_THREAD_EXCERPT`.
   - `SystemPromptBuilder` defaults to deny; mismatch aborts prompt construction.
   - Constants: `BINDING_THREAD_GATE_ACTION` (`soul.binding_thread_intact`),
     `BINDING_THREAD_MISMATCH_REASON` (`binding-thread mismatch`).

2. **Configurable policy** (`agents.binding_thread_mode`)
   - `BindingThreadMode::{Deny, WarnOnly}` — default **Deny** (v1.1).
   - `warn_only` restores legacy annotate + `warn!` + degraded prompt.
   - Wired through `build_daemon_agent_loop` / bootstrap from `AgentsConfig`.

3. **Per-turn `gate.check`** (`loop_core.rs`)
   - Every turn with a system-prompt builder calls
     `gate.check(agent_id, "soul.binding_thread_intact", effect_for_binding_thread(status))`.
   - Deny mode → `ClawftError::SecurityViolation { reason: "binding-thread mismatch" }`
     (no LLM call). Warn-only continues with annotated prompt.

4. **Governance rule** (`governance.rs` + genesis + daemon chat gate)
   - `GovernanceRuleType::BindingThread`.
   - Rule id `soul.binding_thread_intact` (Blocking, Legislative).
   - Evaluated independent of magnitude when action is the BT check and
     mismatch is signaled (elevated security effect / context status).
   - Deny reason is the canonical string `binding-thread mismatch`.
   - Genesis: 25 → **26** rules; daemon chat gate adds the BT rule.

## Acceptance criteria

| AC | Status |
|----|--------|
| New governance rule `soul.binding_thread_intact` | done |
| Evaluated by `gate.check` on every turn | done (`loop_core` prelude) |
| Hard refusal path: Deny / reason `binding-thread mismatch` | done (gate + `SecurityViolation`) |
| Configurable: warn-only vs deny (v1.1 default) | done (`agents.binding_thread_mode`) |
| Tests for both modes | done |

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-types/src/config/mod.rs` | `BindingThreadMode`, `agents.binding_thread_mode` |
| `crates/clawft-core/src/agent/identity.rs` | constants, `BindingThreadMismatch`, helper |
| `crates/clawft-core/src/agent/system_prompt.rs` | deny/warn_only build path |
| `crates/clawft-core/src/agent/effects.rs` | `effect_for_binding_thread` |
| `crates/clawft-core/src/agent/loop_core.rs` | per-turn gate + hard refuse |
| `crates/clawft-core/src/bootstrap.rs` | mode from config |
| `crates/clawft-kernel/src/governance.rs` | rule type, evaluate, genesis helper, tests |
| `crates/clawft-kernel/src/boot.rs` | genesis +1 BT rule; count 26 |
| `crates/clawft-weave/src/daemon.rs` | chat gate BT rule |
| `crates/clawft-kernel/tests/snapshots/…default_config.snap` | `binding_thread_mode` + prior field drift |
| `docs/plans/wave-0h-WEFT-342-result.md` | this file |

## Tests run

```text
scripts/build.sh check                                    # ok
cargo test -p clawft-types --lib binding_thread           # 1 passed
cargo test -p clawft-core --lib binding_thread            # 5 passed
cargo test -p clawft-core --lib system_prompt             # 14 passed
cargo test -p clawft-core --lib identity::                # 12 passed
cargo test -p clawft-core --lib effects::                 # 12 passed
cargo test -p clawft-kernel --lib governance::            # 88 passed
cargo test -p clawft-kernel --test golden_snapshots \
  config_snapshots::default_config_snapshot               # ok
```

### New / key tests

- `config::tests::binding_thread_mode_defaults_to_deny`
- `system_prompt::tests::deny_mode_hard_refuses_on_mismatch`
- `system_prompt::tests::warn_only_mode_annotates_mismatch`
- `loop_core::tests::handle_turn_hard_refuses_on_binding_thread_mismatch_deny_mode`
- `loop_core::tests::handle_turn_continues_on_binding_thread_mismatch_warn_only`
- `governance::tests::binding_thread_rule_denies_mismatch`
- `governance::tests::binding_thread_rule_permits_ok`
- `governance::tests::binding_thread_default_rules_shape`
- `governance::tests::genesis_has_26_rules`

## How to test (for tester)

```bash
# From worktree on branch wave0h/weft-342-binding-refuse
scripts/build.sh check
cargo test -p clawft-core --lib binding_thread
cargo test -p clawft-core --lib system_prompt
cargo test -p clawft-kernel --lib governance::binding_thread
cargo test -p clawft-kernel --lib governance::genesis_has_26
cargo test -p clawft-types --lib binding_thread

# Operator config (warn-only legacy):
# [agents]
# binding_thread_mode = "warn_only"
```

## Worktree

- Path: `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bee-7ac0-92b1-c50b4060c1dc`
- Branch: `wave0h/weft-342-binding-refuse`
- Commit: see branch tip (`git rev-parse wave0h/weft-342-binding-refuse`)
