# ADR-066: Capability tokens and human-join flow for board/channel access, gated by governance

**Date**: 2026-07-03
**Status**: Proposed
**Deciders**: Governance/mesh security review 2026-07-03 (AgentBBS pattern study)
**Depends-On**: ADR-025 (Ed25519 node identity), ADR-063 (substrate signed
envelope), ADR-064 (anti-entropy reconciliation), ADR-057 (substrate read
ACLs), ADR-049 (WeftOS kernel), ADR-033 (three-branch governance),
ADR-022 (ExoChain audit)
**Interlocks-With**: ADR-063 (tokens authorize writes outside one's own
prefix), ADR-064 (tokens gate who may sync a subtree), ADR-065 (tokens gate who
may bridge a channel)

## Context

WeftOS authorizes at two disconnected layers today, and neither expresses
*object-scoped, delegable* access:

- The **capability-class gate** (`crates/clawft-weave/src/capability.rs`,
  referenced by ADR-057) is a four-class model — `Read`, `Chat`, `Write`,
  `Admin` — checked at the daemon's JSON-RPC boundary. It answers "may this
  caller invoke this verb-class" but not "on which board / subtree / channel."
  ADR-057 itself flags this: per-path read ACLs had to be bolted on *beside*
  the capability gate because the gate could not scope to a path.
- The **`GovernanceGate`** (`crates/clawft-kernel/src/gate.rs`) evaluates
  effect-vectors and risk, returning `Permit` / `Deny` / `Defer`, and logs each
  decision to the chain. It is the right place for "is this *action* allowed,"
  but it has no notion of a portable grant a subject can *hold* and *present*.

Meanwhile the mesh is acquiring exactly the surfaces that need object-scoped,
delegable authorization:

- **ADR-063 path-ownership** deny-by-default blocks any write outside an
  identity's own `substrate/<self>/**` prefix — but legitimate cross-prefix
  writes (an operator seeding another node's config, an Actor writing a shared
  board) need an explicit, auditable grant.
- **ADR-064 anti-entropy** must decide *which peers may sync which subtrees* —
  a per-subtree, per-peer grant.
- **ADR-065 bridging** must decide *which external sources / channels* a bridge
  may relay, and support **humans joining** a WeftOS board from an external
  platform without holding a native key.

We need a capability primitive that is (1) fine-grained and object-scoped,
(2) least-privilege by default, (3) cryptographically bound to its holder and
its scope, (4) delegable and revocable, and (5) enforced through the existing
governance gate so every grant and every use is on the audit chain — and a
**human-join** flow layered on top so a person, not just a keyed agent, can be
admitted to a scoped resource with a human-in-the-loop approval.

## Decision

Introduce **capability tokens** — signed, scoped, least-privilege grants — as
the object-level authorization primitive, and route their issuance and their
use through the `GovernanceGate`. Layer a **human-join** flow on top for
admitting humans/agents to a scoped resource behind a signed approval.

### Capability set (what powers exist)

Define a compact capability bitset naming fine-grained powers, least-privilege
by default (empty grants nothing; a fresh identity can read public paths and
write only its own prefix, per ADR-057/063). Illustrative powers:

```
READ_SUBTREE  WRITE_SUBTREE  SYNC_SUBTREE  BRIDGE_CHANNEL
CREATE_BOARD  MODERATE       DELEGATE      ADMIN
```

Roles (e.g. `Guest`, `Operator`, `Moderator`, `Federator`, `Admin`) are
*convenience bundles* of these bits, but every enforcement check inspects the
underlying **bits**, never the role name — so a role rename never changes what
is authorized. This is the AgentBBS `Caps` discipline (their ADR-0004): one
bitset, one `require(held, needed, name)` enforcement primitive, default-safe.

### Capability token (the delegable, scoped grant)

```
CapabilityToken
  ├─ issuer:   NodeId | ActorId       (who granted it)
  ├─ subject:  NodeId | ActorId | BridgeSubkey   (who may use it)
  ├─ caps:     CapabilitySet          (which powers)
  ├─ scope:    PathGlob               (on which object, e.g. substrate/<mesh>/board/ops/**)
  ├─ expires:  Option<u64>            (unix-ms; None = until revoked)
  ├─ id:       TokenId                (BLAKE3 of canonical signing bytes)
  └─ sig:      issuer Ed25519 signature (+ ML-DSA if cross-node, ADR-028)
```

A token is a content-addressed, signed artifact (same discipline as the ADR-063
envelope): verify recomputes `id` then checks the issuer signature. It is
**scoped** to a path-glob (the same glob vocabulary ADR-057 uses: trailing `**`
subtree, single `*` one-segment) so a grant is always "these caps, on this
object," never blanket. It is **delegable** only if the issuer's own token
carries `DELEGATE`, and a delegated token's caps and scope MUST be a **subset**
of the delegator's (attenuation only — you cannot grant more than you hold).
Tokens live in the mesh-owned subtree (`substrate/<mesh-id>/caps/**`), so they
replicate via ADR-064 and survive a node rejoining with a fresh key, exactly as
ADR-057 stores the ACL table.

### Enforcement runs through the governance gate

The `GovernanceGate::check(agent_id, action, context)` path
(`gate.rs`) is extended so `context` carries the presented `CapabilityToken`
(or its id, resolved against the caps subtree). The gate:

1. Verifies the token signature and that `subject == agent_id` (the presenter
   holds the grant).
2. Checks the token is unexpired and not revoked, that `caps` covers the
   requested `action`, and that the request's target path is within `scope`.
3. Feeds the result into the existing effect-vector / risk evaluation, so a
   token that is *valid* can still be `Defer`red to a human or `Deny`ed by a
   governance rule (e.g. a high-risk write escalates even with a write token).
4. Emits the decision to the ExoChain (`governance.permit/deny/defer`), as it
   already does — so token *use* is audited alongside every other governance
   decision, and token *issuance* / *revocation* emit their own
   `caps.granted` / `caps.revoked` events.

This keeps a single enforcement point: the gate is where "may this identity
take this action on this object" is answered, with the capability token as the
evidence it weighs — not a parallel authorization system.

### Human-join flow (admitting a person behind an approval)

A human (or an agent acting for one) joins a scoped resource — a board, a mesh,
a bridged channel — without pre-holding a native key, via a
propose → approve → grant handshake modeled on AgentBBS's human-in-the-loop
approval gate (their ADR-0038):

1. **Join proposal.** The joiner presents a request to a scoped resource: an
   `ActionProposal`-shaped, content-addressed artifact
   (`kind = "join"`, `scope`, `requested_caps`, a freshly generated or bridged
   subject identity). A human arriving over a bridge (ADR-065) is represented by
   their per-source subkey; a human at a native client generates a client-held
   key (the ADR-063 / AgentBBS-0016 "client signs, node verifies" model).
2. **Human approval.** An identity holding `MODERATE`/`ADMIN` on that scope
   issues a **`SignedDecision`** (`Approve` / `Reject`) over the proposal's
   content-addressed id, Ed25519-signed. The decision is **fail-closed and
   veto-wins**: the join is authorized only if an allowed approver signed
   `Approve` and no allowed approver signed `Reject`; an empty approver set
   authorizes nothing. The server (daemon) only *verifies* decisions — it never
   signs an approval on a human's behalf.
3. **Grant issuance.** On a verified approval, the resource issues a
   `CapabilityToken` to the joiner's subject, scoped to exactly the approved
   `scope` and `caps` (never broader than requested, never broader than the
   approver holds). The proposal, the decision, and the issued token are all
   signed artifacts on the chain — the whole join is attributable end to end.

This makes "let this person into this board/channel/subtree" a governed,
signed, audited operation rather than an allow-list edit, and it works for both
native clients and bridged humans.

## Consequences

### Positive

- Object-scoped authorization the capability-class gate never had: grants say
  "these caps, on this path-glob," closing the gap ADR-057 called out and giving
  ADR-063/064/065 the per-object grant they each need.
- Least-privilege and attenuation-only delegation keep the blast radius of any
  single token small; a leaked token is scoped and expirable, not a master key.
- One enforcement point (the governance gate) means authorization is auditable
  and hard to forget — token use flows through the same chain-logged decision
  path as every other governed action.
- Human-join gives a signed, fail-closed, veto-wins admission flow for people
  (native or bridged) without weakening the key-only identity model — the
  daemon never holds or forges a human's approval.
- Tokens are content-addressed signed artifacts stored in the mesh subtree, so
  they replicate and survive rejoins via the same ADR-064 machinery as
  everything else — no separate token distribution system.

### Negative

- A capability bitset caps the number of distinct powers (a `u32` gives 32);
  deliberate, but a ceiling that a sprawling permission taxonomy could hit.
  Reserve bits conservatively.
- Delegation chains must be verified transitively (each link a subset of its
  parent) up to a root issuer; a deep chain is more verification work and a
  larger surface for a misissued intermediate token. Bound delegation depth.
- Revocation is not free: a revoked token id must be checked on every use, which
  means a revocation set that replicates promptly across the mesh (ADR-064) —
  a partitioned node may honor a token the rest of the mesh has revoked until it
  reconverges. Short expiries mitigate; instant global revocation is not
  achievable in a partition-tolerant mesh.
- Routing tokens through the effect-vector governance evaluation means a valid
  token can still be `Defer`red — correct, but it makes "why was my authorized
  action escalated" a two-part answer (token OK, governance rule escalated) that
  operators must be able to read off the chain.

### Neutral

- Capability tokens **complement**, not replace, the ADR-057 read ACL table and
  the ADR-063 path-ownership rule: ownership and public-read defaults handle the
  common case with no token; tokens are the mechanism for the *exceptions*
  (cross-prefix writes, cross-node sync grants, bridge grants, human joins).
- The four-class capability gate (`clawft-weave`) can remain as a coarse outer
  gate during migration; a caller still needs the verb-class *and* a satisfying
  token, the same layering ADR-057 uses for reads.
- Human-join reuses the `ActionProposal` / `SignedDecision` shapes; if a
  general side-effectful-action approval primitive is later added to the kernel,
  join is a special case of it (`kind = "join"`) rather than a separate system.

## Alternatives considered

- **Extend the four-class capability gate with path scoping instead of a new
  token type.** Keeps one authorization concept, but the class gate is a
  session-level check with no delegable, holdable, revocable artifact — it
  cannot express "peer X may sync subtree Y until date Z" as a thing that
  replicates across the mesh. Rejected; a token is the portable grant the mesh
  needs.
- **ACL entries only (extend ADR-057's table to writes and channels).** ACLs
  are node-anchored policy the resource owner writes; they do not model
  *delegation* (a subject re-granting a subset) or *holder-presented* evidence.
  Rejected as the primary mechanism, though the ACL table remains the
  default-policy layer beneath tokens.
- **Auto-admit humans via web-of-trust depth instead of explicit approval.**
  Transitive trust (AgentBBS ADR-0043) scales peer promotion, but admitting a
  *human* to a scoped resource is exactly the case that warrants a human in the
  loop; silent transitive admission is the wrong default for a consent
  boundary. Rejected for human-join; web-of-trust remains appropriate for
  *peer* trust promotion in ADR-064.
- **API-key / bearer-token auth for the join surface.** Simpler for external
  callers, but it reintroduces a shared secret the daemon must store and could
  leak, and it decouples authorization from the key-only identity model.
  Rejected in favor of holder-signed decisions (possession of the key *is* the
  credential), matching AgentBBS ADR-0038.

## References

- **AgentBBS** (github.com/ruvnet/AgentBBS) — **FSL-1.1-MIT** licensed (from
  late.sh), *not* MIT/Apache. Read for design patterns; no code copied. The
  capability-bitset, human-join, and fail-closed-approval design was
  reimplemented from:
  - ADR-0004 (Capability-Based Authorization) — the `Caps` bitset, least-
    privilege `default()`, roles as monotonic cap-bundles, and the single
    `require(held, needed, name)` enforcement primitive.
  - ADR-0038 (Human-in-the-Loop Approval Gates) — content-addressed
    `ActionProposal`, Ed25519-signed `SignedDecision`, and the **fail-closed,
    veto-wins** `is_authorized` rule where the server only verifies and never
    signs on a human's behalf. The human-join flow is a scoped application of
    this.
  - ADR-0015 (Agent mention / loop-in) — the "summon / admit a participant"
    interaction shape for bringing an outside party into a resource.
  - ADR-0016 (Anonymous Client-Held Keys) — client generates and holds the key;
    the daemon only verifies — applied to native human joiners.
  - ADR-0043 (Web-of-Trust) — considered and deliberately *not* used for
    human-join (see alternatives); relevant to peer trust in ADR-064.
- WeftOS internal: ADR-025, ADR-063, ADR-064, ADR-057, ADR-049, ADR-033,
  ADR-022; `crates/clawft-kernel/src/gate.rs`,
  `crates/clawft-weave/src/capability.rs`.
