# Symposium: Universal Topology Browser — Structured Graph Navigation for WeftOS

**Working Title**: "Exposing the geometry of any data structure — from sensor packs to cold cases"
**Status**: Planning
**Target Date**: Sprint 17 (April 2026)

---

## Design Principle

The navigator is not an ontology visualizer. It is a **universal
topology browser**. The data declares its own geometry — hierarchy,
time-series, network, tree, DAG — and the navigator renders that
geometry and lets you drill in and out.

Every node is potentially a subgraph. A sensor reading is a leaf in a
time-series subgraph, which is a child of a sensor node, which lives
inside a machine subgraph, which is part of a fleet graph. A function
is a leaf inside a module, inside a package, inside a repository,
inside an organization. A piece of evidence sits inside an event,
inside a case, inside an investigation.

**The navigator must**:
- Render the topology the data declares (not impose one)
- Let you zoom into any node to see its internal structure
- Let you zoom out to see a node's context in its parent graph
- Show different dimensions of the same graph (temporal, causal,
  spatial, organizational) as overlays
- Work for anything: source code, sensor data, org charts, crime
  scenes, disk trees, robot telemetry, supply chains, ontologies

**The schema doesn't define the data — it describes the geometry.**
A schema says "this is a hierarchy with these node types" or "this is
a time-series with this sampling rate" or "this is a network with
these edge types." The navigator reads the schema and picks the right
rendering strategy. No schema? Force-directed default. Bad schema?
Show what you can, flag what doesn't fit.

---

## The Problem

We have three layers of graph-building tooling:

1. **Graphify** — extracts entities and relationships from source code via
   tree-sitter AST and LLM semantic extraction. Produces a `KnowledgeGraph`
   with typed entities (Module, Class, Function) and typed edges (Contains,
   Calls, Imports). Exports to Obsidian, GraphML, HTML, JSON.

2. **Weaver vault** (new, v0.6.11) — enriches markdown files with YAML
   frontmatter, extracts wikilinks, analyzes link graphs, suggests new
   connections. Ported from weave-nn.

3. **Obsidian graph view** — renders whatever wikilinks exist as a
   force-directed graph.

**The result is a jumbled mess** because:

- Tags are flat strings with no hierarchy or relationships between them
- Auto-linking connects things that mention the same word, not things that
  are semantically related
- There is no schema defining valid entity types, valid relationship types,
  or cardinality constraints
- Every node looks the same — no visual distinction between a concept, a
  component, a person, a decision
- There is no notion of "layers" or "views" — everything is one flat graph
- Graphify's typed model is rich, but the Obsidian export flattens it back
  to untyped wikilinks

**What WebVOWL gets right**: it starts from an OWL ontology — a formal
schema that says "these are the kinds of things, these are the valid
relationships, this is the hierarchy." The visualization *follows* the
structure, it doesn't create it.

---

## Symposium Agenda

### Session 1: What Structure Already Exists (30 min)

Inventory what we already have before building anything new.

- Graphify entity types: Module, Class, Function, Interface, Variable,
  Import, Export, Package, Constant, Enum, TypeAlias, Trait, Macro —
  these ARE an ontology, just not formalized as one
- Graphify relationship types: Contains, Calls, Imports, Extends,
  Implements, References, DependsOn, Returns, Accepts, Throws
- ECC causal model: Causes, Enables, Correlates, Follows, Inhibits,
  Contradicts, EvidenceFor, TriggeredBy
- Frontmatter types: concept, technical, feature, guide, standard,
  architecture, specification, documentation
- Frontmatter status: active, draft, in-progress, archived
- Plugin capabilities: Tool, Channel, PipelineStage, Skill,
  MemoryBackend, Voice
- Agency roles: Root, Supervisor, Service, Worker, User

**Question**: Can we unify these into a single ontology schema that
covers code, documents, agents, and infrastructure?

### Session 2: The Ontology Schema Problem (30 min)

What would a WeftOS ontology actually look like?

**Candidate layers** (inspired by MAESTRO 7-layer):

| Layer | What Lives Here | Example Types |
|-------|----------------|---------------|
| **Concepts** | Abstract ideas, patterns, decisions | Concept, Pattern, Decision (ADR), Principle |
| **Architecture** | System structure | Component, Service, Module, Interface, Protocol |
| **Code** | Implementation | Class, Function, Variable, Type, Test |
| **Data** | Information schema | Entity, Field, Relationship, Schema, Migration |
| **Agents** | Autonomous actors | Agent, Skill, Tool, Plugin, Workflow |
| **Infrastructure** | Runtime environment | Host, Container, Process, Resource, Cron |
| **People** | Organization | Person, Team, Role, Stakeholder |

**Valid cross-layer relationships**:
- Decision → *justifies* → Architecture choice
- Component → *implementedBy* → Module
- Agent → *uses* → Tool
- Schema → *migratedBy* → Migration
- Person → *owns* → Component

**Key design questions**:
1. Fixed schema or extensible? (OWL is extensible by design)
2. Single global ontology or domain-specific schemas that compose?
3. How does the schema get enforced — at write time or view time?
4. Who maintains the schema — hand-authored or inferred from data?

### Session 3: Layout Follows Structure (30 min)

Once we have an ontology, visualization is no longer a layout problem —
it's a projection problem. Different projections of the same graph:

| View | What It Shows | Layout Strategy |
|------|--------------|-----------------|
| **Dependency** | What depends on what | Layered DAG (top-down) |
| **Hierarchy** | What contains what | Tree / indented list |
| **Timeline** | What happened when | Horizontal timeline |
| **Architecture** | How components connect | Layered L-R (presentation → logic → data) |
| **Org chart** | Who owns what | Top-down tree |
| **ER diagram** | How data relates | Grid with compound nodes |
| **Concept map** | How ideas connect | Force-directed (but with clusters) |
| **Process flow** | What triggers what | Swimlane / sequence |

**The navigator widget should**:
- Let the user pick a view (or auto-detect from edge types)
- Filter by ontology layer (show only Architecture, or only Code)
- Color/shape by type (already started with VOWL encoding)
- Collapse layers to reduce clutter
- Show cross-layer edges as dashed lines

### Session 4: The Pipeline — From Raw Content to Structured Graph (30 min)

How content flows through the system today vs. what we need:

**Current pipeline** (broken):
```
Files → frontmatter tags → wikilinks → flat graph → hairball
```

**Proposed pipeline**:
```
1. Schema definition (OWL/YAML/Rust types — the ontology)
     ↓
2. Classification (given a file/entity, what ontology type is it?)
     ↓
3. Relationship extraction (given two typed entities, what's the relationship?)
     ↓
4. Validation (does this edge violate the schema? cardinality? direction?)
     ↓
5. Layout projection (given the typed graph, what view makes sense?)
     ↓
6. Rendering (VOWL navigator widget, Obsidian export, CLI table)
```

**Key components to build**:

- **Ontology schema format**: define it in YAML or Rust (not OWL — too
  complex for our needs). Ship a default schema for WeftOS, let users
  extend for their domains.

- **Type classifier**: given `frontmatter.type`, `file path`, and
  `content`, map to an ontology type. The vault enrichment already does
  primitive inference — upgrade it to use the schema.

- **Relationship validator**: before creating an edge, check the schema.
  "Agent → Contains → Decision" is invalid. "Agent → uses → Tool" is
  valid. Reject or warn on invalid edges.

- **View selector**: given a typed graph, analyze edge type distribution
  and pick the best layout. Expose as `LayoutStrategy` in the navigator
  widget.

### Session 5: Graph Nesting and the Universal Topology Model (30 min)

The same structure appears everywhere: a thing contains other things,
which contain other things. The navigator needs to handle this
natively — every node is a potential subgraph.

**The nesting model**:

```
Fleet                          [network: force-directed]
 ├─ Machine A                  [hierarchy: tree]
 │   ├─ Sensor Pack 1          [time-series: timeline]
 │   │   ├─ Temperature        [stream: sparkline]
 │   │   ├─ Pressure           [stream: sparkline]
 │   │   └─ Vibration          [stream: sparkline]
 │   ├─ Sensor Pack 2          ...
 │   └─ Actuator Group         [dependency: DAG]
 ├─ Machine B                  ...
 └─ Base Station               [single node, no subgraph]
```

Each level declares its own geometry:
- The fleet is a **network** (machines communicate, force-directed)
- Each machine is a **hierarchy** (subsystems contain components, tree)
- Each sensor pack is a **time-series** (readings over time, timeline)
- Each sensor stream is a **scalar sequence** (sparkline)

**Navigation model — "Google Maps for graphs"**:

| Action | What Happens |
|--------|-------------|
| **Click node** | Select it, show detail panel with metadata |
| **Double-click node** | Zoom into its subgraph (if it has one) |
| **Breadcrumb click** | Zoom back out to parent graph |
| **Ctrl+click** | Open subgraph in a split pane (side-by-side) |
| **Pinch/scroll** | Semantic zoom: at low zoom, nodes collapse to labels; at high zoom, nodes expand to show internal structure inline |

**Breadcrumb trail**: always visible, shows your position in the
nesting hierarchy.

```
Fleet > Machine A > Sensor Pack 1 > Temperature
```

**Cross-level edges**: some relationships span nesting levels.
Machine A's actuator depends on Machine B's sensor reading. These
render as dashed lines that "escape" the subgraph boundary and
connect to a node in a sibling or ancestor graph. The navigator
shows them as portal edges — click to jump to the other end.

**How this maps to existing domains**:

| Domain | Level 0 | Level 1 | Level 2 | Level 3 |
|--------|---------|---------|---------|---------|
| **Software** | Organization | Repository | Package/Module | Class/Function |
| **Robotics** | Fleet | Machine | Sensor pack | Individual sensor |
| **Investigation** | Case portfolio | Case | Event timeline | Evidence items |
| **Infrastructure** | Data center | Rack/cluster | Host/VM | Service/process |
| **Filesystem** | Volume | Directory | Subdirectory | File |
| **Corporate** | Company | Division | Team | Person |
| **Supply chain** | Market | Supplier tier | Factory | Production line |

**The schema declares nesting**: a schema field `contains` on a node
type says "instances of this type can be expanded into a subgraph."
The subgraph's geometry is declared by the child type's schema.

```yaml
# Example: robotics domain schema
types:
  Fleet:
    geometry: network
    contains: Machine
    edges: [communicatesWith, monitorsFrom]
  Machine:
    geometry: hierarchy
    contains: [SensorPack, ActuatorGroup, Controller]
    edges: [dependsOn, powers]
  SensorPack:
    geometry: timeline
    contains: Sensor
    edges: [feedsInto]
  Sensor:
    geometry: stream
    dimensions: [value, timestamp]
    edges: [calibratedBy, alarmsAt]
```

**Key insight**: the navigator doesn't need special-case code for
robots vs code vs investigations. It reads the schema's `geometry`
field and selects the layout. If `geometry: timeline`, use timeline
layout. If `geometry: hierarchy`, use tree. If `geometry: network`
or unspecified, use force-directed. The rendering is generic; the
schema carries the domain knowledge.

**Compound nodes in the navigator widget**:

When zoomed out, a node that contains a subgraph renders with a small
indicator (expand chevron or minimap thumbnail). When semantically
zoomed in, the node expands inline to show a simplified version of
its internal structure (first N children, sparkline summary for
time-series, etc.). Full expansion happens on double-click.

### Session 6: Navigator Modes (30 min)

The topology is the constant. **Modes** change the lens — what's
visible, how it's encoded, and what interactions are available. The
navigator toolbar has a mode selector; switching modes re-renders
the same graph with different visual encoding and different
interaction semantics.

**Core modes**:

| Mode | What It Does | Visual Encoding | Interaction |
|------|-------------|----------------|-------------|
| **Explore** | Default browsing — navigate the topology, inspect nodes | Type-based colors/shapes (VOWL) | Click to select, double-click to drill, drag to reposition |
| **Diff** | Overlay Plan/Implemented/Actual — show gaps and drift | Ghost nodes, drift markers, stress coloring | Toggle layers on/off, click gap to see what's missing |
| **Heatmap** | Color nodes by a metric (utilization, risk, coverage, age) | Color gradient from metric value, size scales | Slider to animate over time, hover for values |
| **Flow** | Animate edges to show data/control/money flow direction | Animated particles along edges, thickness = volume | Click edge to see throughput, filter by flow type |
| **Timeline** | Lay out nodes by time, show sequence and duration | Horizontal timeline, swimlanes by type | Scrub time, zoom into time windows, filter by actor |
| **Cluster** | Group nodes by community/type/tag, collapse groups | Convex hull boundaries, inter-group edges bundled | Click group to expand, drag groups to rearrange |
| **Search** | Highlight paths and neighborhoods matching a query | Matched nodes bright, rest dimmed, path highlighted | Type query, click result to focus, show shortest path |
| **Annotate** | Add notes, flags, and markers to nodes/edges | Sticky notes, color-coded flags, drawn regions | Click to add note, draw to create region, export annotations |

**Mode composition**: some modes compose. Explore + Heatmap shows
the topology colored by a metric. Diff + Timeline shows gaps along
a time axis. The navigator treats modes as layers that stack:

- **Base mode** (always one): Explore, Timeline, or Cluster — controls layout
- **Overlay modes** (zero or more): Diff, Heatmap, Flow, Search — modify visual encoding
- **Tool modes** (zero or one): Annotate — changes click behavior

The toolbar shows: `[Explore v] + [Diff] [Heatmap] [Flow] | [Annotate]`

**Mode-specific schemas**: each mode can define its own config in
the schema YAML:

```yaml
modes:
  diff:
    plan_source: reference-architecture.yaml
    implemented_source: graphify-export.json
    actual_source: telemetry-endpoint
  heatmap:
    metrics:
      - name: CPU Usage
        field: cpu_percent
        gradient: [green, amber, red]
        range: [0, 100]
      - name: Risk Score
        field: risk
        gradient: [blue, purple, red]
        range: [0, 10]
  flow:
    edge_types: [dataFlow, controlFlow, moneyFlow]
    particle_speed: proportional_to_throughput
  timeline:
    time_field: created_at
    duration_field: duration_ms
    swimlane_by: type
```

**Diff mode detail** — one of the core analytical modes:

Multiple graphs can share the same topology but represent different
*states* of that topology. Overlaying them reveals gaps, drift, and
stress — the delta between what should exist, what does exist, and
what's actually happening.

**The three-layer model**:

| Layer | Symbol | Represents | Source |
|-------|--------|-----------|--------|
| **Plan** | Ideal delta-t | What *should* exist — the blueprint, spec, or reference architecture | Schema, requirements doc, reference implementation, compliance framework |
| **Implemented** | Phi (φ) | What *does* exist — the analyzed reality | Code analysis, document scan, infrastructure audit, site crawl |
| **Actual** | Delta-t (δt) | What's *happening* — runtime behavior, live metrics, real usage | Telemetry, APM, logs, sensor data, transaction records |

**How overlay rendering works**:

Nodes and edges exist in one or more layers. The visual encoding
shows their layer membership and the delta between layers:

| State | Visual Encoding | Meaning |
|-------|----------------|---------|
| Plan + Implemented + Actual | Solid fill, normal | Fully realized and running |
| Plan + Implemented, no Actual | Solid fill, greyed | Built but dormant/unused |
| Plan only | Dashed outline, ghost node | **Gap**: specified but not built |
| Implemented only, no Plan | Solid fill, yellow warning | **Drift**: built without spec |
| Implemented + Actual, no Plan | Solid fill, orange | **Shadow**: running unplanned |
| Actual only | Red pulse | **Rogue**: active with no implementation or plan |

**Node metrics from overlay alignment**:

Each node can carry quantitative values from the Actual layer that
encode as visual weight on top of the structural topology:

- **Size** scales with a metric (CPU usage, request rate, evidence count)
- **Color saturation** encodes utilization or health (green → amber → red)
- **Edge thickness** encodes throughput or frequency
- **Halo/glow** encodes anomaly score (deviation from Plan baseline)

**Example: Ecommerce site audit**

Plan (reference ecommerce architecture):
```
Storefront → ProductCatalog → Cart → Checkout → Payments → Inventory
                                                    ↓
                                              OrderFulfillment → Shipping
```

Implemented (analyzed target site):
```
Storefront → ProductCatalog → Cart → Checkout → Payments
                                        ↓
                                   GiftCards (unplanned)
```

Overlay shows:
- **Gaps** (dashed ghosts): Inventory, OrderFulfillment, Shipping
- **Drift** (yellow): GiftCards exists but isn't in the plan
- **Alignment**: Storefront, ProductCatalog, Cart, Checkout, Payments match

Actual (runtime telemetry) adds:
- Cart: 2,400 req/s (node grows, green)
- Payments: 12 req/s (node shrinks, but that's normal conversion)
- Checkout: 8s p99 latency (node pulses amber — stress indicator)

**How this maps to other domains**:

| Domain | Plan | Implemented | Actual |
|--------|------|-------------|--------|
| **Software** | Architecture spec | Analyzed codebase | APM/telemetry |
| **Compliance** | Control framework (SOC2, ISO) | Implemented controls | Audit findings |
| **Investigation** | Case theory / hypothesis | Collected evidence | Witness testimony, forensics |
| **Infrastructure** | Capacity plan | Deployed resources | Resource utilization |
| **Robotics** | Mission plan / route | Installed sensors/actuators | Live sensor readings |
| **Org design** | Planned org chart | Current headcount | Actual collaboration patterns (from comms data) |

**Alignment scoring**:

For each node in the Plan graph, compute:
- **Coverage**: % of Plan nodes that have an Implemented counterpart
- **Drift**: % of Implemented nodes that have no Plan counterpart
- **Utilization**: ratio of Actual metric to Plan baseline
- **Coherence**: ECC lambda-2 score per layer and across layers

The navigator can show a summary bar:
```
Coverage: 72% | Drift: 8% | Avg Utilization: 63% | Coherence: 0.71
```

**Node matching across layers**:

Overlay alignment requires matching nodes across the Plan,
Implemented, and Actual graphs. This is a graph alignment problem:

1. **By ID/name**: if nodes share an identifier, match directly
2. **By type + position**: if a Plan node of type `PaymentService`
   exists and the Implemented graph has a node of type `Service`
   named `payments`, match by type + fuzzy name
3. **By topology**: if the structural position matches (same parent,
   same neighbors), match by graph isomorphism heuristic
4. **Manual override**: analyst pins a match that the heuristic missed

The alignment produces a `MatchResult` per node: Matched, Unmatched
(gap), or Novel (drift). This feeds the visual encoding.

**Schema extension for overlays**:

```yaml
# The overlay config lives alongside the domain schema
overlays:
  plan:
    source: reference-architecture.yaml   # or a graphify export
    label: "Reference Architecture"
    style: { ghost: true, opacity: 0.4 }
  implemented:
    source: graphify-export.json          # or live analysis
    label: "Current Implementation"
    style: { fill: true, opacity: 1.0 }
  actual:
    source: telemetry-endpoint            # or metrics snapshot
    label: "Live Metrics"
    metrics:
      - field: cpu_percent
        encoding: size
        range: [0, 100]
      - field: latency_p99_ms
        encoding: color
        thresholds: { green: 200, amber: 1000, red: 5000 }
```

### Session 7: The Investigative Instrument (30 min)

The navigator is not just a viewer — it's a detective's workbench.
The same graph infrastructure that organizes a codebase also analyzes
a crime scene, maps a corporate structure, traces a supply chain, or
reconstructs a timeline from fragments. The key insight: **the schema
defines what you're looking for, and the gaps in the graph tell you
what's missing.**

**The analyst workflow**:

```
1. Ingest raw material (documents, transcripts, records, code)
     ↓
2. Select or infer a domain schema (what kinds of things exist here?)
     ↓
3. Classify entities against the schema (who/what/where/when)
     ↓
4. Extract relationships (who did what to whom, what depends on what)
     ↓
5. Overlay investigative dimensions onto the typed graph
     ↓
6. Identify structural gaps (what's missing that should be there?)
     ↓
7. Generate hypotheses from the gaps
```

**Investigative dimensions as mode combinations** — an analyst
configures the navigator by selecting a base mode and overlays.
The dimensions from Session 6 map to mode combinations:

| Dimension | Base Mode | Overlays | Layout |
|-----------|----------|----------|--------|
| **Temporal** | Timeline | + Diff (if comparing theory vs evidence) | Horizontal swimlanes |
| **Spatial** | Explore/Cluster (by location) | + Heatmap (density) | Force-directed, clustered |
| **Causal** | Explore | + Flow (animated causal edges) | DAG from ECC |
| **Organizational** | Cluster (by org unit) | + Diff (planned vs actual org) | Hierarchy tree |
| **Financial** | Timeline or Explore | + Flow (money direction) + Heatmap (amounts) | Sankey or force |
| **Communication** | Explore | + Heatmap (frequency) + Flow (direction) | Force-directed |
| **Dependency** | Explore | + Diff (plan vs implemented) | Layered DAG |
| **Evidential** | Explore | + Heatmap (confidence) + Search | Weighted force-directed |

**Connection to ECC DEMOCRITUS loop**:

The ECC engine already does continuous Sense → Embed → Search →
Update → CrossRef → Prune → Report. The ontology navigator becomes
the *Report* visualization — but with the schema providing structure:

- **Gap analysis becomes schema-aware**: instead of "this node has no
  connections," it becomes "this Person entity has no 'wasAt' edge to
  any Location entity for the time window in question." The schema
  defines what *should* exist; its absence is the signal.

- **Coherence scoring becomes dimensional**: ECC's lambda-2 coherence
  score can be computed per dimension. The causal graph might be
  coherent (lambda-2 = 0.85) while the temporal dimension has gaps
  (lambda-2 = 0.3) — telling the analyst exactly where to dig.

- **CrossRef becomes cross-dimensional**: finding that Person A
  appears in the communication dimension AND the financial dimension
  AND the temporal dimension at the same time window — that's a
  convergence signal, not visible in any single view.

**Domain schema examples** (shipped as presets):

| Domain | Schema File | Entity Types | Key Relations |
|--------|------------|-------------|---------------|
| Software | `software.yaml` | Module, Class, Function, Test, Config | Contains, Calls, DependsOn, Tests |
| Investigation | `investigation.yaml` | Person, Event, Evidence, Location, Vehicle, Communication | WasAt, Contacted, PossessedBy, Witnessed |
| Organization | `organization.yaml` | Person, Team, Department, Project, System | ReportsTo, Owns, WorksOn, Manages |
| Compliance | `compliance.yaml` | Policy, Control, Risk, Asset, Finding | Mitigates, Applies, ViolatedBy, AuditedBy |
| Infrastructure | `infrastructure.yaml` | Host, Service, Network, Database, Credential | RunsOn, ConnectsTo, Authenticates, Stores |

**Navigator widget changes for investigative use**:

- **Dimension switcher**: toolbar toggle for each dimension overlay
- **Confidence coloring**: edge opacity/thickness reflects ECC
  confidence score (Extracted, Inferred, UserConfirmed, Speculative)
- **Gap highlighting**: nodes that are expected by the schema but
  missing from the data rendered as dashed outlines ("ghost nodes")
- **Convergence markers**: nodes that appear in 3+ dimensions flagged
  visually (the "interesting" nodes an analyst should focus on)
- **Annotation layer**: analyst can pin notes to nodes/edges that
  persist across dimension switches
- **Snapshot/compare**: save a graph state, make changes, diff against
  the saved state to see what the new evidence changed

### Session 8: Implementation Plan (30 min)

What to build, in what order, to get from hairball to universal
topology browser.

**Phase 1 — Schema + Geometry** (Sprint 17):
- Define `TopologySchema` type in `clawft-graphify` with `geometry`
  field (force, tree, layered, timeline, stream, grid)
- Define `contains` field for nesting (what subgraph types a node can
  expand into)
- Ship domain presets: `software.yaml`, `investigation.yaml`,
  `organization.yaml`, `robotics.yaml`
- `weaver vault enrich` uses schema for type classification

**Phase 2 — Nesting + Drill-down** (Sprint 18):
- Navigator widget: double-click to zoom into subgraph
- Breadcrumb trail for navigation history
- Compound node rendering (expand chevron, minimap thumbnail)
- Cross-level edge rendering (portal edges)
- Split-pane view (Ctrl+click to open subgraph alongside parent)

**Phase 3 — Mode System + Explore Mode** (Sprint 18):
- Mode architecture: base mode (controls layout) + overlay modes
  (modify encoding) + tool modes (change interaction)
- Mode selector in toolbar with composition rules
- Explore mode as default base mode (force-directed, VOWL encoding)
- Layout engine: tree, layered DAG, timeline, stream/sparkline
- Auto-detection from schema `geometry` field

**Phase 4 — Search + Cluster + Heatmap Modes** (Sprint 19):
- Search overlay: query, highlight matches, dim rest, show paths
- Cluster base mode: group by type/tag/community, convex hulls
- Heatmap overlay: bind a metric field to color gradient + size
- Relationship validator in graphify pipeline
- Gap detector: schema-expected-but-missing entities/edges

**Phase 5 — Diff + Flow Modes** (Sprint 20):
- Diff overlay: multi-layer graph model (Plan, Implemented, Actual)
- Node matching: by ID, by type+name, by topology heuristic
- Ghost/drift/rogue visual encoding
- Alignment scoring: coverage, drift %, utilization, coherence
- Flow overlay: animated particles along edges, direction + volume
- Timeline base mode: horizontal swimlanes, time scrubbing

**Phase 6 — Annotate + Investigative** (Sprint 21):
- Annotate tool mode: sticky notes, flags, drawn regions
- Confidence coloring from ECC scores
- Ghost nodes for missing entities
- Convergence markers for multi-dimensional presence
- Snapshot/compare: save state, diff against new evidence

**Phase 7 — Semantic Zoom + Inference** (Sprint 22+):
- Semantic zoom: nodes expand/collapse based on zoom level
- At low zoom: nodes show label + type icon
- At mid zoom: nodes show first N children inline
- At high zoom: full subgraph visible without double-click
- EML-based type classifier, schema evolution
- Cross-layer anomaly detection: Actual deviates from Plan
- Mode presets: save mode+overlay combinations as named presets
  (e.g., "Forensic Timeline", "Architecture Audit", "Fleet Monitor")

---

## Success Criteria

1. A vault of 50+ markdown docs renders as a structured graph with
   visible layers, not a hairball
2. The navigator auto-selects an appropriate layout based on content
3. Schema violations are flagged (e.g., "this doc is tagged 'concept'
   but links to 'infrastructure' nodes with no valid cross-layer edge")
4. A new user can look at the graph and understand the system's
   architecture without reading every document
5. An analyst can switch between temporal, causal, and organizational
   dimensions of the same dataset and see different structure each time
6. Gap analysis produces actionable leads: "Person X has no Location
   edge for the time window in question — investigate whereabouts"
7. The same navigator widget works for code analysis, cold case
   investigation, corporate compliance, and infrastructure mapping
   by swapping the domain schema
8. A 3-level nested graph (e.g., fleet → machine → sensor pack)
   navigates smoothly with breadcrumbs and drill-down
9. A sensor stream renders as a time-series inside its parent node,
   not as a disconnected force-directed blob
10. Cross-level edges (machine A's actuator depends on machine B's
    sensor) render as portal edges and are navigable

---

## Open Questions

1. Should the ontology schema be a standalone file (`.weftos-ontology.yaml`)
   or embedded in the project's CLAUDE.md / config?
2. Can we infer the schema from an existing codebase's graphify output
   rather than requiring manual authoring?
3. How does this interact with the ECC causal model — is causality a
   layer in the ontology or an orthogonal dimension?
4. Is OWL/RDF compatibility important for interop, or is a simpler
   custom format better for our use case?
5. How do we handle entities that span layers (e.g., a "service" is both
   architecture and infrastructure)?
6. How do dimensional overlays interact — can an analyst view causal +
   temporal simultaneously, or is it one dimension at a time?
7. What does the "ghost node" UX look like — how do you show something
   that doesn't exist yet without cluttering the graph?

---

## References

### Internal

- Existing graphify types: `clawft-graphify/src/entity.rs`
- Existing VOWL navigator: `docs/src/app/vowl-navigator/`
- Vault enrichment: `clawft-graphify/src/vault/`
- ECC model: `clawft-kernel/src/ecc.rs`
- CSA MAESTRO 7-layer: `mondweep/agentic-ai-security-demo` — layer model

### Ontology Visualization

1. Katifori et al. "Ontology Visualization Methods -- A Survey." *ACM Computing Surveys* 39(4), 2007. DOI: 10.1145/1287620.1287621
   — Foundational survey categorizing ontology viz techniques (node-link, zoomable, space-filling, focus+context).

2. Dudas et al. "Ontology Visualization Methods and Tools: A Survey of the State of the Art." *Knowledge Engineering Review* 33, 2018. DOI: 10.1017/S0269888918000073
   — Updated survey; finds most tools limited to 2D node-link with weak OWL 2 coverage.

3. Lohmann et al. "Visualizing Ontologies with VOWL." *Semantic Web Journal* 7(4), pp. 399-419, 2016. DOI: 10.3233/SW-150200
   — **The VOWL specification paper.** Defines the complete visual notation mapping OWL constructs to graphical primitives.

4. Lohmann et al. "WebVOWL: Web-based Visualization of Ontologies." *EKAW 2014 Satellite Events*, LNAI 8982, 2015. DOI: 10.1007/978-3-319-17966-7_21
   — **The WebVOWL tool paper.** D3.js force-directed rendering of VOWL notation.

5. Negru et al. "Towards a Unified Visual Notation for OWL Ontologies: Insights from a Comparative User Study." *ISWC 2014 Posters & Demos*, 2014.
   — User study showing VOWL is more understandable by non-experts than competing notations.

6. Dudas et al. "Roadmapping and Navigating in the Ontology Visualization Landscape." *EKAW 2014*, LNCS 8876, 2014. DOI: 10.1007/978-3-319-13704-9_11
   — Rule-based recommender for choosing suitable ontology visualization tools.

### Layout Algorithms for Typed Graphs

7. Sugiyama et al. "Methods for Visual Understanding of Hierarchical System Structures." *IEEE Trans. SMC* 11(2), 1981. DOI: 10.1109/TSMC.1981.4308636
   — **The Sugiyama algorithm.** Layer assignment + crossing reduction + coordinate assignment for DAGs.

8. Reingold & Tilford. "Tidier Drawings of Trees." *IEEE Trans. Software Eng.* SE-7(2), 1981. DOI: 10.1109/TSE.1981.234519
   — Canonical tree drawing algorithm producing aesthetically optimal layouts in linear time.

9. Jacomy et al. "ForceAtlas2, a Continuous Graph Layout Algorithm for Handy Network Visualization." *PLOS ONE* 9(6), 2014. DOI: 10.1371/journal.pone.0098679
   — Barnes-Hut simulation with degree-dependent repulsion; default layout in Gephi.

10. Noack, A. "Modularity Clustering is Force-Directed Layout." *Physical Review E* 79(2), 2009. DOI: 10.1103/PhysRevE.79.026102
    — **Proves force-directed layout subsumes modularity clustering.** Theoretical bridge between layout and semantic grouping.

11. Healy & Nikolov. "Hierarchical Drawing Algorithms." Ch. 13 in *Handbook of Graph Drawing and Visualization*, CRC Press, 2013.
    — Comprehensive treatment of Sugiyama variants, crossing minimization heuristics, and coordinate assignment.

### Ontology Learning / Schema Inference

12. Cimiano & Volker. "Text2Onto -- A Framework for Ontology Learning and Data-Driven Change Discovery." *NLDB 2005*, LNCS 3513, 2005. DOI: 10.1007/11428817_21
    — Probabilistic ontology model (POM) supporting incremental updates as corpus changes.

13. Navigli & Velardi. "Learning Domain Ontologies from Document Warehouses and Dedicated Web Sites." *Computational Linguistics* 30(2), 2004. DOI: 10.1162/089120104323093276
    — OntoLearn: terminology extraction + WordNet disambiguation + hierarchical arrangement.

14. Lo et al. "End-to-End Ontology Learning with Large Language Models." *NeurIPS 2024*, 2024. arXiv: 2410.23584
    — OLLM: finetunes an LLM to build taxonomic ontology backbones from scratch; novel evaluation metrics.

15. Babaei Giglou et al. "LLMs4OL 2024: The 1st Large Language Models for Ontology Learning Challenge." *CEUR Workshop Proceedings*, 2024. arXiv: 2409.10146
    — Benchmark for LLM-based ontology learning covering term typing, taxonomy discovery, relation extraction.

### Knowledge Graph Construction from Documents

16. Zhong et al. "A Comprehensive Survey on Automatic Knowledge Graph Construction." *ACM Computing Surveys* 56(4), 2024. DOI: 10.1145/3618295
    — Reviews 300+ methods across NER, relation extraction, event extraction, and schema-constrained pipelines.

17. Jaradeh et al. "Information Extraction Pipelines for Knowledge Graphs." *KAIS* 65, 2023. DOI: 10.1007/s10115-022-01826-x
    — Plumber: 40 reusable components generating 432 distinct KG pipelines; transformer-based pipeline selection.

18. Chen et al. "LLM-Empowered Knowledge Graph Construction: A Survey." arXiv: 2510.20345, 2025.
    — How LLMs reshape the three-layered KG construction pipeline (ontology, extraction, fusion).

### Multi-View / Multi-Perspective Visualization

19. Furnas, G.W. "Generalized Fisheye Views." *CHI '86*, 1986. DOI: 10.1145/22627.22342
    — **Degree-of-interest (DOI) function.** Foundational formalism for focus+context visualization.

20. Cockburn et al. "A Review of Overview+Detail, Zooming, and Focus+Context Interfaces." *ACM Computing Surveys* 41(1), 2009. DOI: 10.1145/1456650.1456652
    — Comprehensive review categorizing multi-scale interfaces with empirical evidence synthesis.

21. Wiens et al. "Semantic Zooming for Ontology Graph Visualizations." *K-CAP 2017*, ACM, 2017. DOI: 10.1145/3148011.3148015
    — Semantic zooming for ontology graphs with three levels of detail; directly extends VOWL/WebVOWL.

### PKM Graph Quality and Zettelkasten Formalization

22. Schmidt, J.F.K. "Niklas Luhmann's Card Index: Thinking Tool, Communication Partner, Publication Machine." In *Forgetting Machines*, Brill, 2016.
    — Luhmann's numbering scheme creates branching addresses enabling non-hierarchical cross-references; produces emergent structure rather than pre-planned taxonomy.

23. Schmidt, J.F.K. "Niklas Luhmann's Card Index: The Fabrication of Serendipity." *Sociologica* 12(1), 2018. DOI: 10.6092/issn.1971-8853/8350
    — How structural properties of the Zettelkasten produced unexpected connections — why flat backlink graphs fail to replicate this.

24. Ahrens, S. *How to Take Smart Notes*, 2nd ed., 2022.
    — Primary English treatment of Zettelkasten methodology; formalizes fleeting/literature/permanent note distinction that most PKM tools fail to enforce.

### OWL/RDF in Software Engineering

25. Atzeni & Atzori. "CodeOntology: RDF-ization of Source Code." *ISWC 2017*, LNCS 10588, 2017. DOI: 10.1007/978-3-319-68204-4_2
    — OWL 2 ontology for code + parser serializing Java to RDF triples; enables SPARQL over code structure.

26. Fill & Burzynski. "Enterprise Architecture Reference Modeling in OWL/RDF." *ER 2005 Workshops*, LNCS 3770, 2005. DOI: 10.1007/11574620_60
    — Ontological formalization of enterprise architecture enabling distributed extension and consistency checking.

### Investigative Analysis and Sensemaking

27. Pirolli, P. and Card, S.K. "The Sensemaking Process and Leverage Points for Analyst Technology as Identified Through Cognitive Task Analysis." *Proc. International Conference on Intelligence Analysis*, 2005.
    — **The sensemaking loop.** Defines the foraging-synthesis cycle that analysts use: search → filter → extract → schematize → build case → tell story. Foundational model for intelligence analysis tooling.

28. Stasko, J., Gorg, C., and Liu, Z. "Jigsaw: Supporting Investigative Analysis through Interactive Visualization." *Information Visualization* 7(2), pp. 118-132, 2008. DOI: 10.1057/palgrave.ivs.9500180
    — Multi-view system for investigative analysis of document collections: list view, timeline, graph, map, and document views of the same entities. Directly relevant to dimensional switching.

29. Bier, E., Card, S.K., and Bodnar, J.W. "Entity-Based Collaboration Tools for Intelligence Analysis." *Proc. IEEE VAST 2008*, pp. 99-106, 2008. DOI: 10.1109/VAST.2008.4677362
    — Entity workspace for analysts: drag entities from documents into a structured canvas, create typed relationships, overlay timeline/geography. Shows how schema-aware entity extraction transforms raw documents into investigative structure.

30. Kang, Y., Gorg, C., and Stasko, J. "Evaluating Visual Analytics Systems for Investigative Analysis: Deriving Design Principles from a Case Study." *Proc. IEEE VAST 2009*, pp. 139-146, 2009. DOI: 10.1109/VAST.2009.5333878
    — Empirically derives design principles for visual analytics in investigation: support flexible schemas, allow analysts to create structure incrementally, show what's missing not just what's present.

31. Heer, J. and Shneiderman, B. "Interactive Dynamics for Visual Analysis." *ACM Queue* 10(2), 2012. DOI: 10.1145/2133416.2146416
    — Taxonomy of interaction techniques for visual analysis: select, explore, reconfigure, encode, abstract/elaborate, filter, connect. Framework for designing the navigator's interaction model.

### Causal and Abductive Reasoning

32. Pearl, J. *Causality: Models, Reasoning, and Inference*, 2nd ed., Cambridge University Press, 2009.
    — Foundational treatment of causal DAGs, do-calculus, and interventional reasoning. Theoretical basis for ECC's causal graph model and gap-based hypothesis generation.

33. Josephson, J.R. and Josephson, S.G. *Abductive Inference: Computation, Philosophy, Technology*, Cambridge University Press, 1994.
    — Formalizes abductive reasoning (inference to the best explanation) — the logic behind "the schema says X should exist but doesn't, therefore investigate." Directly relevant to gap-based hypothesis generation.
