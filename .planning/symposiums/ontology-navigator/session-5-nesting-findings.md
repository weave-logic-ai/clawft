# Session 5 Findings: Graph Nesting

## Data Model: Keep flat graph with Contains edges

Don't add recursive SubGraph structs. The existing KnowledgeGraph
with `RelationType::Contains` edges already works:

- `subgraph()` method extracts subsets by following edges
- Tree calculus encoding derived at render time (0 Contains children
  = Atom, ordered children = Sequence, typed children = Branch)
- `ResourceTree` in exo-resource-tree proves flat-map-with-parent-
  pointers works at scale

Add one convenience method: `fn children(&self, id: &EntityId) -> Vec<&Entity>`

## Drill-Down Algorithm

1. `children(node_id)` — entities reachable via Contains
2. Filter by schema's `contains` field for that node type
3. Build local `subgraph()` from those IDs
4. Layout subgraph using child type's declared geometry
5. Scan for cross-level edges (endpoints outside child set)
6. Cache result keyed by `(node_id, schema_version)`

Pure query, no data copy needed beyond positioned geometry output.

## Breadcrumb State

```rust
struct BreadcrumbFrame {
    node_id: EntityId,
    viewport_transform: Transform2D,  // pan/zoom matrix
    selected_ids: Vec<EntityId>,
    active_mode: ModeState,
}

// Navigation stack: Vec<BreadcrumbFrame>
// Drill-in: push frame, reset zoom to fit, clear selection
// Drill-out: pop and restore
// Forward: separate redo stack, clears on new drill-in
```

Cap inline expansion at 3 levels. Beyond that, force split-pane
or replace-in-place navigation.

## Cross-Level Edge Rendering

- **Expanded subgraph**: edges leaving boundary terminate at portal
  glyph (small circle + arrow on border). Click to navigate.
- **Collapsed subgraph**: aggregate children's external edges onto
  parent node. Merge edges to same target → single weighted edge
  (thickness = count, tooltip = breakdown).
- Reuse existing `Hyperedge` struct for collapsed representation.

## Compound Node Rendering

| Zoom Level | Rendering |
|-----------|-----------|
| Collapsed | Standard shape + chevron + count badge |
| Preview (L3) | 64x64 minimap of internal layout (cached) |
| Expanded (L4) | Children inside enlarged bounding box, 4px inset |

Virtualize at depth 3 or when visible nodes exceed 500.

## Split-Pane View

- Ctrl+click opens child subgraph in right pane (horizontal split)
- Parent keeps viewport, child fits subgraph
- Synchronized selection: selecting a node highlights counterpart
  in other pane with connecting bezier curve in overlay layer
- Max 3 simultaneous panes, additional replaces rightmost
