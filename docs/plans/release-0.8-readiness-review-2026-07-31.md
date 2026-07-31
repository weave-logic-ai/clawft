# Release 0.8 readiness review — 2026-07-31

**Branch:** `release/0.8-staging` · **Tip at review:** `ed08fde8`+ (post medium/governance/dashboard harvest)  
**Scope:** finish residual board items (3)+(4), metaharness (rUv), ADR/design vs code, security/clean/build.

---

## 1. Board completion (this pass)

### 1.1 Optional 0.8 mediums (item 3) — **shipped**

| Ticket | Outcome | Evidence |
|--------|---------|----------|
| WEFT-516 | Done | SASE clustering in graphify (feature-selectable; LP default) |
| WEFT-531 | Done | Offline edge + streaming-merge training surfaces (stub residual ML) |
| WEFT-532 | Done | Per-sensor RVF model slots + tick hot-swap + AND-gate rollback |
| WEFT-547 | Done | ExoChain medium matrix re-walked: 0 FAIL, docs updated |
| WEFT-619 | Done | In-tree `exo-core` / `exo-dag` K6 contract stubs (no postgres) |

### 1.2 Governance end-states (item 4a) — **foundations shipped**

| Ticket | Outcome |
|--------|---------|
| WEFT-634 | Action/tool selectors on `GovernanceRule` |
| WEFT-633 | Spawn approval Defer path + design doc |
| WEFT-635 | ADR-094 user-level spawn permission story |
| WEFT-636 | `GatePrincipal` attribution on gate/chain |
| WEFT-637 | Tools-as-nodes spawn-edge design (existing D3 cited) |
| WEFT-638 | TalkForest retirement plan + `use_legacy_talk_forest` default-on |

### 1.3 1.0.x residual (item 4b) — **partial**

**Closed:** 565, 567, 568, 571, 566, 572, 573 (dashboard/PWA hygiene).

**Still open (honest multi-session / research):**

| Bucket | Tickets |
|--------|---------|
| Upstream-blocked (0.8) | 15, 45, 128, 194, 338, 352, 353, 657 |
| Product (1.0) | 301, 304, 305, 316, 574, 312, 560, 561→575 |
| Research (1.0) | 41, 57, 58, 538, 539 |

**Recommendation for cut:** treat 0.8 open set as **external-block only**; 1.0 product/research as post-release.

---

## 2. Metaharness / rUv analysis

### 2.1 `metaharness score` (path = repo root)

| Dimension | Score | Notes |
|-----------|------:|-------|
| harnessFit | **75** | Above 70 threshold |
| compileConfidence | **100** | Workspace check green |
| taskCoverage | **65** | Room to grow agent task maps |
| toolSafety | **90** | Default-deny MCP posture |
| memoryUsefulness | **51** | AgentDB memory underused in harness config |
| estCostPerRunUsd | 0.024 | |
| archetype | mcp-server-harness | |
| hardConstraints | 6/6 | scaffoldReady true |

### 2.2 OIA composite audit (dry-run)

| Component | Worst |
|-----------|-------|
| Composite | **clean** |
| Threat model | info (scanner saw limited MCP policy surface) |
| MCP scan | clean / no policy file at scan root |
| Genome risk_score | **0.085** (low) |
| publish_readiness | **1** (genome claim — verify with cargo-dist dry-run before tag) |

**rUv search takeaway (metaharness security.yml):** keep dual cargo-audit + npm-audit, optional SBOM, deny-warnings with explicit RUSTSEC ignore list — WeftOS already has `scripts/build.sh audit` / gate; ensure release notes list residual ignores.

### 2.3 Metaharness improvement specs (post-release harness)

1. **Raise taskCoverage (≥80):** mint metaharness tasks for gate / plane-dag / release-dry-run.  
2. **Raise memoryUsefulness:** store release gate outcomes + ADR compliance checks in AgentDB `patterns`.  
3. **OIA identity horizontal:** currently `none` — wire workspace identity to OIA manifest for federation demos.  
4. **Observability:** enable structured audit-log for MCP remote path (696/697) in production profile.  
5. **MCP scan path:** point metaharness at real `mcp-policy` / catalog so scans are not “mcp-disabled” false clean.

---

## 3. ADR / design vs code (sampled compliance)

| ADR / design | Code presence | Gap |
|--------------|---------------|-----|
| ADR-011 raw HNSW | `vector_hnsw.rs` | OK |
| ADR-056 BVH | `clawft-bvh` + kernel spatial_* | OK |
| ADR-088 VectorRef | `vector_ref.rs` + all payloads | OK |
| ADR-093 Phase F join | `vector_join.rs` | Live dual-backend service façade optional residual |
| ADR-078 world model | `world_model` + partition | Live `bvh_published` still false (export-first) |
| ADR-075 MCP | mcp-server listen + window tools | Issued-token mint live API residual |
| ADR-073 Workspace | freeform WM + WindowIntent | Full app embeds / live agent.spawn residual |
| ADR-077 Android | apps/android-splat-capture + UniFFI | Needs SDK machine for APK |
| ADR-094 spawn permission | ADR + GatePrincipal + selectors | Boot defaults for SPAWN-APPROVAL residual |
| ADR-068 TalkForest | legacy flag default on | Full cutover blocked Phase 1 residual |

**Verdict:** Accepted ADRs for 0.8 ship track have **implementation anchors**. Residuals are documented product depth, not missing crates.

---

## 4. Security / cleanliness / buildability

| Check | Result |
|-------|--------|
| `scripts/build.sh check` | **PASS** (incl. WEFT-504 ecc wasm reject) |
| Hardcoded API key literals (sampled) | **0** |
| TODO/FIXME/unimplemented density | ~60 across crates (debt, not ship-stop) |
| `unwrap()` in kernel | High in tests/paths — **pre-existing**; prefer `?` in new code only |
| Kernel warnings | Unused var fixed this pass; irrefutable let-chains remain (style) |
| Metaharness toolSafety | 90 |
| Genome risk | Low |

### Pre-tag release checklist (operator)

1. `scripts/build.sh gate` (full phase gate) on clean machine  
2. `scripts/build.sh release-dry-run` (cargo-dist host triple)  
3. `scripts/build.sh audit` + `npm-audit` — record residual RUSTSEC  
4. Manual smoke: `weft mcp-server --listen 127.0.0.1:8742` with token  
5. Manual smoke: Workspace mode freeform panes  
6. Optional: Windows named-pipe soak on real Windows host  
7. Optional: Android assembleDebug with SDK 35  

---

## 5. Release recommendation

| Question | Answer |
|----------|--------|
| Can we cut **0.8.x** from this branch after gate+dry-run? | **Yes**, with open set limited to **upstream-blocked** tickets |
| Blockers for “everything Done”? | External deps (ruvector #352, ruvllm-wasm, pocket-tts) + multi-session 1.0 product |
| Must-fix before tag? | Run full **gate** + **release-dry-run**; fix any red; no new high tickets open |

### Suggested release note themes

- Spatial: BVH complete Phases A–E, VectorRef, W1 partition, Phase F join  
- Mesh: Clock/fixtures/protocol, chain replay, Merkle tree, QUIC, capability claims  
- LeWM: crate stack, sensors, SIGReg, training surfaces (stub residual)  
- Product: Agent Workspace, MCP HTTP+window tools, Android capture scaffold  
- Governance: selectors, principals, spawn approval foundations  

---

## 6. Follow-up Plane hygiene (optional)

- Batch-comment upstream-blocked tickets with “0.8 ship residual / not release gate”  
- Defer pure research 41/57/58/538/539 deeper into 1.0 with no silent close  
- File residual tickets for: live BVH publish from W1, dual-backend SpatialService façade, Android NDK link, TalkForest default-off when thin-edge proven  

---

*Generated by release readiness campaign 2026-07-31 (Grok Build + metaharness + rUv search + Plane DAG).*
