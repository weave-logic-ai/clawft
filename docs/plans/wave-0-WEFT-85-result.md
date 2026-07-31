# WEFT-85 result — emit `session.append` chain events on every turn

**Branch:** `wave0/weft-85-session-append-chain-event`  
**Base:** `release/0.8-staging`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb68f-9e16-77f3-a9f3-b7f071f59fc3`  
**Date:** 2026-07-31  
**Status:** Implemented (unit tests green; ready for tester)

## Ticket

ws06: substrate — emit `chain_event!` for `session.append` on every appended
turn (MW-7 / WS-O5).

`session.create` only fired on cache+disk miss. Long-running sessions looked
like create → destroy with no hot-loop activity on the ExoChain.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Emit `session.append` on every appended turn | **Done** |
| Sample-rate cap if volume is too high (configurable) | **Done** (`with_append_event_every_n`) |
| Test: 100 turns → 100 events (or sampled count) | **Done** |
| Documented in governance.md | **Done** (`docs/weftos/kernel-governance.md`) |

## What shipped

### 1. Event kind constants

- `clawft_core::chain_event::EVENT_KIND_SESSION_APPEND = "session.append"`
- `clawft_kernel::chain::EVENT_KIND_SESSION_APPEND` (mirrored vocabulary)

### 2. `SessionManager::append_turn`

After a successful write (new file or append), emits:

```text
chain_event!(session, session.append, { key, role, turn_count })
```

Message content is intentionally omitted (PII / volume).

### 3. Sample-rate cap

```rust
SessionManager::with_dir(platform, dir)
    .with_append_event_every_n(10); // 100 turns → 10 events
```

Default `n = 1` (every turn). Counter is per-manager `AtomicU64`.

### 4. Governance docs

`docs/weftos/kernel-governance.md` — new subsection **Session lifecycle chain
events (WEFT-85 / MW-7)** under chain logging: create / append / destroy table,
sample-rate, payload fields, constant locations.

## Tests

```bash
cargo nextest run -p clawft-core -E 'test(session::) | test(chain_event::)'
# 23 passed

# Focused WEFT-85:
# - append_turn_emits_session_append_chain_event_every_turn  (100→100)
# - append_turn_sample_rate_caps_chain_events               (100→10 @ every_n=10)
# - session_lifecycle_chain_events_include_append           (create+2 append+destroy)
# - constants_match_expected_values                         (includes session.append)
```

`scripts/build.sh test clawft-core`: session/chain_event green. One
**pre-existing** failure unrelated to this change:

- `workspace::config::tests::load_merged_config_mcp_servers` —
  `invalid type: null, expected struct MCPServerConfig` (MCP config serde;
  not touched by WEFT-85).

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/src/chain_event.rs` | `EVENT_KIND_SESSION_APPEND` + constant test |
| `crates/clawft-core/src/session.rs` | emit on append; sample rate; 3 tests |
| `crates/clawft-kernel/src/chain.rs` | mirrored constant |
| `docs/weftos/kernel-governance.md` | lifecycle events section |
| `docs/plans/wave-0-WEFT-85-result.md` | this report |

## How to test (for tester)

```bash
# From worktree / branch above
cargo nextest run -p clawft-core -E 'test(append_turn_emits) | test(append_turn_sample) | test(session_lifecycle_chain) | test(constants_match_expected)'
# or
scripts/build.sh test clawft-core   # note pre-existing MCP config fail
```

## Close notes (for lead)

- Plane UUID: `133a6be5-bc71-4e08-ab82-9f7b75593731` (In Progress)
- Close after merge with `plane.sh close … --shipped "session.append chain events" --commits <sha> --tests "session+chain_event 23 pass"`
- LocalFileSink path does **not** emit (out of scope; SessionManager substrate only)
- Commit on feature branch when lead/tester greenlight; do not merge to master
