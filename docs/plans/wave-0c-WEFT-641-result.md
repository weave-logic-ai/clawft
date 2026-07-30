# WEFT-641 result — AtomRegistry + atom.locate + cross-index audit (ADR-069)

**Ticket:** WEFT-641  
**Branch:** `wave0c/weft-641-atom-registry`  
**Date:** 2026-07-30  
**Disposition:** **Shipped** (vertical slice: audit + locate + mint + RPC; GUI deferred)

## Summary

Implements ADR-069 (Panopticon) as a minimal complete slice:

1. **`AtomRegistry`** in `clawft-service-agent` — two O(1) DashMaps (`by_seq`, `by_uid`) plus durable substrate sibling (`chain_seq ↔ turn_id`).
2. **Mint at `KernelTurnAnchor::anchor_turn`** — fire-and-forget after `chain.append` (only seam where all keys co-exist). Optional wiring (`with_atom_registry`); absence is supported (P3).
3. **`audit` / `audit_projections`** — map agreement + flags democritus-style `chain_seq = 0` projection rows (live defect class for WEFT-642).
4. **Daemon RPCs** `atom.locate` / `atom.audit` — read-only; require registry wired via `anchor_chain`.

GUI cross-navigation deferred (ADR-069 slice 3).

## APIs shipped

| Surface | Location | Notes |
|---------|----------|-------|
| `AtomLocator` | `clawft_service_agent::AtomLocator` | coords only (no text/vectors) |
| `Disposition` | `Committed \| Superseded \| Pruned \| AbandonedBranch` | P2 lifecycle |
| `AtomKey` | `ByChainSeq(u64) \| ByUid(String)` | O(1) keys |
| `AtomRegistry::record` | once at anchor | updates maps + sibling |
| `AtomRegistry::locate` | O(1) | never fails on pruned disposition |
| `AtomRegistry::mint_and_record` | anchor helper | derives uid via `turn_universal_id` |
| `AtomRegistry::set_disposition` | lifecycle update | never deletes locator |
| `AtomRegistry::turn_id_for_seq` / `chain_seq_for_turn` | substrate sibling | durable binding |
| `AtomRegistry::audit` / `audit_projections` | ConsistencyReport | flags `chain_seq_zero` |
| `KernelTurnAnchor::with_atom_registry` | optional inject | fire-and-forget mint |
| RPC `atom.locate` | daemon | `{ chain_seq? \| uid? }` → locator JSON |
| RPC `atom.audit` | daemon | optional `projections[]` samples |

## Files

| Path | Change |
|------|--------|
| `crates/clawft-service-agent/src/atom_registry.rs` | **new** — registry + types + unit tests |
| `crates/clawft-service-agent/src/lib.rs` | export module + types |
| `crates/clawft-service-agent/src/substrate_sink.rs` | optional registry field + mint after chain append |
| `crates/clawft-service-agent/tests/atom_registry.rs` | **new** — anchor mint + audit integration |
| `crates/clawft-weave/src/daemon.rs` | `DAEMON_ATOM_REGISTRY`, boot wire, RPC arms |

## Acceptance map

| AC (ADR-069) | Status |
|--------------|--------|
| locate(ByChainSeq) / locate(ByUid) O(1) and agree | ✅ unit + integration |
| Superseded/Pruned still locatable (P2) | ✅ `set_disposition` + unit test |
| Registry removable; no turn failure (P3) | ✅ optional field; integration test without registry |
| locate/audit read-only over projections (P3) | ✅ no projection mutation paths |
| audit flags chain_seq=0 (democritus class) | ✅ `chain_seq_zero` Error |
| substrate durable chain_seq↔turn_id | ✅ sibling maps |
| locator holds no turn text/vectors | ✅ struct + serialize assertion |

## Tests / build

```text
cargo test -p clawft-service-agent atom_registry
  → 6 unit tests ok
cargo test -p clawft-service-agent --test atom_registry
  → 3 integration tests ok
scripts/build.sh check
  → green
```

## Follow-ups

- **WEFT-642** — fix democritus HNSW to thread real `chain_seq` (audit already guards regression).
- **GUI wiring** — graph view / scrubber / semantic pane → `atom.locate` (ADR-069 slice 3).
- Optional durable sibling persistence to substrate JSONL (in-mem sibling is sufficient for reverse resolve this slice).
- Disposition upkeep hooks on prune/supersede paths (registry still resolves; label may lag until wired).
