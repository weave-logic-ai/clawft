---
title: Palantir Foundry Ontology — architecture research
source_urls:
  - https://www.palantir.com/docs/foundry/ontology/overview/
  - https://www.palantir.com/docs/foundry/ontology/overview
  - https://www.palantir.com/docs/foundry/ontology/core-concepts
  - https://www.palantir.com/docs/foundry/object-link-types/object-types-overview/
  - https://www.palantir.com/docs/foundry/object-link-types/link-types-overview/
  - https://www.palantir.com/docs/foundry/object-link-types/properties-overview/
  - https://www.palantir.com/docs/foundry/object-link-types/shared-property-overview
  - https://www.palantir.com/docs/foundry/object-link-types/allow-editing/index.html
  - https://www.palantir.com/docs/foundry/interfaces/interface-overview/
  - https://www.palantir.com/docs/foundry/action-types/overview/
  - https://www.palantir.com/docs/foundry/action-types/overview/index.html
  - https://www.palantir.com/docs/foundry/functions/overview/
  - https://www.palantir.com/docs/foundry/functions/overview
  - https://www.palantir.com/docs/foundry/functions/permissions
  - https://www.palantir.com/docs/foundry/ontology-manager/
  - https://www.palantir.com/docs/foundry/ontology-manager/save-changes
  - https://www.palantir.com/docs/foundry/ontology-sdk/overview
  - https://www.palantir.com/docs/foundry/object-backend/overview
  - https://www.palantir.com/docs/foundry/object-edits/materializations/index.html
  - https://www.palantir.com/docs/foundry/object-permissioning/ontology-permissions
  - https://www.palantir.com/docs/foundry/object-permissioning/object-security-policies
  - https://www.palantir.com/docs/foundry/object-explorer/overview/
  - https://www.palantir.com/docs/foundry/object-views/overview/
  - https://www.palantir.com/docs/foundry/vertex/
  - https://www.palantir.com/docs/foundry/machinery/
  - https://community.palantir.com/t/true-knowledge-graph-capabilities/1045
  - https://medium.com/timbr-ai/palantir-timbr-the-enterprise-race-to-make-data-ai-ready-4b26a1efe89c
created: 2026-04-23
scope: input for WeftOS ontology design; not an adoption plan
---

# Palantir Foundry Ontology — Architecture Research

Foundry describes the Ontology as *"an operational layer for the organization"* that
*"sits on top of the digital assets integrated into the Palantir platform"* and
*"connects them to their real-world counterparts."* It is explicitly positioned as a
**digital twin** of the enterprise — not a data catalog, not a schema registry, but a
runtime over the organization's data + decisions.

## 1. Layered architecture

Foundry's own language splits the stack into **three layers**: a foundation of
digital assets, a **semantic** layer that names and links them, and a **kinetic**
layer that changes them under governance. User-facing applications then compose
on top.

```
+-----------------------------------------------------------------+
| USER APPLICATIONS   Object Explorer | Object Views | Workshop   |
|                     Vertex (graph)  | Machinery (process)       |
+-----------------------------------------------------------------+
| GOVERNANCE          Ontology Manager (branching, proposals,     |
|                     usage, observability, permissions UI)       |
+-----------------------------------------------------------------+
| KINETIC LAYER       Action Types | Functions | Ontology Edits   |
|                     (writeback path via Actions service)        |
+-----------------------------------------------------------------+
| SEMANTIC LAYER      Object Types | Properties | Link Types      |
|                     Interfaces   | Shared Properties            |
+-----------------------------------------------------------------+
| OBJECT BACKEND      Object Storage v2 (indexer + query)         |
|                     Object Data Funnel (datasource + edits)     |
|                     Materializations / writeback datasets       |
+-----------------------------------------------------------------+
| FOUNDATION          Datasets | Streams | Restricted Views |     |
|                     Virtual Tables | Models                     |
+-----------------------------------------------------------------+
| CROSS-CUTTING       Markings, Orgs, Projects (Compass),         |
|                     Object/Property Security Policies           |
+-----------------------------------------------------------------+
```

Each upward arrow exposes a narrower surface: datasources are raw; the object
backend imposes keying and indexing; the semantic layer names entities and
relationships; the kinetic layer defines how those named things are allowed to
change; governance controls who can change the definitions themselves; apps
consume the whole thing.

## 2. The semantic layer

### 2.1 Objects & Object Types
An **object type** is *"the schema definition of a real-world entity or event."*
An **object** is a single instance, an **object set** is a query-result collection
of objects. The analogy Foundry uses explicitly: object types ≈ datasets, objects
≈ rows, object sets ≈ filtered row collections.

Object types are backed by one or more **datasources** (dataset, stream,
restricted view). They must declare a **primary key** property and a **title**
property. Valid primary-key base types are `String`, `Integer`, `Short`; timestamps
are permitted as titles but discouraged as primary keys due to collision risk.
Object types carry metadata including API name, display name, icon, description,
render hints, type classes, and status.

Lifecycle operations include create / edit / copy, configuration of structs and
link types, enabling edits (writeback), Gotham/Gaia integration, and migrations
between Object Storage V1 and V2. The storage-version migration is explicitly
called out as a breaking change.

### 2.2 Properties
A **property** is *"the schema definition of a characteristic of a real-world
entity or event."* Foundry distinguishes property (schema) from **property value**
(instance datum). The base-type set is wide and has governance significance, not
just representation: `String` / `Integer` / `Short` (title + pk), `Date` /
`Timestamp` (title, pk-discouraged), `Boolean` / `Byte` / `Long` (pk-discouraged),
`Float` / `Double` / `Decimal` (title only), plus `Vector`, `Array`, `Struct`
(non-nested), `Media Reference`, `Time Series`, `Attachment`, `Geopoint`,
`Geoshape`, and the security-meaningful `Marking` and `Cipher`.

Properties carry render hints, visibility settings, formatting, and conditional
logic. They can be **derived** (computed from other properties) and can carry
security markings directly.

### 2.3 Link Types
A **link type** is *"the schema definition of a relationship between two object
types."* A **link** is a single instance. Properties: directional, same-ontology
only (no cross-ontology links), self-referential allowed (e.g. `Employee` ↔
`Employee` for manager/report). Many-to-many link types are backed by their own
datasource, functioning like a join table; 1:1 and 1:many are typically backed by
join keys on the object types themselves. The docs draw the database analogy
explicitly: links are to object types what joins are to datasets.

### 2.4 Interfaces
Interfaces are *abstract*; object types are *concrete*. *"Object types are
concrete ... and can be instantiated as objects. By contrast, interfaces are
abstract ... and cannot be instantiated directly but must be instantiated as a
specific object type."* An interface declares **local properties** (the
recommended path) or **shared properties**, **link type constraints**, and
metadata. An object type may implement multiple interfaces.

Consumers: Functions fully consume interfaces (TypeScript v2); Actions
create/modify/delete interface-implementing objects but cannot reference
interface link types directly — they must target concrete link types.

### 2.5 Shared Properties
A **shared property** is *"a property that can be used on multiple object types in
your ontology."* The shared entity is the **definition and metadata** (name, base
type, description, render hints, visibility); *"the underlying object data is not"*
shared. Shared properties show a globe icon in Ontology Manager. Use case:
guarantee that `start_date` on `Employee` and `Contractor` is the same property
semantically. Structs + shared properties compose.

## 3. The kinetic layer

### 3.1 Action Types
An **action type** is *"the definition of a set of changes or edits to objects,
property values, and links that a user can take at once"*, including side-effect
behaviors. An **action** is *"a single transaction that changes the properties of
one or more objects, based on user-defined logic."* Configurable fields:

- **Parameters** — typed inputs, default values, dropdown filtering, override hooks
- **Rules** — business logic governing which edits the action emits
- **Submission criteria** — validation gates (business + permissions)
- **Side effects** — notifications, webhooks
- **Permissions** — who may invoke

Invocation surfaces: UI forms, REST API (OSDK), from within Functions (as
**function-backed actions**). Execution is transactional — *"changes commit
immediately to the ontology and appear across all applications."* OSv2 batch
limit is 10,000 objects per Action.

### 3.2 Functions
Functions are *"pieces of code-based logic that take in input parameters and
return an output"*, *"executed quickly in operational contexts."* Languages:
TypeScript (v1, v2) and Python. First-class ontology access: read properties,
traverse links, emit edits. Invocation contexts include Workshop (chart
aggregations, function-backed columns, object sets), Actions (function-backed
actions), Slate, Quiver, external systems via OSDK, and Pipeline Builder (Python
functions as sidecar containers). Execution is *"server side in an isolated
environment."*

Functions vs. Actions: Functions are reusable pure-ish logic; Actions are the
transactional edit mechanism. A function-backed action wraps a Function to
produce the edit list an Action emits.

### 3.3 Ontology Edits (transactions)
Edits are the unit of change the Actions service applies to the object backend.
An Action is one transaction over edits. The Object Data Funnel ingests both
datasource data and user edits; downstream applications always see the unified
view. Foundry frames Actions as *"a single transaction"* — atomic, commits
immediately, reflected in all user-facing apps.

### 3.4 Writebacks
There are two writeback shapes depending on storage version:

- **OSv1 (Phonograph, deprecating 2026-06-30)**: edits require a **writeback
  dataset**. *"Edits are written to the writeback dataset and not the dataset
  backing an object type or link type."* Writeback datasets are buildable,
  retention-configurable, and must be explicitly generated per object/link type.
- **OSv2**: materialized writeback is optional. Users enable edits by toggling the
  `Edits` configuration on the object type. OSv2 supports multiple
  **materializations** per object type, with either automatic (low-latency) or
  periodic (every ~6 hours) propagation. Materializations are read-only
  downstream views combining datasource + edits.

Permissions split sharply: *"The ability to view objects and links is controlled
by an object type and link type's backing datasources. The ability to view the
edits on objects and links is controlled by the permissions on the writeback
dataset."* A user can legitimately see pre-edit state and be blind to applied
edits.

## 4. The governance layer — Ontology Manager

Ontology Manager is the application that holds the authoring, review, and
observability surface for the entire ontology. It is reachable from the Workspace
sidebar, from right-click `Configure object type` in Data Lineage, or directly at
`/workspace/ontology`.

**What it manages** — every ontology primitive: object types, properties, link
types, shared properties, action types, function types, interfaces. Per-resource
views expose metadata, properties, action types, link graphs, dependents, data,
and usage. Action type and function type views additionally have an
**Observability** tab exposing metrics, monitoring rules, and (for functions)
usage history and version selection.

**Change control is branch-based, via Foundry Branching.** Proposal branches
originate from `main`. *"Every modified Ontological entity will constitute a
separate Task in your proposal and made available for review."* Proposals bundle
all changed entities into a single reviewable unit. At least one approval from an
editor of each affected resource is required; self-approval is permitted if the
editor has edit rights. Merge publishes to `main`. A simpler in-manager save flow
also exists: unsaved local edits → review dialog → Save, with error/warning
gating, merge-conflict resolution against concurrent saves, and typed-name
confirmation for destructive edits on writeback-enabled types.

**Data model**: ontology resources live in Compass projects, inheriting folders,
access requests, markings, and tags. Proposals are versioned and auditable via
Foundry Branching. Ontology Manager also holds observability dashboards (per
action type, per function type) — not a separate monitoring product.

## 5. User-facing applications

### 5.1 Vertex
Vertex is graph exploration and what-if simulation over the ontology. *"Visualize
and quantify cause and effect across the digital twin of your real-world
organization."* Primitives consumed: object types + link types (as nodes/edges,
customizable display), Functions (for derived graph properties and
graph-generation), Actions (for applying ontology-based operations from within
the graph view), time-series properties for temporal analysis. Workflow features
include template-based graph creation, curated graph sharing, pivot navigation,
and scenario simulation.

### 5.2 Machinery
Machinery is **process/workflow analytics** — closer to process mining than to
graph exploration. Models entities that undergo state changes over time (claims,
patients, orders, requisitions). Consumes event logs + action history. Defines
performance metrics, identifies bottlenecks, and supports AIP-assisted multi-step
automation. Depends on ontology primitives for the state-bearing entities.

### 5.3 Object Explorer
*"Object Explorer is a search and analysis tool for answering questions about
anything in the Ontology."* Operations: keyword/property search, preset +
configurable visualizations (charts, maps), drill-down into object sets, set
comparison, **bulk Actions (like writeback)** on an object set, saved
Explorations, export, handoff to Quiver. Primitives: objects, object sets, object
views, properties. It is the primary read+bulk-write interaction point.

### 5.4 Object Views
*"Object Views are reusable representations of object data. They provide a
central hub for all information related to an object and include key information
about the object, including property data, object links, and related
applications."* Two kinds: **Standard** (auto-generated from object type
config — zero-config default) and **Configured** (Workshop-authored, becomes
default once built, user can toggle back). Two form factors: **Full** (whole-page)
and **Panel** (embedded in other apps).

**What is lost without the app tier.** Semantic + kinetic alone give you
the data model and the transactional API, but nothing *interactive*. You lose:
zero-config inspection of any entity (Standard Object Views), ad-hoc exploration
and bulk edits (Object Explorer), relationship navigation and simulation
(Vertex), and process-shape analytics (Machinery). These are not rendering
thin-clients over the API — each encodes non-trivial UX (bulk action dispatch,
pivot-preserving navigation, temporal overlays, event-log shape inference).

## 6. Writebacks & external systems

Foundry's writeback story is **edits-first, not event-first**. The Actions
service is the single commit path; the Object Data Funnel indexes the union of
source datasources + edits into OSv2. Materializations expose the merged view as
a dataset that *downstream pipelines* can consume, and that side channels can
export. External-system propagation is generally **outbound via Action
side-effects** (webhooks, notifications) and **outbound via materialized-dataset
consumers** — there is no documented generic two-way source-of-truth sync; the
writeback dataset is authoritative for edits, the source dataset for source data,
and the materialization is the reconciled view. Transactional guarantees are
per-Action atomic (up to 10k objects/action in OSv2), commit-immediate,
app-consistent.

## 7. Policy & permissions model

Ontology permissions run through Compass (project-based). *"Ontology resources
now use the same permission system as other resource types."* Granularity is
layered:

- **Resource-level (RBAC)** — viewer / editor / owner on each ontology resource
  (object type, link type, action type, function type, interface). For links you
  need edit on both the link type and the linked object types; for actions you
  need edit on the action type and every ontology resource it mutates.
- **Mandatory controls** — Markings (PII, classification), Organizations (hard
  silos), propagating down file hierarchy *and* data dependencies.
- **Object Security Policies (row-level)** — per-instance view permissions
  decoupled from datasource permissions. Inherit mandatory controls by default;
  can add/remove mandatory controls per policy.
- **Property Security Policies (column-level)** — scoped to subsets of properties
  on an object type. Combines with object policies to produce **cell-level**
  security. Primary keys cannot be covered by a property security policy; each
  non-key property can belong to at most one.
- **Function execution** — *"When a function loads object data, either as a
  parameter or via an Object search, the permissions of the end user running the
  function determine which objects are loaded."* RLS applies at function
  invocation, not at authoring.

Legacy pre-project ontology roles exist but are deprecated in favor of the
project-based model.

## 8. What primitive operations are truly primitive

Load-bearing (cannot be derived from anything else in the model):

- **Object Type** — the naming + identity + primary-key anchor.
- **Property** — the field. Base types, markings, and the pk/title flags are
  irreducible.
- **Link Type** — relationships with their own cardinality, direction, and
  backing. Not just a property of type "reference," because links are
  independently permissioned and backed.
- **Action Type** — the transactional write surface; defines what edit-sets are
  expressible and gates them.
- **Writeback / Materialization channel** — without this, "edits" have nowhere
  to live.
- **Ontology Edits (transaction unit)** — the atomic commit primitive the
  Actions service applies.
- **Permissions layers** — RBAC + mandatory (markings/orgs) + ABAC (security
  policies) are each irreducible; you cannot fake mandatory controls with
  RBAC, and you cannot fake RLS with column selection.
- **Branch / Proposal** — without versioned change control, ontology edits are
  ungovernable.

Convenience sugar / derivable (important, not foundational):

- **Shared Properties** — syntactic sugar for "same property definition on many
  object types." Derivable by convention + a property registry; elevated to
  primitive for centralized metadata management.
- **Interfaces** — polymorphism over object types. Could in principle be
  encoded as named predicate-sets on object types, but Foundry elevates it
  because Functions (and eventually Actions) dispatch on interface identity.
- **Functions** — pure computation; not structurally required for a semantic
  layer, but required for any non-trivial business logic in Actions, Workshop,
  Vertex, Slate, and OSDK.
- **Object Views** — pure presentation; derivable from object type + property
  metadata (the "Standard" view is literally auto-generated).
- **Object Explorer / Vertex / Machinery** — consumers of the API, not
  primitives, but each encodes non-trivial capability.

## 9. Known limitations / tradeoffs

- **Closed ecosystem, not W3C semantics.** Foundry is a *property-graph /
  object-graph* model with proprietary identity, not RDF/OWL. *"Palantir's
  ontologies are defined within its own object framework supported by
  ingestion-based pipelines and custom APIs"* — customers sacrifice portability
  of the semantic model itself. There is no native SPARQL, no OWL reasoning, no
  cross-ontology identity.
- **Not a real knowledge graph.** Palantir's own community acknowledges it: *"it
  kind of does the job but feels very cumbersome and not a real KG."* Native
  support for arbitrary node-edge-label-property modeling requires workarounds
  (dummy `Entity` + `Relationship` object types, LLM parsing, visualization
  hacks in Vertex).
- **Cost + operational weight.** Deployments are service-intensive; TCO reaches
  millions/yr for large enterprise. *"Not lightweight and not the fastest tool
  for iterating on ad-hoc data hacks."*
- **Storage-version schism.** OSv1 (Phonograph) and OSv2 are architecturally
  distinct; migrating is a breaking change, and semantics of materializations /
  writeback differ. OSv1 deprecation is 2026-06-30.
- **Edit-vs-view permission asymmetry.** RLS on writeback datasets vs. source
  datasets can let users see "old" state and be invisible to new state — a
  governance subtlety that can mislead.
- **Actions cannot target interface link types directly** — they must target
  concrete link types, leaking abstraction.
- **No cross-ontology links** — a hard boundary that forces everything into one
  namespace.
- **Execution-vs-traceability tradeoff** — every edit flows through Actions and
  is materialized; this buys auditability at the cost of raw write throughput.
- **Ontology Manager is a monolithic governance UI** — proposals bundle all
  ontology-resource changes into one review unit, which simplifies coordination
  but concentrates review pressure.

## 10. Cross-references for future WeftOS design

One-line mapping, no design proposals.

| Foundry concept | Nearest WeftOS analog today |
|---|---|
| Foundation datasources | the path-keyed KV substrate + data-bearing path namespaces |
| Object Storage backend (OSv2) | substrate storage + indexing layer (none direct yet) |
| Object Data Funnel | substrate write path + pub/sub fanout |
| Object Type (schema) | substrate *schema* / path conventions (none formal yet) |
| Object (instance) | a KV entry at a typed path |
| Property | a sub-key / field under an object path |
| Link Type | explicit edge not yet modelled in substrate |
| Shared Property | no analog (would be a property registry) |
| Interface | no analog (would be a predicate/capability bundle) |
| Action Type | adapter-invoked mutation + pub/sub event; no transactional envelope yet |
| Function | runtime-hosted logic (agent loop / pipeline); no first-class ontology-bound form |
| Ontology Edits | substrate writes, ungoverned at ontology level |
| Writeback dataset / Materialization | Snapshot and derived projections |
| Ontology Manager | no analog; Explorer is read-only today |
| Branch / Proposal | no analog; VCS-like layer over ontology missing |
| Markings / Orgs / Projects | substrate permissioning (partial) |
| Object/Property Security Policies | no row/cell-level ontology policy today |
| Vertex | Explorer (graph view, partial) |
| Machinery | no analog (process analytics absent) |
| Object Explorer | Explorer (object inspection) |
| Object Views | Explorer detail panes (partial, not configurable) |
| OSDK | adapter SDKs / service crates (per-service, not ontology-wide) |

## Appendix A — glossary of Foundry terms

- **Ontology** — the named, governed collection of object types + link types +
  action types + functions + interfaces + shared properties that forms the
  organization's digital twin.
- **Object Type** — schema for an entity/event. Backed by ≥1 datasource. Declares
  primary key + title.
- **Object** — instance of an object type.
- **Object Set** — query-result collection of objects.
- **Property** — typed field on an object type. Can be local, derived, or
  shared.
- **Property Value** — the datum at an object × property coordinate.
- **Link Type** — directional relationship schema between two object types
  (possibly same type). Backed by join keys or by a link-type datasource for
  many-to-many.
- **Link** — instance of a link type.
- **Interface** — abstract shape declaring properties + link constraints; object
  types implement interfaces (multiple allowed).
- **Shared Property** — property definition (not data) reused across object
  types, centrally managed.
- **Action Type** — schema for a user-invokable transaction that edits the
  ontology. Contains parameters, rules, submission criteria, side effects,
  permissions.
- **Action** — one execution / transaction of an Action Type.
- **Ontology Edit** — the unit of change (create/modify/delete object,
  create/delete link) that the Actions service commits.
- **Function** — server-side TypeScript/Python logic, ontology-aware, invocable
  from many Foundry surfaces.
- **Function-backed Action** — Action Type whose edit set is produced by a
  Function.
- **Object Storage V1 (Phonograph)** — legacy object database; deprecating
  2026-06-30.
- **Object Storage V2** — next-gen object database; separated indexer + query;
  up to 10k objects/action, 2k properties/type, streaming/incremental indexing.
- **Object Data Funnel** — service reading datasources + Action edits and
  indexing into OSv2.
- **Writeback Dataset** (OSv1) — dataset that captures user edits separately
  from the source dataset.
- **Materialization** (OSv2) — configurable read-only view merging datasource
  data with edits; auto or periodic refresh.
- **Ontology Manager** — application for authoring, reviewing, observing
  ontology resources.
- **Ontology Proposal** — branch-based change-review bundle; approved + merged
  back to `main`.
- **Compass** — Foundry's project/filesystem layer; ontology resources live in
  projects and inherit their permissioning.
- **Marking** — mandatory access control tag (e.g. PII); enforced platform-wide.
- **Organization** — hard silo boundary at the user/resource level.
- **Object Security Policy** — row-level view policy on object instances,
  decoupled from datasource permissioning.
- **Property Security Policy** — column-level policy scoped to a property subset;
  requires an object security policy to exist first.
- **Object Explorer** — search-and-analysis app over object sets, with bulk
  Actions.
- **Object View** — per-object hub UI (Standard or Configured; Full or Panel).
- **Vertex** — graph-exploration + simulation app over object/link types.
- **Machinery** — process-mining / state-transition-analytics app.
- **Ontology SDK (OSDK)** — generated, type-safe client (TypeScript, Python,
  Java, OpenAPI) for external apps to read/write the ontology under
  user-scoped + app-scoped tokens.
