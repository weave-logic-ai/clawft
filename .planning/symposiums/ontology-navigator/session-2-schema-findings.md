# Session 2 Findings: TopologySchema Design

## Core Design Decision

**The schema is a view model, not a data model.** The existing Rust
enums (EntityType, RelationType) define what exists. The TopologySchema
YAML defines how things look and behave in the navigator. A user can
write a `marketing.yaml` that re-geometrizes the same code graph as
a radial diagram without touching Rust code.

## Schema Format Summary

A `.topology.yaml` file has 4 sections:

### 1. Header
`name`, `label`, `version`, `domain` (maps to DomainTag)

### 2. Nodes
Map of entity type discriminant → geometry config:
- `geometry`: force | tree | layered | timeline | stream | grid | geo | radial
- `contains`: what types appear when you drill down
- `style`: shape, color, icon, radius range
- `size_field`: metadata field for node sizing
- `time_field`: for timeline/stream geometry
- `lat_field`/`lng_field`: for geo geometry
- Wildcard `"*"` for fallback

### 3. Edges
List of valid edge types with constraints:
- `type`: matches RelationType display string
- `from`/`to`: entity type constraints (or `"*"`)
- `cardinality`: "1:1" | "1:N" | "N:1" | "N:M"
- `style`: stroke, width, dash, arrow, label
- `animated`: participates in flow mode

### 4. Modes
Config for diff (sources, colors), heatmap (metrics with palettes),
flow (edge types, speed), timeline (node types, lane field)

### 5. Constraints
`max_visible_nodes` (2000), `max_visible_edges` (5000),
`min_confidence` (0.0), `auto_cluster` (true)

## Key Architecture Decisions

1. **Keys match discriminant strings** — `"module"`, `"function"`,
   `"struct_"` etc. Stable string contract already tested in Rust.

2. **Geometry is per-node-type** — a single graph can have Package
   nodes in tree layout and Function nodes in force. The renderer
   picks geometry based on the focal node during drill-down.

3. **`contains` is a filter, not a creator** — it declares what the
   renderer SHOWS when drilling in, not what edges exist. If the data
   has no Contains edges, drill-down will be empty regardless.

4. **Custom types work via wildcard** — `EntityType::Custom("robot")`
   matches key `"robot"` in the schema. If absent, falls back to `"*"`.

5. **ECC causal types forward-compatible** — Causes/Enables/etc would
   be `Custom(String)` relations in the schema. No code changes needed.

6. **Soft validation** — warn on unknown keys, don't reject. Strict
   at render time (missing `time_field` on timeline → fall back to force).

## Presets Delivered

- **software.yaml** (complete): 14 node types, 11 edge types, all 4 modes
- **investigation.yaml** (complete): 14 node types, 12 edge types, timeline + geo
- **robotics.yaml** (sketch): 9 node types, 7 edge types, heatmap for telemetry

## Rust Implementation Shape

```rust
pub struct TopologySchema {
    pub name: String,
    pub label: String,
    pub version: String,
    pub domain: Option<String>,
    pub nodes: HashMap<String, NodeTypeConfig>,
    pub edges: Vec<EdgeTypeConfig>,
    pub modes: ModesConfig,
    pub constraints: ConstraintsConfig,
}

pub enum Geometry {
    Force, Tree, Layered, Timeline,
    Stream, Grid, Geo, Radial,
}
```

Deserializes with `serde_yaml`. ~150 lines of Rust for the types.

## Open Questions for Q&A

1. Should presets be shipped as embedded Rust constants (compiled in)
   or as standalone YAML files in a config directory?
2. The schema has no inheritance — should a `custom.yaml` be able to
   extend `software.yaml` with additional types?
3. How do we handle the case where a graph has nodes from BOTH code
   and forensic domains? Load both presets and merge?
4. The `contains` filter uses RelationType::Contains edges — what
   about other containment-like edges (MethodOf, MemberOf)?
