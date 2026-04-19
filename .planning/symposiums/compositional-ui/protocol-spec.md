# WSP-0.1 — WeftOS Surface Protocol, version 0.1

**Status**: Proposed — Round 2 (Symposium: Compositional UI for an Agentic OS)
**Document type**: Authoritative wire specification
**Audience**: Implementers of conformant renderers, composers, and kernel hosts

This document specifies the **WeftOS Surface Protocol (WSP)**, version
0.1. It is the wire contract between composers (agents or the kernel
acting on an agent's behalf), renderers (egui-canonical today, any
conformant viewer tomorrow), the substrate (kernel, mesh, chain,
service registry), and governance (constitutional layer). A renderer
that implements the requirements of this document MUST be able to
host any surface a conformant WeftOS composer emits, including
surfaces the renderer's authors did not anticipate.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL
NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and
**OPTIONAL** in this document are to be interpreted as described in
RFC 2119 / RFC 8174 when, and only when, they appear in all capitals.

---

## 1. Front matter

### 1.1 Version and status

- **Wire version**: `wsp_version = 1` (0.1 is the document identifier;
  on the wire the version discriminator is integer `1`).
- **Status**: Proposed — Round 2. Binding for the prototype phase.
  Round 3 will address the open questions in §18 and MAY extend but
  MUST NOT contract this surface.
- **Supersedes**: none. First protocol spec in the Compositional UI
  symposium.

### 1.2 Related session documents

Mandatory background for any implementer:

- `foundations.md` — the canon (four predicates, tier-A/B split,
  active-radar loop, digital-exhaust-as-intent, non-negotiable
  privacy constraints). Every requirement in this spec traces back
  to a predicate or non-negotiable in that document.
- `session-3-shared-realtime.md` — event-sourced composition spine
  (§9) derives from its analysis of Figma/Yjs/Slack/mission-control.
- `session-4-ontology-ui.md` — the `PrimitiveRef` tuple (§2, §7) and
  the ontology addressability requirements.
- `session-5-renderer-contracts.md` — the 18-primitive canon (§7)
  and the `CanonWidget` / `CanonResponse` renderer-side trait.
- `session-6-protocol-design.md` — the verb budget (§5), transport
  bindings (§3), CDDL shape (§6, Appendix A). This spec is the
  canonicalisation of that session's recommendations.
- `session-8-governance.md` — goal-gating (§11), goal schema, the
  double-gate enforcement model.

### 1.3 Related ADRs

This spec is the binding resolution of the following ADR numbers
(to be authored in parallel):

- **ADR-001** — adopt the 18-primitive canon; no Tier-C.
- **ADR-002** — `primitive-head` carries state, affordances,
  confidence, variant-id, mutation-axes, privacy-flags by composition.
- **ADR-003** — surface tree is state-diff; observation tapestry is
  event-sourced.
- **ADR-004** — transport: JSON-RPC 2.0 for local/UDS and browser/
  WebSocket; CBOR-framed `MessagePayload::Wsp` for mesh; single CDDL.
- **ADR-005** — 17 WSP verbs, hard cap on the budget.
- **ADR-006** — `PrimitiveRef` tuple as the canonical handle.
- **ADR-007** — goals are first-class substrate objects; every verb
  carries a `goal_id`; double-gated enforcement.
- **ADR-008** — capture wrappers are structurally gated; implicit
  tray-chip auto-composition.
- **ADR-009** — versioning by capability flags and ontology URIs;
  unknown primitive types degrade to `ForeignSurface`.
- **ADR-010** — ontology URIs are stable or break.
- **ADR-011** — observation kinds are open by URI; consumers skip
  unknown kinds.
- **ADR-012** — governance denials are structured; never silent
  no-ops; may elicit consent inline.

### 1.4 Scope

WSP describes the messages exchanged among **composer**, **renderer**,
**substrate**, and **governance**, carrying the following content:

- Tree composition of Tier-A primitives and the single Tier-B
  wrapper (`ForeignSurface`).
- Subscriptions to typed substrate resources.
- Active-radar return signals (topology, doppler, range, bearing,
  explicit, implicit, ambient, state, consent, governance).
- Variant identity and mutation axes for GEPA-style learning.
- Consent grants, capture policies, and governance decisions.
- Participant presence and seat assignment.

### 1.5 Non-goals

WSP does **not** specify:

- Widget look and feel (theme layer sits on top of this spec).
- The concrete egui widget mapping (see `session-5-renderer-contracts.md`).
- Editor-sidecar embedding detail (see `session-7-dev-panel-embedding.md`).
- The goal DSL (see `session-8-governance.md`).
- Cross-substrate identity resolution beyond `actor-id` opacity
  (Round 3, see §18).
- A payment or economic allocation model.

WSP is deliberately small (17 verbs, 17 Tier-A primitives + 1 Tier-B
wrapper, 10 observation kinds). Richness lives in the ontology URIs
the protocol *refers* to, not in the protocol itself. This is the
reverse-DDD arrow in action.

---

## 2. Terminology

The following terms are capitalised throughout this spec and have
precise meanings.

- **Surface** — a composed tree of primitives rooted in a
  `surface-spec`, identified by a `surface-id`, owned by exactly
  one composer, rendered by one or more renderers.
- **Primitive** — a Tier-A base UI element satisfying all four
  predicates from `foundations.md`. There are 17 Tier-A primitives
  (§7).
- **PrimitiveRef** — the canonical tuple addressing a rendered
  primitive: `(iri, instance_iri, cid, context, shape, variant)`.
  Every primitive on the wire is identifiable by one. See §7.1.
- **Variant** — a rendered instance of a primitive at a specific
  point along its declared mutation axes, identified by
  `variant-id`. The active-radar loop attributes return signals to
  variants.
- **ForeignSurface** — the sole Tier-B primitive: a wrapper hosting
  opaque content (browser, terminal, video, xwindow, app,
  camera/mic/screen). Its shell satisfies all four predicates; its
  interior is opaque to composition but emits typed signals.
- **Observation** — a single immutable typed record on a
  subscription-id stream, carrying a monotonic `seq`. Observations
  are the event-sourced half of WSP.
- **Goal** — a first-class substrate object (Evans aggregate root)
  with identity, granted/denied affordances, effect ceiling,
  capture policy, and lifecycle state. Every verb call carries a
  `goal_id`. See §11.
- **Context** — a bounded-context frame (Evans) identified by an
  ontology URI, scoping the ubiquitous language of a surface.
- **Consent** — an explicit, revocable grant recorded on the chain
  that permits a capture wrapper to operate and (optionally) to
  retain raw data. Identified by `consent-id`.
- **Participant** — a principal in a surface: human, agent, service,
  or foreign-shell. Has a seat (role + affordance set), presence,
  and identity.
- **Session** — a logical connection between a renderer and the
  kernel, opened by `session.initialize`, closed by
  `session.shutdown`. Carries its own capability set.
- **Substrate** — the union of kernel, mesh, chain, and service
  registry that owns ground truth. Primitives subscribe to substrate
  resources by URI.
- **Subscription** — a named interest in a typed substrate resource,
  opened by `subscribe`, delivering event-sourced `substrate.update`
  notifications. Identified by `subscription-id`.
- **Verb** — a WSP method (§5). One of 17. Verbs are either
  client-to-server (request) or server-to-client (notification).
- **Composer** — the agent (or kernel acting on its behalf) that
  authors `surface.compose`, `surface.update`, and `mutate` calls.
- **Renderer** — any conformant viewer that accepts compositions and
  emits observations. Does not mutate surface structure.
- **Governance** — the constitutional layer adjudicating every
  `invoke`, `mutate`, and capture-sensitive call against the active
  goal set.
- **Gate** — an enforcement point (routing or handler) that consults
  governance before a call reaches its effect.

---

## 3. Transport bindings

WSP is transport-polymorphic. One CDDL schema (Appendix A) governs
three transports with two encodings.

### 3.1 Local (UDS)

- **Rail**: Unix domain socket exposed by the kernel daemon
  (existing `clawft-rpc::protocol`).
- **Framing**: line-delimited JSON-RPC 2.0, one message per line
  terminated by `\n`. Both requests/responses and notifications
  SHALL be carried on the same socket.
- **Envelope**: `Request { jsonrpc: "2.0", id, method, params }`,
  `Response { jsonrpc: "2.0", id, result | error }`, and the new
  `Notification { jsonrpc: "2.0", method, params }`. Notifications
  MUST NOT carry an `id` and MUST NOT elicit a response.
- **Encoding**: JSON. Map keys are strings matching the CDDL field
  names in Appendix A.
- **Concurrency**: the kernel MAY process requests out of arrival
  order. Clients MUST correlate on `id`. Notifications SHALL preserve
  the order emitted by the kernel on a given subscription.

### 3.2 Mesh (cross-node)

- **Rail**: `clawft-kernel::ipc::MeshIpcEnvelope`.
- **Framing**: a new payload variant on `MessagePayload`:

```rust
// Added to MessagePayload; #[non_exhaustive] preserves back-compat.
Wsp { frame: Vec<u8> }
```

- **Envelope**: the CBOR-encoded bytes inside `frame` are a
  self-describing WSP frame — either a `Request`, `Response`, or
  `Notification` CDDL structure, keyed by integer map keys on hot
  paths (see Appendix A).
- **Encoding**: CBOR (RFC 8949). Integer tags are used where defined
  in Appendix A; otherwise string keys matching the JSON form.
- **Routing**: the outer `MeshIpcEnvelope` handles routing, peer
  auth, replay, and encryption. WSP MUST NOT attempt to duplicate
  any of those concerns inside the frame.

### 3.3 Browser / third-party renderer

- **Rail**: WebSocket.
- **Framing**: JSON-RPC 2.0 over WebSocket text frames. One logical
  WSP session per WebSocket connection.
- **Encoding**: JSON by default. A server MAY offer CBOR via the
  `wsp.cbor` capability during `session.initialize`; if both sides
  agree, subsequent frames on that WebSocket are WebSocket binary
  frames containing CBOR.

### 3.4 Encoding equivalence rule

The CDDL in Appendix A is authoritative. For any CDDL structure:

1. The **CBOR** encoding uses integer map keys where the CDDL
   specifies them and the field order listed in the CDDL (keys are
   still a set, not a list; order is only a hint for encoder
   stability).
2. The **JSON** encoding uses the string key names as written in
   Appendix A. A JSON map key MUST match the CDDL field name
   verbatim; renderers MUST NOT accept abbreviations.
3. Integers that are bounded in CDDL MUST be represented as
   non-fractional JSON numbers within the bound; floats MAY be
   represented as JSON numbers (but MUST round-trip exactly via
   double-precision IEEE 754).
4. `any` CDDL values map to arbitrary JSON values and arbitrary
   CBOR values; a renderer MUST NOT re-encode `any` fields when
   relaying.

A single CDDL, two encodings. A renderer that speaks only JSON is
still conformant. A renderer that speaks CBOR MUST also be able to
decode JSON for `session.initialize` (handshake is always JSON-safe).

---

## 4. Session lifecycle

### 4.1 `session.initialize`

Every session begins with exactly one `session.initialize` request
from the renderer. The kernel (server side) responds with a mirror
capability set.

- **Direction**: renderer → server (request).
- **Params**:

  ```cddl
  client-caps = {
    wsp-version     : uint,                ; this document: 1
    persona         : ontology-uri,        ; predicate 4 + Cooper posture
    locale          : tstr,
    supported-encodings : [* "json" / "cbor"],
    supported-primitives : [* ontology-uri],
    supported-wrappers   : [* ontology-uri],
    supported-topics     : [* ontology-uri],
    a11y-hints      : { * tstr => any },
    ? cbor-tag-profile  : uint,            ; for future CBOR tag assignment
    ? initial-goal  : goal-id,             ; if absent, adhoc-scratch opens
  }
  ```

- **Response**:

  ```cddl
  server-caps = {
    wsp-version     : uint,                ; MUST match a supported version
    encoding        : "json" / "cbor",
    supported-primitives : [* ontology-uri],
    supported-wrappers   : [* ontology-uri],
    supported-topics     : [* ontology-uri],
    capability-flags : [* tstr],           ; feature flags; see §14
    session-id      : tstr,
    ? deprecations  : [* { uri: ontology-uri, replaced-by: ontology-uri }],
  }
  ```

- **Version negotiation**: the server MUST respond with a
  `wsp-version` value equal to one offered in `client-caps.wsp-version`
  (encoded as the single integer this spec defines, 1, or any future
  version a server supports). If no common version is found, the
  server MUST fail with error code `-32002` (§15) and close the
  session.
- **Capability flags**: unknown capability flags MUST be ignored. A
  server MUST NOT fail initialization on an unrecognised flag.
- **Persona**: the client's declared persona (developer-in-dev-panel,
  regional-manager, war-room-participant, witness) is an input to
  composition heuristics. It is NOT a security boundary.

### 4.2 `session.shutdown`

- **Direction**: renderer → server (request).
- **Params**: `{ }` (empty).
- **Response**: `{ ok: true }`.
- **Effect**: the kernel SHALL drain every open subscription and
  observation stream owned by this session, dispose every surface
  not shared with another session, and flush any pending chain
  events. Sessions that do not `session.shutdown` before closing
  the transport are treated as crashed; the kernel MUST reap them
  via inactivity timeout and MUST chain-log the abnormal closure.

### 4.3 Reconnection semantics

A renderer that disconnects and reconnects within the reconnection
window (default 60 seconds; server-advertised in
`capability-flags`) MAY reattach to its prior session by supplying
the prior `session-id` in `client-caps` under the optional
`reattach: session-id` field. The server MUST:

1. Restore the surface tree snapshots at their last known
   `surface-version`.
2. Replay observation streams from `since_seq` values the client
   provides per subscription.
3. Re-evaluate goal context (goals may have suspended or sealed
   while disconnected; a denied verb MUST be denied the same way
   post-reconnect).

If the reconnection window has passed, the server MUST fail
reattachment and the renderer MUST initialise a fresh session.

---

## 5. Verb catalog

WSP defines exactly **17 verbs**. New behaviour arrives via ontology
URIs (§14), never via new verbs. The verb budget is a hard cap.

Verbs are grouped by namespace. All `namespace.verb` forms follow
dot notation. Observation-bearing verbs open a stream whose
deliveries use the `namespace.update` notification name.

Every request and notification in WSP carries two mandatory envelope
fields in addition to its typed params:

- `goal_id: goal-id` — the active goal. Absence is rejected at the
  routing gate (§11). The kernel opens `adhoc-scratch` implicitly
  on `session.initialize` when `initial-goal` is not declared.
- `variant_id: variant-id` — the variant of the caller's current
  surface pulse (for return-signal attribution). Verbs that are not
  bound to a specific surface (e.g. `session.initialize`) MAY omit
  `variant_id`.

### 5.1 Session (2)

#### 5.1.1 `session.initialize`
- **Direction**: renderer → server (request).
- **Params**: `client-caps` (§4.1).
- **Response**: `server-caps` (§4.1).
- **Errors**: `-32002 VersionUnsupported`.
- **JSON example**:

  ```json
  {
    "jsonrpc": "2.0", "id": 1, "method": "session.initialize",
    "params": {
      "wsp-version": 1, "persona": "persona://dev-panel",
      "locale": "en-US",
      "supported-encodings": ["json"],
      "supported-primitives": ["ui://pressable", "ui://field", "ui://chip"],
      "supported-wrappers": [],
      "supported-topics": ["resource://", "service://"],
      "a11y-hints": {}
    }
  }
  ```

#### 5.1.2 `session.shutdown`
- **Direction**: renderer → server (request).
- **Params**: `{ }`.
- **Response**: `{ ok: true }`.
- **Errors**: none expected.

### 5.2 Surface (4)

#### 5.2.1 `surface.compose`
- **Direction**: composer → server (request).
- **Params**: `surface-spec` (Appendix A).
- **Response**: `{ surface_id: surface-id, surface_version: uint }`.
- **Errors**: `-32010 Malformed`, `-32011 GoalMissing`,
  `-32012 ContextMissing`, `-32013 GatingDenied`.
- **Semantics**: publishes a new surface. The kernel assigns
  `surface_id` (MAY be renderer-hinted). `surface_version` starts at 1.

#### 5.2.2 `surface.get`
- **Direction**: renderer → server (request).
- **Params**: `{ surface_id: surface-id }`.
- **Response**: `surface-spec`.
- **Errors**: `-32014 SurfaceUnknown`.
- **Semantics**: snapshot for cold start or after version mismatch.

#### 5.2.3 `surface.update`
- **Direction**: composer → server (request).
- **Params**: `{ surface_id, base_version: uint, ops: [* op] }` where
  `op` is one of `add | remove | replace` addressed by JSON-Pointer
  path within the primitive tree.
- **Response**: `{ new_version: uint }`.
- **Errors**: `-32015 VersionMismatch`, `-32010 Malformed`,
  `-32013 GatingDenied`.
- **Notification cascade**: on success, the server SHALL emit
  `surface.update` notifications to every other renderer subscribed
  to this surface, carrying the same ops.

#### 5.2.4 `surface.dispose`
- **Direction**: composer → server (request).
- **Params**: `{ surface_id: surface-id }`.
- **Response**: `{ ok: true }`.
- **Semantics**: end-of-life. All subscriptions rooted on the surface
  are closed; the surface transitions to a tombstone (§9.6).

### 5.3 Substrate (2)

#### 5.3.1 `subscribe`
- **Direction**: composer or renderer → server (request).
- **Params**: `{ resource_uri: ontology-uri, ? filter: any,
  ? since_seq: uint }`.
- **Response**: `{ subscription_id: subscription-id }`.
- **Errors**: `-32013 GatingDenied`, `-32016 TopicUnknown`.
- **Stream**: the server SHALL emit `substrate.update` notifications
  carrying `observation` frames with `kind = "state"` or the more
  specific kind the topic declares.

#### 5.3.2 `unsubscribe`
- **Direction**: composer or renderer → server (request).
- **Params**: `{ subscription_id: subscription-id }`.
- **Response**: `{ ok: true }`.

### 5.4 Observation (2)

#### 5.4.1 `observe`
- **Direction**: renderer → server (request).
- **Params**: `{ surface_id: surface-id, channels: [* tstr] }` where
  `channels` is a subset of `["topology", "doppler", "range",
  "bearing", "explicit", "implicit", "ambient", "state", "consent",
  "governance"]`.
- **Response**: `{ observation_id: subscription-id }`.
- **Stream**: `observation.update` notifications.
- **Semantics**: opens the active-radar return channel. Return
  signals are delivered on a separate stream from substrate state.

#### 5.4.2 `observe.close`
- **Direction**: renderer → server (request).
- **Params**: `{ observation_id: subscription-id }`.
- **Response**: `{ ok: true }`.

### 5.5 Invocation (2)

#### 5.5.1 `invoke`
- **Direction**: renderer or composer → server (request).
- **Params**: `{ surface_id, path: tstr, affordance: tstr,
  args: any, actor: actor-id }`.
- **Response**: `{ result: any }` on allow; otherwise structured error.
- **Errors**: `-32020 ConsentRequired`, `-32021 AffordanceUnknown`,
  `-32013 GatingDenied`.
- **Semantics**: typed invocation of an affordance enumerated on
  the primitive head. The kernel MUST reject an `invoke` against an
  affordance not present in the head's `affordances` list.

#### 5.5.2 `mutate`
- **Direction**: composer → server (request).
- **Params**: `{ surface_id, path: tstr, axis: tstr,
  new_variant: variant-id }`.
- **Response**: `{ ok: true }`.
- **Errors**: `-32013 GatingDenied`, `-32022 AxisFrozen`,
  `-32023 AxisUnknown`.
- **Semantics**: the GEPA-style variant change. Always gate-checked.
  `consent` / `safety` / `brand` axes MUST NOT be mutated (§12).

### 5.6 Governance + consent (3)

#### 5.6.1 `gate.check`
- **Direction**: composer → server (request).
- **Params**: `{ action: tstr, subject: tstr, context: any }`.
- **Response**: `{ decision: "allow" | { deny: { reason, policy-uri,
  ? elicit: ontology-uri } } | { elicit: ontology-uri } }`.
- **Semantics**: pre-flight honesty check used by composers to
  enumerate affordances accurately before publishing.

#### 5.6.2 `consent.request`
- **Direction**: composer or governance → server (request).
- **Params**: `{ scope: ontology-uri, purpose: tstr, duration: tstr,
  goal_id: goal-id }`.
- **Response**: `{ consent_id: tstr, surface_id: surface-id }`
  — a consent-flow surface is composed as a side effect (§11.3).
- **Errors**: `-32013 GatingDenied`.

#### 5.6.3 `consent.revoke`
- **Direction**: any principal on the consent → server (request).
- **Params**: `{ consent_id: tstr }`.
- **Response**: `{ ok: true }`.
- **Semantics**: revocation cascades. Every capture channel bound
  to the consent-id ceases immediately, and the associated tray
  chip updates.

### 5.7 Introspection (2)

#### 5.7.1 `ontology.describe`
- **Direction**: composer or renderer → server (request).
- **Params**: `{ target: ontology-uri | surface-id }`.
- **Response**: `{ iri, state-schema: ontology-uri, affordances:
  [* affordance], confidence-model, mutation-axes: [* mutation-axis],
  ? context: ontology-uri, ? shape: ontology-uri }`.
- **Semantics**: the agent queries the ontology via this verb; it
  does not guess (predicate 1).

#### 5.7.2 `cancel`
- **Direction**: any → server (request, JSON-RPC id target).
- **Params**: `{ id: uint | subscription_id: subscription-id }`.
- **Response**: `{ ok: true }`.
- **Semantics**: universal cancellation for requests and streams.
  The kernel MUST tolerate cancellation of already-complete
  requests by returning `ok: true` without error.

### 5.8 Sample CBOR encoding (illustrative)

A `surface.compose` request carrying a single `Pressable` with
integer map keys (using the profile in Appendix A §A.12):

```
A3                         # map(3)
  00                       # key 0 (jsonrpc-version marker)
  01                       # value 1
  01                       # key 1 (method)
  6D 73 75 72 66 61 63 65  # "surface.co... "
  2E 63 6F 6D 70 6F 73 65  # ...mpose" (truncated for brevity)
  02                       # key 2 (params)
  A4                       # map(4) — surface-spec
    ...                    # id, version, root, governance
```

The normative shape is the CDDL; the byte stream above is illustrative
of a conforming encoder that chose integer keys.

---

## 6. Primitive head

Every primitive on the wire carries a **head** by composition. The
head is the enforcement surface for predicates 1, 2, the active-radar
loop, and the digital-exhaust-as-intent framing.

### 6.1 Mandatory fields

A conforming `primitive-head` carries ALL of the following:

| Field           | CDDL type           | Purpose                                           |
|-----------------|--------------------|---------------------------------------------------|
| `type`          | `ontology-uri`      | Typed identity (predicate 1).                    |
| `path`          | `tstr`              | JSON-Pointer path within surface.                |
| `state`         | `any`               | Typed per primitive-type. MAY be empty map.      |
| `affordances`   | `[* affordance]`    | Enumerable operations (predicate 2).             |
| `confidence`    | `confidence`        | Provenance tuple (predicate 2).                  |
| `variant-id`    | `variant-id`        | Active-radar attribution.                        |
| `mutation-axes` | `[* mutation-axis]` | Legal GEPA axes; empty = none.                   |
| `privacy-flags` | `privacy-flags`     | Capture/retention/off-node/consent-id.           |

### 6.2 CDDL

```cddl
primitive-head = {
  type           : ontology-uri,
  path           : tstr,
  state          : any,
  affordances    : [* affordance],
  confidence     : confidence,
  variant-id     : variant-id,
  mutation-axes  : [* mutation-axis],
  privacy-flags  : privacy-flags,
}

affordance = {
  name           : tstr,
  verb           : "read" / "write" / "invoke" / "subscribe" / "dismiss",
  ? actors       : [* actor-id],
  ? args-schema  : ontology-uri,
}

confidence = {
  source         : "deterministic" / "inference" / "cache" / "input",
  ? value        : float64,
  ? interval     : [float64, float64],
  ? as-of        : tdate,
}

privacy-flags = {
  captured       : bool,
  retained       : bool,
  off-node       : bool,
  consent-id     : tstr / null,
}

mutation-axis = {
  name           : tstr,
  kind           : "discrete" / "continuous" / "categorical",
  ? domain       : any,
  frozen-by      : [* "brand" / "safety" / "consent" / "user-request"],
}
```

### 6.3 Validation rules

A kernel receiving a frame MUST validate the following. Any failure
is a **malformed frame** (§6.4) and the frame MUST be rejected at
the kernel boundary — never delivered to a renderer.

1. `type` is a syntactically valid `ontology-uri`.
2. `path` is a syntactically valid JSON-Pointer.
3. `confidence.source` is one of the four enumerated values.
4. `privacy-flags.captured` implies either `consent-id != null` and
   pointing to an active consent, or the primitive is not a
   `ForeignSurface` with a non-empty `capture-channels` list.
5. `privacy-flags.retained` implies `captured = true`.
6. `privacy-flags.off-node` implies `consent-id != null`.
7. Every `affordance.name` is unique within the `affordances` list.
8. Every `mutation-axis.name` is unique within `mutation-axes`.
9. `variant-id` is non-empty.
10. For every axis whose `frozen-by` contains `"safety"` or
    `"consent"`, that axis MUST NOT be listed as mutable — the
    kernel MUST remove it before composition proceeds. Safety and
    consent axes are never mutable, period.

### 6.4 Malformed-at-boundary rule

A frame failing validation at the kernel boundary is **malformed**
and MUST NOT reach any renderer. The kernel SHALL:

1. Respond to the originating request with error `-32010 Malformed`,
   carrying a structured detail message identifying the failing rule.
2. Chain-log the rejection under `governance.malformed`.
3. NOT degrade silently. A missing `privacy-flags` is not zero; it
   is rejection.

This rule is what makes "privacy non-negotiable" a wire property
rather than an aspiration. Every renderer that receives a frame has
the guarantee that `privacy-flags` is present and internally
consistent.

---

## 7. Primitive catalog

WSP defines **18 primitives**. 17 are Tier A (satisfy all four
predicates). 1 is Tier B (`ForeignSurface`); its shell satisfies the
four predicates, its interior is opaque to composition and emits
signal.

### 7.1 PrimitiveRef

Every rendered primitive is identifiable by a `PrimitiveRef` tuple:

```cddl
primitive-ref = {
  iri           : ontology-uri,    ; the class
  instance-iri  : ontology-uri,    ; the concrete binding
  cid           : tstr,            ; blake3(bytes)
  context       : ontology-uri,    ; bounded-context IRI
  shape         : ontology-uri,    ; SHACL shape / TopologySchema
  variant       : variant-id,      ; for active-radar attribution
}
```

A composer references a primitive in `surface.update` ops by the
tuple; a renderer emits `PrimitiveRef` on every return-signal
observation.

### 7.2 Tier A — the 18 primitives

> **Amended 2026-04-19**: Row 18 (`ui://canvas`) added per ADR-013.
> The `ui://modal` section includes the `modality` field per ADR-014.
> `primitive-head` picks up the optional `tooltip` field and the
> `reorderable` per-affordance flag per the ADR-006 amendment.


Each primitive is described with its ontology IRI stem, purpose,
primitive-specific CDDL body (composed with `primitive-head`), legal
mutation axes, privacy-flags relevance, and a reference egui note.

#### 7.2.1 `Chip` — `ui://chip`

A labelled, tinted, interactive status token.

```cddl
chip = { primitive-head,
  label         : tstr,
  kind          : "consent" / "capture" / "status" / "identity" / "goal",
  ? icon        : ontology-uri,
  ? tint        : tstr,
}
```

- **Mutation axes**: `ordering`, `copy`, `density`, `tint` (when
  `kind != "consent"` and `kind != "capture"`).
- **Privacy relevance**: the `kind = "capture"` chip is the tray
  chip auto-composed by the kernel whenever any `foreign-surface`
  declares a live capture channel (§11.4). Agents MUST NOT compose
  this chip directly.
- **egui reference**: `egui::Frame` + `Painter::circle_filled` +
  `selectable_label`; persisted selection via `ctx.memory`.

#### 7.2.2 `Button` — `ui://pressable`

An invoke-one-verb affordance.

```cddl
pressable = { primitive-head,
  label         : tstr,
  ? icon        : ontology-uri,
  ? hotkey      : tstr,
  ? voice       : [* tstr],
  ? style       : "primary" / "secondary" / "ghost" / "destructive",
}
```

- **Mutation axes**: `copy`, `icon`, `style` (if not `destructive`),
  `placement`.
- **Privacy relevance**: N/A (buttons do not capture).
- **egui reference**: `ui.add(egui::Button::new(label))`.

#### 7.2.3 `Field` — `ui://field`

A typed input binding (text / number / date / enum / boolean /
multiline).

```cddl
field = { primitive-head,
  name          : tstr,
  schema        : ontology-uri,
  value         : any,
  dirty         : bool,
  ? errors      : [* tstr],
  ? placeholder : tstr,
}
```

- **Mutation axes**: `placement`, `label-copy`, `placeholder-copy`.
  `schema` and `name` are **frozen**.
- **Privacy relevance**: `privacy-flags.captured` SHOULD be `true`
  for fields whose schema is sensitive (`sensitivity` declared in
  the shape). The kernel MAY auto-mark captured.
- **egui reference**: `TextEdit::*`, `DragValue`,
  `egui_extras::DatePickerButton`, `ComboBox`, `Checkbox`.

#### 7.2.4 `Toggle` — `ui://toggle`

A boolean binding distinguished from `Field` because agents reason
about it specifically.

```cddl
toggle = { primitive-head,
  name          : tstr,
  value         : bool,
  ? label       : tstr,
}
```

- **Mutation axes**: `copy`, `placement`.
- **Privacy relevance**: toggles on capture wrappers are governance-
  mediated (§11.2).
- **egui reference**: `ui.toggle_value(&mut bool, label)`.

#### 7.2.5 `Select` — `ui://select`

A closed-choice picker.

```cddl
select = { primitive-head,
  name          : tstr,
  choices       : [* { value: any, label: tstr, ? disabled: bool }],
  selected      : any,
  ? multiple    : bool,
}
```

- **Mutation axes**: `ordering`, `density`, `modality` (combo vs
  radio vs list).
- **Privacy relevance**: N/A.
- **egui reference**: `ComboBox`, `RadioButton`, `TableBuilder`.

#### 7.2.6 `Slider` — `ui://slider`

A continuous range binding.

```cddl
slider = { primitive-head,
  name          : tstr,
  value         : float64,
  range         : [float64, float64],
  ? step        : float64,
  ? unit        : tstr,
}
```

- **Mutation axes**: `granularity` (step), `modality` (linear vs
  log), `placement`.
- **Privacy relevance**: N/A.
- **egui reference**: `egui::Slider`.

#### 7.2.7 `Stack` — `ui://stack`

One-axis flow (row / column).

```cddl
stack = { primitive-head,
  axis          : "row" / "column",
  ? gap         : float64,
  ? align       : "start" / "center" / "end" / "stretch",
  children      : [* primitive-head],
}
```

- **Mutation axes**: `ordering`, `density`, `gap`.
- **Privacy relevance**: N/A.
- **egui reference**: `ui.horizontal(|ui| ...)` / `ui.vertical(|ui| ...)`.

#### 7.2.8 `Grid` — `ui://grid`

Two-axis regular layout.

```cddl
grid = { primitive-head,
  columns       : uint,
  ? gap         : [float64, float64],
  children      : [* primitive-head],
}
```

- **Mutation axes**: `ordering`, `density`, `granularity` (columns).
- **Privacy relevance**: N/A.
- **egui reference**: `egui::Grid::new(id).num_columns(n)`.

#### 7.2.9 `Strip` — `ui://strip`

Fixed-ratio, fixed-size division (dashboards, split panes).

```cddl
strip = { primitive-head,
  orientation   : "horizontal" / "vertical",
  sizes         : [* size],
  children      : [* primitive-head],
}

size = "remainder" / "relative" / "exact" / { kind: tstr, v: float64 }
```

- **Mutation axes**: `granularity` (size values), `ordering`.
- **Privacy relevance**: N/A.
- **egui reference**: `egui_extras::StripBuilder`.

#### 7.2.10 `Dock` — `ui://dock`

Resizable tabbed / split panels with persisted geometry. Mission
Console chrome.

```cddl
dock = { primitive-head,
  layout        : any,              ; opaque DockState
  tabs          : [* { id: tstr, title: tstr, surface: surface-id }],
  focus         : tstr,
}
```

- **Mutation axes**: `ordering`, `density`.
- **Privacy relevance**: N/A.
- **egui reference**: `egui_dock::DockArea` + `DockState`.

#### 7.2.11 `Sheet` — `ui://sheet`

A scrollable region with sticky headers.

```cddl
sheet = { primitive-head,
  ? stick-to-bottom : bool,
  ? sticky-header   : primitive-head,
  children      : [* primitive-head],
}
```

- **Mutation axes**: `density`, `ordering`.
- **Privacy relevance**: N/A.
- **egui reference**: `egui::ScrollArea::vertical().stick_to_bottom(true)`.

#### 7.2.12 `Modal` — `ui://modal` *(amended 2026-04-19 per ADR-014)*

A foreground surface whose behaviour is selected by the required
`modality` field — one primitive, four modes (modal / floating / tool
/ toast).

```cddl
modal = { primitive-head,
  modality      : "modal" / "floating" / "tool" / "toast",
  ? title       : i18n-string,
  ? dismissable : bool,                    ; default per modality
  ? position    : "centre" / "docked-right" / "docked-left" / [2*float],
  ? ttl-ms      : uint,                    ; required iff modality = "toast"
  body          : [* primitive-ref],
  ; affordances list MAY include confirm | cancel | dismiss; these are
  ; frozen-by="safety" and never GEPA-legible regardless of modality.
}
```

Per-modality semantics (ADR-014 Table):

| modality | Scrim | Focus capture | Dismissable | Default position | Interrupt-priority |
|----------|-------|---------------|-------------|------------------|-------------------|
| `modal`    | yes | yes | via affordance | centre        | immediate  |
| `floating` | no  | no  | always         | last position | no interrupt |
| `tool`     | no  | no  | pinnable       | docked-right  | no interrupt |
| `toast`    | no  | no  | auto (ttl-ms)  | top-right     | ambient    |

Validation: a frame with `modality = "toast"` and no `ttl-ms` is
malformed; a frame with `modality != "toast"` but containing `ttl-ms`
is malformed.

- **Mutation axes**: `copy` (title only). The safety affordance set
  is frozen; additive verbs MAY be appended by schema.
- **Privacy relevance**: consent modals MUST use `modality = "modal"`
  and carry `privacy-flags.consent-id`. Tooltips under ADR-006 §7 are
  rendered as `modality = "tool"` with `dismissable = true`.
- **egui reference**: `egui::Area::new(id).order(Order::*)` with
  `Order::Foreground` + scrim for `modal`, `Order::Middle` for
  `floating` / `tool`, `Order::Tooltip` for `toast`.

#### 7.2.13 `Table` — `ui://table`

Sortable / filterable tabular data with selection.

```cddl
table = { primitive-head,
  columns       : [* { name: tstr, schema: ontology-uri,
                       ? sort: "asc" / "desc" / "none" }],
  rows          : [* { cells: [* any], ? id: tstr }],
  ? selection   : [* tstr],
  ? bind        : subscription-id,
}
```

- **Mutation axes**: `ordering` (columns), `density`.
- **Privacy relevance**: rows with sensitive schema inherit
  `privacy-flags.captured`.
- **egui reference**: `egui_extras::TableBuilder`.

#### 7.2.14 `Tree` — `ui://tree`

Hierarchical disclosure.

```cddl
tree = { primitive-head,
  nodes         : [* tree-node],
}

tree-node = {
  id            : tstr,
  label         : tstr,
  ? children    : [* tree-node],
  ? expanded    : bool,
}
```

- **Mutation axes**: `density`, `ordering` (sibling order).
- **Privacy relevance**: N/A.
- **egui reference**: `egui::CollapsingHeader`.

#### 7.2.15 `Gauge` — `ui://gauge`

A single scalar with bounds, thresholds, and a confidence halo.

```cddl
gauge = { primitive-head,
  value         : float64,
  range         : [float64, float64],
  unit          : tstr,
  ? thresholds  : [* { at: float64, label: tstr, tint: tstr }],
  ? bind        : subscription-id,
  ? style       : "linear" / "radial",
}
```

- **Mutation axes**: `style` (linear vs radial), `density`.
- **Privacy relevance**: N/A.
- **egui reference**: `ProgressBar` or `Painter::arc`.

#### 7.2.16 `Plot` — `ui://plot`

Continuous-time series.

```cddl
plot = { primitive-head,
  series        : [* { name: tstr, schema: ontology-uri,
                       samples: [* [float64, float64]] }],
  ? window      : [float64, float64],
  ? bind        : subscription-id,
  ? density     : "sparse" / "normal" / "dense",
}
```

- **Mutation axes**: `granularity` (sample density), `modality`
  (line vs area vs bar).
- **Privacy relevance**: N/A.
- **egui reference**: `egui_plot::Plot::new(id)`.

#### 7.2.17 `Media` — `ui://media`

An image / icon / glyph primitive (decoded pixels the composer owns).

```cddl
media = { primitive-head,
  source        : ontology-uri,   ; asset IRI or bytes://, file://, https://
  ? alt         : tstr,
  ? dims        : [uint, uint],
}
```

- **Mutation axes**: `placement`, `density` (size).
- **Privacy relevance**: `alt` is the agent's read handle.
- **egui reference**: `egui::Image::new(uri)` +
  `egui_extras::install_image_loaders`.

#### 7.2.18 `StreamView` — `ui://stream-view`

A live-tailing view over an event-sourced subscription.

```cddl
stream-view = { primitive-head,
  feed          : subscription-id,
  cursor        : "live" / uint,         ; "live" or specific seq
  ? density     : "compact" / "comfortable" / "dense",
  ? row-schema  : ontology-uri,
}
```

- **Mutation axes**: `density`, `granularity` (row-schema).
- **Privacy relevance**: inherits from the underlying subscription's
  sensitivity.
- **egui reference**: `ScrollArea` + ring buffer pulled from `Store`.

Note: `Pressable` (7.2.2) is named `Pressable` on the wire though
colloquially known as `Button`; the ontology stem `ui://pressable` is
authoritative.

#### 7.2.19 `Canvas` — `ui://canvas` *(added 2026-04-19 per ADR-013)*

A typed rectangular surface on which callers draw arbitrary layered
content. Closes the Fractal-Clock / paint_bezier / pan_zoom /
dancing_strings / painting demo gap.

```cddl
canvas = { primitive-head,
  size          : [2*uint],                ; render target in logical pixels
  ? view-box    : [4*float],                ; current pan/zoom viewport (x, y, w, h)
  ? layers      : [+ canvas-layer],
  ? cursor      : [2*float],                ; last hit-test, if tracked
}

canvas-layer = {
  id        : tstr,
  kind      : "strokes" / "bezier" / "shapes" / "raster",
  content   : any,                          ; layer-type-specific payload
}
```

Affordances the canvas MUST expose when writable:

| Affordance | Verb | Notes |
|------------|------|-------|
| `draw` | `invoke` | Append or replace a layer. Flows through state-diff. |
| `hit-test` | `invoke` | Returns top-most hit; feeds `topology` echo channel. |
| `pan` | `invoke` | Updates `view-box`. Feeds `doppler`. |
| `zoom` | `invoke` | Updates `view-box`. Feeds `doppler`. |
| `snapshot` | `observe` | Streams raster frames — privacy-gated per §11.4. |

- **Mutation axes**: `density` (layer detail), `granularity`
  (sampling rate for strokes). Layer content kinds are frozen by
  schema.
- **Privacy relevance**: `snapshot` subscriptions count as a capture
  channel — the kernel MUST auto-compose the capture tray chip
  (§11.4) whenever any `canvas` subscription is live.
- **egui reference**: `ui.allocate_painter(size, Sense::click_and_drag())`
  + per-layer calls into `Painter`; `view-box` handled with
  `RectTransform::from_to`.

### 7.3 Tier B — `ForeignSurface`

`ForeignSurface` is the sole Tier-B primitive. Its shell is Tier A
(satisfies all four predicates); its interior is opaque to
composition but emits typed signals.

```cddl
foreign-surface = { primitive-head,      ; type MUST be "ui://foreign"
  wrapper-type      : ontology-uri,      ; "foreign://browser" etc.
  identity-hint     : {
    ? url            : tstr,
    ? bundle         : tstr,
    ? pid            : uint,
    ? media-id       : tstr,
  },
  shell-state       : {
    opened-at        : tdate,
    focus            : bool,
    size             : [uint, uint],
    ? last-url       : tstr,
    ? exit-code      : int,
    health           : "starting" / "ready" / "busy" / "crashed" / "closed",
  },
  presence-signal   : {
    active           : bool,
    attention        : "foreground" / "background" / "hidden",
    activity-rate    : float64,
  },
  capture-channels  : [* capture-channel],
}

capture-channel = {
  kind             : "mic" / "camera" / "screen" / "clipboard" / "a11y-tree",
  emits            : "interpretation" / "raw",
  consent-id       : tstr,          ; MUST reference an active grant
  ? sample-rate-hz : float64,
}
```

**Validation**: a `foreign-surface` with non-empty `capture-channels`
and a `privacy-flags.consent-id = null` is malformed and MUST be
rejected at the kernel boundary (§6.4). The kernel MUST verify that
every `capture-channel.consent-id` references a currently-active
consent grant.

#### 7.3.1 The six canonical wrapper types

Every implementer MUST support the following `wrapper-type` ontology
IRIs. A renderer that cannot host a type renders a degraded
`ForeignSurface` with `shell-state.health = "crashed"` and a
structured error observation (§9.4).

- `foreign://browser` — Chromium/WebView pane. Affordances:
  `navigate`, `reload`, `screenshot`, `read-a11y-tree`,
  `execute-script` (governance-gated), `export-text`. Signals:
  load-start, title, url, a11y-tree-digest, screenshot.
- `foreign://terminal` — PTY host. Affordances: `write`,
  `screenshot-buffer`, `tail-output`, `resize`, `kill`. Signals:
  resize, exit-code, output-rate, active-process, clipboard-read.
- `foreign://video` — media player pane. Affordances: `play`,
  `pause`, `seek`, `set-track`, `subtitles`. Signals: play-state,
  position, track, audio-level, caption-text.
- `foreign://xwindow` — X11 / Wayland window embed. Affordances:
  `focus`, `resize`, `screenshot`, `terminate`. Signals:
  window-title, PID, focus, resize, last-input-at.
- `foreign://app` — nested WeftOS app (another egui/Tauri process).
  Affordances: `launch`, `focus`, `terminate`, `send-message`.
  Signals: bundle-id, PID, handshake version, last-message, health.
- `foreign://camera-mic-screen` — ambient capture. Disambiguated by
  `capture-channels[*].kind` ∈ `{mic, camera, screen}`. Affordances:
  `enable`, `disable`, `subscribe-digest`. Signals: level / frames /
  transcript / OCR-text; raw frames only under positive `raw` consent.

Each wrapper MUST emit at least the `presence-signal` on every
observation tick (default 1 Hz), and MUST participate in
`observation` with `kind = "ambient"` using interpretation digests
unless an active `emits = "raw"` consent is in force.

### 7.4 Unknown primitive fallback

Per §14, a renderer that receives a `primitive-head` with a `type`
it does not support MUST render the primitive as a
`foreign-surface` with:

- `wrapper-type = "foreign://app"`,
- `identity-hint.bundle = <the unknown type IRI>`,
- `shell-state.health = "ready"`,
- `capture-channels = []`,

and preserve the head's `affordances` and `variant-id`. This is the
Adaptive Cards `fallbackText` pattern lifted into the canon.

---

## 8. Surface tree model (state-diff)

### 8.1 Composition and updates

A surface is a **state-diff** object. The initial snapshot is a
`surface-spec`; updates are ordered patch lists.

```cddl
surface-spec = {
  id            : surface-id,
  version       : uint,                  ; starts at 1
  persona       : ontology-uri,
  mode          : "desktop" / "mission-console" / "embedded-dev",
  governance    : {
    project     : ontology-uri,
    goals       : [* goal-id],
    constraints : [* ontology-uri],
  },
  context       : ontology-uri,          ; bounded-context frame
  root          : primitive-head,
  subscriptions : [* { id: subscription-id, uri: ontology-uri, ? filter: any }],
}

op = { kind: "add" / "remove" / "replace",
       path: tstr,                        ; JSON-Pointer
       ? value: primitive-head / any }
```

### 8.2 Versioning

- `surface_version` is a per-surface monotonically increasing `uint`,
  starting at 1.
- Every `surface.update` SHALL increment `surface_version` by exactly 1.
- An `op.path` targeting a field not present on the primitive's
  declared schema is **malformed**; the kernel MUST reject the whole
  op list (all-or-nothing).
- An `op` that replaces a primitive's `type` is permitted but
  MUST include a fresh `variant-id`; the kernel MUST NOT reuse
  variant ids across type changes.

### 8.3 Reconciliation rules

A renderer:

1. Tracks the current `surface_version` per surface-id.
2. On receiving a `surface.update` notification where
   `base_version == current_version`, applies ops in declared order
   and advances `current_version`.
3. On mismatch (`base_version != current_version`), MUST discard the
   update and issue `surface.get(surface_id)`.
4. MUST NOT apply ops partially; an op that fails applies as a
   no-op and the renderer MUST issue `surface.get`.
5. On `surface.dispose`, MUST retain a tombstone for the duration
   of any outstanding observations (§9.6).

### 8.4 Unknown primitive type fallback

A renderer encountering a `primitive-head.type` it does not support
substitutes a `foreign-surface` per §7.4. The substitution is
local to the renderer; the composer sees no change.

### 8.5 Shared surfaces

A surface MAY be rendered by multiple renderers simultaneously
(Mission Console). The composer is authoritative. The kernel:

- Serialises all `surface.update` calls globally per surface-id.
- Broadcasts each accepted update as a `surface.update` notification
  to every subscribed renderer.
- Rejects concurrent structural edits from non-composer participants
  with `-32013 GatingDenied`.

Composition edits and payload edits travel on different channels.
Payload edits inside a leaf primitive (e.g. concurrent text editing
in a `Field`) MAY use embedded CRDT state transmitted inside the
primitive's `state` field; WSP does not define the CRDT shape —
that is a concern of the primitive's ontology shape.

---

## 9. Observation tapestry (event-sourced)

Observations are the event-sourced half of WSP, carrying the
active-radar return signal, substrate state deltas, ambient wrapper
signals, and governance events.

### 9.1 Observation structure

```cddl
observation = [
  subscription-id,
  uint,                 ; seq — monotonic within subscription
  tdate,                ; wall clock
  observation-kind,
  observation-body,
]

observation-kind = "topology" / "doppler" / "range" / "bearing"
                 / "explicit" / "implicit" / "ambient"
                 / "state" / "consent" / "governance"
```

`observation-kind` is an **open string**: future kinds are added by
ontology URI without changing the wire. Consumers that do not
understand a kind MUST skip it and continue.

### 9.2 Canonical bodies

```cddl
topology   = { surface-id, path, dwell-ms: uint, order-index: uint,
               primitive-ref: primitive-ref }
doppler    = { surface-id, path, velocity: float64,
               direction: "forward" / "backward",
               primitive-ref: primitive-ref }
range      = { surface-id, path, latency-ms: uint,
               primitive-ref: primitive-ref }
bearing    = { surface-id, path, affordance: tstr, actor: actor-id,
               primitive-ref: primitive-ref }
explicit   = { surface-id, path, affordance: tstr, args: any,
               confidence: confidence,
               primitive-ref: primitive-ref }
implicit   = { surface-id, path, signal: tstr, value: any,
               primitive-ref: primitive-ref }
ambient    = { wrapper: ontology-uri, channel: tstr, digest: tstr,
               ? raw-ref: tstr }
state      = { uri: ontology-uri, value: any,
               confidence: confidence }
consent    = { consent-id: tstr, event: "granted" / "revoked" / "expired",
               scope: ontology-uri, actor: actor-id }
governance = { decision: tstr, rule: ontology-uri, subject: ontology-uri,
               goal-id: goal-id, effect: [* { axis: tstr, v: float64 }] }
```

### 9.3 Delivery

- `observation` frames are delivered as `substrate.update` or
  `observation.update` notifications (depending on whether the
  stream was opened via `subscribe` or `observe`).
- `seq` is per-stream monotonic and MUST increase by exactly 1 per
  delivered observation. Gaps signal drop or coalesce; consumers
  MAY request replay via `since_seq`.

### 9.4 Variant attribution

Every observation carrying a `primitive-ref` MUST set
`primitive-ref.variant` to the `variant-id` the pulse was emitted
with. ECC and GEPA require this for attribution. The kernel MUST
NOT pre-aggregate observations; aggregation is the consumer's job.

### 9.5 Back-pressure

The renderer MAY declare per-channel `max_inflight` in
`client-caps.capability-flags` (e.g. `"wsp.bp.topology=256"`). When
back-pressure triggers, the kernel SHALL:

- For drop-tolerant channels (`topology`, `doppler`, `range`,
  `bearing`, `implicit`, `ambient`): drop the oldest outstanding
  frame and increment a `drop-count` reported in a synthetic
  `observation` with `kind = "governance"` and
  `decision = "bp-drop"`.
- For drop-intolerant channels (`explicit`, `consent`,
  `governance`, `state`): queue without limit and apply flow control
  by stalling the source; the kernel SHALL emit a
  `governance.backpressure` chain event when stalled for more than
  1 second.

### 9.6 Tombstones

A disposed surface leaves a tombstone valid for the longer of (a)
the maximum outstanding observation's reconnection window or (b)
60 seconds. During the tombstone window, observation replay MAY
include the disposed primitive's identity in `primitive-ref`.
After the window, the kernel MAY garbage-collect the tombstone.

### 9.7 Causal collapse cascade wiring

The ECC subsystem consumes observation streams directly. WSP does
not mutate or transform observations on behalf of ECC. The
`digital-exhaust-as-intent` framing means observations are the
primary product of the renderer, not a by-product.

---

## 10. Subscription model

### 10.1 Addressing

Subscriptions address substrate resources by `ontology-uri`. Examples:

- `resource://project/store-47/orders/live` — a resource-tree node.
- `mesh://peers/active` — the active mesh peer set.
- `chain://events/governance.vote` — a chain event kind.
- `service://weaver/tasks` — a service-registry slice.
- `topic://mesh.leaf.$pubkey.announce` — a mesh topic.

Every conforming kernel MUST support at minimum:

- `resource://...` — the resource tree (existing substrate).
- `service://...` — the service registry (existing substrate).
- `chain://...` — ExoChain event streams (existing substrate).

Additional URI schemes (e.g. `topic://`, `mesh://`) are advertised
in `server-caps.supported-topics`.

### 10.2 Lifecycle

- A subscription is created by `subscribe` (§5.3.1). The kernel
  returns a `subscription-id`.
- Observations stream on that id until `unsubscribe` (§5.3.2) or
  session shutdown.
- A subscription MAY be **bound** into a primitive field (e.g.
  `Gauge.bind`, `Plot.bind`, `StreamView.feed`). Binding does not
  duplicate the subscription — multiple primitives MAY share one
  subscription-id.
- The composer is responsible for creating subscriptions and
  embedding their ids in primitive specs. The kernel MUST NOT
  auto-subscribe on behalf of a composer.

### 10.3 Resource-tree paths

Resource-tree URIs follow the hierarchical path model of the
kernel's resource tree. A filter MAY be supplied to narrow the
subscription (e.g. `filter = { kind: "orders", status: "open" }`).
The kernel MUST NOT expand the filter shape beyond what the
resource type declares.

### 10.4 Chain-event bindings

A subscription to `chain://events/<kind>` receives `observation`
frames with `kind = "state"` whose `body.uri` is the event IRI and
`body.value` is the event payload. Chain subscriptions are
append-only and replayable from any historical `seq`.

---

## 11. Goal, consent, and governance

Goals are the constitutional frame against which every verb is
adjudicated.

### 11.1 Mandatory `goal_id`

Every WSP request (not every notification) MUST carry a `goal_id`
envelope field. The kernel:

1. Resolves the goal from the substrate at routing-gate time.
2. Checks the call's computed affordance against
   `goal.granted_affordances` minus `goal.denied_affordances`.
3. Checks the computed effect vector against `goal.effect_ceiling`.
4. On deny, returns `-32013 GatingDenied` with structured reason.

Requests with `goal_id == null` MUST be rejected at the routing gate
with `-32011 GoalMissing`. The default session goal is `adhoc-scratch`,
which is permissive but chain-recorded and visible in the tray
(§11.4).

### 11.2 `gate.check` pre-flight

Composers SHOULD call `gate.check` before offering an affordance so
that enumerated affordances are honest (predicate 2). A composer
MUST NOT publish an affordance on a primitive head that `gate.check`
would deny for the current caller.

### 11.3 `consent.request` and `consent.revoke`

A `consent.request` verb:

1. Composes a consent-flow surface (a `Modal` with `Field` and
   `Pressable` children) that the kernel presents to the user.
2. Returns a `consent_id` once the user approves or denies.
3. Chain-logs the decision under `governance.goal.consent.grant`
   or `.revoke`.

A `consent.revoke` verb cascades: every capture channel bound to
the consent-id ceases immediately and the associated tray chip
updates.

### 11.4 Capture-chip auto-composition

Whenever any `foreign-surface` on any surface owned by a session
declares a non-empty `capture-channels` list, the kernel MUST
auto-compose a persistent `Chip` with `kind = "capture"` into the
kernel-owned tray surface for that session. The chip:

- Is composed by the kernel, not by the agent.
- Lists every live capture channel and its consent-id.
- Is NOT removable by the agent; only by `consent.revoke`.
- Has a `dismiss` affordance that delegates to `consent.revoke`.

A conforming renderer MUST render this chip prominently whenever it
is present. This is the **wire-level** enforcement of the
foundations.md non-negotiable "Observable to the user". A renderer
that refuses to render the capture chip is non-conformant.

### 11.5 Privacy-flags enforcement

The kernel enforces the `privacy-flags` declarations at the boundary:

1. `captured = true` without a valid `consent-id` is malformed
   (§6.4).
2. `retained = true` implies chain-logged retention at observation
   time; `ambient` observations with `raw-ref` set without
   retention consent are malformed.
3. `off-node = true` is gated at mesh send time; the mesh transport
   MUST NOT send an outgoing `MeshIpcEnvelope::Wsp` frame whose
   top-level `surface-spec` contains any primitive with
   `off-node = false`.

---

## 12. Mutation and GEPA

### 12.1 `mutate` verb

`mutate` (§5.5.2) changes the variant-id of a primitive along a
declared axis. Parameters:

- `surface_id` — the surface.
- `path` — JSON-Pointer to the primitive.
- `axis` — the axis name (MUST be listed in the primitive head's
  `mutation-axes`).
- `new_variant` — the new variant-id.

Every `mutate` call is gate-checked against the active goal before
it applies. Governance denials return `-32013 GatingDenied` with
`policy-uri` indicating the binding rule.

### 12.2 Legal mutation axes

The following axes are legal to declare and to mutate:

- `ordering` — visual order of children.
- `density` — visual density (compact / comfortable / dense).
- `copy` — textual content of labels, titles, captions.
- `timing` — animation durations, reveal delays.
- `granularity` — step size (sliders, plots), column count (grids),
  sample density (plots).
- `modality` — split between modalities (text vs voice vs icon;
  linear vs radial).
- `placement` — position hints within the parent.
- `icon` — icon choice within a declared set.
- `tint` — non-brand tint.
- `style` — style variant within a declared set (`primary` vs
  `secondary`).

### 12.3 Forbidden axes

The following axes MUST NOT be declared in any `mutation-axes` list
and MUST NOT be mutated under any circumstance:

- `consent` — consent flows (including `Modal` consent affordances).
- `safety` — `confirm` / `cancel` / `dismiss` on `Modal`;
  destructive-verb warnings.
- `brand` — brand marks on `Media`; logos.

The kernel MUST reject frames that declare any of these axes at
composition time (§6.4), and MUST reject `mutate` calls targeting
them with `-32022 AxisFrozen`.

### 12.4 Return-signal attribution

Every observation emitted by a primitive MUST carry the
`primitive-ref.variant` value matching the primitive head's
current `variant-id`. ECC + GEPA consume this stream and attribute
returns to pulses without requiring shared state.

---

## 13. Participant model

### 13.1 Session identity

Every session has:

- A stable `session-id` (assigned by the kernel at `initialize`).
- A `persona` (declared by the client).
- An `actor-id` (derived from the underlying transport auth: UDS peer
  creds, mesh Noise pubkey, or WebSocket auth token).

### 13.2 Human / agent parity

WSP does not distinguish humans from agents at the verb level. Both
call `invoke`, `mutate`, etc. The `actor-id` field carries the
identity; whether that identity is human or agent is the governance
layer's concern.

### 13.3 Seat assignment (Mission Console)

A surface with `mode = "mission-console"` MAY have multiple
participants. Each participant is assigned a **seat**:

```cddl
seat = {
  actor-id,
  role             : ontology-uri,        ; e.g. "seat://counsel", "seat://observer"
  affordance-filter: [* tstr],            ; allowed affordance names
  variant          : variant-id,          ; the variant this seat sees
  presence         : {
    focus-path     : tstr,                ; JSON-Pointer into surface
    attention      : "focused" / "idle" / "away",
    speaking       : bool,
  }
}
```

Seats are declared at composition time and MAY be updated via
`surface.update` ops on the `seats` array. Presence is reported via
`observation` frames with `kind = "implicit"` and
`body.signal = "presence"`.

### 13.4 Witness events

Every `invoke`, `mutate`, `consent.request`, `consent.revoke`, and
surface lifecycle event on a shared surface is a witness event and
MUST be chain-logged. All participants on the surface MUST receive
an observation with `kind = "governance"` summarising the event.

---

## 14. Versioning

### 14.1 Wire version

A single integer, negotiated at `session.initialize`. This document
is `wsp_version = 1`. A kernel SHALL reject any session attempting
a version it does not support with error `-32002`.

### 14.2 Capability flags

Every new optional feature ships as a string flag in `capability-flags`.
Unknown flags MUST be ignored. Capability flags in common use:

- `wsp.cbor` — CBOR encoding on WebSocket transport.
- `wsp.reattach` — session reattachment supported.
- `wsp.bp.<channel>=<n>` — per-channel back-pressure limit.
- `wsp.goals.v1` — full Goal schema supported (§11).
- `wsp.seats.v1` — Mission Console seat model supported.

### 14.3 Ontology URI stability

Ontology URIs are **stable-or-break**. Once published, a URI's
meaning MUST NOT change. New behaviour arrives as a new URI. A
kernel MAY deprecate a URI via the `deprecations` field in
`server-caps`; deprecation does not remove the URI, it signals a
preferred replacement.

### 14.4 Forward-compatibility

- Older renderers that encounter a `primitive-head.type` they do not
  support MUST render a `foreign-surface` fallback per §7.4.
- Older consumers that encounter an unknown `observation-kind` MUST
  skip and continue.
- New fields on any CDDL map MUST be additive; consumers MUST
  ignore unknown fields.

---

## 15. Error taxonomy

WSP uses JSON-RPC 2.0 error codes. Standard JSON-RPC errors retain
their meanings:

| Code    | Name             | Meaning                                  |
|---------|------------------|------------------------------------------|
| -32700  | ParseError       | Invalid JSON or CBOR.                    |
| -32600  | InvalidRequest   | Malformed request envelope.              |
| -32601  | MethodNotFound   | Unknown verb.                            |
| -32602  | InvalidParams    | Params do not match the verb's schema.   |
| -32603  | InternalError    | Implementation defect.                   |

WSP-specific error codes (-32000 to -32099 range):

| Code    | Name             | Meaning                                  |
|---------|------------------|------------------------------------------|
| -32002  | VersionUnsupported | `wsp-version` not mutually supported.  |
| -32010  | Malformed        | Head validation (§6.3) failed.           |
| -32011  | GoalMissing      | `goal_id` absent or unresolved.          |
| -32012  | ContextMissing   | `context` absent or unrecognised.        |
| -32013  | GatingDenied     | Governance denied the call.              |
| -32014  | SurfaceUnknown   | `surface_id` not found.                  |
| -32015  | VersionMismatch  | `base_version != current_version`.       |
| -32016  | TopicUnknown     | Subscription URI scheme unknown.         |
| -32020  | ConsentRequired  | Call requires unissued consent.          |
| -32021  | AffordanceUnknown| Affordance not declared on the head.     |
| -32022  | AxisFrozen       | Mutation axis is frozen.                 |
| -32023  | AxisUnknown      | Mutation axis not declared.              |
| -32030  | SessionClosed    | Session has been shut down.              |
| -32031  | RateLimited      | Back-pressure refusal.                   |

Every error MUST carry a structured `data` field:

```cddl
error-data = {
  reason           : tstr,
  ? policy-uri     : ontology-uri,
  ? elicit         : ontology-uri,
  ? detail         : any,
}
```

`elicit` indicates a follow-up verb (typically `consent.request`)
that might allow the caller to retry successfully.

---

## 16. Security considerations

### 16.1 Capture privacy

The privacy non-negotiables of `foundations.md` are enforced at the
wire level, not by renderer discipline:

1. **Per-user, never telemetry.** Observation data lives in the
   user's own chain-backed substrate. `off-node = true` requires
   positive consent.
2. **Governance-gated by default.** Every capture wrapper is
   gate-checked per §11; a malformed capture frame is rejected at
   the kernel boundary.
3. **Signal lossy by policy.** `ambient` observations default to
   interpretation digests. Raw bytes require `emits = "raw"` and an
   active consent-id.
4. **Observable to the user.** The capture chip (§11.4) is
   auto-composed by the kernel. A renderer that does not render it
   is non-conformant.

### 16.2 Cross-substrate identity (unsolved)

`actor-id` is opaque. Its format across substrates (did:key, ExoChain
pubkey, external OIDC) is not resolved in this version. See §18.

### 16.3 Mutation on sensitive surfaces

Consent flows, safety affordances (`confirm`, `cancel`, `dismiss`),
and brand marks MUST NOT be mutable (§12.3). The kernel MUST reject
any composition or `mutate` touching these at the boundary.

### 16.4 Transport auth

WSP inherits transport auth:

- UDS: SO_PEERCRED-derived `actor-id`.
- Mesh: Noise handshake pubkey.
- WebSocket: application-defined bearer or mTLS.

WSP does not define its own cryptographic identity; it consumes the
identity the transport provides.

### 16.5 Replay

Observation streams are replayable from `since_seq`. A replay is
subject to the same governance gating as the live stream; a consent
revoked between observation time and replay time MUST cause the
replay to elide or redact observations bound to that consent.

---

## 17. Compliance tests

A conformant implementation MUST pass the following tests. Each test
is described in operational terms; a dedicated test harness will
encode them in a later revision.

### 17.1 Handshake test

1. Open a session with `wsp-version = 1`.
2. Assert the server responds with `wsp-version = 1`.
3. Open with `wsp-version = 999`.
4. Assert the server fails with `-32002 VersionUnsupported`.

### 17.2 Primitive round-trip test

1. Compose a surface containing one `Pressable`, one `Field`, and
   one `Chip`.
2. Assert the kernel accepts it and returns `surface_version = 1`.
3. Issue `surface.get` and assert the returned spec matches byte-for-byte
   (within encoding-equivalence rules).
4. Issue `surface.update` adding a second `Pressable`; assert
   `new_version = 2`.
5. Issue `surface.get` with stale `base_version = 1` via mismatch
   simulation; assert `-32015 VersionMismatch`.

### 17.3 Subscription test

1. Open a subscription to `resource://test/scalar`.
2. Assert `observation` frames arrive with monotonically-increasing
   `seq`.
3. Disconnect mid-stream.
4. Reconnect and issue `subscribe` with `since_seq`; assert the
   replay begins at the expected sequence.

### 17.4 Observation test

1. Compose a surface with a `Pressable` and declare `variant-id =
   "v1"`.
2. Open `observe` with channels `["bearing", "explicit"]`.
3. Simulate a user click on the pressable.
4. Assert a `bearing` observation arrives with `primitive-ref.variant
   = "v1"` and `affordance` matching the declared name.
5. Assert an `explicit` observation arrives with matching `variant`.

### 17.5 Governance denial test

1. Compose a surface with a `ForeignSurface(foreign://terminal)`
   whose `capture-channels` contains a mic channel without a
   `consent-id`.
2. Assert the composition is rejected with `-32010 Malformed`.
3. Issue `consent.request` for the scope; obtain `consent_id`.
4. Recompose with the `consent-id` populated.
5. Assert the composition succeeds and a capture `Chip` is
   auto-composed into the tray surface.
6. Issue `consent.revoke`; assert the capture channel terminates
   and the tray chip updates.

### 17.6 Fallback test

1. Compose a surface with a `primitive-head.type =
   "ui://experimental.futuregauge"` unknown to the renderer.
2. Assert the renderer renders a `foreign-surface` with
   `wrapper-type = "foreign://app"` and
   `identity-hint.bundle = "ui://experimental.futuregauge"`.
3. Assert the primitive's `affordances` and `variant-id` survive
   the fallback.

### 17.7 Mutation test

1. Compose a `Pressable` with `mutation-axes = [{ name: "copy",
   kind: "discrete" }]` and `variant-id = "v1"`.
2. Issue `mutate` on axis `copy` with `new_variant = "v2"`.
3. Assert the primitive's `variant-id` updates to `v2` and
   subsequent observations carry it.
4. Issue `mutate` on axis `safety`; assert `-32022 AxisFrozen`.

### 17.8 Capture chip auto-composition test

1. Compose a `ForeignSurface(foreign://camera-mic-screen)` with an
   active consent.
2. Assert the kernel-owned tray surface contains a `Chip` with
   `kind = "capture"` referencing the consent-id.
3. Revoke the consent.
4. Assert the chip is removed from the tray.

---

## 18. Open questions (Round 3)

This section enumerates known gaps. Round 3 of the symposium MUST
address them before WSP-0.2 is frozen.

### 18.1 Cross-substrate identity

`actor-id` is a string today. `did:key` is tentative. The spec does
not define how identity delegates across substrates (a signed
delegation proof that the receiving substrate's governance
verifies). This is Rasmus's unsolved problem #1 and is scheduled
for joint resolution with the ExoChain and ADR-009 tracks.

### 18.2 Observation back-pressure beyond drop/queue

The current model (§9.5) handles drop-tolerant vs drop-intolerant
channels. A richer model would allow **coalescing** (merge two
pending `topology` updates into one), **degrading** (reduce sample
rate of ambient digests), and **prioritising** (governance ahead of
implicit). Round 3 should propose a typed back-pressure policy
schema.

### 18.3 Mission Console projection negotiation

A single composer pushing to N renderers with potentially divergent
affordances per seat (§13.3) is supported by the seat model, but
the negotiation mechanism for "this seat sees variant A, that seat
sees variant B" is not specified. Proposal: a `Projection` object
per renderer selecting an actor-filtered affordance view; pending
Round 3.

### 18.4 Ontology manifest delivery

`session.initialize` returns `supported-primitives` but does not
ship the ontology manifest itself. Should the kernel pre-ship a
manifest at initialize time, or should consumers use
`ontology.describe` on first use? Lazy fetch is currently implied;
pre-ship would reduce first-render latency.

### 18.5 RVF + WSP relationship

WSP frames could be carried as `MessagePayload::Rvf { segment_type:
"wsp", data }` instead of a dedicated `MessagePayload::Wsp`. This
would let RVF signing and replay protection apply uniformly. Needs
alignment with the RVF WG.

### 18.6 CBOR-over-WebSocket for observation streams

JSON is the WebSocket default. CBOR would halve bytes for
observation streams and is likely worth the complexity for Mission
Console. Needs a perf test, not a guess.

---

## 19. Appendix A — CDDL source

The following is the concatenated CDDL source for WSP-0.1. It is
authoritative; the JSON and CBOR encodings are derived from it per
§3.4.

```cddl
; ═══════════════════════════════════════════════════════════════
; WSP-0.1 — WeftOS Surface Protocol
; Concatenated CDDL. RFC 8610.
; ═══════════════════════════════════════════════════════════════

; ── A.1 Foundational types ─────────────────────────────────────

ontology-uri    = tstr .regexp "^[a-z]+://[a-zA-Z0-9._/\\-#]+$"
actor-id        = tstr
variant-id      = tstr
surface-id      = tstr
subscription-id = tstr
goal-id         = tstr

; ── A.2 Confidence and privacy ─────────────────────────────────

confidence = {
  source     : "deterministic" / "inference" / "cache" / "input",
  ? value    : float64,
  ? interval : [float64, float64],
  ? as-of    : tdate,
}

privacy-flags = {
  captured   : bool,
  retained   : bool,
  off-node   : bool,
  consent-id : tstr / null,
}

; ── A.3 Affordances and mutation ───────────────────────────────

affordance = {
  name         : tstr,
  verb         : "read" / "write" / "invoke" / "subscribe" / "dismiss",
  ? actors     : [* actor-id],
  ? args-schema: ontology-uri,
}

mutation-axis = {
  name         : tstr,
  kind         : "discrete" / "continuous" / "categorical",
  ? domain     : any,
  frozen-by    : [* "brand" / "safety" / "consent" / "user-request"],
}

; ── A.4 Primitive head ─────────────────────────────────────────

primitive-head = {
  type           : ontology-uri,
  path           : tstr,
  state          : any,
  affordances    : [* affordance],
  confidence     : confidence,
  variant-id     : variant-id,
  mutation-axes  : [* mutation-axis],
  privacy-flags  : privacy-flags,
}

primitive-ref = {
  iri          : ontology-uri,
  instance-iri : ontology-uri,
  cid          : tstr,
  context      : ontology-uri,
  shape        : ontology-uri,
  variant      : variant-id,
}

; ── A.5 Tier-A primitives ──────────────────────────────────────

chip       = { primitive-head,
  label : tstr,
  kind  : "consent" / "capture" / "status" / "identity" / "goal",
  ? icon : ontology-uri,
  ? tint : tstr,
}

pressable  = { primitive-head,
  label : tstr,
  ? icon   : ontology-uri,
  ? hotkey : tstr,
  ? voice  : [* tstr],
  ? style  : "primary" / "secondary" / "ghost" / "destructive",
}

field      = { primitive-head,
  name   : tstr,
  schema : ontology-uri,
  value  : any,
  dirty  : bool,
  ? errors      : [* tstr],
  ? placeholder : tstr,
}

toggle     = { primitive-head,
  name  : tstr,
  value : bool,
  ? label : tstr,
}

select     = { primitive-head,
  name     : tstr,
  choices  : [* { value: any, label: tstr, ? disabled: bool }],
  selected : any,
  ? multiple : bool,
}

slider     = { primitive-head,
  name  : tstr,
  value : float64,
  range : [float64, float64],
  ? step : float64,
  ? unit : tstr,
}

stack      = { primitive-head,
  axis   : "row" / "column",
  ? gap   : float64,
  ? align : "start" / "center" / "end" / "stretch",
  children : [* primitive-head],
}

grid       = { primitive-head,
  columns : uint,
  ? gap    : [float64, float64],
  children : [* primitive-head],
}

strip      = { primitive-head,
  orientation : "horizontal" / "vertical",
  sizes       : [* size],
  children    : [* primitive-head],
}

size = "remainder" / "relative" / "exact"
     / { kind: tstr, v: float64 }

dock       = { primitive-head,
  layout : any,
  tabs   : [* { id: tstr, title: tstr, surface: surface-id }],
  focus  : tstr,
}

sheet      = { primitive-head,
  ? stick-to-bottom : bool,
  ? sticky-header   : primitive-head,
  children : [* primitive-head],
}

modal      = { primitive-head,
  title : tstr,
  body  : primitive-head,
}

table      = { primitive-head,
  columns : [* { name: tstr, schema: ontology-uri,
                 ? sort: "asc" / "desc" / "none" }],
  rows    : [* { cells: [* any], ? id: tstr }],
  ? selection : [* tstr],
  ? bind      : subscription-id,
}

tree       = { primitive-head,
  nodes : [* tree-node],
}

tree-node = {
  id    : tstr,
  label : tstr,
  ? children : [* tree-node],
  ? expanded : bool,
}

gauge      = { primitive-head,
  value : float64,
  range : [float64, float64],
  unit  : tstr,
  ? thresholds : [* { at: float64, label: tstr, tint: tstr }],
  ? bind       : subscription-id,
  ? style      : "linear" / "radial",
}

plot       = { primitive-head,
  series : [* { name: tstr, schema: ontology-uri,
                samples: [* [float64, float64]] }],
  ? window  : [float64, float64],
  ? bind    : subscription-id,
  ? density : "sparse" / "normal" / "dense",
}

media      = { primitive-head,
  source : ontology-uri,
  ? alt  : tstr,
  ? dims : [uint, uint],
}

stream-view = { primitive-head,
  feed   : subscription-id,
  cursor : "live" / uint,
  ? density    : "compact" / "comfortable" / "dense",
  ? row-schema : ontology-uri,
}

; ── A.6 Tier-B — ForeignSurface ────────────────────────────────

foreign-surface = { primitive-head,
  wrapper-type     : ontology-uri,
  identity-hint    : {
    ? url          : tstr,
    ? bundle       : tstr,
    ? pid          : uint,
    ? media-id     : tstr,
  },
  shell-state      : {
    opened-at      : tdate,
    focus          : bool,
    size           : [uint, uint],
    ? last-url     : tstr,
    ? exit-code    : int,
    health         : "starting" / "ready" / "busy" / "crashed" / "closed",
  },
  presence-signal  : {
    active         : bool,
    attention      : "foreground" / "background" / "hidden",
    activity-rate  : float64,
  },
  capture-channels : [* capture-channel],
}

capture-channel = {
  kind             : "mic" / "camera" / "screen" / "clipboard" / "a11y-tree",
  emits            : "interpretation" / "raw",
  consent-id       : tstr,
  ? sample-rate-hz : float64,
}

; ── A.7 Surface root ───────────────────────────────────────────

surface-spec = {
  id            : surface-id,
  version       : uint,
  persona       : ontology-uri,
  mode          : "desktop" / "mission-console" / "embedded-dev",
  governance    : {
    project     : ontology-uri,
    goals       : [* goal-id],
    constraints : [* ontology-uri],
  },
  context       : ontology-uri,
  root          : primitive-head,
  subscriptions : [* { id: subscription-id, uri: ontology-uri, ? filter: any }],
  ? seats       : [* seat],
}

seat = {
  actor-id,
  role              : ontology-uri,
  affordance-filter : [* tstr],
  variant           : variant-id,
  presence          : {
    focus-path : tstr,
    attention  : "focused" / "idle" / "away",
    speaking   : bool,
  }
}

op = {
  kind  : "add" / "remove" / "replace",
  path  : tstr,
  ? value : primitive-head / any,
}

; ── A.8 Observation tapestry ───────────────────────────────────

observation = [
  subscription-id,
  uint,
  tdate,
  observation-kind,
  observation-body,
]

observation-kind = "topology" / "doppler" / "range" / "bearing"
                 / "explicit" / "implicit" / "ambient"
                 / "state" / "consent" / "governance" / tstr

topology   = { surface-id, path: tstr, dwell-ms: uint,
               order-index: uint, primitive-ref: primitive-ref }
doppler    = { surface-id, path: tstr, velocity: float64,
               direction: "forward" / "backward",
               primitive-ref: primitive-ref }
range      = { surface-id, path: tstr, latency-ms: uint,
               primitive-ref: primitive-ref }
bearing    = { surface-id, path: tstr, affordance: tstr,
               actor: actor-id, primitive-ref: primitive-ref }
explicit   = { surface-id, path: tstr, affordance: tstr,
               args: any, confidence: confidence,
               primitive-ref: primitive-ref }
implicit   = { surface-id, path: tstr, signal: tstr,
               value: any, primitive-ref: primitive-ref }
ambient    = { wrapper: ontology-uri, channel: tstr,
               digest: tstr, ? raw-ref: tstr }
state      = { uri: ontology-uri, value: any,
               confidence: confidence }
consent    = { consent-id: tstr,
               event: "granted" / "revoked" / "expired",
               scope: ontology-uri, actor: actor-id }
governance = { decision: tstr, rule: ontology-uri,
               subject: ontology-uri, goal-id: goal-id,
               effect: [* { axis: tstr, v: float64 }] }

observation-body = topology / doppler / range / bearing
                 / explicit / implicit / ambient / state
                 / consent / governance
                 / { * tstr => any }

; ── A.9 Session handshake ──────────────────────────────────────

client-caps = {
  wsp-version          : uint,
  persona              : ontology-uri,
  locale               : tstr,
  supported-encodings  : [* "json" / "cbor"],
  supported-primitives : [* ontology-uri],
  supported-wrappers   : [* ontology-uri],
  supported-topics     : [* ontology-uri],
  a11y-hints           : { * tstr => any },
  ? cbor-tag-profile   : uint,
  ? initial-goal       : goal-id,
  ? reattach           : tstr,
}

server-caps = {
  wsp-version          : uint,
  encoding             : "json" / "cbor",
  supported-primitives : [* ontology-uri],
  supported-wrappers   : [* ontology-uri],
  supported-topics     : [* ontology-uri],
  capability-flags     : [* tstr],
  session-id           : tstr,
  ? deprecations       : [* { uri: ontology-uri, replaced-by: ontology-uri }],
}

; ── A.10 Errors ────────────────────────────────────────────────

error-data = {
  reason       : tstr,
  ? policy-uri : ontology-uri,
  ? elicit     : ontology-uri,
  ? detail     : any,
}

; ── A.11 Envelopes (JSON-RPC 2.0 + Notification) ───────────────

request = {
  jsonrpc : "2.0",
  id      : uint / tstr,
  method  : tstr,
  params  : any,
  ; WSP envelope fields
  goal_id : goal-id,
  ? variant_id : variant-id,
}

response = {
  jsonrpc : "2.0",
  id      : uint / tstr,
  ? result: any,
  ? error : {
    code    : int,
    message : tstr,
    ? data  : error-data,
  }
}

notification = {
  jsonrpc : "2.0",
  method  : tstr,
  params  : any,
}

; ── A.12 Integer-key profile (CBOR hot path, informative) ──────
; When cbor-tag-profile = 1 is declared, a renderer MAY encode the
; envelope with integer keys:
;   0 = jsonrpc, 1 = id, 2 = method, 3 = params,
;   4 = result, 5 = error, 6 = goal_id, 7 = variant_id
; Receivers MUST accept either form. String keys are always valid.
```

---

## 20. Appendix B — Example flows

Three complete example traces illustrating typical interactions.
Each trace shows bytes on the wire; JSON is used for clarity. CBOR
equivalents follow the encoding rule of §3.4.

### B.1 Compose a `Field` and receive the composed surface

**Context**: A dev-panel composer wants to render a single `Field`
for the user to enter a migration target path.

**Frame 1**: renderer → server (request) — `session.initialize`.

```json
{
  "jsonrpc": "2.0", "id": 1, "method": "session.initialize",
  "params": {
    "wsp-version": 1,
    "persona": "persona://dev-panel",
    "locale": "en-US",
    "supported-encodings": ["json"],
    "supported-primitives": [
      "ui://pressable", "ui://field", "ui://chip",
      "ui://stack", "ui://modal"
    ],
    "supported-wrappers": [],
    "supported-topics": ["resource://", "service://"],
    "a11y-hints": {},
    "initial-goal": "goal:migration-dry-run-a1b2c3"
  }
}
```

**Frame 2**: server → renderer (response).

```json
{
  "jsonrpc": "2.0", "id": 1,
  "result": {
    "wsp-version": 1, "encoding": "json",
    "supported-primitives": [
      "ui://pressable", "ui://field", "ui://chip",
      "ui://stack", "ui://modal", "ui://sheet"
    ],
    "supported-wrappers": ["foreign://browser", "foreign://terminal"],
    "supported-topics": ["resource://", "service://", "chain://"],
    "capability-flags": ["wsp.reattach", "wsp.goals.v1"],
    "session-id": "sess:5f32"
  }
}
```

**Frame 3**: composer → server (request) — `surface.compose`.

```json
{
  "jsonrpc": "2.0", "id": 2, "method": "surface.compose",
  "goal_id": "goal:migration-dry-run-a1b2c3",
  "variant_id": "v1",
  "params": {
    "id": "surf:42",
    "version": 1,
    "persona": "persona://dev-panel",
    "mode": "embedded-dev",
    "governance": {
      "project": "project://weftos",
      "goals": ["goal:migration-dry-run-a1b2c3"],
      "constraints": []
    },
    "context": "context://weftos/migration",
    "root": {
      "type": "ui://field",
      "path": "/",
      "state": { "value": "" },
      "affordances": [
        { "name": "write", "verb": "write", "args-schema": "schema://string" }
      ],
      "confidence": { "source": "input" },
      "variant-id": "v1",
      "mutation-axes": [],
      "privacy-flags": {
        "captured": false, "retained": false,
        "off-node": false, "consent-id": null
      },
      "name": "target-path",
      "schema": "schema://filesystem-path",
      "value": "",
      "dirty": false
    },
    "subscriptions": []
  }
}
```

**Frame 4**: server → composer (response).

```json
{ "jsonrpc": "2.0", "id": 2,
  "result": { "surface_id": "surf:42", "surface_version": 1 } }
```

### B.2 Observe a click on a `Pressable`

**Context**: The surface from B.1 includes a `Pressable("Run dry
run")` as a sibling of the `Field`. The renderer opens an
observation stream and the user clicks.

**Frame 1**: renderer → server (request) — `observe`.

```json
{
  "jsonrpc": "2.0", "id": 3, "method": "observe",
  "goal_id": "goal:migration-dry-run-a1b2c3",
  "params": {
    "surface_id": "surf:42",
    "channels": ["bearing", "explicit", "range"]
  }
}
```

**Frame 2**: server → renderer (response).

```json
{ "jsonrpc": "2.0", "id": 3,
  "result": { "observation_id": "obs:7d" } }
```

**Frame 3**: renderer → server (notification) — `observation.update`
carrying a `range` observation (user hovered for 812 ms before
clicking):

```json
{
  "jsonrpc": "2.0", "method": "observation.update",
  "params": {
    "subscription_id": "obs:7d",
    "observation": [
      "obs:7d", 1, "2026-04-19T14:02:11.203Z", "range",
      {
        "surface-id": "surf:42",
        "path": "/children/1",
        "latency-ms": 812,
        "primitive-ref": {
          "iri": "ui://pressable",
          "instance-iri": "surf:42#children/1",
          "cid": "blake3:7a3f...",
          "context": "context://weftos/migration",
          "shape": "schema://verb",
          "variant": "v1"
        }
      }
    ]
  }
}
```

**Frame 4**: renderer → server (notification) — the click itself:

```json
{
  "jsonrpc": "2.0", "method": "observation.update",
  "params": {
    "subscription_id": "obs:7d",
    "observation": [
      "obs:7d", 2, "2026-04-19T14:02:11.205Z", "bearing",
      {
        "surface-id": "surf:42",
        "path": "/children/1",
        "affordance": "invoke",
        "actor": "user:mathew@weavelogic.ai",
        "primitive-ref": {
          "iri": "ui://pressable",
          "instance-iri": "surf:42#children/1",
          "cid": "blake3:7a3f...",
          "context": "context://weftos/migration",
          "shape": "schema://verb",
          "variant": "v1"
        }
      }
    ]
  }
}
```

**Frame 5**: renderer → server (request) — the resulting `invoke`:

```json
{
  "jsonrpc": "2.0", "id": 4, "method": "invoke",
  "goal_id": "goal:migration-dry-run-a1b2c3",
  "variant_id": "v1",
  "params": {
    "surface_id": "surf:42",
    "path": "/children/1",
    "affordance": "invoke",
    "args": { "target": "/claw/root/.../src" },
    "actor": "user:mathew@weavelogic.ai"
  }
}
```

**Frame 6**: server → renderer (response).

```json
{ "jsonrpc": "2.0", "id": 4,
  "result": { "result": { "dry_run_id": "dr:9e82" } } }
```

A corresponding `explicit` observation with matching `variant-id`
is emitted on the same stream, attributing the act to pulse v1.

### B.3 Mutate a `Chip`'s ordering

**Context**: GEPA has two variants of a tray `Chip` strip, `v1`
(alphabetical) and `v2` (most-recent-first). The composer mutates
the `ordering` axis of the parent `Stack`.

**Frame 1**: composer → server (request) — `mutate`.

```json
{
  "jsonrpc": "2.0", "id": 5, "method": "mutate",
  "goal_id": "goal:migration-dry-run-a1b2c3",
  "variant_id": "v1",
  "params": {
    "surface_id": "surf:42",
    "path": "/children/2",
    "axis": "ordering",
    "new_variant": "v2"
  }
}
```

**Frame 2**: server → composer (response).

```json
{ "jsonrpc": "2.0", "id": 5, "result": { "ok": true } }
```

**Frame 3**: server → renderer (notification) — `surface.update`
replacing the `Stack.children` ordering:

```json
{
  "jsonrpc": "2.0", "method": "surface.update",
  "params": {
    "surface_id": "surf:42",
    "base_version": 1,
    "ops": [
      { "kind": "replace",
        "path": "/children/2/variant-id",
        "value": "v2" },
      { "kind": "replace",
        "path": "/children/2/children",
        "value": [ /* reordered chip heads */ ] }
    ],
    "new_version": 2
  }
}
```

**Frame 4**: subsequent observations on the `Stack` carry
`primitive-ref.variant = "v2"`; ECC attributes new returns to the
new pulse. If the user's downstream behaviour shifts toward
coherence rising (faster selections, fewer reversals), GEPA will
keep `v2`; otherwise it will revert.

If the composer had attempted to mutate an axis frozen by safety —
say, the `Modal` affordance set — the server would have returned
error `-32022 AxisFrozen` with `data.reason = "axis frozen by
safety"` and `data.policy-uri` pointing to the governing rule.

---

*End of WSP-0.1.*
