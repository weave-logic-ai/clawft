# ADR-004: Schema Shape — State-Diff Surface Tree + Event-Sourced Observation Tapestry

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Session 6 (RQ6) and Session 3 (RQ3, shared real-time surfaces)
converge on a non-obvious answer: the surface tree and the observation
tapestry have fundamentally different shapes and must travel on
different spines. LSP / Figma / Notion publish an authoritative
surface snapshot and apply versioned patches (state-diff). Yjs /
Liveblocks / pure CRDT systems merge ops commutatively and never
present a canonical snapshot. Matrix / Slack / mission-control
append events forever. Picking any one of these and applying it
everywhere breaks. Session 6 rejected "all event-sourced" (cold-start
latency, unnecessary folds for structural primitives) and "all
state-diff" (collapses the doppler/topology/bearing history the ECC
depends on — foundations §"Digital exhaust = intent"). Session 3's
agent-composition stress test (§"Agent-composition stress-test")
proved CRDT-for-everything cannot merge schema-changing composition
edits safely.

## Decision

WSP uses two spines, one per layer, chosen to match the nature of
what each carries.

**Surface tree — state-diff with monotonic `surface_version: u64`.**
`surface.compose` publishes an authoritative snapshot. `surface.update`
carries a version-numbered patch list (add / remove / replace by
JSON-Pointer path within the primitive tree). Updates include the
`base_version` they apply against; renderers reject deltas whose
base does not match current and re-issue `surface.get` to cold-start.
This is LSP `TextDocumentSyncKind.Incremental` played back. The
surface is a *structure*, not a history.

**Observation tapestry — event-sourced with monotonic `seq: u64` per
subscription.** Every observation (topology, doppler, range, bearing,
explicit, implicit, ambient, state, consent, governance) is an
immutable typed event. Renderers reconnect with `since_seq` for
replay. Matches `weftos-leaf-types::Subscribe.since_seq`. The
substrate is a *history* — intent is history — and collapsing it
to "last value" destroys the ECC substrate.

**CRDT — only inside leaf primitive payloads.** Text in a `Field`,
items in a collaborative `List`, annotations in a leaf note: CRDT
merge is fine and should be used (Yjs/Liveblocks shape). Structural
edits (compose, recompose, retype, swap IRI) travel on the state-diff
spine and are server-serialised. Session 3 §2 proved structural edits
cannot live in a pure CRDT without breaking intention preservation
(subtree moves, schema changes).

**Version mismatch policy.** On `base_version` mismatch the renderer
re-issues `surface.get` — no silent drift, no partial apply. Presence
records and subscription bindings that reference a stale primitive
path are garbage-collected by the server on every structural commit
(Session 3 rec. 2, "Presence is ontology-addressed, never pixel-
addressed").

## Consequences

### Positive
- Renderer cold-start is cheap: fetch snapshot + last N observations
  needed. No full-log fold required to draw the first frame.
- ECC receives the full observation history at resolution — no
  protocol-level aggregation, no lossy collapse. Active-radar loop
  (foundations §"active-radar loop") is satisfiable end-to-end.
- Audit is free: the observation spine is already event-sourced;
  `governance.goal.*` events (ADR-008) land on the same rails.
- Structural edits are unambiguously serialised; no two renderers
  ever disagree on whether a primitive exists (Session 3 stress
  test items 1, 2, 6, 8 all pass).
- CRDT is preserved *where it works* (freeform payload), banished
  *where it doesn't* (schema-changing composition). Both schools
  of thought get what they are good at.

### Negative
- Two spines to implement and test. Renderer authors must keep
  state-diff version semantics and event-seq semantics separately
  honest.
- Late-joiner replay on the observation side can be heavy; we rely
  on Session 3 rec. 8 (first-class `replay-since(t)` with compaction)
  to keep it bounded.
- Cross-node causal ordering between the two spines is not
  automatically consistent; a client sees "primitive X appeared"
  (state-diff) before "user interacted with X" (event-sourced) only
  if the composer ordered the compose ack before releasing the
  interaction observation. Handshake discipline, not wire guarantee.

### Neutral
- Session 3 open question ("OT vs event-sourced for the composition
  spine") is closed in favour of state-diff with a rejected-delta
  retry, which is simpler than OT and adequate for our schema.

## Alternatives considered

1. **All event-sourced (Matrix shape)** — rejected per Session 6 §3:
   renderers would need to fold the entire log to draw, every
   primitive addition is three or four events, cold-start is slow.
   A surface is a structure not a history.
2. **All state-diff (LSP shape)** — rejected: collapses the
   observation tapestry to last-value and breaks ECC. Intent is
   history, not a scalar.
3. **Pure CRDT everywhere (Liveblocks/Yjs shape)** — rejected per
   Session 3 §"Agent-composition stress-test": CRDTs cannot merge
   schema-changing composition safely; subtree moves / retypes /
   IRI swaps are not text-ops. CRDT is preserved only inside leaf
   payloads.
4. **OT on a typed composition alphabet (Google Docs shape)** —
   considered (Session 3 open Q) and rejected: OT on structural
   ops is implementable but requires transform tables we do not
   have; state-diff + retry is simpler and adequate for our
   schema-first canon.
5. **Two spines, single transport frame** — we do this: state-diff
   and events share transport (ADR-003) but use different verbs
   (`surface.*` vs `observe.*` / `substrate.update`).

## Related

- Sessions: `session-6-protocol-design.md` (§3 schema shape, §6
  subscriptions, §7 return-signal channel), `session-3-shared-realtime.md`
  (rec. 1 event-sourced spine, rec. 5 split channels, §stress-test),
  `session-9-agentic-os-canon.md` (rec. 5 stream envelope).
- Foundation elements: predicate 3 (streaming-native),
  §"Digital exhaust = intent", §"active-radar loop" (return as
  history).
- ADRs: ADR-003 (transport), ADR-005 (verb set), ADR-007 (return
  schema).
