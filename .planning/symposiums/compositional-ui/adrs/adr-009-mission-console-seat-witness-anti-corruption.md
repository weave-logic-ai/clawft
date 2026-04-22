# ADR-009: Mission Console — Seat, Witness, Anti-Corruption

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Session 3 (RQ3) established that every pattern in the shared
real-time surface lineage (Figma, Liveblocks, Yjs, Google Docs,
Plan 9 Acme, NORAD mission-control, Twitch, incident dashboards,
Slack, Notion) imposes invariants on who may mutate what. When the
surface is composed by an agent at runtime (foundations' twist),
*presence as cosmetic cursor* collapses, *CRDT-for-everything*
collapses, and *role separation* becomes more important rather than
less. NORAD's mission-control discipline — every participant has a
seat, every non-trivial verb is witnessed on the loop — survives the
agent-composition twist and is the pattern the Mission Console must
inherit. Session 4 (RQ4) made `ContextMap` a Tier-A primitive
because Mission Console spans Evans bounded contexts (a regional
manager across many stores; counsel across client projects), and
Evans ch. 14 is unambiguous: cross-context surfaces without an
explicit translation layer corrupt each other's models. Session 8
(goals) made every participant — human or agent — carry a principal
id; seats and witnesses are now nameable.

## Decision

Mission Console is built around three invariants, not one: every
participant gets a seat, every verb is witnessed on-chain, and
every cross-context surface renders a typed `ContextMap` primitive
with an anti-corruption translation layer.

**Seat rule (NORAD role separation, made executable).** Humans,
agents, services, weaver-the-composer, capture sidecars, and
`ForeignSurface` shells all present in the same participants list.
Each seat carries `{principal_id, role, affordance_set,
context_membership[], attention_priority}`. No silent daemon:
weaver is visible as a seat whose verb is `compose`. No invisible
governance: `governance-counsel` is a seat whose verb is
`gate.check`. An agent joining a Mission Console mid-task emits a
`presence.join` event on the loop, exactly like a human.

**Witness rule (mission-control loop discipline).** Every verb that
mutates surface or substrate state is a chain event under
`governance.goal.*` (ADR-008) or the observation-stream spine
(ADR-004). No silent mutation. Read-only subscribes log at a weaker
tier. Replay is free because the spine is the audit (Session 3 rec.
4). Late-joiners issue `replay-since(t)` on the room and receive a
compacted event stream plus a snapshot, rendered as "what happened
while you were away" (Session 3 rec. 8).

**Anti-corruption rule (Evans ch. 14).** `ContextMap` is a Tier-A
primitive in the canon (ADR-001 #) carried as `ui://context-map`.
Every Mission Console surface declares its active `ContextFrame`
IRI; the composition protocol rejects primitive bindings whose IRI
is not reachable under that frame's namespace closure (Session 4
§"bounded-context enforcement"). Cross-context verbs flow through
an **anti-corruption adapter**: IRI translation at the composition
boundary, declared per-bridge, with each translation chain-logged.
A `ForeignParticipant` wrapper (analogous to `ForeignSurface`)
admits participants from external contexts with an explicit
affordance whitelist (Session 3 §"Bounded contexts and cross-tenant
collaboration").

**Presence is ontology-addressed, never pixel-addressed** (Session 3
rec. 2). Presence records `{participant_id, role, primitive_id,
affordance_in_progress, variant_id, context_membership}`. Renderers
resolve to pixels locally. Recomposition never invalidates presence
semantics. Cursor rendering is the shell's job, not the protocol's.

**Composition authority has a seat and a budget** (Session 3 rec. 9).
Weaver's mutations enter a preview channel first; governance
(ADR-008) may require human confirmation before promotion to the
canonical log. Role separation applied to agents: composing does
not imply auto-commit.

**Witness mode admits broadcast-only participants.** A Mission
Console may have active operators and passive witnesses (Session 3
rec. 10); the witness channel is broadcast with one-way reactions.
Witnesses are seats with `affordance_set = [react]` only.

**Tombstones, not deletes** (Session 3 rec. 10): removed primitives
leave a tombstone carrying the last schema version and reason; late
writes against them fail cleanly; replay renders them as struck-
through historical facts.

## Consequences

### Positive
- NORAD's strongest property — witnessed action on a recorded loop
  — transfers to the agent-composition case without dilution. Audit
  is the primary channel, not a bolt-on.
- Cross-context Mission Consoles (regional manager across stores;
  counsel across clients) cannot silently leak terms — the
  composition protocol refuses to bind an out-of-namespace IRI.
- Presence is stable across recomposition; a user focused on
  `field://order.qty@v7` remains focused even if the surface
  re-lays around them.
- Weaver being a visible seat makes agent composition legible to
  humans; "Weaver added a Gauge" is an event on the loop,
  inspectable and revocable.
- Witnesses as first-class seats (with `react`-only affordances)
  accommodate counselling-session / war-room observer postures
  without a parallel framework.

### Negative
- Every participant carrying context-membership means
  `session.initialize` grows; sessions that span three contexts
  emit three anti-corruption adapter declarations.
- Witness-mode broadcast plus read-only subscribes can be noisy in
  audit; compaction policy per room is a deployment concern.
- Preview-before-commit on weaver compositions adds a round trip on
  governance-gated surfaces; transient UI surfaces get a fast
  path (preview auto-commits after N ms) to preserve feel.

### Neutral
- Session 3 open question (variant reconciliation on audit) is
  resolved in favour of: canonical variant is what audit replays;
  per-participant variant deltas are recorded but subordinate.

## Alternatives considered

1. **Presence as Figma-cursor (pixel coordinates)** — rejected per
   Session 3 rec. 2 and §agent-composition-stress-test item 4:
   pixel coords are meaningless when the layout recomposes; the
   Mission Console is the case where recomposition happens most.
2. **Single "room owner" authority (Twitch-broadcast shape)** —
   rejected: Mission Console is multi-operator by design; witness
   mode is a *sub-mode*, not the whole model.
3. **Silent daemon agents (no seats)** — rejected per NORAD
   discipline: invisible actors are the pre-condition for
   split-brain authority. Weaver must have a seat.
4. **Cross-context surfaces with implicit translation** —
   rejected per Evans ch. 14 / Session 4 §"bounded-context
   enforcement": implicit translation is the corruption vector
   DDD names explicitly. An anti-corruption adapter is non-optional.
5. **CRDT spine for composition** — rejected per ADR-004 and
   Session 3 stress test: CRDT does not merge schema-changing
   composition. Structural edits serialise through the room server.
6. **Variants diverge freely per participant** — rejected: safety /
   consent / governance affordances must converge across
   participants (foundations non-negotiable). Shared-variant in
   Mission Console with free mutation in solo surfaces is the
   working compromise (Session 1 open Q closed).

## Related

- Sessions: `session-3-shared-realtime.md` (entire doc — NORAD,
  mission-control witnessed action, agent-composition stress test,
  bounded contexts, all 10 recs), `session-4-ontology-ui.md`
  (rec. 7 `ContextMap` as Tier-A, §"bounded-context enforcement"),
  `session-8-governance.md` (§"Conflict resolution" and
  `governance.goal.*` chain events), `session-9-agentic-os-canon.md`
  (rec. 9 goals as resources, Rasmus multi-agent protocols crosswalk).
- Foundation elements: predicate 1 (ontology-addressable),
  predicate 2 (self-describing), §"Two tiers" (ForeignSurface /
  ForeignParticipant parallel).
- ADRs: ADR-001 (canon includes `ui://context-map`), ADR-004
  (event-sourced spine — audit), ADR-007 (return schema echoes
  per-participant variant), ADR-008 (goal delegation + witness
  chain events), ADR-010 (reverse-DDD aggregate → Surface).
