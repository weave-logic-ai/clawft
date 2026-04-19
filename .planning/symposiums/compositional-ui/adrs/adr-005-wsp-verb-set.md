# ADR-005: WSP Verb Set — 17 Namespaced Verbs

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Session 6 §4 delivered the verb inventory for WSP: a hard cap around
twenty, every verb justified by a predicate or tier obligation, no
overlap. Session 1's historical canon recommended the X11
request/reply/event/error quadrant extended with stream verbs
(`open`, `chunk`, `commit`, `cancel`) — but those are
*transport primitives*, not protocol verbs; the protocol verbs sit
above them. Session 2's gap analysis (8 clean industry gaps) means
several verbs (`gate.check`, `consent.request`, `mutate`,
`ontology.describe`) have no analogue in the competitor cohort; the
verb set must therefore be explicit, not implicit. Session 9 rec. 5
anchors `subscribe` on `TopicRouter` + `A2ARouter` so the in-process
and mesh paths share shape. A verb list that drifts later
destabilises every downstream renderer; freezing it now lets ADR-006
(primitive head) and ADR-007 (return schema) refer to named verbs
without forward declarations.

## Decision

Seventeen verbs, all namespaced, each carrying a direction, params,
and response shape. No new verbs may be added without an ADR that
cites the predicate / tier / non-negotiable it serves. New
capabilities arrive as ontology URIs, not as new verbs.

### Session (2)

1. **`session.initialize(client_caps) → server_caps`** — request.
   LSP-style capability handshake. Carries `wsp_version: u8`,
   encoding preference (JSON | CBOR), supported primitive IRIs,
   supported wrapper IRIs, supported substrate topic prefixes,
   declared persona, locale, accessibility hints.
2. **`session.shutdown() → ok`** — request. Drains all
   subscriptions and composed surfaces for this session.

### Surfaces (4)

3. **`surface.compose(surface_spec) → surface_handle`** — request.
   Composer publishes a new surface; returns handle + initial
   `surface_version`. `surface_spec` is the primitive tree per ADR-001
   / ADR-006.
4. **`surface.get(surface_id) → surface_spec`** — request. Renderer
   requests current snapshot (cold start or version mismatch).
5. **`surface.update(surface_id, base_version, ops[]) → new_version`**
   — request. Composer pushes a patch list (add / remove / replace by
   JSON-Pointer path).
6. **`surface.dispose(surface_id) → ok`** — request. End-of-life.

### Substrate subscriptions (2)

7. **`subscribe(resource_uri, filter?) → subscription_id`** — request.
   Subscribe to a typed substrate resource. Produces
   `substrate.update` **notifications** (event-sourced, per ADR-004).
8. **`unsubscribe(subscription_id) → ok`** — request.

### Observation / return-signal stream (2)

9. **`observe(surface_id, channels[]) → observation_id`** — request.
   Opens the active-radar return channel for a composed surface.
   `channels` selects signal families (topology, doppler, range,
   bearing, explicit, implicit, ambient). Produces
   `observation.update` **notifications**.
10. **`observe.close(observation_id) → ok`** — request.

### Affordance invocation (2)

11. **`invoke(surface_id, path, affordance, args, actor) → result`**
    — request. Typed invocation of a primitive's affordance. `actor`
    is the ontology identity of the caller. Implicitly gate-checked;
    returns structured denial on refusal, never a silent no-op
    (see ADR-008).
12. **`mutate(surface_id, variant_axis, variant_id) → ok`** — request.
    The GEPA-style experimental mutation verb. Rejected by
    governance when the axis is marked fixed (`mutation-axes` on
    the primitive head). Separate from `surface.update` on purpose:
    mutation is a deliberate experiment, not a content push
    (foundations §"active-radar loop").

### Governance + consent (3)

13. **`gate.check(action, subject, context) → decision`** — request.
    Pre-flight check returning `allow | deny | elicit(reason)`. Used
    by the composer to enumerate honestly which affordances to offer
    right now (predicate 2).
14. **`consent.request(scope, purpose, duration) → consent_id`** —
    request. Opens a consent flow; a persistent tray chip is
    implicitly composed by the kernel whenever capture is live
    (ADR-012; foundations non-negotiable 4).
15. **`consent.revoke(consent_id) → ok`** — request.

### Introspection (2)

16. **`ontology.describe(primitive_type | surface_id) → description`**
    — request. Returns typed identity, state schema, affordance set,
    confidence model, mutation axes (predicate 1). Agents query this
    before acting; they do not guess.
17. **`cancel(request_id | stream_id) → ok`** — request. Universal
    cancel; matches LSP cancellation and MCP cancellation.

**Notifications (derived, not counted against the verb budget)**:
`substrate.update`, `observation.update`, and `surface.invalidate`
(issued on version mismatch or server-forced snapshot reseed). All
follow the `{ method, params }` envelope of ADR-003.

## Consequences

### Positive
- Seventeen verbs is legible to a single reader; growth is gated by
  ADR so the set cannot sprawl. Predicate and tier cites ride the
  verb spec permanently.
- Namespaces (`session.`, `surface.`, `observe.`, `consent.`,
  `ontology.`, etc.) keep a tooling `grep` useful even as clients
  proliferate.
- `gate.check` + structured `deny` on `invoke` / `mutate` means
  affordance enumeration is honest (predicate 2) without a silent
  no-op class of bug (Session 2 gap #2).
- `mutate` being distinct from `surface.update` means GEPA
  experiments cannot be disguised as content and always cross the
  governance gate (foundations, ADR-012 safety rule).
- `consent.*` being in-band (not a side door) closes the
  "permissions-as-filters vs permissions-as-affordances" open
  question from Session 3 in favour of affordances.

### Negative
- Seventeen verbs is still a lot to spec conformance tests for;
  the test matrix per encoding (ADR-003) is non-trivial.
- `ontology.describe` is chatty if used per first-use (Session 6
  §12 open Q); caching policy is a deployment concern.
- Some verbs (`mutate`, `gate.check`) have no LSP/MCP analogue, so
  tooling vendors will need a bespoke shim on first encounter.

### Neutral
- Capability flags (`session.initialize` return) carry future
  additions gracefully; renderers that do not declare a capability
  do not receive verbs that require it.

## Alternatives considered

1. **Fewer verbs, overload existing (MCP-minimalist)** — rejected:
   overloading `invoke` for mutation, consent, and gating collapses
   the denial-reason surface and makes governance auditability
   brittle.
2. **More verbs (separate `create` / `attach` / `move` / `style`)**
   — rejected: those are *ops* on `surface.update`'s patch list,
   not first-class verbs. Keeping them inside patches preserves
   atomicity (one `surface.update` = one version bump).
3. **No `gate.check` verb — rely on post-hoc denial** — rejected
   per Session 2 gap #2: predicate 2 requires that agents enumerate
   affordances, not attempt and hope. `gate.check` is the
   pre-flight half of that contract.
4. **`mutate` folded into `surface.update`** — rejected: GEPA
   experiments deserve a distinct governance surface so that fixed
   axes (consent, safety, brand) can be denied structurally rather
   than by advisory convention.
5. **Merging `observe` into `subscribe`** — rejected per Session 6
   §7: return signals are *about* a surface, not resources from
   the substrate; conflating the streams makes ECC attribution
   fragile (need to resynthesise the variant-id attribution from
   message ordering — error-prone).

## Related

- Sessions: `session-6-protocol-design.md` (§4 verbs), `session-1-historical-canon.md`
  (X11 quadrant + stream verbs, rec. 1), `session-2-agentic-ui-sota.md`
  (gap 2 affordance enumeration, gap 5 interruption policy,
  gap 8 return signals), `session-9-agentic-os-canon.md` (rec. 5).
- Foundation elements: all four predicates, §"Active-radar loop",
  §"Non-negotiable privacy constraints".
- ADRs: ADR-003 (transport), ADR-004 (schema shape), ADR-006
  (primitive head), ADR-007 (return schema), ADR-008 (goals).
