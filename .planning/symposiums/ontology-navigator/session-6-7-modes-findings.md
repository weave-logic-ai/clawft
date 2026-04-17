# Session 6-7 Findings: Modes + Investigative Instrument

## Rust Type Definitions

```rust
enum BaseMode { Explore, Timeline, Cluster, Wardley }
enum OverlayMode { Diff, Heatmap, Flow, Search }
enum ToolMode { Annotate, Measure, Select }

struct NavigatorState {
    base: BaseMode,
    overlay: Option<OverlayMode>,   // single overlay at a time
    tool: Option<ToolMode>,
    viewport: Viewport,
    selection: HashSet<EntityId>,
    overlay_config: OverlayConfig,
}
```

## State Machine: What Reruns on Mode Change

| Change | Layout | Encode | Interaction | Budget |
|--------|--------|--------|-------------|--------|
| Base switch | RERUN | RERUN | RERUN | <300ms |
| Overlay switch | keep | RERUN | keep | <100ms |
| Tool switch | keep | keep | RERUN | <16ms |
| Pan/zoom | keep | keep | keep | 60fps |

**Key rule**: only base mode changes rerun layout. Overlay and tool
changes are visual-only — that's why they feel instant.

## Topological Distortion (Heatmap Overlay)

When `distortion_strength > 0`:
1. Walk schema to find expected-but-missing connections
2. Insert invisible virtual attractor nodes at interpolated positions
3. Attractors pull nearby visible nodes during force iteration → "dent"
4. Color gradient encodes attractor force strength (cool=weak, hot=strong)

## Diff Overlay: Node Matching Cascade

1. **IRI match**: identical IRI = same node. O(n) hash join.
2. **Type+name fuzzy**: same entity_type + Levenshtein > 0.8
3. **Topology heuristic**: Jaccard overlap of neighbor labels > 0.6
4. **Manual pin**: analyst overrides

Visual encoding: solid green = aligned, ghost-distortion = gap,
yellow = drift, red = rogue.

Governance traceability: Plan = requirements, Implemented = design,
Actual = test results. Coverage % = fraction with solid-green path
through all three layers.

## Timeline Base Mode

- X = continuous time from `time_field` metadata
- Y = swimlanes by `lane_field` (defaults to entity_type)
- Duration events render as horizontal bars
- Point events render as circles
- Fisheye-on-time: sparse periods compress, dense clusters expand

## Search Overlay

Query matches label (substring), IRI (prefix), metadata (JSON path).
Matched nodes full opacity, rest at 15%. Path highlighting via
Dijkstra shortest path. Neighborhood via N-hop BFS with opacity
falloff.

## Cold Case Walkthrough (200 evidence items)

1. Ingest via graphify pipeline, forensic schema. ~2s
2. Explore mode, force layout. Click person → detail panel. ~300ms
3. Switch Timeline (base change, relayout). Scrub to 48hr window. ~250ms
4. Overlay Heatmap, metric=confidence. Distortion reveals missing
   phone records near suspect cluster. ~80ms
5. Drill into event cluster. Double-click → subgraph. ~100ms
6. Switch overlay to Diff. Theory vs evidence. Yellow drift on
   witness statements, red rogue alibi document. ~90ms
7. Annotate tool. Pin note to rogue node. ~16ms
8. Export annotated state for report.

Total interactive budget: base <300ms, overlay <100ms, tool <16ms.
