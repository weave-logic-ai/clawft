# WEFT-472 — Element 10 tracker reconciliation (ClawHub vs deployment)

**Date**: 2026-07-31  
**Status**: Accepted  
**Ticket**: WEFT-472  
**Branch**: `docs/weft-472-element10`  
**Cycle**: 0.8.x · **Label**: ws14-deployment · **Lane**: planning/governance  
**Source audit**: `.planning/reviews/0.7.0-release-gate/14-deployment-release.md` (Orphaned Work)  
**Tracker**: `.planning/sparc/phase4/10-deployment-community/04-element-10-tracker.md`

---

## Problem

Element 10 (“Deployment & Community”) is marked **COMPLETE** for K2–K5, but the tracker still lists ClawHub product features (`weft skills` publish/install, Ed25519 signing, vector search) as if they were deployment-owned. That blurred **ws14-deployment** with **ws04-plugin-skills**, security, and memory search — and left the 0.7.0 audit orphan open.

## Decision (ownership)

Historical SPARC workstream **K** bundled Docker, sandbox, security plugin, ClawHub, and benchmarks in one Element. For **ongoing Plane ownership**, split as follows:

| K unit | What it is | Ongoing owner | Not ws14 |
|--------|------------|---------------|----------|
| **K2 / K2-CI** | Multi-arch Docker, VPS scripts, PR gates, release pipeline, GHCR | **ws14-deployment** | — |
| **K3** | Per-agent sandbox (WASM + OS), `SandboxPolicy` | **ws04-plugin-skills** (runtime enforcement also touches core/kernel) | Product security surface, not release eng |
| **K3a** | `clawft-security` checks + `weft security scan` | **ws04-plugin-skills** / security tooling | Same |
| **K4** | ClawHub client, skill publish/install CLI, Ed25519 skill signing, community star/comment | **ws04-plugin-skills** (primary); dashboard bridge **ws09** (WEFT-301) | **Not** deployment |
| **K4 · vector search** | Local HNSW / keyword fallback for skill search | **ws06-memory** (H2 / embedder) + **ws04** (ClawHub consumer) | Not deployment |
| **K5** | Benchmarks vs OpenClaw, MVP skills under `skills/` | **ws14-deployment** for CI/bench *release quality*; skill content **ws04** | Split: scripts vs skill packs |

**Rule of thumb**: anything that ships skills, trust roots, or marketplace UX is **ws04** (or **ws09** for HTTP/UI bridge). Anything that ships containers, CI gates, or release artifacts is **ws14**. Element 10 COMPLETE remains valid as a **historical sprint** status; it is not a claim that remaining ClawHub gaps are ws14 work.

## Remaining work (de-duplicated)

| Item | Where tracked | Owner | Notes |
|------|---------------|-------|-------|
| Agent auto-search when local skill miss → ClawHub | Element 10 K4 checkbox still open; **no separate WEFT required for ownership** — product follow-up under **ws04** if revived | ws04 | Depends on skill loader + agent loop; not release gate |
| Dashboard `/skills` install/uninstall → real loader | **WEFT-301** (Todo, 1.0.x) | ws09 | Bridge TODOs; depends on loader + ClawHub path |
| Skill signing trust root / rotation docs | **WEFT-69** (Done) | ws04 | Already closed under plugin-skills |
| Shell skill approval at install | **WEFT-63** (Done) | ws04 | Install-time security |
| Gateway bridge skill install stubs | **WEFT-168** (Done) | ws05 | Earlier bridge work; residual dashboard gap is WEFT-301 |
| Docker / CI / release cadence items | WEFT-441…WEFT-550 cluster under **ws14-deployment** | ws14 | True deployment orphans stay on ws14 |

No second Element-10 tracker row is needed for K4 product work: **do not file new ws14 tickets for ClawHub**. Prefer **ws04** / **ws09** / **ws06** labels.

## Tracker + audit actions (this ticket)

1. **Tracker** (`.planning/sparc/phase4/10-deployment-community/04-element-10-tracker.md`): add ownership matrix, re-home K4 remaining work, keep COMPLETE as historical.
2. **Audit** (`14-deployment-release.md` Orphaned Work): mark Element 10 row **closed by WEFT-472**.
3. **This note**: canonical reconciliation record under `docs/plans/`.

## Acceptance criteria

| AC | Status |
|----|--------|
| Workstream ownership for ClawHub features clarified (deployment vs security vs community) | **Done** — table above |
| Tracker updated for ownership + remaining work | **Done** — Element 10 tracker amended |
| De-duplication vs other workstream trackers / Plane items | **Done** — WEFT-301 / 63 / 69 / 168 mapped; no ws14 ClawHub clones |
| Audit row marked closed with WEFT-N | **Done** — WEFT-472 |

## Out of scope

- Implementing agent auto-search or WEFT-301 bridge wiring.
- Plane API state transitions (operators may mark WEFT-472 Done with this commit SHA).
- Empty `development_notes/10-deployment-community/phase-K-*` stubs (separate orphan in the same audit).

## Files

| Path | Change |
|------|--------|
| `docs/plans/weft-472-element10-reconciliation.md` | This note (new) |
| `.planning/sparc/phase4/10-deployment-community/04-element-10-tracker.md` | Ownership + remaining-work section |
| `.planning/reviews/0.7.0-release-gate/14-deployment-release.md` | Orphaned row closed → WEFT-472 |
