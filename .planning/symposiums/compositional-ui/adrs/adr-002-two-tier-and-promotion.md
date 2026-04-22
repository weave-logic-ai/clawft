# ADR-002: Two-Tier Model and ForeignSurface Promotion Path

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

`foundations.md` commits to a hard two-tier split: Tier-A primitives
satisfy all four predicates; Tier-B is a single wrapper
(`ForeignSurface`) whose *shell* is Tier-A and whose *interior* is
opaque to primitive composition but participates as a typed signal
tapestry (presence / identity / activity / outcome). There is no
Tier-C "halfway primitive" admitted under any circumstance
(foundations §"Anti-rule"). Session 1 located the same split across
history (X11 pixmap, RDP semantic remoting) and Session 5 codified it
in the eighteen-primitive canon. Session 9's framing paper recommends
this remain non-negotiable (rec. 4). What has not yet been written
down is *how* a wrapper earns promotion to a native Tier-A primitive
when its pattern has been proved, and *what* canonical wrapper types
ship from day one.

## Decision

Enforce the two-tier rule as canon with six canonical Tier-B wrapper
types and a four-stage promotion gate.

**Tier-A rule.** A primitive enters the canon only if it satisfies all
four predicates: typed IRI identity (`ui://…`), first-class
`{state, affordances, confidence}` read-off, `{open|chunk|commit|cancel}`
stream verbs, and peer human/agent operation. A proposed primitive
that satisfies three is rejected, not adopted.

**Tier-B rule.** The one primitive admitted with opaque interior is
`ui://foreign`. Its shell is Tier-A (all four predicates satisfied on
the wrapper itself; `wrapper-type` IRI under `foreign://…`). Its
interior is opaque to composition but emits presence / identity /
activity / outcome signals that the substrate ingests. A wrapper
that refuses to emit these signals is non-conformant.

**Six canonical wrapper types ship from day one**:

1. `foreign://browser` — Chromium/WebView pane; affordances
   `navigate`, `reload`, `screenshot`, `read-a11y-tree`,
   `execute-script` (gated), `export-text`.
2. `foreign://terminal` — PTY host; affordances `write`,
   `screenshot-buffer`, `tail-output`, `resize`, `kill`.
3. `foreign://video` — media player; affordances `play`, `pause`,
   `seek`, `set-track`, `subtitles`.
4. `foreign://xwindow` — embedded X11/Wayland window; affordances
   `focus`, `resize`, `screenshot`, `terminate`.
5. `foreign://app` — nested WeftOS app (another egui/Tauri process);
   affordances `launch`, `focus`, `terminate`, `send-message`.
6. `foreign://{camera,mic,screen}` — ambient capture (privacy
   non-negotiable, see ADR-012).

**Promotion gate** — four mandatory stages before `foreign://X` becomes
a native `ui://x` Tier-A primitive:

1. **Usage evidence** — six months of non-trivial use logged on the
   observation stream across ≥2 distinct surfaces/projects.
2. **Predicate-conformance proof** — demonstrable path to all four
   predicates: a typed IRI, state/affordance/confidence schema, stream
   verbs implemented, agent/human peer operation.
3. **ADR** — written proposal that cites evidence + predicate proof +
   impact on canon size. Round-table review in the symposium.
4. **Canon amendment** — ADR accepted, canon table in ADR-001
   amended, `foreign://X` retired to a deprecated alias that surfaces
   the new `ui://x` to legacy renderers.

Wrapping is the default; native is the reward for proving the
pattern is worth owning. No "halfway" state exists between the two.

## Consequences

### Positive
- Agents can rely on "all Tier-A primitives satisfy all four
  predicates" without exception — no ontology-addressable-but-not-
  streaming holes to discover at runtime.
- Opaque content (legacy apps, browsers, video, capture streams) is
  admitted honestly as `ForeignSurface`, not forced into a half-
  compliant Tier-A slot that would silently violate predicates.
- The promotion gate prevents the canon from sprawling; a new
  primitive carries the cost of proving itself.
- The six wrapper types cover the realistic near-term corpus (dev
  panel browser/terminal/video, ambient capture, nested apps) with
  known-good egui implementations (wry, egui_term, GStreamer).

### Negative
- A primitive with a genuinely rich interior (e.g. an embedded
  notebook) cannot earn partial-compliance credit; it lives as
  `ForeignSurface` until it passes the full gate.
- Promotion takes time — six months of usage is slow for a pre-1.0
  system; we accept the latency to preserve canon integrity.
- Signal-emission obligations on wrappers are non-trivial to
  implement per kind; we cannot ship a half-wrapper.

### Neutral
- The promotion ADR becomes a forcing function to write down what
  changed about a wrapper over time — a fringe benefit for audit.

## Alternatives considered

1. **Admit Tier-C "three-of-four" primitives** — rejected per
   foundations §"Anti-rule"; admitting Tier-C is what made AT-SPI
   (Session 1) and Cedar-OS (Session 9) unable to serve agents as
   peers. Once the split is blurred, predicate 1 becomes per-widget
   opt-in, and the floor of the canon collapses.
2. **No wrappers — ban opaque content** — rejected: browsers, video,
   and capture are the realistic substrates users live in. Banning
   them pushes the user outside the ontology entirely and destroys
   the tapestry signal channel (foundations §"Tier B as woven
   signal tapestry").
3. **Promote on first use** — rejected: too permissive; every
   one-off integration would earn a native primitive, and the
   canon would balloon past twenty within a quarter.
4. **Per-wrapper ad-hoc promotion rules** — rejected: opens the
   door to advocacy pressure per project. A uniform four-stage gate
   is the only defence against "this wrapper is important enough to
   skip the rules".

## Related

- Sessions: `session-1-historical-canon.md` (X11 pixmap, RDP
  semantic remoting, rec. 9), `session-5-renderer-contracts.md`
  (ForeignSurface per-kind implementation), `session-9-agentic-os-canon.md`
  (rec. 4 — Tier B single-primitive).
- Foundation elements: four predicates, §"Two tiers",
  §"Anti-rule" (no Tier-C), canonical wrapper-types list.
- ADRs: ADR-001 (canon), ADR-006 (primitive head), ADR-012
  (capture privacy invariants).
