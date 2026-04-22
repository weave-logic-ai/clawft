# ADR-013: Add `ui://canvas` as a Tier-A Freeform Painter Primitive

**Date**: 2026-04-19
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (canon crosswalk)

## Context

The 2026-04-19 crosswalk of ADR-001's 18-primitive canon against
the full `https://www.egui.rs/#demo` surface (hosted locally via
`weft-demo-lab`) surfaced a class of demos that no existing primitive
covers cleanly:

- **Fractal Clock** (vendored `fractal_clock.rs`)
- **paint_bezier** (DemoWindows)
- **painting** (DemoWindows)
- **pan_zoom** (DemoWindows)
- **dancing_strings** (DemoWindows)

All of these allocate a typed `egui::Rect`, claim drag sense with
`ui.allocate_painter(size, Sense::drag())`, and paint arbitrary
primitives (circles, lines, beziers, polygons) via the `Painter`
API. The shared shape is "a typed rectangular surface the caller
draws on freely, with pan / zoom / hit-test interaction".

`ui://plot` is time-series only; `ui://media` is decoded-image only;
neither accepts draw calls. Routing these surfaces through
`ui://foreign` (Tier-B) would discard ontology-addressability —
the primary predicate — for a pattern that is both common and
cleanly typed. That is the exact anti-pattern ADR-002's promotion
path was designed to prevent.

## Decision

Add `ui://canvas` as the **18th Tier-A primitive** in the canon.
ADR-001 is amended in place; `weft-demo-lab` and `protocol-spec.md
§7.2.x` are updated accordingly.

```cddl
canvas = primitive-head // {
  type       : "ui://canvas",
  state      : {
    size         : [2*int],              ; render target in logical pixels
    ? view-box  : [4*float],              ; current pan/zoom viewport (x, y, w, h)
    ? layers    : [+ canvas-layer],       ; typed drawing layers (ADR-004 leaf CRDTs)
    ? cursor    : [2*float],              ; last hit-test position, if tracked
  },
  affordances : [
    * { name: "draw",       verb: "invoke",   ... },
    * { name: "hit-test",   verb: "invoke",   ... },
    * { name: "pan",        verb: "invoke",   ... },
    * { name: "zoom",       verb: "invoke",   ... },
    * { name: "snapshot",   verb: "observe",  ... },
  ],
}

canvas-layer = {
  id        : tstr,
  kind      : "strokes" | "bezier" | "shapes" | "raster",
  content   : any,                        ; layer-type-specific payload
}
```

### Affordances

- **`draw`** — append or replace a canvas-layer. Takes a `layer-id`
  + `content`. State updates flow through ADR-004's state-diff tree.
- **`hit-test`** — given a point, return the top-most layer and
  feature hit. Return feeds the active-radar `topology` channel
  (ADR-007) when the returned point is on a typed feature.
- **`pan` / `zoom`** — update `view-box`. Return echo carries
  `doppler` (delta over time) for coherence scoring.
- **`snapshot`** — subscribe to rasterised frames for observation /
  recording. Raster snapshots are privacy-gated per ADR-012.

### Implementation

- **egui**: `ui.allocate_painter(size, Sense::click_and_drag())` +
  per-layer painter calls. `view-box` handled via
  `RectTransform::from_to`. Hit-test via point-in-shape checks per
  layer.
- **Browser/wasm** (M1): same allocate_painter path; egui's immediate-
  mode calls compile to WebGL2 via eframe-wasm. No renderer changes.

### Relationship to existing primitives

- `ui://plot` is **not** deprecated; it remains specialised for time-
  series with axes, ticks, legends, and coherence overlays, backed by
  `egui_plot`. Canvas is the general-purpose drawing surface.
- `ui://media` is **not** deprecated; it renders *decoded* images or
  icons. Canvas may *include* raster layers, but the authoring affordance
  is draw-on-surface, not display-static-asset.
- Custom3D (`custom3d_glow.rs`) remains a Tier-B `ui://foreign` wrapper
  of kind `custom-gl`; it needs a `glow::Context` handle, which
  violates predicate 1 (ontology-addressability of the GL command
  stream). Promotion to Tier-A waits on a typed GPU-command schema.

## Consequences

### Positive
- Closes the largest single coverage gap revealed by the demo
  crosswalk. 5 of 20 demo behaviors move from "needs foreign" to
  "canon-native".
- Agents can compose fractal-clock-shaped surfaces (drawings,
  annotation layers, spatial pan/zoom maps) via `invoke draw`
  without a bespoke renderer.
- Layers are CRDT leaves under ADR-004, so Mission Console
  collaborative drawing falls out automatically — shared canvas
  without a new sync protocol.
- Privacy-flag discipline applies: `snapshot` is subject to the
  same capture-chip auto-composition rule as `foreign://screen`.

### Negative
- The primitive canon grows from 18 to 19. This is within the
  human-memorable bound but cuts the margin ADR-001 called out.
- `canvas-layer` payload shapes are deliberately open-ended;
  ontology governance must pin accepted kinds per project or agents
  will invent incompatible variants.
- `hit-test` return echoes can be high-rate; observation
  back-pressure (Round-3 open question, `ROADMAP.md`) is non-
  optional for canvas-heavy rooms.

### Neutral
- The existing egui spike's `oscilloscope.rs` block continues to
  use `egui_plot`; the canvas primitive does not replace it.

## Alternatives considered

1. **Treat canvas shapes as `ui://foreign` kind=`canvas`** —
   rejected: the drawing surface is typed, hit-testable, and
   governance-relevant (drawings on an architectural plan are
   authored content, not opaque pixels). Foreign-surfacing throws
   predicate 1 away for a structurally typed case.
2. **Specialise `ui://plot` to accept non-series payloads** —
   rejected: conflates two primitives whose axes, scales, and
   interaction models diverge. The `egui_plot` API does not
   accommodate freeform layers cleanly, and plot's overlay/legend
   vocabulary is meaningless for a canvas.
3. **Add `ui://vector` (SVG-only) and a separate `ui://raster-paint`** —
   rejected: two narrow primitives for the same demo category
   violate the canon's minimalism. The `canvas-layer.kind`
   enumeration covers both within one primitive.
4. **Delay until M2 / prototype experience** — rejected: we ship
   `weft-demo-lab` today, the gap is visible, and the canvas shape
   is unambiguous. Deferring forces a foreign-surface workaround
   every demo session.

## Related

- Session sources: `session-1-historical-canon.md` (X11 Porter-Duff
  draw rules), `session-5-renderer-contracts.md` (rec. 4, Custom3D
  deferral), `session-9-agentic-os-canon.md` (rec. 3 canvas.rs
  extension — this ADR is the spec that rec. hinted at).
- Foundation elements: predicate 1 (ontology-addressable —
  canvas-layer kinds are IRI-typed), predicate 3 (streaming-native
  — draw / snapshot are streamed), predicate 4 (dual-operator —
  humans draw; agents invoke draw / hit-test).
- ADRs: ADR-001 (canon table amended to include row 18 canvas),
  ADR-004 (state-diff tree + CRDT leaves — canvas layers are the
  archetypal CRDT leaf), ADR-007 (active-radar — hit-test feeds
  topology; pan/zoom feeds doppler), ADR-012 (snapshot capture
  gated by the capture-chip rule).
