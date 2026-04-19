# ADR-001: Primitive Canon (20 Items — 19 Tier-A + 1 Tier-B)

**Date**: 2026-04-18 (initial), 2026-04-19 (amended — see ADR-013, ADR-014)
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

**Amendment note (2026-04-19)**: Two changes in this revision:

1. **Reconciliation with `protocol-spec.md §7.2`** — the spec already
   listed `StreamView` (`ui://stream-view`) as the streaming-native
   anchor primitive (session-6 CDDL rec. 5), but it was missing from
   the original ADR-001 table. Added as row 18.
2. **Crosswalk against the egui demo surface** (hosted locally via
   `weft-demo-lab`) surfaced three gaps: a freeform 2D painting
   surface, a semantic distinction between modal and non-modal
   floating surfaces, and two affordance-level additions. Resolved
   by:
   - **ADR-013** adds `ui://canvas` as a new Tier-A primitive
     (row 19).
   - **ADR-014** extends `ui://modal` with a `modality` field
     (`modal | floating | tool | toast`) — one primitive, typed
     modality.
   - **ADR-006 amendment** adds `tooltip` and `reorderable` to the
     primitive-head.

## Context

Session 5 (Renderer Contracts, RQ5) derived the smallest primitive set that
satisfies the four predicates from `foundations.md` (ontology-addressable,
self-describing, streaming-native, dual-operator) and maps cleanly onto
egui's immediate-mode model. Session 1's historical canon (Naked Objects'
reflection rule, Morphic's halo, X11's pixmap = opaque handle) and
Session 4's reverse-DDD mapping table (aggregate root → Surface, entity →
Chip, value → Field) both converge on the same shape: roughly eighteen
primitives, one of which is the sole Tier-B wrapper. Session 9's framing
paper made this a load-bearing recommendation (rec. 3, "extend canvas.rs").
Without a frozen canonical list, every downstream track re-invents its
vocabulary and the renderer-disposability success criterion from the
AGENDA fails.

## Decision

Freeze the canon at twenty primitives — nineteen Tier-A (all four
predicates, mandatory) plus one Tier-B wrapper (`ForeignSurface`, whose
*shell* is Tier-A). Each carries an ontology IRI stem under
`ui://`, a purpose statement, a four-predicate checklist (all four for
Tier-A; shell only for Tier-B), and a community egui crate mapping.

| # | IRI stem | Purpose | P1 | P2 | P3 | P4 | egui implementation |
|---|----------|---------|----|----|----|----|---------------------|
| 1 | `ui://chip` | Labelled interactive status token | Y | Y | Y | Y | `egui::Frame` + `selectable_label` |
| 2 | `ui://pressable` (Button) | Invoke-one-verb affordance | Y | Y | Y | Y | `egui::Button` / `Hyperlink` |
| 3 | `ui://field` | Typed input binding | Y | Y | Y | Y | `TextEdit` / `DragValue` / `DatePickerButton` / `ComboBox` / `CodeEditor` |
| 4 | `ui://toggle` | Boolean binding (agent-reasoned) | Y | Y | Y | Y | `ui.toggle_value`, `Checkbox` |
| 5 | `ui://select` | Closed-choice picker | Y | Y | Y | Y | `ComboBox` / `RadioButton` / `TableBuilder` |
| 6 | `ui://slider` | Continuous range binding | Y | Y | Y | Y | `egui::Slider` |
| 7 | `ui://stack` | One-axis container | Y | Y | Y | Y | `ui.horizontal` / `ui.vertical` |
| 8 | `ui://grid` | Two-axis regular layout | Y | Y | Y | Y | `egui::Grid` |
| 9 | `ui://strip` | Fixed-ratio division | Y | Y | Y | Y | `egui_extras::StripBuilder` |
| 10 | `ui://dock` | Resizable tabbed / split panels | Y | Y | Y | Y | `egui_dock::DockArea` |
| 11 | `ui://sheet` | Scrollable region, sticky headers | Y | Y | Y | Y | `egui::ScrollArea` |
| 12 | `ui://modal` | Floating surface with typed modality (modal / floating / tool / toast) — see ADR-014 | Y | Y | Y | Y | `egui::Area` + scrim / `Window` |
| 13 | `ui://table` | Sortable tabular data | Y | Y | Y | Y | `egui_extras::TableBuilder` |
| 14 | `ui://tree` | Hierarchical disclosure | Y | Y | Y | Y | `egui::CollapsingHeader` |
| 15 | `ui://gauge` | Scalar with bounds + confidence halo | Y | Y | Y | Y | `ProgressBar` / `Painter::arc` |
| 16 | `ui://plot` | Continuous time-series | Y | Y | Y | Y | `egui_plot` |
| 17 | `ui://media` | Decoded image / icon / glyph | Y | Y | Y | Y | `egui::Image` + `egui_extras` loaders |
| 18 | `ui://stream-view` | Live-tailing view over event-sourced subscription — streaming-native anchor | Y | Y | Y | Y | `ScrollArea` + ring buffer from `Store` |
| 19 | `ui://canvas` | Freeform 2D painter surface (pan / zoom / hit-test / draw) — see ADR-013 | Y | Y | Y | Y | `ui.allocate_painter()` + `Sense::drag` + `Painter` |
| 20 | `ui://foreign` (Tier B) | Opaque content host | shell Y | shell Y | shell Y | shell Y | per-kind (wry, egui_term, GStreamer, capture APIs) |

Deliberately *composed* (not primitives): `Form` = `Stack(Field…) +
Button`; `Card` = themed `Stack` with `Gauge/Chip` cluster;
`Toolbar/Menu/Tabs` = `Stack(Button…)` / `Stack(Select) + Sheet`;
`Notification/Toast` = `Modal` at top order; `Launcher/Tray` =
`Dock + Chip`. Keeping these out of the canon preserves the
reverse-DDD arrow (Session 4): composition is the authored layer,
primitives are the vocabulary.

## Consequences

### Positive
- One renderer-agnostic vocabulary every agent and renderer agrees on.
- Community egui crates (`egui_dock`, `egui_plot`, `egui_extras`,
  `egui_term`) do the retained-mode heavy lifting; no in-house forks.
- Composition-over-primitive keeps the canon ≤25, satisfies
  foundations' anti-Tier-C rule, and pushes bespoke UI into wiring.
- IRIs stabilise the agent `query`/`invoke`/`subscribe` surface;
  second-renderer success criterion becomes mechanically testable.

### Negative
- Ambient-capture wrappers (`camera`/`mic`/`screen`) all collapse
  into one `ui://foreign` type — they differ only in `wrapper-type`,
  which moves nuance into runtime checks.
- `Form` / `Card` / `Tabs` becoming compositions will surprise
  anyone who expects shadcn-style catalogues.
- Eighteen is already near the upper bound a human can remember;
  any future addition needs an explicit ADR.

### Neutral
- The existing spike at `crates/clawft-gui-egui/src/{blocks,shell,live}`
  is ~60% compliant; the remainder reorganises under this canon or
  demotes to `ForeignSurface` / composition.

## Alternatives considered

1. **Flat catalogue of ~50 primitives (AI Elements shape)** —
   rejected: Session 2 showed AI Elements fails 3/4 predicates per
   widget; a larger catalogue would make agent introspection worse,
   not better. Composition already covers the extra shapes.
2. **Purely Morphic (every visible object is a primitive)** —
   rejected per Session 1: in-image identity cannot leave the host
   runtime, violating predicate 1 at the wire.
3. **Naked-Objects one-rule-per-construct (no geometry)** —
   rejected per Session 4: produces bureaucratic uniform screens
   (Cooper posture failure); our canon must consult
   `ontology-navigator` geometry before choosing a primitive.
4. **No Tier-B at all; demand all content be Tier-A** — rejected per
   foundations: browsers, videos, X windows, nested Tauri apps
   legitimately cannot satisfy predicate 1; banning them hides
   them from governance and erases the tapestry signal channel.

## Related

- Sessions: `session-1-historical-canon.md` (rec. 6 halo,
  rec. 9 ForeignSurface), `session-4-ontology-ui.md` (DDD→primitive
  table), `session-5-renderer-contracts.md` (18-primitive canon,
  egui mapping), `session-9-agentic-os-canon.md` (rec. 3).
- Foundation elements: four predicates, two-tier rule
  (foundations §Two tiers), Non-negotiable §"Anti-rule" (no Tier-C).
- ADRs: ADR-002 (promotion path), ADR-006 (primitive head,
  amended), ADR-010 (reverse-DDD mapping), ADR-013 (`ui://canvas`
  primitive added), ADR-014 (`ui://modal` modality split).
- Validation: full crosswalk against `https://www.egui.rs/#demo`
  surface (hosted locally in `weft-demo-lab`) on 2026-04-19 —
  17 of 20 visible demo behaviors already mapped to this canon
  directly; the three gaps drove ADR-013/014 and the ADR-006
  amendment.
