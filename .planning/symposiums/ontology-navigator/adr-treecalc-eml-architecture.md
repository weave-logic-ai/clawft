# ADR: Tree Calculus + EML as Dual Computational Substrate

**Status**: Proposed
**Date**: 2026-04-17
**Context**: Universal Topology Browser architecture

## Decision

The topology browser's layout engine uses **two complementary
computational models** operating on the same data:

- **Tree calculus** handles discrete structural logic: pattern
  matching, dispatch, composition, graph decomposition
- **EML** handles continuous mathematics: learned parameters,
  scoring functions, threshold optimization

Both execute as native Rust. No LLM. No JavaScript layout libraries.

## The Two Sides

| Aspect | Tree Calculus | EML |
|--------|--------------|-----|
| **Domain** | Structure, logic, dispatch | Numbers, optimization, approximation |
| **Operation** | Triage (leaf/stem/fork match) | exp-ln function composition |
| **Input** | Topology tree (discrete) | Feature vector (continuous) |
| **Output** | Transformed tree | Scalar or parameter vector |
| **Learning** | Fixed rules (5 reductions) | Trained from data (CD updates) |
| **Cost** | Pattern match (nanoseconds) | exp + ln (nanoseconds) |

## Combined Pipeline

```
Raw graph
  │
  ▼
[1] Topology encoding (tree calculus)
    Graph → Tree { Atom | Sequence | Branch }
    Every node classified by triage: leaf/stem/fork
  │
  ▼
[2] Structural dispatch (tree calculus)
    Triage on tree shape:
      Atom   → point layout (no children, just position)
      Sequence → timeline/stream layout
      Branch → recursive layout (tree, DAG, force)
  │
  ▼
[3] Parameter computation (EML)
    Per-dispatch-branch, EML models compute continuous params:
      LayoutModel(node_count, edge_count, density)
        → spring_length, repulsion, damping, gravity
      ClusterThresholdModel(node_count, edge_density, community_count)
        → cohesion_threshold, split_size
      ForensicCoherenceModel(density, avg_confidence, ...)
        → coherence_score per dimension
  │
  ▼
[4] Layout reduction (tree calculus)
    Apply positioned geometry as tree→tree transform:
      Input: topology-tree + EML params
      Output: geometry-tree (each node has x, y, width, height)
    Reductions compose: layout ∘ heatmap ∘ flow = single pass
  │
  ▼
[5] Serialization (RVF)
    Geometry-tree → RVF wire frame
    Signed by ExoChain, zero-copy deserializable
  │
  ▼
[6] Rendering (thin client)
    RVF frame → pixels
    Browser (SVG/Canvas), TUI (ratatui), Tauri (native)
    Renderer does NO layout — just paints coordinates
```

## Why This Is Fast

Step 1-2: Tree encoding + triage = pattern matching. Compiler
optimizes to jump tables. Nanoseconds per node.

Step 3: EML evaluation = `a * exp(b * x) + c * ln(d * x + e)`.
Five multiplies, one exp, one ln per model. Already benchmarked
at <1μs per evaluation in eml-core.

Step 4: Layout reduction = tree walk with position accumulation.
O(n) for tree/timeline, O(n log n) for force (Barnes-Hut).

Step 5: RVF serialization = memcpy into segments. Zero-copy.

Step 6: Rendering = iterate positioned nodes, emit SVG/draw calls.

**Total**: a 10K-node graph should layout in <50ms in pure Rust.
No GC, no JS event loop, no D3 simulation ticks.

## What EML Models We Need

### Existing (already trained)
- **LayoutModel**: graph stats → physics params (6 outputs)
- **ClusterThresholdModel**: → community detection thresholds
- **SurpriseScorerModel**: → edge surprise score
- **ForensicCoherenceModel**: → dimension coherence
- **QueryFusionModel**: → search relevance

### New (to build)
- **GeometryClassifier**: graph features → geometry type
  Input: (max_in_degree, has_cycles, timestamp_ratio,
  contains_edge_ratio, degree_variance)
  Output: probability vector over [force, tree, layered,
  timeline, stream, grid]
  Training: manually classified graphs from existing exports

- **SpacingModel**: per-geometry-type positioning params
  Input: (node_count, avg_label_length, depth, breadth)
  Output: (node_spacing, level_gap, margin)
  Training: user feedback on layout quality (1-5 rating)

- **OverlayAlignmentModel**: node matching confidence
  Input: (name_similarity, type_match, topology_similarity,
  neighbor_overlap)
  Output: match_probability
  Training: manually confirmed Plan↔Implemented node pairs

## Connection to Existing Architecture

The tree calculus + EML model slots into the existing crate
structure:

| Crate | Role |
|-------|------|
| `clawft-types` | `Topology` enum definition |
| `eml-core` | EML model evaluation (already exists) |
| `clawft-graphify` | Graph → Topology encoding, schema loading |
| `clawft-graphify::vault` | Document → typed graph pipeline |
| `clawft-kernel::ecc` | Causal edge types, coherence scoring |
| `clawft-weave` | CLI commands (`weaver topology layout`) |
| `exo-resource-tree` | RVF serialization of geometry trees |

No new crates needed. The `Topology` type is ~10 lines of Rust.
The triage dispatcher is ~30 lines. The integration with existing
EML models is function calls to `eml_core::EmlModel::evaluate()`.

## What the Renderer Receives

```rust
struct PositionedGraph {
    nodes: Vec<PositionedNode>,
    edges: Vec<PositionedEdge>,
    viewport: Rect,
    metadata: GraphMetadata,
}

struct PositionedNode {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    label: String,
    node_type: String,
    style: NodeStyle,       // from schema
    metrics: HashMap<String, f64>,  // for heatmap
    has_subgraph: bool,     // expandable indicator
}

struct PositionedEdge {
    source_id: String,
    target_id: String,
    label: Option<String>,
    path: Vec<(f64, f64)>,  // control points for curves
    style: EdgeStyle,       // from schema
    animated: bool,         // for flow mode
}
```

The React VOWL navigator becomes a consumer of this JSON. The
Tauri GUI becomes a consumer. A TUI becomes a consumer. The
renderer is fungible — the intelligence is in the Rust layout
engine.

## Trade-offs

**Pro**: blazing fast, formally grounded, no JS dependencies for
layout, renderer-agnostic, cryptographically auditable, composable
mode system

**Con**: we need to implement layout algorithms in Rust (force-
directed, Sugiyama, Reingold-Tilford) instead of using d3/elkjs.
This is real engineering work — probably 2-3 sprints for the core
algorithms.

**Mitigation**: start with the simplest layout (tree via
Reingold-Tilford, ~200 LOC in Rust) and force-directed (Barnes-Hut,
~400 LOC). These two cover the most common topologies. Add Sugiyama
later.

## Open Questions

1. Should the GeometryClassifier EML model replace or supplement
   the rule-based auto-detection heuristic from Session 3?
   (Recommendation: rule-based first, EML learns from corrections)

2. How do we handle the force-directed simulation's iterative
   nature? Tree calculus reductions are one-shot. Force layout
   needs N iterations. Do we run the simulation to convergence
   server-side and send the final positions?

3. Should the Topology enum be three-valued (Atom/Sequence/Branch)
   or should it carry richer structure (geometry hint, EML model
   ID, schema reference)?
