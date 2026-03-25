# ECC Symposium Results Report

**Date**: 2026-03-22
**Status**: COMPLETE — All 14 questions resolved, SPARC plan created, ready for K3c implementation

---

## Symposium Outcome

The ECC Symposium convened 8 research agents across two phases to analyze 18 Mentra documents, 22 WeftOS documents, 6 Rust crate implementations, and the ClawStage conversational AI architecture. All findings were synthesized into 5 symposium documents and a complete SPARC implementation plan.

**14 design questions were raised, debated, and resolved** through iterative Q&A between the research panels and the project architect.

---

## Key Architectural Decisions

### D1: The Nervous System Model

WeftOS is not an agent orchestrator with an optional cognitive feature — it is a **cognitive platform** where every kernel instance is a node in a distributed nervous system. From ESP32 sensor nodes to Blackwell GPU servers, they all run the same kernel (`Kernel<P: Platform>`), differentiated only by boot-time calibration and Platform trait implementation.

**Origin**: Q9, Q11 — mentra-cortex IS a kernel instance, hardware-agnostic design.

### D2: Forest of Trees, Not One Graph

The CMVG is implemented as a **polyglot tree ensemble** — multiple domain-specific structures (ExoChain, Resource Tree, HNSW Index, Causal Graph, future domain trees) linked by CrossRefs and Impulses. ExoChain stays linear as the witness chain. Each structure uses data structures appropriate to its domain.

**Origin**: Q2 — confirmed by ClawStage's proven 5-engine forest architecture with 22 RVF segment types, CrossRefs, and Impulses.

### D3: Self-Calibrating Cognitive Tick

The cognitive tick interval is not fixed — it is self-calibrated at boot via synthetic benchmarks, auto-adjusted at runtime, and advertised to peers as a cluster membership property. This enables heterogeneous swarms where a glasses node (50ms tick) and a server node (10ms tick) participate in the same nervous system with coordinated delta sync.

**Origin**: Q4 — user insight that tick interval should be hardware-determined, not hardcoded.

### D4: CRDTs for Convergence, Merkle for Verification

State synchronization uses two complementary layers: ruvector-delta-consensus (CRDTs) for automatic conflict-free merging, and Merkle causal graphs for cryptographic verification and provenance. CRDTs ensure nodes agree; Merkle proves they agreed honestly.

**Origin**: Q10 — user insight that the two approaches are complementary, not competing.

### D5: DEMOCRITUS as Nervous System Operation

The DEMOCRITUS causal extraction pipeline is not a batch job or separate service — it is the natural operation of the nervous system. Edge nodes create causal relations at runtime; GPU nodes assess relations that weren't clear locally and push refined embeddings back down. The 16-hour batch becomes 30-second micro-batches distributed across the network.

**Origin**: Q12 — user insight on continuous bidirectional flow vs batch processing.

### D6: BLAKE3 Forward, SHAKE-256 Present

New ECC code uses BLAKE3 (aligning with the original architecture in exo-core). Existing ExoChain/Resource Tree keep SHAKE-256 until the K6 migration brings everything to BLAKE3 via exo-core. The single-hash-family invariant is maintained within each subsystem.

**Origin**: Q1 — investigation of development notes revealed SHAKE-256 was a K0 pragmatic choice, BLAKE3 was always the target.

### D7: Per-Tree Scoring with Uniform CrossRef Indexing

Each tree/structure defines its own N-dimensional scoring (EffectVector, NodeScoring, domain-specific vectors). What's uniform is the CrossRef indexing system — Universal Node IDs (BLAKE3 hashes) enable grafting (linking) and shaking (pruning) across any pair of structures regardless of their internal dimensionality.

**Origin**: Q5 — user insight that topology is per-tree, indexing is universal.

### D8: One Feature Flag, Boot Decides

Compile-time `--features ecc` includes all ECC modules. Boot-time calibration determines what's actually active based on hardware capability. No combinatorial feature flag complexity.

**Origin**: Q8 — user insight that build provides capability, boot provides configuration.

---

## Scope Allocation

### K3c (ECC Substrate — Single Node)

| Deliverable | Est. Lines |
|-------------|-----------|
| CausalGraph (typed DAG) | ~800 |
| HnswService (kernel wrapper) | ~300 |
| CognitiveTick (calibrated timer loop) | ~500 |
| Calibration (boot-time benchmark) | ~400 |
| CrossRef store (cross-tree edges) | ~400 |
| Impulse queue (ephemeral events) | ~300 |
| Boot integration, tools, tests | ~800 |
| **Total** | **~3,500** |

### Deferred to K4+

| Item | Phase | Rationale |
|------|-------|-----------|
| WASM cognitive modules | K4 | Requires wasmtime |
| RVF persistence migration | K4 | Replace JSON shortcuts |
| Platform traits (Android, ESP32) | K4+ | Hardware-specific |
| CMVG delta sync | K5 | Requires clustering |
| CRDT + Merkle layered sync | K5 | Requires multi-node |
| Spectral offloading to peers | K5 | Requires clustering |
| SONA integration | K5 | Requires training data |
| DEMOCRITUS bidirectional flywheel | K5 | Requires multi-node |
| BLAKE3 ExoChain migration | K6 | exo-core integration |
| Cross-node delta rate matching | K6 | Requires networking |

---

## What the Research Discovered

### From the Mentra Project (18 documents)

1. **ECC is viable on a $30 ARM SoC** — 3.6ms per cognitive tick on Cortex-A53, all 9 business questions passed
2. **The embedding space IS the model** — 80-90% of intelligence from vector geometry, not LLM inference
3. **Inverted persistence** — ephemeral cognition + permanent memory (opposite of LLMs)
4. **CMVG is a general-purpose cognitive primitive** — applicable to 8+ domains beyond conversation
5. **Camera-as-sensor** — event-triggered snapshots yield 36-720x bandwidth savings
6. **Filler phrase latency masking** — pre-cached responses mask remote inference time

### From ClawStage (6 documents)

7. **The forest architecture** — 5 specialized trees bound by CrossRefs, Impulses, shared HNSW, and a witness chain
8. **CMVG was extracted from ClawStage** — the paradigm originated in the conversational AI engine
9. **Universal Node IDs** — BLAKE3 hashes enable cross-tree addressing
10. **Impulse-driven inter-tree communication** — ephemeral causal events processed in HLC order

### From WeftOS Codebase (21 crates, 421 tests)

11. **HNSW is already implemented** — clawft-core has full instant-distance integration (95% ready)
12. **ExoChain provides 85% of Merkle provenance** — append-only, hash-linked, Ed25519 signed
13. **The kernel already depends on clawft-core** — no new coupling for HNSW integration
14. **SHAKE-256 was a pragmatic K0 choice** — BLAKE3 via exo-core was always the target
15. **The "Adding a New Gated Subsystem" pattern** documents exactly how to add ECC modules

---

## Panel Verdicts

| Panel | Domain | Verdict | Key Contribution |
|-------|--------|---------|-----------------|
| **Mentra Architecture** | Core paradigm, CMVG, test rig | PASS | ECC paradigm definition, benchmark data |
| **Mentra Hardware** | Device profile, sensors, Samsung | PASS | Hardware feasibility, tiered inference design |
| **Mentra Daemon & Audio** | Cognitive daemon, audio, WiFi | PASS | Tick loop design, TTS findings, WiFi PSM |
| **Mentra SDK & Vision** | MentraOS platform, paradigm vision | PASS | Existing platform mapping, paradigm naming |
| **WeftOS Core Architecture** | Kernel modules, K-phases, ExoChain | PASS | 55% infrastructure already exists |
| **WeftOS K2 Symposium** | A2A services, RUV ecosystem | PASS | HNSW routing, CRDT + Merkle synergy |
| **WeftOS K3 Symposium** | Tool lifecycle, sandbox, agent loop | PASS | ExoChain as provenance, sandbox for ECC |
| **WeftOS Crate Analysis** | Source code implementation review | PASS | Corrected false assumptions (kernel→core dep) |

**All 8 panels PASS. ECC Symposium APPROVED for K3c implementation.**

---

## Deliverables Produced

| # | Document | Location |
|---|----------|----------|
| 1 | Mentra Research Index | `mentra/docs/MENTRA_RESEARCH_INDEX.md` |
| 2 | Symposium Overview | `clawft/docs/weftos/ecc-symposium/00-symposium-overview.md` |
| 3 | Research Synthesis | `clawft/docs/weftos/ecc-symposium/01-research-synthesis.md` |
| 4 | Gap Analysis | `clawft/docs/weftos/ecc-symposium/02-gap-analysis.md` |
| 5 | Documentation Update Guide | `clawft/docs/weftos/ecc-symposium/03-documentation-update-guide.md` |
| 6 | Q&A Roundtable (14/14 resolved) | `clawft/docs/weftos/ecc-symposium/04-qa-roundtable.md` |
| 7 | Symposium Results Report | `clawft/docs/weftos/ecc-symposium/05-symposium-results-report.md` |
| 8 | K3c SPARC Plan | `clawft/.planning/sparc/weftos/03c-ecc-integration.md` |
