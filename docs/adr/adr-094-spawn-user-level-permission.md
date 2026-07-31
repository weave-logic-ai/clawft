# ADR-094: Spawn-at-user-level permission story

**Date**: 2026-07-31  
**Status**: Accepted (foundation)  
**Deciders**: Governance end-state wave (WEFT-635)  
**Depends-On**: ADR-033 (three-branch governance), ADR-045 (tiered router permissions),
ADR-066 (capability tokens / human-join governance)  
**Relates-To**: WEFT-633 (Defer + grant approval UX), WEFT-634 (action/tool selectors),
WEFT-636 (gate principals), M4 agent-initiated spawn

## Context

Agents may spawn children (`agent_spawn`) on a user's behalf. Control already
holds at the gate (effect magnitude + optional human approval), but the
**permission story** was implicit:

- Who authorises a spawn?
- How is that authority scoped (tools, depth, budget, lifetime)?
- How is it revoked?
- How does attribution appear on the chain?

Without an explicit model, GA approval-UX (WEFT-633) and multi-tenant principals
(WEFT-636) cannot compose cleanly.

## Decision

### D1 — User is the authority principal

Every spawn evaluation carries a [`GatePrincipal`](../../crates/clawft-kernel/src/governance.rs)
with:

| Field | Meaning |
|-------|---------|
| `agent_id` | Acting agent (parent or child) |
| `user_id` | Human / tenant who authorised the session |
| `parent_agent_id` | Parent agent when evaluating a child |
| `conv_id` | Conversation scope |

The **user** is the root of authority. Agents act as delegates. A spawn without
a resolvable `user_id` is still gated by magnitude / selectors, but cannot
receive a user-scoped grant token (future).

### D2 — Permission modes for spawn

| Mode | Gate outcome | When |
|------|--------------|------|
| **Autonomous grant** | `Permit` | Operator exempts `tool.agent_spawn`, or no blocking selector rule |
| **In-conversation approval** | `EscalateToHuman` → gate `Defer` | `SPAWN-APPROVAL` rule (WEFT-633/634) + `human_approval_required` |
| **Hard deny** | `Deny` | Blocking rule matches without human-approval path |
| **Pre-granted scope** (future) | `Permit` with scoped token | User issues a capability claim (ADR-066) for spawn depth/budget |

Foundation shipped now: modes 1–3 via action/tool selectors +
`GovernanceRule::spawn_requires_approval()` + existing `DeferInteractor`
(WEFT-331) for grant/deny/cancel.

### D3 — Scoping

Spawn authority is scoped by the intersection of:

1. **Tool / action selector** — rule matches `tool.agent_spawn` / tool `agent_spawn`
2. **Spawn depth** — M4 D5 recursion cap (`SpawnSpec.depth`)
3. **Budget** — per-child `SpawnBudget` clamped to daemon base (WEFT-631)
4. **Conversation** — `parent_conv_id` + forest edges (TriggeredBy / EvidenceFor)
5. **User** — `GatePrincipal.user_id` on the chain record (WEFT-636)

Revocation (foundation):

- Deactivate the `SPAWN-APPROVAL` / grant rule, or remove action exemption
- Cancel running children via spawn registry (`TaskStatus::Cancelled`)
- Future: TTL / revoke on capability claims (ADR-066) — not required for 0.8.x

### D4 — Mapping onto the governance engine

```
agent_spawn tool
  → effect_for_tool (magnitude ≈ 0.93)
  → gate.check(agent_id, "tool.agent_spawn", context{user_id, conv_id, effect, tool})
  → GovernanceRequest + GatePrincipal
  → evaluate(): selector match + force_on_match / magnitude
  → EscalateToHuman | Deny | Permit
  → GateDecision::Defer | Deny | Permit
  → DeferInteractor (panel) on Defer → Allow resumes tool; Deny cancels cleanly
  → chain witness includes principal + decision
```

### D5 — What is not in this ADR

- Full capability-token mint/revoke UI (remains ADR-066)
- Cross-tenant spawn federation
- Automatic grant learning from repeated Approves

## Consequences

- **Positive**: Approval-UX and principal attribution share one model; selectors
  make spawn-only policy possible without lowering global risk threshold.
- **Negative**: Full pre-granted capability claims still future work; operators
  must enable `human_approval_required` + install `SPAWN-APPROVAL` (or rely on
  magnitude + blocking rule) for Defer UX.
- **Neutral**: Default open/dev engines remain permissive unless rules are added.

## Acceptance (WEFT-635)

- [x] User-level spawn permission model documented (this ADR)
- [x] Scoping + revocation semantics defined (D3)
- [x] Maps onto the governance engine (D4; types in `GatePrincipal`, selectors)
