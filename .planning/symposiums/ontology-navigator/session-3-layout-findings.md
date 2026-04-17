# Session 3 Findings: Layout Follows Structure

## Algorithm Decisions

**Two libraries, not six**: `d3-force` (already integrated) + `elkjs` (Sugiyama DAG, tree, rect-packing, compound nodes). ELK is actively maintained, runs in JS (~400KB async), and handles compound nodes natively — critical for our nesting model.

| Geometry | Algorithm | Library |
|----------|-----------|---------|
| Network | D3-force (velocity Verlet) | `d3-force` |
| Tree | Reingold-Tilford | `d3-hierarchy` |
| DAG | Sugiyama (longest-path + barycenter) | `elkjs` |
| Timeline | Swimlane + time axis | Custom on `d3-scale` |
| Stream | Inline SVG sparkline | `d3-shape` |
| Grid/ER | Rect-packing | `elkjs` |

## Auto-Detection Heuristic (no schema fallback)

Run in order, first match wins:

1. >50% nodes have timestamp → timeline
2. Kahn's algorithm: cyclic → force
3. All nodes in-degree ≤ 1 → tree
4. Acyclic, some multi-parent → Sugiyama DAG
5. >60% edges are "Contains"/"Parent" → tree (cross-edges as dotted arcs)
6. Low degree variance → grid; high variance → force
7. Fallback: force-directed

Implementable as `detectGeometry(graph) -> LayoutType` in ~80 lines.

## Semantic Zoom Levels

| Level | Scale | Render |
|-------|-------|--------|
| L1 Distant | <0.3 | Colored dots, label on hover, edges hidden below 0.15 |
| L2 Neighborhood | 0.3–0.8 | Shaped nodes, edge labels, thin edges |
| L3 Detail | 0.8–1.5 | Full node card, metadata, inline sparklines |
| L4 Subgraph | >1.5 or double-click | Child graph expanded inline with own geometry |

Driven by `transform.k` → quantized `zoomLevel` state.

## Performance

- SVG viable up to ~2,000 visible nodes
- Beyond that: viewport culling via quadtree (already in D3), not React virtualization
- elkjs async load, no WASM needed below 5K nodes

## Transitions

- **FLIP technique**: record old positions, compute new layout, animate 500ms
- **Drill-in/out**: zoom-to-fit target, crossfade parent→child
- **Anti-teleport rule**: if >80% of nodes move >50% viewport, insert 200ms gather-to-center before 300ms expand

## Open Questions for Q&A

1. Should we ship elkjs as a dependency now, or start with d3-force-only and add elkjs when we implement tree/DAG modes?
2. The 2K node SVG limit — do we expect real-world graphs to exceed this? If so, should we plan Canvas rendering from the start?
3. For timeline mode, should the time axis be continuous (every second) or event-driven (only show times where nodes exist)?
