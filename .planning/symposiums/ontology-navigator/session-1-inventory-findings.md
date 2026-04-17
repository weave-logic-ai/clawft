# Session 1 Findings: What Structure Already Exists

## Summary

The codebase already contains a **substantial implicit ontology** —
far more than initially assumed. The inventory reveals:

- **26 entity types** (12 code + 12 forensic + 2 shared)
- **21 relationship types** (10 code + 11 forensic + 2 shared)
- **8 causal edge types** (ECC)
- **5 EML learned models** that tune layout, scoring, and clustering
- **3 confidence levels** with weights
- **6 analysis question types** for gap detection

This is not "we need to build an ontology" — we need to **formalize
and expose the ontology we already have**.

## Key Findings

### 1. Two Domains Already Exist: Code and Forensic

Graphify already has two complete domain schemas via `DomainTag`:

**Code domain** (0x20): Module, Class, Function, Import, Config,
Service, Endpoint, Interface, Struct, Enum, Constant, Package

**Forensic domain** (0x21): Person, Event, Evidence, Location,
Timeline, Document, Hypothesis, Organization, PhysicalObject,
DigitalArtifact, FinancialRecord, Communication

Each domain has its own relationship types (Calls/Imports/Contains
for code; WitnessedBy/FoundAt/Contradicts/Precedes for forensic).
**The domain preset system we're designing already has its skeleton
in entity.rs.**

### 2. EML Models Already Tune Layout

Five learned models exist that bridge data structure → visual
rendering:

- **LayoutModel**: 3 inputs (node_count, edge_count, density) →
  6 physics params. Already doing auto-layout tuning.
- **ClusterThresholdModel**: determines community boundaries
- **SurpriseScorerModel**: highlights unexpected connections
- **ForensicCoherenceModel**: scores investigative graph quality
- **QueryFusionModel**: ranks search relevance

**These should feed directly into the mode system.** Heatmap mode
can color by surprise score. The forensic coherence model gives us
per-dimension lambda-2 scoring. The layout model is already doing
geometry auto-detection via EML.

### 3. Gap Analysis Infrastructure Exists

`SuggestedQuestion` with 6 types (AmbiguousEdge, BridgeNode,
VerifyInferred, IsolatedNodes, LowCohesion, NoSignal) is already
schema-aware gap detection. `GraphDiff` tracks new/removed
nodes/edges. **Ghost nodes can be generated from SuggestedQuestion
output.**

### 4. Temporal Metadata Exists in ECC

CausalNode has `created_at` (HLC timestamp), CausalEdge has
`timestamp` and `chain_seq` (ExoChain sequence). **Timeline mode
has data to work with immediately — no new extraction needed.**

### 5. Community/Hierarchy Already Computed

Graphify computes community membership, bridge nodes, degree
centrality, and betweenness. The Obsidian canvas export already
groups by community with 600px spacing. **Cluster mode's convex
hulls can use existing community data.**

### 6. Stubs Ready for Extension

SVG export, GraphML, and Cypher (Neo4j) are stubbed but not
implemented. The VOWL JSON export we planned (OG-1 in sprint17.md)
slots in alongside these.

## Entity Type → Geometry Mapping (Proposed)

Based on the inventory, natural geometry assignments:

| Entity Type | Natural Geometry | Why |
|-------------|-----------------|-----|
| Module, Package | Tree (contains children) | Hierarchical containment |
| Class, Struct | Tree (contains methods/fields) | Hierarchical containment |
| Function | Leaf (no subgraph) | Terminal node |
| Service, Endpoint | Network | Peer-to-peer communication |
| Timeline | Timeline | Explicitly temporal |
| Event, Evidence | Timeline or Network | Time-ordered with cross-links |
| Person, Organization | Hierarchy or Network | Org chart or social graph |
| Sensor/Stream (future) | Stream | Time-series data |

## Open Questions for Q&A

1. The forensic domain (Person, Event, Evidence, Location etc) is
   already in entity.rs but is it being used in production? Or is
   it speculative code from the cold-case symposium?
2. The EML LayoutModel takes (node_count, edge_count, density) — 
   should it also take detected_geometry as an input feature?
3. Should Custom(String) entity types be permitted in the topology
   schema, or should we require all types to be declared?
4. The code and forensic domains share 2 relationship types
   (RelatedTo, CaseOf). Should the topology schema enforce strict
   domain separation or allow cross-domain edges?
