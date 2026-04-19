# Session 8 — Goals as Constitutional Governance (RQ8)

**Scope**: How project- and task-scoped goals become first-class
substrate objects that outlive agent sessions and constrain what
surfaces an agent may compose, what affordances it may invoke, and
what signals it may capture. Goals sit as a constitutional layer
between the agent and the surface protocol (R6) — every `query`,
`invoke`, `subscribe`, and `mutate` call on a primitive carries a
goal id and is adjudicated against the goals the substrate currently
honours. Read against Cooper's goal-directed design thesis (goals,
not tasks, are the unit of user intent), Evans's aggregate-root
discipline, and the three-branch governance model already shipping in
`crates/clawft-kernel/src/governance.rs`. Out of scope: the wire
format itself (R6), identity delegation across substrates (touched in
crosswalk, owned by Rasmus's unsolved-problem #1), and the physical
renderer.

---

## Why goals, not tasks

Cooper, *About Face*, ch. 1–2: **"Users have goals, not tasks."** A
task is an instrument a user picks up to close a goal; the goal is
the durable thing. Classical UI designs around tasks (the user fills
this form, runs that command) and treats the goal as implicit in the
designer's head. Agentic UI cannot do that — the agent is the one
composing the surface, and if the goal is implicit, the agent is
guessing. The goal must be an explicit object the agent can read,
reason against, and (when authorised) split, defer, or close.

This session's claim: a goal is the agent-OS analogue of an **Evans
aggregate root** — a consistency boundary with a stable identity,
strict invariants, and exclusive authority over a cluster of child
objects (subgoals, granted capabilities, consent grants, captured
signals). Everything an agent does while that goal is active is
constrained by what the goal permits. When the goal closes, the
aggregate is sealed — the audit record is what remains.

This is also the minimum required to make the active-radar loop
(`foundations.md`, lines 211–273) coherent: if the display is a pulse
and the user's response is the return echo, the **goal is the frame
of reference** against which the return is interpreted. Without a
goal binding, "doppler shift toward coherence rising" has no anchor;
the ECC has nothing to decide *rising against what*. Goals make the
radar loop goal-directed in Cooper's exact sense.

---

## Goal schema proposal

A goal is a chain-anchored aggregate with the following canonical
fields. Every field is part of the substrate object; none live only
in agent memory.

```rust
pub struct Goal {
    // Identity (immutable, chain-anchored)
    pub id: GoalId,                     // blake3(parent_id || creator || nonce)
    pub parent: Option<GoalId>,         // forms the goal forest
    pub root: GoalId,                   // project-scoped root goal
    pub genesis_seq: u64,               // exochain sequence at creation
    pub creator: PrincipalId,           // who opened it (human or agent)
    pub delegated_to: Vec<PrincipalId>, // non-exclusive; forest, not tree

    // Intent (ubiquitous-language, Evans)
    pub title: String,                  // short, imperative
    pub ontology_ref: OntologyIri,      // typed domain term
    pub success_predicate: Predicate,   // how we recognise completion
    pub abandon_predicate: Predicate,   // how we recognise drift/loss

    // Constraints (the constitutional layer)
    pub granted_affordances: AffordanceSet, // allow-list of primitive ops
    pub denied_affordances: AffordanceSet,  // explicit deny-list (wins)
    pub effect_ceiling: EffectVector,       // 5D cap per action
    pub surface_scope: SurfaceScope,        // which panes may be composed
    pub capture_policy: CapturePolicy,      // camera/mic/screen consent
    pub environment_class: EnvironmentClass,// Dev / Staging / Prod

    // Lifecycle
    pub state: GoalState,               // Draft → Active → ... → Sealed
    pub opened_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub suspended_until: Option<DateTime<Utc>>,

    // Return-signal binding
    pub coherence_baseline: f32,        // initial ECC coherence
    pub return_channels: Vec<ReturnChannel>, // which signals feed this goal

    // Persistence
    pub checkpoint: GoalCheckpoint,     // last good state, for rehydration
}
```

**Invariants** (enforced at aggregate level; violation is a chain-
level constitutional violation, same weight as `governance.root.
supersede`):

1. `denied_affordances` is authoritative over `granted_affordances`
   on conflict. A denial on a parent goal propagates down the forest;
   a child cannot re-grant what an ancestor denied.
2. `effect_ceiling` is the max-allowed magnitude in each of the five
   dimensions already defined by `EffectVector` (risk, fairness,
   privacy, novelty, security). A child goal may tighten but never
   relax any dimension.
3. `capture_policy` must satisfy `foundations.md` lines 188–208: a
   goal that permits `foreign://mic`, `foreign://camera`, or
   `foreign://screen` must carry an explicit per-goal consent record
   and a visible tray chip. No goal may silently enable capture.
4. A goal's `state` may only move forward in the state machine
   below; transitions are chain events.
5. The root goal of a project has `parent == None` and is created at
   project bootstrap, not at agent-session start.

**Why an aggregate root rather than a policy record.** Evans's whole
point in ch. 6 is that invariants live with the aggregate, not with
the rule engine. A plain policy record can be evaluated; only an
aggregate root can be *responsible* for the set of downstream
objects — subgoals, consents, trajectories, captures — that must
stay consistent across agent restarts, mesh peer changes, and
context compactions. That responsibility is what the goal gives us
that a standalone rule cannot.

---

## Binding to surfaces — the protocol shape

Every call on the surface protocol (R6) carries a `goal_id`. The
protocol envelope is:

```
{ primitive: <ontology iri>,
  verb: query | invoke | subscribe | cancel | commit,
  goal_id: <GoalId>,
  variant_id: <opaque, from active-radar loop>,
  args: ... }
```

The kernel rejects any call with `goal_id == None` at the boundary —
there is no "no-goal" mode. The default dev session opens an
`adhoc-scratch` goal with a permissive but chain-recorded policy; the
absence of a named goal is itself a goal, and the user can see it.

**Enforcement is double-gated**, reusing the pattern ADR-033 already
ships:

- **Routing gate** (A2A router): before a message reaches the handler,
  the gate fetches `Goal(goal_id)` from the substrate, maps the
  primitive + verb to an affordance, checks `granted_affordances`
  minus `denied_affordances`, and checks the call's computed
  `EffectVector` against `effect_ceiling`.
- **Handler gate** (surface host): at dispatch time, the host re-
  evaluates — goal state may have moved (suspended, escalated) while
  the message was in flight. This is the same defense-in-depth pattern
  `agent_loop.rs` already uses for `exec`, `cron.add`, `cron.remove`.

The ECC-gated `GovernanceGate` (`gate.rs` lines 293–461) already maps
`GovernanceDecision::Deny → GateDecision::Deny` with a chain receipt.
We extend it with `GoalContext` in the `context` JSON: goal id,
surface scope, and variant id flow in, and `governance.goal.*` events
flow out. No new enforcement backend is needed — the schema change
is the protocol extension.

**Worked example** (`migration-dry-run` goal):

- Granted: `fs.read`, `fs.stat`, `shell.exec(dry_run=true)`,
  `plan.render`, `diff.render`.
- Denied: `fs.write`, `shell.exec(destructive=true)`,
  `mesh.send(kind=commit)`.
- Effect ceiling: `{ risk: 0.2, security: 0.1, privacy: 0.0, ... }`.
- Surface scope: allow `Panel#plan`, `Panel#diff`, `Tray#status`;
  deny every `ForeignSurface` of type `foreign://terminal` except
  the one launched as read-only by the dry-run harness.

An agent attempting `fs.write` inside this goal is denied at the
routing gate before the handler ever sees it; the denial is logged
as `governance.goal.deny` with the exact rule, the effect vector,
and the variant id. The user sees a single line in the Mission
Console: "dry-run goal blocked write attempt by weaver-agent-3".

---

## Binding to return signals

The active-radar loop treats display as pulse and user interaction
as return (foundations.md, lines 211–273). Each goal declares which
return channels are its signal versus noise:

- **Explicit return**: click, voice utterance, form submit against a
  primitive composed *within this goal's surface scope*.
- **Implicit return**: dwell time, scroll depth, focus duration on
  primitives in scope.
- **Exhaust return** (capture wrappers): OCR'd screen text, audio
  transcript fragment, camera presence events — only if
  `capture_policy` permits and a tray chip is live.

Each return carries the `goal_id` and `variant_id` the pulse was
emitted with. ECC computes a **coherence delta per goal**, not a
global one. `success_predicate` and `abandon_predicate` are evaluated
against the rolling coherence trace. When coherence rises past the
success predicate's threshold, the goal can auto-advance to
`Satisfied` (still a human confirmation for sealing); when it falls
past the abandon predicate's threshold, the goal transitions to
`Drifting` and surfaces an escalation chip.

This is the piece the governance layer uniquely enables: without a
goal frame, the coherence signal is aggregated across whatever the
user happens to be doing and says nothing about any particular
intent. With the goal frame, the same return-signal schema tells us
*per goal* whether the agent's pulses are helping or hurting. GEPA
mutation (foundations.md lines 236–253) optimises within the legal
mutation schema *for that goal's coherence*, not the user's general
satisfaction.

---

## Lifecycle state machine

```
Draft ──open──▶ Active ──suspend──▶ Suspended ──resume──▶ Active
                  │                                         │
                  ├──fork──▶ child Active (authority check)  │
                  │                                         │
                  ├──delegate──▶ Active (peer added)         │
                  │                                         │
                  ├──drift──▶ Drifting ──recover──▶ Active   │
                  │                  │                       │
                  │                  └──abandon──▶ Abandoned │
                  │                                         │
                  ├──satisfy──▶ Satisfied ──seal──▶ Sealed ◀─┘
                  │
                  └──conflict──▶ Escalated (human)
```

Transitions are chain events under `governance.goal.*`. Every event
carries the initiator's principal and, where relevant, the witness
signature of the human who authorised the transition. `Sealed` is
terminal: the aggregate becomes read-only, its trajectory is
promoted into the cross-session pattern store (see Persistence), and
subsequent references to `goal_id` return only audit data.

**Authority per transition**:

| Transition | Who may initiate |
|-|-|
| open (root) | Project owner (human) |
| open (child) | Agent or human with `goal.fork` on parent |
| suspend / resume | Any principal on `delegated_to` |
| delegate | Creator or existing delegate (dual-sign if cross-org) |
| drift / satisfy / abandon | System (ECC-triggered) or any delegate |
| seal | Creator + human witness (even if creator is human) |
| escalate | System; resolution requires human |

Sealing is intentionally heavier than opening. An unsealed goal is
cheap; a sealed goal is an audit commitment.

---

## Persistence — surviving the session boundary

This is the clause that makes goals useful rather than decorative.
The symposium's North Star (`AGENDA.md` lines 192–196) is explicit:
project-scoped goals must outlive individual agent sessions, context
compactions, and session restarts. Four persistence mechanisms,
chosen to match what the codebase already ships:

1. **Chain-anchored identity** (`chain.rs` `ChainLoggable` + new
   `GoalLifecycleEvent`). Goal create/state transitions are events
   on the ExoChain the same way governance decisions already are
   (see `GovernanceDecisionEvent`, `chain.rs:2312`). An agent that
   boots with no memory can reconstruct the active goal set by
   replaying `governance.goal.*` events from the root.
2. **Aggregate checkpoint** (substrate object). The mutable part of
   the goal — current state, coherence trace, working-memory
   references — lives as an aggregate object keyed by `goal_id`. On
   context compaction or session restart, the new agent session
   loads the checkpoint before accepting any surface calls. This is
   Rasmus's model-control-plane layer (AGENDA.md 95–100) bound to a
   surface-composable object, not a free-floating context blob.
3. **Trajectory pattern extraction** (`TrajectoryRecorder`,
   `governance.rs:583`). The existing recorder already captures
   agent decisions; we scope it per-goal and let a sealed goal
   promote its extracted patterns into a cross-session pattern
   store. A second session pursuing a similar goal inherits the
   priors, not a copy of the prior conversation.
4. **Capture-policy revocability**. Consent grants attached to a
   goal are chain-recorded with an explicit `valid_until` or
   `until_goal_state`; on goal seal, outstanding consents are
   automatically revoked and a revocation event chained. This is
   how `foundations.md` line 195–196 ("per-invocation consent,
   per-session consent, or standing consent with an audit trail")
   becomes implementable — the goal is the consent's expiry frame.

Across mesh-peer changes (Rasmus unsolved problem #2), the goal's
chain-anchored identity is the coordination key. A peer that joins
reads the root goal's genesis sequence; a peer whose view diverges
at any `governance.goal.*` event is assumed to have a split aggregate
and is isolated until reconciliation, the same mechanism
ADR-033's `governance.genesis` uses to prevent split-brain
governance.

---

## Conflict resolution protocol

Two goals conflict when their `granted`/`denied` sets, effect
ceilings, or capture policies cannot be simultaneously satisfied.
Example: "ship this feature today" grants `shell.exec(destructive)`;
"no privacy-sensitive data leaves this node" denies
`mesh.send(payload_class=capture)`. An agent scripting a commit-and-
push sequence straddles both.

Resolution, in order:

1. **Deny beats grant, always.** If any active ancestor or sibling
   denies an affordance the current goal would grant, the deny
   wins and the call is rejected at the routing gate. No conflict
   is ever silently resolved in favour of permissiveness.
2. **Most specific goal wins within permitted space.** Among goals
   that all grant an affordance, the one with the narrowest
   `surface_scope` and lowest `effect_ceiling` is selected as the
   evaluating frame. This is Evans's "small aggregate" rule — the
   narrower boundary holds the invariants.
3. **Human escalation on unresolved conflict.** If two active goals
   are incomparable — neither is an ancestor of the other and each
   grants what the other denies — both transition to `Escalated`
   and an escalation chip appears in the tray. The agent cannot
   compose any primitive requiring the disputed affordance until
   the human picks a winner or splits the work into sequential
   goals.
4. **Seal-on-completion.** When one of the conflicting goals
   reaches `Sealed`, the conflict is retired and the surviving
   goal resumes normal evaluation without human intervention.

This maps one-for-one to what `governance.rs` already does with
`EscalateToHuman` at the rule level; we lift it from per-rule to
per-aggregate.

---

## Audit and observability

Every goal lifecycle event and every gate decision mentioning a
`goal_id` is a chain event under `governance.goal.*`:

- `governance.goal.open` / `seal` / `suspend` / `resume` / `fork` /
  `delegate` / `drift` / `satisfy` / `abandon` / `escalate`
- `governance.goal.permit` / `warn` / `defer` / `deny` (extends the
  four `governance.*` event kinds `gate.rs` already logs)
- `governance.goal.consent.grant` / `revoke` (for capture policies)

The payload always carries `goal_id`, `variant_id`, `effect`, and
`evaluated_rules`, same shape as the existing
`GovernanceDecisionEvent` in `chain.rs:2312`. No new event codec
is needed.

**User observability** must not overwhelm:

- **Tray chip** shows at most one indicator per active root goal;
  clicking opens the Mission Console scoped to that root's forest.
- **Mission Console** renders the goal forest as the top-of-surface
  aggregate view (Evans's aggregate-root-at-top-of-surface rule
  from foundations.md lines 154–156). Each child goal is a Chip
  primitive with its coherence trace and pending decisions.
- **Deny tray** shows the last N gate denies involving an active
  goal, so the user is never surprised by silent refusal; each
  entry links to its chain receipt.
- **Capture surface** (non-negotiable) lives alongside active goals
  and shows exactly which captures any current goal is permitting.
  If any goal permits capture, the chip is visible; if no goal
  does, the chip is absent. There is no "capture-on, no goal to
  attribute it to" state.

Cooper posture mapping: the tray chip is **daemonic** (quiet,
continuous), the Mission Console goal view is **sovereign** (takes
focus when invoked), the deny tray is **transient** (shows and
retires). Three postures, three observability surfaces, one
aggregate backend.

---

## Crosswalk: Rasmus's identity-and-delegation problem

Rasmus lists identity-and-delegation as unsolved (AGENDA.md 105).
Our current answer (ExoChain + dual signing + RVF crypto +
governance gates + claims) is strong within a substrate but
unspecified across substrates. Goals give that cross-substrate
story:

- A goal is the **unit of delegation**, not an agent. "I delegate
  this goal (with these affordances, this ceiling, this capture
  policy) to peer X" is more precise than "I delegate this agent".
  The delegate inherits the goal's constitutional frame, full stop.
- Cross-substrate delegation becomes **goal portability**: export a
  goal + its granted capabilities (attested as a ZK-SNARK proof of
  the goal's chain lineage) to a peer on another substrate; the
  peer's governance verifies the proof and admits the goal into
  its own aggregate. This matches ADR-033's
  `governance.root.supersede` semantics, scaled down from
  constitutional root to individual aggregate.
- Delegation is **revocable by goal seal**, which the origin
  substrate controls unilaterally. The remote delegate learns of
  revocation through chain gossip and stops honouring the goal; any
  call with the revoked `goal_id` is a `governance.goal.deny` on
  the remote side.

This is also how the privacy constraints of foundations.md line
193–208 stay honest across peers: capture consent is a goal
property, goals are delegated not copied, and revocation is chain-
anchored. No substrate may silently carry a capture consent
forward.

---

## Recommendations for canon

Numbered for ADR sequencing. Each cites the predicate it
strengthens (P1–P4 from foundations.md) or the tier (A/B). Pre-ADR;
accept under this symposium.

1. **Goal is a first-class substrate object** with the schema in
   this doc; no agent-memory-only goal representation is
   protocol-legal. *P1, P2, tier A — goals must be ontology-
   addressable and self-describing.*
2. **Every surface-protocol call carries `goal_id`**; absence is
   rejected. *P2, P4 — affordance enumeration is goal-scoped; the
   agent cannot ask what is possible without declaring why.*
3. **The default no-goal goal is `adhoc-scratch`**, permissive but
   chain-recorded and visible in the tray. Accepting no-goal work
   silently violates the honesty axiom of foundations.md.
4. **Double-gated enforcement extends to goals** (routing gate +
   handler gate, same as ADR-033). Extend `GovernanceGate` to
   consume `goal_id` from the context JSON; map
   `GovernanceDecision::Deny` through. No new enforcement backend.
   *Defense-in-depth, tier A.*
5. **Capture wrappers bind to goal consent**; absence of a goal
   with `capture_policy.permit` disables the wrapper. The tray
   chip is visible whenever any such goal is active. *Tier B
   non-negotiable, foundations.md lines 188–208.*
6. **GEPA mutation is goal-scoped**: the return-signal schema
   attributes variants to the goal's coherence trace, not a
   global trace. Mutation is forbidden on goals whose
   `environment_class` is `Production` unless the mutation is
   itself granted in the goal's affordance set. *P3, active-
   radar-loop conformance.*
7. **Sealed goals promote trajectory patterns cross-session**; the
   `TrajectoryRecorder` is scoped per-goal, and seal is the
   promotion event. *Persistence across session boundaries;
   AGENDA.md lines 192–196.*
8. **Conflict between active goals is chain-anchored and human-
   escalated**; no silent precedence. Deny always beats grant.
   *Constitutional separation of branches carried down to the
   aggregate level.*
9. **Delegation is per-goal, not per-agent**; cross-substrate
   export uses ZK-proof of chain lineage. *Answers Rasmus's
   unsolved problem #1 at the goal granularity.*

---

## Open questions

- **Goal granularity economics.** Every call carrying a `goal_id`
  and every goal state being a chain event has cost. What is the
  right cap on active goals per project (10? 100?), and is that
  a protocol-level constant or a per-environment setting? The
  risk is a UI full of tiny one-affordance goals that turn
  governance into noise.
- **Sub-second goal switching.** Voice-mode expects rapid context
  shifts. Can we express "within this utterance, evaluate against
  goal X" without opening a child goal each time? A session-local
  "goal focus" cursor that rides on the message envelope without
  creating chain events looks right, but the conflict-resolution
  story needs to prove it doesn't become a backdoor to unsealed
  authority.
- **ECC + GEPA feedback on goal definitions themselves.** If the
  system learns that "migration-dry-run" goals consistently lack
  the `fs.stat` affordance and users hand-amend them, should the
  canon goal template auto-update? This is editable policy, which
  is a constitutional change under ADR-033 and needs a separate
  supersede pathway.
- **Human authorship burden.** Cooper's goal-directed thesis
  assumes the designer writes the goals. Here the user must.
  What is the minimum ceremony for opening a useful goal? A
  natural-language "what are you trying to do?" prompt that the
  agent translates into an ontology reference, with the user
  confirming granted/denied sets — is that the minimum, or is
  even that too much?
- **Seal semantics for long-lived goals.** A project-root goal
  may live for years. Sealing it erases working agency; never
  sealing it leaves an unbounded audit surface. Is there a
  `Dormant` state between `Active` and `Sealed` that pauses
  everything without promoting patterns? This symposium leans yes
  but does not commit.
- **Goal-vs-persona boundary.** Cooper's personas and our goals
  are both pre-session structures the designer (or project owner)
  commits to. Are personas emergent from observed goals, or are
  they a separate first-class object? Punted to RQ2 synthesis;
  noted here to keep the crosswalk honest.
