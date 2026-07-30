# WEFT-137 result — Chain-anchored anchoring beyond MockAnchor

**Ticket:** WEFT-137  
**Branch:** `wave0h/weft-137-chain-anchor`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bee-7ac0-92b1-c4e34a19e4e7`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-137 (wave-0h)

## Problem

`ChainAnchor` in `crates/clawft-kernel/src/chain.rs` only had `MockAnchor`.
No durable / external-ledger path was wired, and ADR-041 left backend
selection open (OpenTimestamps / Ethereum / consortium). Audit gap:
`.planning/reviews/0.7.0-release-gate/02-kernel-governance.md` row #48.

## What shipped

### Useful default: file ledger (custom hash chain)

`FileLedgerAnchor` — append-only JSONL ledger of ExoChain head hashes,
each entry hash-linked with SHAKE-256:

```
entry_hash = shake256(seq_be ‖ prev_entry_hash ‖ anchored_hash ‖ ts_secs_be)
```

- `anchor` appends + fsyncs a line; `tx_id = file-ledger-{seq}`
- `verify` reloads the ledger, matches `tx_id`, recomputes commitment
- `verify_chain` walks the whole ledger for integrity

This is production-useful for air-gapped and operator-local audit without
picking a public blockchain yet.

### External ledger stub (real wiring, deferred network)

`ExternalLedgerAnchor` — durable intent JSONL + optional `endpoint`
binding. Receipts use `external-stub:{endpoint|pending}:{seq}`. Network
POST is intentionally not performed until a target ledger is chosen
(ADR-041); the config + trait surface are ready for a transport drop-in.

### Frequency policy + controller

`AnchoringController` wraps any `ChainAnchor` with:

| Gate | Config field | Default |
|------|--------------|---------|
| Wall-clock | `min_interval_secs` | 300 |
| Event delta | `min_events_between` | 100 |

- `try_anchor` / `try_anchor_chain_head` respect policy (`Ok(None)` when suppressed)
- `force_anchor` bypasses policy
- Successful head anchors also append `chain` / `external_anchor` audit events

### Kernel config

```toml
[kernel.chain.external_anchor]
backend = "file"          # none | mock | file | external
ledger_path = "~/.clawft/chain/anchors.jsonl"
# endpoint = "https://…"  # external backend only
min_interval_secs = 300
min_events_between = 100
```

Types: `ChainAnchorBackend`, `ChainExternalAnchorConfig` on
`ChainConfig::external_anchor` (`clawft-types`).

### Boot wiring

When `external_anchor` is set and chain is enabled, boot builds
`AnchoringController`, logs backend + policy, best-effort anchors the
current head, and exposes `Kernel::chain_anchor()`.

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-types/src/config/kernel.rs` | `ChainExternalAnchorConfig`, backend enum, field on `ChainConfig`, serde tests |
| `crates/clawft-kernel/src/chain_anchor.rs` | **new** — FileLedger, ExternalStub, policy, controller, 14 unit tests |
| `crates/clawft-kernel/src/chain.rs` | Trait docs list non-mock impls |
| `crates/clawft-kernel/src/lib.rs` | `mod chain_anchor` + re-exports |
| `crates/clawft-kernel/src/boot.rs` | `ChainSubsystem.chain_anchor`, boot build, `Kernel::chain_anchor` |
| `crates/clawft-kernel/tests/*` + weave daemon tests | `external_anchor: None` on `ChainConfig` literals |
| `docs/plans/wave-0h-WEFT-137-result.md` | this report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Concrete anchor against an external ledger (custom hash chain / stub) | **Done** — `FileLedgerAnchor` (hash-linked ledger) + `ExternalLedgerAnchor` stub |
| Configurable via kernel config | **Done** — `kernel.chain.external_anchor` |
| Anchoring frequency policy | **Done** — `min_interval_secs` + `min_events_between` |
| Tests with mock ledger | **Done** — existing `MockAnchor` + controller mock path |

## Tests

**`clawft-kernel` (`chain_anchor::tests`) — 14**

- `file_ledger_anchor_roundtrip_and_chain`
- `file_ledger_verify_rejects_unknown_tx`
- `file_ledger_detects_tamper`
- `external_stub_records_intent_with_endpoint`
- `external_stub_pending_without_endpoint`
- `frequency_policy_event_gate` / `frequency_policy_time_gate`
- `controller_try_anchor_respects_policy`
- `controller_from_config_{file,none,mock,external}`
- `try_anchor_chain_head_emits_audit_event`
- `mock_anchor_still_works_via_controller`

**`clawft-types` (config tests) — 3**

- `chain_external_anchor_defaults`
- `chain_config_external_anchor_serde`
- `chain_external_anchor_camel_case_and_external_backend`

**Existing** `chain::tests::mock_anchor_roundtrip` still green.

## Verification

```bash
cargo test -p clawft-kernel --lib chain_anchor
# 14 passed

cargo test -p clawft-kernel --lib mock_anchor
# 2 passed

cargo test -p clawft-types --lib chain_external_anchor
# 3 passed

scripts/build.sh check
# ok
```

## Follow-ups

- Real OpenTimestamps / Ethereum transport behind `ExternalLedgerAnchor`
  once a ledger target is chosen (replaces intent-only submit).
- Optional periodic re-anchor task (timer service) beyond boot-time +
  on-demand `try_anchor_chain_head`.
- RPC surface to force/inspect external anchors for operators.
