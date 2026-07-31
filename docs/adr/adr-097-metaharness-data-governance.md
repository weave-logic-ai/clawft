# ADR-097: Universal MetaHarness governance over all WeftOS data surfaces

- **Status**: Draft (Proposed)
- **Date**: 2026-07-31
- **Plane**: WEFT-728 (governance), parent WEFT-724
- **Deciders**: Pending (platform, security, sensor, mesh maintainers)
- **Depends-On**: ADR-096 (MetaHarness foundation), ADR-022 (ExoChain audit),
  ADR-057 (substrate read ACL), ADR-090 (LeWM decoupling)
- **Related**: ADR-078 / Graph Views (fusion), ADR-095 (batch plane),
  ADR-046 (forest), ADR-056 (BVH), mesh ADRs (026+),
  `docs/research/metaharness-foundation.md`,
  `docs/research/ruv-worldgraph-vs-weftos.md`

## Context

ADR-096 places MetaHarness under agentic development and fusion **policy**
evolution. WeftOS data is broader than ViewSpecs and agent prompts:

| Class | Examples |
|-------|----------|
| **Filesystem** | workspace trees, graphify-out, splat datasets, quilt layers, config/views |
| **Databases / indexes** | AgentDB / `.swarm`, RVF stores, HNSW/DiskANN, sqlite side stores |
| **Hardware sensors** | phone/Pi capture, ToF, IMU, multi-cam, future RF/array |
| **Connected nodes** | mesh peers, edge nodes, worldmodel-service topologies |
| **Substrate** | OntologyAdapter paths, publish gates, WASM panel auth |
| **Chain / governance** | ExoChain events, principals, spawn approval |
| **Cognitive forest** | CausalGraph, CrossRefs, SessionView, ECC |
| **Spatial / WM** | BVH leaves, world_model.json, LeWM latents |
| **Fusion Views** | live multi-source Graph Views (operational sensor fusion) |

Without a **single governance doctrine**, each surface invents its own ad-hoc
change process — or worse, silent mutation as data churns.

## Decision

### 1. One governance loop for every surface class

All WeftOS data surfaces are covered by the **same MetaHarness-aligned loop**:

```text
inventory → classify surface → ACL check → change proposal
  → evaluate (anchors/fixtures/tests) → immutable receipt
  → promote (confirm / keys / PR) → audit (chain when required)
```

“Data churn” (new sensor readings, new files, new peer state) is normal and
**does not** require a receipt. **Policy** that decides how data is admitted,
retained, fused, exposed, or promoted **does**.

### 2. Surface inventory (normative classes)

| Surface ID | Owner (crate/doc) | Policy examples | State audit |
|------------|-------------------|-----------------|-------------|
| `fs.workspace` | repo / agent tools | path allowlists, ignore rules | git |
| `fs.artifacts` | splatd, graphify-out | retention, PII scrub | job logs + chain opt |
| `db.agentdb` | ruflo / AgentDB | namespace ACLs, pattern keys | receipts + MH patterns |
| `db.vector` | HNSW / DiskANN / Hybrid | index_id, cold/hot promote | bench + WEFT vector |
| `sensor.local` | capture edge, sensor-head | sample rate, window caps | View attach + events |
| `sensor.fusion_view` | Graph Views | ViewSpec, soft edges, promote gate | flywheel + chain |
| `mesh.peer` | QUIC/mesh services | trust, topic ACL | chain / mesh events |
| `substrate.path` | ADR-057 / MCP | read/write gates | substrate audit |
| `chain.exochain` | ADR-022 | what must log | chain itself |
| `ecc.forest` | ADR-046/062 | impulse vs mutate | chain + ECC |
| `spatial.bvh` | ADR-056/078 | leaf tags, VectorRef | chain on publish |
| `wm.lewm` | ADR-090 | impulse-only into ECC | R1–R5 checks |
| `plane.board` | Plane / plane-dag | claim/close rules | Plane comments |

New surfaces **must** be added to this table (or a machine-readable sibling
under `.metaharness/weftos/surfaces.yaml`) before they accept ungoverned policy.

### 3. Policy vs state

| Kind | Examples | Requires flywheel/receipt? |
|------|----------|----------------------------|
| **State** | new frame file, new co-observe edge instance, chain event append | No — normal operation |
| **Policy** | ViewSpec caps, soft_edge min_score, substrate denylist, retention days, which mesh topics attach | **Yes** — evaluate→receipt→promote |
| **Authority** | ECC R1–R5, spawn principals, dual-sign | **Human + ADR**; not Darwin-auto |

### 4. Same promote discipline as ADR-096

- Evaluate never mutates champion policy.
- Promote needs explicit confirm (keys or human merge of receipt-backed PR).
- Grok/Claude both follow `.grok/rules/metaharness.md`.
- Kernel/`weft` runtime remains free of Node MetaHarness dependency.

### 5. Fusion is the reference implementation

Graph Views (sensor fusion) are the **first full vertical** of this doctrine:
live multi-source attach (state) + ViewSpec (policy) + promote to BVH (state
with chain) + optional batch analytics policy (policy). Other surfaces copy
this split, not invent parallel governance.

### 6. Explicit non-goals

- Encrypting every file with MH (different concern).
- Replacing substrate ACL or ExoChain with receipts alone.
- Auto-evolving security denylists without human gate.

## Consequences

### Positive

- One vocabulary for “what can change when sensors/mesh/agents churn.”
- Prevents shadow stores (untracked DBs, ad-hoc fusion) outside inventory.
- Aligns with rUv WorldGraph provenance culture under WeftOS names.

### Negative / risks

- Inventory drift if new crates skip the table — mitigate via PR checklist +
  `scripts/metaharness/weftos-score.sh` surface counts.
- Over-process for tiny config tweaks — allow “receipt-lite” PR template for
  low-risk policy with still-visible review.

## Follow-ups

- [x] Plane WEFT-728
- [ ] Machine-readable `.metaharness/weftos/surfaces.yaml`
- [ ] PR template checkbox: “policy change? MH receipt / waiver”
- [ ] Wire high-risk promotes to ExoChain event kinds

## References

1. ADR-096 MetaHarness foundation  
2. Graph Views research F1–F10  
3. ruflo ADR-150 optional MH constraints  
4. `@metaharness/flywheel` promote semantics  
