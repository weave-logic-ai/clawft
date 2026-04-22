# ADR-006: Primitive Head — Six Mandatory Fields at the Kernel Boundary

**Date**: 2026-04-18 (initial), 2026-04-19 (amended)
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

**Amendment note (2026-04-19)**: The demo-lab crosswalk surfaced two
affordance-level patterns that are ubiquitous in egui but not new
primitives: tooltips (hover info on nearly every widget) and drag-
and-drop reordering (Stack children, Table rows). Rather than grow
the canon, these are added as optional head fields — `tooltip` as a
first-class optional, `reorderable` as a distinguished affordance
flag. No wire break; renderers ignore unknown optionals per
`protocol-spec.md §14`.

## Context

Foundations sets three first-class readable properties on every
primitive: state, affordances, confidence. Session 5 (renderer
contracts) demonstrated that `variant-id` and `mutation-axes` must
ride on the head too — without them the active-radar loop has no
attribution anchor and GEPA cannot know which axes are safe to
vary. Session 8 (governance) and foundations' non-negotiable privacy
constraints require `privacy-flags` on every primitive, not optional
metadata: a surface that captures must declare so structurally so
that the kernel can refuse malformed frames before they reach the
renderer (Session 6 §9). Session 2's gap analysis confirmed that
every competitor in the industry has at least one of these fields
absent, which is why no competitor achieves agent-peer composition.
If we let any of the six slip to "optional", we inherit the same
hole.

## Decision

Every WSP frame carrying a primitive (Tier-A or Tier-B shell)
**must** include the following six fields on the primitive head.
A frame missing any of them is *malformed* at the kernel boundary
and is rejected before reaching the renderer — this is a
type-system fact, not a policy.

```cddl
primitive-head = {
  type        : ontology-uri,               ; (1) identity / IRI
  path        : tstr,                       ;     JSON-Pointer within surface
  state       : any,                        ; (2) typed per primitive-type
  affordances : [* affordance],             ; (3) what caller may do now
  confidence  : confidence,                 ; (4) predicate 2
  variant     : variant-id,                 ; (5) active-radar attribution
  mutation-axes : [* mutation-axis],        ; (6) legal GEPA axes (empty list = none legal)
  privacy     : privacy-flags,              ; (7) mandatory; capture/retention/off-node
  ? tooltip   : i18n-string,                ; (8) optional hover-help (amended 2026-04-19)
}

affordance = {
  name        : tstr,
  verb        : tstr,                       ; WSP verb to invoke (ADR-005)
  ? actors    : [+ actor-kind],             ; "human" | "agent" | "any" — default any
  ? args-schema : ontology-uri,
  ? reorderable : bool,                     ; (amended 2026-04-19) child-reorder permitted
}
```

Field rules, all enforced at the kernel boundary:

**1. `state`** — typed per primitive-type (`ui://field.value`,
`ui://gauge.value`, etc.). Renderer reads state, never infers it.
Dirty flag and validation errors live inside state where the
primitive-type schema demands them.

**2. `affordances`** — a non-empty list for any primitive the user
or agent may act upon. Each affordance is `{name, verb, actors?,
args-schema?}` and is the result of `raw_affordances ∩
governance.permit(caller, primitive, resource)` (Session 9 §Layer-2
missing gap). Empty list means "read-only right now"; absent
`affordances` field means malformed.

**3. `confidence`** — `{source, value?, interval?, as-of?}` where
`source ∈ {deterministic, inference, cache, input}`. A value
produced by inference without confidence is malformed (Cedar-OS
failure mode, Session 9 negative lesson 2).

**4. `variant-id`** — opaque string, composer-assigned at
`surface.compose` time and stamped on every `surface.update` that
touches rendered state. The renderer echoes variant on every return
observation so ECC can attribute echoes without shared state
(ADR-007).

**5. `mutation-axes`** — declares which axes GEPA may mutate.
Empty list means no mutation is legal on this primitive. Each axis
carries a `frozen-by` metadata list (`"brand" | "safety" | "consent"
| "user-request"`) so governance denials can cite a reason.
Consent flows, `Modal.affordances`, and safety affordances are
never legal to mutate (foundations §"active-radar loop" invariant).

**6. `privacy-flags`** — `{captured, retained, off-node, consent-id}`,
always populated. A `foreign://{camera,mic,screen}` surface with a
non-empty `capture-channels` list and an absent or expired
`consent-id` is a malformed frame that the kernel rejects — the
renderer never sees it. This is the structural precondition for
foundations non-negotiable 1–4 (per-user, governance-gated,
signal-lossy, observable) and is reinforced by ADR-012.

**7. `tooltip` (optional, amended 2026-04-19)** — `i18n-string`.
Hover / long-press / accessibility help-text. Rendered as a
transient `Modal(modality="tool")` in the renderer; routed to
AT-SPI-like descriptions for screen readers; spoken on long-press
in voice mode. Absent field = no tooltip.

**8. `reorderable` (optional, per-affordance, amended 2026-04-19)** —
container primitives (`Stack`, `Grid`, `Table`, `Tree`) that permit
drag-to-reorder children declare it on the `reorder` affordance.
The renderer implements drag-and-drop only when this flag is
present; governance can refuse to add the affordance without
refusing the whole primitive. This closes the egui-demo drag-and-
drop gap without a new primitive.

## Consequences

### Positive
- A typed kernel reject at frame boundary means no silent privacy
  degradation class of bug can exist. Frames without `privacy-flags`
  or with capture + expired consent are refused before render.
- Agents enumerate `affordances` honestly (predicate 2) — the
  listed set is exactly what the governance gate will allow; no
  "attempt and hope" pattern.
- ECC + GEPA attribution is always reconstructable: the six fields
  on the head carry everything needed to map a return observation
  to the pulse that produced it.
- Cedar-OS's missing-confidence gap (Session 9 negative lesson 2)
  cannot recur: a primitive without confidence does not pass the
  kernel.
- Conformance tests become structural: parse-then-validate is all
  a renderer needs, not runtime heuristics.

### Negative
- Six required fields is a larger head than minimal alternatives;
  JSON payloads grow by roughly a hundred bytes per primitive even
  without state.
- Composers must emit `variant-id` even for surfaces they never
  intend to mutate; we accept the small cost to avoid a special
  case.
- `confidence` for deterministic primitives (a `Stack` layout, a
  button with `source="deterministic"`) feels redundant but is
  required for uniform parsing.

### Neutral
- The head is deliberately orthogonal to the primitive-type schema:
  adding a new primitive IRI (ADR-002 promotion path) does not
  touch the head, only the `state` shape.

## Alternatives considered

1. **Optional `privacy-flags` (default to "no capture")** —
   rejected: silent defaulting is the Adtech SDK failure mode.
   Foundations non-negotiable 4 ("observable to the user") requires
   structural presence so a surface cannot capture without saying
   so on the wire.
2. **`variant-id` optional on non-GEPA-legible primitives** —
   rejected: every return observation must be attributable so ECC
   can compute coherence-delta without reconstructing ordering.
   Non-mutable primitives emit a stable variant; the cost is one
   string field.
3. **`mutation-axes` stored out-of-band in ontology** — rejected:
   Session 6 §5 invariant 2 — declaring mutation axes in-band means
   governance can reject mutations against undeclared axes without
   an out-of-band schema lookup on every `mutate` call. Latency
   and correctness both improve.
4. **Five fields, fold `confidence` into `state`** — rejected: the
   agent reads confidence independently of a primitive-type-specific
   state parser; folding them couples two concerns and breaks
   uniform introspection.
5. **Seven fields, add `provenance` as first-class** — considered;
   rejected for now. `confidence.source` carries the minimum
   provenance signal; chain-anchored provenance lives in governance
   events (ADR-008 `governance.goal.*`). Promotion to a head field
   is an ADR if future tracks prove it is load-bearing.

## Related

- Sessions: `session-6-protocol-design.md` (§5 CDDL, §8 variant +
  mutation, §9 governance + consent, §11 rec. 5),
  `session-5-renderer-contracts.md` (rec. 7 variant + mutation
  axes, rec. 8 CanonResponse fields), `session-9-agentic-os-canon.md`
  (rec. 2 first-class head fields).
- Foundation elements: predicate 2 (self-describing), predicate 3
  (streaming-native), §"Active-radar loop", §"Non-negotiable
  privacy constraints" 1–4.
- ADRs: ADR-001 (canon), ADR-002 (tier split), ADR-004 (schema
  shape), ADR-005 (verb set), ADR-007 (return schema), ADR-012
  (capture privacy).
