# ADR-008: Goal Aggregate + Per-Goal Delegation

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Session 8 (RQ8) establishes that goals — not tasks, not sessions —
are the unit of user intent (Cooper, *About Face* ch. 1–2) and that
they must outlive agent sessions, context compactions, and session
restarts (AGENDA lines 192–196). Every competitor in Session 2's
teardown treats goals as prompt text, markdown memory, or custom-
agent personas; none treats goals as substrate-resident
constitutional entities. Session 9 (framing, rec. 9) ties the
reverse-DDD arrow to this: the goal is the domain object that drives
the surface; it is a resource, not agent state. Rasmus's unsolved
problem #1 (identity + delegation) finds a clean answer when
delegation is per-goal, not per-agent. The active-radar loop's
coherence trace (ADR-007) is only meaningful against a goal frame;
without one, "doppler toward coherence rising" has no anchor.

## Decision

A `Goal` is an Evans aggregate root, chain-anchored, first-class in
the substrate. Every WSP call carries `goal_id` in its envelope and
is adjudicated against the goals the substrate currently honours.
Delegation is per-goal, not per-agent, and is revocable by seal.

**Aggregate schema (canonical fields)** — per Session 8:

- **Identity**: `id` (blake3 of parent‖creator‖nonce), `parent`,
  `root`, `genesis_seq`, `creator`, `delegated_to[]`.
- **Intent**: `title`, `ontology_ref`, `success_predicate`,
  `abandon_predicate`.
- **Constraints**: `granted_affordances`, `denied_affordances`
  (deny wins), `effect_ceiling` (5-D: risk / fairness / privacy /
  novelty / security — child may only tighten), `surface_scope`,
  `capture_policy`, `environment_class`.
- **Lifecycle**: `state` (Draft → Active → Suspended / Drifting /
  Satisfied / Escalated / Abandoned / Sealed), `opened_at`,
  `deadline?`, `suspended_until?`.
- **Return binding**: `coherence_baseline`, `return_channels[]`.
- **Persistence**: `checkpoint`.

**Invariants** (chain-level violations, same weight as
`governance.root.supersede`): (1) deny beats grant through the
forest; (2) `effect_ceiling` dimensions only tighten on child;
(3) capture-permitting goals carry explicit consent + visible tray
chip (ADR-012); (4) state transitions are forward-only; (5) project
root has `parent=None` and is created at bootstrap.

**Envelope rule**: every WSP verb's envelope carries `goal_id`.
Absence is rejected at the kernel boundary — there is no
"no-goal" mode. The default is an explicitly chain-recorded
`adhoc-scratch` goal, visible in the tray.

**Double-gated enforcement** (extends ADR-033 pattern already
shipping in `governance.rs`): (a) routing gate (A2A router) fetches
the goal before dispatch and checks grants ∩ denies plus
`effect_ceiling`; (b) handler gate re-evaluates at dispatch time
because state may have moved. No new enforcement backend; extend
`GovernanceGate` to consume `goal_id` from context JSON.

**GovernanceGate binding**: every `invoke` / `mutate` implicitly
gate-checks; denials are structured `deny{reason, policy-uri,
elicit?}` (ADR-005 verb 11 / 12). `governance.goal.*` chain events
log permit / warn / defer / deny with `{goal_id, variant_id, effect,
evaluated_rules}` in the same payload shape as existing
`GovernanceDecisionEvent`.

**Delegation is per-goal, not per-agent**: "I delegate this goal
(with these affordances, this ceiling, this capture policy) to peer
X" is the unit. Cross-substrate delegation is goal portability:
export goal + granted capabilities as a ZK-SNARK of chain lineage;
remote governance admits the goal into its aggregate. Revocation is
chain-anchored via seal; remote delegate stops honouring on gossip
arrival. This is the canonical answer to Rasmus unsolved problem #1.

**Return-signal binding** (ADR-007): return observations carry
`goal_id` alongside `variant_id`. ECC computes coherence-delta
**per goal**, not a global scalar. `success_predicate` /
`abandon_predicate` are evaluated against the rolling per-goal
coherence trace, triggering `Satisfied` (human-confirmed seal) or
`Drifting` (escalation chip).

**Conflict resolution** (Session 8 §Conflict): deny always wins;
among conflicting grants, the narrowest surface-scope + lowest
effect-ceiling wins; unresolved cross-tree conflicts escalate to
human; seal retires a conflict.

## Consequences

### Positive
- Goals become a substrate entity agents query, not prompt text
  they guess against (Session 2 gap #7 closed).
- Active-radar coherence is goal-directed in Cooper's exact sense;
  ECC can say "variant X helped goal G" rather than "the user
  seemed happier".
- Cross-substrate delegation has a concrete protocol: export the
  goal + lineage proof, not the agent. Rasmus unsolved problem #1
  answered at the goal granularity.
- `capture_policy` binds per-goal, so foundations non-negotiable 1–4
  are enforced with an expiry frame (seal revokes consent).
- Trajectory pattern extraction (`TrajectoryRecorder`) scoped
  per-goal means patterns promoted on seal are attributable, not
  soup.
- `adhoc-scratch` default prevents silent no-goal mode — a
  hard-to-close honesty hole.

### Negative
- Every WSP call carries `goal_id`, which increases envelope size
  and requires every composer to manage goal context. Session 8
  open Q flags sub-second goal switching cost.
- Chain events per goal transition cost storage; an over-granular
  "goal per affordance" pattern would turn governance into noise.
  Caps (suggested 10–100 active per project) are a per-environment
  setting.
- Human authorship burden: users must articulate goals. Session 8
  open Q (minimum ceremony) remains open; natural-language
  goal-opening + agent translation to ontology is the working
  proposal.
- Sealing is heavy (creator + human witness); long-lived project-
  root goals may need a `Dormant` state (Session 8 open Q).

### Neutral
- Existing `clawft-types::goal` (`Goal`, `GoalTree`, `GoalStatus`)
  is the starting seed; the schema above extends it with the
  constraint / lifecycle / return-binding fields.

## Alternatives considered

1. **Goal as agent memory / prompt text (Claude Code `CLAUDE.md`,
   Cursor `@Past Chats`)** — rejected per Session 2 gap #7:
   file-shaped goals are not queryable-at-runtime substrate objects
   and cannot carry constraints the governance layer can enforce.
2. **Goal as policy record, not aggregate** — rejected per
   Evans ch. 6 (Session 8 rationale): a policy record can be
   evaluated but cannot be *responsible* for downstream subgoals,
   consents, trajectories, and captures across agent restarts.
   Aggregate responsibility is what we need.
3. **Delegate agents, not goals** — rejected: "delegate this agent
   with these tools" is ambiguous about *what the agent is for*.
   Per-goal delegation names the constitutional frame explicitly
   and is revocable at the right granularity.
4. **Goals optional, default implicit** — rejected: silent
   no-goal mode violates the honesty axiom of foundations; an
   explicit chain-recorded `adhoc-scratch` is the only acceptable
   default.
5. **Goals scoped per session (Plandex-style plan branches)** —
   rejected: plans-as-artefacts is a decent shape but project-
   scoped goals are the load-bearing persistence unit (AGENDA
   north star). Per-session goals collapse back to prompt memory.

## Related

- Sessions: `session-8-governance.md` (entire doc — schema,
  invariants, enforcement, lifecycle, persistence, conflict,
  crosswalk), `session-9-agentic-os-canon.md` (rec. 9 goals as
  resources, rec. 7 interruption posture, Rasmus unsolved problem
  #1 crosswalk), `session-2-agentic-ui-sota.md` (gap 7 goals
  everywhere are prompts), `session-4-ontology-ui.md` (reverse-DDD
  aggregate → Surface).
- Foundation elements: predicate 1 (ontology-addressable),
  predicate 2 (self-describing), §"Active-radar loop" (goal as
  frame of reference), §"Non-negotiable privacy constraints"
  (capture binds to consent binds to goal).
- Existing ADR: ADR-033 (constitutional governance, double-gated
  enforcement) — extended by goal context, not replaced.
- Symposium ADRs: ADR-005 (verb envelope carries goal_id),
  ADR-006 (primitive head), ADR-007 (return schema per-goal),
  ADR-009 (Mission Console seat/witness), ADR-012 (capture
  consent bound to goal).
