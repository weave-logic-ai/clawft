# ExoChain Security Plan — Medium-Severity Closeout (WEFT-547)

**Date**: 2026-07-31  
**Tracker**: WEFT-547  
**Sources**:
- `.planning/development_notes/exochain-fix-plan.md` (8-agent / 48+ task matrix)
- `.planning/development_notes/exochain-certification-medium.md`
- `.planning/development_notes/exochain-certification-critical.md`
- `.planning/development_notes/exochain-certification-nonkernel.md`
- `.planning/development_notes/governance-certification.md`

## Objective

Walk every remaining **medium-severity** ExoChain instrumentation row from the
fix-plan / certification cap matrix. Close each with a code fix **or** an
explicit deferral. Update certification totals.

## Method (re-audit 2026-07-31)

For each medium row:
1. Locate the public state-mutating method.
2. Confirm event constant in `chain.rs` / `chain_event.rs` (or local graphify const).
3. Confirm `cm.append()` / `push_chain_event` / `chain_event!` / `tracing` target `"chain_event"`.
4. Confirm `#[cfg(feature = "exochain")]` where the call is compile-gated on kernel `ChainManager`.

## Medium-severity disposition

### Kernel matrix (`exochain-certification-medium.md` — 32 rows)

| Result | Count | Notes |
|--------|------:|-------|
| PASS | 31 | All prior PASS rows re-confirmed instrumented |
| CONDITIONAL PASS | 1 | `hnsw_service::load_from_file` static ctor — `load_from_file_logged` is the chain path |
| FAIL | **0** | **`wasm_runner/tools_fs.rs` remediated** (write/create_dir/remove/copy/move emit `wasm.fs.*`) |

**Item 32 remediation (code already on branch):**  
`FsWriteFileTool`, `FsCreateDirTool`, `FsRemoveTool`, `FsCopyTool`, `FsMoveTool` call
`clawft_core::chain_event::push_chain_event` under `#[cfg(feature = "exochain")]` with kinds
matching `EVENT_KIND_WASM_FS_*` in `chain.rs`. Read-only tools (read/stat/exists/glob/read_dir)
intentionally omit mutation audit events.

### Fix-plan agents 5–8 (medium groups)

| Agent | Scope | Disposition |
|------:|-------|-------------|
| 5 | causal + artifacts | **CLOSED — fixed** (all methods `cm.append`) |
| 6 | container/process/wasm/agency | **CLOSED — fixed** (incl. wasm fs mutations) |
| 7 | mesh + persistence + reconciler | **CLOSED — fixed** |
| 8 | sandbox/session/workspace/tools/graphify/init | **CLOSED — fixed** (non-kernel `chain_event!` / tracing) |

Agents 1–4 (critical/high) were out of WEFT-547 scope; certified separately.

### Governance medium gap (from `governance-certification.md`)

| Gap | Severity | Disposition |
|-----|----------|-------------|
| GAP-1 `config_service::delete_typed` lacked gate | medium | **CLOSED — fixed** (`GateBackend::check` present before mutation) |
| GAP-2 sandbox GovernanceGate | low | **DEFERRED** — intentional architecture (sandbox = enforcement); optional defense-in-depth later |
| GAP-3 `cron::remove_job` governance | low | **DEFERRED** — chain audit exists via `EVENT_KIND_CRON_REMOVE`; governance parity with `add_job` is 0.9.x polish |

## Explicit deferrals (recorded)

| Item | Reason | Target |
|------|--------|--------|
| Full K6 ChainManager → `exo-dag::DagStore` cutover | Depends on WEFT-619 substrate crates + hash re-anchor | 0.9.x+ (ADR-043) |
| GAP-2 sandbox↔GovernanceGate bridge | Architectural separation is correct today | optional / later |
| GAP-3 `remove_job` governance gate | Low risk; audit event already present | 0.9.x |

## Certification totals (post closeout)

| Document | Before | After |
|----------|--------|-------|
| Medium kernel rows FAIL | 1 | **0** |
| Medium kernel PASS + CONDITIONAL | 31 | **32** (31 PASS + 1 CONDITIONAL) |
| Governance high-priority gates | 14/14 | 14/14 |
| Governance medium gaps open | 1 | **0** (GAP-1 fixed) |

## Artifacts updated

- This file (`exochain-security-plan.md`) — WEFT-547 closeout ledger
- `exochain-certification-medium.md` — item 32 → PASS; summary counts
- `exochain-fix-plan.md` — medium agent checkboxes marked done
- `governance-certification.md` — GAP-1 closed; totals refreshed

## Verdict

**WEFT-547 medium-severity rows are closed.** No open medium FAIL remains in the
kernel certification matrix. Residual items are low-severity deferrals or the
separate K6 hash-migration epic (WEFT-619 / ADR-043).
