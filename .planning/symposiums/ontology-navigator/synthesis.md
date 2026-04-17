# Symposium Synthesis: Universal Topology Browser

**Date**: 2026-04-16 — 2026-04-17
**Status**: COMPLETE
**Sessions completed**: 1-7 + Tree Calculus + ArcKit + IRI Identity + Q&A

---

## Architectural Breakthrough: Tree Calculus + EML

Tree calculus (treecalcul.us) provides the structural logic substrate.
EML provides the continuous math substrate. Together they form a
dual computational model where:

- **Tree calculus** handles: pattern matching on graph structure,
  dispatch to layout algorithms, composition of mode overlays,
  graph decomposition into subgraphs
- **EML** handles: learned physics parameters, scoring functions,
  geometry classification, threshold optimization

Both execute as native Rust. No LLM, no JavaScript layout libraries.
Combined pipeline: Graph → Topology tree → Triage dispatch → EML
parameter computation → Layout reduction → RVF serialized geometry
→ Thin renderer (any frontend).

**Estimated performance**: 10K-node graph in <50ms. Triage is
nanosecond pattern matching. EML is one exp + one ln per model.
Layout is O(n log n) for force, O(n) for tree/timeline.

The React VOWL navigator becomes a renderer prototype consuming
positioned JSON. The real intelligence is the Rust layout engine.

See `adr-treecalc-eml-architecture.md` for full pipeline design.

---

## Key Findings

### 1. We already have two domain ontologies

Session 1 revealed 26 entity types split across Code (12) and
Forensic (12) domains, with 21 relationship types. The forensic
domain (Person, Event, Evidence, Location, Hypothesis) was built
for the cold-case symposium and maps directly to investigation.yaml.
The "build an ontology" problem is actually "expose and formalize
the ontology we already have."

### 2. The schema is a view model, not a data model

Session 2's strongest decision: TopologySchema describes **geometry
and rendering**, not data structure. Keys match existing Rust enum
discriminant strings. Custom types handled via wildcard fallback.
The schema is a `.topology.yaml` file that users can write in 5
minutes. Three presets delivered: software, investigation, robotics.

### 3. Two libraries, not six

Session 3 recommends `elkjs` + `d3-force` as the complete layout
stack. elkjs handles tree, DAG (Sugiyama), grid, and compound
nodes natively. d3-force handles network layout. Auto-detection
from graph topology in ~80 lines when no schema exists.

### 4. EML models already bridge data → rendering

Five learned models exist. LayoutModel already takes graph density
and outputs physics parameters. ForensicCoherenceModel scores
investigation graph quality. SurpriseScorerModel highlights
unexpected connections. These plug directly into heatmap mode.

### 5. Modes are the interaction model

The navigator has three mode layers that compose:
- Base mode (layout): Explore, Timeline, Cluster
- Overlay modes (encoding): Diff, Heatmap, Flow, Search
- Tool mode (interaction): Annotate

The diff overlay (Plan/Implemented/Actual) is one mode, not
the entire system. Modes stack and compose.

---

## Architecture Decisions (Confirmed)

| Decision | Rationale |
|----------|-----------|
| YAML schema format | Supports comments, clean nesting, serde_yaml mature |
| Geometry per-node-type | Package=tree, Function=force, Event=timeline in same graph |
| Keys = discriminant strings | Stable contract, already tested, Custom types via wildcard |
| elkjs + d3-force | Two deps cover all 8 geometry types |
| Semantic zoom via transform.k | 4 levels: dot → shape → card → subgraph |
| FLIP transitions | Record-compute-animate, 500ms, anti-teleport rule |
| Soft schema validation | Warn don't reject, fall back to force on missing fields |

---

## Q&A Decisions (Founder Responses)

All 7 open questions answered. See `qa-decisions.md` for full detail.

| # | Decision |
|---|----------|
| 1 | Schemas are **composable like Docker configs** (base + project + local, layered merge) |
| 2 | **Infer schema from code/data**, diff against declared schema to detect drift |
| 3 | Causality is a **full orthogonal dimension** (two axes: temporal + counterfactual) |
| 4 | **Speak OWL/RDF** for interop — use internally, export correctly |
| 5 | Cross-layer entities resolved by **IRI** (same entity, multiple layer views) |
| 6 | **Single overlay mode** active at a time (start simple, relax if needed) |
| 7 | Missing nodes = **topological distortion** (heatmap overlay now, 3D surface future) |

Additional breakthrough: **IRI-based identity** replaces string-based
entity types. Same word ("Service") can mean three different things —
the IRI is the concept, the word is just a label. See `adr-identity-iri.md`.

---

## Additional Findings

### Tree Calculus + EML Dual Substrate
Tree calculus handles structural logic (triage dispatch), EML handles
continuous math (learned parameters). Combined pipeline in pure Rust:
Graph → Topology tree → Triage → EML params → Layout reduction → RVF.
Estimated <50ms for 10K nodes. See `adr-treecalc-eml-architecture.md`.

### ArcKit Governance Pattern
ArcKit's 60 doc-types across 9 phases map onto WeftOS's assessment
pipeline. Its traceability matrix (requirement → design → test) IS
a topology the navigator should render. Wardley Map evolution axis
is a new geometry type. See `session-arckit-findings.md`.

### AI Framework Validation
7 formal schemas directly importable: ML Schema (OWL), PROV-O (OWL),
DTDL (JSON-LD), AAS (JSON/XML), ArchiMate (XML), BPMN (XSD),
OPC UA (XML). PROV-O's Entity/Activity/Agent triad is a universal
provenance layer that composes onto any domain. See
`session-ai-frameworks-findings.md`.

### Schema Builder Agents
Five specialized agents that produce composable schema fragments
from any source material:
- **Framework agent**: parse published standards (NIST, EU AI Act, etc.)
- **Codebase agent**: run graphify + cluster → inferred schema
- **Document agent**: NER + relationship extraction from planning docs
- **Visual agent**: vision model extracts topology from napkin drawings
- **Telemetry agent**: trace/metric analysis → observed topology

All produce fragments in the same YAML format, compose via Docker-
config layering. Draft schemas require human `ratify` before
activation (prompt injection mitigation). See
`session-schema-agents-findings.md`.

---

## Revised Implementation Plan

### Phase 1 — Schema + Identity (Sprint 17-18)
1. Define `TopologySchema` Rust types with IRI support (~200 LOC)
2. Add `iri: Option<String>` to Entity struct
3. Ship composable presets: software, investigation, governance
4. Schema inference: `weaver ontology infer` + `weaver ontology diff`
5. Add Wardley geometry type

### Phase 2 — Rust Layout Engine (Sprint 18-19)
6. `Topology { Atom | Sequence | Branch }` in clawft-types
7. Triage dispatcher + EML parameter integration
8. Reingold-Tilford tree layout in Rust (~200 LOC)
9. Force-directed (Barnes-Hut) in Rust (~400 LOC)
10. RVF serialization of positioned geometry

### Phase 3 — Navigator Modes (Sprint 19-20)
11. Mode system: base + overlay + tool
12. Explore mode (force/tree/layered via Rust engine)
13. Heatmap overlay (metric → color gradient)
14. Topological distortion for missing nodes
15. Drill-down + breadcrumbs

### Phase 4 — Governance Integration (Sprint 20-21)
16. governance.yaml preset with traceability edges
17. Diff overlay (Plan/Implemented/Actual)
18. `weft govern` artifact generation from assessment findings
19. Wardley Map geometry renderer
20. IRI-based graph merging

### Phase 5 — Schema Builder Agents (Sprint 22)
21. Framework schema agent (parse OWL/XSD/PDF → topology YAML)
22. Codebase schema agent (graphify → inferred schema)
23. Document schema agent (NER + relation extraction → draft schema)
24. Telemetry schema agent (traces/metrics → observed topology)
25. Draft/ratify workflow (confidence scores, human approval gate)

### Phase 6 — Advanced (Sprint 23+)
26. Visual schema agent (napkin/whiteboard → topology YAML)
27. EML GeometryClassifier (auto-detect layout from graph)
28. Context-based IRI resolution (word sense disambiguation)
29. Flow animation mode
30. Semantic zoom levels
31. 3D topological surface (future R&D)

---

## Symposium Artifacts

- `AGENDA.md` — 8-session agenda with full design
- `session-1-inventory-findings.md` — typed structure inventory
- `session-2-schema-findings.md` — TopologySchema format + 3 presets
- `session-3-layout-findings.md` — algorithm catalog + auto-detection
- `session-treecalc-findings.md` — tree calculus as formal foundation
- `adr-treecalc-eml-architecture.md` — dual substrate architecture decision
- `adr-identity-iri.md` — IRI-based identity for ontology interop
- `session-arckit-findings.md` — ArcKit governance ontology pattern
- `session-4-pipeline-findings.md` — pipeline architecture + trait chain
- `session-5-nesting-findings.md` — drill-down, breadcrumbs, cross-level edges
- `session-6-7-modes-findings.md` — mode system types, state machine, cold case walkthrough
- `session-ramp-findings.md` — Google RAMP assessment ontology + 7R (Ratify) + declared vs observed
- `session-ai-frameworks-findings.md` — 7 importable schemas + maturity scoring frameworks
- `session-schema-agents-findings.md` — 5 schema builder agent types + composability model
- `qa-decisions.md` — founder Q&A responses (7 decisions)
- `synthesis.md` — this document

## Final Architecture Summary

### The Universal Topology Browser is:

1. **A Rust layout engine** that consumes a KnowledgeGraph + TopologySchema
   and produces positioned geometry via tree calculus dispatch + EML
   parameter computation. <50ms for 10K nodes.

2. **A mode system** with three independent axes (base + overlay + tool)
   where only base changes rerun layout (<300ms), overlay changes
   re-encode visuals (<100ms), and tool changes swap interaction (<16ms).

3. **A thin renderer** that paints positioned geometry from RVF (Rust
   native) or JSON (browser). The renderer is fungible — React, Tauri,
   TUI, or any future frontend.

4. **A composable schema system** (Docker-config style) with IRI-based
   identity that resolves the word-vs-concept problem and enables graph
   merging, OWL/RDF interop, and multi-domain views.

5. **A pipeline** of 6 trait-chained stages (Ingest → Classify →
   ExtractEdges → Encode → Layout → Serialize), half built today.

### What exists vs what to build:

| Built | To Build |
|-------|----------|
| AST extraction (tree-sitter) | Layout engine (force, tree, Sugiyama) |
| Vault enrichment (v0.6.11) | TopologySchema types + loader |
| KnowledgeGraph + Contains edges | IRI resolution layer |
| EML models (5 trained) | Schema-validated edge checking |
| ResourceTree (Merkle, nesting) | Tree calculus triage formalization |
| RVF serialization | JSON serialization for browser |
| SecurityScanner (50+ checks) | Telemetry + OWL/RDF ingestors |
| Governance engine (ADR-033) | Mode system types + state machine |
| Assessment service (ADR-023) | Topological distortion renderer |
| VOWL navigator prototype (React) | governance.yaml + wardley geometry |
