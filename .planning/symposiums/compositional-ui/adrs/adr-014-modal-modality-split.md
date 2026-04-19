# ADR-014: Extend `ui://modal` with Typed Modality — One Primitive, Four Modes

**Date**: 2026-04-19
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (canon crosswalk)

## Context

ADR-001 row 12 defined `ui://modal` as "focus-capturing bounded
decision" — the classic scrim + centred-panel dialog. The
2026-04-19 demo crosswalk showed that egui (and every serious
desktop surface framework) uses floating window-like surfaces
for three *additional*, semantically distinct purposes:

- **Tool palettes / inspectors** — stay open alongside the main
  work, do not capture focus, draggable, resizable. Uses:
  `DemoWindows` itself (every stock demo is a floating window), tool
  windows in graphics apps, inspector panes.
- **Transient toasts / notifications** — appear at top layer, do
  not capture focus, auto-dismiss. Uses: status messages, progress
  ticker, "saved" confirmations.
- **Tooltips** — the lightest non-modal floater; hover-triggered
  help text (ADR-006 amendment makes this an optional head field,
  but the renderer still composes a transient floating surface
  under the hood).

A single `Modal` primitive cannot model all four without a
semantic hint. The choice between modal and non-modal changes:
scrim presence, focus behaviour, dismissibility, interruption
priority (ADR-008 governance policy), and screen-reader
announcement. Naming them the same but behaving differently is
exactly the posture collapse Cooper warns about in *About Face*
ch. 9.

## Decision

Extend `ui://modal`'s `state` with a mandatory `modality` field
enumerating the four modes:

```cddl
modal-state = {
  modality : "modal" | "floating" | "tool" | "toast",
  ? title : i18n-string,
  ? dismissable : bool,                    ; default: true for modal/toast, always-true for floating/tool
  ? position : "centre" | "docked-right" | "docked-left" | [2*float],
  ? ttl-ms : uint,                         ; required for toast, forbidden otherwise
  body : [* primitive-ref],                ; composed primitives inside
}
```

Per-modality semantics:

| Modality | Scrim | Focus capture | Dismissable | Default position | Governance interruption-priority |
|-|-|-|-|-|-|
| **modal** | yes | yes | via affordance | centre | immediate |
| **floating** | no | no | always (close-x) | last position | no interrupt |
| **tool** | no | no | pinnable | docked-right default | no interrupt |
| **toast** | no | no | auto (ttl-ms) | top-right | ambient |

No new primitive IRI. No wire break. Existing surfaces that
declared `modal-state` without `modality` are rejected as malformed
under ADR-006 rules (which require the state to be typed per
primitive-type); since WSP-0.1 is Proposed and nothing consumes
legacy payloads yet, there is no migration.

## Consequences

### Positive
- Four semantically distinct floating-surface patterns are
  expressible as one primitive with one field. Ontology-addressable
  agents can query `primitive.state.modality` to distinguish
  behaviour without widening the canon.
- Governance gates on interruption (ADR-008) get a natural hook:
  `modal` requires explicit permit; `toast` uses ambient quota;
  `tool` and `floating` are free.
- Renderers implement one code path with a `modality` match;
  renderer-agnosticism is preserved.
- Resolves the `window_options` / `extra_viewport` /
  `DemoWindows`-floating-pane demos without adding primitives.

### Negative
- The four modalities share a primitive IRI but behave differently.
  Agents that match on IRI only will misroute. Mitigation: the
  `modality` field is part of `state`, not the IRI — ADR-006
  already requires state-type parsing.
- `toast` and `modal` share a CDDL shape but diverge on
  `ttl-ms` presence/absence. Schema validation must be conditional
  on `modality`, not structural only.

### Neutral
- ADR-006 §7 (tooltip) specifies the renderer compose a
  `Modal(modality="tool")` for tooltip display; this is the
  canonical wire representation of tooltips at render time.

## Alternatives considered

1. **Four separate primitives: `ui://dialog`, `ui://window`,
   `ui://panel`, `ui://toast`** — rejected: quadruples the
   floating-surface section of the canon for what is semantically
   one primitive with a behavioural switch. Violates the canon's
   minimalism and gives agents four IRIs to learn where one would
   do.
2. **Keep `ui://modal` as "modal only", add `ui://window` as a
   sibling** — rejected: still two primitives for "floating
   surface with content", and the toast/tool distinction is left
   without a home. Half-measure.
3. **Encode modality in the IRI (`ui://modal/floating`)** —
   rejected: IRI stems are stable-or-break per ADR-003 /
   `protocol-spec.md §14`, so IRI-encoded modality is effectively
   four primitives under a shared prefix. No better than
   alternative 1.

## Related

- Session sources: `session-5-renderer-contracts.md` (rec. 12 ­—
  Modal + tooltip + extra-viewport deferred to ADR), Cooper
  *About Face* ch. 9 (posture taxonomy).
- Foundation elements: predicate 2 (self-describing — `modality`
  is a state field the agent reads before acting), posture
  taxonomy.
- ADRs: ADR-001 (canon, amended to reference this modality),
  ADR-006 (primitive-head — tooltip renders as
  `Modal(modality="tool")`), ADR-008 (goal governance —
  interruption priority per modality), ADR-009 (Mission Console —
  seat/witness still apply to every modality).
