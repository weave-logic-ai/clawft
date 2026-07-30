# WEFT-324 result — Public `chain.append` RPC for WitnessRecord

**Ticket:** WEFT-324  
**Branch:** `wave0g/weft-324-chain-append-rpc`  
**SHA:** branch tip (`git rev-parse wave0g/weft-324-chain-append-rpc`)  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb489-7de2-72f2-89c6-2bb2e0dd69cf`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-324 (wave-0g)

## Problem

`weaver soul promote` wrote a `WitnessRecord` only to
`<workspace>/.weftos/audit/soul-promote.log` (JSONL) plus a
`tracing::info!(target = "chain_event", ...)` event, because the daemon
did not expose a public `chain.append` RPC. The soul-cmd trait shape was
already forward-compatible; the TODO(`agent-core-v1.1`) marked the gap.

## What shipped

### `clawft-service-agent` — wire types (`protocol.rs`)

| Type | Role |
|------|------|
| `WitnessRecord` | Canonical witness payload (`kind`, `entries`, `hash_before`, `hash_after`, `ts`) |
| `ChainAppendParams` | `{ source (default `"soul"`), record: WitnessRecord }` |
| `ChainAppendResult` | `{ sequence, chain_id, hash, source, kind }` |

Re-exported from `clawft_service_agent::{WitnessRecord, ChainAppendParams, ChainAppendResult}`.

### `clawft-weave` — daemon + capability

| Item | Detail |
|------|--------|
| `chain.append` RPC arm | Deserializes `ChainAppendParams`, rejects empty `kind`, proxies to `ChainManager::append(source, kind, payload)` under `exochain` |
| Capability | `Write` (anonymous denied; UDS clients get implicit `admin` from `DaemonClient::call`) |
| Protocol re-exports | Weave `protocol` re-exports the service-agent types for chain callers |

### `weaver soul promote`

| Item | Detail |
|------|--------|
| Primary path | `chain.append` RPC with `source = "soul"` |
| Offline / redundant | Always writes local audit JSONL first; RPC failure is warned, promote still succeeds |
| TODO | `TODO(agent-core-v1.1)` removed |

## Acceptance

| Criterion | Status |
|-----------|--------|
| New daemon RPC arm accepting a WitnessRecord payload | Yes |
| New types in `clawft-service-agent/src/protocol.rs` | Yes |
| `weaver soul promote` switched to RPC | Yes (with local audit retained) |
| Local-audit-log fallback retained as offline mode | Yes (always written; sole durable path if RPC fails) |
| Unit + integration tests covering the new RPC arm | Yes |
| `TODO(agent-core-v1.1)` removed | Yes |

## Tests

**`clawft-service-agent` (`protocol::tests`)**

- `chain_append_params_round_trip`
- `chain_append_params_default_source`

**`clawft-weave` (`capability::tests`)**

- `chain_append_requires_write` (+ listed in write-methods table)

**`clawft-weave` (`daemon::tests`, `exochain`)**

- `chain_append_rpc_witnesses_soul_promote` — boots kernel with chain, dispatches RPC, asserts event on `ChainManager`
- `chain_append_rpc_rejects_empty_kind`

**`clawft-weave` (`commands::soul_cmd::tests`)**

- Existing promote / audit-log suite still green (7 tests)

## Verification

```text
cargo check -p clawft-service-agent -p clawft-weave
# ok

cargo test -p clawft-service-agent --lib protocol
# 2 passed

cargo test -p clawft-weave --lib capability::
# 10 passed (incl. chain_append_requires_write)

cargo test -p clawft-weave --lib commands::soul_cmd
# 7 passed

cargo test -p clawft-weave --lib daemon::tests::chain_append
# 2 passed
```

## Files changed

- `crates/clawft-service-agent/src/protocol.rs`
- `crates/clawft-service-agent/src/lib.rs`
- `crates/clawft-weave/src/capability.rs`
- `crates/clawft-weave/src/protocol.rs`
- `crates/clawft-weave/src/daemon.rs`
- `crates/clawft-weave/src/commands/soul_cmd.rs`
- `docs/plans/wave-0g-WEFT-324-result.md` (this file)

## Notes

- Cross-listed cancelled tickets (WEFT-17 / WEFT-81 / WEFT-501) described the same gap; this ticket is the ws11 load-bearing implementation.
- Agent journal write path remains a separate task (WEFT-325 / journal hook); it can reuse `chain.append` with `source = "agent"` once ready.
- Local UDS clients do not need an explicit auth token (`DaemonClient` attaches `admin`). Non-UDS / restricted scopes must present a write-or-admin token.
