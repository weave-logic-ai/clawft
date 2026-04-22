# Session 6 — Surface Protocol Design (RQ6)

**Status**: Draft for symposium review
**Author**: System Architecture track
**Reads against**: `foundations.md` (canon), `AGENDA.md` RQ6, and the
existing `clawft-rpc`, `clawft-kernel::ipc`, `clawft-kernel::mesh_ipc`
bindings.
**Supersedes**: nothing — first protocol design pass.

This is the canonical deliverable for the protocol track: the wire
format that carries **surfaces** composed by agents, **substrate
subscriptions**, **state + affordances + confidence**, **return
signals**, **variant ids**, **mutation schemas**, and **governance
gates**. It is a design document, not an implementation; every
recommendation cites the foundations predicate or tier it serves.

---

## 1. Scope

The Surface Protocol (hereafter **WSP**, *WeftOS Surface Protocol*)
describes the messages exchanged among:

- **Composer** — an agent (or the kernel on behalf of an agent) that
  assembles a tree of primitives and declares substrate subscriptions.
- **Renderer** — any conformant viewer (egui today, toy web renderer
  tomorrow, Mission Console multi-user panel next) that draws
  primitives and emits return signals.
- **Substrate** — the kernel, mesh, chain, and service registry that
  own ground truth for resources, state, and governance.
- **Governance** — the constitutional-governance layer that admits or
  denies compose / mutate / observe operations.

Out of scope for this document: concrete egui widget mapping (RQ5),
Cursor/VSCode embedding (RQ7), goal DSL (RQ8). The protocol must be
orthogonal to all three.

**Non-negotiables inherited from `foundations.md`**:

1. Every primitive on the wire carries **state, affordances,
   confidence** as first-class fields (Tier A predicate 2).
2. Every render carries a **variant-id** (active-radar loop).
3. Every capture wrapper carries **consent + privacy flags**
   inline, not as metadata (digital-exhaust = intent).
4. No Tier-C. A message either is a Tier-A primitive, or it is a
   `ForeignSurface` (Tier B).

Designs that violate any of these are rejected at review, regardless
of other merits.

---

## 2. Transport + framing

### Recommendation

- **Local** (client ↔ kernel, same node): line-delimited JSON-RPC 2.0
  over the existing Unix domain socket (`clawft-rpc::protocol`).
  Reuse `Request`/`Response`, add a Notification envelope (new).
- **Mesh / cross-node** (composer on node A, renderer on node B):
  CBOR-encoded WSP frames wrapped in a `MeshIpcEnvelope` with a
  reserved `MessagePayload::Wsp { frame: Vec<u8> }` payload variant
  added to `clawft-kernel::ipc::MessagePayload`.
- **Browser / third-party renderer**: JSON-RPC 2.0 over WebSocket, one
  logical session per connection, same frame shapes as local.

### Rationale

1. **Bind against existing rails, don't invent.** The kernel already
   speaks JSON over UDS locally and JSON over mesh envelopes. LSP
   proved JSON-RPC + capability negotiation ages well across a huge
   client surface; MCP confirms it for AI use cases. We take the same
   bet, explicitly.
2. **CBOR where the weight pays for itself.** The return-signal
   stream (topology, doppler, range, bearing) is high-frequency and
   small-per-record; CBOR keyed by integer tags halves bytes and
   removes JSON number parsing. `weftos-leaf-types` already uses
   `ciborium` for leaf push — the house style supports it.
3. **Single schema, two encodings.** The CDDL in §5 is authoritative;
   JSON is the JSON-equivalent serialisation of the same CDDL
   structures with field names used as map keys. A renderer that
   speaks only JSON is still conformant. This follows CBOR-as-binary-
   JSON practice (RFC 8949) and avoids a two-schema split.
4. **Notifications are not requests.** LSP's request / response /
   notification trichotomy is load-bearing. Today `clawft-rpc` only
   has request/response. We add an explicit `Notification` envelope
   so that return-signal streams and state deltas don't pollute the
   correlation table.

### Framing additions to existing IPC

```
// New payload variant on MessagePayload (backward-compat: #[non_exhaustive])
Wsp { frame: Vec<u8> }     // CBOR-encoded WSP frame, self-describing
```

```
// New envelope on clawft-rpc (backward-compat: new type, not a breaking change)
pub struct Notification {
    pub method: String,
    pub params: serde_json::Value,  // or CBOR Value on the mesh path
}
```

WSP never introduces a new socket or a new daemon. It is the surface
method-namespace *and* frame vocabulary carried by the rails we have.

---

## 3. Schema shape — event-sourced OR state-diff?

### Recommendation

**Both, at different layers. This is not a compromise — it falls out
of the two-tier canon.**

- **Surface tree** (primitive composition): **state-diff** model.
  `surface.compose` publishes an authoritative snapshot; subsequent
  `surface.update` carries a version-numbered patch (add/remove/
  replace by `surface_id + path`). This is what LSP does for text
  documents (full + incremental) and it matches how renderers like
  egui actually reconcile.
- **Observation tapestry** (substrate state, return signals,
  wrapper activity, consent ledger): **event-sourced** stream. Each
  observation is an immutable typed event with a monotonic `seq` per
  subscription. This matches Matrix's room-state model and our own
  exochain mental model, and it is mandatory for replay, audit, and
  ECC ingestion.

### Arguments considered and rejected

- *All event-sourced.* Renderers would need to fold the entire event
  log before drawing the first frame, and every new primitive
  addition is three or four events (create, set-state, attach-
  affordances, set-confidence). A surface tree is a *structure*, not
  a history. Rendering latency and cold-start both suffer.
- *All state-diff.* Digital exhaust is intent (§foundations). Intent
  is a *history*, not a current value. Collapsing it to "last
  doppler" throws away the entire ECC substrate. The whole active-
  radar loop requires the stream.

The split is principled: **the surface is a view; the substrate is
history.** Matching shapes.

### Version semantics

- Every surface has a monotonically increasing `surface_version: u64`
  incremented on every `surface.update`. Updates include the
  `base_version` they apply against; renderers reject deltas that
  don't match their current version and re-issue `surface.get`.
  (This is LSP's `TextDocumentSyncKind.Incremental` played back.)
- Every observation stream has a monotonically increasing `seq: u64`
  within a subscription-id; renderers reconnect with `since_seq` for
  replay. Matches `weftos-leaf-types::Subscribe.since_seq`.

---

## 4. Core verbs (≤ 20)

Grouped by role. All methods are `namespace.verb`; all subscribe-style
methods produce `namespace.update` notifications.

### Session (2)

1. `session.initialize(client_caps) -> server_caps` — LSP-style
   capability handshake. Carries protocol version, encoding
   preference (JSON | CBOR), supported primitive types, supported
   wrapper types, supported substrate topic prefixes, declared
   persona (predicate 4), locale, accessibility hints.
2. `session.shutdown() -> ok` — drains all subscriptions and
   composed surfaces for this session.

### Surfaces (4)

3. `surface.compose(surface_spec) -> surface_handle` — composer
   publishes a new surface. Returns a handle + initial
   `surface_version`. `surface_spec` is the primitive tree (CDDL §5).
4. `surface.get(surface_id) -> surface_spec` — renderer requests
   the current snapshot (for cold start or after version mismatch).
5. `surface.update(surface_id, base_version, ops[]) -> new_version`
   — composer pushes a patch list (add, remove, replace by JSON
   Pointer-style path within the primitive tree).
6. `surface.dispose(surface_id) -> ok` — end-of-life.

### Substrate subscriptions (2)

7. `subscribe(resource_uri, filter?) -> subscription_id` — subscribe
   to a typed substrate resource (resource tree node, mesh peer set,
   chain event kind, service registry slice, weaver topic). Produces
   `substrate.update` notifications (event-sourced, see §3).
8. `unsubscribe(subscription_id) -> ok`.

### Observation / return-signal stream (2)

9. `observe(surface_id, channels[]) -> observation_id` — renderer
   opens the active-radar return channel for a composed surface.
   `channels` selects which signal families are emitted (topology,
   doppler, range, bearing, explicit, implicit, ambient-capture).
   Produces `observation.update` notifications.
10. `observe.close(observation_id) -> ok`.

### Affordance invocation (2)

11. `invoke(surface_id, path, affordance, args, actor) -> result` —
    typed invocation of a primitive's affordance. `actor` is the
    ontology identity of who is calling (human, agent-X, agent-Y).
    Governance gate-check is implicit and returns a structured
    denial on refusal (never a silent no-op).
12. `mutate(surface_id, variant_axis, variant_id) -> ok` — the
    GEPA-style mutation verb; rejected by governance when the axis
    is marked fixed. Separate from `surface.update` on purpose:
    mutation is a *deliberate experiment*, not a content push.

### Governance + consent (3)

13. `gate.check(action, subject, context) -> decision` — explicit
    pre-flight check that returns `allow | deny | elicit(reason)`.
    Used by composer to enumerate affordances honestly before
    offering them.
14. `consent.request(scope, purpose, duration) -> consent_id` —
    opens a consent flow surface; a persistent tray chip is
    implicitly composed so the user sees live capture state
    (§foundations non-negotiable 4).
15. `consent.revoke(consent_id) -> ok`.

### Introspection (2)

16. `ontology.describe(primitive_type | surface_id) ->
    description` — returns the typed identity, state schema,
    affordance set, confidence model, mutation axes (predicate 1).
    The agent queries this; it does not guess.
17. `cancel(request_id | stream_id) -> ok` — universal cancel,
    matches LSP cancellation + MCP cancellation.

**Count: 17 verbs.** Under budget. Every verb has a direct cite to
a predicate or tier obligation, and none overlap.

---

## 5. CDDL sketch

What follows is a reduced-to-essentials CDDL (RFC 8610). It uses the
house convention of integer map keys on hot paths and string map
keys on negotiation paths. A JSON renderer reads the same structure
with string keys.

```cddl
; ── Foundational types ────────────────────────────────────────

ontology-uri   = tstr .regexp "^[a-z]+://[a-zA-Z0-9._/\\-]+$"
actor-id       = tstr                       ; did:key | agent:pid | user:pubkey-hex
variant-id     = tstr                       ; opaque; composer-assigned
surface-id     = tstr
subscription-id = tstr
confidence     = {
  source      : "deterministic" / "inference" / "cache" / "input",
  ? value     : float64,                    ; 0.0..1.0 when probabilistic
  ? interval  : [float64, float64],         ; [lo, hi]
  ? as-of     : tdate,                      ; freshness
}
privacy-flags  = {                          ; first-class, never optional on capture
  captured    : bool,                       ; does this surface observe?
  retained    : bool,                       ; is raw data retained?
  off-node    : bool,                       ; does it leave the user's node?
  consent-id  : tstr / null,
}

; ── The envelope every primitive inherits ─────────────────────

primitive-head = {
  type        : ontology-uri,               ; e.g. "ui://pressable"
  path        : tstr,                       ; JSON-Pointer within the surface
  ? state     : any,                        ; typed per primitive-type
  affordances : [* affordance],             ; what actor may do now
  confidence  : confidence,                 ; predicate 2
  ? variant   : variant-id,                 ; active-radar (predicate 2 + loop)
  ? mutation-axes : [* mutation-axis],      ; legal GEPA axes, omit = none legal
  privacy     : privacy-flags,
}

affordance = {
  name        : tstr,                       ; "invoke" | "subscribe" | ...
  verb        : "read" / "write" / "invoke" / "subscribe" / "dismiss",
  ? actors    : [* actor-id],               ; restrict to these callers (gate)
  ? args-schema : ontology-uri,             ; declared, not inferred
}

mutation-axis = {
  name        : tstr,                       ; "copy" | "density" | "ordering"
  kind        : "discrete" / "continuous" / "categorical",
  ? domain    : any,                        ; declared variation range
  frozen-by   : [* "brand" / "safety" / "consent" / "user-request"],
}

; ── Five representative Tier-A primitives ─────────────────────

pressable = { primitive-head,                ; "ui://pressable"
  label       : tstr,
  ? icon      : ontology-uri,
  ? hotkey    : tstr,
  ? voice     : [* tstr],                   ; utterance triggers — dual-operator
}

gauge = { primitive-head,                    ; "ui://gauge"
  value       : float64,
  range       : [float64, float64],
  unit        : tstr,
  ? bind      : subscription-id,            ; live-bound to substrate
}

field = { primitive-head,                    ; "ui://field"
  name        : tstr,
  schema      : ontology-uri,               ; validation via ontology
  value       : any,
  dirty       : bool,
  ? errors    : [* tstr],
}

chip = { primitive-head,                     ; "ui://chip" (persistent presence)
  label       : tstr,
  kind        : "consent" / "capture" / "status" / "identity",
}

stream-view = { primitive-head,              ; "ui://stream-view"
  feed        : subscription-id,
  cursor      : "live" / uint,              ; live-tail vs specific seq
  ? density   : "compact" / "comfortable" / "dense",
}

; ── ForeignSurface (Tier B) — opaque inside, compliant outside

foreign-surface = { primitive-head,          ; "ui://foreign"
  wrapper-type : ontology-uri,              ; "foreign://browser" etc.
  identity-hint : {
    ? url      : tstr,
    ? bundle   : tstr,
    ? pid      : uint,
  },
  shell-state : {
    opened-at  : tdate,
    focus      : bool,
    size       : [uint, uint],
    ? last-url : tstr,
    ? exit-code: int,
    health     : "starting" / "ready" / "busy" / "crashed" / "closed",
  },
  presence-signal : {                       ; tier-B-as-tapestry (§foundations)
    active     : bool,
    attention  : "foreground" / "background" / "hidden",
    activity-rate : float64,                ; 0..1, lossy by policy
  },
  capture-channels : [* capture-channel],   ; empty when no capture
  ; privacy-flags on the head ALWAYS populated when capture-channels non-empty
}

capture-channel = {
  kind        : "mic" / "camera" / "screen" / "clipboard" / "a11y-tree",
  emits       : "interpretation" / "raw",    ; §foundations non-negotiable 3
  consent-id  : tstr,                        ; must point to an active grant
}

; ── Observation tapestry — event-sourced, tagged cbor 27 ──────

observation = #6.27([                        ; cbor tag 27 ("uri-hash-like")
  subscription-id,
  uint,                                       ; seq
  tdate,                                      ; wall-clock
  observation-kind,
  observation-body,
])

observation-kind = "topology" / "doppler" / "range" / "bearing"
                 / "explicit" / "implicit" / "ambient" / "state"
                 / "consent" / "governance"

; 6 canonical bodies; others declared via ontology
topology    = { surface-id, path, dwell-ms: uint, order-index: uint }
doppler     = { surface-id, path, velocity: float64, direction: "forward"/"backward" }
range       = { surface-id, path, latency-ms: uint }
bearing     = { surface-id, path, affordance: tstr, actor: actor-id }
explicit    = { surface-id, path, affordance: tstr, args: any, confidence: confidence }
ambient     = { wrapper: ontology-uri, channel: tstr, digest: tstr }  ; lossy interpretation

observation-body = topology / doppler / range / bearing
                 / explicit / ambient / { * tstr => any }  ; open for extension

; ── Surface root ──────────────────────────────────────────────

surface-spec = {
  id          : surface-id,
  version     : uint,
  persona     : ontology-uri,               ; Cooper + predicate 4
  mode        : "desktop" / "mission-console" / "embedded-dev",
  governance  : {                           ; goals + constraints bound to this surface
    project   : ontology-uri,
    goals     : [* ontology-uri],
    constraints : [* ontology-uri],
  },
  root        : primitive-head,             ; actual primitive type is one of §5
  subscriptions : [* { id: subscription-id, uri: ontology-uri, filter: any }],
}
```

The above covers Pressable, Gauge, Field, Chip, StreamView,
ForeignSurface, the observation stream, and the surface root —
six representative shapes plus the wrapper.

Two load-bearing points the CDDL enforces:

1. **`privacy-flags` and `confidence` are on `primitive-head`,
   not optional.** Every primitive inherits them by composition.
   A wire frame missing them is malformed, not merely "underspecified."
2. **`variant-id` is on the head and `mutation-axes` declares the
   legal axes in-band.** The active-radar return signals can always
   attribute back to the exact pulse, and governance can reject
   mutations against undeclared or frozen axes without out-of-band
   lookup.

---

## 6. Subscriptions — how primitives bind to substrate

- **Addressing**: `ontology-uri` throughout. Examples:
  `resource://project/store-47/orders/live`,
  `mesh://peers/active`, `chain://events/governance.vote`,
  `service://weaver/tasks`, `topic://mesh.leaf.$pubkey.announce`.
- **Creation**: the composer emits one `subscribe` per unique
  resource and embeds the returned `subscription-id` inside
  primitive fields (e.g. `gauge.bind`, `stream-view.feed`). The
  subscription is a *first-class shared resource* within the
  surface — multiple primitives can bind the same id.
- **Delivery**: `substrate.update` notifications carrying
  `observation(kind="state")` frames, event-sourced with `seq`. The
  renderer applies them to bound primitives; state fans out from one
  subscription to many views without the composer re-sending.
- **Backpressure**: renderer returns `cancel(subscription_id)` or
  the `session.initialize` can declare `max_inflight` per channel.
  LSP's token-based progress model applies. Ambient-capture
  channels (`kind=ambient`) are batched by the kernel into digests
  before delivery; raw frames never cross the session boundary
  unless `capture-channel.emits="raw"` AND an active consent-id is
  quoted in the request.

---

## 7. Return-signal channel

This is the part most wire protocols omit entirely, and it's the one
that makes WSP an intent-collection protocol rather than a display
protocol (`foundations.md`, "digital exhaust = intent").

- Return signals are delivered as **observations on their own
  stream**, created by `observe`, not interleaved with substrate
  state. This mirrors Matrix's split of message events from state
  events and avoids the composer having to sift the stream to
  reconstruct ECC inputs.
- The six canonical bodies (`topology`, `doppler`, `range`,
  `bearing`, `explicit`, `ambient`) map 1:1 to the
  `foundations.md` active-radar vocabulary. New kinds are added by
  ontology URI, not by changing the wire.
- Every observation carries the `variant-id` it was attributed to
  (via the `surface-id + path` pair, which resolves to a specific
  `primitive-head.variant`). ECC and GEPA consume this stream
  directly — the protocol does not pre-aggregate, because the
  intent is the history.
- **Privacy**: `ambient` observations default to
  interpretation-only digests. Raw bytes require a positive
  `consent-id` and are carried in a sibling `observation` tagged
  with `kind=explicit, args: { retained: true, consent-id: ... }`
  so the chain audit trail records the retention decision.

---

## 8. Variant + mutation schema on the wire

1. Composer assigns a `variant-id` at `surface.compose` time. It
   is also attached per-primitive on every `surface.update` that
   touches a primitive's rendered state.
2. Composer declares `mutation-axes` on the primitive head. An
   axis not declared is fixed. The `frozen-by` list is advisory
   metadata the governance layer uses to produce specific denial
   reasons ("brand", "safety", "consent", "user-request").
3. The `mutate` verb carries `(surface_id, variant_axis,
   variant_id)` and is always gate-checked before it applies.
   Consent / safety affordances (§foundations non-negotiable:
   "cannot mutate consent or safety flows ever") are enforced by
   the governance layer returning `deny{reason="frozen:safety"}`,
   not by the composer's good manners.
4. The variant-id is opaque to the renderer. The renderer MUST
   echo it on every return-signal observation so ECC can attribute
   echoes to pulses without shared state.

---

## 9. Governance + consent integration

### Two places, one schema

- **Pre-flight** (`gate.check`) — the composer enumerates what is
  *allowed right now* before offering the affordance (predicate 2).
  Agents enumerate affordances; they do not attempt and hope
  (`foundations.md` §1.2).
- **In-flight** — every `invoke` and `mutate` is implicitly
  gate-checked. Denials are structured: `decision = { deny:
  { reason, policy-uri, elicit?: ontology-uri } }`. `elicit`
  allows governance to ask for consent inline, producing a
  follow-up `consent.request` surface composed by the governance
  agent.

### Consent is not metadata

- `privacy-flags` on every `primitive-head` declares capture
  status, retention, off-node movement, and the governing
  consent-id.
- A `foreign-surface` with a non-empty `capture-channels` list
  and an absent or expired `consent-id` is a **malformed frame**
  — the kernel rejects it before it reaches the renderer. This is
  deliberate: silent degradation of privacy is the failure mode
  we are ruling out at the type-system level.
- A persistent tray `chip{kind="capture"}` is implicitly composed
  by the kernel whenever any `foreign-surface` declares a live
  capture channel. This is not a renderer obligation; it is a
  protocol obligation. "Observable to the user" (§foundations
  non-negotiable 4) becomes a thing the schema guarantees.
- All consent grants and revocations are appended to the chain
  via the existing `clawft-kernel::chain` path. Protocol adds no
  new storage.

---

## 10. Versioning strategy

Three layers; different rules for each.

1. **Wire version** — single integer in `session.initialize`
   (`wsp_version: u8`). Must be mutually supported. Incompatible
   client/server fail the handshake with a structured error (LSP-
   style error -32002).
2. **Capability flags** — every new feature ships as a capability
   flag on `server_caps` and `client_caps`. Unknown capabilities
   MUST be ignored (LSP rule). New primitive types arrive as new
   ontology URIs; older renderers that don't support a URI draw a
   `foreign-surface` fallback with `identity-hint.bundle` set to
   the URI (this is the Adaptive Cards `fallbackText` pattern
   lifted into the canon — versioning by graceful degradation).
3. **Ontology URIs are stable or they break.** Primitive types
   and substrate resource types are identified by URI. URIs
   never change meaning; new behaviour is a new URI. This is
   Cap'n Proto's "rename freely, renumber never" applied to the
   semantic layer instead of the binary one.

Consequences:

- A renderer that only speaks WSP-1 and knows `ui://pressable`,
  `ui://gauge`, `ui://field`, `ui://chip`, `ui://stream-view`,
  `ui://foreign` can render anything an agent composes, because
  novel types degrade to `ui://foreign`. A design goal: the
  renderer minimum viable set never grows unboundedly.
- The observation stream is extended by adding new
  `observation-kind` values (open `tstr`). Consumers that don't
  know a kind skip it and continue. ECC ingests only what it
  understands. This is Matrix's event-type model.

---

## 11. Recommendations for canon

Numbered, each with a predicate / tier cite.

1. **Adopt WSP as the canonical surface protocol for WeftOS.**
   Binds existing rails (UDS + mesh envelopes) rather than
   inventing transport. [All four predicates.]
2. **Encode on two rails: JSON-RPC for local + WebSocket,
   CBOR for mesh and for the observation stream.** Single CDDL,
   two encodings. [Streaming-native predicate 3.]
3. **Add `MessagePayload::Wsp { frame: Vec<u8> }` to
   `clawft-kernel::ipc`.** Mesh envelope carries CBOR-encoded WSP
   frames without changing the outer routing layer. [Tier-A + B.]
4. **Add a `Notification` envelope to `clawft-rpc::protocol`.**
   Required for observation / substrate streams; do not abuse
   request/response. [Predicate 3.]
5. **`primitive-head` carries `state`, `affordances`,
   `confidence`, `variant-id`, `privacy-flags`, and
   `mutation-axes` by composition.** A frame lacking these is
   malformed. Reject in the kernel, not the renderer. [Predicate
   2 + active-radar loop + digital-exhaust = intent.]
6. **Surface tree is state-diff with `surface_version`;
   observation tapestry is event-sourced with `seq`.** Shape
   matches intent. [Predicate 2 vs predicate 3 + foundations
   ECC.]
7. **17 verbs, all namespaced, all capability-gated.** Verb
   budget is a hard cap; new capabilities arrive as ontology
   URIs, not as new verbs. [Simplicity + predicate 1.]
8. **`ForeignSurface` is the only Tier-B escape hatch, and its
   capture channels + consent-id are structural preconditions,
   not optional fields.** No Tier-C. [Foundations canon.]
9. **Persistent capture `chip{kind="capture"}` is composed by
   the kernel, not the agent, whenever any live capture exists.**
   Protocol-level invariant. [Non-negotiable 4.]
10. **Version by capability flags and ontology URIs, never by
    breaking the schema.** Older renderers degrade to foreign-
    surface fallbacks on unknown primitive types. [Adaptive
    Cards lesson.]
11. **Every `observation` records the `variant-id` it attributes
    to.** No protocol-level aggregation. ECC + GEPA receive the
    raw stream. [Active-radar loop, non-negotiable.]
12. **Governance denials are structured `deny{reason,
    policy-uri, elicit?}`, never a silent no-op.** Composer can
    elicit consent and retry. [Predicate 2, honest affordances.]

---

## 12. Open questions (for symposium)

- **Cross-substrate identity.** `actor-id` is a string today;
  `did:key` format is tentative. This is Rasmus's unsolved
  problem #1 ("identity + delegation across org boundaries"). The
  protocol should not close it unilaterally — we need ExoChain
  + ADR-009 alignment.
- **Back-pressure semantics on the observation stream.** We
  propose LSP-style progress tokens; a harder question is *what
  the kernel does when the renderer is slow* (drop, coalesce,
  queue-and-degrade). Matches the mesh-runtime back-pressure
  discussion already in flight.
- **Multi-renderer surfaces (Mission Console).** `surface_id` is
  a single handle; Mission Console wants one composer pushing to
  N renderers with potentially divergent affordances per actor.
  Current proposal: a `Projection` object per renderer that
  selects an actor-filtered affordance view. Needs its own
  session in round 2.
- **Where does the ontology live on the wire?** `ontology-uri`
  assumes clients can resolve the URI to a schema. We should
  decide whether `session.initialize` pre-ships a schema manifest
  (LSP-ish) or whether `ontology.describe` is mandatory per type
  first-use (lazy).
- **RVF + WSP relationship.** WSP frames could themselves be
  carried as `MessagePayload::Rvf { segment_type, data }` with a
  reserved WSP discriminant. Worth aligning with the RVF WG
  before we fork a second CBOR envelope.
- **Encoding for browser / WASM renderers.** JSON-over-WebSocket
  is the fallback; a CBOR-over-WebSocket path would halve the
  bytes for observation streams. Needs a perf test, not a guess.

---

## 13. Closing

WSP's unusual shape — privacy and confidence in the head,
variant-id and mutation-axes first-class, observation stream
separate from state — is a direct reading of `foundations.md`.
Every predicate and every non-negotiable appears in the wire.
If the protocol doesn't carry these, it is not our protocol —
it is an ordinary display protocol wearing our labels.

The protocol is deliberately small (17 verbs, 6 primitive
shapes + 1 wrapper, 6 observation bodies). Small protocols
survive; big ones fossilise. The richness lives in the ontology
URIs the protocol *refers* to, not in the protocol itself. That
is the reverse-DDD arrow in action: substrate + ontology are
the heavy objects; the wire is just the view.
