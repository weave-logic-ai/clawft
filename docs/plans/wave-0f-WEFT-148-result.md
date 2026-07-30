# WEFT-148 result — Rate-limit add_peer() and governance-evaluation requests

**Branch:** `wave0f/weft-148-mesh-rate-limit`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb478-7734-71b1-b768-fb2a8302d4f6`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30

## Problem

No DoS protection on `add_peer()` (beyond a global 100 ms debounce) or on
governance evaluation. A malicious or buggy peer/agent could flood either path.
Audit: `.planning/reviews/0.7.0-release-gate/02-kernel-governance.md` row #59.

## What shipped

Shared keyed sliding-window rate limiter plus wiring on both hot paths.

| Surface | Keying | Default | Config |
|---------|--------|---------|--------|
| `ClusterMembership::add_peer` | **per source** — peer `address` if set, else `id` | 1 add / 100 ms / source | `ClusterConfig::{peer_add_max_per_source, peer_add_window_ms}` or `with_peer_add_rate_limit` / `with_min_peer_interval(0)` to disable |
| `GovernanceEngine::evaluate` | **per principal** — non-empty `agent_id`, else `node_id` | 256 evals / 1 s / principal | `with_eval_rate_limit` / `set_eval_rate_limit`; `GovernanceGate::with_eval_rate_limit` |

On limit exceeded:

- `add_peer` → `ClusterError::RateLimited`
- `evaluate` → `GovernanceDecision::Deny("rate limited: evaluation quota exceeded for principal '…'")` (no rule scoring)

`max_per_window = 0` disables enforcement (tests use this via
`with_min_peer_interval(Duration::ZERO)` / `RateLimitConfig::unlimited()`).

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/rate_limit.rs` | **new** — `RateLimitConfig`, `KeyedRateLimiter`, defaults, unit tests |
| `crates/clawft-kernel/src/lib.rs` | `pub mod rate_limit` |
| `crates/clawft-kernel/src/cluster.rs` | per-source limiter; serde config fields; builders; tests |
| `crates/clawft-kernel/src/governance.rs` | per-principal limiter on `evaluate`; builders; tests |
| `crates/clawft-kernel/src/gate.rs` | `with_eval_rate_limit` passthrough |
| `docs/plans/wave-0f-WEFT-148-result.md` | this report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Rate limit `add_peer` per source | Done — address-preferred source key |
| Rate limit governance-evaluation per principal | Done — agent_id / node_id key |
| Configurable thresholds | Done — `RateLimitConfig` + ClusterConfig fields + builders |
| Tests for limit enforcement | Done — rate_limit (5), cluster (4 new/updated), governance (4) |

## Verification

```bash
scripts/build.sh test clawft-kernel
# 2092 passed, 1 failed (pre-existing golden config snapshot drift:
#   clawft-kernel::golden_snapshots config_snapshots::default_config_snapshot
#   — adds workspace_root / discord nitro / skills.autogen / routing fields
#   unrelated to WEFT-148)

# Focused (from nextest log):
#   rate_limit::tests::* (5) PASS
#   cluster::tests::rate_limited_peer_additions PASS
#   cluster::tests::rate_limit_is_per_source_not_global PASS
#   cluster::tests::rate_limit_configurable_threshold PASS
#   cluster::tests::rate_limit_source_key_prefers_address PASS
#   governance::tests::eval_rate_limit_* (4) PASS
```

## Follow-ups

- Optionally surface peer-add / eval rate limits in `clawft.toml` / kernel config
  (ClusterConfig fields are serde-ready; KernelConfig cluster subsection does not
  yet forward them).
- Metrics counters for rate-limit hits (admin RPC).
- Golden snapshot `default_config` needs a separate refresh for unrelated config
  surface growth (not this ticket).
