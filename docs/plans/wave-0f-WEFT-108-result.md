# Wave 0f — WEFT-108 result

**Ticket**: ws02: mesh — implement IpcScope::Restricted browser default + browser_policy rules  
**Branch**: `wave0f/weft-108-browser-ipc-scope`  
**Base**: `release/0.8-staging`  
**Commit**: `d9c7abd449005761be7bbfb1d7f012fa61d9cfda`  
**Status**: implemented; clawft-kernel lib tests green  
**Pushed**: no (lead merge)

## What shipped

1. **Public topics + Restricted topic scope** (`capability.rs`)
   - `is_public_topic(topic)` — `"public"` or `public.*` prefix.
   - `IpcScope::Restricted` topic access limited to public topics (PID allowlist unchanged).
   - `AgentCapabilities::browser_default()` already used `Restricted([])`; now topic checks enforce S7.

2. **`browser_policy` rule type** (`governance.rs`)
   - `GovernanceRuleType::{General, BrowserPolicy}` on `GovernanceRule.rule_type` (serde default = General).
   - `GovernanceRule::browser_policy()`, `browser_policy_default_rules()` → BP-001..003.
   - `GovernanceEngine::evaluate` applies BrowserPolicy rules platform-aware and **independent of effect magnitude**.
   - Helpers: `filter_by_type`, `browser_policy_rules()`.

3. **Genesis anchoring** (`boot.rs`)
   - BP-001..003 chained into genesis rules (22 → **25** rules).
   - Chain payload includes `rule_type`.

4. **Docs**
   - `docs/guides/browser.md` (new) — defaults, public topics, browser_policy, operator notes.
   - `docs/weftos/kernel-governance.md` — Restricted table + API types updated.

## Acceptance criteria

| AC | Status |
|----|--------|
| Browser builds default to IpcScope::Restricted | done (`browser_default()` + tests) |
| browser_policy rule type in governance | done (`GovernanceRuleType::BrowserPolicy`, BP-001..003) |
| Test: browser node cannot subscribe to non-public topics | done (`browser_node_cannot_subscribe_to_non_public_topics` + governance BP tests) |
| Documented in docs/guides/browser.md | done |

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/capability.rs` | `is_public_topic`, Restricted topic limit, browser topic tests |
| `crates/clawft-kernel/src/governance.rs` | rule type, BP rules, evaluate path, genesis 25, tests |
| `crates/clawft-kernel/src/boot.rs` | genesis +3 BP rules, rule_type in chain payload |
| `crates/clawft-kernel/src/gate.rs` | `rule_type` field on rule literals |
| `crates/clawft-weave/src/daemon.rs` | `rule_type` field on rule literals |
| `crates/clawft-service-agent/tests/witness_chain.rs` | `rule_type` field on rule literal |
| `docs/guides/browser.md` | new guide |
| `docs/weftos/kernel-governance.md` | Restricted + rule type docs |

## Tests run

```text
scripts/build.sh check                         # ok
cargo test -p clawft-kernel --lib              # 2068 passed, 2 ignored
cargo test -p clawft-kernel --lib browser_     # 15 passed
cargo test -p clawft-kernel --lib capability:: # 49 passed
cargo test -p clawft-kernel --lib governance:: # 81 passed
cargo check -p clawft-weave                    # ok
cargo check -p clawft-service-agent            # ok
```

### Note

`scripts/build.sh test clawft-kernel` hit one **pre-existing** golden snapshot failure
(`config_snapshots::default_config_snapshot`) for unrelated config fields
(`max_grantable_level`, `skills.autogen`, discord chunking, etc.). Not introduced by
WEFT-108. All lib unit tests pass.

### New / key tests

- `capability::tests::browser_node_cannot_subscribe_to_non_public_topics`
- `capability::tests::restricted_scope_limits_topics_to_public`
- `capability::tests::is_public_topic_prefix_rules`
- `governance::tests::browser_policy_denies_non_public_topic_for_browser`
- `governance::tests::browser_policy_allows_public_topic_for_browser`
- `governance::tests::browser_policy_does_not_apply_to_native_platform`
- `governance::tests::browser_policy_denies_spawn_for_browser`
- `governance::tests::browser_policy_default_rules_shape`
- `governance::tests::filter_by_browser_policy_type`
- `governance::tests::genesis_has_25_rules`

## How to test (for tester)

```bash
# From worktree on branch wave0f/weft-108-browser-ipc-scope
scripts/build.sh check
cargo test -p clawft-kernel --lib browser_
cargo test -p clawft-kernel --lib capability::
cargo test -p clawft-kernel --lib governance::

# Focused AC test:
cargo test -p clawft-kernel --lib browser_node_cannot_subscribe_to_non_public_topics
cargo test -p clawft-kernel --lib browser_policy_denies_non_public_topic_for_browser
```

## Worktree

- Path: `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb478-7733-7932-bf2f-8ff6a8f5ee11`
- Branch: `wave0f/weft-108-browser-ipc-scope`
- Commit: `d9c7abd449005761be7bbfb1d7f012fa61d9cfda`
