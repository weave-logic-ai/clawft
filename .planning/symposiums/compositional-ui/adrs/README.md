# Compositional UI Symposium — ADR Index

These ADRs are symposium-scoped (numbered 001 onward within this
folder) and do not collide with the repo-wide `docs/adr/` numbering.
All are **Proposed — symposium round 2** pending round-table review.

Each ADR follows: Status / Context / Decision / Consequences /
Alternatives considered / Related. The canon (four predicates,
Tier-A vs Tier-B split, active-radar loop, non-negotiable privacy)
is enforced by every ADR — none proposes changes to it.

| # | Title | One-line summary |
|---|-------|------------------|
| 001 | Primitive canon (18 items) | Freezes 17 Tier-A primitives + 1 Tier-B (`ui://foreign`); IRI stem + four-predicate check + egui crate per primitive. |
| 002 | Two-tier model & ForeignSurface promotion | No Tier-C; six canonical wrapper types ship day one; four-stage promotion gate (usage / proof / ADR / canon amendment). |
| 003 | WSP transport bindings | Local UDS + JSON-RPC, mesh CBOR in `MessagePayload::Wsp`, browser JSON-RPC over WebSocket — one CDDL, two encodings, no new rail. |
| 004 | Schema shape — state-diff + event-sourced | Surface tree is state-diff with `surface_version`; observation tapestry is event-sourced with `seq`; CRDT only inside leaf payloads. |
| 005 | WSP verb set (17 verbs) | Seventeen namespaced verbs: session (2), surfaces (4), subscribe (2), observe (2), invoke/mutate (2), governance/consent (3), introspection (2). |
| 006 | Primitive head (mandatory fields) | Every primitive head carries `state`, `affordances`, `confidence`, `variant-id`, `mutation-axes`, `privacy-flags`; missing any = malformed at kernel. |
| 007 | Active-radar return schema | `CanonResponse` extends `egui::Response`; topology / doppler / range / bearing emitted on `ux/returns`; every echo stamps its variant-id. |
| 008 | Goal aggregate + per-goal delegation | `Goal` is an Evans aggregate root, chain-anchored; every WSP call carries `goal_id`; delegation is per-goal; `GovernanceGate` binding extended with `goal_id`. |
| 009 | Mission Console: seat / witness / anti-corruption | NORAD discipline for agent-composed shared surfaces: every participant gets a seat, every verb is witnessed on-chain, `ContextMap` enforces bounded contexts. |
| 010 | Reverse-DDD primitive mapping | Canonical table: aggregate root → Surface, entity → Chip, value → Field, invariant → AffordanceGate, context boundary → IRI namespace + governance, anti-corruption → IRI translation. |
| 011 | Dev-panel embedding — Option C hybrid | egui-wasm in VSCode `WebviewView` + `weft-capture` native sidecar for mic/camera/screen; M0/M1/M2 milestones; zero-business-logic extension. |
| 012 | Capture privacy + tray chip | `privacy-flags` structural at kernel boundary; consent per-goal with expiry on seal; kernel (not agent) composes persistent non-dismissible tray chip when capture is live. |

## Authority + cross-reference

- **Canon source**: `../foundations.md` — four predicates, Tier-A/B,
  active-radar loop, non-negotiable privacy. All twelve ADRs cite
  at least one foundation element.
- **Research source**: `../session-1-historical-canon.md` through
  `../session-9-agentic-os-canon.md`. Every ADR cites at least two
  sessions.
- **Agenda**: `../AGENDA.md` — RQ1..RQ9 mapping. The ADR track is
  the Round 2 synthesis deliverable.
- **Renderer**: egui is canonical. Renderer-disposability (AGENDA
  success criterion 1) is proved when a second renderer can
  implement these ADRs without touching composition logic.

## Reading order

For a first pass, read in dependency order: **001 → 002 → 006 →
003 → 004 → 005 → 007 → 010 → 008 → 009 → 011 → 012**. Each builds
on the ones before it. For a second pass (conformance), read
**006 → 007 → 008 → 012** as the "kernel boundary" quartet — these
are the structural invariants that make foundations' non-negotiables
type-system facts rather than conventions.
