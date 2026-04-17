# Q&A Decisions — Founder Responses

**Date**: 2026-04-17

---

## 1. Schema Files: Composable like Docker configs

**Q**: Standalone `.weftos-ontology.yaml` or embedded in CLAUDE.md?

**A**: Composable. Like Docker configs — layer them.

**Decision**: Schema files compose via layering. A base schema
(`software.yaml`) can be extended by a project schema
(`myproject.yaml`) which can be extended by a user schema
(`local.yaml`). Later layers override earlier layers. Fields
merge (node types added, not replaced), styles override.

```
base:    weftos:software.yaml        (ships with WeftOS)
project: .weftos/ontology.yaml       (checked into repo)
local:   .weftos/ontology.local.yaml (gitignored, user prefs)
```

Schema loader reads all three, merges in order. Identical to
how Docker Compose handles `docker-compose.yml` +
`docker-compose.override.yml` + env-specific files.

**Implementation**: add `extends: "software"` field to schema
header. Loader resolves the chain and deep-merges.

---

## 2. Schema Inference: Generate from code/data/exhaust

**Q**: Can we infer schema from existing graphify output?

**A**: Yes. Infer from code, data, and digital exhaust. A stored
schema next to inferred schema shows ongoing drift/changes.

**Decision**: Two schema sources that diff against each other:

- **Declared schema** — hand-authored or generated once, checked in
- **Inferred schema** — continuously generated from graphify analysis

The diff between them is itself a valuable signal:
- New entity types appearing in code that aren't in the schema → schema drift
- Schema types that no longer appear in code → dead ontology
- Relationship patterns changing → architectural evolution

**Implementation**: `weaver ontology infer <path>` generates a
schema from a graphify KnowledgeGraph. `weaver ontology diff`
compares declared vs inferred. The inferred schema uses the
same YAML format so it can be promoted to declared.

---

## 3. Causality: Full orthogonal dimension (two axes)

**Q**: Is causality a layer in the ontology or orthogonal?

**A**: Full orthogonal dimension. Probably two:
- **Temporal** — what happened when (delta-t)
- **Counterfactual/logical** — what would happen if (causal reasoning)

**Decision**: ECC causal edges are NOT ontology layers. They are
a separate dimensional axis that can overlay ANY ontology layer.
The causal graph lives alongside the structural graph — same
nodes, different edge sets.

Two causal dimensions:
1. **Temporal causality** — A preceded B, A triggered B (observable)
2. **Counterfactual causality** — if A hadn't happened, B wouldn't
   have happened (logical/inferred)

Both are overlay modes in the navigator, not base modes.

---

## 4. OWL/RDF Compatibility: Follow the pattern

**Q**: OWL/RDF or simpler custom format?

**A**: It is a pattern. Unless we have a compelling alternate syntax,
stick to it.

**Decision**: Use OWL/RDF as the interop format. The internal
representation can be simpler (YAML schemas, Rust types), but
export MUST produce valid OWL/RDF. Import MUST consume it.

This means:
- IRIs are mandatory for published schemas
- `same_as` maps to `owl:sameAs`
- `contains` maps to `rdfs:subClassOf` or custom properties
- VOWL JSON export is already RDF-compatible
- Consider `oxigraph` or `sophia` Rust crates for RDF parsing

We don't need to USE OWL internally, but we need to SPEAK it.

---

## 5. Cross-Layer Entities: Use IRI

**Q**: How handle entities that span layers?

**A**: Use IRI.

**Decision**: An entity that appears in multiple layers (e.g.,
"Service" is both architecture and infrastructure) gets one IRI
and appears in both layers' views. The navigator shows it in
whichever layer the user is viewing. When filtering by layer,
cross-layer entities remain visible with a visual indicator
(border highlight, badge) showing they belong to other layers too.

This is the correct OWL approach — a class can be a member of
multiple taxonomies simultaneously via `rdf:type`.

---

## 6. Dimensional Overlays: Single mode active

**Q**: Can analyst view causal + temporal simultaneously?

**A**: TBD, but seems like single mode to me.

**Decision**: Start with single overlay mode active at a time.
The base mode (Explore, Timeline, Cluster) can have ONE overlay
(Diff, Heatmap, Flow) active. If composition proves valuable
after testing, we can relax this constraint.

Rationale: composing two overlays (e.g., Heatmap + Flow) requires
resolving visual conflicts (both want to control color). Single
overlay avoids this complexity initially.

---

## 7. Ghost Nodes: Topological distortion, not placeholder icons

**Q**: What does the ghost node UX look like?

**A**: Empty space. Like dark matter — they should push and pull
on the graph. Use the z-axis to create a topology surface that
things lay on, like a topological map with elevation. Something
that's needed creates distorted space around it, giving a visual
cue to look at other layers.

**Decision**: Ghost nodes are NOT rendered as dashed outlines or
placeholder icons. They are **topological distortion** — the
absence of a node warps the graph around it, like gravitational
lensing.

Implementation approach:
- **Force layout**: add a virtual attractor at the position where
  the missing node should be. Nearby nodes are pulled toward the
  empty space, creating a visible "dent" or cluster with a hole
  in the middle.
- **Z-axis surface**: render the graph on a 3D surface (or 2.5D
  height map). Missing nodes create elevation changes — valleys
  or ridges that distort the flat graph plane. Like a topographic
  map where the terrain tells you something is there even when
  you can't see it directly.
- **IR/overlay analogy**: like infrared data overlaid on visible
  light imagery. The "visible" layer shows what exists. The
  "infrared" layer (toggle-able) shows the force fields of what
  SHOULD exist, warping the surface.

This is a more sophisticated approach than ghost nodes. The
navigator doesn't show the missing thing — it shows the **effect
of the missing thing** on everything else. The analyst's eye is
drawn to the distortion, not to a placeholder.

**Phase 1 (now)**: Heatmap/topology overlay in SVG. Missing nodes
generate a force field that distorts neighbor positions. The
distortion is visualized as a color gradient overlay — warm colors
where the "pull" of a missing node is strongest. Implemented
entirely in the existing force layout by adding virtual attractors
with no rendered node.

**Phase 2 (future exploration)**: 3D surface rendering. The graph
lays on a heightfield mesh where elevation encodes structural
completeness. Missing nodes create valleys; high-connectivity nodes
create peaks. WebGL or Canvas shader. This is a separate R&D
effort — compelling visual but significant implementation cost.
Should be explored as its own mini-symposium when the 2D overlay
proves the concept.
