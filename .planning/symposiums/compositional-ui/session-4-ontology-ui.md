# Session 4 — Ontology → UI

**RQ4 — Data geometry → rendering strategy, no human designer in the loop.**
Reads Evans with the DDD arrow reversed. Sibling:
`../ontology-navigator/synthesis.md` (load-bearing).

---

## Scope

How the ontology *composes* a surface — not visualises the graph,
*is* the graph made interactive. Prior art in five families:

1. **Naked Objects** (Pawson 2004) — closest prior art; UI reflected
   off domain objects; no separate presentation layer.
2. **Linked Data / RDF / LDP / LDF / Solid** — web-scale semantic
   data; apps decoupled from storage via shared vocabularies/shapes.
3. **Protégé / OWL / SKOS / VOWL** — meta-UIs rendering ontologies
   themselves; tool isomorphic to the model it authors.
4. **Notion / Roam / Obsidian / Logseq** — schema-lite commercial
   systems; interface co-determined by the data the user entered.
5. **TreeBase / Scrapscript** — content-addressed data-first
   environments; the artefact's structure is its interface.

RQ4 is not "should we" — `foundations.md` already committed predicate
1. RQ4 is *how*, *how far*, *where it breaks*.

---

## Per-system teardowns

### Naked Objects (Pawson, Trinity College Dublin 2004)

Three design rules, in Pawson's own ordering:

1. All business logic is encapsulated on the domain object
   (*behavioural completeness*).
2. The UI is a **direct representation** of the domain object —
   every user action creates/retrieves an object or invokes a method
   on one.
3. The UI is **entirely auto-generated** from the domain object
   definitions (reflection at runtime or source-code generation).

Rendering rules, derived from the reference framework:

| Domain construct     | Widget                                                                 |
|----------------------|------------------------------------------------------------------------|
| Scalar property      | Formatted, in-place editable text (value-object formatter per type)    |
| Object reference     | Icon with drag-and-drop target; type-gated drop; auto dropdown of live instances |
| Collection           | List of icons; grey blob affordance to add; remove via per-item popup  |
| Action (method)      | Menu item under the object's icon; prefix stripped; signature → dialog |
| Void action          | Mutates state in place                                                 |
| Object-returning     | Opens a new window with that object as focus                           |
| `aboutXxx()` methods | Availability/permissions predicate — greys the action out conditionally |

Aggregate root → default focus window (Pawson calls it "the object"),
with its collections and references laid out in compartments, actions
hoisted to a pop-up menu. Value objects → in-line formatted text.
Entities-with-identity → draggable icons. **There is no "screen" in
Naked Objects. There are objects.**

Limitations: single-user, procedural assumption; weak for wizard
flows, dashboards, streaming; no confidence or partial-value model;
enterprise adoption stalled because UX only cohered for
operational/back-office software. Correct but undersupplied — one
posture (sovereign, data-entry), one tier of primitives.

**Take**: the reflection → widget rule per construct; the `aboutXxx`
affordance gate (our predicate-2 "affordances"). **Reject**: one
rule per construct. Our ontology must declare *geometry* as well as
type (per ontology-navigator Session 2).

### Linked Data / RDF / LDP / LDF / Solid

The web-scale version of "data as UI". Four principles (Berners-Lee
2006) — URIs name, HTTP dereferences, RDF structures, links connect.
A generic application can render an unknown resource by fetching it,
reading `rdf:type`, `rdfs:label`, and traversing `owl:sameAs`.

Solid extends this with:

- **WebID** — an HTTP URI that dereferences to an agent profile.
- **Pod** — an LDP container hierarchy owned by a WebID.
- **Type Index** — a registry mapping `rdf:type` values to pod
  locations (`solid:instance`, `solid:instanceContainer`). An
  application doesn't ask "where are the bookmarks" — it asks the
  Type Index "where is anything of type `bookm:Bookmark`".
- **ShEx / SHACL shapes** — the schema a generic renderer reads to
  know which properties matter, their cardinality, their datatype.

The move that matters for us: **the data pod and the UI app are
separate deployments**. Discovery is runtime, via the ontology. Any
app that speaks the shape can render the data. This is predicate 1
(*ontology-addressable*) taken to its web-scale limit.

Solid punts: no streaming, no confidence, no dual-operator path.
SHACL validates but carries no affordances. Every Solid app
reinvents its widget mapping.

### Protégé / OWL / SKOS (meta-UIs)

Protégé is an ontology editor whose UI *is* an ontology renderer.
The main views:

- **Class hierarchy tree** (left) — `rdfs:subClassOf` rendered as
  tree containment.
- **Class description form** (centre) — for the selected class:
  superclasses, equivalent classes, disjoint classes, annotations,
  restrictions. Each panel is a list with add/remove affordances.
- **Property tabs** — object/datatype/annotation properties, each
  rendered the same way with domain/range pickers.
- **Individuals tab** — instance forms with property assertions.
- **Manchester Syntax editor** — nested expressions as a single row.

The rule Protégé enforces: **every construct in the meta-model gets a
generic widget**, and the whole UI is built from ~8 widget types:
tree, list, form, expression-row, picker, hierarchy-view, tabbed
container, and detail pane. The UI scales to any ontology because
the widget set addresses the meta-model, not the domain.

SKOS is the important simplification. Its primitives — `Concept`,
`ConceptScheme`, `prefLabel`, `altLabel`, `broader`, `narrower`,
`related`, `member` — map one-to-one to obvious widgets: a tree
(broader/narrower), a tag strip (labels), a relation line (related),
a grouping box (member). SKOS is so narrow that the rendering rule
is a lookup, not a strategy.

**Take**: the meta-UI pattern. If our ontology has ~10 predicates
(see recommendations), the full surface vocabulary can be covered by
~20 primitives addressing those predicates.

### Notion / Roam / Obsidian / Logseq

Notion is the closest commercial implementation of schema-driven
composition. A database has *properties* (typed columns); *views*
(table, board, calendar, timeline, gallery, list) consume the same
schema and render accordingly. The user picks the view, but *only
views whose required property type is present in the schema are
available* — a calendar view requires a date property; a board view
requires a select property. This is a schema→view selection rule,
just implemented by a human designer (you) on behalf of the data.

Roam and Logseq expose a lower-level substrate: everything is a
block, blocks link, attributes (`key:: value`) act as typed
predicates, and the entire store is queryable via Datalog. What Roam
does *not* formalise — cardinality, types, domain/range — is exactly
what SHACL/ShEx add to Solid and what our TopologySchema adds to
WeftOS. Obsidian sits even further back: wikilinks and frontmatter
are untyped; Canvas is manual layout. Obsidian is the negative
example — a graph without an ontology degenerates to a hairball
(ontology-navigator Session 1 documented this).

**Take from Notion**: schema → candidate-view set is a decidable
function; let the ontology+geometry choose it.
**Take from Roam/Logseq**: Datalog over a typed substrate is the
right query idiom (we already use Datomic-shaped thinking in ECC).
**Reject from Obsidian**: untyped linking.

### TreeBase / Scrapscript

TreeBase (Breck Yunits, PLDB) uses a grammar-defined tree language;
the tree *is* the database and *is* the source. Every node has a
type (from the grammar), every type has a rendering rule.

Scrapscript (tekknolagi) content-addresses every expression. Code
and data share a single addressable substrate (scraps). Any scrap
carries its own type. Composition is by hash reference; versions are
immutable.

The relevance: **content-addressing turns primitives into first-class
citizens of the knowledge graph**. A WeftOS primitive (`WeftButton`,
`WeftChip`) is itself an entity with an IRI, a shape, and a content
address (RVF CID). A composed surface is a tree of CIDs. This is the
substrate-level move that lets predicate 1 extend to the primitives
themselves — not just to the data they render.

---

## The reverse-DDD arrow — practical consequences

Classical DDD: `domain → model → code → UI`. The UI is the
*terminal* step, authored by a designer who reads the model and
produces screens.

Reversed: `substrate + ontology → schema → composition → renderer`.
The UI is a *projection*, computed at runtime.

What actually flips in practice, read against Evans:

1. **Ubiquitous language becomes executable.** In Evans the language
   is a social artefact the team maintains. Here it is the ontology
   IRIs (see `../ontology-navigator/adr-identity-iri.md`). The IRI
   *is* the term; the primitive bound to it *is* its executable
   form. A term you can't render is a term that isn't in the
   language.

2. **Aggregate roots become focal surfaces.** Evans's aggregate is a
   consistency boundary rooted on one entity. In the reversed
   arrow, the aggregate root becomes the *Focal* — the primitive a
   composition zooms into when it drills (ontology-navigator
   Session 5, "drill-down"). Non-root entities become children in
   the compound node; value objects become leaf fields; the
   aggregate's invariants become `about*`-style affordance gates.

3. **Entities vs value objects becomes identity-vs-inline.** An
   entity has an IRI and gets an addressable primitive (draggable,
   linkable, subscribable). A value object has no IRI and renders
   inline in its owning entity's surface (Pawson's formatted-text
   rule — not coincidentally).

4. **Bounded contexts become governance gates.** Evans's context
   boundary keeps "Customer" in sales from leaking into "Customer"
   in support. In the reversed arrow the boundary is enforced by
   the *governance layer* against the composition protocol: an
   agent composing in context A queries only IRIs under context A's
   namespace, and the renderer *refuses* to bind a primitive to an
   IRI from context B unless a context map (below) declares the
   bridge. The "Service" problem from `adr-identity-iri.md` is
   exactly this; IRIs solve it at the substrate level, the
   composition protocol enforces it at the surface.

5. **Context maps become first-class surface primitives.** A
   Mission Console spanning contexts (the `foundations.md` example:
   regional-manager across many stores) needs an explicit context
   map primitive — declaring which mappings are in play, which
   terms are aliased, which are translated, which are blocked.
   Evans drew context maps on whiteboards; we render them.

6. **Anti-corruption layers become IRI translation at the
   composition boundary.** When a substrate emits terms from a
   foreign ontology (e.g. a foreign surface scraping a web app),
   the wrapper's schema declares the translation to the canon
   namespace. No term enters the agent-composable layer untranslated.

7. **Strategic DDD moves up the stack.** The strategic layer is no
   longer "what subdomains exist" — it's "what ontologies exist and
   how do they compose". The schema-builder agents
   (`../ontology-navigator/session-schema-agents-findings.md`) are
   the tactical instrument of this strategic layer.

**The arrow is not reversed everywhere.** Evans's tactical patterns
(aggregates, factories, repositories, services) still apply inside
the substrate. The flip is only at the "code → UI" step. The UI
stops being the output of modelling and becomes a live projection of
modelling done once.

---

## Answers to the central questions

**(1) How does data geometry select the rendering strategy?**

The ontology-navigator symposium already resolved this
(`synthesis.md`, Session 3). Geometry is declared per node type in
the TopologySchema (`force | tree | layered | timeline | stream |
grid | geo | radial | wardley`). When absent, the Session 3
auto-detection heuristic runs (timestamp ratio → timeline, cyclic →
force, in-degree ≤ 1 → tree, etc.). The layout engine reads geometry
+ density + type distribution and dispatches to an algorithm
(Reingold-Tilford for tree, Sugiyama for DAG, ForceAtlas2 for
network, timeline-on-scale for temporal). Geometry drives layout,
then layout drives primitive choice (e.g. stream geometry → spark-
line primitive; grid geometry → table primitive).

**(2) Minimum schema metadata for primitive auto-configuration?**

Synthesising Naked Objects, Notion, SHACL, and our TopologySchema:

- `iri` — concept identity
- `cardinality` — 0..1, 1, 0..n, 1..n
- `datatype` — scalar kind (text, int, float, bool, date, duration,
  enum, reference)
- `geometry` — layout hint (per ontology-navigator)
- `contains` — child type(s) for drill-down
- `label_field` / `time_field` / `lat_field` / `value_field` —
  semantic role hints
- `affordances` — declared operations (read, write, subscribe,
  invoke, dismiss)
- `confidence_field` — pointer to the provenance tuple
- `sensitivity` — governance class (public, tenant, private, secret)

Nine fields, all optional except `iri`. A primitive binds by reading
these; anything missing falls through to a safe default.

**(3) Aggregate roots / entities / value objects → primitive
hierarchy?**

Direct mapping, Evans-reversed:

| DDD construct            | Primitive                                    |
|--------------------------|----------------------------------------------|
| Aggregate root           | Focal `Surface` (drill target, addressable)  |
| Entity (non-root)        | `Chip` (draggable, linkable) or `Card`       |
| Value object             | `Field` (inline, formatted)                  |
| Collection               | `List`, `Table`, `Gallery`, or `Timeline` (chosen by geometry) |
| Action/method            | `Verb` (button, menu item, or voice intent)  |
| Invariant                | `AffordanceGate` (greys/hides verbs)         |
| Repository               | `Query` primitive (binds to subscription)    |
| Domain service           | `Verb` with no `this` (toolbar/command-palette action) |
| Event                    | `StreamEvent` (feeds subscriptions)          |
| Bounded context          | `ContextFrame` (scopes the composition)      |
| Context map              | `ContextMap` primitive (explicit bridge)     |

This is the executable half of Evans.

**(4) Bounded-context enforcement — "no term leakage"?**

Three layers, each non-negotiable:

- **Substrate**: every entity carries an `iri`; IRIs are namespaced
  by context; cross-namespace references are explicit
  (`same_as`/`bridgedBy`).
- **Composition protocol**: the agent declares the `ContextFrame` it
  is composing under; the renderer rejects primitive bindings whose
  IRI is not reachable under that frame's namespace closure.
- **Governance**: the governance layer audits every composition and
  refuses submissions that violate context boundaries (same machine
  that enforces `foundations.md`'s tier-A/tier-B split).

The composition protocol *MUST* carry the active context-frame IRI
in the envelope — a surface with no declared context is rejected.

**(5) What does "ontology addressability" look like in practice?**

A typed tuple, not just a URI:

```
PrimitiveRef = (
    iri:          "https://weftos.weavelogic.ai/ont/ui#Chip",
    instance_iri: "https://acme.example/org/person/42#chip",
    cid:          blake3(primitive_bytes),
    context:      "https://acme.example/ctx/sales",
    shape:        "https://acme.example/shapes/Person",
    variant:      "v3-dense",
)
```

The `iri` is the *class* (the primitive type in the ontology). The
`instance_iri` is the *concrete* binding (this Chip for this
Person). The `cid` is the content address of the rendered bytes
(RVF serialisation, scrapscript-style). The `context` scopes
governance. The `shape` drives auto-configuration. The `variant`
closes the loop for GEPA/active-radar (per `foundations.md`).

An agent `query`s `iri`, `invoke`s against `instance_iri`,
`subscribe`s via `cid`. Humans interact through the rendered
surface; their returns reference the same tuple. Every one of the
four predicates is satisfiable from this tuple alone.

---

## Recommendations for canon

Each recommendation cites predicate (P1/P2/P3/P4) and tier (A/B).
Numbering continues any existing symposium scheme locally; adopt the
global number when ADR-ified.

1. **Adopt Pawson's reflection rule per construct, expanded.** Scalar
   → `Field`; reference → `Chip`; collection → geometry-chosen
   container; action → `Verb`; invariant → `AffordanceGate`. This is
   P1 + P2 + Tier A. *(Reject the one-rule-per-construct limit;
   collections must consult geometry.)*

2. **Every primitive carries a `PrimitiveRef` tuple.** Five fields
   above plus the `variant` for GEPA. Closes P1 + P2 and supplies
   the active-radar return-signal schema
   (`foundations.md` §"active-radar loop").

3. **Ontology declares geometry; geometry selects layout; layout
   narrows primitives.** Do not let primitives pick themselves
   from type alone — always via geometry. Inherits
   `ontology-navigator/synthesis.md` §3. P1 + Tier A.

4. **Minimum schema: nine metadata fields.** `iri`, `cardinality`,
   `datatype`, `geometry`, `contains`, semantic-role hints,
   `affordances`, `confidence_field`, `sensitivity`. All optional
   except `iri`. P1 + P2 + P3.

5. **Aggregate root → `Surface`; entity → `Chip|Card`; value object
   → `Field`.** The DDD→primitive mapping table above is canon.
   P1 + Tier A.

6. **Every composition carries an active `ContextFrame` IRI.** No
   context, no render. Governance enforces namespace closure
   (§"bounded-context enforcement"). P1 + governance.

7. **`ContextMap` is a Tier-A primitive, not a one-off.** Mission
   Console work explicitly requires it. P1 + P2 + Tier A.

8. **SKOS `broader/narrower/related/member` are the default
   relation semantics for hierarchical surfaces.** Tree primitives
   bind to `broader`; tag strips to `altLabel`; the outliner idiom
   from Roam/Logseq becomes our default container when no geometry
   is declared. P1 + Tier A.

9. **Adopt Solid Type Index as the pod-discovery pattern for
   substrate-scoped surfaces.** Agents ask the substrate "where is
   anything of type X" via IRI, not by pathing. Extends
   `adr-identity-iri.md`. P1 + P4.

10. **Primitives are content-addressed** (Scrapscript rule). Their
    bytes live in RVF, referenced by CID in the composition tree.
    Enables immutable variants for GEPA, reproducible returns for
    ECC, and hash-addressed sharing across nodes. P1 + P3.

---

## Open questions

1. **SHACL vs our TopologySchema — pick one or export both?** Q&A
   decision #4 says speak OWL/RDF for interop. Does SHACL replace
   our schema, live alongside it, or get auto-generated from it?
   Leaning toward: TopologySchema is authored; SHACL exported.

2. **How much of the primitive set is declared in the ontology vs
   discovered from runtime?** Naked Objects does it purely at
   runtime (reflection). A content-addressed primitive set lets us
   do it declaratively. Hybrid is likely; where is the cut?

3. **Voice-first in the ontology.** Each `Verb` needs a voice
   surface (P4). Is the voice-intent-IRI a separate predicate on
   the verb, or an affordance? Probably a predicate; needs ADR.

4. **GEPA variant legality.** Per `foundations.md`, some primitives
   are mutation-legible and some aren't. This is a schema
   constraint — where does `mutation_schema` live on the primitive
   IRI? (Speculative: a SHACL shape on the primitive class itself.)

5. **Cross-substrate context maps.** A Mission Console that spans
   two substrates (two tenants' pods) needs a federated
   context-map primitive. Rasmus's unsolved problem 1 (identity
   + delegation across orgs) and ours intersect here.

6. **Notion-style view switching vs agent-chosen layout.** Users
   will want to flip to a different view. How much of the
   candidate-view set is surfaced to the human vs pre-selected by
   the agent? Leans: always show the legal view set as a chip
   strip, agent's pick is the default. ADR-worthy.

7. **When does a term earn an IRI?** Ontology sprawl is the
   predictable failure. The schema-builder agents
   (`ontology-navigator/session-schema-agents-findings.md`) are the
   enforcement lever, but the policy (ratify threshold, decay rule)
   is still unwritten.

---

## Relevant file paths

- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/compositional-ui/foundations.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/compositional-ui/AGENDA.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/ontology-navigator/AGENDA.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/ontology-navigator/synthesis.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/ontology-navigator/adr-identity-iri.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/ontology-navigator/session-2-schema-findings.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/ontology-navigator/session-3-layout-findings.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/ontology-navigator/session-schema-agents-findings.md`

---

## External sources

- Pawson, *Naked Objects* thesis, Trinity College Dublin, 2004
  (Apache Causeway mirror).
- Berners-Lee, *Linked Data Design Issues*, 2006.
- W3C Solid Protocol, Type Indexes spec, ACP/WAC, ShEx, SHACL.
- W3C SKOS Primer + Reference.
- Protégé documentation (Stanford).
- Notion database docs (notion.com).
- Roam data-structure write-up (zsolt.blog, 2021).
- Scrapscript (scrapscript.org; Max Bernstein compiler notes).
- Linked Data Fragments (linkeddatafragments.org, AtomGraph
  Web-Client, Graphity).
