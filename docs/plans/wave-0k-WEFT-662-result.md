# WEFT-662 result — upstream rvf-runtime 0.2 bug reports

**Branch:** `wave0k/weft-662-rvf-bugs`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4dd-9e94-7350-bce2-784afa1080db`  
**Date:** 2026-07-30  
**Agent:** coder-662 (Wave 0k)

## Summary

Acceptance for WEFT-662 was **filing three upstream bugs** against `rvf-runtime`
(with repro snippets) and documenting local workarounds — not re-implementing
the workarounds (those already shipped with WEFT-616 / cow-memory).

All three bugs were verified against crates.io source **0.2.0** (workspace pin)
and confirmed still present in **0.3.0**. Issues filed on the crate’s declared
repository `ruvnet/RuVector`.

## Upstream issues (done)

| # | Bug | GitHub |
|---|-----|--------|
| 1 | macOS link failure — hard-coded glibc `__errno_location` in `locking.rs` | https://github.com/ruvnet/RuVector/issues/746 |
| 2 | `RvfStore::open()` resets `metric` to L2 and zeroes `last_witness_hash` | https://github.com/ruvnet/RuVector/issues/747 |
| 3 | `delete()` bitmap permanent — re-ingest does not undelete; `compact()` drops re-ingested data | https://github.com/ruvnet/RuVector/issues/748 |

## Files changed

| File | Change |
|------|--------|
| `docs/research/rvf-runtime-0.2-upstream-bugs.md` | Full bug reports, repros, expected fixes, local workarounds, version matrix, removal plan |
| `docs/plans/wave-0k-WEFT-662-result.md` | This result |

No production Rust changes in this ticket. Local workarounds were already in tree:

| Bug | Workaround (pre-existing) |
|-----|---------------------------|
| 1 | `crates/clawft-cow-memory/src/macos_errno_shim.rs` — sole `#[no_mangle]` `__errno_location` → `libc::__error()` on macOS. Dual-shim in `clawft-core` intentionally **removed** earlier (fat-LTO multiply-define; WEFT-615). Consolidation complete. |
| 2 | `crates/clawft-cow-memory/src/normalize.rs` — always L2 + L2-normalize ingest/query (agenticow discipline). Witness-hash reopen residual documented on `PromoteReport.parent_hash`. |
| 3 | `BranchableMemory::delete` does **not** call `RvfStore::delete` on working tips; crate-level tombstones + chain-walk mask (`branchable_memory.rs`, `chain_walk.rs`, `tests/tombstone.rs`). Promote-to-base still uses RVF delete (documented limitation in `ops.rs`). |

## Acceptance checklist

- [x] Three upstream issues filed with repro snippets  
- [x] Issue URLs recorded in-repo (`docs/research/…` + this result)  
- [x] Local workarounds documented (and confirmed consolidated for errno shim)  
- [ ] Plane work item WEFT-662 closed with issue URLs + commit SHA (lead / plane workflow)  
- [ ] Upstream fixes land → follow removal plan in research doc; bump `rvf-runtime`

## How to verify

```bash
# Issues exist and are open
gh issue view 746 -R ruvnet/RuVector
gh issue view 747 -R ruvnet/RuVector
gh issue view 748 -R ruvnet/RuVector

# Docs present
test -f docs/research/rvf-runtime-0.2-upstream-bugs.md
test -f docs/plans/wave-0k-WEFT-662-result.md

# Existing workarounds still compile (optional smoke)
# scripts/build.sh check
# cargo test -p clawft-cow-memory
```

No code path change → full `scripts/build.sh test` not required for AC; research
docs only. Recommend lead still run `scripts/build.sh check` on merge wave if
desired.

## Commit

- **Branch:** `wave0k/weft-662-rvf-bugs`
- **Message:** `docs(rvf): WEFT-662 report upstream rvf-runtime 0.2 bugs (#746–#748)`
- **SHA:** branch tip of `wave0k/weft-662-rvf-bugs` (message includes WEFT-662)
- **No push** (wave protocol)

## Residual / follow-ups

1. **Plane close** — attach issue URLs + commit SHA to WEFT-662; transition Done.
2. **rvf-runtime upgrade** — when #746–#748 land, bump workspace dep, delete
   `macos_errno_shim` if safe, optionally adopt durable Cosine / RVF delete.
3. **Vector-by-id gap** — still no public getter on 0.2 `RvfStore` (handoff note);
   not one of the three AC bugs; file separately if needed.
4. **Promote path** — `ops.rs` still calls `RvfStore::delete` on `base` and
   inherits sticky-bitmap risk until #748 is fixed.
5. **0.3.x** — bugs still present; no reason to rush pin bump for these alone.
