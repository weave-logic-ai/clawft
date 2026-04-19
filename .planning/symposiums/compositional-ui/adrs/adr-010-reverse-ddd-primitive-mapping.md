# ADR-010: Reverse-DDD Primitive Mapping

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Foundations commits to the reverse-DDD arrow: classical DDD runs
`domain → model → code → UI`; WeftOS runs `substrate state +
ontology → schema → composition → renderer`. The flip happens at the
"code → UI" step. Session 4 (RQ4) worked through the arrow in
practice, reading Evans ch. 6 (aggregates), ch. 14 (bounded contexts
and context maps), and the classical tactical patterns (entity,
value object, service, repository, invariant, anti-corruption).
Naked Objects (Pawson 2004) is the closest prior art — domain
objects drive the UI directly via reflection — but its one-rule-
per-construct made every screen look the same and it had no
streaming, no confidence, no variant-id. Session 5 (renderer
contracts) confirmed the 18-primitive canon lines up with Evans's
tactical constructs when we let ontology geometry choose between
container primitives. Session 9 rec. 1 (`ResourceId` addressing)
and rec. 9 (goals as resources) lock the arrow into a concrete
substrate. Without a canonical mapping table, every downstream
agent-author re-derives it and drift begins.

## Decision

Adopt the following canonical mapping from Evans tactical DDD to
the 18-primitive canon. This is the reverse-DDD arrow made
executable; it is how an agent walking an ontology node decides
which primitive to compose.

| DDD construct          | Primitive                                       | Predicate anchor |
|------------------------|-------------------------------------------------|------------------|
| Aggregate root         | `ui://surface` (focal drill target, addressable)| P1 addressability |
| Entity (non-root)      | `ui://chip` (draggable, linkable) or `ui://card` composition | P1 + P4 |
| Value object           | `ui://field` (inline, formatted)                | P2 + P3 (dirty / validation) |
| Collection             | `ui://list` / `ui://table` / `ui://tree` / `ui://plot` (chosen by ontology geometry) | P1 + P3 |
| Action / method        | `ui://pressable` (button, menu, voice intent)   | P4 (voice) |
| Invariant              | `AffordanceGate` (greys/hides verbs on primitive head) | P2 |
| Repository             | `subscribe` verb bound to a typed resource IRI  | P3 |
| Domain service         | `ui://pressable` with no `this` (toolbar / command-palette) | P4 |
| Domain event           | `observe` / substrate update notifications      | P3 |
| Bounded context        | IRI namespace + governance `ContextFrame`        | P1 + governance |
| Context map            | `ui://context-map` Tier-A primitive             | P1 + Tier A |
| Anti-corruption layer  | IRI translation at composition boundary (Session 4 §5 + ADR-009) | P1 |

Rules that govern the mapping:

**Aggregate root → `Surface`.** The focal primitive a composition
zooms into on drill-down (Session 4 §"reverse-DDD arrow" practical
consequence 2). The aggregate's invariants become `AffordanceGate`
rules on the surface's primitive head (ADR-006).

**Entity vs value object → identity vs inline.** Entities have an
IRI and get an addressable primitive (`Chip` / `Card`,
draggable/linkable/subscribable). Value objects have no IRI and
render inline in their owning entity's surface (Pawson's
formatted-text rule; Session 4 practical consequence 3).

**Collection → geometry-chosen container.** The ontology declares
geometry (per `ontology-navigator`); geometry selects the container
primitive. `force | network` → `ui://plot` or bespoke; `tree` →
`ui://tree`; `timeline` → `ui://plot` with temporal axis; `grid` →
`ui://table`; `layered` → `ui://stack + sheet`. Collections never
pick themselves from type alone — always via geometry (Session 4
rec. 3).

**Action / service → `Pressable`.** Actions with a `this` are
methods hoisted to the primitive's affordance list. Domain services
(no `this`) become toolbar/command-palette `Pressable`s at the
surface level. Voice is an equivalent channel: the same action is
invokable by utterance matching a declared voice phrase on the
`Pressable` (ADR-011 voice modality).

**Invariant → `AffordanceGate`.** An invariant that would forbid
a verb in a state simply removes it from `affordances` on the
primitive head (or marks it disabled with a deny reason). Pawson's
`aboutXxx` predicate made executable.

**Bounded context → IRI namespace + governance.** Three-layer
enforcement (Session 4 §"bounded-context enforcement"): substrate
IRIs namespaced by context; composition protocol rejects cross-
namespace bindings without declared bridges; governance audits and
refuses violations. No primitive renders without a declared
`ContextFrame`.

**Anti-corruption → IRI translation.** When a substrate emits terms
from a foreign ontology (scraping wrapper, external pod), the
wrapper's schema declares the translation to the canonical
namespace. No term enters the agent-composable layer untranslated
(Session 4 §5).

**SKOS relation semantics as defaults** (Session 4 rec. 8). Tree
primitives bind to `broader / narrower`; tag strips to `altLabel`;
related lines to `related`; grouping to `member`. These are the
outliner-idiom defaults when no geometry is declared.

**Primitives are content-addressed** (Session 4 rec. 10,
Scrapscript). The primitive bytes themselves live in RVF,
referenced by CID in the composition tree. Enables immutable GEPA
variants, reproducible ECC returns, hash-addressed sharing across
nodes. The primitive-head `type` is the class IRI; `instance_iri`
names the concrete binding; `cid` is the content address of the
rendered bytes (Session 4 §"ontology addressability in practice").

## Consequences

### Positive
- Agents walk the ontology and choose primitives by table, not by
  heuristic. New ontology nodes become composable as soon as their
  geometry + cardinality + datatype are declared.
- Evans's tactical patterns survive intact inside the substrate;
  only the "code → UI" step flips. DDD-literate developers read
  the system.
- Context maps and anti-corruption layers (ch. 14) become visible
  user surfaces, not whiteboard artefacts. Mission Console's
  cross-context case (ADR-009) has structure at hand.
- SKOS defaults mean the outliner / tagging idiom works without any
  geometry declaration for small ontologies — Roam/Logseq
  legibility for free.
- Content-addressed primitives give GEPA reproducible variants and
  ECC reproducible return signals; variants can be snapshotted
  cross-substrate.

### Negative
- Authors must think in aggregates. A codebase mentality of
  "everything is a screen" does not transfer; new contributors
  need onboarding on the arrow.
- Geometry declaration is sometimes absent in fast-moving
  ontologies; the SKOS default is serviceable but not always
  best-fit.
- Content-addressed primitives impose RVF serialisation on every
  rendered node; small perf cost on hot loops.

### Neutral
- The mapping is opinionated but extensible — a domain may declare
  a specialised primitive IRI that overrides the default (with an
  ADR per the ADR-002 promotion gate).

## Alternatives considered

1. **Pawson's one-rule-per-construct (strict Naked Objects)** —
   rejected per Session 1 §5 + Session 4 §Naked Objects: produces
   visually flat, bureaucratic-feeling screens (Cooper posture
   failure). Geometry consultation restores variety without
   breaking the arrow.
2. **No explicit mapping — let agents infer** — rejected: every
   agent reinvents the wheel, drift accumulates, audits become
   guesswork. The table is canon precisely to prevent this.
3. **Naked-Objects-plus-geometry but no content addressing** —
   considered and rejected per Session 4 rec. 10: Scrapscript /
   TreeBase's content addressing is what lets GEPA variants be
   first-class; skipping it forecloses the active-radar loop's
   reproducibility.
4. **Entities as `Surface` (flat — no root/non-root split)** —
   rejected: aggregate-root vs non-root entity is a real Evans
   distinction with consistency-boundary consequences. Collapsing
   them breaks the root-is-focal-drill-target rule.
5. **Anti-corruption as runtime convention** — rejected per Evans
   ch. 14: without a declared translation, contexts corrupt each
   other's models silently. Ours must be a declared IRI translation
   that governance can audit.

## Related

- Sessions: `session-4-ontology-ui.md` (entire doc — especially
  §"reverse-DDD arrow" and §"Aggregate roots / entities / value
  objects → primitive hierarchy" table), `session-5-renderer-contracts.md`
  (18-primitive canon aligned with the table),
  `session-9-agentic-os-canon.md` (rec. 1 `ResourceId` addressing,
  rec. 9 goals as resources), `session-1-historical-canon.md`
  (Naked Objects teardown, rec. 7 entity-drives-surface with
  posture hint).
- Foundation elements: §"Reverse-DDD arrow", §"Rasmus four-layer
  crosswalk", predicate 1, predicate 2.
- ADRs: ADR-001 (canon), ADR-006 (primitive head carries IRI +
  variant), ADR-008 (goals as aggregate roots in governance),
  ADR-009 (Mission Console context maps).
